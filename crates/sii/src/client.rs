//! High-level Chilean SII client orchestrating authentication, upload, and status checks.
//!
//! Implements the complete communication lifecycle with the SII (`docs/dte-spec.md` §7, RF-02, ADR-04):
//! 1. Seed acquisition and token exchange ([`SiiClient::obtener_token`]).
//! 2. Envelope upload via multipart HTTP POST ([`SiiClient::enviar`]).
//! 3. Asynchronous status query by Track ID ([`SiiClient::estado`]).
//!
//! Note that in accordance with ADR-04, document or envelope rejection by the SII
//! is returned as a domain value ([`EstadoEnvio::Rechazado`]), not an [`Error`].

use crate::auth::{Semilla, SolicitudToken, Token};
use crate::codes::Ambiente;
use crate::envio::{EnvioBoleta, EnvioDte};
use crate::error::{Error, Result};
use crate::signature::{Certificate, Signer};
use crate::transport::{Request, Transport};
use crate::types::Rut;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::fmt;
use std::sync::RwLock;

/// Tracking identifier assigned by the Chilean SII upon receiving an envelope upload.
#[derive(Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TrackId(String);

impl TrackId {
    /// Creates a new `TrackId` from a string representation.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into().trim().to_string())
    }

    /// Creates a new `TrackId` from a numeric 64-bit integer.
    #[must_use]
    pub fn from_u64(id: u64) -> Self {
        Self(id.to_string())
    }

    /// Returns a string slice of the Track ID.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Parses the Track ID into a `u64` numeric value if possible.
    #[must_use]
    pub fn as_u64(&self) -> Option<u64> {
        self.0.parse().ok()
    }
}

impl AsRef<str> for TrackId {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TrackId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for TrackId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TrackId").field(&self.0).finish()
    }
}

impl From<u64> for TrackId {
    #[inline]
    fn from(id: u64) -> Self {
        Self::from_u64(id)
    }
}

impl From<&str> for TrackId {
    #[inline]
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for TrackId {
    #[inline]
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&TrackId> for TrackId {
    #[inline]
    fn from(id: &TrackId) -> Self {
        id.clone()
    }
}

/// Detailed metadata returned by the SII when querying an upload status.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DetalleEnvio {
    /// Track ID associated with the query.
    pub track_id: Option<TrackId>,
    /// SII status code (e.g. "EPR", "REC", "SOK", "RCT", "RFR", "-11").
    pub estado: String,
    /// Human-readable descriptive explanation from the SII.
    pub glosa: String,
    /// DTE type code if present in the summary response.
    pub tipo_docto: Option<u16>,
    /// Count of informed documents in the envelope.
    pub informados: Option<u32>,
    /// Count of accepted documents in the envelope.
    pub aceptados: Option<u32>,
    /// Count of rejected documents in the envelope.
    pub rechazados: Option<u32>,
    /// Count of accepted documents with repairs/observations.
    pub reparos: Option<u32>,
    /// Attention/reference identifier from the SII response header.
    pub num_atencion: Option<String>,
    /// Raw unescaped XML response string for audit purposes.
    pub raw_xml: Option<String>,
}

/// Processing status of an envelope submitted to the Chilean SII (ADR-04).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EstadoEnvio {
    /// Envelope was accepted by the SII (all documents valid, or schema/cover valid).
    Aceptado(DetalleEnvio),
    /// Envelope or contained documents were rejected by the SII.
    Rechazado(DetalleEnvio),
    /// Envelope is queued or currently undergoing processing.
    EnProceso(DetalleEnvio),
}

impl EstadoEnvio {
    /// Returns `true` if the status is [`EstadoEnvio::Aceptado`].
    #[inline]
    #[must_use]
    pub const fn is_aceptado(&self) -> bool {
        matches!(self, Self::Aceptado(_))
    }

    /// Returns `true` if the status is [`EstadoEnvio::Rechazado`].
    #[inline]
    #[must_use]
    pub const fn is_rechazado(&self) -> bool {
        matches!(self, Self::Rechazado(_))
    }

    /// Returns `true` if the status is [`EstadoEnvio::EnProceso`].
    #[inline]
    #[must_use]
    pub const fn is_en_proceso(&self) -> bool {
        matches!(self, Self::EnProceso(_))
    }

