//! Handlers for `sii libro` subcommands (`build`).

use crate::args::LibroBuildArgs;
use crate::commands::cert::load_identity;
use crate::commands::xml_utils::{extract_tag_text, write_output};
use crate::error::CliError;
use crate::output::{print_human, print_json};
use sii::codes::{Ambiente, TipoDte};
use sii::dte::parse_date;
use sii::libro::{
    DetalleLibro, LibroCompraVenta, PeriodoTributario, TipoEnvio, TipoLibro, TipoOperacion,
};
use sii::signature::SignatureOptions;
use sii::types::{ManualClock, Monto, Rut};
use std::path::Path;

#[derive(serde::Deserialize)]
struct LibroInput {
    rut_emisor: String,
    rut_envia: Option<String>,
    periodo: String,
    fch_resol: Option<String>,
    nro_resol: Option<u32>,
    operacion: Option<String>,
    tipo_libro: Option<String>,
    tipo_envio: Option<String>,
    detalles: Option<Vec<DetalleLibro>>,
}

/// Executes `sii libro build`.
pub(crate) fn run_libro_build(
    args: &LibroBuildArgs,
    is_json: bool,
    is_quiet: bool,
) -> Result<(), CliError> {
    let mut libro = if let Some(input_path) = &args.input {
        build_libro_from_json(input_path)?
    } else {
        build_libro_from_args(args)?
    };

    // Add any specified DTE files as ledger details
    for dte_path in &args.dte {
        let detalle = extract_detalle_from_dte_file(dte_path)?;
        libro.agregar_detalle(detalle);
    }

    let clock = ManualClock::new(time::OffsetDateTime::now_utc());

    // Output XML (signed if --cert is supplied, otherwise unsigned)
    let (output_bytes, was_signed) = if let Some(cert_path) = &args.cert {
        let amb_arg = args.ambiente.ok_or_else(|| {
            CliError::MissingArgument(
                "--ambiente (requerido para firmar el libro de compra/venta)".into(),
            )
        })?;
        let ambiente: Ambiente = amb_arg.into();
        let identity = load_identity(cert_path, args.cert_key.as_deref(), ambiente)?;
        let signed = libro
            .firmar(&clock, &identity, &identity, SignatureOptions::default())
            .map_err(CliError::Signature)?;
        (signed, true)
    } else {
        let xml = libro.to_xml(&clock).map_err(CliError::Dte)?;
        (xml, false)
    };

    write_output(&output_bytes, args.output.as_deref())?;

    let caratula = libro.caratula();
    let num_docs = libro.len();

    if is_json {
        let payload = serde_json::json!({
            "status": "ok",
            "periodo": caratula.periodo_tributario.to_string(),
            "tipo_operacion": caratula.tipo_operacion.to_string(),
            "tipo_libro": caratula.tipo_libro.to_string(),
            "tipo_envio": caratula.tipo_envio.to_string(),
            "rut_emisor": caratula.rut_emisor_libro.to_string(),
            "total_documentos": num_docs,
            "firmado": was_signed,
            "output_file": args.output.as_ref().map(|p| p.display().to_string()),
        });
        print_json(&payload);
    } else if !is_quiet {
        print_human(&format!(
            "Libro de {} ({}) período {} generado exitosamente ({} documentos{}).",
            caratula.tipo_operacion,
            caratula.tipo_libro,
            caratula.periodo_tributario,
            num_docs,
            if was_signed {
                ", firmado digitalmente"
            } else {
                ""
            }
        ));
        if let Some(out) = &args.output {
            print_human(&format!("Guardado en: {}", out.display()));
        }
    }

    Ok(())
}

fn build_libro_from_json(path: &Path) -> Result<LibroCompraVenta, CliError> {
    let bytes = std::fs::read(path).map_err(CliError::Io)?;
    let input: LibroInput = serde_json::from_slice(&bytes).map_err(CliError::Json)?;

    let rut_emisor = Rut::parse(&input.rut_emisor)?;
    let rut_envia = if let Some(r) = &input.rut_envia {
        Rut::parse(r)?
    } else {
        rut_emisor
    };

    let periodo = PeriodoTributario::parse(&input.periodo).map_err(CliError::Dte)?;

    let fch_resol = if let Some(f) = &input.fch_resol {
        parse_date(f).map_err(CliError::Dte)?
    } else {
        time::Date::from_calendar_date(2026, time::Month::January, 1)
            .map_err(|e| CliError::Dte(sii::Error::InvalidDate(e.to_string())))?
    };

    let nro_resol = input.nro_resol.unwrap_or(0);
    let operacion = parse_operacion(input.operacion.as_deref().unwrap_or("venta"))?;
    let tipo_libro = parse_tipo_libro(input.tipo_libro.as_deref().unwrap_or("mensual"))?;
    let tipo_envio = parse_tipo_envio(input.tipo_envio.as_deref().unwrap_or("total"))?;

    let mut libro = LibroCompraVenta::nuevo(
        rut_emisor, rut_envia, periodo, fch_resol, nro_resol, operacion,
    )
    .tipo_libro(tipo_libro)
    .tipo_envio(tipo_envio);

    if let Some(detalles) = input.detalles {
        for d in detalles {
            libro.agregar_detalle(d);
        }
    }

    Ok(libro)
}

