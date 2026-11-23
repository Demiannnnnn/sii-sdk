//! End-to-end integration tests for `sii-cli` (`docs/md/dte-spec.md`, `tasks/cli/task-01-cli.md`).

use sii::signature::verify;
use sii_sim::{HttpServer, Scenario, SiiSimulator};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

type TestResult = Result<(), Box<dyn std::error::Error>>;

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

fn setup_test_env()
-> Result<(tempfile_helper::TempDir, PathBuf, PathBuf), Box<dyn std::error::Error>> {
    let tmp = tempfile_helper::TempDir::new("sii_cli_test")?;
    let cert_path = tmp.path().join("cert.pem");
    let label = "PRIVATE KEY";
    let b64 = TEST_KEY_PKCS8_DER_B64.replace('\n', "");
    let mut key_pem = format!("-----BEGIN {label}-----\n");
    for chunk in b64.as_bytes().chunks(64) {
        key_pem.push_str(std::str::from_utf8(chunk)?);
        key_pem.push('\n');
    }
    key_pem.push_str("-----END ");
    key_pem.push_str(label);
    key_pem.push_str("-----\n");

    let full_pem = format!("{TEST_CERT_PEM}\n{key_pem}");
    fs::write(&cert_path, full_pem)?;

    let caf_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sii/tests/fixtures/caf-33.xml");
    let caf_path = tmp.path().join("caf.xml");
    fs::copy(&caf_src, &caf_path)?;

    Ok((tmp, cert_path, caf_path))
}

mod tempfile_helper {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    pub(crate) struct TempDir(PathBuf);

    impl TempDir {
        pub(crate) fn new(prefix: &str) -> Result<Self, Box<dyn std::error::Error>> {
            let count = COUNTER.fetch_add(1, Ordering::Relaxed);
            let id = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos();
            let p = std::env::temp_dir().join(format!("{prefix}_{id}_{count}"));
            fs::create_dir_all(&p)?;
            Ok(Self(p))
        }

