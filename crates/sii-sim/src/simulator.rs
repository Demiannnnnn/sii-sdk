//! Sans-IO simulation core for Chilean SII endpoints (`docs/md/simulator-spec.md` §1).

use crate::error::Result;
use crate::scenario::{Scenario, ScenarioConfig, TrackStatusConfig};
use sii::transport::{Request, Response, Transport};
use sii::types::Rut;
use sii::{Error as SiiError, TrackId};
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

/// Simulated upload submission record captured by the simulator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatedUpload {
    /// Track ID assigned by the simulator.
    pub track_id: TrackId,
    /// RUT of the sender (`rutSender` + `dvSender`), if parsed from the upload.
    pub rut_sender: Option<Rut>,
    /// RUT of the company (`rutCompany` + `dvCompany`), if parsed from the upload.
    pub rut_company: Option<Rut>,
    /// Session token string extracted from the `Cookie: TOKEN=` header.
    pub token: Option<String>,
    /// Raw XML payload bytes from the uploaded envelope file.
    pub xml_payload: Vec<u8>,
}

/// In-memory sans-IO simulator imitating Chilean SII Web Services and upload endpoints.
///
/// Implements [`Transport`] so it can be passed directly to [`sii::SiiClient`].
///
/// Responses are deterministic, based on [`Scenario`] and configurable per Track ID.
pub struct SiiSimulator {
    config: RwLock<ScenarioConfig>,
    track_counter: AtomicU64,
    uploads: RwLock<Vec<SimulatedUpload>>,
    requests: RwLock<Vec<Request>>,
}

impl Default for SiiSimulator {
    fn default() -> Self {
        Self::new()
    }
}