    /// Returns a reference to the inner [`DetalleEnvio`].
    #[must_use]
    pub const fn detalle(&self) -> &DetalleEnvio {
        match self {
            Self::Aceptado(d) | Self::Rechazado(d) | Self::EnProceso(d) => d,
        }
    }

    /// Returns the SII status code string (e.g. "EPR", "REC", "RCT").
    #[inline]
    #[must_use]
    pub fn codigo(&self) -> &str {
        &self.detalle().estado
    }

    /// Returns the descriptive glosa explanation string.
    #[inline]
    #[must_use]
    pub fn glosa(&self) -> &str {
        &self.detalle().glosa
    }
}

/// High-level client for interacting with Chilean SII Web Services and endpoints.
pub struct SiiClient<T: Transport> {
    ambiente: Ambiente,
    transport: T,
    rut_emisor: RwLock<Option<Rut>>,
}

impl<T: Transport> SiiClient<T> {
    /// Creates a new `SiiClient` targeting the specified environment using `transport`.
    pub fn new(ambiente: Ambiente, transport: T) -> Self {
        Self {
            ambiente,
            transport,
            rut_emisor: RwLock::new(None),
        }
    }

    /// Configures a default company RUT (`RutEmisor`) for status queries.
    #[must_use]
    pub fn with_rut(self, rut: Rut) -> Self {
        self.set_rut_emisor(rut);
        self
    }

    /// Returns the target execution environment.
    #[inline]
    #[must_use]
    pub const fn ambiente(&self) -> Ambiente {
        self.ambiente
    }

    /// Returns a reference to the underlying transport instance.
    #[inline]
    #[must_use]
    pub const fn transport(&self) -> &T {
        &self.transport
    }

    /// Sets or updates the remembered company RUT.
    pub fn set_rut_emisor(&self, rut: Rut) {
        if let Ok(mut lock) = self.rut_emisor.write() {
            *lock = Some(rut);
        }
    }

    /// Returns the currently remembered company RUT, if set.
    #[must_use]
    pub fn rut_emisor(&self) -> Option<Rut> {
        if let Ok(lock) = self.rut_emisor.read() {
            *lock
        } else {
            None
        }
    }

    /// Acquires a fresh authentication seed from the SII (`CrSeed.jws`).
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if the network request fails or the SII returns an error status.
    pub fn obtener_semilla(&self) -> Result<Semilla> {
        let req = Request::get(self.ambiente.seed_url());
        let resp = self.transport.send(&req)?;
        Semilla::parse_xml(&resp.body)
    }

    /// Exchanges a digitally signed token request (`SolicitudToken`) for a session [`Token`].
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if the network request fails or token creation is rejected.
    pub fn obtener_token_con_solicitud(&self, solicitud: &SolicitudToken) -> Result<Token> {
        let req = Request::post(self.ambiente.token_url(), solicitud.as_bytes().to_vec())
            .with_header("Content-Type", "application/xml; charset=UTF-8");
        let resp = self.transport.send(&req)?;
        Token::parse_xml(&resp.body)
    }

    /// Performs the full seed acquisition and token exchange flow.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if seed acquisition, digital signing, or token exchange fails.
    pub fn obtener_token(&self, signer: &dyn Signer, cert: &dyn Certificate) -> Result<Token> {
        let semilla = self.obtener_semilla()?;
        let solicitud = semilla.firmar(signer, cert)?;
        self.obtener_token_con_solicitud(&solicitud)
    }

    /// Submits a signed [`EnvioDte`] envelope to the SII and returns its assigned [`TrackId`].
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if serialization fails, network transport fails, or upload is rejected.
    pub fn enviar(&self, envio: &EnvioDte, token: &Token) -> Result<TrackId> {
        self.set_rut_emisor(*envio.rut_emisor());
        let xml = if let Some(bytes) = envio.signed_xml() {
            bytes.to_vec()
        } else {
            let clock = crate::types::ManualClock::new(time::OffsetDateTime::UNIX_EPOCH);
            envio.to_xml(&clock)?
        };
        self.enviar_raw(*envio.rut_envia(), *envio.rut_emisor(), &xml, token)
    }

