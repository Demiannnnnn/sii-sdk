//! Integration tests for purchase and sales ledger (TASK-LIBRO-01).

use quick_xml::events::Event;
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use rsa::RsaPrivateKey;
use rsa::RsaPublicKey;
use rsa::pkcs1v15::SigningKey;
use rsa::rand_core::OsRng;
use rsa::signature::{SignatureEncoding, Signer as _};
use sha1::Sha1;
use sii::codes::TipoDte;
use sii::dte::{Detalle, Dte, Emisor, Receptor};
use sii::libro::{
    DetalleLibro, LibroCompraVenta, PeriodoTributario, TipoOperacion, TotalesPeriodo,
};
use sii::signature::{SignatureOptions, Signer, X509Certificate, verify, verify_by_reference};
use sii::types::{ManualClock, Monto, Rut};
use std::io::Cursor;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

struct TestSigner {
    private_key: RsaPrivateKey,
}

impl Signer for TestSigner {
    fn sign(&self, data: &[u8]) -> sii::Result<Vec<u8>> {
        let key = SigningKey::<Sha1>::new(self.private_key.clone());
        Ok(key.sign(data).to_vec())
    }
}

fn test_signer_and_cert() -> Result<(TestSigner, X509Certificate), Box<dyn std::error::Error>> {
    let mut rng = OsRng;
    let priv_key = RsaPrivateKey::new(&mut rng, 2048)?;
    let pub_key = RsaPublicKey::from(&priv_key);
    let cert = X509Certificate::from_parts(vec![0x30, 0x03, 0x02, 0x01, 0x01], pub_key);
    Ok((
        TestSigner {
            private_key: priv_key,
        },
        cert,
    ))
}

fn clock() -> Result<ManualClock, Box<dyn std::error::Error>> {
    let date = time::Date::from_calendar_date(2026, time::Month::October, 21)?;
    let t = time::Time::from_hms(12, 30, 0)?;
    Ok(ManualClock::new(date.with_time(t).assume_utc()))
}

fn resol_date() -> Result<time::Date, Box<dyn std::error::Error>> {
    Ok(time::Date::from_calendar_date(
        2014,
        time::Month::August,
        22,
    )?)
}

/// Strips comments and non-significant whitespace from XML bytes to yield canonical XML.
fn canonicalize_xml(bytes: &[u8]) -> Result<Vec<u8>, quick_xml::Error> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let mut writer = Writer::new(Cursor::new(Vec::new()));

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Eof => break,
            Event::Comment(_) | Event::Decl(_) => {}
            event => {
                writer.write_event(event)?;
            }
        }
        buf.clear();
    }

    Ok(writer.into_inner().into_inner())
}

#[test]
fn test_serialize_libro_against_fixture() -> TestResult {
    let fixture_raw = include_bytes!("fixtures/libro-venta.xml");
    let expected_canonical = canonicalize_xml(fixture_raw)?;

    let rut_emisor = Rut::parse("76.086.428-5")?;
    let rut_envia = Rut::parse("11.111.111-1")?;
    let periodo = PeriodoTributario::new(2026, 10)?;
    let rut_cliente = Rut::parse("77.777.777-7")?;

    let mut libro = LibroCompraVenta::nuevo(
        rut_emisor,
        rut_envia,
        periodo,
        resol_date()?,
        0,
        TipoOperacion::Venta,
    );

    let doc1 = DetalleLibro::new(TipoDte::FacturaAfecta, 101)
        .fch_doc_str("2026-10-15")?
        .rut_doc(rut_cliente)
        .rzn_soc("CLIENTE DE PRUEBAS SA")
        .mnt_exe(Monto::ZERO)
        .mnt_neto(Monto::from_pesos(100_000))
        .mnt_iva(Monto::from_pesos(19_000))
        .mnt_total(Monto::from_pesos(119_000));

    let doc2 = DetalleLibro::new(TipoDte::FacturaAfecta, 102)
        .fch_doc_str("2026-10-20")?
        .rut_doc(rut_cliente)
        .rzn_soc("CLIENTE DE PRUEBAS SA")
        .mnt_exe(Monto::ZERO)
        .mnt_neto(Monto::from_pesos(50_000))
        .mnt_iva(Monto::from_pesos(9_500))
        .mnt_total(Monto::from_pesos(59_500));

    libro.agregar_detalle(doc1);
    libro.agregar_detalle(doc2);

    let generated_xml = libro.to_xml(&clock()?)?;
    let generated_canonical = canonicalize_xml(&generated_xml)?;

    assert_eq!(
        String::from_utf8(generated_canonical.clone())?,
        String::from_utf8(expected_canonical.clone())?,
        "generated LibroCompraVenta canonical XML must match fixture byte-for-byte"
    );
    assert_eq!(generated_canonical, expected_canonical);

    Ok(())
}