impl SiiSimulator {
    /// Creates a new simulator with default configuration ([`Scenario::Aceptado`]).
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: RwLock::new(ScenarioConfig::default()),
            track_counter: AtomicU64::new(100_000_001),
            uploads: RwLock::new(Vec::new()),
            requests: RwLock::new(Vec::new()),
        }
    }

    /// Creates a new simulator configured with a specific scenario.
    #[must_use]
    pub fn with_scenario(self, scenario: Scenario) -> Self {
        if let Ok(mut lock) = self.config.write() {
            lock.default_scenario = scenario;
        }
        self
    }

    /// Creates a new simulator with custom [`ScenarioConfig`].
    #[must_use]
    pub fn with_config(self, config: ScenarioConfig) -> Self {
        if let Ok(mut lock) = self.config.write() {
            *lock = config;
        }
        self
    }

    /// Updates the default simulation scenario.
    pub fn set_scenario(&self, scenario: Scenario) {
        if let Ok(mut lock) = self.config.write() {
            lock.default_scenario = scenario;
        }
    }

    /// Returns the currently active default scenario.
    #[must_use]
    pub fn scenario(&self) -> Scenario {
        if let Ok(lock) = self.config.read() {
            lock.default_scenario
        } else {
            Scenario::Aceptado
        }
    }

    /// Registers a custom status query response for a specific Track ID.
    pub fn register_track_status(&self, track_id: impl Into<String>, status: TrackStatusConfig) {
        if let Ok(mut lock) = self.config.write() {
            lock.track_statuses.insert(track_id.into(), status);
        }
    }

    /// Returns all envelope uploads captured by the simulator.
    #[must_use]
    pub fn uploads(&self) -> Vec<SimulatedUpload> {
        if let Ok(lock) = self.uploads.read() {
            lock.clone()
        } else {
            Vec::new()
        }
    }

    /// Returns all HTTP requests transmitted through the simulator.
    #[must_use]
    pub fn requests(&self) -> Vec<Request> {
        if let Ok(lock) = self.requests.read() {
            lock.clone()
        } else {
            Vec::new()
        }
    }

    /// Clears recorded uploads and requests.
    pub fn clear(&self) {
        if let Ok(mut lock) = self.uploads.write() {
            lock.clear();
        }
        if let Ok(mut lock) = self.requests.write() {
            lock.clear();
        }
    }

    /// Processes an incoming HTTP request and returns the simulated response.
    ///
    /// # Errors
    ///
    /// Returns [`SimError`](crate::error::SimError) if internal state cannot be read.
    pub fn handle_request(&self, req: &Request) -> Result<Response> {
        if let Ok(mut lock) = self.requests.write() {
            lock.push(req.clone());
        }

        let url = &req.url;

        if url.ends_with("/CrSeed.jws") || url.contains("/CrSeed.jws") {
            return Ok(self.handle_seed(req));
        }

        if url.ends_with("/GetTokenFromSeed.jws") || url.contains("/GetTokenFromSeed.jws") {
            return Ok(self.handle_token(req));
        }

        if url.ends_with("/DTEUpload") || url.contains("/DTEUpload") {
            return Ok(self.handle_upload(req));
        }

        if url.ends_with("/QueryEstUp.jws") || url.contains("/QueryEstUp.jws") {
            return Ok(self.handle_status(req));
        }

        // Endpoint not recognized: 404
        Ok(Response::new(
            404,
            b"<html><body><h1>404 Not Found (sii-sim)</h1></body></html>".to_vec(),
        )
        .with_header("Content-Type", "text/html; charset=UTF-8"))
    }

    fn handle_seed(&self, _req: &Request) -> Response {
        let (scenario, fixed_seed) = if let Ok(lock) = self.config.read() {
            (lock.default_scenario, lock.fixed_seed.clone())
        } else {
            (Scenario::Aceptado, None)
        };

        if scenario == Scenario::SemillaError {
            let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<SII:RESPUESTA xmlns:SII="http://www.sii.cl/XMLSchema">
    <SII:RESP_HDR>
        <ESTADO>-1</ESTADO>
        <GLOSA>Error interno de base de datos</GLOSA>
    </SII:RESP_HDR>
</SII:RESPUESTA>"#;
            return Response::new(200, xml.as_bytes().to_vec())
                .with_header("Content-Type", "text/xml; charset=UTF-8");
        }

        let seed = fixed_seed.unwrap_or_else(|| "0079009845665".to_string());
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<SII:RESPUESTA xmlns:SII="http://www.sii.cl/XMLSchema">
    <SII:RESP_HDR>
        <ESTADO>00</ESTADO>
        <GLOSA>Operacion Exitosa</GLOSA>
    </SII:RESP_HDR>
    <SII:RESP_BODY>
        <SEMILLA>{seed}</SEMILLA>
    </SII:RESP_BODY>
</SII:RESPUESTA>"#
        );

        Response::new(200, xml.into_bytes()).with_header("Content-Type", "text/xml; charset=UTF-8")
    }

    fn handle_token(&self, _req: &Request) -> Response {
        let (scenario, fixed_token) = if let Ok(lock) = self.config.read() {
            (lock.default_scenario, lock.fixed_token.clone())
        } else {
            (Scenario::Aceptado, None)
        };

        if scenario == Scenario::TokenInvalido {
            let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<SII:RESPUESTA xmlns:SII="http://www.sii.cl/XMLSchema">
    <SII:RESP_HDR>
        <ESTADO>11</ESTADO>
        <GLOSA>Firma invalida o certificado no autorizado</GLOSA>
    </SII:RESP_HDR>
</SII:RESPUESTA>"#;
            return Response::new(200, xml.as_bytes().to_vec())
                .with_header("Content-Type", "text/xml; charset=UTF-8");
        }

        let token = fixed_token.unwrap_or_else(|| "SIM_TOKEN_79009845".to_string());
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<SII:RESPUESTA xmlns:SII="http://www.sii.cl/XMLSchema">
    <SII:RESP_HDR>
        <ESTADO>00</ESTADO>
        <GLOSA>Token Creado</GLOSA>
    </SII:RESP_HDR>
    <SII:RESP_BODY>
        <TOKEN>{token}</TOKEN>
    </SII:RESP_BODY>
</SII:RESPUESTA>"#
        );

        Response::new(200, xml.into_bytes()).with_header("Content-Type", "text/xml; charset=UTF-8")
    }

    fn handle_upload(&self, req: &Request) -> Response {
        let scenario = self.scenario();

        let token_cookie = req.header("Cookie").and_then(extract_cookie_token);

        if scenario == Scenario::TokenInvalido
            || token_cookie
                .as_deref()
                .is_some_and(|t| t.to_ascii_uppercase().contains("INVALID"))
        {
            let xml = r"<RECEPCIONDTE>
    <STATUS>5</STATUS>
    <DETAIL>Token no autenticado o sesion caducada</DETAIL>
</RECEPCIONDTE>";
            return Response::new(200, xml.as_bytes().to_vec())
                .with_header("Content-Type", "text/xml; charset=UTF-8");
        }

        if scenario == Scenario::UploadRechazado {
            let xml = r"<RECEPCIONDTE>
    <STATUS>1</STATUS>
    <DETAIL>Envio Rechazado por Error en Caratula</DETAIL>
</RECEPCIONDTE>";
            return Response::new(200, xml.as_bytes().to_vec())
                .with_header("Content-Type", "text/xml; charset=UTF-8");
        }

        let next_id = self.track_counter.fetch_add(1, Ordering::SeqCst);
        let track_id = TrackId::from_u64(next_id);

        let parsed = parse_multipart_envelope(&req.body);

        let upload = SimulatedUpload {
            track_id: track_id.clone(),
            rut_sender: parsed.rut_sender,
            rut_company: parsed.rut_company,
            token: token_cookie,
            xml_payload: parsed.xml_payload,
        };

        if let Ok(mut lock) = self.uploads.write() {
            lock.push(upload);
        }

        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<RECEPCIONDTE>
    <STATUS>0</STATUS>
    <TRACKID>{tid}</TRACKID>
</RECEPCIONDTE>"#,
            tid = track_id.as_str()
        );

        Response::new(200, xml.into_bytes()).with_header("Content-Type", "text/xml; charset=UTF-8")
    }

    fn handle_status(&self, req: &Request) -> Response {
        let (scenario, track_statuses) = if let Ok(lock) = self.config.read() {
            (lock.default_scenario, lock.track_statuses.clone())
        } else {
            (Scenario::Aceptado, std::collections::HashMap::new())
        };

        let req_xml = std::str::from_utf8(&req.body).unwrap_or_default();
        let query_track_id =
            extract_tag_value(req_xml, "TrackId").unwrap_or_else(|| "100000001".to_string());
        let query_token = extract_tag_value(req_xml, "Token");

        if scenario == Scenario::TokenInvalido
            || query_token
                .as_deref()
                .is_some_and(|t| t.to_ascii_uppercase().contains("INVALID"))
        {
            let xml = format!(
                r#"<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/">
<soapenv:Body>
<ns1:getEstUpResponse xmlns:ns1="https://maullin.sii.cl/DTEWS/QueryEstUp.jws">
<getEstUpReturn xsi:type="xsd:string">&lt;?xml version="1.0"?&gt;&lt;SII:RESPUESTA xmlns:SII="http://www.sii.cl/XMLSchema"&gt;&lt;SII:RESP_HDR&gt;&lt;TRACKID&gt;{query_track_id}&lt;/TRACKID&gt;&lt;ESTADO&gt;-11&lt;/ESTADO&gt;&lt;GLOSA&gt;Token Invalido o Vencido&lt;/GLOSA&gt;&lt;/SII:RESP_HDR&gt;&lt;/SII:RESPUESTA&gt;</getEstUpReturn>
</ns1:getEstUpResponse>
</soapenv:Body>
</soapenv:Envelope>"#
            );
            return Response::new(200, xml.into_bytes())
                .with_header("Content-Type", "text/xml; charset=UTF-8");
        }

        let status_config = if let Some(custom) = track_statuses.get(&query_track_id) {
            custom.clone()
        } else {
            match scenario {
                Scenario::Aceptado => TrackStatusConfig::aceptado("Envio Procesado"),
                Scenario::Rechazado => {
                    TrackStatusConfig::rechazado("Envio Rechazado por Documentos")
                }
                Scenario::EnProceso => {
                    TrackStatusConfig::en_proceso("Envio Recibido Conforme, en cola de validacion")
                }
                Scenario::UploadRechazado => TrackStatusConfig {
                    estado: "RCT".to_string(),
                    glosa: "Envio Rechazado Totalmente".to_string(),
                    aceptados: Some(0),
                    rechazados: Some(1),
                    reparos: Some(0),
                    tipo_docto: Some(33),
                },
                Scenario::SemillaError | Scenario::TokenInvalido => TrackStatusConfig {
                    estado: "-11".to_string(),
                    glosa: "Error en Autenticacion".to_string(),
                    aceptados: None,
                    rechazados: None,
                    reparos: None,
                    tipo_docto: None,
                },
            }
        };

        let body_part = if let (Some(a), Some(r)) =
            (status_config.aceptados, status_config.rechazados)
        {
            let doc_type = status_config.tipo_docto.unwrap_or(33);
            let reparos = status_config.reparos.unwrap_or(0);
            let inf = a.saturating_add(r).max(1);
            format!(
                "&lt;SII:RESP_BODY&gt;&lt;TIPO_DOCTO&gt;{doc_type}&lt;/TIPO_DOCTO&gt;&lt;INFORMADOS&gt;{inf}&lt;/INFORMADOS&gt;&lt;ACEPTADOS&gt;{a}&lt;/ACEPTADOS&gt;&lt;RECHAZADOS&gt;{r}&lt;/RECHAZADOS&gt;&lt;REPAROS&gt;{reparos}&lt;/REPAROS&gt;&lt;/SII:RESP_BODY&gt;"
            )
        } else {
            String::new()
        };

        let xml = format!(
            r#"<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/">
<soapenv:Body>
<ns1:getEstUpResponse xmlns:ns1="https://maullin.sii.cl/DTEWS/QueryEstUp.jws">
<getEstUpReturn xsi:type="xsd:string">&lt;?xml version="1.0"?&gt;&lt;SII:RESPUESTA xmlns:SII="http://www.sii.cl/XMLSchema"&gt;&lt;SII:RESP_HDR&gt;&lt;TRACKID&gt;{query_track_id}&lt;/TRACKID&gt;&lt;ESTADO&gt;{estado}&lt;/ESTADO&gt;&lt;GLOSA&gt;{glosa}&lt;/GLOSA&gt;&lt;/SII:RESP_HDR&gt;{body_part}&lt;/SII:RESPUESTA&gt;</getEstUpReturn>
</ns1:getEstUpResponse>
</soapenv:Body>
</soapenv:Envelope>"#,
            estado = status_config.estado,
            glosa = status_config.glosa,
        );

        Response::new(200, xml.into_bytes()).with_header("Content-Type", "text/xml; charset=UTF-8")
    }
}

