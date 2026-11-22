//! Integration tests for `XMLDSig` digital signature, canonicalization (C14N), and verification.

use rsa::RsaPrivateKey;
use rsa::pkcs1v15::SigningKey;
use rsa::rand_core::OsRng;
use rsa::signature::{SignatureEncoding, Signer as _};
use rsa::traits::PublicKeyParts;
use rsa::RsaPublicKey;
use sha1::Sha1;
use sha2::Sha256;
use sii::Result;
use sii::signature::{
    Certificate, DigestAlgorithm, SignatureAlgorithm, SignatureOptions, Signer, X509Certificate,
    sign_dte, sign_dte_with_options, verify,
};

struct FileKeySigner {
    private_key: RsaPrivateKey,
    algo: SignatureAlgorithm,
}

impl Signer for FileKeySigner {
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self.algo {
            SignatureAlgorithm::RsaSha1 => {
                let key = SigningKey::<Sha1>::new(self.private_key.clone());
                Ok(key.sign(data).to_vec())
            }
            SignatureAlgorithm::RsaSha256 => {
                let key = SigningKey::<Sha256>::new(self.private_key.clone());
                Ok(key.sign(data).to_vec())
            }
        }
    }
}

fn generate_test_key_and_cert() -> (FileKeySigner, X509Certificate) {
    let mut rng = OsRng;
    let priv_key = RsaPrivateKey::new(&mut rng, 2048).expect("generate test rsa key");
    let pub_key = RsaPublicKey::from(&priv_key);

    let cert = X509Certificate::from_parts(vec![0x30, 0x03, 0x02, 0x01, 0x01], pub_key);

    let mock_signer = FileKeySigner {
        private_key: priv_key,
        algo: SignatureAlgorithm::RsaSha1,
    };

    (mock_signer, cert)
}

#[test]
fn test_fixture_cert_parsing_and_public_key() {
    let cert_pem = include_str!("fixtures/test-cert.txt");
    let cert = X509Certificate::from_pem(cert_pem).expect("parse cert pem");

    assert_ne!(cert.raw_der(), &[]);
    let pub_key = cert.public_key().expect("extract public key");
    assert_eq!(pub_key.size(), 256); // 2048-bit RSA key
}

#[test]
fn test_sign_dte_reproducible_and_verifiable() {
    let (mock_signer, cert) = generate_test_key_and_cert();
    let dte_xml = include_bytes!("fixtures/dte-33.xml");

    let output_first = sign_dte(dte_xml, &mock_signer, &cert).expect("sign_dte first time");
    let output_second = sign_dte(dte_xml, &mock_signer, &cert).expect("sign_dte second time");

    // Must be completely reproducible with fixed key and document (SIG-01)
    assert_eq!(
        output_first, output_second,
        "signing same document must be deterministic"
    );

    // Verification must pass (SIG-02, RF-03)
    let ok = verify(&output_first).expect("verify signed dte");
    assert!(ok, "signature verification must succeed");
}

#[test]
fn test_verify_fixture_signed_dte() {
    let fixture_signed = include_bytes!("fixtures/dte-33-signed.xml");
    let ok = verify(fixture_signed).expect("verify fixture signed dte");
    assert!(ok, "fixture signed dte must verify successfully");
}

#[test]
fn test_tamper_payload_detection() {
    let (mock_signer, cert) = generate_test_key_and_cert();
    let dte_xml = include_bytes!("fixtures/dte-33.xml");
    let signed_bytes = sign_dte(dte_xml, &mock_signer, &cert).expect("sign_dte");

    let signed_str = String::from_utf8(signed_bytes).expect("utf8");

    // 1. Alter Folio
    let tampered_folio = signed_str.replace("<Folio>60</Folio>", "<Folio>61</Folio>");
    assert!(
        !verify(tampered_folio.as_bytes()).expect("verify"),
        "altered Folio must fail"
    );

    // 2. Alter Net amount
    let tampered_monto =
        signed_str.replace("<MntNeto>100000</MntNeto>", "<MntNeto>200000</MntNeto>");
    assert!(
        !verify(tampered_monto.as_bytes()).expect("verify"),
        "altered MntNeto must fail"
    );

    // 3. Alter Item description
    let tampered_item = signed_str.replace("Servidor de aplicaciones", "Servidor alterado");
    assert!(
        !verify(tampered_item.as_bytes()).expect("verify"),
        "altered Item must fail"
    );
}

