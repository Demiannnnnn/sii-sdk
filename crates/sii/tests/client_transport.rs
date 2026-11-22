//! Integration tests for `Transport`, `SiiClient`, and `MemoryTransport` (TRANSPORT-01).
//!
//! Tests the complete flow: seed acquisition -> token exchange -> envelope upload -> status query.
//! Verifies that rejection by the SII is returned as a domain value (`EstadoEnvio::Rechazado`, ADR-04),
//! and that no real network or real secrets are used (QA-01, SEC-01).

use rsa::RsaPrivateKey;
use rsa::RsaPublicKey;
use rsa::pkcs1v15::SigningKey;
use rsa::rand_core::OsRng;
use rsa::signature::{SignatureEncoding, Signer as _};
use sha1::Sha1;
use sii::codes::{Ambiente, TipoDte};
use sii::dte::{Detalle, Dte, Emisor, Receptor};
use sii::error::{Error, Result};
use sii::signature::{SignatureOptions, Signer, X509Certificate, sign_dte};
use sii::types::{ManualClock, Monto, Rut};
use sii::{EnvioDte, MemoryTransport, Method, Response, SiiClient, TrackId};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

struct TestSigner {
    private_key: RsaPrivateKey,
}

impl Signer for TestSigner {
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
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

const SEED_XML_OK: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<SII:RESPUESTA xmlns:SII="http://www.sii.cl/XMLSchema">
    <SII:RESP_HDR>
        <ESTADO>00</ESTADO>
        <GLOSA>Operacion Exitosa</GLOSA>
    </SII:RESP_HDR>
    <SII:RESP_BODY>
        <SEMILLA>0079009845665</SEMILLA>
    </SII:RESP_BODY>
</SII:RESPUESTA>"#;

const TOKEN_XML_OK: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<SII:RESPUESTA xmlns:SII="http://www.sii.cl/XMLSchema">
    <SII:RESP_HDR>
        <ESTADO>00</ESTADO>
        <GLOSA>Token Creado</GLOSA>
    </SII:RESP_HDR>
    <SII:RESP_BODY>
        <TOKEN>TOKEN_TEST_MOCK_VAL</TOKEN>
    </SII:RESP_BODY>
</SII:RESPUESTA>"#;

const UPLOAD_XML_OK: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<RECEPCIONDTE>
    <STATUS>0</STATUS>
    <TRACKID>987654321</TRACKID>
</RECEPCIONDTE>"#;

const STATUS_SOAP_OK: &str = r#"<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/">
<soapenv:Body>
<ns1:getEstUpResponse xmlns:ns1="https://maullin.sii.cl/DTEWS/QueryEstUp.jws">
<getEstUpReturn xsi:type="xsd:string">&lt;?xml version="1.0"?&gt;&lt;SII:RESPUESTA xmlns:SII="http://www.sii.cl/XMLSchema"&gt;&lt;SII:RESP_HDR&gt;&lt;TRACKID&gt;987654321&lt;/TRACKID&gt;&lt;ESTADO&gt;EPR&lt;/ESTADO&gt;&lt;GLOSA&gt;Envio Procesado&lt;/GLOSA&gt;&lt;/SII:RESP_HDR&gt;&lt;SII:RESP_BODY&gt;&lt;TIPO_DOCTO&gt;33&lt;/TIPO_DOCTO&gt;&lt;INFORMADOS&gt;1&lt;/INFORMADOS&gt;&lt;ACEPTADOS&gt;1&lt;/ACEPTADOS&gt;&lt;RECHAZADOS&gt;0&lt;/RECHAZADOS&gt;&lt;REPAROS&gt;0&lt;/REPAROS&gt;&lt;/SII:RESP_BODY&gt;&lt;/SII:RESPUESTA&gt;</getEstUpReturn>
</ns1:getEstUpResponse>
</soapenv:Body>
</soapenv:Envelope>"#;

const STATUS_SOAP_REJECTED: &str = r#"<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/">
<soapenv:Body>
<ns1:getEstUpResponse xmlns:ns1="https://maullin.sii.cl/DTEWS/QueryEstUp.jws">
<getEstUpReturn xsi:type="xsd:string">&lt;?xml version="1.0"?&gt;&lt;SII:RESPUESTA xmlns:SII="http://www.sii.cl/XMLSchema"&gt;&lt;SII:RESP_HDR&gt;&lt;TRACKID&gt;987654321&lt;/TRACKID&gt;&lt;ESTADO&gt;EPR&lt;/ESTADO&gt;&lt;GLOSA&gt;Envio Rechazado por Firma Invalida&lt;/GLOSA&gt;&lt;/SII:RESP_HDR&gt;&lt;SII:RESP_BODY&gt;&lt;TIPO_DOCTO&gt;33&lt;/TIPO_DOCTO&gt;&lt;INFORMADOS&gt;1&lt;/INFORMADOS&gt;&lt;ACEPTADOS&gt;0&lt;/ACEPTADOS&gt;&lt;RECHAZADOS&gt;1&lt;/RECHAZADOS&gt;&lt;REPAROS&gt;0&lt;/REPAROS&gt;&lt;/SII:RESP_BODY&gt;&lt;/SII:RESPUESTA&gt;</getEstUpReturn>
</ns1:getEstUpResponse>
</soapenv:Body>
</soapenv:Envelope>"#;

#[test]
fn test_client_full_flow_success() -> TestResult {
    let transport = MemoryTransport::new();
    let ambiente = Ambiente::Certificacion;

    transport.register_exact(
        ambiente.seed_url(),
        Response::new(200, SEED_XML_OK.as_bytes().to_vec()),
    );
    transport.register_exact(
        ambiente.token_url(),
        Response::new(200, TOKEN_XML_OK.as_bytes().to_vec()),
    );
    transport.register_exact(
        ambiente.upload_url(),
        Response::new(200, UPLOAD_XML_OK.as_bytes().to_vec()),
    );
    transport.register_exact(
        ambiente.query_status_url(),
        Response::new(200, STATUS_SOAP_OK.as_bytes().to_vec()),
    );

    let client = SiiClient::new(ambiente, transport);
    let (signer, cert) = test_signer_and_cert()?;

    // Step 1: Obtain Token (seeds + signs + exchanges)
    let token = client.obtener_token(&signer, &cert)?;
    assert_eq!(token.as_str(), "TOKEN_TEST_MOCK_VAL");

    // Step 2: Build and sign an EnvioDTE envelope
    let dte = test_factura_afecta(101)?;
    let dte_xml = dte.to_xml()?;
    let signed_dte = sign_dte(&dte_xml, &signer, &cert)?;

    let rut_emisor = Rut::parse("76.086.428-5")?;
    let rut_envia = Rut::parse("11.111.111-1")?;
    let resol_date = time::Date::from_calendar_date(2024, time::Month::January, 1)?;

    let mut envio = EnvioDte::nuevo(rut_emisor, rut_envia, resol_date, 0);
    envio.agregar(TipoDte::FacturaAfecta, &signed_dte)?;

    let clock = test_clock()?;
    let signed_envio_xml = envio.firmar(&clock, &signer, &cert, SignatureOptions::default())?;
    envio.set_signed_xml(signed_envio_xml);

    // Step 3: Upload envelope and receive TrackId
    let track_id = client.enviar(&envio, &token)?;
    assert_eq!(track_id.as_str(), "987654321");
    assert_eq!(track_id.as_u64(), Some(987_654_321));

    // Step 4: Query processing status
    let estado = client.estado(track_id, &token)?;
    assert!(estado.is_aceptado());
    assert!(!estado.is_rechazado());
    assert_eq!(estado.codigo(), "EPR");
    assert_eq!(estado.glosa(), "Envio Procesado");
    assert_eq!(estado.detalle().aceptados, Some(1));
    assert_eq!(estado.detalle().rechazados, Some(0));

    // Step 5: Verify all HTTP requests captured by MemoryTransport
    let requests = client.transport().requests();
    assert_eq!(requests.len(), 4, "expected exactly 4 requests");

    // Request 1: GET seed
    assert_eq!(requests[0].method, Method::Get);
    assert_eq!(requests[0].url, ambiente.seed_url());

    // Request 2: POST token
    assert_eq!(requests[1].method, Method::Post);
    assert_eq!(requests[1].url, ambiente.token_url());
    let token_req_xml = String::from_utf8(requests[1].body.clone())?;
    assert!(token_req_xml.contains("<Semilla>0079009845665</Semilla>"));
    assert!(token_req_xml.contains("<Signature"));

    // Request 3: Multipart upload POST
    assert_eq!(requests[2].method, Method::Post);
    assert_eq!(requests[2].url, ambiente.upload_url());
    assert_eq!(requests[2].header("Cookie"), Some("TOKEN=TOKEN_TEST_MOCK_VAL"));
    assert!(
        requests[2]
            .header("Content-Type")
            .unwrap()
            .starts_with("multipart/form-data")
    );
    let upload_body = String::from_utf8_lossy(&requests[2].body);
    assert!(upload_body.contains("name=\"rutSender\""));
    assert!(upload_body.contains("name=\"rutCompany\""));
    assert!(upload_body.contains("name=\"archivo\""));

    // Request 4: SOAP status query POST
    assert_eq!(requests[3].method, Method::Post);
    assert_eq!(requests[3].url, ambiente.query_status_url());
    assert_eq!(
        requests[3].header("Content-Type"),
        Some("text/xml; charset=UTF-8")
    );
    let soap_req = String::from_utf8(requests[3].body.clone())?;
    assert!(soap_req.contains("<TrackId xsi:type=\"xsd:string\">987654321</TrackId>"));
    assert!(soap_req.contains("<Token xsi:type=\"xsd:string\">TOKEN_TEST_MOCK_VAL</Token>"));

    Ok(())
}

#[test]
fn test_adr04_rejection_is_domain_value_not_err() -> TestResult {
    // ADR-04: Que el SII rechace un envío es un resultado del dominio (EstadoEnvio::Rechazado), no un Err.
    let transport = MemoryTransport::new();
    let ambiente = Ambiente::Certificacion;
    transport.register_exact(
        ambiente.query_status_url(),
        Response::new(200, STATUS_SOAP_REJECTED.as_bytes().to_vec()),
    );

    let client = SiiClient::new(ambiente, transport).with_rut(Rut::parse("76.086.428-5")?);
    let token = sii::auth::Token::new("MOCK_TOKEN_XYZ");

    let result = client.estado(TrackId::new("987654321"), &token);

    // MUST be Ok, NOT Err!
    assert!(
        result.is_ok(),
        "rejection by SII must not be returned as an Err"
    );
    let estado = result?;

    assert!(estado.is_rechazado(), "must be EstadoEnvio::Rechazado");
    assert!(!estado.is_aceptado());
    assert_eq!(estado.codigo(), "EPR");
    assert!(estado.glosa().contains("Rechazado"));
    assert_eq!(estado.detalle().rechazados, Some(1));
    assert_eq!(estado.detalle().aceptados, Some(0));

    Ok(())
}

#[test]
fn test_status_en_proceso_intermediate() -> TestResult {
    let transport = MemoryTransport::new();
    let ambiente = Ambiente::Certificacion;

    let in_process_xml = r#"<SII:RESPUESTA xmlns:SII="http://www.sii.cl/XMLSchema">
<SII:RESP_HDR>
    <TRACKID>112233</TRACKID>
    <ESTADO>REC</ESTADO>
    <GLOSA>Envio Recibido Conforme, en cola de validacion</GLOSA>
</SII:RESP_HDR>
</SII:RESPUESTA>"#;

    transport.register_exact(
        ambiente.query_status_url(),
        Response::new(200, in_process_xml.as_bytes().to_vec()),
    );

    let client = SiiClient::new(ambiente, transport).with_rut(Rut::parse("76.086.428-5")?);
    let token = sii::auth::Token::new("TOKEN");

    let estado = client.estado(TrackId::new("112233"), &token)?;
    assert!(estado.is_en_proceso());
    assert!(!estado.is_aceptado());
    assert!(!estado.is_rechazado());
    assert_eq!(estado.codigo(), "REC");
    Ok(())
}

#[test]
fn test_upload_rejection_returns_err() {
    let transport = MemoryTransport::new();
    let ambiente = Ambiente::Certificacion;

    let error_upload_xml = r"<RECEPCIONDTE>
<STATUS>5</STATUS>
<DETAIL>Token no autenticado o sesion caducada</DETAIL>
</RECEPCIONDTE>";

    transport.register_exact(
        ambiente.upload_url(),
        Response::new(200, error_upload_xml.as_bytes().to_vec()),
    );

    let client = SiiClient::new(ambiente, transport);
    let rut = Rut::parse("76.086.428-5").unwrap();
    let token = sii::auth::Token::new("INVALID_TOKEN");

    let err = client
        .enviar_raw(rut, rut, b"<xml></xml>", &token)
        .expect_err("upload should fail");

    match err {
        Error::Transport(msg) => {
            assert!(msg.contains("status 5"));
            assert!(msg.contains("Token no autenticado"));
        }
        other => panic!("expected Error::Transport, got {other:?}"),
    }
}

#[test]
fn test_ambiente_urls_certificacion_and_produccion() {
    let cert = Ambiente::Certificacion;
    assert_eq!(cert.base_url(), "https://maullin.sii.cl");
    assert_eq!(cert.seed_url(), "https://maullin.sii.cl/DTEWS/CrSeed.jws");
    assert_eq!(
        cert.token_url(),
        "https://maullin.sii.cl/DTEWS/GetTokenFromSeed.jws"
    );
    assert_eq!(
        cert.upload_url(),
        "https://maullin.sii.cl/cgi_dte/UPL/DTEUpload"
    );
    assert_eq!(
        cert.query_status_url(),
        "https://maullin.sii.cl/DTEWS/QueryEstUp.jws"
    );

    let prod = Ambiente::Produccion;
    assert_eq!(prod.base_url(), "https://palena.sii.cl");
    assert_eq!(prod.seed_url(), "https://palena.sii.cl/DTEWS/CrSeed.jws");
    assert_eq!(
        prod.token_url(),
        "https://palena.sii.cl/DTEWS/GetTokenFromSeed.jws"
    );
    assert_eq!(
        prod.upload_url(),
        "https://palena.sii.cl/cgi_dte/UPL/DTEUpload"
    );
    assert_eq!(
        prod.query_status_url(),
        "https://palena.sii.cl/DTEWS/QueryEstUp.jws"
    );
}