impl Transport for SiiSimulator {
    fn send(&self, request: &Request) -> std::result::Result<Response, SiiError> {
        self.handle_request(request)
            .map_err(|e| SiiError::Transport(e.to_string()))
    }
}

fn extract_cookie_token(cookie: &str) -> Option<String> {
    for part in cookie.split(';') {
        let trimmed = part.trim();
        if let Some(stripped) = trimmed.strip_prefix("TOKEN=") {
            return Some(stripped.trim().to_string());
        }
        if let Some(stripped) = trimmed.strip_prefix("token=") {
            return Some(stripped.trim().to_string());
        }
    }
    None
}

fn extract_tag_value(xml: &str, tag_name: &str) -> Option<String> {
    let open_tag = format!("<{tag_name}");
    let close_tag = format!("</{tag_name}>");

    let start_pos = xml.find(&open_tag)?;
    let content_start = xml
        .get(start_pos..)?
        .find('>')?
        .saturating_add(start_pos)
        .saturating_add(1);
    let end_pos = xml
        .get(content_start..)?
        .find(&close_tag)?
        .saturating_add(content_start);

    let val = xml.get(content_start..end_pos)?.trim();
    if val.is_empty() {
        None
    } else {
        Some(val.to_string())
    }
}

#[derive(Default)]
struct ParsedUpload {
    rut_sender: Option<Rut>,
    rut_company: Option<Rut>,
    xml_payload: Vec<u8>,
}

