//! Handlers for `sii envio` subcommands (`enviar`, `estado`).

use crate::args::{EnvioEnviarArgs, EnvioEstadoArgs};
use crate::commands::cert::load_identity;
use crate::commands::transport::CliTransport;
use crate::commands::xml_utils::{extract_tag_text, write_output};
use crate::error::CliError;
use crate::output::{print_human, print_json};
use sii::auth::Token;
use sii::client::SiiClient;
use sii::codes::{Ambiente, TipoDte};
use sii::dte::parse_date;
use sii::envio::{EnvioBoleta, EnvioDte};
use sii::signature::SignatureOptions;
use sii::types::{ManualClock, Rut};

/// Executes `sii envio enviar`.
pub(crate) fn run_envio_enviar(
    args: &EnvioEnviarArgs,
    is_json: bool,
    is_quiet: bool,
) -> Result<(), CliError> {
    let ambiente: Ambiente = args.ambiente.into();
    let transport = CliTransport::build(args.sim, args.sim_url.as_deref());
    let client = SiiClient::new(ambiente, transport);

    // Build or read the envelope XML
    let (envelope_bytes, rut_emisor, rut_envia) = if let Some(envio_path) = &args.envio {
        let bytes = std::fs::read(envio_path).map_err(CliError::Io)?;
        let xml_str = std::str::from_utf8(&bytes)
            .map_err(|e| CliError::Dte(sii::Error::Xml(format!("XML no es UTF-8: {e}"))))?;

        let emisor_str = extract_tag_text(xml_str, "RutEmisor").ok_or_else(|| {
            CliError::Dte(sii::Error::Xml(
                "falta etiqueta <RutEmisor> en sobre".into(),
            ))
        })?;
        let envia_str = extract_tag_text(xml_str, "RutEnvia").ok_or_else(|| {
            CliError::Dte(sii::Error::Xml("falta etiqueta <RutEnvia> en sobre".into()))
        })?;

        let emisor = Rut::parse(emisor_str)?;
        let envia = Rut::parse(envia_str)?;
        (bytes, emisor, envia)
    } else if !args.dte.is_empty() {
        build_and_sign_envelope(args, ambiente)?
    } else {
        return Err(CliError::MissingArgument(
            "se requiere --dte <ARCHIVO...> o --envio <ARCHIVO>".into(),
        ));
    };

    if let Some(out_path) = &args.output {
        write_output(&envelope_bytes, Some(out_path))?;
    }

    // Authenticate (Token)
    let token = if let Some(tok) = &args.token {
        Token::new(tok)
    } else {
        let cert_path = args.cert.as_deref().ok_or_else(|| {
            CliError::MissingArgument(
                "--cert (o --token) es requerido para autenticar el envío".into(),
            )
        })?;
        let identity = load_identity(cert_path, args.cert_key.as_deref(), ambiente)?;
        client
            .obtener_token(&identity, &identity)
            .map_err(|e| CliError::Transport(format!("falló la obtención de token del SII: {e}")))?
    };

    // Upload envelope
    let track_id = client
        .enviar_raw(rut_envia, rut_emisor, &envelope_bytes, &token)
        .map_err(|e| CliError::Transport(format!("falló la subida del sobre: {e}")))?;

    if is_json {
        let payload = serde_json::json!({
            "status": "ok",
            "ambiente": format!("{ambiente:?}"),
            "rut_emisor": rut_emisor.to_string(),
            "rut_envia": rut_envia.to_string(),
            "track_id": track_id.as_str(),
            "token": token.as_str(),
        });
        print_json(&payload);
    } else if !is_quiet {
        print_human("Envío cargado exitosamente.");
        print_human(&format!("Ambiente: {ambiente:?}"));
        print_human(&format!("RUT Emisor: {rut_emisor}"));
        print_human(&format!("Track ID: {track_id}"));
        print_human(&format!("Token: {token}"));
    }

    Ok(())
}