#[test]
fn test_totales_periodo_auto_aggregation() -> TestResult {
    let rut_emisor = Rut::parse("76.086.428-5")?;
    let rut_envia = Rut::parse("11.111.111-1")?;
    let periodo = PeriodoTributario::new(2026, 10)?;
    let rut_cliente = Rut::parse("77.777.777-7")?;

    let mut libro = LibroCompraVenta::nuevo(
        rut_emisor,
        rut_envia,
        periodo,
        resol_date()?,
        0,
        TipoOperacion::Venta,
    );

    // 2 active Facturas Afectas (33)
    libro.agregar_detalle(
        DetalleLibro::new(TipoDte::FacturaAfecta, 1)
            .rut_doc(rut_cliente)
            .mnt_neto(Monto::from_pesos(100_000))
            .mnt_iva(Monto::from_pesos(19_000)),
    );
    libro.agregar_detalle(
        DetalleLibro::new(TipoDte::FacturaAfecta, 2)
            .rut_doc(rut_cliente)
            .mnt_neto(Monto::from_pesos(200_000))
            .mnt_iva(Monto::from_pesos(38_000)),
    );

    // 1 annulled Factura Afecta (33) - should not contribute amounts to totals
    libro.agregar_detalle(DetalleLibro::new(TipoDte::FacturaAfecta, 3).anulado(true));

    // 1 Factura Exenta (34)
    libro.agregar_detalle(
        DetalleLibro::new(TipoDte::FacturaExenta, 1)
            .rut_doc(rut_cliente)
            .mnt_exe(Monto::from_pesos(50_000)),
    );

    // 1 Nota de Credito (61)
    libro.agregar_detalle(
        DetalleLibro::new(TipoDte::NotaCredito, 1)
            .rut_doc(rut_cliente)
            .mnt_neto(Monto::from_pesos(30_000))
            .mnt_iva(Monto::from_pesos(5_700)),
    );

    let totales = libro.calcular_totales()?;
    assert_eq!(totales.len(), 3, "expected totals for 33, 34 and 61");

    // Totales for Factura Afecta (33)
    let tot33 = totales
        .iter()
        .find(|t| t.tpo_doc == TipoDte::FacturaAfecta)
        .ok_or("missing 33")?;
    assert_eq!(tot33.tot_doc, 3);
    assert_eq!(tot33.tot_anulado, Some(1));
    assert_eq!(tot33.tot_mnt_neto, Monto::from_pesos(300_000));
    assert_eq!(tot33.tot_mnt_iva, Monto::from_pesos(57_000));
    assert_eq!(tot33.tot_mnt_total, Monto::from_pesos(357_000));

    // Totales for Factura Exenta (34)
    let tot34 = totales
        .iter()
        .find(|t| t.tpo_doc == TipoDte::FacturaExenta)
        .ok_or("missing 34")?;
    assert_eq!(tot34.tot_doc, 1);
    assert_eq!(tot34.tot_anulado, None);
    assert_eq!(tot34.tot_mnt_exe, Monto::from_pesos(50_000));
    assert_eq!(tot34.tot_mnt_total, Monto::from_pesos(50_000));

    // Totales for Nota de Credito (61)
    let tot61 = totales
        .iter()
        .find(|t| t.tpo_doc == TipoDte::NotaCredito)
        .ok_or("missing 61")?;
    assert_eq!(tot61.tot_doc, 1);
    assert_eq!(tot61.tot_mnt_neto, Monto::from_pesos(30_000));
    assert_eq!(tot61.tot_mnt_iva, Monto::from_pesos(5_700));
    assert_eq!(tot61.tot_mnt_total, Monto::from_pesos(35_700));

    Ok(())
}

