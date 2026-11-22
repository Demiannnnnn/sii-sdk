//! Integration tests for the Chilean SII simulator (`sii-sim`).
//!
//! Tests the complete flow: seed acquisition -> token exchange -> envelope upload -> status query
//! against both in-memory `SiiSimulator` and live loopback `HttpServer`.
//!
//! Verifies rejection handling (ADR-04), non-loopback rejection (QA-01, RF-04),
//! and per-TrackID custom responses.

use rsa::pkcs1v15::SigningKey;
use rsa::rand_core::OsRng;
use rsa::signature::{SignatureEncoding, Signer as _};
use rsa::{RsaPrivateKey, RsaPublicKey};
use sha1::Sha1;
use sii::codes::{Ambiente, TipoDte};
use sii::dte::{Detalle, Dte, Emisor, Receptor};
use sii::error::Result as SiiResult;
use sii::signature::{SignatureOptions, Signer, X509Certificate, sign_dte};
use sii::types::{ManualClock, Monto, Rut};
use sii::{EnvioDte, SiiClient, TrackId};
use sii_sim::{
    HttpServer, Scenario, ScenarioConfig, SiiSimulator, SimError, SimTransport, TrackStatusConfig,
};
use std::sync::Arc;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

struct TestSigner {
    private_key: RsaPrivateKey,
}

impl Signer for TestSigner {
    fn sign(&self, data: &[u8]) -> SiiResult<Vec<u8>> {
        let key = SigningKey::<Sha1>::new(self.private_key.clone());
        Ok(key.sign(data).to_vec())
    }
}