fn prepare_dte_bytes(bytes: &[u8]) -> Result<Vec<u8>, CliError> {
    let str_val =
        std::str::from_utf8(bytes).map_err(|e| CliError::Dte(sii::Error::Xml(e.to_string())))?;
    if let Some(pos) = str_val.find("<DTE") {
        if let Some(slice) = str_val.get(pos..) {
            if let Some(end_pos) = slice.rfind("</DTE>") {
                let end_total = pos.saturating_add(end_pos).saturating_add(6);
                if let Some(sub) = str_val.get(pos..end_total) {
                    return Ok(sub.as_bytes().to_vec());
                }
            }
            return Ok(slice.as_bytes().to_vec());
        }
    }
    Ok(bytes.to_vec())
}

fn build_and_sign_envelope(
    args: &EnvioEnviarArgs,
    ambiente: Ambiente,
) -> Result<(Vec<u8>, Rut, Rut), CliError> {
    let cert_path = args.cert.as_deref().ok_or_else(|| {
        CliError::MissingArgument("--cert (requerido para firmar el sobre de envío)".into())
    })?;
    let identity = load_identity(cert_path, args.cert_key.as_deref(), ambiente)?;

    let first_dte_bytes = std::fs::read(
        args.dte
            .first()
            .ok_or_else(|| CliError::MissingArgument("no se especificaron archivos DTE".into()))?,
    )
    .map_err(CliError::Io)?;

    let first_xml_str = std::str::from_utf8(&first_dte_bytes)
        .map_err(|e| CliError::Dte(sii::Error::Xml(format!("DTE no es UTF-8: {e}"))))?;

    let rut_emisor = if let Some(r) = &args.rut_emisor {
        Rut::parse(r)?
    } else {
        let r_str = extract_tag_text(first_xml_str, "RUTEmisor").ok_or_else(|| {
            CliError::Dte(sii::Error::Xml(
                "no se pudo inferir RUTEmisor del DTE; use --rut-emisor".into(),
            ))
        })?;
        Rut::parse(r_str)?
    };

    let rut_envia = if let Some(r) = &args.rut_envia {
        Rut::parse(r)?
    } else {
        rut_emisor
    };

    let fch_resol = if let Some(f) = &args.fch_resol {
        parse_date(f).map_err(CliError::Dte)?
    } else {
        time::Date::from_calendar_date(2026, time::Month::January, 1)
            .map_err(|e| CliError::Dte(sii::Error::InvalidDate(e.to_string())))?
    };

    let clock = ManualClock::new(time::OffsetDateTime::now_utc());

    // Inspect first DTE to determine if it is a boleta
    let tipo_str = extract_tag_text(first_xml_str, "TipoDTE")
        .ok_or_else(|| CliError::Dte(sii::Error::Xml("falta <TipoDTE> en el documento".into())))?;
    let tipo_code = tipo_str
        .trim()
        .parse::<u16>()
        .map_err(|e| CliError::Dte(sii::Error::Xml(format!("TipoDTE inválido: {e}"))))?;
    let is_boleta = TipoDte::try_from(tipo_code)
        .map_err(CliError::Dte)?
        .is_boleta();

    let envelope_bytes = if is_boleta {
        let mut envio = EnvioBoleta::nuevo(rut_emisor, rut_envia, fch_resol, args.nro_resol);
        for dte_path in &args.dte {
            let bytes = std::fs::read(dte_path).map_err(CliError::Io)?;
            let str_val = std::str::from_utf8(&bytes)
                .map_err(|e| CliError::Dte(sii::Error::Xml(e.to_string())))?;
            let code_str = extract_tag_text(str_val, "TipoDTE")
                .ok_or_else(|| CliError::Dte(sii::Error::Xml("falta <TipoDTE>".into())))?;
            let code = code_str
                .trim()
                .parse::<u16>()
                .map_err(|e| CliError::Dte(sii::Error::Xml(e.to_string())))?;
            let t = TipoDte::try_from(code).map_err(CliError::Dte)?;
            let clean_bytes = prepare_dte_bytes(&bytes)?;
            envio.agregar(t, &clean_bytes).map_err(CliError::Dte)?;
        }
        envio
            .firmar(&clock, &identity, &identity, SignatureOptions::default())
            .map_err(CliError::Signature)?
    } else {
        let mut envio = EnvioDte::nuevo(rut_emisor, rut_envia, fch_resol, args.nro_resol);
        for dte_path in &args.dte {
            let bytes = std::fs::read(dte_path).map_err(CliError::Io)?;
            let str_val = std::str::from_utf8(&bytes)
                .map_err(|e| CliError::Dte(sii::Error::Xml(e.to_string())))?;
            let code_str = extract_tag_text(str_val, "TipoDTE")
                .ok_or_else(|| CliError::Dte(sii::Error::Xml("falta <TipoDTE>".into())))?;
            let code = code_str
                .trim()
                .parse::<u16>()
                .map_err(|e| CliError::Dte(sii::Error::Xml(e.to_string())))?;
            let t = TipoDte::try_from(code).map_err(CliError::Dte)?;
            let clean_bytes = prepare_dte_bytes(&bytes)?;
            envio.agregar(t, &clean_bytes).map_err(CliError::Dte)?;
        }
        envio
            .firmar(&clock, &identity, &identity, SignatureOptions::default())
            .map_err(CliError::Signature)?
    };

    Ok((envelope_bytes, rut_emisor, rut_envia))
}