#[test]
fn test_libro_signed_verifies() -> TestResult {
    let (signer, cert) = test_signer_and_cert()?;

    let rut_emisor = Rut::parse("76.086.428-5")?;
    let rut_envia = Rut::parse("11.111.111-1")?;
    let periodo = PeriodoTributario::new(2026, 10)?;
    let rut_cliente = Rut::parse("77.777.777-7")?;

    let mut libro = LibroCompraVenta::nuevo(
        rut_emisor,
        rut_envia,
        periodo,
        resol_date()?,
        0,
        TipoOperacion::Venta,
    );

    libro.agregar_detalle(
        DetalleLibro::new(TipoDte::FacturaAfecta, 101)
            .fch_doc_str("2026-10-15")?
            .rut_doc(rut_cliente)
            .rzn_soc("CLIENTE DE PRUEBAS SA")
            .mnt_neto(Monto::from_pesos(100_000))
            .mnt_iva(Monto::from_pesos(19_000)),
    );

    let signed_xml = libro.firmar(&clock()?, &signer, &cert, SignatureOptions::default())?;

    // Verify by reference to the EnvioLibro ID
    assert!(
        verify_by_reference(&signed_xml, libro.envio_id_str())?,
        "signature over EnvioLibro must verify by reference"
    );

    // Verify using standard enveloped verify
    assert!(
        verify(&signed_xml)?,
        "signature over EnvioLibro must verify with standard verify"
    );

    let xml_text = String::from_utf8(signed_xml)?;
    assert!(xml_text.starts_with("<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>"));
    assert!(xml_text.contains("<TipoOperacion>VENTA</TipoOperacion>"));
    assert!(xml_text.contains("<PeriodoTributario>2026-10</PeriodoTributario>"));
    assert!(xml_text.contains("<TmstFirma>2026-10-21T12:30:00</TmstFirma>"));
    assert!(xml_text.trim_end().ends_with("</LibroCompraVenta>"));

    Ok(())
}

#[test]
fn test_libro_signature_tamper_is_detected() -> TestResult {
    let (signer, cert) = test_signer_and_cert()?;

    let rut_emisor = Rut::parse("76.086.428-5")?;
    let rut_envia = Rut::parse("11.111.111-1")?;
    let periodo = PeriodoTributario::new(2026, 10)?;
    let rut_cliente = Rut::parse("77.777.777-7")?;

    let mut libro = LibroCompraVenta::nuevo(
        rut_emisor,
        rut_envia,
        periodo,
        resol_date()?,
        0,
        TipoOperacion::Venta,
    );

    libro.agregar_detalle(
        DetalleLibro::new(TipoDte::FacturaAfecta, 101)
            .rut_doc(rut_cliente)
            .mnt_neto(Monto::from_pesos(100_000))
            .mnt_iva(Monto::from_pesos(19_000)),
    );

    let signed_xml = libro.firmar(&clock()?, &signer, &cert, SignatureOptions::default())?;
    assert!(verify(&signed_xml)?);

    // Tamper with payload (change net amount)
    let text = String::from_utf8(signed_xml)?;
    let tampered = text.replace("<MntNeto>100000</MntNeto>", "<MntNeto>999999</MntNeto>");

    assert!(
        !verify(tampered.as_bytes())?,
        "tampered XML payload must fail signature verification"
    );
    assert!(
        !verify_by_reference(tampered.as_bytes(), libro.envio_id_str())?,
        "tampered XML payload must fail verify_by_reference"
    );

    Ok(())
}

