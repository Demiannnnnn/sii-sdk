//! Handlers for `sii dte` subcommands (`build`, `timbrar`, `firmar`).

use crate::args::{DteBuildArgs, DteFirmarArgs, DteTimbrarArgs};
use crate::commands::cert::load_identity;
use crate::commands::xml_utils::{extract_tag_text, write_output};
use crate::error::CliError;
use crate::output::{print_human, print_json};
use sii::Clock;
use sii::caf::Caf;
use sii::codes::{Ambiente, TipoDte};
use sii::dte::{Detalle, Dte, Emisor, Receptor, Referencia, parse_date};
use sii::signature::sign_dte;
use sii::ted::{Dd, Ted, clean_truncate};
use sii::types::{ManualClock, Monto, Rut, format_timestamp};

/// Executes `sii dte build`.
pub(crate) fn run_dte_build(
    args: &DteBuildArgs,
    is_json: bool,
    is_quiet: bool,
) -> Result<(), CliError> {
    let dte = if let Some(input_path) = &args.input {
        let content = std::fs::read(input_path).map_err(CliError::Io)?;
        serde_json::from_slice::<Dte>(&content).map_err(CliError::Json)?
    } else {
        build_dte_from_args(args)?
    };

    let output_bytes = if args.format.eq_ignore_ascii_case("json") {
        serde_json::to_vec_pretty(&dte).map_err(CliError::Json)?
    } else {
        dte.to_xml().map_err(CliError::Dte)?
    };

    write_output(&output_bytes, args.output.as_deref())?;

    if is_json {
        let payload = serde_json::json!({
            "status": "ok",
            "tipo": dte.tipo().code(),
            "tipo_nombre": dte.tipo().description(),
            "folio": dte.folio(),
            "emisor": dte.encabezado().emisor.rut_emisor.to_string(),
            "receptor": dte.encabezado().receptor.rut_recep.to_string(),
            "monto_total": dte.total().as_pesos(),
            "output_file": args.output.as_ref().map(|p| p.display().to_string()),
            "format": args.format,
        });
        print_json(&payload);
    } else if !is_quiet {
        print_human(&format!(
            "DTE {} ({}) folio {} construido exitosamente (total: ${}).",
            dte.tipo().code(),
            dte.tipo().description(),
            dte.folio(),
            dte.total().as_pesos()
        ));
        if let Some(out) = &args.output {
            print_human(&format!("Guardado en: {}", out.display()));
        }
    }

    Ok(())
}

fn build_emisor_from_args(args: &DteBuildArgs) -> Result<Emisor, CliError> {
    let emisor_rut_str = args
        .emisor_rut
        .as_deref()
        .ok_or_else(|| CliError::MissingArgument("--emisor-rut".into()))?;
    let emisor_rut = Rut::parse(emisor_rut_str)?;

    let emisor_razon = args
        .emisor_razon
        .as_deref()
        .ok_or_else(|| CliError::MissingArgument("--emisor-razon".into()))?;
    let emisor_giro = args
        .emisor_giro
        .as_deref()
        .ok_or_else(|| CliError::MissingArgument("--emisor-giro".into()))?;
    let emisor_acteco = args.emisor_acteco.unwrap_or(465_900);
    let emisor_dir = args
        .emisor_dir
        .as_deref()
        .ok_or_else(|| CliError::MissingArgument("--emisor-dir".into()))?;
    let emisor_comuna = args
        .emisor_comuna
        .as_deref()
        .ok_or_else(|| CliError::MissingArgument("--emisor-comuna".into()))?;

    let mut emisor = Emisor::new(
        emisor_rut,
        emisor_razon,
        emisor_giro,
        emisor_acteco,
        emisor_dir,
        emisor_comuna,
    );
    if let Some(ciudad) = &args.emisor_ciudad {
        emisor = emisor.ciudad_origen(ciudad);
    }
    Ok(emisor)
}

fn build_receptor_from_args(args: &DteBuildArgs) -> Result<Receptor, CliError> {
    let receptor_rut_str = args
        .receptor_rut
        .as_deref()
        .ok_or_else(|| CliError::MissingArgument("--receptor-rut".into()))?;
    let receptor_rut = Rut::parse(receptor_rut_str)?;

    let receptor_razon = args
        .receptor_razon
        .as_deref()
        .ok_or_else(|| CliError::MissingArgument("--receptor-razon".into()))?;
    let receptor_giro = args.receptor_giro.as_deref().unwrap_or("PARTICULAR");
    let receptor_dir = args.receptor_dir.as_deref().unwrap_or("SANTIAGO");
    let receptor_comuna = args.receptor_comuna.as_deref().unwrap_or("SANTIAGO");

    let mut receptor = Receptor::new(
        receptor_rut,
        receptor_razon,
        receptor_giro,
        receptor_dir,
        receptor_comuna,
    );
    if let Some(ciudad) = &args.receptor_ciudad {
        receptor = receptor.ciudad_recep(ciudad);
    }
    Ok(receptor)
}