fn build_libro_from_args(args: &LibroBuildArgs) -> Result<LibroCompraVenta, CliError> {
    let rut_emisor_str = args
        .rut_emisor
        .as_deref()
        .ok_or_else(|| CliError::MissingArgument("--rut-emisor".into()))?;
    let rut_emisor = Rut::parse(rut_emisor_str)?;

    let rut_envia = if let Some(r) = &args.rut_envia {
        Rut::parse(r)?
    } else {
        rut_emisor
    };

    let periodo_str = args
        .periodo
        .as_deref()
        .ok_or_else(|| CliError::MissingArgument("--periodo (formato YYYY-MM)".into()))?;
    let periodo = PeriodoTributario::parse(periodo_str).map_err(CliError::Dte)?;

    let fch_resol = if let Some(f) = &args.fch_resol {
        parse_date(f).map_err(CliError::Dte)?
    } else {
        time::Date::from_calendar_date(2026, time::Month::January, 1)
            .map_err(|e| CliError::Dte(sii::Error::InvalidDate(e.to_string())))?
    };

    let operacion = parse_operacion(&args.operacion)?;
    let tipo_libro = parse_tipo_libro(&args.tipo_libro)?;
    let tipo_envio = parse_tipo_envio(&args.tipo_envio)?;

    Ok(LibroCompraVenta::nuevo(
        rut_emisor,
        rut_envia,
        periodo,
        fch_resol,
        args.nro_resol,
        operacion,
    )
    .tipo_libro(tipo_libro)
    .tipo_envio(tipo_envio))
}

fn parse_operacion(s: &str) -> Result<TipoOperacion, CliError> {
    match s.trim().to_ascii_lowercase().as_str() {
        "compra" => Ok(TipoOperacion::Compra),
        "venta" => Ok(TipoOperacion::Venta),
        other => Err(CliError::InvalidInput(format!(
            "tipo de operación inválido '{other}' (esperado: 'compra' o 'venta')"
        ))),
    }
}

fn parse_tipo_libro(s: &str) -> Result<TipoLibro, CliError> {
    match s.trim().to_ascii_lowercase().as_str() {
        "mensual" => Ok(TipoLibro::Mensual),
        "especial" => Ok(TipoLibro::Especial),
        "rectifica" => Ok(TipoLibro::Rectifica),
        other => Err(CliError::InvalidInput(format!(
            "tipo de libro inválido '{other}' (esperado: 'mensual', 'especial' o 'rectifica')"
        ))),
    }
}

fn parse_tipo_envio(s: &str) -> Result<TipoEnvio, CliError> {
    match s.trim().to_ascii_lowercase().as_str() {
        "total" => Ok(TipoEnvio::Total),
        "parcial" => Ok(TipoEnvio::Parcial),
        "final" => Ok(TipoEnvio::Final),
        "ajuste" => Ok(TipoEnvio::Ajuste),
        other => Err(CliError::InvalidInput(format!(
            "tipo de envío inválido '{other}' (esperado: 'total', 'parcial', 'final' o 'ajuste')"
        ))),
    }
}

fn extract_detalle_from_dte_file(path: &Path) -> Result<DetalleLibro, CliError> {
    let bytes = std::fs::read(path).map_err(CliError::Io)?;
    let xml_str = std::str::from_utf8(&bytes)
        .map_err(|e| CliError::Dte(sii::Error::Xml(format!("DTE no es UTF-8: {e}"))))?;

    let tipo_str = extract_tag_text(xml_str, "TipoDTE")
        .ok_or_else(|| CliError::Dte(sii::Error::Xml("falta <TipoDTE>".into())))?;
    let tipo_code = tipo_str
        .trim()
        .parse::<u16>()
        .map_err(|e| CliError::Dte(sii::Error::Xml(e.to_string())))?;
    let tipo = TipoDte::try_from(tipo_code).map_err(CliError::Dte)?;

    let folio_str = extract_tag_text(xml_str, "Folio")
        .ok_or_else(|| CliError::Dte(sii::Error::Xml("falta <Folio>".into())))?;
    let folio = folio_str
        .trim()
        .parse::<u64>()
        .map_err(|e| CliError::Dte(sii::Error::Xml(e.to_string())))?;

    let mut detalle = DetalleLibro::new(tipo, folio);

    if let Some(f_str) = extract_tag_text(xml_str, "FchEmis") {
        if let Ok(f) = parse_date(f_str) {
            detalle.fch_doc = Some(f);
        }
    }

    if let Some(r_str) = extract_tag_text(xml_str, "RUTRecep") {
        if let Ok(r) = Rut::parse(r_str) {
            detalle.rut_doc = Some(r);
        }
    }

    if let Some(rs_str) = extract_tag_text(xml_str, "RznSocRecep") {
        detalle.rzn_soc = Some(rs_str.to_string());
    }

    if let Some(neto_str) = extract_tag_text(xml_str, "MntNeto") {
        if let Ok(n) = neto_str.trim().parse::<i64>() {
            detalle.mnt_neto = Some(Monto::from_pesos(n));
        }
    }

    if let Some(iva_str) = extract_tag_text(xml_str, "IVA") {
        if let Ok(i) = iva_str.trim().parse::<i64>() {
            detalle.mnt_iva = Some(Monto::from_pesos(i));
        }
    }

    if let Some(tot_str) = extract_tag_text(xml_str, "MntTotal") {
        if let Ok(t) = tot_str.trim().parse::<i64>() {
            detalle.mnt_total = Some(Monto::from_pesos(t));
        }
    }

    if let Some(exe_str) = extract_tag_text(xml_str, "MntExe") {
        if let Ok(e) = exe_str.trim().parse::<i64>() {
            detalle.mnt_exe = Some(Monto::from_pesos(e));
        }
    }

    Ok(detalle)
}