#[test]
fn test_agregar_dte_and_dte_timbrado() -> TestResult {
    let rut_emisor = Rut::parse("76.086.428-5")?;
    let rut_receptor = Rut::parse("77.777.777-7")?;

    let emisor = Emisor::new(
        rut_emisor,
        "DISTRIBUIDORA Y COMERCIAL EJEMPLO SPA",
        "VENTA AL POR MAYOR",
        465_900,
        "AVENIDA LIBERTADOR 1234",
        "SANTIAGO",
    );
    let receptor = Receptor::new(
        rut_receptor,
        "CLIENTE DE PRUEBAS SA",
        "SERVICIOS INFORMATICOS",
        "CALLE COMERCIAL 567",
        "PROVIDENCIA",
    );

    let dte = Dte::factura_afecta()
        .folio(501)
        .fch_emis_str("2026-10-18")?
        .emisor(emisor)
        .receptor(receptor)
        .item(
            Detalle::new(1, "Licencia de software", Monto::from_pesos(100_000))
                .qty(1)
                .prc(Monto::from_pesos(100_000)),
        )
        .build()?;

    // In a sales ledger, the counterparty is the receptor
    let mut libro_venta = LibroCompraVenta::nuevo(
        rut_emisor,
        Rut::parse("11.111.111-1")?,
        PeriodoTributario::new(2026, 10)?,
        resol_date()?,
        0,
        TipoOperacion::Venta,
    );
    libro_venta.agregar_dte(&dte);
    assert_eq!(libro_venta.len(), 1);
    let doc_venta = &libro_venta.detalles()[0];
    assert_eq!(doc_venta.tipo_doc, TipoDte::FacturaAfecta);
    assert_eq!(doc_venta.nro_doc, 501);
    assert_eq!(doc_venta.rut_doc, Some(rut_receptor));
    assert_eq!(doc_venta.rzn_soc.as_deref(), Some("CLIENTE DE PRUEBAS SA"));
    assert_eq!(doc_venta.mnt_neto, Some(Monto::from_pesos(100_000)));
    assert_eq!(doc_venta.mnt_iva, Some(Monto::from_pesos(19_000)));
    assert_eq!(doc_venta.mnt_total, Some(Monto::from_pesos(119_000)));

    // In a purchase ledger, the counterparty is the emisor
    let mut libro_compra = LibroCompraVenta::nuevo(
        rut_receptor,
        Rut::parse("11.111.111-1")?,
        PeriodoTributario::new(2026, 10)?,
        resol_date()?,
        0,
        TipoOperacion::Compra,
    );
    libro_compra.agregar_dte(&dte);
    let doc_compra = &libro_compra.detalles()[0];
    assert_eq!(doc_compra.rut_doc, Some(rut_emisor));
    assert_eq!(
        doc_compra.rzn_soc.as_deref(),
        Some("DISTRIBUIDORA Y COMERCIAL EJEMPLO SPA")
    );

    Ok(())
}

#[test]
fn test_libro_manual_totales() -> TestResult {
    let (signer, cert) = test_signer_and_cert()?;

    let rut_emisor = Rut::parse("76.086.428-5")?;
    let mut libro = LibroCompraVenta::nuevo(
        rut_emisor,
        Rut::parse("11.111.111-1")?,
        PeriodoTributario::new(2026, 10)?,
        resol_date()?,
        0,
        TipoOperacion::Venta,
    );

    libro.agregar_totales(
        TotalesPeriodo::new(TipoDte::FacturaAfecta, 10)
            .tot_mnt_neto(Monto::from_pesos(1_000_000))
            .tot_mnt_iva(Monto::from_pesos(190_000))
            .tot_mnt_total(Monto::from_pesos(1_190_000)),
    );

    assert_eq!(libro.len(), 0);
    assert!(libro.is_empty());

    let signed_xml = libro.firmar(&clock()?, &signer, &cert, SignatureOptions::default())?;
    assert!(verify(&signed_xml)?);

    let text = String::from_utf8(signed_xml)?;
    assert!(text.contains("<TotDoc>10</TotDoc>"));
    assert!(text.contains("<TotMntNeto>1000000</TotMntNeto>"));
    assert!(!text.contains("<Detalle>"));

    Ok(())
}

#[test]
fn test_libro_empty_sin_movimiento() -> TestResult {
    let rut_emisor = Rut::parse("76.086.428-5")?;
    let libro = LibroCompraVenta::nuevo(
        rut_emisor,
        Rut::parse("11.111.111-1")?,
        PeriodoTributario::new(2026, 10)?,
        resol_date()?,
        0,
        TipoOperacion::Venta,
    );

    let xml = libro.to_xml(&clock()?)?;
    let text = String::from_utf8(xml)?;
    assert!(text.contains("<Caratula>"));
    assert!(text.contains("<TmstFirma>2026-10-21T12:30:00</TmstFirma>"));
    assert!(!text.contains("<ResumenPeriodo>"));
    assert!(!text.contains("<Detalle>"));

    Ok(())
}

#[test]
fn test_periodo_tributario_validation() {
    assert!(PeriodoTributario::new(2026, 1).is_ok());
    assert!(PeriodoTributario::new(2026, 12).is_ok());
    assert!(PeriodoTributario::new(1999, 5).is_err());
    assert!(PeriodoTributario::new(2100, 5).is_err());
    assert!(PeriodoTributario::new(2026, 0).is_err());
    assert!(PeriodoTributario::new(2026, 13).is_err());

    let p = PeriodoTributario::parse("2026-05").unwrap();
    assert_eq!(p.year(), 2026);
    assert_eq!(p.month(), 5);
    assert_eq!(p.to_string(), "2026-05");
    assert_eq!(p.to_compact_str(), "202605");

    assert!(PeriodoTributario::parse("2026/05").is_err());
    assert!(PeriodoTributario::parse("2026").is_err());
    assert!(PeriodoTributario::parse("invalid").is_err());
}
