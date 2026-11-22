//! Integration tests for Chilean SII authentication: seed parsing, `XMLDSig` signing, and token management (AUTH-01).

use rsa::RsaPrivateKey;
use rsa::RsaPublicKey;
use rsa::pkcs1v15::SigningKey;
use rsa::rand_core::OsRng;
use rsa::signature::{SignatureEncoding, Signer as _};
use sha1::Sha1;
use sii::Result;
use sii::auth::{RespuestaSemilla, RespuestaToken, Semilla, Token, firmar_semilla};
use sii::signature::{Signer, X509Certificate, verify};

struct MockKeySigner {
    private_key: RsaPrivateKey,
}

impl Signer for MockKeySigner {
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
        let key = SigningKey::<Sha1>::new(self.private_key.clone());
        Ok(key.sign(data).to_vec())
    }
}

fn generate_test_signer_and_cert()
-> std::result::Result<(MockKeySigner, X509Certificate), Box<dyn std::error::Error>> {
    let mut rng = OsRng;
    let priv_key = RsaPrivateKey::new(&mut rng, 2048)?;
    let pub_key = RsaPublicKey::from(&priv_key);
    let cert = X509Certificate::from_parts(vec![0x30, 0x03, 0x02, 0x01, 0x01], pub_key);
    let signer = MockKeySigner {
        private_key: priv_key,
    };
    Ok((signer, cert))
}

#[test]
fn test_parse_semilla_fixture_ok() {
    let xml = include_bytes!("fixtures/semilla-resp-ok.xml");
    let resp = RespuestaSemilla::parse(xml).expect("parse seed response ok");

    assert!(resp.is_ok());
    assert_eq!(resp.estado, "00");
    assert_eq!(resp.glosa, "Operacion Exitosa");

    let semilla = resp.semilla().expect("get seed reference");
    assert_eq!(semilla.valor(), "0079009845665");
    assert_eq!(semilla.as_str(), "0079009845665");
    assert_eq!(format!("{semilla}"), "0079009845665");

    // Also parse directly with Semilla::parse_xml
    let direct_semilla = Semilla::parse_xml(xml).expect("direct seed parse");
    assert_eq!(direct_semilla.valor(), "0079009845665");
}

#[test]
fn test_parse_semilla_fixture_error() {
    let xml = include_bytes!("fixtures/semilla-resp-error.xml");
    let resp = RespuestaSemilla::parse(xml).expect("parse seed response error");

    assert!(!resp.is_ok());
    assert_eq!(resp.estado, "-1");
    assert_eq!(resp.glosa, "Error interno de base de datos");
    assert!(resp.semilla.is_none());

    let err = resp.semilla().expect_err("should return error on failure");
    match err {
        sii::Error::Auth(msg) => {
            assert!(msg.contains("-1"));
            assert!(msg.contains("Error interno de base de datos"));
        }
        other => panic!("expected Error::Auth, got {other:?}"),
    }

    // Direct parse must also fail
    assert!(Semilla::parse_xml(xml).is_err());
}

#[test]
fn test_parse_token_fixture_ok() {
    let xml = include_bytes!("fixtures/token-resp-ok.xml");
    let resp = RespuestaToken::parse(xml).expect("parse token response ok");

    assert!(resp.is_ok());
    assert_eq!(resp.estado, "00");
    assert_eq!(resp.glosa, "Token Creado");

    let token = resp.token().expect("get token reference");
    assert_eq!(token.valor(), "AG12345678ABCD90");
    assert_eq!(token.as_str(), "AG12345678ABCD90");

    // Direct parse
    let direct_token = Token::parse_xml(xml).expect("direct token parse");
    assert_eq!(direct_token.valor(), "AG12345678ABCD90");
}

#[test]
fn test_parse_token_fixture_error() {
    let xml = include_bytes!("fixtures/token-resp-error.xml");
    let resp = RespuestaToken::parse(xml).expect("parse token response error");

    assert!(!resp.is_ok());
    assert_eq!(resp.estado, "11");
    assert_eq!(resp.glosa, "Firma invalida o certificado no autorizado");
    assert!(resp.token.is_none());

    let err = resp.token().expect_err("should return error on failure");
    match err {
        sii::Error::Auth(msg) => {
            assert!(msg.contains("11"));
            assert!(msg.contains("Firma invalida"));
        }
        other => panic!("expected Error::Auth, got {other:?}"),
    }

    assert!(Token::parse_xml(xml).is_err());
}