fn parse_multipart_envelope(body: &[u8]) -> ParsedUpload {
    let mut parsed = ParsedUpload::default();
    let body_str = String::from_utf8_lossy(body);

    let mut sender_num = None;
    let mut sender_dv = None;
    let mut company_num = None;
    let mut company_dv = None;

    for part in body_str.split("---------------------------") {
        if part.contains("name=\"rutSender\"") {
            sender_num = extract_part_body(part);
        } else if part.contains("name=\"dvSender\"") {
            sender_dv = extract_part_body(part);
        } else if part.contains("name=\"rutCompany\"") {
            company_num = extract_part_body(part);
        } else if part.contains("name=\"dvCompany\"") {
            company_dv = extract_part_body(part);
        } else if part.contains("name=\"archivo\"") {
            if let Some(xml_str) = extract_part_body(part) {
                parsed.xml_payload = xml_str.into_bytes();
            }
        }
    }

    if let (Some(num), Some(dv)) = (sender_num, sender_dv) {
        let rut_str = format!("{num}-{dv}");
        parsed.rut_sender = Rut::parse(&rut_str).ok();
    }

    if let (Some(num), Some(dv)) = (company_num, company_dv) {
        let rut_str = format!("{num}-{dv}");
        parsed.rut_company = Rut::parse(&rut_str).ok();
    }

    parsed
}

fn extract_part_body(part: &str) -> Option<String> {
    let sep = "\r\n\r\n";
    let idx = part.find(sep)?;
    let body_start = idx.saturating_add(sep.len());
    let rest = part.get(body_start..)?;
    let trimmed = rest.trim_end_matches("\r\n").trim_end_matches("--").trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
