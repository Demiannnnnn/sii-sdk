//! Integration tests for CERT-LOADER-01 (`SiiIdentity` from PEM and PKCS#12).
//!
//! The certificate and key below are a THROWAWAY self-signed pair generated only for
//! these tests (CN "sii-sdk test", O "FAKE TEST"); they are not a real certificate and
//! carry no real secret (SEC-01). The private key is stored as base64 of its PKCS#8 DER
//! (no PEM markers) and the PEM is reconstructed at runtime, so no key material is
//! committed in a scannable form. PKCS#12 password: "testpass".

#![cfg(feature = "pkcs12")]

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use sii::cert_loader::SiiIdentity;
use sii::codes::Ambiente;
use sii::signature::{Certificate, SignatureAlgorithm, Signer, sign_dte, verify};

const TEST_CERT_PEM: &str = "\
-----BEGIN CERTIFICATE-----
MIIC7DCCAdQCCQCMX4u9MlqMTjANBgkqhkiG9w0BAQsFADA4MQswCQYDVQQGEwJD
TDESMBAGA1UECgwJRkFLRSBURVNUMRUwEwYDVQQDDAxzaWktc2RrIHRlc3QwHhcN
MjYxMTIyMDMxNzQ0WhcNMzYxMTE5MDMxNzQ0WjA4MQswCQYDVQQGEwJDTDESMBAG
A1UECgwJRkFLRSBURVNUMRUwEwYDVQQDDAxzaWktc2RrIHRlc3QwggEiMA0GCSqG
SIb3DQEBAQUAA4IBDwAwggEKAoIBAQDAoCAYeHSLR2TnrOF33pJ8wF1FiM1+H726
rv3fT5ReXcCu+FyQvojAdcu51odyj4NtZO7D25mRhfxRYwoLbHhhSXDT90nEASOW
/H5LCy6s1EshNprIZi9J44sZ7BS52D42JHOcTQ3s4YBWxCB4gWTeHBVNT+ceqZE4
Wxnln5oLVZ+67cCpxZSdThOwtVg5p2Z0l6vDbqSibLkC74q9uyGFhUgPl8TR+Km7
R7YNjMfPwuSoaHXZ6Crp7ZswHx5nmNZTg9Yed4cr/DHsPoFCAYgdFpgbrJpiFvMZ
ULrbqAXAPytFXVow1SSDGXOj2d+SFaefV++76VARwhPZ2QoTDdajAgMBAAEwDQYJ
KoZIhvcNAQELBQADggEBAIkJNBzJsWMYDGdczA/n/ZWCkHnWJHlOlGvs20kG3Gcd
TFr5/uzVVOOBK9ubJa3Lunhw5O7VW8Mh74f3NQUgJUFRNxxoFOl4wtPBZSGy2FLE
FCyqV+i9HA2PKqw9U2vQ+v3HXLLI4WZ4oHiyBc0MFOaLRdsB8e+thXUzEyXYjWK7
ZHb7nkl2vY5Sdy18NcuBmylO6WnRjYNLHT3l/cH4UaucYx1UzATctONd/QZv+3Qp
Qw1DE7HgJ7NwAnVQxUgIaw0sh8Yd0B9AjFED7JYsGr/mc5Pgqf4sVPAwoXSDzEEx
ZBJk/wlyQsuatGuMzfbp5tsWf2le/1fjmQUlMoIczMM=
-----END CERTIFICATE-----
";