        pub(crate) fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}

fn step_dte_build(bin: &str, dte_xml: &Path) -> TestResult {
    let out_arg = dte_xml.to_str().ok_or("str conversion failed")?;
    let status = Command::new(bin)
        .args([
            "dte",
            "build",
            "--tipo",
            "33",
            "--folio",
            "60",
            "--emisor-rut",
            "76.086.428-5",
            "--emisor-razon",
            "DISTRIBUIDORA Y COMERCIAL EJEMPLO SPA",
            "--emisor-giro",
            "VENTA AL POR MAYOR DE MAQUINARIA",
            "--emisor-acteco",
            "465900",
            "--emisor-dir",
            "AVENIDA LIBERTADOR 1234",
            "--emisor-comuna",
            "SANTIAGO",
            "--receptor-rut",
            "77.777.777-7",
            "--receptor-razon",
            "CLIENTE DE PRUEBAS SA",
            "--item-nombre",
            "Servidor de aplicaciones",
            "--item-precio",
            "100000",
            "-o",
            out_arg,
        ])
        .status()?;
    assert!(status.success());
    assert!(dte_xml.exists());

    let raw_xml = fs::read_to_string(dte_xml)?;
    assert!(raw_xml.contains("<TipoDTE>33</TipoDTE>"));
    assert!(raw_xml.contains("<Folio>60</Folio>"));
    assert!(raw_xml.contains("<RUTEmisor>76086428-5</RUTEmisor>"));
    Ok(())
}

fn step_dte_timbrar(bin: &str, dte_xml: &Path, caf_path: &Path, dte_stamped: &Path) -> TestResult {
    let dte_arg = dte_xml.to_str().ok_or("str conversion failed")?;
    let caf_arg = caf_path.to_str().ok_or("str conversion failed")?;
    let out_arg = dte_stamped.to_str().ok_or("str conversion failed")?;
    let status = Command::new(bin)
        .args([
            "dte", "timbrar", "--dte", dte_arg, "--caf", caf_arg, "--folio", "60", "-o", out_arg,
        ])
        .status()?;
    assert!(status.success());
    assert!(dte_stamped.exists());

    let stamped_str = fs::read_to_string(dte_stamped)?;
    assert!(stamped_str.contains("<TED version=\"1.0\">"));
    assert!(stamped_str.contains("<FRMT algoritmo=\"SHA1withRSA\">"));
    Ok(())
}

fn step_dte_firmar(
    bin: &str,
    dte_stamped: &Path,
    cert_path: &Path,
    dte_signed: &Path,
) -> TestResult {
    let dte_arg = dte_stamped.to_str().ok_or("str conversion failed")?;
    let cert_arg = cert_path.to_str().ok_or("str conversion failed")?;
    let out_arg = dte_signed.to_str().ok_or("str conversion failed")?;
    let status = Command::new(bin)
        .args([
            "dte",
            "firmar",
            "--dte",
            dte_arg,
            "--cert",
            cert_arg,
            "--ambiente",
            "certificacion",
            "-o",
            out_arg,
        ])
        .status()?;
    assert!(status.success());
    assert!(dte_signed.exists());

    let signed_bytes = fs::read(dte_signed)?;
    let is_valid = verify(&signed_bytes)?;
    assert!(
        is_valid,
        "XMLDSig signature must be cryptographically valid"
    );
    Ok(())
}

fn step_envio_enviar(bin: &str, dte_signed: &Path, cert_path: &Path) -> TestResult {
    let dte_arg = dte_signed.to_str().ok_or("str conversion failed")?;
    let cert_arg = cert_path.to_str().ok_or("str conversion failed")?;
    let output = Command::new(bin)
        .args([
            "envio",
            "enviar",
            "--dte",
            dte_arg,
            "--cert",
            cert_arg,
            "--ambiente",
            "certificacion",
            "--sim",
            "--json",
        ])
        .output()?;
    assert!(output.status.success());

    let stdout_str = String::from_utf8(output.stdout)?;
    assert!(stdout_str.contains("\"track_id\": \"100000001\""));
    assert!(stdout_str.contains("\"status\": \"ok\""));
    Ok(())
}

fn step_envio_estado(bin: &str, cert_path: &Path) -> TestResult {
    let cert_arg = cert_path.to_str().ok_or("str conversion failed")?;
    let output = Command::new(bin)
        .args([
            "envio",
            "estado",
            "100000001",
            "--rut-emisor",
            "76.086.428-5",
            "--cert",
            cert_arg,
            "--ambiente",
            "certificacion",
            "--sim",
            "--json",
        ])
        .output()?;
    assert!(output.status.success());

    let stdout_str = String::from_utf8(output.stdout)?;
    assert!(stdout_str.contains("\"estado\": \"EPR\""));
    assert!(stdout_str.contains("\"clasificacion\": \"Aceptado\""));
    Ok(())
}

#[test]
fn test_cli_end_to_end_pipeline() -> TestResult {
    let (tmp, cert_path, caf_path) = setup_test_env()?;
    let bin = env!("CARGO_BIN_EXE_sii");

    let dte_xml = tmp.path().join("dte.xml");
    let dte_stamped = tmp.path().join("dte_stamped.xml");
    let dte_signed = tmp.path().join("dte_signed.xml");

    step_dte_build(bin, &dte_xml)?;
    step_dte_timbrar(bin, &dte_xml, &caf_path, &dte_stamped)?;
    step_dte_firmar(bin, &dte_stamped, &cert_path, &dte_signed)?;
    step_envio_enviar(bin, &dte_signed, &cert_path)?;
    step_envio_estado(bin, &cert_path)?;

    Ok(())
}

#[test]
fn test_cli_live_http_server_simulation() -> TestResult {
    let (tmp, cert_path, _caf_path) = setup_test_env()?;
    let bin = env!("CARGO_BIN_EXE_sii");

    let sim = Arc::new(SiiSimulator::new().with_scenario(Scenario::Aceptado));
    let server = HttpServer::bind("127.0.0.1:0")?;
    let mut handle = server.spawn(Arc::clone(&sim))?;
    let base_url = handle.url("");

    let signed_fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../sii/tests/fixtures/dte-33-signed.xml");
    let signed_path = tmp.path().join("fixture_signed.xml");
    fs::copy(&signed_fixture, &signed_path)?;

    let signed_arg = signed_path.to_str().ok_or("str conversion failed")?;
    let cert_arg = cert_path.to_str().ok_or("str conversion failed")?;

    let output = Command::new(bin)
        .args([
            "envio",
            "enviar",
            "--dte",
            signed_arg,
            "--cert",
            cert_arg,
            "--ambiente",
            "certificacion",
            "--sim-url",
            &base_url,
            "--json",
        ])
        .output()?;
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout_str = String::from_utf8(output.stdout)?;
    assert!(stdout_str.contains("\"track_id\": \"100000001\""));

    let output = Command::new(bin)
        .args([
            "envio",
            "estado",
            "100000001",
            "--rut-emisor",
            "76.086.428-5",
            "--cert",
            cert_arg,
            "--ambiente",
            "certificacion",
            "--sim-url",
            &base_url,
            "--json",
        ])
        .output()?;
    assert!(output.status.success());

    let stdout_str = String::from_utf8(output.stdout)?;
    assert!(stdout_str.contains("\"clasificacion\": \"Aceptado\""));

    handle.stop();
    Ok(())
}

#[test]
fn test_cli_libro_build_and_sign() -> TestResult {
    let (tmp, cert_path, _caf_path) = setup_test_env()?;
    let bin = env!("CARGO_BIN_EXE_sii");

    let signed_fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../sii/tests/fixtures/dte-33-signed.xml");
    let signed_path = tmp.path().join("dte1.xml");
    fs::copy(&signed_fixture, &signed_path)?;

    let libro_xml = tmp.path().join("libro.xml");
    let signed_arg = signed_path.to_str().ok_or("str conversion failed")?;
    let cert_arg = cert_path.to_str().ok_or("str conversion failed")?;
    let libro_arg = libro_xml.to_str().ok_or("str conversion failed")?;

    let output = Command::new(bin)
        .args([
            "libro",
            "build",
            "--rut-emisor",
            "76.086.428-5",
            "--periodo",
            "2026-10",
            "--operacion",
            "venta",
            "--tipo-libro",
            "mensual",
            "--dte",
            signed_arg,
            "--cert",
            cert_arg,
            "--ambiente",
            "certificacion",
            "-o",
            libro_arg,
            "--json",
        ])
        .output()?;
    assert!(output.status.success());
    assert!(libro_xml.exists());

    let raw_xml = fs::read(&libro_xml)?;
    let is_valid = verify(&raw_xml)?;
    assert!(is_valid, "XMLDSig signature on ledger must be valid");
    Ok(())
}

#[test]
fn test_cli_errors_and_exit_codes() -> TestResult {
    let (tmp, _cert_path, caf_path) = setup_test_env()?;
    let bin = env!("CARGO_BIN_EXE_sii");

    // Invalid RUT should fail with ExitCode 1 and descriptive error
    let output = Command::new(bin)
        .args([
            "dte",
            "build",
            "--tipo",
            "33",
            "--emisor-rut",
            "12.345.678-9", // bad DV
            "--emisor-razon",
            "Test",
            "--emisor-giro",
            "Test",
            "--emisor-dir",
            "Dir",
            "--emisor-comuna",
            "Com",
            "--receptor-rut",
            "77.777.777-7",
            "--receptor-razon",
            "Rec",
            "--item-nombre",
            "Item",
            "--item-precio",
            "1000",
        ])
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    let stderr_str = String::from_utf8(output.stderr)?;
    assert!(stderr_str.contains("RUT inválido"));

    // Folio outside CAF range should fail with ExitCode 1
    let dummy_dte = tmp.path().join("dummy_dte.xml");
    let dummy_arg = dummy_dte.to_str().ok_or("str conversion failed")?;
    let caf_arg = caf_path.to_str().ok_or("str conversion failed")?;

    let status = Command::new(bin)
        .args([
            "dte",
            "build",
            "--tipo",
            "33",
            "--folio",
            "60",
            "--emisor-rut",
            "76.086.428-5",
            "--emisor-razon",
            "SPA",
            "--emisor-giro",
            "Giro",
            "--emisor-dir",
            "Dir",
            "--emisor-comuna",
            "Com",
            "--receptor-rut",
            "77.777.777-7",
            "--receptor-razon",
            "Rec",
            "--item-nombre",
            "Item",
            "--item-precio",
            "1000",
            "-o",
            dummy_arg,
        ])
        .status()?;
    assert!(status.success());

    let output = Command::new(bin)
        .args([
            "dte", "timbrar", "--dte", dummy_arg, "--caf", caf_arg, "--folio",
            "99999", // outside range 1..=100
        ])
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    let stderr_str = String::from_utf8(output.stderr)?;
    assert!(stderr_str.contains("outside authorized CAF range") || stderr_str.contains("CAF"));

    Ok(())
}
