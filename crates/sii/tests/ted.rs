//! Integration tests for Electronic Tax Stamp (TED) and PDF417 (TASK-TED-01).

use sii::codes::TipoDte;
use sii::dte::{Detalle, Dte, Emisor, Receptor};
use sii::error::Error;
use sii::ted::Ted;
use sii::types::{ManualClock, Monto, Rut};
use std::path::{Path, PathBuf};

fn find_fixture_path(filename: &str) -> PathBuf {
    let candidate1 = Path::new("tests/fixtures").join(filename);
    if candidate1.exists() {
        return candidate1;
    }
    Path::new("crates/sii/tests/fixtures").join(filename)
}

fn extract_tag<'a>(xml: &'a [u8], tag: &str) -> Option<&'a [u8]> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let start = xml.windows(open.len()).position(|w| w == open.as_bytes())?;
    let close_pos = xml
        .windows(close.len())
        .position(|w| w == close.as_bytes())?;
    let end = close_pos.checked_add(close.len())?;
    xml.get(start..end)
}

fn sample_emisor() -> Result<Emisor, Error> {
    let rut = Rut::parse("76.086.428-5")?;
    Ok(Emisor::new(
        rut,
        "DISTRIBUIDORA Y COMERCIAL EJEMPLO SPA",
        "VENTA AL POR MAYOR DE MAQUINARIA",
        465_900,
        "AVENIDA LIBERTADOR 1234",
        "SANTIAGO",
    )
    .ciudad_origen("SANTIAGO"))
}

fn sample_receptor() -> Result<Receptor, Error> {
    let rut = Rut::parse("77.777.777-7")?;
    Ok(Receptor::new(
        rut,
        "CLIENTE DE PRUEBAS SA",
        "SERVICIOS INFORMATICOS",
        "CALLE COMERCIAL 567",
        "PROVIDENCIA",
    )
    .ciudad_recep("SANTIAGO"))
}

fn sample_dte() -> Result<Dte, Error> {
    Dte::factura_afecta()
        .folio(60)
        .fch_emis_str("2026-10-21")?
        .emisor(sample_emisor()?)
        .receptor(sample_receptor()?)
        .item(
            Detalle::new(1, "Servidor de aplicaciones", Monto::from_pesos(100_000))
                .codigo("INT1", "PROD-01")
                .qty(1)
                .prc(Monto::from_pesos(100_000)),
        )
        .build()
}

fn sample_caf() -> Result<sii::Caf, Error> {
    let caf_path = find_fixture_path("caf-33.xml");
    let caf_bytes = std::fs::read(caf_path).map_err(|e| Error::InvalidCaf(e.to_string()))?;
    sii::Caf::parse(&caf_bytes)
}

fn fixed_clock() -> Result<ManualClock, Error> {
    let date = time::Date::from_calendar_date(2026, time::Month::October, 21)
        .map_err(|e| Error::InvalidDate(e.to_string()))?;
    let time = time::Time::from_hms(15, 30, 0).map_err(|e| Error::InvalidDate(e.to_string()))?;
    Ok(ManualClock::new(date.with_time(time).assume_utc()))
}

#[test]
fn test_dd_canonical_byte_for_byte_against_fixture() {
    let caf = sample_caf().expect("caf");
    let dte = sample_dte().expect("dte");
    let clock = fixed_clock().expect("clock");

    let stamped = dte.timbrar(&caf, 60, &clock).expect("timbrar must succeed");

    let generated_dd = stamped.ted().dd().to_canonical_xml();

    let fixture_raw = std::fs::read(find_fixture_path("dd-33.xml")).expect("read dd-33.xml");
    let expected_dd = extract_tag(&fixture_raw, "DD").expect("extract DD");

    assert_eq!(
        generated_dd, expected_dd,
        "Generated DD must match canonical fixture byte-for-byte (TED-01, RF-01, RF-04)"
    );
}

#[test]
fn test_ted_pdf417_data_byte_for_byte_against_fixture() {
    let caf = sample_caf().expect("caf");
    let dte = sample_dte().expect("dte");
    let clock = fixed_clock().expect("clock");

    let stamped = dte.timbrar(&caf, 60, &clock).expect("timbrar must succeed");

    let pdf417_data = stamped.ted_pdf417_data();

    let fixture_raw = std::fs::read(find_fixture_path("ted-33.xml")).expect("read ted-33.xml");
    let expected_ted = extract_tag(&fixture_raw, "TED").expect("extract TED");

    assert_eq!(
        pdf417_data, expected_ted,
        "PDF417 data must match serialized TED fixture byte-for-byte (TED-02, RF-03, RF-04)"
    );
}