fn build_detalle_from_args(args: &DteBuildArgs) -> Result<Detalle, CliError> {
    let item_nombre = args
        .item_nombre
        .as_deref()
        .ok_or_else(|| CliError::MissingArgument("--item-nombre".into()))?;
    let item_precio = args
        .item_precio
        .ok_or_else(|| CliError::MissingArgument("--item-precio".into()))?;
    let monto_item = Monto::from_pesos(item_precio);
    let cantidad = args.item_cantidad.unwrap_or(1);

    Ok(Detalle::new(1, item_nombre, monto_item)
        .qty(cantidad)
        .prc(monto_item))
}

fn build_referencia_from_args(args: &DteBuildArgs) -> Result<Referencia, CliError> {
    let ref_tipo = args
        .ref_tipo
        .as_deref()
        .ok_or_else(|| CliError::MissingArgument("--ref-tipo".into()))?;
    let ref_folio = args
        .ref_folio
        .as_deref()
        .ok_or_else(|| CliError::MissingArgument("--ref-folio".into()))?;
    let ref_fecha_str = args
        .ref_fecha
        .as_deref()
        .ok_or_else(|| CliError::MissingArgument("--ref-fecha".into()))?;
    let ref_fecha = parse_date(ref_fecha_str).map_err(CliError::Dte)?;
    let ref_codigo = args
        .ref_codigo
        .ok_or_else(|| CliError::MissingArgument("--ref-codigo (1, 2 o 3)".into()))?;
    let ref_razon = args
        .ref_razon
        .as_deref()
        .ok_or_else(|| CliError::MissingArgument("--ref-razon".into()))?;

    Ok(Referencia {
        nro_lin_ref: 1,
        tpo_doc_ref: ref_tipo.to_string(),
        ind_global: None,
        folio_ref: ref_folio.to_string(),
        rut_otr: None,
        fch_ref: ref_fecha,
        cod_ref: Some(ref_codigo),
        razon_ref: Some(ref_razon.to_string()),
    })
}

fn build_dte_from_args(args: &DteBuildArgs) -> Result<Dte, CliError> {
    let tipo_code = args
        .tipo
        .ok_or_else(|| CliError::MissingArgument("--tipo (código DTE: 33, 34, 39, etc.)".into()))?;
    let tipo = TipoDte::try_from(tipo_code).map_err(CliError::Dte)?;
    let folio = args.folio.unwrap_or(1);

    let fecha = if let Some(f) = &args.fecha {
        parse_date(f).map_err(CliError::Dte)?
    } else {
        time::OffsetDateTime::now_utc().date()
    };

    let emisor = build_emisor_from_args(args)?;
    let receptor = build_receptor_from_args(args)?;
    let detalle = build_detalle_from_args(args)?;
    let referencia = if tipo.requires_reference() {
        Some(build_referencia_from_args(args)?)
    } else {
        None
    };

    build_dte_instance(tipo, folio, fecha, emisor, receptor, detalle, referencia)
}

fn build_dte_instance(
    tipo: TipoDte,
    folio: u64,
    fecha: time::Date,
    emisor: Emisor,
    receptor: Receptor,
    detalle: Detalle,
    referencia: Option<Referencia>,
) -> Result<Dte, CliError> {
    match tipo {
        TipoDte::FacturaAfecta => Dte::factura_afecta()
            .folio(folio)
            .fch_emis(fecha)
            .emisor(emisor)
            .receptor(receptor)
            .item(detalle)
            .build()
            .map_err(CliError::Dte),
        TipoDte::FacturaExenta => Dte::factura_exenta()
            .folio(folio)
            .fch_emis(fecha)
            .emisor(emisor)
            .receptor(receptor)
            .item(detalle)
            .build()
            .map_err(CliError::Dte),
        TipoDte::Boleta => Dte::boleta()
            .folio(folio)
            .fch_emis(fecha)
            .emisor(emisor)
            .receptor(receptor)
            .item(detalle)
            .build()
            .map_err(CliError::Dte),
        TipoDte::BoletaExenta => Dte::boleta_exenta()
            .folio(folio)
            .fch_emis(fecha)
            .emisor(emisor)
            .receptor(receptor)
            .item(detalle)
            .build()
            .map_err(CliError::Dte),
        TipoDte::GuiaDespacho => Dte::guia_despacho()
            .folio(folio)
            .fch_emis(fecha)
            .emisor(emisor)
            .receptor(receptor)
            .item(detalle)
            .ind_traslado(1)
            .build()
            .map_err(CliError::Dte),
        TipoDte::NotaDebito => {
            let mut b = Dte::nota_debito()
                .folio(folio)
                .fch_emis(fecha)
                .emisor(emisor)
                .receptor(receptor)
                .item(detalle);
            if let Some(r) = referencia {
                b = b.referencia(r);
            }
            b.build().map_err(CliError::Dte)
        }
        TipoDte::NotaCredito => {
            let mut b = Dte::nota_credito()
                .folio(folio)
                .fch_emis(fecha)
                .emisor(emisor)
                .receptor(receptor)
                .item(detalle);
            if let Some(r) = referencia {
                b = b.referencia(r);
            }
            b.build().map_err(CliError::Dte)
        }
    }
}