fn test_signer_and_cert()
-> std::result::Result<(TestSigner, X509Certificate), Box<dyn std::error::Error>> {
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

fn test_emisor() -> std::result::Result<Emisor, Box<dyn std::error::Error>> {
    Ok(Emisor::new(
        Rut::parse("76.086.428-5")?,
        "EMPRESA DE PRUEBA SPA",
        "SERVICIOS INFORMATICOS",
        620_100,
        "CALLE FALSA 123",
        "SANTIAGO",
    ))
}

fn test_receptor() -> std::result::Result<Receptor, Box<dyn std::error::Error>> {
    Ok(Receptor::new(
        Rut::parse("77.777.777-7")?,
        "CLIENTE DE PRUEBAS SA",
        "SERVICIOS",
        "AVENIDA CENTRAL 456",
        "PROVIDENCIA",
    ))
}

fn test_factura_afecta(folio: u64) -> std::result::Result<Dte, Box<dyn std::error::Error>> {
    Ok(Dte::factura_afecta()
        .folio(folio)
        .fch_emis_str("2026-11-21")?
        .emisor(test_emisor()?)
        .receptor(test_receptor()?)
        .item(
            Detalle::new(1, "Licencia Software", Monto::from_pesos(50_000))
                .qty(1)
                .prc(Monto::from_pesos(50_000)),
        )
        .build()?)
}

fn test_clock() -> std::result::Result<ManualClock, Box<dyn std::error::Error>> {
    let date = time::Date::from_calendar_date(2026, time::Month::November, 21)?;
    let t = time::Time::from_hms(10, 0, 0)?;
    Ok(ManualClock::new(date.with_time(t).assume_utc()))
}

fn build_signed_envio(
    folio: u64,
    signer: &dyn Signer,
    cert: &X509Certificate,
) -> std::result::Result<EnvioDte, Box<dyn std::error::Error>> {
    let dte = test_factura_afecta(folio)?;
    let dte_xml = dte.to_xml()?;
    let signed_dte = sign_dte(&dte_xml, signer, cert)?;

    let rut_emisor = Rut::parse("76.086.428-5")?;
    let rut_envia = Rut::parse("11.111.111-1")?;
    let resol_date = time::Date::from_calendar_date(2024, time::Month::January, 1)?;

    let mut envio = EnvioDte::nuevo(rut_emisor, rut_envia, resol_date, 0);
    envio.agregar(TipoDte::FacturaAfecta, &signed_dte)?;

    let clock = test_clock()?;
    let signed_envio_xml = envio.firmar(&clock, signer, cert, SignatureOptions::default())?;
    envio.set_signed_xml(signed_envio_xml);

    Ok(envio)
}

#[test]
fn test_client_full_flow_aceptado() -> TestResult {
    let sim = SiiSimulator::new().with_scenario(Scenario::Aceptado);
    let client = SiiClient::new(Ambiente::Certificacion, &sim);
    let (signer, cert) = test_signer_and_cert()?;

    // 1. Obtener Token
    let token = client.obtener_token(&signer, &cert)?;
    assert_eq!(token.as_str(), "SIM_TOKEN_79009845");

    // 2. Armar y enviar sobre
    let envio = build_signed_envio(101, &signer, &cert)?;
    let track_id = client.enviar(&envio, &token)?;
    assert_eq!(track_id.as_str(), "100000001");

    // 3. Consultar estado
    let estado = client.estado(&track_id, &token)?;
    assert!(estado.is_aceptado());
    assert!(!estado.is_rechazado());
    assert!(!estado.is_en_proceso());
    assert_eq!(estado.codigo(), "EPR");
    assert_eq!(estado.detalle().aceptados, Some(1));
    assert_eq!(estado.detalle().rechazados, Some(0));

    // 4. Verificar registro de subida en el simulador
    let uploads = sim.uploads();
    assert_eq!(uploads.len(), 1);
    assert_eq!(uploads[0].track_id, track_id);
    assert_eq!(uploads[0].rut_sender, Some(Rut::parse("11.111.111-1")?));
    assert_eq!(uploads[0].rut_company, Some(Rut::parse("76.086.428-5")?));
    assert_eq!(uploads[0].token.as_deref(), Some("SIM_TOKEN_79009845"));
    assert_ne!(uploads[0].xml_payload.len(), 0);

    Ok(())
}

#[test]
fn test_client_full_flow_rechazado_adr04() -> TestResult {
    let sim = SiiSimulator::new().with_scenario(Scenario::Rechazado);
    let client = SiiClient::new(Ambiente::Certificacion, &sim);
    let (signer, cert) = test_signer_and_cert()?;

    let token = client.obtener_token(&signer, &cert)?;
    let envio = build_signed_envio(102, &signer, &cert)?;
    let track_id = client.enviar(&envio, &token)?;

    // ADR-04: Que el SII rechace un envío es un resultado del dominio (EstadoEnvio::Rechazado), no un Err.
    let estado = client.estado(&track_id, &token)?;
    assert!(estado.is_rechazado(), "must be EstadoEnvio::Rechazado");
    assert!(!estado.is_aceptado());
    assert!(!estado.is_en_proceso());
    assert_eq!(estado.codigo(), "EPR");
    assert!(estado.glosa().contains("Rechazado"));
    assert_eq!(estado.detalle().rechazados, Some(1));
    assert_eq!(estado.detalle().aceptados, Some(0));

    Ok(())
}

#[test]
fn test_client_full_flow_en_proceso() -> TestResult {
    let sim = SiiSimulator::new().with_scenario(Scenario::EnProceso);
    let client = SiiClient::new(Ambiente::Certificacion, &sim);
    let (signer, cert) = test_signer_and_cert()?;

    let token = client.obtener_token(&signer, &cert)?;
    let envio = build_signed_envio(103, &signer, &cert)?;
    let track_id = client.enviar(&envio, &token)?;

    let estado = client.estado(&track_id, &token)?;
    assert!(estado.is_en_proceso());
    assert!(!estado.is_aceptado());
    assert!(!estado.is_rechazado());
    assert_eq!(estado.codigo(), "REC");
    assert!(estado.glosa().contains("Recibido Conforme"));

    Ok(())
}

#[test]
fn test_custom_status_per_track_id() -> TestResult {
    let sim = SiiSimulator::new().with_scenario(Scenario::EnProceso);

    sim.register_track_status(
        "555001",
        TrackStatusConfig::aceptado("Envio Aceptado Especial"),
    );
    sim.register_track_status(
        "555002",
        TrackStatusConfig::rechazado("Envio Rechazado por Firma"),
    );

    let client =
        SiiClient::new(Ambiente::Certificacion, &sim).with_rut(Rut::parse("76.086.428-5")?);
    let token = sii::auth::Token::new("MOCK_TOKEN");

    // Track 555001: custom Aceptado
    let est1 = client.estado(TrackId::new("555001"), &token)?;
    assert!(est1.is_aceptado());
    assert_eq!(est1.glosa(), "Envio Aceptado Especial");

    // Track 555002: custom Rechazado
    let est2 = client.estado(TrackId::new("555002"), &token)?;
    assert!(est2.is_rechazado());
    assert_eq!(est2.glosa(), "Envio Rechazado por Firma");

    // Track sin registro explícito: cae al default del simulador (EnProceso)
    let est_default = client.estado(TrackId::new("999999"), &token)?;
    assert!(est_default.is_en_proceso());
    assert_eq!(est_default.codigo(), "REC");

    Ok(())
}

#[test]
fn test_scenario_token_invalido() -> TestResult {
    let sim = SiiSimulator::new().with_scenario(Scenario::TokenInvalido);
    let client = SiiClient::new(Ambiente::Certificacion, &sim);
    let (signer, cert) = test_signer_and_cert()?;

    // Token acquisition fails with status 11
    let token_err = client.obtener_token(&signer, &cert);
    assert!(token_err.is_err());
    let err_msg = format!("{:?}", token_err.err().unwrap());
    assert!(err_msg.contains("11") || err_msg.contains("Firma invalida"));

    // Upload with invalid token fails with status 5
    let bad_token = sii::auth::Token::new("INVALID_TOKEN");
    let rut = Rut::parse("76.086.428-5")?;
    let upload_err = client.enviar_raw(rut, rut, b"<xml/>", &bad_token);
    assert!(upload_err.is_err());
    let upload_msg = format!("{:?}", upload_err.err().unwrap());
    assert!(upload_msg.contains("status 5") || upload_msg.contains("Token no autenticado"));

    Ok(())
}

#[test]
fn test_scenario_upload_rechazado() -> TestResult {
    let sim = SiiSimulator::new().with_scenario(Scenario::UploadRechazado);
    let client = SiiClient::new(Ambiente::Certificacion, &sim);
    let (signer, cert) = test_signer_and_cert()?;

    let token = client.obtener_token(&signer, &cert)?;
    let rut = Rut::parse("76.086.428-5")?;
    let upload_err = client.enviar_raw(rut, rut, b"<xml/>", &token);
    assert!(upload_err.is_err());
    let msg = format!("{:?}", upload_err.err().unwrap());
    assert!(msg.contains("status 1") || msg.contains("Caratula"));

    Ok(())
}

#[test]
fn test_scenario_semilla_error() {
    let sim = SiiSimulator::new().with_scenario(Scenario::SemillaError);
    let client = SiiClient::new(Ambiente::Certificacion, &sim);

    let seed_err = client.obtener_semilla();
    assert!(seed_err.is_err());
    let msg = format!("{:?}", seed_err.err().unwrap());
    assert!(msg.contains("-1") || msg.contains("base de datos"));
}

#[test]
fn test_http_server_live_network() -> TestResult {
    let sim = Arc::new(SiiSimulator::new().with_scenario(Scenario::Aceptado));
    let server = HttpServer::bind("127.0.0.1:0")?;
    let mut handle = server.spawn(Arc::clone(&sim))?;

    let base_url = handle.url("");
    let sim_transport = SimTransport::http(base_url);
    let client = SiiClient::new(Ambiente::Certificacion, sim_transport);
    let (signer, cert) = test_signer_and_cert()?;

    // Live HTTP seed & token exchange
    let token = client.obtener_token(&signer, &cert)?;
    assert_eq!(token.as_str(), "SIM_TOKEN_79009845");

    // Live HTTP upload & status query
    let envio = build_signed_envio(201, &signer, &cert)?;
    let track_id = client.enviar(&envio, &token)?;
    assert_eq!(track_id.as_str(), "100000001");

    let estado = client.estado(&track_id, &token)?;
    assert!(estado.is_aceptado());
    assert_eq!(estado.codigo(), "EPR");

    handle.stop();
    Ok(())
}

#[test]
fn test_non_loopback_rejected_qa01() {
    let res = HttpServer::bind("8.8.8.8:8080");
    assert!(res.is_err());
    match res.err().unwrap() {
        SimError::NonLoopbackAddress(addr) => {
            assert!(addr.contains("8.8.8.8"));
        }
        other => panic!("expected SimError::NonLoopbackAddress, got {other:?}"),
    }
}

#[test]
fn test_json_configuration_loading() -> TestResult {
    let json = r#"{
        "default_scenario": "rechazado",
        "fixed_seed": "009988776655",
        "fixed_token": "CUSTOM_TOKEN_XYZ",
        "track_statuses": {
            "777": {
                "estado": "EPR",
                "glosa": "Envio Aceptado Especial",
                "aceptados": 1,
                "rechazados": 0,
                "reparos": 0,
                "tipo_docto": 33
            }
        }
    }"#;

    let config = ScenarioConfig::from_json(json)?;
    let sim = SiiSimulator::new().with_config(config);
    let client = SiiClient::new(Ambiente::Certificacion, &sim);
    let (signer, cert) = test_signer_and_cert()?;

    let seed = client.obtener_semilla()?;
    assert_eq!(seed.as_str(), "009988776655");

    let token = client.obtener_token(&signer, &cert)?;
    assert_eq!(token.as_str(), "CUSTOM_TOKEN_XYZ");

    let client_rut = client.with_rut(Rut::parse("76.086.428-5")?);
    let est_777 = client_rut.estado(TrackId::new("777"), &token)?;
    assert!(est_777.is_aceptado());

    let est_default = client_rut.estado(TrackId::new("888"), &token)?;
    assert!(est_default.is_rechazado());

    Ok(())
}