#[test]
fn test_frmt_verification_with_caf_public_key() {
    let caf = sample_caf().expect("caf");
    let dte = sample_dte().expect("dte");
    let clock = fixed_clock().expect("clock");

    let stamped = dte.timbrar(&caf, 60, &clock).expect("timbrar");

    // Verify signature with CAF public key (RF-02, RF-04)
    stamped
        .ted()
        .verificar_firma(caf.public_key())
        .expect("FRMT digital signature must verify with CAF public key");
}

#[test]
fn test_timbrar_within_range() {
    let caf = sample_caf().expect("caf");
    let dte = sample_dte().expect("dte");
    let clock = fixed_clock().expect("clock");

    // Range is [1, 100]
    for folio in [1, 50, 60, 100] {
        let stamped = dte
            .timbrar(&caf, folio, &clock)
            .unwrap_or_else(|e| panic!("timbrar for folio {folio} failed: {e}"));
        assert_eq!(stamped.folio(), folio);
        assert_eq!(stamped.ted().dd().f(), folio);
        stamped
            .ted()
            .verificar_firma(caf.public_key())
            .unwrap_or_else(|e| panic!("verify signature for folio {folio} failed: {e}"));
    }
}

#[test]
fn test_timbrar_out_of_range_fails() {
    let caf = sample_caf().expect("caf");
    let dte = sample_dte().expect("dte");
    let clock = fixed_clock().expect("clock");

    // Folio 0 is below minimum 1
    let err_under = dte.timbrar(&caf, 0, &clock).unwrap_err();
    assert_eq!(
        err_under,
        Error::FolioOutOfRange {
            folio: 0,
            min: 1,
            max: 100,
        }
    );

    // Folio 101 is above maximum 100
    let err_over = dte.timbrar(&caf, 101, &clock).unwrap_err();
    assert_eq!(
        err_over,
        Error::FolioOutOfRange {
            folio: 101,
            min: 1,
            max: 100,
        }
    );
}

#[test]
fn test_timbrar_caf_type_mismatch_fails() {
    let caf = sample_caf().expect("caf"); // Tipo 33 (Factura Afecta)
    let clock = fixed_clock().expect("clock");

    // Build Factura Exenta (Tipo 34)
    let dte_exenta = Dte::factura_exenta()
        .folio(60)
        .fch_emis_str("2026-10-21")
        .unwrap()
        .emisor(sample_emisor().unwrap())
        .receptor(sample_receptor().unwrap())
        .item(Detalle::new(1, "Item Exento", Monto::from_pesos(50_000)))
        .build()
        .unwrap();

    let err = dte_exenta.timbrar(&caf, 60, &clock).unwrap_err();
    match err {
        Error::DteValidation(msg) => {
            assert!(
                msg.contains("CAF document type") && msg.contains("does not match DTE type"),
                "expected type mismatch message, got: {msg}"
            );
        }
        other => panic!("expected DteValidation, got {other:?}"),
    }
}

#[test]
fn test_timbrar_caf_issuer_rut_mismatch_fails() {
    let caf = sample_caf().expect("caf"); // Issuer is 76086428-5
    let clock = fixed_clock().expect("clock");

    // Build DTE with different issuer RUT 77777777-7
    let diff_emisor = Emisor::new(
        Rut::parse("77.777.777-7").unwrap(),
        "OTRO EMISOR SPA",
        "GIRO",
        12345,
        "DIR",
        "CMNA",
    );

    let dte_diff = Dte::factura_afecta()
        .folio(60)
        .fch_emis_str("2026-10-21")
        .unwrap()
        .emisor(diff_emisor)
        .receptor(sample_receptor().unwrap())
        .item(Detalle::new(1, "Item", Monto::from_pesos(100_000)))
        .build()
        .unwrap();

    let err = dte_diff.timbrar(&caf, 60, &clock).unwrap_err();
    match err {
        Error::DteValidation(msg) => {
            assert!(
                msg.contains("CAF issuer RUT") && msg.contains("does not match DTE issuer RUT"),
                "expected issuer RUT mismatch message, got: {msg}"
            );
        }
        other => panic!("expected DteValidation, got {other:?}"),
    }
}