#[test]
fn test_token_confidentiality_redaction_in_debug_and_display() {
    // SEC-02: Secret token value must never appear in Debug or Display
    let secret_val = "SECRET_TOKEN_XYZ_987654";
    let token = Token::new(secret_val);

    let debug_output = format!("{token:?}");
    assert!(
        !debug_output.contains(secret_val),
        "Debug output must not reveal secret token value"
    );
    assert!(debug_output.contains("[REDACTED]"));

    let display_output = format!("{token}");
    assert!(
        !display_output.contains(secret_val),
        "Display output must not reveal secret token value"
    );
    assert!(display_output.contains("[REDACTED]"));

    // Ensure raw access is still available when intentionally needed
    assert_eq!(token.as_str(), secret_val);
    assert_eq!(token.valor(), secret_val);

    // Also verify RespuestaToken Debug redaction
    let resp = RespuestaToken {
        estado: "00".to_string(),
        glosa: "OK".to_string(),
        token: Some(token),
    };
    let resp_debug = format!("{resp:?}");
    assert!(
        !resp_debug.contains(secret_val),
        "RespuestaToken Debug must not leak secret token value"
    );
}

#[test]
fn test_firmar_semilla_reproducible_and_verifiable() {
    let (signer, cert) = generate_test_signer_and_cert().expect("generate test keys");
    let semilla = Semilla::new("0079009845665");

    let solicitud1 = firmar_semilla(&semilla, &signer, &cert).expect("firmar semilla first time");
    let solicitud2 = firmar_semilla(&semilla, &signer, &cert).expect("firmar semilla second time");

    // Must be completely reproducible with same key and seed (RF-02, SIG-01)
    assert_eq!(
        solicitud1.as_bytes(),
        solicitud2.as_bytes(),
        "signing same seed must be deterministic and reproducible"
    );

    // Verify XML structure
    let xml_str = solicitud1.xml();
    assert!(xml_str.starts_with("<getToken><item><Semilla>0079009845665</Semilla></item>"));
    assert!(xml_str.ends_with("</Signature></getToken>"));
    assert!(xml_str.contains("<Reference URI=\"\">"));
    assert!(xml_str.contains("http://www.w3.org/2000/09/xmldsig#enveloped-signature"));
    assert!(xml_str.contains("http://www.w3.org/2000/09/xmldsig#rsa-sha1"));

    // Signature verification must pass (SIG-02, RF-02)
    let ok = solicitud1.verify().expect("verify via SolicitudToken");
    assert!(ok, "signature must verify successfully via SolicitudToken");

    let ok_direct = verify(solicitud1.as_bytes()).expect("verify via signature::verify");
    assert!(
        ok_direct,
        "signature must verify successfully via signature::verify"
    );

    // Also verify Semilla::firmar method
    let solicitud_method = semilla.firmar(&signer, &cert).expect("semilla.firmar()");
    assert_eq!(solicitud_method.as_bytes(), solicitud1.as_bytes());
}

#[test]
fn test_firmar_semilla_tampered_fails_verification() {
    let (signer, cert) = generate_test_signer_and_cert().expect("generate test keys");
    let semilla = Semilla::new("0079009845665");
    let solicitud = firmar_semilla(&semilla, &signer, &cert).expect("firmar semilla");

    // 1. Tamper payload (modify seed number)
    let tampered_seed_xml = solicitud.xml().replace(
        "<Semilla>0079009845665</Semilla>",
        "<Semilla>0079009845666</Semilla>",
    );
    let ok = verify(tampered_seed_xml.as_bytes()).expect("verify tampered seed");
    assert!(!ok, "tampered seed must fail signature verification");

    // 2. Tamper SignatureValue
    let (head, tail) = solicitud.xml().split_once("<SignatureValue>").unwrap();
    let (sig_content, rest) = tail.split_once("</SignatureValue>").unwrap();
    let mut modified_sig = sig_content.to_string();
    if let Some(c) = modified_sig.chars().next() {
        let repl = if c == 'A' { 'B' } else { 'A' };
        modified_sig.replace_range(..1, &repl.to_string());
    }
    let tampered_sig_xml = format!("{head}<SignatureValue>{modified_sig}</SignatureValue>{rest}");
    let ok = verify(tampered_sig_xml.as_bytes()).expect("verify tampered sig");
    assert!(!ok, "tampered signature value must fail verification");
}

#[test]
fn test_firmar_empty_seed_fails() {
    let (signer, cert) = generate_test_signer_and_cert().expect("generate test keys");
    let empty_semilla = Semilla::new("");
    let res = firmar_semilla(&empty_semilla, &signer, &cert);
    assert!(res.is_err(), "signing empty seed must return error");
}

#[test]
fn test_malformed_xml_responses() {
    let invalid_xml = b"<INVALID><UNCLOSED>";
    assert!(RespuestaSemilla::parse(invalid_xml).is_err());
    assert!(RespuestaToken::parse(invalid_xml).is_err());

    let missing_estado = b"<RESPUESTA><RESP_HDR><GLOSA>No estado</GLOSA></RESP_HDR></RESPUESTA>";
    assert!(RespuestaSemilla::parse(missing_estado).is_err());
    assert!(RespuestaToken::parse(missing_estado).is_err());
}
