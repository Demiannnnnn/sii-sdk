//! Integration tests for DTE model and XML serialization (TASK-DTE-01).

use quick_xml::events::Event;
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use sii::codes::TipoDte;
use sii::dte::{Detalle, Dte, Emisor, Receptor};
use sii::error::Error;
use sii::types::{Monto, Rut};
use std::io::Cursor;

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

#[test]
fn test_serialize_dte_33_against_fixture() {
    let fixture_raw = include_bytes!("fixtures/dte-33.xml");
    let expected_canonical = canonicalize_xml(fixture_raw).unwrap();

    let dte = Dte::factura_afecta()
        .folio(60)
        .fch_emis_str("2026-10-21")
        .expect("valid date")
        .emisor(sample_emisor().unwrap())
        .receptor(sample_receptor().unwrap())
        .item(
            Detalle::new(1, "Servidor de aplicaciones", Monto::from_pesos(100_000))
                .codigo("INT1", "PROD-01")
                .qty(1)
                .prc(Monto::from_pesos(100_000)),
        )
        .build()
        .expect("build factura afecta must succeed");

    assert_eq!(dte.tipo(), TipoDte::FacturaAfecta);
    assert_eq!(dte.folio(), 60);
    assert_eq!(dte.id(), "F60T33");
    assert_eq!(dte.neto(), Some(Monto::from_pesos(100_000)));
    assert_eq!(dte.iva(), Some(Monto::from_pesos(19_000)));
    assert_eq!(dte.total(), Monto::from_pesos(119_000));
    assert_eq!(dte.exento(), None);

    let xml_bytes = dte.to_xml().expect("to_xml must succeed");

    // Byte-for-byte exact comparison against canonical fixture
    assert_eq!(
        xml_bytes, expected_canonical,
        "Generated XML must match canonical fixture byte-for-byte (DTE-01)"
    );

    // Verify XML as string
    let xml_str = String::from_utf8(xml_bytes).expect("valid UTF-8 XML");
    assert!(xml_str.starts_with("<DTE version=\"1.0\"><Documento ID=\"F60T33\">"));
    assert!(xml_str.contains("<TipoDTE>33</TipoDTE>"));
    assert!(xml_str.contains("<Folio>60</Folio>"));
    assert!(xml_str.contains("<FchEmis>2026-10-21</FchEmis>"));
    assert!(xml_str.contains("<RUTEmisor>76086428-5</RUTEmisor>"));
    assert!(xml_str.contains("<RUTRecep>77777777-7</RUTRecep>"));
    assert!(xml_str.contains("<MntNeto>100000</MntNeto>"));
    assert!(xml_str.contains("<TasaIVA>19</TasaIVA>"));
    assert!(xml_str.contains("<IVA>19000</IVA>"));
    assert!(xml_str.contains("<MntTotal>119000</MntTotal>"));
    assert!(xml_str.contains("<NroLinDet>1</NroLinDet>"));
    assert!(xml_str.ends_with("</Documento></DTE>"));
}