#[test]
fn test_tamper_signature_value_detection() {
    let (mock_signer, cert) = generate_test_key_and_cert();
    let dte_xml = include_bytes!("fixtures/dte-33.xml");
    let signed_bytes = sign_dte(dte_xml, &mock_signer, &cert).expect("sign_dte");
    let signed_str = String::from_utf8(signed_bytes).expect("utf8");

    // Tamper SignatureValue
    let (head, tail) = signed_str.split_once("<SignatureValue>").unwrap();
    let (sig_body, rest) = tail.split_once("</SignatureValue>").unwrap();
    let mut modified_sig = sig_body.to_string();
    if let Some(c) = modified_sig.chars().next() {
        let replacement = if c == 'A' { 'B' } else { 'A' };
        modified_sig.replace_range(..1, &replacement.to_string());
    }
    let tampered_sig_doc = format!("{head}<SignatureValue>{modified_sig}</SignatureValue>{rest}");
    assert!(
        !verify(tampered_sig_doc.as_bytes()).expect("verify"),
        "altered SignatureValue must fail"
    );
}

#[test]
fn test_tamper_digest_value_detection() {
    let (mock_signer, cert) = generate_test_key_and_cert();
    let dte_xml = include_bytes!("fixtures/dte-33.xml");
    let signed_bytes = sign_dte(dte_xml, &mock_signer, &cert).expect("sign_dte");
    let signed_str = String::from_utf8(signed_bytes).expect("utf8");

    // Tamper DigestValue
    let (head, tail) = signed_str.split_once("<DigestValue>").unwrap();
    let (digest_body, rest) = tail.split_once("</DigestValue>").unwrap();
    let mut modified_digest = digest_body.to_string();
    if let Some(c) = modified_digest.chars().next() {
        let replacement = if c == 'A' { 'B' } else { 'A' };
        modified_digest.replace_range(..1, &replacement.to_string());
    }
    let tampered_digest_doc = format!("{head}<DigestValue>{modified_digest}</DigestValue>{rest}");
    assert!(
        !verify(tampered_digest_doc.as_bytes()).expect("verify"),
        "altered DigestValue must fail"
    );
}

#[test]
fn test_c14n_insensitivity_to_irrelevant_whitespace() {
    let (mock_signer, cert) = generate_test_key_and_cert();
    let dte_xml = include_bytes!("fixtures/dte-33.xml");
    let signed_bytes = sign_dte(dte_xml, &mock_signer, &cert).expect("sign_dte");
    let signed_str = String::from_utf8(signed_bytes).expect("utf8");

    // Add irrelevant whitespace inside tags (which C14N normalizes)
    let reformatted = signed_str
        .replace(
            "<Documento ID=\"F60T33\">",
            "<Documento    ID=\"F60T33\"   >",
        )
        .replace("<IdDoc>", "<IdDoc   >")
        .replace("<Totales>", "<Totales >");

    assert!(
        verify(reformatted.as_bytes()).expect("verify reformatted"),
        "reformatting whitespace inside tags must still pass verification due to C14N"
    );
}

#[test]
fn test_sha256_signature_algorithm() {
    let (mut mock_signer, cert) = generate_test_key_and_cert();
    mock_signer.algo = SignatureAlgorithm::RsaSha256;

    let dte_xml = include_bytes!("fixtures/dte-33.xml");
    let options = SignatureOptions {
        signature_algorithm: SignatureAlgorithm::RsaSha256,
        digest_algorithm: DigestAlgorithm::Sha256,
    };

    let signed_bytes =
        sign_dte_with_options(dte_xml, &mock_signer, &cert, options).expect("sign sha256");
    let ok = verify(&signed_bytes).expect("verify sha256");
    assert!(ok, "SHA-256 XMLDSig must verify successfully");
}