/// Executes `sii dte timbrar`.
pub(crate) fn run_dte_timbrar(
    args: &DteTimbrarArgs,
    is_json: bool,
    is_quiet: bool,
) -> Result<(), CliError> {
    let caf_bytes = std::fs::read(&args.caf).map_err(|e| {
        CliError::Caf(sii::Error::InvalidCaf(format!(
            "no se pudo leer el archivo CAF {}: {e}",
            args.caf.display()
        )))
    })?;
    let caf = Caf::parse(&caf_bytes).map_err(CliError::Caf)?;

    let dte_bytes = std::fs::read(&args.dte).map_err(CliError::Io)?;

    let clock_time = if let Some(ts) = &args.timestamp {
        time::OffsetDateTime::parse(ts, &time::format_description::well_known::Rfc3339)
            .map_err(|e| CliError::InvalidDate(format!("timestamp inválido: {e}")))?
    } else {
        time::OffsetDateTime::now_utc()
    };
    let clock = ManualClock::new(clock_time);

    let is_json_input = dte_bytes.starts_with(b"{");

    let (stamped_xml, folio_stamped, dte_tipo) = if is_json_input {
        let dte = serde_json::from_slice::<Dte>(&dte_bytes).map_err(CliError::Json)?;
        let folio = args.folio.unwrap_or(dte.folio());
        let stamped = dte.timbrar(&caf, folio, &clock).map_err(CliError::Dte)?;
        let xml = stamped.to_xml().map_err(CliError::Dte)?;
        (xml, folio, dte.tipo())
    } else {
        stamp_xml_dte(&dte_bytes, &caf, args.folio, &clock)?
    };

    write_output(&stamped_xml, args.output.as_deref())?;

    let (min_f, max_f) = caf.rango();

    if is_json {
        let payload = serde_json::json!({
            "status": "ok",
            "folio": folio_stamped,
            "tipo": dte_tipo.code(),
            "caf_rango_desde": min_f,
            "caf_rango_hasta": max_f,
            "output_file": args.output.as_ref().map(|p| p.display().to_string()),
        });
        print_json(&payload);
    } else if !is_quiet {
        print_human(&format!(
            "DTE {} timbrado exitosamente con folio {} (rango CAF: {}-{}).",
            dte_tipo.code(),
            folio_stamped,
            min_f,
            max_f
        ));
        if let Some(out) = &args.output {
            print_human(&format!("Guardado en: {}", out.display()));
        }
    }

    Ok(())
}