#[test]
fn test_serialize_dte_34_against_fixture() {
    let fixture_raw = include_bytes!("fixtures/dte-34.xml");
    let expected_canonical = canonicalize_xml(fixture_raw).unwrap();

    let emisor_exento = Emisor::new(
        Rut::parse("76.086.428-5").expect("valid RUT"),
        "SERVICIOS DE CAPACITACION EJEMPLO SPA",
        "CAPACITACION Y EDUCACION PROFESIONAL",
        854_900,
        "AVENIDA LIBERTADOR 1234",
        "SANTIAGO",
    )
    .ciudad_origen("SANTIAGO");

    let dte = Dte::factura_exenta()
        .folio(100)
        .fch_emis_str("2026-10-21")
        .expect("valid date")
        .emisor(emisor_exento)
        .receptor(sample_receptor().unwrap())
        .item(
            Detalle::new(
                1,
                "Curso de capacitacion tecnica exento de IVA",
                Monto::from_pesos(100_000),
            )
            .codigo("CURSO", "CAP-01")
            .qty(2)
            .prc(Monto::from_pesos(50_000)),
        )
        .build()
        .expect("build factura exenta must succeed");

    assert_eq!(dte.tipo(), TipoDte::FacturaExenta);
    assert_eq!(dte.folio(), 100);
    assert_eq!(dte.id(), "F100T34");
    assert_eq!(dte.neto(), None);
    assert_eq!(dte.iva(), None);
    assert_eq!(dte.exento(), Some(Monto::from_pesos(100_000)));
    assert_eq!(dte.total(), Monto::from_pesos(100_000));

    let xml_bytes = dte.to_xml().expect("to_xml must succeed");

    // Byte-for-byte exact comparison against canonical fixture
    assert_eq!(
        xml_bytes, expected_canonical,
        "Generated XML must match canonical fixture byte-for-byte (DTE-01)"
    );

    let xml_str = String::from_utf8(xml_bytes).expect("valid UTF-8 XML");
    assert!(xml_str.starts_with("<DTE version=\"1.0\"><Documento ID=\"F100T34\">"));
    assert!(xml_str.contains("<TipoDTE>34</TipoDTE>"));
    assert!(xml_str.contains("<Folio>100</Folio>"));
    assert!(xml_str.contains("<MntExe>100000</MntExe>"));
    assert!(xml_str.contains("<MntTotal>100000</MntTotal>"));
    assert!(xml_str.contains("<IndExe>1</IndExe>"));

    // Exenta must NEVER contain IVA or MntNeto tags (DTE-02)
    assert!(!xml_str.contains("<MntNeto>"));
    assert!(!xml_str.contains("<TasaIVA>"));
    assert!(!xml_str.contains("<IVA>"));
}

#[test]
fn test_factura_afecta_iva_exact_rounding() {
    let cases = [
        (100, 19, 119),
        (1, 0, 1),
        (3, 1, 4),
        (8, 2, 10),
        (15, 3, 18),
        (25, 5, 30),
        (1_000, 190, 1_190),
        (5_000, 950, 5_950),
        (123_456, 23_457, 146_913),
    ];

    for (neto, expected_iva, expected_total) in cases {
        let dte = Dte::factura_afecta()
            .folio(1)
            .fch_emis_str("2026-11-21")
            .unwrap()
            .emisor(sample_emisor().unwrap())
            .receptor(sample_receptor().unwrap())
            .item(Detalle::new(1, "Item", Monto::from_pesos(neto)))
            .build()
            .unwrap();

        assert_eq!(dte.neto(), Some(Monto::from_pesos(neto)));
        assert_eq!(dte.iva(), Some(Monto::from_pesos(expected_iva)));
        assert_eq!(dte.total(), Monto::from_pesos(expected_total));

        let xml = String::from_utf8(dte.to_xml().unwrap()).unwrap();
        assert!(xml.contains(&format!("<MntNeto>{neto}</MntNeto>")));
        assert!(xml.contains(&format!("<IVA>{expected_iva}</IVA>")));
        assert!(xml.contains(&format!("<MntTotal>{expected_total}</MntTotal>")));
    }
}

#[test]
fn test_factura_afecta_with_mixed_items() {
    let dte = Dte::factura_afecta()
        .folio(5)
        .fch_emis_str("2026-11-21")
        .unwrap()
        .emisor(sample_emisor().unwrap())
        .receptor(sample_receptor().unwrap())
        .item(Detalle::new(1, "Item Afecto", Monto::from_pesos(100_000)))
        .item(Detalle::new(2, "Item Exento", Monto::from_pesos(50_000)).ind_exe(1))
        .build()
        .unwrap();

    assert_eq!(dte.neto(), Some(Monto::from_pesos(100_000)));
    assert_eq!(dte.exento(), Some(Monto::from_pesos(50_000)));
    assert_eq!(dte.iva(), Some(Monto::from_pesos(19_000)));
    assert_eq!(dte.total(), Monto::from_pesos(169_000));

    let xml = String::from_utf8(dte.to_xml().unwrap()).unwrap();
    assert!(xml.contains("<MntNeto>100000</MntNeto>"));
    assert!(xml.contains("<MntExe>50000</MntExe>"));
    assert!(xml.contains("<TasaIVA>19</TasaIVA>"));
    assert!(xml.contains("<IVA>19000</IVA>"));
    assert!(xml.contains("<MntTotal>169000</MntTotal>"));
}