#[test]
fn test_timbre_reproducible_with_clock() {
    let caf = sample_caf().expect("caf");
    let dte = sample_dte().expect("dte");
    let clock = fixed_clock().expect("clock");

    let stamped1 = dte.timbrar(&caf, 60, &clock).unwrap();
    let stamped2 = dte.timbrar(&caf, 60, &clock).unwrap();

    assert_eq!(
        stamped1.ted().raw_dd(),
        stamped2.ted().raw_dd(),
        "DD must be reproducible (QA-03)"
    );
    assert_eq!(
        stamped1.ted().frmt(),
        stamped2.ted().frmt(),
        "FRMT must be deterministic (QA-03)"
    );
    assert_eq!(stamped1.ted_pdf417_data(), stamped2.ted_pdf417_data());
}

#[test]
fn test_tampered_dd_fails_verification() {
    let caf = sample_caf().expect("caf");
    let dte = sample_dte().expect("dte");
    let clock = fixed_clock().expect("clock");

    let stamped = dte.timbrar(&caf, 60, &clock).unwrap();
    let ted = stamped.ted();

    // Tamper with DD bytes (change folio 60 to 61)
    let tampered_bytes = String::from_utf8(ted.raw_dd().to_vec())
        .unwrap()
        .replace("<F>60</F>", "<F>61</F>")
        .into_bytes();

    let parsed_tampered = format!(
        "<TED version=\"1.0\">{}<FRMT algoritmo=\"SHA1withRSA\">{}</FRMT></TED>",
        String::from_utf8(tampered_bytes).unwrap(),
        ted.frmt()
    );

    let parsed_ted = Ted::parse(parsed_tampered.as_bytes()).expect("parse");
    let result = parsed_ted.verificar_firma(caf.public_key());
    assert!(
        result.is_err(),
        "Tampered DD must fail verification (SIG-02, QA-02)"
    );
}

#[test]
fn test_tampered_frmt_fails_verification() {
    let caf = sample_caf().expect("caf");
    let dte = sample_dte().expect("dte");
    let clock = fixed_clock().expect("clock");

    let stamped = dte.timbrar(&caf, 60, &clock).unwrap();
    let ted = stamped.ted();

    // Corrupt signature
    let corrupted_xml = format!(
        "<TED version=\"1.0\">{}<FRMT algoritmo=\"SHA1withRSA\">AAAA</FRMT></TED>",
        String::from_utf8(ted.raw_dd().to_vec()).unwrap()
    );

    let parsed_corrupted = Ted::parse(corrupted_xml.as_bytes()).expect("parse");
    let result = parsed_corrupted.verificar_firma(caf.public_key());
    assert!(
        result.is_err(),
        "Corrupted FRMT signature must fail verification"
    );
}

#[test]
fn test_stamped_dte_xml_serialization() {
    let caf = sample_caf().expect("caf");
    let dte = sample_dte().expect("dte");
    let clock = fixed_clock().expect("clock");

    let stamped = dte.timbrar(&caf, 60, &clock).expect("timbrar");
    let xml_bytes = stamped.to_xml().expect("to_xml");

    let xml_str = String::from_utf8(xml_bytes).expect("valid utf-8");

    // Must start with DTE and Documento
    assert!(xml_str.starts_with("<DTE version=\"1.0\"><Documento ID=\"F60T33\">"));
    assert!(xml_str.contains("<Encabezado>"));
    assert!(xml_str.contains("<Detalle>"));

    // Must include the complete TED element
    assert!(xml_str.contains("<TED version=\"1.0\"><DD>"));
    assert!(xml_str.contains("<CAF version=\"1.0\">"));
    assert!(xml_str.contains("<FRMT algoritmo=\"SHA1withRSA\">"));
    assert!(xml_str.ends_with("</Documento></DTE>"));
}

#[test]
fn test_ted_parse_roundtrip_with_fixture() {
    let fixture_raw = std::fs::read(find_fixture_path("ted-33.xml")).expect("read ted-33.xml");
    let clean_bytes = extract_tag(&fixture_raw, "TED").expect("extract TED");

    let parsed = Ted::parse(clean_bytes).expect("parse ted fixture");
    assert_eq!(parsed.dd().re().to_string(), "76086428-5");
    assert_eq!(parsed.dd().td(), TipoDte::FacturaAfecta);
    assert_eq!(parsed.dd().f(), 60);
    assert_eq!(parsed.dd().rr().to_string(), "77777777-7");
    assert_eq!(parsed.dd().mnt(), Monto::from_pesos(119_000));
    assert_eq!(parsed.dd().it1(), "Servidor de aplicaciones");
    assert_eq!(parsed.dd().tsted(), "2026-10-21T15:30:00");

    let caf = sample_caf().expect("caf");
    parsed
        .verificar_firma(caf.public_key())
        .expect("parsed fixture must verify with CAF public key");
}