    /// Submits a signed [`EnvioBoleta`] envelope to the SII and returns its assigned [`TrackId`].
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if serialization fails, network transport fails, or upload is rejected.
    pub fn enviar_boleta(&self, envio: &EnvioBoleta, token: &Token) -> Result<TrackId> {
        self.set_rut_emisor(*envio.rut_emisor());
        let xml = if let Some(bytes) = envio.signed_xml() {
            bytes.to_vec()
        } else {
            let clock = crate::types::ManualClock::new(time::OffsetDateTime::UNIX_EPOCH);
            envio.to_xml(&clock)?
        };
        self.enviar_raw(*envio.rut_envia(), *envio.rut_emisor(), &xml, token)
    }

    /// Submits raw signed envelope XML using standard multipart upload (`DTEUpload`).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] if network transmission fails or the upload is rejected.
    pub fn enviar_raw(
        &self,
        rut_sender: Rut,
        rut_company: Rut,
        xml: &[u8],
        token: &Token,
    ) -> Result<TrackId> {
        self.set_rut_emisor(rut_company);
        let boundary = "---------------------------sii_sdk_boundary_2026";
        let body = build_multipart_upload_body(boundary, rut_sender, rut_company, xml);

        let content_type = format!("multipart/form-data; boundary={boundary}");
        let cookie = format!("TOKEN={}", token.as_str());

        let req = Request::post(self.ambiente.upload_url(), body)
            .with_header("Content-Type", content_type)
            .with_header("Cookie", cookie)
            .with_header(
                "User-Agent",
                "Mozilla/4.0 (compatible; PROG 1.0; YComp 5.0)",
            );

        let resp = self.transport.send(&req)?;
        parse_upload_response(&resp.body)
    }

    /// Queries the processing status of an envelope by its [`TrackId`].
    ///
    /// Uses the company RUT configured via [`SiiClient::with_rut`] or remembered from
    /// the most recent [`SiiClient::enviar`] call.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if no company RUT is available or network transmission fails.
    pub fn estado(&self, track_id: impl Into<TrackId>, token: &Token) -> Result<EstadoEnvio> {
        let rut = self.rut_emisor().ok_or_else(|| {
            Error::Transport(
                "no company RUT configured for status query; use with_rut or estado_con_rut"
                    .to_string(),
            )
        })?;
        self.estado_con_rut(rut, track_id, token)
    }

    /// Queries the processing status of an envelope with an explicit company RUT (`QueryEstUp.jws`).
    ///
    /// Note that rejection by the SII is returned as `Ok` containing [`EstadoEnvio::Rechazado`] (ADR-04).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`] if network transmission or communication fails.
    pub fn estado_con_rut(
        &self,
        rut_company: Rut,
        track_id: impl Into<TrackId>,
        token: &Token,
    ) -> Result<EstadoEnvio> {
        let tid = track_id.into();
        let soap_body =
            build_soap_query_est_up(self.ambiente.query_status_url(), rut_company, &tid, token);

        let req = Request::post(self.ambiente.query_status_url(), soap_body.into_bytes())
            .with_header("Content-Type", "text/xml; charset=UTF-8")
            .with_header("SOAPAction", "\"\"");

        let resp = self.transport.send(&req)?;
        parse_estado_response(&resp.body)
    }
}

fn build_multipart_upload_body(
    boundary: &str,
    rut_sender: Rut,
    rut_company: Rut,
    xml: &[u8],
) -> Vec<u8> {
    let mut out = Vec::new();
    append_form_field(
        &mut out,
        boundary,
        "rutSender",
        &rut_sender.number().to_string(),
    );
    append_form_field(&mut out, boundary, "dvSender", &rut_sender.dv().to_string());
    append_form_field(
        &mut out,
        boundary,
        "rutCompany",
        &rut_company.number().to_string(),
    );
    append_form_field(
        &mut out,
        boundary,
        "dvCompany",
        &rut_company.dv().to_string(),
    );

    out.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    out.extend_from_slice(
        b"Content-Disposition: form-data; name=\"archivo\"; filename=\"envio.xml\"\r\n",
    );
    out.extend_from_slice(b"Content-Type: text/xml\r\n\r\n");
    out.extend_from_slice(xml);
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    out
}

fn append_form_field(out: &mut Vec<u8>, boundary: &str, name: &str, value: &str) {
    out.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    out.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
    );
    out.extend_from_slice(value.as_bytes());
    out.extend_from_slice(b"\r\n");
}