#[test]
fn test_builder_rejects_invalid_rut() {
    assert!(matches!(Rut::parse("12.345.678-9"), Err(Error::InvalidRut)));
    assert!(matches!(Rut::parse("76.123.456-7"), Err(Error::InvalidRut)));
    assert!(matches!(Rut::parse("not-a-rut"), Err(Error::InvalidRut)));
}

#[test]
fn test_builder_validation_errors() {
    // Missing folio
    let err = Dte::factura_afecta()
        .fch_emis_str("2026-11-21")
        .unwrap()
        .emisor(sample_emisor().unwrap())
        .receptor(sample_receptor().unwrap())
        .item(Detalle::new(1, "Item", Monto::from_pesos(100)))
        .build();
    assert!(matches!(err, Err(Error::DteValidation(_))));

    // Zero folio
    let err = Dte::factura_afecta()
        .folio(0)
        .fch_emis_str("2026-11-21")
        .unwrap()
        .emisor(sample_emisor().unwrap())
        .receptor(sample_receptor().unwrap())
        .item(Detalle::new(1, "Item", Monto::from_pesos(100)))
        .build();
    assert!(matches!(err, Err(Error::DteValidation(_))));

    // Missing emission date
    let err = Dte::factura_afecta()
        .folio(1)
        .emisor(sample_emisor().unwrap())
        .receptor(sample_receptor().unwrap())
        .item(Detalle::new(1, "Item", Monto::from_pesos(100)))
        .build();
    assert!(matches!(err, Err(Error::DteValidation(_))));

    // Missing emisor
    let err = Dte::factura_afecta()
        .folio(1)
        .fch_emis_str("2026-11-21")
        .unwrap()
        .receptor(sample_receptor().unwrap())
        .item(Detalle::new(1, "Item", Monto::from_pesos(100)))
        .build();
    assert!(matches!(err, Err(Error::DteValidation(_))));

    // Missing receptor
    let err = Dte::factura_afecta()
        .folio(1)
        .fch_emis_str("2026-11-21")
        .unwrap()
        .emisor(sample_emisor().unwrap())
        .item(Detalle::new(1, "Item", Monto::from_pesos(100)))
        .build();
    assert!(matches!(err, Err(Error::DteValidation(_))));

    // Empty details
    let err = Dte::factura_afecta()
        .folio(1)
        .fch_emis_str("2026-11-21")
        .unwrap()
        .emisor(sample_emisor().unwrap())
        .receptor(sample_receptor().unwrap())
        .build();
    assert!(matches!(err, Err(Error::DteValidation(_))));

    // Inconsistent line numbers
    let err = Dte::factura_afecta()
        .folio(1)
        .fch_emis_str("2026-11-21")
        .unwrap()
        .emisor(sample_emisor().unwrap())
        .receptor(sample_receptor().unwrap())
        .item(Detalle::new(2, "Item", Monto::from_pesos(100)))
        .build();
    assert!(matches!(err, Err(Error::DteValidation(_))));

    // Empty item description
    let err = Dte::factura_afecta()
        .folio(1)
        .fch_emis_str("2026-11-21")
        .unwrap()
        .emisor(sample_emisor().unwrap())
        .receptor(sample_receptor().unwrap())
        .item(Detalle::new(1, "   ", Monto::from_pesos(100)))
        .build();
    assert!(matches!(err, Err(Error::DteValidation(_))));

    // Negative line amount
    let err = Dte::factura_afecta()
        .folio(1)
        .fch_emis_str("2026-11-21")
        .unwrap()
        .emisor(sample_emisor().unwrap())
        .receptor(sample_receptor().unwrap())
        .item(Detalle::new(1, "Item", Monto::from_pesos(-100)))
        .build();
    assert!(matches!(err, Err(Error::DteValidation(_))));

    // Factura afecta with zero neto
    let err = Dte::factura_afecta()
        .folio(1)
        .fch_emis_str("2026-11-21")
        .unwrap()
        .emisor(sample_emisor().unwrap())
        .receptor(sample_receptor().unwrap())
        .item(Detalle::new(1, "Item", Monto::from_pesos(0)))
        .build();
    assert!(matches!(err, Err(Error::DteValidation(_))));

    // Factura exenta with zero total
    let err = Dte::factura_exenta()
        .folio(1)
        .fch_emis_str("2026-11-21")
        .unwrap()
        .emisor(sample_emisor().unwrap())
        .receptor(sample_receptor().unwrap())
        .item(Detalle::new(1, "Item", Monto::from_pesos(0)))
        .build();
    assert!(matches!(err, Err(Error::DteValidation(_))));
}