fn stamp_xml_dte(
    xml_bytes: &[u8],
    caf: &Caf,
    explicit_folio: Option<u64>,
    clock: &ManualClock,
) -> Result<(Vec<u8>, u64, TipoDte), CliError> {
    let xml_str = std::str::from_utf8(xml_bytes)
        .map_err(|e| CliError::Dte(sii::Error::Xml(format!("XML no es UTF-8: {e}"))))?;

    let tipo_str = extract_tag_text(xml_str, "TipoDTE")
        .ok_or_else(|| CliError::Dte(sii::Error::Xml("falta etiqueta <TipoDTE>".into())))?;
    let tipo_code = tipo_str
        .trim()
        .parse::<u16>()
        .map_err(|e| CliError::Dte(sii::Error::Xml(format!("código TipoDTE inválido: {e}"))))?;
    let tipo = TipoDte::try_from(tipo_code).map_err(CliError::Dte)?;

    let folio = if let Some(f) = explicit_folio {
        f
    } else {
        let folio_str = extract_tag_text(xml_str, "Folio")
            .ok_or_else(|| CliError::Dte(sii::Error::Xml("falta etiqueta <Folio>".into())))?;
        folio_str
            .trim()
            .parse::<u64>()
            .map_err(|e| CliError::Dte(sii::Error::Xml(format!("folio inválido: {e}"))))?
    };

    let fch_emis_str = extract_tag_text(xml_str, "FchEmis")
        .ok_or_else(|| CliError::Dte(sii::Error::Xml("falta etiqueta <FchEmis>".into())))?;
    let fch_emis = parse_date(fch_emis_str).map_err(CliError::Dte)?;

    let emisor_str = extract_tag_text(xml_str, "RUTEmisor")
        .ok_or_else(|| CliError::Dte(sii::Error::Xml("falta etiqueta <RUTEmisor>".into())))?;
    let re = Rut::parse(emisor_str)?;

    let receptor_str = extract_tag_text(xml_str, "RUTRecep")
        .ok_or_else(|| CliError::Dte(sii::Error::Xml("falta etiqueta <RUTRecep>".into())))?;
    let rr = Rut::parse(receptor_str)?;

    let rsr_str = extract_tag_text(xml_str, "RznSocRecep").unwrap_or("CLIENTE");
    let it1_str = extract_tag_text(xml_str, "NmbItem").unwrap_or("ITEM");

    let mnt_str = extract_tag_text(xml_str, "MntTotal")
        .ok_or_else(|| CliError::Dte(sii::Error::Xml("falta etiqueta <MntTotal>".into())))?;
    let mnt_val = mnt_str
        .trim()
        .parse::<i64>()
        .map_err(|e| CliError::Dte(sii::Error::Xml(format!("monto inválido: {e}"))))?;
    let mnt = Monto::from_pesos(mnt_val);

    caf.verificar_folio(folio).map_err(CliError::Caf)?;
    if caf.tipo() != tipo {
        return Err(CliError::Caf(sii::Error::DteValidation(format!(
            "el tipo de DTE ({tipo}) no coincide con el tipo del CAF ({})",
            caf.tipo()
        ))));
    }
    if caf.emisor_rut() != &re {
        return Err(CliError::Caf(sii::Error::DteValidation(format!(
            "el RUT emisor ({re}) no coincide con el RUT del CAF ({})",
            caf.emisor_rut()
        ))));
    }

    let dd = Dd {
        re,
        td: tipo,
        f: folio,
        fe: fch_emis,
        rr,
        rsr: clean_truncate(rsr_str, 40),
        mnt,
        it1: clean_truncate(it1_str, 40),
        caf_xml: caf.raw_xml().to_vec(),
        tsted: format_timestamp(&clock.now()),
    };

    let ted = Ted::new(dd, caf).map_err(CliError::Dte)?;

    let doc_close = "</Documento>";
    let pos = xml_str.find(doc_close).ok_or_else(|| {
        CliError::Dte(sii::Error::Xml(
            "falta etiqueta de cierre </Documento>".into(),
        ))
    })?;

    let before = xml_str.get(..pos).unwrap_or(xml_str);
    let after = xml_str.get(pos..).unwrap_or_default();

    let mut result = Vec::with_capacity(xml_bytes.len().saturating_add(ted.raw_ted().len()));
    result.extend_from_slice(before.as_bytes());
    result.extend_from_slice(ted.raw_ted());
    result.extend_from_slice(after.as_bytes());

    Ok((result, folio, tipo))
}

/// Executes `sii dte firmar`.
pub(crate) fn run_dte_firmar(
    args: &DteFirmarArgs,
    is_json: bool,
    is_quiet: bool,
) -> Result<(), CliError> {
    let ambiente: Ambiente = args.ambiente.into();
    let identity = load_identity(&args.cert, args.cert_key.as_deref(), ambiente)?;

    let dte_bytes = std::fs::read(&args.dte).map_err(CliError::Io)?;

    let signed_bytes = sign_dte(&dte_bytes, &identity, &identity).map_err(CliError::Signature)?;

    write_output(&signed_bytes, args.output.as_deref())?;

    if is_json {
        let payload = serde_json::json!({
            "status": "ok",
            "ambiente": format!("{ambiente:?}"),
            "algoritmo": format!("{:?}", identity.signature_algorithm()),
            "output_file": args.output.as_ref().map(|p| p.display().to_string()),
        });
        print_json(&payload);
    } else if !is_quiet {
        print_human(&format!(
            "DTE firmado exitosamente con XMLDSig (algoritmo: {:?}, ambiente: {:?}).",
            identity.signature_algorithm(),
            ambiente
        ));
        if let Some(out) = &args.output {
            print_human(&format!("Guardado en: {}", out.display()));
        }
    }

    Ok(())
}