/// Executes `sii envio estado <trackid>`.
pub(crate) fn run_envio_estado(
    args: &EnvioEstadoArgs,
    is_json: bool,
    is_quiet: bool,
) -> Result<(), CliError> {
    let ambiente: Ambiente = args.ambiente.into();
    let transport = CliTransport::build(args.sim, args.sim_url.as_deref());
    let client = SiiClient::new(ambiente, transport);

    let token = if let Some(tok) = &args.token {
        Token::new(tok)
    } else {
        let cert_path = args.cert.as_deref().ok_or_else(|| {
            CliError::MissingArgument(
                "--cert (o --token) es requerido para autenticar la consulta de estado".into(),
            )
        })?;
        let identity = load_identity(cert_path, args.cert_key.as_deref(), ambiente)?;
        client
            .obtener_token(&identity, &identity)
            .map_err(|e| CliError::Transport(format!("falló la obtención de token del SII: {e}")))?
    };

    let rut_emisor_str = args
        .rut_emisor
        .as_deref()
        .ok_or_else(|| CliError::MissingArgument("--rut-emisor".into()))?;
    let rut_emisor = Rut::parse(rut_emisor_str)?;

    let estado = client
        .estado_con_rut(rut_emisor, args.track_id.as_str(), &token)
        .map_err(|e| CliError::Transport(format!("falló la consulta de estado: {e}")))?;

    let detalle = estado.detalle();
    let clasificacion = if estado.is_aceptado() {
        "Aceptado"
    } else if estado.is_rechazado() {
        "Rechazado"
    } else {
        "EnProceso"
    };

    if is_json {
        let payload = serde_json::json!({
            "track_id": args.track_id,
            "estado": estado.codigo(),
            "clasificacion": clasificacion,
            "glosa": estado.glosa(),
            "tipo_docto": detalle.tipo_docto,
            "informados": detalle.informados,
            "aceptados": detalle.aceptados,
            "rechazados": detalle.rechazados,
            "reparos": detalle.reparos,
        });
        print_json(&payload);
    } else if !is_quiet {
        print_human(&format!("Estado del envío Track ID {}:", args.track_id));
        print_human(&format!("Estado SII: {}", estado.codigo()));
        print_human(&format!("Clasificación: {clasificacion}"));
        print_human(&format!("Glosa: {}", estado.glosa()));
        if let Some(tipo) = detalle.tipo_docto {
            print_human(&format!("Tipo Documento: {tipo}"));
        }
        if let Some(n) = detalle.informados {
            print_human(&format!("Documentos Informados: {n}"));
        }
        if let Some(n) = detalle.aceptados {
            print_human(&format!("Documentos Aceptados: {n}"));
        }
        if let Some(n) = detalle.rechazados {
            print_human(&format!("Documentos Rechazados: {n}"));
        }
        if let Some(n) = detalle.reparos {
            print_human(&format!("Documentos con Reparos: {n}"));
        }
    }

    Ok(())
}
