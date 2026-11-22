//! Integration tests for CAF parsing, validation, and TED signing (CAF-01).

use rsa::RsaPublicKey;
use rsa::pkcs1::DecodeRsaPublicKey;
use rsa::pkcs1v15::VerifyingKey;
use rsa::signature::Verifier;
use sha1::Sha1;
use sii::caf::Caf;
use sii::codes::TipoDte;
use sii::error::Error;
use std::path::{Path, PathBuf};

fn find_fixture_path(filename: &str) -> PathBuf {
    let candidate1 = Path::new("tests/fixtures").join(filename);
    if candidate1.exists() {
        return candidate1;
    }
    Path::new("crates/sii/tests/fixtures").join(filename)
}

#[test]
fn test_fixture_caf_33_parse_and_fields() {
    let xml_path = find_fixture_path("caf-33.xml");
    let xml_bytes = std::fs::read(&xml_path).expect("read caf-33.xml");

    let caf = Caf::parse(&xml_bytes).expect("parse CAF");
    assert_eq!(caf.rango(), (1, 100));
    assert_eq!(caf.tipo(), TipoDte::FacturaAfecta);
    assert_eq!(caf.emisor_rut().to_string(), "76086428-5");
    assert_eq!(caf.razon_social(), "EMPRESA DE PRUEBA SPA");
    assert_eq!(caf.fecha_autorizacion(), "2026-01-01");
    assert_eq!(caf.idk(), "100");
    assert_eq!(caf.algoritmo_firma(), "SHA1withRSA");
    assert!(caf.public_key_pem().is_some());
    assert_ne!(caf.raw_da().len(), 0);
    assert_ne!(caf.raw_xml().len(), 0);
}

#[test]
fn test_fixture_caf_33_folio_range_check() {
    let xml_path = find_fixture_path("caf-33.xml");
    let xml_bytes = std::fs::read(&xml_path).expect("read caf-33.xml");
    let caf = Caf::parse(&xml_bytes).expect("parse CAF");

    // Containment checks
    assert!(caf.contiene(1));
    assert!(caf.contiene(50));
    assert!(caf.contiene(100));
    assert!(!caf.contiene(0));
    assert!(!caf.contiene(101));
    assert!(!caf.contiene(9999));

    // Result verification checks
    assert!(caf.verificar_folio(1).is_ok());
    assert!(caf.verificar_folio(50).is_ok());
    assert!(caf.verificar_folio(100).is_ok());

    let err_under = caf.verificar_folio(0).unwrap_err();
    assert_eq!(
        err_under,
        Error::FolioOutOfRange {
            folio: 0,
            min: 1,
            max: 100,
        }
    );

    let err_over = caf.verificar_folio(101).unwrap_err();
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
fn test_fixture_caf_33_signature_verification() {
    let xml_path = find_fixture_path("caf-33.xml");
    let pub_path = find_fixture_path("sii-test-pubkey.pub");

    let xml_bytes = std::fs::read(&xml_path).expect("read caf-33.xml");
    let pub_pem = std::fs::read_to_string(&pub_path).expect("read pubkey");
    let sii_pub = RsaPublicKey::from_pkcs1_pem(pub_pem.trim()).expect("parse sii pubkey");

    let mut caf = Caf::parse(&xml_bytes).expect("parse CAF");
    assert!(!caf.is_firma_verificada());

    caf.verificar_firma(&sii_pub).expect("verify signature");
    assert!(caf.is_firma_verificada());

    // Also test parse_and_verify convenience constructor
    let verified_caf = Caf::parse_and_verify(&xml_bytes, &sii_pub).expect("parse and verify");
    assert!(verified_caf.is_firma_verificada());
}

#[test]
fn test_fixture_caf_33_tampered_fails() {
    let xml_path = find_fixture_path("caf-33.xml");
    let pub_path = find_fixture_path("sii-test-pubkey.pub");

    let xml_bytes = std::fs::read(&xml_path).expect("read caf-33.xml");
    let pub_pem = std::fs::read_to_string(&pub_path).expect("read pubkey");
    let sii_pub = RsaPublicKey::from_pkcs1_pem(pub_pem.trim()).expect("parse sii pubkey");

    // Tamper DA range
    let xml_str = String::from_utf8(xml_bytes).expect("utf8");
    let tampered_xml = xml_str.replace("<H>100</H>", "<H>500</H>");
    let mut caf = Caf::parse(tampered_xml.as_bytes()).expect("parse without verification");
    let result = caf.verificar_firma(&sii_pub);
    assert!(result.is_err());
    assert!(!caf.is_firma_verificada());

    // Tamper FRMA signature
    let tampered_sig = xml_str
        .replace("<FRMA", "<FRMA_OLD")
        .replace("</FRMA>", "</FRMA_OLD>");
    let tampered_sig = format!("{tampered_sig}<FRMA algoritmo=\"SHA1withRSA\">AAAA</FRMA>");
    let mut caf2 = Caf::parse(tampered_sig.as_bytes()).expect("parse without verification");
    assert!(caf2.verificar_firma(&sii_pub).is_err());
}

#[test]
fn test_fixture_caf_33_ted_signing() {
    let xml_path = find_fixture_path("caf-33.xml");
    let xml_bytes = std::fs::read(&xml_path).expect("read caf-33.xml");
    let caf = Caf::parse(&xml_bytes).expect("parse CAF");

    let dd_test =
        b"<DD><RE>76086428-5</RE><TD>33</TD><F>1</F><FE>2026-01-01</FE><MNT>10000</MNT></DD>";
    let sig_bytes = caf.sign_ted(dd_test).expect("sign TED");
    assert_ne!(sig_bytes.len(), 0);

    // Signer trait produces identical signature
    let signer_sig = caf.signer().sign(dd_test).expect("signer trait");
    assert_eq!(sig_bytes, signer_sig);

    // Verify signature using the public key from the CAF
    let verifying_key = VerifyingKey::<Sha1>::new(caf.public_key().clone());
    let sig = rsa::pkcs1v15::Signature::try_from(sig_bytes.as_slice()).expect("parse signature");
    verifying_key
        .verify(dd_test, &sig)
        .expect("verify signature");
}

#[test]
fn test_fixture_caf_33_debug_redaction() {
    let xml_path = find_fixture_path("caf-33.xml");
    let xml_bytes = std::fs::read(&xml_path).expect("read caf-33.xml");
    let caf = Caf::parse(&xml_bytes).expect("parse CAF");

    let debug_str = format!("{caf:?}");
    assert!(debug_str.contains("private_key: \"<redacted>\""));
    assert!(!debug_str.contains("BEGIN RSA PRIVATE KEY"));
    assert!(!debug_str.contains("MIICXQIBAAKBgQ"));
}