#[test]
fn test_monto_no_f64_integer_precision() {
    let large_neto = Monto::from_pesos(999_999_999);
    let dte = Dte::factura_afecta()
        .folio(1)
        .fch_emis_str("2026-11-21")
        .unwrap()
        .emisor(sample_emisor().unwrap())
        .receptor(sample_receptor().unwrap())
        .item(Detalle::new(1, "Heavy Equipment", large_neto))
        .build()
        .unwrap();

    let expected_iva = Monto::from_pesos(190_000_000); // round(999999999 * 0.19) = 189999999.81 -> 190000000
    assert_eq!(dte.iva(), Some(expected_iva));
    assert_eq!(dte.total(), Monto::from_pesos(1_189_999_999));
}

#[test]
fn test_quick_xml_roundtrip_verification() {
    let dte = Dte::factura_afecta()
        .folio(60)
        .fch_emis_str("2026-10-21")
        .unwrap()
        .emisor(sample_emisor().unwrap())
        .receptor(sample_receptor().unwrap())
        .item(
            Detalle::new(1, "Item", Monto::from_pesos(100_000))
                .codigo("INT1", "001")
                .qty(1)
                .prc(Monto::from_pesos(100_000)),
        )
        .build()
        .unwrap();

    let xml = dte.to_xml().unwrap();
    let mut reader = Reader::from_reader(xml.as_slice());
    reader.config_mut().trim_text(true);

    let mut element_names = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8(e.name().as_ref().to_vec()).unwrap();
                element_names.push(name);
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => panic!("parsing XML error: {e}"),
        }
        buf.clear();
    }

    assert_eq!(
        element_names,
        vec![
            "DTE",
            "Documento",
            "Encabezado",
            "IdDoc",
            "TipoDTE",
            "Folio",
            "FchEmis",
            "Emisor",
            "RUTEmisor",
            "RznSoc",
            "GiroEmis",
            "Acteco",
            "DirOrigen",
            "CmnaOrigen",
            "CiudadOrigen",
            "Receptor",
            "RUTRecep",
            "RznSocRecep",
            "GiroRecep",
            "DirRecep",
            "CmnaRecep",
            "CiudadRecep",
            "Totales",
            "MntNeto",
            "TasaIVA",
            "IVA",
            "MntTotal",
            "Detalle",
            "NroLinDet",
            "CdgItem",
            "TpoCodigo",
            "VlrCodigo",
            "NmbItem",
            "QtyItem",
            "PrcItem",
            "MontoItem",
        ]
    );
}

#[cfg(feature = "serde")]
#[test]
fn test_serde_json_roundtrip() {
    let dte = Dte::factura_afecta()
        .folio(60)
        .fch_emis_str("2026-10-21")
        .unwrap()
        .emisor(sample_emisor().unwrap())
        .receptor(sample_receptor().unwrap())
        .item(Detalle::new(1, "Item", Monto::from_pesos(100_000)))
        .build()
        .unwrap();

    let json = serde_json::to_string(&dte).expect("serialize to JSON must succeed");
    let deserialized: Dte =
        serde_json::from_str(&json).expect("deserialize from JSON must succeed");

    assert_eq!(dte, deserialized);
}