fn build_soap_query_est_up(
    endpoint_url: &str,
    rut_company: Rut,
    track_id: &TrackId,
    token: &Token,
) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<SOAP-ENV:Envelope xmlns:SOAP-ENV=\"http://schemas.xmlsoap.org/soap/envelope/\" \
xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" \
xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\">\
<SOAP-ENV:Body>\
<m:getEstUp xmlns:m=\"{endpoint_url}\">\
<Rut xsi:type=\"xsd:string\">{rut}</Rut>\
<Dv xsi:type=\"xsd:string\">{dv}</Dv>\
<TrackId xsi:type=\"xsd:string\">{track_id}</TrackId>\
<Token xsi:type=\"xsd:string\">{token}</Token>\
</m:getEstUp>\
</SOAP-ENV:Body>\
</SOAP-ENV:Envelope>",
        rut = rut_company.number(),
        dv = rut_company.dv(),
        track_id = track_id.as_str(),
        token = token.as_str()
    )
}

fn parse_upload_response(xml: &[u8]) -> Result<TrackId> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);

    let mut status = String::new();
    let mut track_id = String::new();
    let mut detail = String::new();

    let mut current_tag = Vec::new();
    let mut buf = Vec::new();

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                current_tag = e.name().local_name().as_ref().to_ascii_uppercase();
            }
            Ok(Event::Text(ref t)) => {
                let txt = t.unescape().map_err(|e| Error::Xml(e.to_string()))?;
                let trimmed = txt.trim();
                if !trimmed.is_empty() {
                    match current_tag.as_slice() {
                        b"STATUS" => status = trimmed.to_string(),
                        b"TRACKID" => track_id = trimmed.to_string(),
                        b"DETAIL" | b"ERROR" | b"GLOSA" if detail.is_empty() => {
                            detail = trimmed.to_string();
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(_)) => current_tag.clear(),
            Ok(Event::Eof) => break,
            Err(e) => return Err(Error::Xml(format!("invalid upload response XML: {e}"))),
            _ => {}
        }
    }

    if status == "0" || status == "00" {
        if !track_id.is_empty() {
            return Ok(TrackId::new(track_id));
        }
        return Err(Error::Xml(
            "SII upload returned status 0 but no TRACKID found".to_string(),
        ));
    }

    let err_msg = if detail.is_empty() {
        format!("SII upload rejected with status {status}")
    } else {
        format!("SII upload rejected with status {status}: {detail}")
    };
    Err(Error::Transport(err_msg))
}

fn parse_estado_response(xml: &[u8]) -> Result<EstadoEnvio> {
    let raw_text = std::str::from_utf8(xml).map_err(|e| Error::Xml(e.to_string()))?;
    let inner_xml = extract_inner_soap_return(raw_text).unwrap_or_else(|| raw_text.to_string());

    let mut reader = Reader::from_reader(inner_xml.as_bytes());
    reader.config_mut().trim_text(true);

    let mut estado = String::new();
    let mut glosa = String::new();
    let mut track_id = None;
    let mut tipo_docto = None;
    let mut informados = None;
    let mut aceptados = None;
    let mut rechazados = None;
    let mut reparos = None;
    let mut num_atencion = None;

    let mut current_tag = Vec::new();
    let mut buf = Vec::new();

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                current_tag = e.name().local_name().as_ref().to_ascii_uppercase();
            }
            Ok(Event::Text(ref t)) => {
                let txt = t.unescape().map_err(|e| Error::Xml(e.to_string()))?;
                let trimmed = txt.trim();
                if !trimmed.is_empty() {
                    match current_tag.as_slice() {
                        b"ESTADO" if estado.is_empty() => estado = trimmed.to_string(),
                        b"GLOSA" if glosa.is_empty() => glosa = trimmed.to_string(),
                        b"TRACKID" => track_id = Some(TrackId::new(trimmed)),
                        b"TIPO_DOCTO" => tipo_docto = trimmed.parse().ok(),
                        b"INFORMADOS" => informados = trimmed.parse().ok(),
                        b"ACEPTADOS" => aceptados = trimmed.parse().ok(),
                        b"RECHAZADOS" => rechazados = trimmed.parse().ok(),
                        b"REPAROS" => reparos = trimmed.parse().ok(),
                        b"NUM_ATENCION" => num_atencion = Some(trimmed.to_string()),
                        _ => {}
                    }
                }
            }
            Ok(Event::End(_)) => current_tag.clear(),
            Ok(Event::Eof) => break,
            Err(e) => return Err(Error::Xml(format!("invalid status response XML: {e}"))),
            _ => {}
        }
    }

    if estado.is_empty() {
        return Err(Error::Xml(
            "missing <ESTADO> tag in SII status response".to_string(),
        ));
    }

    let detalle = DetalleEnvio {
        track_id,
        estado: estado.clone(),
        glosa,
        tipo_docto,
        informados,
        aceptados,
        rechazados,
        reparos,
        num_atencion,
        raw_xml: Some(inner_xml),
    };

    Ok(classify_estado(&estado, detalle))
}