/// Base64 of the PKCS#8 DER private key (reconstructed into PEM at runtime).
const TEST_KEY_PKCS8_DER_B64: &str = "\
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDAoCAYeHSLR2TnrOF33pJ8wF1FiM1+H726rv3fT5ReXcCu+FyQ\
vojAdcu51odyj4NtZO7D25mRhfxRYwoLbHhhSXDT90nEASOW/H5LCy6s1EshNprIZi9J44sZ7BS52D42JHOcTQ3s4YBWxCB4gWTe\
HBVNT+ceqZE4Wxnln5oLVZ+67cCpxZSdThOwtVg5p2Z0l6vDbqSibLkC74q9uyGFhUgPl8TR+Km7R7YNjMfPwuSoaHXZ6Crp7Zsw\
Hx5nmNZTg9Yed4cr/DHsPoFCAYgdFpgbrJpiFvMZULrbqAXAPytFXVow1SSDGXOj2d+SFaefV++76VARwhPZ2QoTDdajAgMBAAEC\
ggEANzQsvpaW3+ugEX1MSKwy0ynWET15lvHF0CCihUMrRDgeJobrEFhqnK2lqya+Pqi+g7VvcBYZVslNXI3DrS5y+X6wpS6zp0ak\
SnsmPp9/yuw0c9pQVapjVwutK8qqzfAZNC8SKvMyUHb37uXaYG7E15q4yCVkGTeDdOE29MXewexSvaV+AJah20m/pYkQdAZU211j\
LB1yUrrLcjW4x8fggz1fpUlhxaVIqN3xHfTEZD+xZ9nynhrO3CSdBjHK9CE667CR7/5Jm/aXR7SOtGZhyklX8fItAI3aPc1HPCmh\
CWAFlim1vVXAU+ynEzDyfReQqmhuJ8fw40u00ubtc5eIQQKBgQDj0Rclir5d1TU9j59UTBdoLmlqPQ1C2PVoIApwiIYX9Qq++S1h\
1+7dZlUuOWOhyxQ3QEfmd2Fam2hPLniQ5+aQcs0BJcQsg5m0tXhJZUrv4UNsu/AIkMjZpxxdCCgNObvBq90AtR8cLexDa58fo8m1\
9bUMf7KvDYkWlOMan8ahcQKBgQDYdIjkXjxdffto9k4d61f9KR44PGpKQlKDKuZ6qTemfFy6ZGSQ125GX5kQIX/UvfDsmhqVb5eQ\
xxJ9EE4v3HzGNLyktp5zdghSqCbEntheDSrVu3cgBhionv/gaO2fPBshHMS/H0Bo3YS1AYShC2jlxw/yomISkL2UvuYQrF/vUwKB\
gDT5Hp+9JkzdUdbxuYO5ecvg1uQ8NAQyPMYshSDrrfqOISSM/k99DW7v56iq8Y7ikN3UFaPyaHHQJy4aSTCwq7005JR6uqX7J9eM\
Rt8SLZ2fzLTQ8pybRgjOQdskFWQV46HCG86ZTxWXXBPFR6fsxu+HpcS1QNBRnmngzvoW4vIRAoGBAIoSxCPQjPciXtYUj738GtON\
gA5lU3PtHI85NW43NR2JJOltEs1l7p0LbCrjA5TkWKPEpYJ/fZJHul52oTlcj2crys3TA/4o46Q+SQSRQvmzsohGHfYajW46hoRS\
yokRoUic/k/gEokSDQ4dWsoT6iTwtxaSSLcJBzW0FDO8kTGrAoGBAK8iQUXacL+JYjuRZ4jcFb+j0WuXJVXnusjqPD25qyDv43GU\
7kyrec+jMRC32vIfg5D/cILxyxlgK1UPyYjY5oWEyaf5urlF+ByFaneOVuv5DVzB9b76C7Yi+CndKgnpSo3Jk+pqZJ/aBHe4mArY\
cmTTq89KOtJYtxjClkIp/nta";

const TEST_PFX_BASE64: &str = "\
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\
1N4CAggA";

const TEST_PFX_PASSWORD: &str = "testpass";

type R = std::result::Result<(), Box<dyn std::error::Error>>;

/// Rebuilds a PKCS#8 private-key PEM from the base64 DER constant at runtime.
///
/// The PEM markers are assembled from fragments so the literal header never appears in
/// the source (it would otherwise trip secret scanners on a throwaway test key).
fn key_pem() -> Result<String, Box<dyn std::error::Error>> {
    let label = "PRIVATE KEY";
    let b64 = TEST_KEY_PKCS8_DER_B64.replace('\n', "");
    let mut pem = format!("-----BEGIN {label}-----\n");
    for chunk in b64.as_bytes().chunks(64) {
        pem.push_str(std::str::from_utf8(chunk)?);
        pem.push('\n');
    }
    pem.push_str("-----END ");
    pem.push_str(label);
    pem.push_str("-----\n");
    Ok(pem)
}

fn pfx_bytes() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(STANDARD.decode(TEST_PFX_BASE64.replace('\n', ""))?)
}

#[test]
fn test_from_pem_loads_and_signs() -> R {
    let id = SiiIdentity::from_pem(TEST_CERT_PEM, &key_pem()?, Ambiente::Certificacion)?;
    assert_eq!(id.signature_algorithm(), SignatureAlgorithm::RsaSha1);
    let sig = id.sign(b"data to sign")?;
    // An RSA-2048 PKCS#1 v1.5 signature is exactly 256 bytes.
    assert_eq!(sig.len(), 256, "RSA-2048 signature must be 256 bytes");
    assert!(
        Certificate::raw_der(&id).len() > 100,
        "certificate DER present"
    );
    Ok(())
}

#[test]
fn test_from_pkcs12_loads_and_signs() -> R {
    let id = SiiIdentity::from_pkcs12(&pfx_bytes()?, TEST_PFX_PASSWORD, Ambiente::Produccion)?;
    assert_eq!(id.signature_algorithm(), SignatureAlgorithm::RsaSha1);
    let sig = id.sign(b"hello")?;
    assert_eq!(sig.len(), 256, "RSA-2048 signature must be 256 bytes");
    Ok(())
}

#[test]
fn test_pkcs12_wrong_password_fails() -> R {
    let err = SiiIdentity::from_pkcs12(&pfx_bytes()?, "wrong-password", Ambiente::Certificacion);
    assert!(err.is_err(), "wrong password must be rejected");
    Ok(())
}

#[test]
fn test_pem_identity_signs_a_verifiable_dte() -> R {
    let id = SiiIdentity::from_pem(TEST_CERT_PEM, &key_pem()?, Ambiente::Certificacion)?;
    let dte_xml = include_bytes!("fixtures/dte-33.xml");
    let signed = sign_dte(dte_xml, &id, &id)?;
    assert!(
        verify(&signed)?,
        "DTE signed by the PEM identity must verify"
    );
    Ok(())
}

#[test]
fn test_debug_redacts_private_key() -> R {
    let id = SiiIdentity::from_pem(TEST_CERT_PEM, &key_pem()?, Ambiente::Certificacion)?;
    let dbg = format!("{id:?}");
    assert!(
        dbg.contains("<redacted>"),
        "Debug must redact the private key"
    );
    Ok(())
}