fn extract_inner_soap_return(raw: &str) -> Option<String> {
    let lower = raw.to_ascii_lowercase();
    let start_tag = "<getestupreturn";
    let start_idx = lower.find(start_tag)?;
    let tag_close = raw.get(start_idx..)?.find('>')?;
    let content_start = start_idx.saturating_add(tag_close).saturating_add(1);

    let end_tag = "</getestupreturn>";
    let end_idx = lower.find(end_tag)?;
    let content = raw.get(content_start..end_idx)?;

    Some(
        content
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&amp;", "&"),
    )
}

fn classify_estado(estado: &str, detalle: DetalleEnvio) -> EstadoEnvio {
    let upper = estado.to_ascii_uppercase();
    let num_rechazados = detalle.rechazados.unwrap_or(0);
    let num_aceptados = detalle.aceptados.unwrap_or(0);

    if upper == "EPR" {
        if num_rechazados > 0 {
            return EstadoEnvio::Rechazado(detalle);
        }
        if num_aceptados > 0 {
            return EstadoEnvio::Aceptado(detalle);
        }
        return EstadoEnvio::EnProceso(detalle);
    }

    if upper == "REC"
        || upper == "PRD"
        || upper == "PDR"
        || upper == "-11"
        || upper.contains("PROCESO")
    {
        return EstadoEnvio::EnProceso(detalle);
    }

    if upper == "SOK"
        || upper == "CRT"
        || upper == "FOK"
        || upper == "0"
        || upper == "00"
        || upper.contains("ACEPTAD")
    {
        return EstadoEnvio::Aceptado(detalle);
    }

    if upper == "RCT"
        || upper == "RFR"
        || upper == "RSC"
        || upper == "RCH"
        || upper.starts_with('R')
        || upper.starts_with('-')
        || upper.contains("RECHAZ")
        || num_rechazados > 0
    {
        return EstadoEnvio::Rechazado(detalle);
    }

    if num_aceptados > 0 {
        EstadoEnvio::Aceptado(detalle)
    } else {
        EstadoEnvio::EnProceso(detalle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_id_representations() {
        let tid1 = TrackId::new("12345678");
        assert_eq!(tid1.as_str(), "12345678");
        assert_eq!(tid1.as_u64(), Some(12_345_678));
        assert_eq!(format!("{tid1}"), "12345678");
        assert_eq!(format!("{tid1:?}"), "TrackId(\"12345678\")");

        let tid2 = TrackId::from(99_887_766_u64);
        assert_eq!(tid2.as_str(), "99887766");
        assert_eq!(tid2.as_u64(), Some(99_887_766));
    }

    #[test]
    fn test_parse_upload_response_ok() {
        let xml = b"<RECEPCIONDTE><STATUS>0</STATUS><TRACKID>987654</TRACKID></RECEPCIONDTE>";
        let track_id = parse_upload_response(xml).expect("parse ok");
        assert_eq!(track_id.as_str(), "987654");
    }

    #[test]
    fn test_parse_upload_response_rejected() {
        let xml = b"<RECEPCIONDTE><STATUS>5</STATUS><DETAIL>Token invalido o expirado</DETAIL></RECEPCIONDTE>";
        let err = parse_upload_response(xml).expect_err("should reject");
        match err {
            Error::Transport(msg) => {
                assert!(msg.contains("status 5"));
                assert!(msg.contains("Token invalido"));
            }
            other => panic!("expected Error::Transport, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_estado_response_soap_epr_aceptado() {
        let soap = r#"<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/">
<soapenv:Body>
<ns1:getEstUpResponse xmlns:ns1="https://maullin.sii.cl/DTEWS/QueryEstUp.jws">
<getEstUpReturn xsi:type="xsd:string">&lt;?xml version="1.0"?&gt;&lt;SII:RESPUESTA xmlns:SII="http://www.sii.cl/XMLSchema"&gt;&lt;SII:RESP_HDR&gt;&lt;TRACKID&gt;53292&lt;/TRACKID&gt;&lt;ESTADO&gt;EPR&lt;/ESTADO&gt;&lt;GLOSA&gt;Envio Procesado&lt;/GLOSA&gt;&lt;/SII:RESP_HDR&gt;&lt;SII:RESP_BODY&gt;&lt;TIPO_DOCTO&gt;33&lt;/TIPO_DOCTO&gt;&lt;INFORMADOS&gt;1&lt;/INFORMADOS&gt;&lt;ACEPTADOS&gt;1&lt;/ACEPTADOS&gt;&lt;RECHAZADOS&gt;0&lt;/RECHAZADOS&gt;&lt;REPAROS&gt;0&lt;/REPAROS&gt;&lt;/SII:RESP_BODY&gt;&lt;/SII:RESPUESTA&gt;</getEstUpReturn>
</ns1:getEstUpResponse>
</soapenv:Body>
</soapenv:Envelope>"#;

        let res = parse_estado_response(soap.as_bytes()).expect("parse estado soap");
        assert!(res.is_aceptado());
        assert!(!res.is_rechazado());
        assert_eq!(res.codigo(), "EPR");
        assert_eq!(res.glosa(), "Envio Procesado");
        assert_eq!(res.detalle().track_id.as_ref().unwrap().as_str(), "53292");
        assert_eq!(res.detalle().aceptados, Some(1));
        assert_eq!(res.detalle().rechazados, Some(0));
    }

    #[test]
    fn test_parse_estado_response_soap_epr_rechazado() {
        let soap = r#"<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/">
<soapenv:Body>
<ns1:getEstUpResponse xmlns:ns1="https://maullin.sii.cl/DTEWS/QueryEstUp.jws">
<getEstUpReturn xsi:type="xsd:string">&lt;?xml version="1.0"?&gt;&lt;SII:RESPUESTA xmlns:SII="http://www.sii.cl/XMLSchema"&gt;&lt;SII:RESP_HDR&gt;&lt;TRACKID&gt;53292&lt;/TRACKID&gt;&lt;ESTADO&gt;EPR&lt;/ESTADO&gt;&lt;GLOSA&gt;Envio Rechazado por Documentos&lt;/GLOSA&gt;&lt;/SII:RESP_HDR&gt;&lt;SII:RESP_BODY&gt;&lt;TIPO_DOCTO&gt;33&lt;/TIPO_DOCTO&gt;&lt;INFORMADOS&gt;1&lt;/INFORMADOS&gt;&lt;ACEPTADOS&gt;0&lt;/ACEPTADOS&gt;&lt;RECHAZADOS&gt;1&lt;/RECHAZADOS&gt;&lt;REPAROS&gt;0&lt;/REPAROS&gt;&lt;/SII:RESP_BODY&gt;&lt;/SII:RESPUESTA&gt;</getEstUpReturn>
</ns1:getEstUpResponse>
</soapenv:Body>
</soapenv:Envelope>"#;

        let res = parse_estado_response(soap.as_bytes()).expect("parse estado soap");
        assert!(res.is_rechazado());
        assert!(!res.is_aceptado());
        assert_eq!(res.codigo(), "EPR");
        assert_eq!(res.detalle().rechazados, Some(1));
    }

    #[test]
    fn test_parse_estado_response_direct_xml() {
        let xml = r#"<SII:RESPUESTA xmlns:SII="http://www.sii.cl/XMLSchema">
<SII:RESP_HDR>
<TRACKID>991122</TRACKID>
<ESTADO>REC</ESTADO>
<GLOSA>Envio Recibido</GLOSA>
</SII:RESP_HDR>
</SII:RESPUESTA>"#;

        let res = parse_estado_response(xml.as_bytes()).expect("parse direct xml");
        assert!(res.is_en_proceso());
        assert_eq!(res.codigo(), "REC");
        assert_eq!(res.glosa(), "Envio Recibido");
    }
}
