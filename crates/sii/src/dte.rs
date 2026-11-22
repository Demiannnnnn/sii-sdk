//! DTE (Documento Tributario Electrónico) models, builders, and SII XML generation.
//!
//! Provides typed representations of electronic tax documents compliant with the Chilean
//! Servicio de Impuestos Internos (SII) specifications (`DTE_v10.xsd`, `docs/dte-spec.md` §3).
//!
//! This initial cut implements models and XML generation for:
//! - Factura Electrónica (afecta a IVA 19 %) - Código SII 33
//! - Factura No Afecta o Exenta Electrónica - Código SII 34

use crate::caf::Caf;
use crate::codes::TipoDte;
use crate::error::Error;
use crate::ted::{Dd, Ted};
use crate::types::{Clock, Monto, Rut};
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::writer::Writer;
use std::io::Cursor;

/// Formats a `time::Date` as `YYYY-MM-DD`.
#[must_use]
pub fn format_date(date: &time::Date) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        date.month() as u8,
        date.day()
    )
}

/// Parses a `YYYY-MM-DD` string into a `time::Date`.
///
/// # Errors
///
/// Returns [`Error::InvalidDate`] if the string is not formatted as `YYYY-MM-DD`
/// or represents an invalid calendar date.
pub fn parse_date(s: &str) -> Result<time::Date, Error> {
    let parts: Vec<&str> = s.trim().split('-').collect();
    if parts.len() != 3 {
        return Err(Error::InvalidDate(format!(
            "expected YYYY-MM-DD format, got: {s}"
        )));
    }
    let year_str = parts
        .first()
        .ok_or_else(|| Error::InvalidDate(s.to_string()))?;
    let month_str = parts
        .get(1)
        .ok_or_else(|| Error::InvalidDate(s.to_string()))?;
    let day_str = parts
        .get(2)
        .ok_or_else(|| Error::InvalidDate(s.to_string()))?;

    let year = year_str
        .parse::<i32>()
        .map_err(|e| Error::InvalidDate(format!("invalid year in {s}: {e}")))?;
    let month_num = month_str
        .parse::<u8>()
        .map_err(|e| Error::InvalidDate(format!("invalid month in {s}: {e}")))?;
    let day = day_str
        .parse::<u8>()
        .map_err(|e| Error::InvalidDate(format!("invalid day in {s}: {e}")))?;

    let month = time::Month::try_from(month_num)
        .map_err(|e| Error::InvalidDate(format!("invalid month {month_num} in {s}: {e}")))?;

    time::Date::from_calendar_date(year, month, day)
        .map_err(|e| Error::InvalidDate(format!("invalid calendar date {s}: {e}")))
}

#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize)]
struct IdDocHelper {
    tipo_dte: TipoDte,
    folio: u64,
    fch_emis: String,
    fch_venc: Option<String>,
    ind_servicio: Option<u8>,
    term_pago_cdg: Option<u8>,
    fma_pago: Option<u8>,
    tipo_despacho: Option<u8>,
    ind_traslado: Option<u8>,
}

#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize)]
struct ReferenciaHelper {
    nro_lin_ref: u32,
    tpo_doc_ref: String,
    ind_global: Option<u8>,
    folio_ref: String,
    rut_otr: Option<String>,
    fch_ref: String,
    cod_ref: Option<u8>,
    razon_ref: Option<String>,
}

/// Reference to another tax document or commercial transaction (`Referencia` in SII schema).
///
/// Mandatory for Credit Notes (Tipo 61) and Debit Notes (Tipo 56) in accordance with
/// rule DTE-02 and Chilean SII regulations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Referencia {
    /// Sequential reference line number (`NroLinRef`, 1..=40).
    pub nro_lin_ref: u32,
    /// Referenced document type code or string (`TpoDocRef`, e.g. `"33"`, `"34"`, `"SET"`).
    pub tpo_doc_ref: String,
    /// Global reference indicator (`IndGlobal`, 1 = global reference across documents).
    pub ind_global: Option<u8>,
    /// Referenced document folio number or alphanumeric reference (`FolioRef`).
    pub folio_ref: String,
    /// Other issuer RUT if referencing a third-party document (`RUTOtr`).
    pub rut_otr: Option<String>,
    /// Referenced document date (`FchRef`).
    pub fch_ref: time::Date,
    /// Reference action code (`CodRef`: 1 = Anula, 2 = Corrige texto, 3 = Corrige montos).
    pub cod_ref: Option<u8>,
    /// Reason or description of the reference (`RazonRef`, max 90 characters).
    pub razon_ref: Option<String>,
}

#[cfg(feature = "serde")]
impl serde::Serialize for Referencia {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let helper = ReferenciaHelper {
            nro_lin_ref: self.nro_lin_ref,
            tpo_doc_ref: self.tpo_doc_ref.clone(),
            ind_global: self.ind_global,
            folio_ref: self.folio_ref.clone(),
            rut_otr: self.rut_otr.clone(),
            fch_ref: format_date(&self.fch_ref),
            cod_ref: self.cod_ref,
            razon_ref: self.razon_ref.clone(),
        };
        helper.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Referencia {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = ReferenciaHelper::deserialize(deserializer)?;
        let fch_ref = parse_date(&helper.fch_ref).map_err(serde::de::Error::custom)?;
        Ok(Self {
            nro_lin_ref: helper.nro_lin_ref,
            tpo_doc_ref: helper.tpo_doc_ref,
            ind_global: helper.ind_global,
            folio_ref: helper.folio_ref,
            rut_otr: helper.rut_otr,
            fch_ref,
            cod_ref: helper.cod_ref,
            razon_ref: helper.razon_ref,
        })
    }
}

impl Referencia {
    /// Creates a new `Referencia` with standard mandatory fields.
    #[must_use]
    pub fn new(
        nro_lin_ref: u32,
        tpo_doc_ref: impl Into<String>,
        folio_ref: impl Into<String>,
        fch_ref: time::Date,
    ) -> Self {
        Self {
            nro_lin_ref,
            tpo_doc_ref: tpo_doc_ref.into(),
            ind_global: None,
            folio_ref: folio_ref.into(),
            rut_otr: None,
            fch_ref,
            cod_ref: None,
            razon_ref: None,
        }
    }

    /// Creates a new `Referencia` with action code and justification (standard for Credit/Debit notes).
    #[must_use]
    pub fn with_reason(
        nro_lin_ref: u32,
        tpo_doc_ref: impl Into<String>,
        folio_ref: impl Into<String>,
        fch_ref: time::Date,
        cod_ref: u8,
        razon_ref: impl Into<String>,
    ) -> Self {
        Self {
            nro_lin_ref,
            tpo_doc_ref: tpo_doc_ref.into(),
            ind_global: None,
            folio_ref: folio_ref.into(),
            rut_otr: None,
            fch_ref,
            cod_ref: Some(cod_ref),
            razon_ref: Some(razon_ref.into()),
        }
    }

    /// Sets the action code (`CodRef`: 1 = Anula, 2 = Corrige texto, 3 = Corrige montos).
    #[must_use]
    pub const fn cod_ref(mut self, cod: u8) -> Self {
        self.cod_ref = Some(cod);
        self
    }

    /// Sets the reason or description for this reference (`RazonRef`).
    #[must_use]
    pub fn razon_ref(mut self, razon: impl Into<String>) -> Self {
        self.razon_ref = Some(razon.into());
        self
    }

    /// Sets the global reference indicator (`IndGlobal`).
    #[must_use]
    pub const fn ind_global(mut self, ind: u8) -> Self {
        self.ind_global = Some(ind);
        self
    }

    /// Sets another issuer tax ID (`RUTOtr`).
    #[must_use]
    pub fn rut_otr(mut self, rut: impl Into<String>) -> Self {
        self.rut_otr = Some(rut.into());
        self
    }
}

/// Transport and carrier details (`Transporte` in SII schema).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Transporte {
    /// Vehicle license plate (`Patente`, e.g. `"AB1234"`).
    pub patente: Option<String>,
    /// Carrier company or independent transporter RUT (`RUTTrans`).
    pub rut_trans: Option<Rut>,
    /// Driver information (`Chofer`).
    pub chofer: Option<Chofer>,
    /// Destination physical address (`DirDest`).
    pub dir_dest: Option<String>,
    /// Destination commune (`CmnaDest`).
    pub cmna_dest: Option<String>,
    /// Destination city (`CiudadDest`).
    pub ciudad_dest: Option<String>,
}

impl Transporte {
    /// Creates a new empty `Transporte`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the vehicle license plate.
    #[must_use]
    pub fn patente(mut self, patente: impl Into<String>) -> Self {
        self.patente = Some(patente.into());
        self
    }

    /// Sets the carrier tax ID (RUT).
    #[must_use]
    pub fn rut_trans(mut self, rut: Rut) -> Self {
        self.rut_trans = Some(rut);
        self
    }

    /// Sets the driver information.
    #[must_use]
    pub fn chofer(mut self, chofer: Chofer) -> Self {
        self.chofer = Some(chofer);
        self
    }

    /// Sets the destination address.
    #[must_use]
    pub fn dir_dest(mut self, dir: impl Into<String>) -> Self {
        self.dir_dest = Some(dir.into());
        self
    }

    /// Sets the destination commune.
    #[must_use]
    pub fn cmna_dest(mut self, cmna: impl Into<String>) -> Self {
        self.cmna_dest = Some(cmna.into());
        self
    }

    /// Sets the destination city.
    #[must_use]
    pub fn ciudad_dest(mut self, ciudad: impl Into<String>) -> Self {
        self.ciudad_dest = Some(ciudad.into());
        self
    }
}

/// Driver identification (`Chofer` in SII schema).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Chofer {
    /// Driver tax ID (`RUTChofer`).
    pub rut_chofer: Option<Rut>,
    /// Driver full name (`NombreChofer`).
    pub nombre_chofer: Option<String>,
}

impl Chofer {
    /// Creates a new `Chofer`.
    #[must_use]
    pub fn new(rut_chofer: Option<Rut>, nombre_chofer: Option<String>) -> Self {
        Self {
            rut_chofer,
            nombre_chofer,
        }
    }
}

/// Product or item code (`CdgItem` in SII schema).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CdgItem {
    /// Code type identifier (e.g. `"INT1"`, `"EAN"`).
    pub tpo_codigo: String,
    /// Code value.
    pub vlr_codigo: String,
}

impl CdgItem {
    /// Creates a new `CdgItem`.
    #[must_use]
    pub fn new(tpo_codigo: impl Into<String>, vlr_codigo: impl Into<String>) -> Self {
        Self {
            tpo_codigo: tpo_codigo.into(),
            vlr_codigo: vlr_codigo.into(),
        }
    }
}

/// Line item detail (`Detalle` in SII schema).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Detalle {
    /// Sequential line number (`NroLinDet` in schema).
    pub nro_lin_det: u32,
    /// Item codes (`CdgItem`).
    pub cdg_item: Vec<CdgItem>,
    /// Exemption indicator (`IndExe`: 1 = exempt or non-taxable item).
    pub ind_exe: Option<u8>,
    /// Item name or short description (`NmbItem`).
    pub nmb_item: String,
    /// Additional item description (`DscItem`).
    pub dsc_item: Option<String>,
    /// Quantity (`QtyItem`).
    pub qty_item: Option<u32>,
    /// Unit of measurement (`UnmdItem`).
    pub unmd_item: Option<String>,
    /// Unit price (`PrcItem`).
    pub prc_item: Option<Monto>,
    /// Total line amount (`MontoItem`).
    pub monto_item: Monto,
}

impl Detalle {
    /// Creates a new `Detalle` with mandatory fields.
    #[must_use]
    pub fn new(nro_lin_det: u32, nmb_item: impl Into<String>, monto_item: Monto) -> Self {
        Self {
            nro_lin_det,
            cdg_item: Vec::new(),
            ind_exe: None,
            nmb_item: nmb_item.into(),
            dsc_item: None,
            qty_item: None,
            unmd_item: None,
            prc_item: None,
            monto_item,
        }
    }

    /// Sets the quantity.
    #[must_use]
    pub const fn qty(mut self, qty: u32) -> Self {
        self.qty_item = Some(qty);
        self
    }

    /// Sets the unit price.
    #[must_use]
    pub const fn prc(mut self, prc: Monto) -> Self {
        self.prc_item = Some(prc);
        self
    }

    /// Adds an item code.
    #[must_use]
    pub fn codigo(mut self, tpo: impl Into<String>, vlr: impl Into<String>) -> Self {
        self.cdg_item.push(CdgItem::new(tpo, vlr));
        self
    }

    /// Sets the additional item description.
    #[must_use]
    pub fn dsc(mut self, dsc: impl Into<String>) -> Self {
        self.dsc_item = Some(dsc.into());
        self
    }

    /// Sets the unit of measurement.
    #[must_use]
    pub fn unmd(mut self, unmd: impl Into<String>) -> Self {
        self.unmd_item = Some(unmd.into());
        self
    }

    /// Sets the exemption indicator (e.g. 1 for exempt).
    #[must_use]
    pub const fn ind_exe(mut self, ind: u8) -> Self {
        self.ind_exe = Some(ind);
        self
    }
}

/// Document identification (`IdDoc` in SII schema).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdDoc {
    /// DTE type code (`TipoDTE`).
    pub tipo_dte: TipoDte,
    /// Sequential authorized document number (`Folio`).
    pub folio: u64,
    /// Document issue date (`FchEmis`).
    pub fch_emis: time::Date,
    /// Document expiration / due date (`FchVenc`).
    pub fch_venc: Option<time::Date>,
    /// Service indicator (`IndServicio`).
    pub ind_servicio: Option<u8>,
    /// Payment terms code (`TermPagoCdg`).
    pub term_pago_cdg: Option<u8>,
    /// Payment method (`FmaPago`).
    pub fma_pago: Option<u8>,
    /// Dispatch type code (`TipoDespacho`).
    pub tipo_despacho: Option<u8>,
    /// Transfer indicator code (`IndTraslado`).
    pub ind_traslado: Option<u8>,
}

#[cfg(feature = "serde")]
impl serde::Serialize for IdDoc {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let helper = IdDocHelper {
            tipo_dte: self.tipo_dte,
            folio: self.folio,
            fch_emis: format_date(&self.fch_emis),
            fch_venc: self.fch_venc.as_ref().map(format_date),
            ind_servicio: self.ind_servicio,
            term_pago_cdg: self.term_pago_cdg,
            fma_pago: self.fma_pago,
            tipo_despacho: self.tipo_despacho,
            ind_traslado: self.ind_traslado,
        };
        helper.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for IdDoc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = IdDocHelper::deserialize(deserializer)?;
        let fch_emis = parse_date(&helper.fch_emis).map_err(serde::de::Error::custom)?;
        let fch_venc = match helper.fch_venc {
            Some(s) => Some(parse_date(&s).map_err(serde::de::Error::custom)?),
            None => None,
        };
        Ok(Self {
            tipo_dte: helper.tipo_dte,
            folio: helper.folio,
            fch_emis,
            fch_venc,
            ind_servicio: helper.ind_servicio,
            term_pago_cdg: helper.term_pago_cdg,
            fma_pago: helper.fma_pago,
            tipo_despacho: helper.tipo_despacho,
            ind_traslado: helper.ind_traslado,
        })
    }
}

impl IdDoc {
    /// Creates a new `IdDoc` with mandatory fields.
    #[must_use]
    pub const fn new(tipo_dte: TipoDte, folio: u64, fch_emis: time::Date) -> Self {
        Self {
            tipo_dte,
            folio,
            fch_emis,
            fch_venc: None,
            ind_servicio: None,
            term_pago_cdg: None,
            fma_pago: None,
            tipo_despacho: None,
            ind_traslado: None,
        }
    }

    /// Sets the expiration / due date.
    #[must_use]
    pub const fn fch_venc(mut self, date: time::Date) -> Self {
        self.fch_venc = Some(date);
        self
    }

    /// Sets the service indicator.
    #[must_use]
    pub const fn ind_servicio(mut self, ind: u8) -> Self {
        self.ind_servicio = Some(ind);
        self
    }

    /// Sets the payment terms code.
    #[must_use]
    pub const fn term_pago_cdg(mut self, cdg: u8) -> Self {
        self.term_pago_cdg = Some(cdg);
        self
    }

    /// Sets the payment method.
    #[must_use]
    pub const fn fma_pago(mut self, fma: u8) -> Self {
        self.fma_pago = Some(fma);
        self
    }

    /// Sets the dispatch type code (`TipoDespacho`).
    #[must_use]
    pub const fn tipo_despacho(mut self, tipo: u8) -> Self {
        self.tipo_despacho = Some(tipo);
        self
    }

    /// Sets the transfer indicator code (`IndTraslado`).
    #[must_use]
    pub const fn ind_traslado(mut self, ind: u8) -> Self {
        self.ind_traslado = Some(ind);
        self
    }
}

/// Document issuer (`Emisor` in SII schema).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Emisor {
    /// Issuer tax ID (`RUTEmisor`).
    pub rut_emisor: Rut,
    /// Legal business name (`RznSoc`).
    pub rzn_soc: String,
    /// Business activity line / turn (`GiroEmis`).
    pub giro_emis: String,
    /// Economic activity codes registered with SII (`Acteco`).
    pub acteco: Vec<u32>,
    /// SII branch code (`CdgSIISucur`).
    pub cdg_sii_sucur: Option<String>,
    /// Origin physical address (`DirOrigen`).
    pub dir_origen: String,
    /// Origin municipality / commune (`CmnaOrigen`).
    pub cmna_origen: String,
    /// Origin city (`CiudadOrigen`).
    pub ciudad_origen: Option<String>,
}

impl Emisor {
    /// Creates a new `Emisor` with mandatory fields.
    #[must_use]
    pub fn new(
        rut_emisor: Rut,
        rzn_soc: impl Into<String>,
        giro_emis: impl Into<String>,
        acteco: u32,
        dir_origen: impl Into<String>,
        cmna_origen: impl Into<String>,
    ) -> Self {
        Self {
            rut_emisor,
            rzn_soc: rzn_soc.into(),
            giro_emis: giro_emis.into(),
            acteco: vec![acteco],
            cdg_sii_sucur: None,
            dir_origen: dir_origen.into(),
            cmna_origen: cmna_origen.into(),
            ciudad_origen: None,
        }
    }

    /// Adds an additional economic activity code.
    #[must_use]
    pub fn add_acteco(mut self, acteco: u32) -> Self {
        self.acteco.push(acteco);
        self
    }

    /// Sets the SII branch code.
    #[must_use]
    pub fn cdg_sii_sucur(mut self, cdg: impl Into<String>) -> Self {
        self.cdg_sii_sucur = Some(cdg.into());
        self
    }

    /// Sets the origin city.
    #[must_use]
    pub fn ciudad_origen(mut self, ciudad: impl Into<String>) -> Self {
        self.ciudad_origen = Some(ciudad.into());
        self
    }
}

/// Document receiver (`Receptor` in SII schema).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Receptor {
    /// Receiver tax ID (`RUTRecep`).
    pub rut_recep: Rut,
    /// Receiver legal business or person name (`RznSocRecep`).
    pub rzn_soc_recep: String,
    /// Receiver business activity (`GiroRecep`).
    pub giro_recep: Option<String>,
    /// Receiver address (`DirRecep`).
    pub dir_recep: Option<String>,
    /// Receiver commune (`CmnaRecep`).
    pub cmna_recep: Option<String>,
    /// Receiver city (`CiudadRecep`).
    pub ciudad_recep: Option<String>,
}

impl Receptor {
    /// Creates a new `Receptor` with mandatory invoice fields.
    #[must_use]
    pub fn new(
        rut_recep: Rut,
        rzn_soc_recep: impl Into<String>,
        giro_recep: impl Into<String>,
        dir_recep: impl Into<String>,
        cmna_recep: impl Into<String>,
    ) -> Self {
        Self {
            rut_recep,
            rzn_soc_recep: rzn_soc_recep.into(),
            giro_recep: Some(giro_recep.into()),
            dir_recep: Some(dir_recep.into()),
            cmna_recep: Some(cmna_recep.into()),
            ciudad_recep: None,
        }
    }

    /// Creates a minimal receiver for retail final consumer boletas (`RUT 66.666.666-6`, "Consumidor Final").
    #[must_use]
    pub fn consumidor_final() -> Self {
        // 66.666.666-6 is the standard Chilean generic consumer RUT
        let rut = Rut::new(66_666_666, '6')
            .unwrap_or_else(|_| Rut::parse("66.666.666-6").unwrap_or_else(|_| unreachable!()));
        Self {
            rut_recep: rut,
            rzn_soc_recep: "Consumidor Final".to_string(),
            giro_recep: None,
            dir_recep: None,
            cmna_recep: None,
            ciudad_recep: None,
        }
    }

    /// Creates a minimal receiver with only RUT and name (RF-01: receptor puede ir mínimo en boleta).
    #[must_use]
    pub fn minimo(rut_recep: Rut, rzn_soc_recep: impl Into<String>) -> Self {
        Self {
            rut_recep,
            rzn_soc_recep: rzn_soc_recep.into(),
            giro_recep: None,
            dir_recep: None,
            cmna_recep: None,
            ciudad_recep: None,
        }
    }

    /// Sets the receiver city.
    #[must_use]
    pub fn ciudad_recep(mut self, ciudad: impl Into<String>) -> Self {
        self.ciudad_recep = Some(ciudad.into());
        self
    }
}

/// Document monetary totals (`Totales` in SII schema).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Totales {
    /// Net taxable amount (`MntNeto`). Omitted in exempt documents.
    pub mnt_neto: Option<Monto>,
    /// Exempt amount (`MntExe`).
    pub mnt_exe: Option<Monto>,
    /// VAT rate percentage (`TasaIVA`, typically 19 for 19 %). Omitted in exempt documents.
    pub tasa_iva: Option<u32>,
    /// VAT amount (`IVA`). Omitted in exempt documents.
    pub iva: Option<Monto>,
    /// Grand total amount (`MntTotal`).
    pub mnt_total: Monto,
}

impl Totales {
    /// Constructs totals for a taxable document (Tipo 33), computing 19 % IVA.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidMonto`] on arithmetic overflow.
    pub fn afecto(neto: Monto, exe: Option<Monto>) -> Result<Self, Error> {
        let iva = neto.iva_19()?;
        let total = match exe {
            Some(e) => neto.add_monto(e)?.add_monto(iva)?,
            None => neto.add_monto(iva)?,
        };
        Ok(Self {
            mnt_neto: Some(neto),
            mnt_exe: exe,
            tasa_iva: Some(19),
            iva: Some(iva),
            mnt_total: total,
        })
    }

    /// Constructs totals for an exempt document (Tipo 34) without VAT.
    #[must_use]
    pub const fn exento(total_exento: Monto) -> Self {
        Self {
            mnt_neto: None,
            mnt_exe: Some(total_exento),
            tasa_iva: None,
            iva: None,
            mnt_total: total_exento,
        }
    }
}

/// Document header (`Encabezado` in SII schema).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Encabezado {
    /// Document identification (`IdDoc`).
    pub id_doc: IdDoc,
    /// Document issuer (`Emisor`).
    pub emisor: Emisor,
    /// Document receiver (`Receptor`).
    pub receptor: Receptor,
    /// Transport / dispatch carrier information (`Transporte`).
    pub transporte: Option<Transporte>,
    /// Document monetary totals (`Totales`).
    pub totales: Totales,
}

impl Encabezado {
    /// Creates a new `Encabezado`.
    #[must_use]
    pub const fn new(id_doc: IdDoc, emisor: Emisor, receptor: Receptor, totales: Totales) -> Self {
        Self {
            id_doc,
            emisor,
            receptor,
            transporte: None,
            totales,
        }
    }

    /// Sets the transport / dispatch details.
    #[must_use]
    pub fn with_transporte(mut self, transporte: Transporte) -> Self {
        self.transporte = Some(transporte);
        self
    }
}

/// Electronic Tax Document (DTE).
///
/// Represents an unsigned DTE containing a `<Documento ID="...">` tag with
/// its `Encabezado`, `Detalle`, and optional `Referencia` items.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Dte {
    /// Internal document identifier (used in `Documento ID="..."`, e.g. `"F60T33"`).
    pub id: String,
    /// Document header.
    pub encabezado: Encabezado,
    /// Document line items.
    pub detalles: Vec<Detalle>,
    /// Document references (`Referencia`).
    pub referencias: Vec<Referencia>,
}

impl Dte {
    /// Creates a new `Dte` without references after validating internal consistency.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DteValidation`] if validation fails.
    pub fn new(encabezado: Encabezado, detalles: Vec<Detalle>) -> Result<Self, Error> {
        Self::with_referencias(encabezado, detalles, Vec::new())
    }

    /// Creates a new `Dte` with references after validating internal consistency.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DteValidation`] if validation fails or mandatory rules (DTE-02) are violated.
    pub fn with_referencias(
        encabezado: Encabezado,
        detalles: Vec<Detalle>,
        referencias: Vec<Referencia>,
    ) -> Result<Self, Error> {
        let tipo = encabezado.id_doc.tipo_dte;
        let folio = encabezado.id_doc.folio;
        if folio == 0 {
            return Err(Error::DteValidation(
                "folio must be greater than zero".into(),
            ));
        }
        if detalles.is_empty() {
            return Err(Error::DteValidation(
                "at least one detail line is required".into(),
            ));
        }

        validate_detalles(&detalles)?;
        validate_referencias(tipo, &referencias)?;

        if tipo == TipoDte::GuiaDespacho {
            match encabezado.id_doc.ind_traslado {
                Some(1..=9) => {}
                Some(invalid) => {
                    return Err(Error::DteValidation(format!(
                        "invalid ind_traslado {invalid} in guía de despacho (52): must be between 1 and 9"
                    )));
                }
                None => {
                    return Err(Error::DteValidation(
                        "guía de despacho (52) requires ind_traslado (RF-02)".into(),
                    ));
                }
            }
            if let Some(despacho) = encabezado.id_doc.tipo_despacho {
                if !(1..=3).contains(&despacho) {
                    return Err(Error::DteValidation(format!(
                        "invalid tipo_despacho {despacho}: must be between 1 and 3"
                    )));
                }
            }
        }

        let id = format!("F{}T{}", folio, tipo.code());
        Ok(Self {
            id,
            encabezado,
            detalles,
            referencias,
        })
    }

    /// Creates a builder for Factura Electrónica (afecta a IVA 19 %) - Tipo 33.
    #[must_use]
    pub fn factura_afecta() -> FacturaAfectaBuilder {
        FacturaAfectaBuilder::new()
    }

    /// Creates a builder for Factura No Afecta o Exenta Electrónica - Tipo 34.
    #[must_use]
    pub fn factura_exenta() -> FacturaExentaBuilder {
        FacturaExentaBuilder::new()
    }

    /// Creates a builder for Boleta Electrónica (afecta a IVA 19 %) - Tipo 39.
    #[must_use]
    pub fn boleta() -> BoletaBuilder {
        BoletaBuilder::new()
    }

    /// Creates a builder for Boleta Exenta Electrónica - Tipo 41.
    #[must_use]
    pub fn boleta_exenta() -> BoletaExentaBuilder {
        BoletaExentaBuilder::new()
    }

    /// Creates a builder for Guía de Despacho Electrónica - Tipo 52.
    #[must_use]
    pub fn guia_despacho() -> GuiaDespachoBuilder {
        GuiaDespachoBuilder::new()
    }

    /// Creates a builder for Nota de Débito Electrónica - Tipo 56.
    #[must_use]
    pub fn nota_debito() -> NotaDebitoBuilder {
        NotaDebitoBuilder::new()
    }

    /// Creates a builder for Nota de Crédito Electrónica - Tipo 61.
    #[must_use]
    pub fn nota_credito() -> NotaCreditoBuilder {
        NotaCreditoBuilder::new()
    }

    /// Serializes the document to canonical SII XML bytes (`<DTE version="1.0">...`).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Xml`] if serialization fails.
    pub fn to_xml(&self) -> Result<Vec<u8>, Error> {
        let mut writer = Writer::new(Cursor::new(Vec::new()));

        let mut dte_start = BytesStart::new("DTE");
        dte_start.push_attribute(("version", "1.0"));
        writer
            .write_event(Event::Start(dte_start))
            .map_err(|e| Error::Xml(e.to_string()))?;

        let mut doc_start = BytesStart::new("Documento");
        doc_start.push_attribute(("ID", self.id.as_str()));
        writer
            .write_event(Event::Start(doc_start))
            .map_err(|e| Error::Xml(e.to_string()))?;

        write_encabezado(&mut writer, &self.encabezado)?;

        for detalle in &self.detalles {
            write_detalle(&mut writer, detalle)?;
        }

        for referencia in &self.referencias {
            write_referencia(&mut writer, referencia)?;
        }

        writer
            .write_event(Event::End(BytesEnd::new("Documento")))
            .map_err(|e| Error::Xml(e.to_string()))?;

        writer
            .write_event(Event::End(BytesEnd::new("DTE")))
            .map_err(|e| Error::Xml(e.to_string()))?;

        Ok(writer.into_inner().into_inner())
    }

    /// Returns a slice of document references.
    #[inline]
    #[must_use]
    pub fn referencias(&self) -> &[Referencia] {
        &self.referencias
    }

    /// Returns `true` if this document is a boleta (types 39 or 41).
    #[inline]
    #[must_use]
    pub const fn is_boleta(&self) -> bool {
        self.tipo().is_boleta()
    }

    /// Returns `true` if this document must be sent via `EnvioBOLETA`.
    #[inline]
    #[must_use]
    pub const fn is_envio_boleta(&self) -> bool {
        self.tipo().is_envio_boleta()
    }

    /// Returns `true` if this document must be sent via `EnvioDTE`.
    #[inline]
    #[must_use]
    pub const fn is_envio_dte(&self) -> bool {
        self.tipo().is_envio_dte()
    }

    /// Returns the DTE type code.
    #[must_use]
    pub const fn tipo(&self) -> TipoDte {
        self.encabezado.id_doc.tipo_dte
    }

    /// Returns the document folio.
    #[must_use]
    pub const fn folio(&self) -> u64 {
        self.encabezado.id_doc.folio
    }

    /// Returns the internal document ID attribute string.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns a reference to the document header.
    #[must_use]
    pub const fn encabezado(&self) -> &Encabezado {
        &self.encabezado
    }

    /// Returns a slice of document line items.
    #[must_use]
    pub fn detalles(&self) -> &[Detalle] {
        &self.detalles
    }

    /// Returns the grand total amount.
    #[must_use]
    pub const fn total(&self) -> Monto {
        self.encabezado.totales.mnt_total
    }

    /// Returns the net amount if taxable.
    #[must_use]
    pub const fn neto(&self) -> Option<Monto> {
        self.encabezado.totales.mnt_neto
    }

    /// Returns the 19 % IVA amount if taxable.
    #[must_use]
    pub const fn iva(&self) -> Option<Monto> {
        self.encabezado.totales.iva
    }

    /// Returns the exempt amount if present.
    #[must_use]
    pub const fn exento(&self) -> Option<Monto> {
        self.encabezado.totales.mnt_exe
    }

    /// Applies an electronic tax stamp (TED) using the provided CAF, folio, and clock (ADR-01, TED-01).
    ///
    /// Validates that:
    /// - The folio is within the CAF's authorized range `[min, max]` (CAF-01).
    /// - The CAF document type matches this DTE's document type.
    /// - The CAF issuer RUT matches this DTE's issuer RUT.
    /// - The DTE has at least one detail line item.
    ///
    /// # Errors
    ///
    /// Returns [`Error::FolioOutOfRange`] if `folio` is outside the CAF authorized range.
    /// Returns [`Error::DteValidation`] if CAF document type or issuer RUT does not match,
    /// or if the document has no details.
    /// Returns [`Error::Crypto`] if digital signing fails.
    pub fn timbrar(&self, caf: &Caf, folio: u64, clock: &impl Clock) -> Result<DteTimbrado, Error> {
        DteTimbrado::new(self, caf, folio, clock)
    }
}

/// Electronic Tax Document stamped with an electronic tax stamp (`TED`).
///
/// Encapsulates a validated [`Dte`] and its [`Ted`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DteTimbrado {
    dte: Dte,
    ted: Ted,
}

impl DteTimbrado {
    /// Creates a new `DteTimbrado` by applying an electronic tax stamp (`TED`) to a DTE.
    ///
    /// Validates that:
    /// - The folio is within the CAF's authorized range `[min, max]` (CAF-01).
    /// - The CAF document type matches this DTE's document type.
    /// - The CAF issuer RUT matches this DTE's issuer RUT.
    /// - The DTE has at least one detail line item.
    ///
    /// # Errors
    ///
    /// Returns [`Error::FolioOutOfRange`] if `folio` is outside the CAF authorized range.
    /// Returns [`Error::DteValidation`] if CAF document type or issuer RUT does not match,
    /// or if the document has no details.
    /// Returns [`Error::Crypto`] if digital signing fails.
    pub fn new(dte: &Dte, caf: &Caf, folio: u64, clock: &impl Clock) -> Result<Self, Error> {
        caf.verificar_folio(folio)?;

        if caf.tipo() != dte.tipo() {
            return Err(Error::DteValidation(format!(
                "CAF document type ({}) does not match DTE type ({})",
                caf.tipo(),
                dte.tipo()
            )));
        }

        if caf.emisor_rut() != &dte.encabezado.emisor.rut_emisor {
            return Err(Error::DteValidation(format!(
                "CAF issuer RUT ({}) does not match DTE issuer RUT ({})",
                caf.emisor_rut(),
                dte.encabezado.emisor.rut_emisor
            )));
        }

        if dte.detalles.is_empty() {
            return Err(Error::DteValidation(
                "at least one detail line is required to stamp DTE".into(),
            ));
        }

        let mut stamped_dte = dte.clone();
        if stamped_dte.encabezado.id_doc.folio != folio {
            stamped_dte.encabezado.id_doc.folio = folio;
            stamped_dte.id = format!("F{}T{}", folio, dte.tipo().code());
        }

        let dd = Dd::from_dte(&stamped_dte, caf, folio, &clock.now())?;
        let ted = Ted::new(dd, caf)?;

        Ok(Self {
            dte: stamped_dte,
            ted,
        })
    }

    /// Returns a reference to the underlying DTE.
    #[inline]
    #[must_use]
    pub const fn dte(&self) -> &Dte {
        &self.dte
    }

    /// Returns a reference to the electronic tax stamp (TED).
    #[inline]
    #[must_use]
    pub const fn ted(&self) -> &Ted {
        &self.ted
    }

    /// Returns the exact serialized `<TED>` bytes required for generating the PDF417 2D barcode (ADR-06, TED-02).
    #[inline]
    #[must_use]
    pub fn ted_pdf417_data(&self) -> &[u8] {
        self.ted.pdf417_data()
    }

    /// Returns the document folio number.
    #[inline]
    #[must_use]
    pub const fn folio(&self) -> u64 {
        self.dte.folio()
    }

    /// Returns the document type.
    #[inline]
    #[must_use]
    pub const fn tipo(&self) -> TipoDte {
        self.dte.tipo()
    }

    /// Returns the internal document ID attribute string.
    #[inline]
    #[must_use]
    pub fn id(&self) -> &str {
        self.dte.id()
    }

    /// Returns a slice of document references.
    #[inline]
    #[must_use]
    pub fn referencias(&self) -> &[Referencia] {
        self.dte.referencias()
    }

    /// Returns `true` if this document is a boleta (types 39 or 41).
    #[inline]
    #[must_use]
    pub const fn is_boleta(&self) -> bool {
        self.dte.is_boleta()
    }

    /// Returns `true` if this document must be sent via `EnvioBOLETA`.
    #[inline]
    #[must_use]
    pub const fn is_envio_boleta(&self) -> bool {
        self.dte.is_envio_boleta()
    }

    /// Returns `true` if this document must be sent via `EnvioDTE`.
    #[inline]
    #[must_use]
    pub const fn is_envio_dte(&self) -> bool {
        self.dte.is_envio_dte()
    }

    /// Serializes the stamped DTE to XML (`<DTE version="1.0">...<Documento ID="...">...<TED>...</TED></Documento></DTE>`).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Xml`] if serialization fails.
    pub fn to_xml(&self) -> Result<Vec<u8>, Error> {
        let mut writer = Writer::new(Cursor::new(Vec::new()));

        let mut dte_start = BytesStart::new("DTE");
        dte_start.push_attribute(("version", "1.0"));
        writer
            .write_event(Event::Start(dte_start))
            .map_err(|e| Error::Xml(e.to_string()))?;

        let mut doc_start = BytesStart::new("Documento");
        doc_start.push_attribute(("ID", self.dte.id.as_str()));
        writer
            .write_event(Event::Start(doc_start))
            .map_err(|e| Error::Xml(e.to_string()))?;

        write_encabezado(&mut writer, &self.dte.encabezado)?;

        for detalle in &self.dte.detalles {
            write_detalle(&mut writer, detalle)?;
        }

        for referencia in &self.dte.referencias {
            write_referencia(&mut writer, referencia)?;
        }

        // Insert the raw TED XML into writer cursor
        std::io::Write::write_all(writer.get_mut(), self.ted.raw_ted())
            .map_err(|e| Error::Xml(e.to_string()))?;

        writer
            .write_event(Event::End(BytesEnd::new("Documento")))
            .map_err(|e| Error::Xml(e.to_string()))?;

        writer
            .write_event(Event::End(BytesEnd::new("DTE")))
            .map_err(|e| Error::Xml(e.to_string()))?;

        Ok(writer.into_inner().into_inner())
    }
}

/// Builder for Factura Electrónica (afecta a IVA 19 %) - Tipo 33.
#[derive(Debug, Clone, Default)]
pub struct FacturaAfectaBuilder {
    folio: Option<u64>,
    fch_emis: Option<time::Date>,
    fch_venc: Option<time::Date>,
    ind_servicio: Option<u8>,
    documento_id: Option<String>,
    emisor: Option<Emisor>,
    receptor: Option<Receptor>,
    detalles: Vec<Detalle>,
    referencias: Vec<Referencia>,
}

/// Alias for [`FacturaAfectaBuilder`].
pub type FacturaBuilder = FacturaAfectaBuilder;

impl FacturaAfectaBuilder {
    /// Creates a new `FacturaAfectaBuilder`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the authorized document folio number.
    #[must_use]
    pub const fn folio(mut self, folio: u64) -> Self {
        self.folio = Some(folio);
        self
    }

    /// Sets the emission date.
    #[must_use]
    pub const fn fch_emis(mut self, date: time::Date) -> Self {
        self.fch_emis = Some(date);
        self
    }

    /// Parses and sets the emission date from a `YYYY-MM-DD` string.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidDate`] if the date format is invalid.
    pub fn fch_emis_str(mut self, s: &str) -> Result<Self, Error> {
        self.fch_emis = Some(parse_date(s)?);
        Ok(self)
    }

    /// Sets the due date.
    #[must_use]
    pub const fn fch_venc(mut self, date: time::Date) -> Self {
        self.fch_venc = Some(date);
        self
    }

    /// Sets the service indicator.
    #[must_use]
    pub const fn ind_servicio(mut self, ind: u8) -> Self {
        self.ind_servicio = Some(ind);
        self
    }

    /// Overrides the internal document ID attribute (default is `F{folio}T33`).
    #[must_use]
    pub fn documento_id(mut self, id: impl Into<String>) -> Self {
        self.documento_id = Some(id.into());
        self
    }

    /// Sets the document issuer.
    #[must_use]
    pub fn emisor(mut self, emisor: Emisor) -> Self {
        self.emisor = Some(emisor);
        self
    }

    /// Sets the document receiver.
    #[must_use]
    pub fn receptor(mut self, receptor: Receptor) -> Self {
        self.receptor = Some(receptor);
        self
    }

    /// Adds a line item detail.
    #[must_use]
    pub fn item(mut self, item: Detalle) -> Self {
        self.detalles.push(item);
        self
    }

    /// Adds multiple line item details.
    #[must_use]
    pub fn items(mut self, items: impl IntoIterator<Item = Detalle>) -> Self {
        self.detalles.extend(items);
        self
    }

    /// Adds a document reference.
    #[must_use]
    pub fn referencia(mut self, ref_item: Referencia) -> Self {
        self.referencias.push(ref_item);
        self
    }

    /// Adds multiple document references.
    #[must_use]
    pub fn referencias(mut self, refs: impl IntoIterator<Item = Referencia>) -> Self {
        self.referencias.extend(refs);
        self
    }

    /// Builds and validates the `Dte` for Factura Afecta (Tipo 33).
    ///
    /// Automatically calculates 19 % IVA (`round(MntNeto * 0.19)`) and `MntTotal`
    /// conforming to rules DTE-02, DTE-03, and DTE-04.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DteValidation`] if required fields are missing or validation rules fail.
    pub fn build(self) -> Result<Dte, Error> {
        let folio = self
            .folio
            .ok_or_else(|| Error::DteValidation("missing folio".into()))?;
        if folio == 0 {
            return Err(Error::DteValidation(
                "folio must be greater than zero".into(),
            ));
        }
        let fch_emis = self
            .fch_emis
            .ok_or_else(|| Error::DteValidation("missing fch_emis (issue date)".into()))?;
        let emisor = self
            .emisor
            .ok_or_else(|| Error::DteValidation("missing emisor".into()))?;
        let receptor = self
            .receptor
            .ok_or_else(|| Error::DteValidation("missing receptor".into()))?;

        if self.detalles.is_empty() {
            return Err(Error::DteValidation(
                "at least one detail line is required".into(),
            ));
        }

        validate_detalles(&self.detalles)?;

        let mut neto = Monto::ZERO;
        let mut exento = Monto::ZERO;
        let mut has_exento = false;

        for item in &self.detalles {
            if item.ind_exe == Some(1) {
                exento = exento.add_monto(item.monto_item)?;
                has_exento = true;
            } else {
                neto = neto.add_monto(item.monto_item)?;
            }
        }

        if neto <= Monto::ZERO {
            return Err(Error::DteValidation(
                "factura afecta (33) must contain at least one taxable item (neto > 0)".into(),
            ));
        }

        let mnt_exe_opt = if has_exento && exento > Monto::ZERO {
            Some(exento)
        } else {
            None
        };

        let totales = Totales::afecto(neto, mnt_exe_opt)?;

        let mut id_doc = IdDoc::new(TipoDte::FacturaAfecta, folio, fch_emis);
        if let Some(venc) = self.fch_venc {
            id_doc = id_doc.fch_venc(venc);
        }
        if let Some(ind) = self.ind_servicio {
            id_doc = id_doc.ind_servicio(ind);
        }

        let encabezado = Encabezado::new(id_doc, emisor, receptor, totales);
        let mut dte = Dte::with_referencias(encabezado, self.detalles, self.referencias)?;
        if let Some(id) = self.documento_id {
            dte.id = id;
        }
        Ok(dte)
    }
}

/// Builder for Factura No Afecta o Exenta Electrónica - Tipo 34.
#[derive(Debug, Clone, Default)]
pub struct FacturaExentaBuilder {
    folio: Option<u64>,
    fch_emis: Option<time::Date>,
    fch_venc: Option<time::Date>,
    ind_servicio: Option<u8>,
    documento_id: Option<String>,
    emisor: Option<Emisor>,
    receptor: Option<Receptor>,
    detalles: Vec<Detalle>,
    referencias: Vec<Referencia>,
}

impl FacturaExentaBuilder {
    /// Creates a new `FacturaExentaBuilder`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the authorized document folio number.
    #[must_use]
    pub const fn folio(mut self, folio: u64) -> Self {
        self.folio = Some(folio);
        self
    }

    /// Sets the emission date.
    #[must_use]
    pub const fn fch_emis(mut self, date: time::Date) -> Self {
        self.fch_emis = Some(date);
        self
    }

    /// Parses and sets the emission date from a `YYYY-MM-DD` string.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidDate`] if the date format is invalid.
    pub fn fch_emis_str(mut self, s: &str) -> Result<Self, Error> {
        self.fch_emis = Some(parse_date(s)?);
        Ok(self)
    }

    /// Sets the due date.
    #[must_use]
    pub const fn fch_venc(mut self, date: time::Date) -> Self {
        self.fch_venc = Some(date);
        self
    }

    /// Sets the service indicator.
    #[must_use]
    pub const fn ind_servicio(mut self, ind: u8) -> Self {
        self.ind_servicio = Some(ind);
        self
    }

    /// Overrides the internal document ID attribute (default is `F{folio}T34`).
    #[must_use]
    pub fn documento_id(mut self, id: impl Into<String>) -> Self {
        self.documento_id = Some(id.into());
        self
    }

    /// Sets the document issuer.
    #[must_use]
    pub fn emisor(mut self, emisor: Emisor) -> Self {
        self.emisor = Some(emisor);
        self
    }

    /// Sets the document receiver.
    #[must_use]
    pub fn receptor(mut self, receptor: Receptor) -> Self {
        self.receptor = Some(receptor);
        self
    }

    /// Adds a line item detail.
    #[must_use]
    pub fn item(mut self, item: Detalle) -> Self {
        self.detalles.push(item);
        self
    }

    /// Adds multiple line item details.
    #[must_use]
    pub fn items(mut self, items: impl IntoIterator<Item = Detalle>) -> Self {
        self.detalles.extend(items);
        self
    }

    /// Adds a document reference.
    #[must_use]
    pub fn referencia(mut self, ref_item: Referencia) -> Self {
        self.referencias.push(ref_item);
        self
    }

    /// Adds multiple document references.
    #[must_use]
    pub fn referencias(mut self, refs: impl IntoIterator<Item = Referencia>) -> Self {
        self.referencias.extend(refs);
        self
    }

    /// Builds and validates the `Dte` for Factura Exenta (Tipo 34).
    ///
    /// Guarantees that all line items have `IndExe = 1` and that no IVA or net amount
    /// tags are present (DTE-02).
    ///
    /// # Errors
    ///
    /// Returns [`Error::DteValidation`] if required fields are missing or validation rules fail.
    pub fn build(mut self) -> Result<Dte, Error> {
        let folio = self
            .folio
            .ok_or_else(|| Error::DteValidation("missing folio".into()))?;
        if folio == 0 {
            return Err(Error::DteValidation(
                "folio must be greater than zero".into(),
            ));
        }
        let fch_emis = self
            .fch_emis
            .ok_or_else(|| Error::DteValidation("missing fch_emis (issue date)".into()))?;
        let emisor = self
            .emisor
            .ok_or_else(|| Error::DteValidation("missing emisor".into()))?;
        let receptor = self
            .receptor
            .ok_or_else(|| Error::DteValidation("missing receptor".into()))?;

        if self.detalles.is_empty() {
            return Err(Error::DteValidation(
                "at least one detail line is required".into(),
            ));
        }

        // For Factura Exenta (34), every item is exempt: ensure IndExe is 1.
        for item in &mut self.detalles {
            item.ind_exe = Some(1);
        }

        validate_detalles(&self.detalles)?;

        let mut exento = Monto::ZERO;
        for item in &self.detalles {
            exento = exento.add_monto(item.monto_item)?;
        }

        if exento <= Monto::ZERO {
            return Err(Error::DteValidation(
                "factura exenta (34) must have positive exempt total (MntExe > 0)".into(),
            ));
        }

        let totales = Totales::exento(exento);

        let mut id_doc = IdDoc::new(TipoDte::FacturaExenta, folio, fch_emis);
        if let Some(venc) = self.fch_venc {
            id_doc = id_doc.fch_venc(venc);
        }
        if let Some(ind) = self.ind_servicio {
            id_doc = id_doc.ind_servicio(ind);
        }

        let encabezado = Encabezado::new(id_doc, emisor, receptor, totales);
        let mut dte = Dte::with_referencias(encabezado, self.detalles, self.referencias)?;
        if let Some(id) = self.documento_id {
            dte.id = id;
        }
        Ok(dte)
    }
}

/// Builder for Boleta Electrónica (afecta a IVA 19 %) - Tipo 39.
#[derive(Debug, Clone, Default)]
pub struct BoletaBuilder {
    folio: Option<u64>,
    fch_emis: Option<time::Date>,
    fch_venc: Option<time::Date>,
    ind_servicio: Option<u8>,
    documento_id: Option<String>,
    emisor: Option<Emisor>,
    receptor: Option<Receptor>,
    detalles: Vec<Detalle>,
    referencias: Vec<Referencia>,
}

impl BoletaBuilder {
    /// Creates a new `BoletaBuilder`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the authorized document folio number.
    #[must_use]
    pub const fn folio(mut self, folio: u64) -> Self {
        self.folio = Some(folio);
        self
    }

    /// Sets the emission date.
    #[must_use]
    pub const fn fch_emis(mut self, date: time::Date) -> Self {
        self.fch_emis = Some(date);
        self
    }

    /// Parses and sets the emission date from a `YYYY-MM-DD` string.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidDate`] if the date format is invalid.
    pub fn fch_emis_str(mut self, s: &str) -> Result<Self, Error> {
        self.fch_emis = Some(parse_date(s)?);
        Ok(self)
    }

    /// Sets the due date.
    #[must_use]
    pub const fn fch_venc(mut self, date: time::Date) -> Self {
        self.fch_venc = Some(date);
        self
    }

    /// Sets the service indicator.
    #[must_use]
    pub const fn ind_servicio(mut self, ind: u8) -> Self {
        self.ind_servicio = Some(ind);
        self
    }

    /// Overrides the internal document ID attribute (default is `F{folio}T39`).
    #[must_use]
    pub fn documento_id(mut self, id: impl Into<String>) -> Self {
        self.documento_id = Some(id.into());
        self
    }

    /// Sets the document issuer.
    #[must_use]
    pub fn emisor(mut self, emisor: Emisor) -> Self {
        self.emisor = Some(emisor);
        self
    }

    /// Sets the document receiver.
    ///
    /// Per RF-01, receiver can be minimal or omitted (defaults to [`Receptor::consumidor_final`]).
    #[must_use]
    pub fn receptor(mut self, receptor: Receptor) -> Self {
        self.receptor = Some(receptor);
        self
    }

    /// Adds a line item detail.
    #[must_use]
    pub fn item(mut self, item: Detalle) -> Self {
        self.detalles.push(item);
        self
    }

    /// Adds multiple line item details.
    #[must_use]
    pub fn items(mut self, items: impl IntoIterator<Item = Detalle>) -> Self {
        self.detalles.extend(items);
        self
    }

    /// Adds a document reference.
    #[must_use]
    pub fn referencia(mut self, ref_item: Referencia) -> Self {
        self.referencias.push(ref_item);
        self
    }

    /// Adds multiple document references.
    #[must_use]
    pub fn referencias(mut self, refs: impl IntoIterator<Item = Referencia>) -> Self {
        self.referencias.extend(refs);
        self
    }

    /// Builds and validates the `Dte` for Boleta Electrónica (Tipo 39).
    ///
    /// Automatically calculates 19 % IVA and totals. Defaults receiver to
    /// [`Receptor::consumidor_final`] if not explicitly specified (RF-01).
    ///
    /// # Errors
    ///
    /// Returns [`Error::DteValidation`] if required fields are missing or validation rules fail.
    pub fn build(self) -> Result<Dte, Error> {
        let folio = self
            .folio
            .ok_or_else(|| Error::DteValidation("missing folio".into()))?;
        if folio == 0 {
            return Err(Error::DteValidation(
                "folio must be greater than zero".into(),
            ));
        }
        let fch_emis = self
            .fch_emis
            .ok_or_else(|| Error::DteValidation("missing fch_emis (issue date)".into()))?;
        let emisor = self
            .emisor
            .ok_or_else(|| Error::DteValidation("missing emisor".into()))?;
        let receptor = self.receptor.unwrap_or_else(Receptor::consumidor_final);

        if self.detalles.is_empty() {
            return Err(Error::DteValidation(
                "at least one detail line is required".into(),
            ));
        }

        validate_detalles(&self.detalles)?;

        let mut neto = Monto::ZERO;
        let mut exento = Monto::ZERO;
        let mut has_exento = false;

        for item in &self.detalles {
            if item.ind_exe == Some(1) {
                exento = exento.add_monto(item.monto_item)?;
                has_exento = true;
            } else {
                neto = neto.add_monto(item.monto_item)?;
            }
        }

        if neto <= Monto::ZERO {
            return Err(Error::DteValidation(
                "boleta electrónica (39) must contain at least one taxable item (neto > 0)".into(),
            ));
        }

        let mnt_exe_opt = if has_exento && exento > Monto::ZERO {
            Some(exento)
        } else {
            None
        };

        let totales = Totales::afecto(neto, mnt_exe_opt)?;

        let mut id_doc = IdDoc::new(TipoDte::Boleta, folio, fch_emis);
        if let Some(venc) = self.fch_venc {
            id_doc = id_doc.fch_venc(venc);
        }
        if let Some(ind) = self.ind_servicio {
            id_doc = id_doc.ind_servicio(ind);
        }

        let encabezado = Encabezado::new(id_doc, emisor, receptor, totales);
        let mut dte = Dte::with_referencias(encabezado, self.detalles, self.referencias)?;
        if let Some(id) = self.documento_id {
            dte.id = id;
        }
        Ok(dte)
    }
}

/// Builder for Boleta Exenta Electrónica - Tipo 41.
#[derive(Debug, Clone, Default)]
pub struct BoletaExentaBuilder {
    folio: Option<u64>,
    fch_emis: Option<time::Date>,
    fch_venc: Option<time::Date>,
    ind_servicio: Option<u8>,
    documento_id: Option<String>,
    emisor: Option<Emisor>,
    receptor: Option<Receptor>,
    detalles: Vec<Detalle>,
    referencias: Vec<Referencia>,
}

impl BoletaExentaBuilder {
    /// Creates a new `BoletaExentaBuilder`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the authorized document folio number.
    #[must_use]
    pub const fn folio(mut self, folio: u64) -> Self {
        self.folio = Some(folio);
        self
    }

    /// Sets the emission date.
    #[must_use]
    pub const fn fch_emis(mut self, date: time::Date) -> Self {
        self.fch_emis = Some(date);
        self
    }

    /// Parses and sets the emission date from a `YYYY-MM-DD` string.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidDate`] if the date format is invalid.
    pub fn fch_emis_str(mut self, s: &str) -> Result<Self, Error> {
        self.fch_emis = Some(parse_date(s)?);
        Ok(self)
    }

    /// Sets the due date.
    #[must_use]
    pub const fn fch_venc(mut self, date: time::Date) -> Self {
        self.fch_venc = Some(date);
        self
    }

    /// Sets the service indicator.
    #[must_use]
    pub const fn ind_servicio(mut self, ind: u8) -> Self {
        self.ind_servicio = Some(ind);
        self
    }

    /// Overrides the internal document ID attribute (default is `F{folio}T41`).
    #[must_use]
    pub fn documento_id(mut self, id: impl Into<String>) -> Self {
        self.documento_id = Some(id.into());
        self
    }

    /// Sets the document issuer.
    #[must_use]
    pub fn emisor(mut self, emisor: Emisor) -> Self {
        self.emisor = Some(emisor);
        self
    }

    /// Sets the document receiver.
    ///
    /// Per RF-01, receiver can be minimal or omitted (defaults to [`Receptor::consumidor_final`]).
    #[must_use]
    pub fn receptor(mut self, receptor: Receptor) -> Self {
        self.receptor = Some(receptor);
        self
    }

    /// Adds a line item detail.
    #[must_use]
    pub fn item(mut self, item: Detalle) -> Self {
        self.detalles.push(item);
        self
    }

    /// Adds multiple line item details.
    #[must_use]
    pub fn items(mut self, items: impl IntoIterator<Item = Detalle>) -> Self {
        self.detalles.extend(items);
        self
    }

    /// Adds a document reference.
    #[must_use]
    pub fn referencia(mut self, ref_item: Referencia) -> Self {
        self.referencias.push(ref_item);
        self
    }

    /// Adds multiple document references.
    #[must_use]
    pub fn referencias(mut self, refs: impl IntoIterator<Item = Referencia>) -> Self {
        self.referencias.extend(refs);
        self
    }

    /// Builds and validates the `Dte` for Boleta Exenta Electrónica (Tipo 41).
    ///
    /// Ensures all items have `IndExe = 1` and that no IVA tags are generated (DTE-02).
    ///
    /// # Errors
    ///
    /// Returns [`Error::DteValidation`] if required fields are missing or validation rules fail.
    pub fn build(mut self) -> Result<Dte, Error> {
        let folio = self
            .folio
            .ok_or_else(|| Error::DteValidation("missing folio".into()))?;
        if folio == 0 {
            return Err(Error::DteValidation(
                "folio must be greater than zero".into(),
            ));
        }
        let fch_emis = self
            .fch_emis
            .ok_or_else(|| Error::DteValidation("missing fch_emis (issue date)".into()))?;
        let emisor = self
            .emisor
            .ok_or_else(|| Error::DteValidation("missing emisor".into()))?;
        let receptor = self.receptor.unwrap_or_else(Receptor::consumidor_final);

        if self.detalles.is_empty() {
            return Err(Error::DteValidation(
                "at least one detail line is required".into(),
            ));
        }

        // For Boleta Exenta (41), every item is exempt: ensure IndExe is 1.
        for item in &mut self.detalles {
            item.ind_exe = Some(1);
        }

        validate_detalles(&self.detalles)?;

        let mut exento = Monto::ZERO;
        for item in &self.detalles {
            exento = exento.add_monto(item.monto_item)?;
        }

        if exento <= Monto::ZERO {
            return Err(Error::DteValidation(
                "boleta exenta (41) must have positive exempt total (MntExe > 0)".into(),
            ));
        }

        let totales = Totales::exento(exento);

        let mut id_doc = IdDoc::new(TipoDte::BoletaExenta, folio, fch_emis);
        if let Some(venc) = self.fch_venc {
            id_doc = id_doc.fch_venc(venc);
        }
        if let Some(ind) = self.ind_servicio {
            id_doc = id_doc.ind_servicio(ind);
        }

        let encabezado = Encabezado::new(id_doc, emisor, receptor, totales);
        let mut dte = Dte::with_referencias(encabezado, self.detalles, self.referencias)?;
        if let Some(id) = self.documento_id {
            dte.id = id;
        }
        Ok(dte)
    }
}

/// Builder for Guía de Despacho Electrónica - Tipo 52.
#[derive(Debug, Clone, Default)]
pub struct GuiaDespachoBuilder {
    folio: Option<u64>,
    fch_emis: Option<time::Date>,
    fch_venc: Option<time::Date>,
    tipo_despacho: Option<u8>,
    ind_traslado: Option<u8>,
    documento_id: Option<String>,
    emisor: Option<Emisor>,
    receptor: Option<Receptor>,
    transporte: Option<Transporte>,
    detalles: Vec<Detalle>,
    referencias: Vec<Referencia>,
}

impl GuiaDespachoBuilder {
    /// Creates a new `GuiaDespachoBuilder`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the authorized document folio number.
    #[must_use]
    pub const fn folio(mut self, folio: u64) -> Self {
        self.folio = Some(folio);
        self
    }

    /// Sets the emission date.
    #[must_use]
    pub const fn fch_emis(mut self, date: time::Date) -> Self {
        self.fch_emis = Some(date);
        self
    }

    /// Parses and sets the emission date from a `YYYY-MM-DD` string.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidDate`] if the date format is invalid.
    pub fn fch_emis_str(mut self, s: &str) -> Result<Self, Error> {
        self.fch_emis = Some(parse_date(s)?);
        Ok(self)
    }

    /// Sets the expiration / due date.
    #[must_use]
    pub const fn fch_venc(mut self, date: time::Date) -> Self {
        self.fch_venc = Some(date);
        self
    }

    /// Sets the dispatch type code (`TipoDespacho`: 1 = emisor a cliente, 2 = emisor a otras inst., 3 = cliente).
    #[must_use]
    pub const fn tipo_despacho(mut self, tipo: u8) -> Self {
        self.tipo_despacho = Some(tipo);
        self
    }

    /// Sets the mandatory transfer indicator code (`IndTraslado`: 1 = venta, 2 = ventas por efectuar, 3 = consignaciones, 5 = traslado interno, etc.).
    #[must_use]
    pub const fn ind_traslado(mut self, ind: u8) -> Self {
        self.ind_traslado = Some(ind);
        self
    }

    /// Overrides the internal document ID attribute (default is `F{folio}T52`).
    #[must_use]
    pub fn documento_id(mut self, id: impl Into<String>) -> Self {
        self.documento_id = Some(id.into());
        self
    }

    /// Sets the document issuer.
    #[must_use]
    pub fn emisor(mut self, emisor: Emisor) -> Self {
        self.emisor = Some(emisor);
        self
    }

    /// Sets the document receiver.
    #[must_use]
    pub fn receptor(mut self, receptor: Receptor) -> Self {
        self.receptor = Some(receptor);
        self
    }

    /// Sets the transportation / carrier details (`Transporte`).
    #[must_use]
    pub fn transporte(mut self, transporte: Transporte) -> Self {
        self.transporte = Some(transporte);
        self
    }

    /// Adds a line item detail.
    #[must_use]
    pub fn item(mut self, item: Detalle) -> Self {
        self.detalles.push(item);
        self
    }

    /// Adds multiple line item details.
    #[must_use]
    pub fn items(mut self, items: impl IntoIterator<Item = Detalle>) -> Self {
        self.detalles.extend(items);
        self
    }

    /// Adds a document reference.
    #[must_use]
    pub fn referencia(mut self, ref_item: Referencia) -> Self {
        self.referencias.push(ref_item);
        self
    }

    /// Adds multiple document references.
    #[must_use]
    pub fn referencias(mut self, refs: impl IntoIterator<Item = Referencia>) -> Self {
        self.referencias.extend(refs);
        self
    }

    /// Builds and validates the `Dte` for Guía de Despacho Electrónica (Tipo 52).
    ///
    /// Validates mandatory transfer fields conforming to RF-02 (`IndTraslado` in 1..=9).
    ///
    /// # Errors
    ///
    /// Returns [`Error::DteValidation`] if required fields are missing or validation rules fail.
    pub fn build(self) -> Result<Dte, Error> {
        let folio = self
            .folio
            .ok_or_else(|| Error::DteValidation("missing folio".into()))?;
        if folio == 0 {
            return Err(Error::DteValidation(
                "folio must be greater than zero".into(),
            ));
        }
        let fch_emis = self
            .fch_emis
            .ok_or_else(|| Error::DteValidation("missing fch_emis (issue date)".into()))?;
        let emisor = self
            .emisor
            .ok_or_else(|| Error::DteValidation("missing emisor".into()))?;
        let receptor = self
            .receptor
            .ok_or_else(|| Error::DteValidation("missing receptor".into()))?;

        let ind_traslado = self.ind_traslado.ok_or_else(|| {
            Error::DteValidation("guía de despacho (52) requires ind_traslado (RF-02)".into())
        })?;
        if !(1..=9).contains(&ind_traslado) {
            return Err(Error::DteValidation(format!(
                "invalid ind_traslado {ind_traslado}: must be between 1 and 9"
            )));
        }

        if let Some(td) = self.tipo_despacho {
            if !(1..=3).contains(&td) {
                return Err(Error::DteValidation(format!(
                    "invalid tipo_despacho {td}: must be between 1 and 3"
                )));
            }
        }

        if self.detalles.is_empty() {
            return Err(Error::DteValidation(
                "at least one detail line is required".into(),
            ));
        }

        validate_detalles(&self.detalles)?;

        let mut neto = Monto::ZERO;
        let mut exento = Monto::ZERO;
        let mut has_exento = false;

        for item in &self.detalles {
            if item.ind_exe == Some(1) {
                exento = exento.add_monto(item.monto_item)?;
                has_exento = true;
            } else {
                neto = neto.add_monto(item.monto_item)?;
            }
        }

        let totales = if neto > Monto::ZERO {
            let mnt_exe_opt = if has_exento && exento > Monto::ZERO {
                Some(exento)
            } else {
                None
            };
            Totales::afecto(neto, mnt_exe_opt)?
        } else {
            Totales::exento(exento)
        };

        let mut id_doc =
            IdDoc::new(TipoDte::GuiaDespacho, folio, fch_emis).ind_traslado(ind_traslado);
        if let Some(td) = self.tipo_despacho {
            id_doc = id_doc.tipo_despacho(td);
        }
        if let Some(venc) = self.fch_venc {
            id_doc = id_doc.fch_venc(venc);
        }

        let mut encabezado = Encabezado::new(id_doc, emisor, receptor, totales);
        if let Some(trans) = self.transporte {
            encabezado = encabezado.with_transporte(trans);
        }
        let mut dte = Dte::with_referencias(encabezado, self.detalles, self.referencias)?;
        if let Some(id) = self.documento_id {
            dte.id = id;
        }
        Ok(dte)
    }
}

/// Builder for Nota de Débito Electrónica - Tipo 56.
#[derive(Debug, Clone, Default)]
pub struct NotaDebitoBuilder {
    folio: Option<u64>,
    fch_emis: Option<time::Date>,
    fch_venc: Option<time::Date>,
    ind_servicio: Option<u8>,
    documento_id: Option<String>,
    emisor: Option<Emisor>,
    receptor: Option<Receptor>,
    detalles: Vec<Detalle>,
    referencias: Vec<Referencia>,
}

impl NotaDebitoBuilder {
    /// Creates a new `NotaDebitoBuilder`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the authorized document folio number.
    #[must_use]
    pub const fn folio(mut self, folio: u64) -> Self {
        self.folio = Some(folio);
        self
    }

    /// Sets the emission date.
    #[must_use]
    pub const fn fch_emis(mut self, date: time::Date) -> Self {
        self.fch_emis = Some(date);
        self
    }

    /// Parses and sets the emission date from a `YYYY-MM-DD` string.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidDate`] if the date format is invalid.
    pub fn fch_emis_str(mut self, s: &str) -> Result<Self, Error> {
        self.fch_emis = Some(parse_date(s)?);
        Ok(self)
    }

    /// Sets the due date.
    #[must_use]
    pub const fn fch_venc(mut self, date: time::Date) -> Self {
        self.fch_venc = Some(date);
        self
    }

    /// Sets the service indicator.
    #[must_use]
    pub const fn ind_servicio(mut self, ind: u8) -> Self {
        self.ind_servicio = Some(ind);
        self
    }

    /// Overrides the internal document ID attribute (default is `F{folio}T56`).
    #[must_use]
    pub fn documento_id(mut self, id: impl Into<String>) -> Self {
        self.documento_id = Some(id.into());
        self
    }

    /// Sets the document issuer.
    #[must_use]
    pub fn emisor(mut self, emisor: Emisor) -> Self {
        self.emisor = Some(emisor);
        self
    }

    /// Sets the document receiver.
    #[must_use]
    pub fn receptor(mut self, receptor: Receptor) -> Self {
        self.receptor = Some(receptor);
        self
    }

    /// Adds a line item detail.
    #[must_use]
    pub fn item(mut self, item: Detalle) -> Self {
        self.detalles.push(item);
        self
    }

    /// Adds multiple line item details.
    #[must_use]
    pub fn items(mut self, items: impl IntoIterator<Item = Detalle>) -> Self {
        self.detalles.extend(items);
        self
    }

    /// Adds a document reference. At least one reference is mandatory for debit notes (RF-03, DTE-02).
    #[must_use]
    pub fn referencia(mut self, ref_item: Referencia) -> Self {
        self.referencias.push(ref_item);
        self
    }

    /// Adds multiple document references.
    #[must_use]
    pub fn referencias(mut self, refs: impl IntoIterator<Item = Referencia>) -> Self {
        self.referencias.extend(refs);
        self
    }

    /// Convenience method to add a reference to a previous document with action code and reason.
    #[must_use]
    pub fn referencia_doc(
        mut self,
        tpo_doc_ref: impl Into<String>,
        folio_ref: impl Into<String>,
        fch_ref: time::Date,
        cod_ref: u8,
        razon_ref: impl Into<String>,
    ) -> Self {
        let line = u32::try_from(self.referencias.len())
            .unwrap_or(0)
            .saturating_add(1);
        self.referencias.push(Referencia::with_reason(
            line,
            tpo_doc_ref,
            folio_ref,
            fch_ref,
            cod_ref,
            razon_ref,
        ));
        self
    }

    /// Builds and validates the `Dte` for Nota de Débito Electrónica (Tipo 56).
    ///
    /// Exige al menos una `Referencia` válida (DTE-02, RF-03).
    ///
    /// # Errors
    ///
    /// Returns [`Error::DteValidation`] if references are missing or validation rules fail.
    pub fn build(self) -> Result<Dte, Error> {
        let folio = self
            .folio
            .ok_or_else(|| Error::DteValidation("missing folio".into()))?;
        if folio == 0 {
            return Err(Error::DteValidation(
                "folio must be greater than zero".into(),
            ));
        }
        let fch_emis = self
            .fch_emis
            .ok_or_else(|| Error::DteValidation("missing fch_emis (issue date)".into()))?;
        let emisor = self
            .emisor
            .ok_or_else(|| Error::DteValidation("missing emisor".into()))?;
        let receptor = self
            .receptor
            .ok_or_else(|| Error::DteValidation("missing receptor".into()))?;

        if self.referencias.is_empty() {
            return Err(Error::DteValidation(
                "nota de débito (56) requires at least one document reference (Referencia) (DTE-02, RF-03)".into(),
            ));
        }

        if self.detalles.is_empty() {
            return Err(Error::DteValidation(
                "at least one detail line is required".into(),
            ));
        }

        validate_detalles(&self.detalles)?;

        let mut neto = Monto::ZERO;
        let mut exento = Monto::ZERO;
        let mut has_exento = false;

        for item in &self.detalles {
            if item.ind_exe == Some(1) {
                exento = exento.add_monto(item.monto_item)?;
                has_exento = true;
            } else {
                neto = neto.add_monto(item.monto_item)?;
            }
        }

        let totales = if neto > Monto::ZERO {
            let mnt_exe_opt = if has_exento && exento > Monto::ZERO {
                Some(exento)
            } else {
                None
            };
            Totales::afecto(neto, mnt_exe_opt)?
        } else {
            Totales::exento(exento)
        };

        let mut id_doc = IdDoc::new(TipoDte::NotaDebito, folio, fch_emis);
        if let Some(venc) = self.fch_venc {
            id_doc = id_doc.fch_venc(venc);
        }
        if let Some(ind) = self.ind_servicio {
            id_doc = id_doc.ind_servicio(ind);
        }

        let encabezado = Encabezado::new(id_doc, emisor, receptor, totales);
        let mut dte = Dte::with_referencias(encabezado, self.detalles, self.referencias)?;
        if let Some(id) = self.documento_id {
            dte.id = id;
        }
        Ok(dte)
    }
}

/// Builder for Nota de Crédito Electrónica - Tipo 61.
#[derive(Debug, Clone, Default)]
pub struct NotaCreditoBuilder {
    folio: Option<u64>,
    fch_emis: Option<time::Date>,
    fch_venc: Option<time::Date>,
    ind_servicio: Option<u8>,
    documento_id: Option<String>,
    emisor: Option<Emisor>,
    receptor: Option<Receptor>,
    detalles: Vec<Detalle>,
    referencias: Vec<Referencia>,
}

impl NotaCreditoBuilder {
    /// Creates a new `NotaCreditoBuilder`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the authorized document folio number.
    #[must_use]
    pub const fn folio(mut self, folio: u64) -> Self {
        self.folio = Some(folio);
        self
    }

    /// Sets the emission date.
    #[must_use]
    pub const fn fch_emis(mut self, date: time::Date) -> Self {
        self.fch_emis = Some(date);
        self
    }

    /// Parses and sets the emission date from a `YYYY-MM-DD` string.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidDate`] if the date format is invalid.
    pub fn fch_emis_str(mut self, s: &str) -> Result<Self, Error> {
        self.fch_emis = Some(parse_date(s)?);
        Ok(self)
    }

    /// Sets the due date.
    #[must_use]
    pub const fn fch_venc(mut self, date: time::Date) -> Self {
        self.fch_venc = Some(date);
        self
    }

    /// Sets the service indicator.
    #[must_use]
    pub const fn ind_servicio(mut self, ind: u8) -> Self {
        self.ind_servicio = Some(ind);
        self
    }

    /// Overrides the internal document ID attribute (default is `F{folio}T61`).
    #[must_use]
    pub fn documento_id(mut self, id: impl Into<String>) -> Self {
        self.documento_id = Some(id.into());
        self
    }

    /// Sets the document issuer.
    #[must_use]
    pub fn emisor(mut self, emisor: Emisor) -> Self {
        self.emisor = Some(emisor);
        self
    }

    /// Sets the document receiver.
    #[must_use]
    pub fn receptor(mut self, receptor: Receptor) -> Self {
        self.receptor = Some(receptor);
        self
    }

    /// Adds a line item detail.
    #[must_use]
    pub fn item(mut self, item: Detalle) -> Self {
        self.detalles.push(item);
        self
    }

    /// Adds multiple line item details.
    #[must_use]
    pub fn items(mut self, items: impl IntoIterator<Item = Detalle>) -> Self {
        self.detalles.extend(items);
        self
    }

    /// Adds a document reference. At least one reference is mandatory for credit notes (RF-03, DTE-02).
    #[must_use]
    pub fn referencia(mut self, ref_item: Referencia) -> Self {
        self.referencias.push(ref_item);
        self
    }

    /// Adds multiple document references.
    #[must_use]
    pub fn referencias(mut self, refs: impl IntoIterator<Item = Referencia>) -> Self {
        self.referencias.extend(refs);
        self
    }

    /// Convenience method to add a reference to a previous document with action code and reason.
    #[must_use]
    pub fn referencia_doc(
        mut self,
        tpo_doc_ref: impl Into<String>,
        folio_ref: impl Into<String>,
        fch_ref: time::Date,
        cod_ref: u8,
        razon_ref: impl Into<String>,
    ) -> Self {
        let line = u32::try_from(self.referencias.len())
            .unwrap_or(0)
            .saturating_add(1);
        self.referencias.push(Referencia::with_reason(
            line,
            tpo_doc_ref,
            folio_ref,
            fch_ref,
            cod_ref,
            razon_ref,
        ));
        self
    }

    /// Builds and validates the `Dte` for Nota de Crédito Electrónica (Tipo 61).
    ///
    /// Exige al menos una `Referencia` válida (DTE-02, RF-03).
    ///
    /// # Errors
    ///
    /// Returns [`Error::DteValidation`] if references are missing or validation rules fail.
    pub fn build(self) -> Result<Dte, Error> {
        let folio = self
            .folio
            .ok_or_else(|| Error::DteValidation("missing folio".into()))?;
        if folio == 0 {
            return Err(Error::DteValidation(
                "folio must be greater than zero".into(),
            ));
        }
        let fch_emis = self
            .fch_emis
            .ok_or_else(|| Error::DteValidation("missing fch_emis (issue date)".into()))?;
        let emisor = self
            .emisor
            .ok_or_else(|| Error::DteValidation("missing emisor".into()))?;
        let receptor = self
            .receptor
            .ok_or_else(|| Error::DteValidation("missing receptor".into()))?;

        if self.referencias.is_empty() {
            return Err(Error::DteValidation(
                "nota de crédito (61) requires at least one document reference (Referencia) (DTE-02, RF-03)".into(),
            ));
        }

        if self.detalles.is_empty() {
            return Err(Error::DteValidation(
                "at least one detail line is required".into(),
            ));
        }

        validate_detalles(&self.detalles)?;

        let mut neto = Monto::ZERO;
        let mut exento = Monto::ZERO;
        let mut has_exento = false;

        for item in &self.detalles {
            if item.ind_exe == Some(1) {
                exento = exento.add_monto(item.monto_item)?;
                has_exento = true;
            } else {
                neto = neto.add_monto(item.monto_item)?;
            }
        }

        let is_text_correction = self.referencias.iter().any(|r| r.cod_ref == Some(2));

        let totales = if neto > Monto::ZERO {
            let mnt_exe_opt = if has_exento && exento > Monto::ZERO {
                Some(exento)
            } else {
                None
            };
            Totales::afecto(neto, mnt_exe_opt)?
        } else if exento > Monto::ZERO {
            Totales::exento(exento)
        } else if is_text_correction {
            // Text correction allows zero-amount line items
            Totales {
                mnt_neto: None,
                mnt_exe: None,
                tasa_iva: None,
                iva: None,
                mnt_total: Monto::ZERO,
            }
        } else {
            return Err(Error::DteValidation(
                "nota de crédito (61) must have positive net or exempt amounts unless correcting text (CodRef = 2)".into(),
            ));
        };

        let mut id_doc = IdDoc::new(TipoDte::NotaCredito, folio, fch_emis);
        if let Some(venc) = self.fch_venc {
            id_doc = id_doc.fch_venc(venc);
        }
        if let Some(ind) = self.ind_servicio {
            id_doc = id_doc.ind_servicio(ind);
        }

        let encabezado = Encabezado::new(id_doc, emisor, receptor, totales);
        let mut dte = Dte::with_referencias(encabezado, self.detalles, self.referencias)?;
        if let Some(id) = self.documento_id {
            dte.id = id;
        }
        Ok(dte)
    }
}

fn validate_detalles(detalles: &[Detalle]) -> Result<(), Error> {
    for (i, item) in detalles.iter().enumerate() {
        let expected_line = u32::try_from(i)
            .map_err(|_| Error::DteValidation("line count overflow".into()))?
            .saturating_add(1);
        if item.nro_lin_det != expected_line {
            return Err(Error::DteValidation(format!(
                "inconsistent detail line number: expected {expected_line}, got {}",
                item.nro_lin_det
            )));
        }
        if item.nmb_item.trim().is_empty() {
            return Err(Error::DteValidation(format!(
                "empty item name at line {expected_line}"
            )));
        }
        if item.monto_item < Monto::ZERO {
            return Err(Error::DteValidation(format!(
                "negative line amount at line {expected_line}"
            )));
        }
    }
    Ok(())
}

fn validate_referencias(tipo: TipoDte, referencias: &[Referencia]) -> Result<(), Error> {
    if tipo.requires_reference() && referencias.is_empty() {
        return Err(Error::DteValidation(format!(
            "{} ({}) requires at least one document reference (Referencia) (DTE-02, RF-03)",
            tipo.description(),
            tipo.code()
        )));
    }

    for (i, ref_item) in referencias.iter().enumerate() {
        let expected_line = u32::try_from(i)
            .map_err(|_| Error::DteValidation("reference count overflow".into()))?
            .saturating_add(1);
        if ref_item.nro_lin_ref != expected_line {
            return Err(Error::DteValidation(format!(
                "inconsistent reference line number: expected {expected_line}, got {}",
                ref_item.nro_lin_ref
            )));
        }
        if ref_item.tpo_doc_ref.trim().is_empty() {
            return Err(Error::DteValidation(format!(
                "empty TpoDocRef at reference line {expected_line}"
            )));
        }
        if ref_item.folio_ref.trim().is_empty() {
            return Err(Error::DteValidation(format!(
                "empty FolioRef at reference line {expected_line}"
            )));
        }

        if tipo.requires_reference() {
            match ref_item.cod_ref {
                Some(1..=3) => {}
                Some(invalid) => {
                    return Err(Error::DteValidation(format!(
                        "invalid CodRef {invalid} at reference line {expected_line}: must be 1, 2, or 3"
                    )));
                }
                None => {
                    return Err(Error::DteValidation(format!(
                        "missing CodRef at reference line {expected_line}: required for credit and debit notes"
                    )));
                }
            }
            match &ref_item.razon_ref {
                Some(razon) if !razon.trim().is_empty() => {}
                _ => {
                    return Err(Error::DteValidation(format!(
                        "missing or empty RazonRef at reference line {expected_line}: required for credit and debit notes"
                    )));
                }
            }
        }
    }
    Ok(())
}

fn write_element<W: std::io::Write>(
    writer: &mut Writer<W>,
    tag: &str,
    text: &str,
) -> Result<(), Error> {
    writer
        .write_event(Event::Start(BytesStart::new(tag)))
        .map_err(|e| Error::Xml(e.to_string()))?;
    let escaped = quick_xml::escape::escape(text);
    writer
        .write_event(Event::Text(BytesText::new(&escaped)))
        .map_err(|e| Error::Xml(e.to_string()))?;
    writer
        .write_event(Event::End(BytesEnd::new(tag)))
        .map_err(|e| Error::Xml(e.to_string()))?;
    Ok(())
}

fn write_encabezado<W: std::io::Write>(
    writer: &mut Writer<W>,
    enc: &Encabezado,
) -> Result<(), Error> {
    writer
        .write_event(Event::Start(BytesStart::new("Encabezado")))
        .map_err(|e| Error::Xml(e.to_string()))?;

    write_id_doc(writer, &enc.id_doc)?;
    write_emisor(writer, &enc.emisor)?;
    write_receptor(writer, &enc.receptor)?;
    if let Some(trans) = &enc.transporte {
        write_transporte(writer, trans)?;
    }
    write_totales(writer, &enc.totales)?;

    writer
        .write_event(Event::End(BytesEnd::new("Encabezado")))
        .map_err(|e| Error::Xml(e.to_string()))?;
    Ok(())
}

fn write_transporte<W: std::io::Write>(
    writer: &mut Writer<W>,
    trans: &Transporte,
) -> Result<(), Error> {
    writer
        .write_event(Event::Start(BytesStart::new("Transporte")))
        .map_err(|e| Error::Xml(e.to_string()))?;

    if let Some(patente) = &trans.patente {
        write_element(writer, "Patente", patente)?;
    }
    if let Some(rut_trans) = &trans.rut_trans {
        write_element(writer, "RUTTrans", &rut_trans.to_string())?;
    }
    if let Some(chofer) = &trans.chofer {
        writer
            .write_event(Event::Start(BytesStart::new("Chofer")))
            .map_err(|e| Error::Xml(e.to_string()))?;
        if let Some(rut_chofer) = &chofer.rut_chofer {
            write_element(writer, "RUTChofer", &rut_chofer.to_string())?;
        }
        if let Some(nombre) = &chofer.nombre_chofer {
            write_element(writer, "NombreChofer", nombre)?;
        }
        writer
            .write_event(Event::End(BytesEnd::new("Chofer")))
            .map_err(|e| Error::Xml(e.to_string()))?;
    }
    if let Some(dir) = &trans.dir_dest {
        write_element(writer, "DirDest", dir)?;
    }
    if let Some(cmna) = &trans.cmna_dest {
        write_element(writer, "CmnaDest", cmna)?;
    }
    if let Some(ciudad) = &trans.ciudad_dest {
        write_element(writer, "CiudadDest", ciudad)?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("Transporte")))
        .map_err(|e| Error::Xml(e.to_string()))?;
    Ok(())
}

fn write_id_doc<W: std::io::Write>(writer: &mut Writer<W>, id_doc: &IdDoc) -> Result<(), Error> {
    writer
        .write_event(Event::Start(BytesStart::new("IdDoc")))
        .map_err(|e| Error::Xml(e.to_string()))?;

    write_element(writer, "TipoDTE", &id_doc.tipo_dte.code().to_string())?;
    write_element(writer, "Folio", &id_doc.folio.to_string())?;
    write_element(writer, "FchEmis", &format_date(&id_doc.fch_emis))?;

    if let Some(td) = id_doc.tipo_despacho {
        write_element(writer, "TipoDespacho", &td.to_string())?;
    }
    if let Some(it) = id_doc.ind_traslado {
        write_element(writer, "IndTraslado", &it.to_string())?;
    }
    if let Some(ind) = id_doc.ind_servicio {
        write_element(writer, "IndServicio", &ind.to_string())?;
    }
    if let Some(venc) = &id_doc.fch_venc {
        write_element(writer, "FchVenc", &format_date(venc))?;
    }
    if let Some(cdg) = id_doc.term_pago_cdg {
        write_element(writer, "TermPagoCdg", &cdg.to_string())?;
    }
    if let Some(fma) = id_doc.fma_pago {
        write_element(writer, "FmaPago", &fma.to_string())?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("IdDoc")))
        .map_err(|e| Error::Xml(e.to_string()))?;
    Ok(())
}

fn write_emisor<W: std::io::Write>(writer: &mut Writer<W>, emisor: &Emisor) -> Result<(), Error> {
    writer
        .write_event(Event::Start(BytesStart::new("Emisor")))
        .map_err(|e| Error::Xml(e.to_string()))?;

    write_element(writer, "RUTEmisor", &emisor.rut_emisor.to_string())?;
    write_element(writer, "RznSoc", &emisor.rzn_soc)?;
    write_element(writer, "GiroEmis", &emisor.giro_emis)?;

    for act in &emisor.acteco {
        write_element(writer, "Acteco", &act.to_string())?;
    }
    if let Some(cdg) = &emisor.cdg_sii_sucur {
        write_element(writer, "CdgSIISucur", cdg)?;
    }

    write_element(writer, "DirOrigen", &emisor.dir_origen)?;
    write_element(writer, "CmnaOrigen", &emisor.cmna_origen)?;

    if let Some(ciudad) = &emisor.ciudad_origen {
        write_element(writer, "CiudadOrigen", ciudad)?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("Emisor")))
        .map_err(|e| Error::Xml(e.to_string()))?;
    Ok(())
}

fn write_receptor<W: std::io::Write>(
    writer: &mut Writer<W>,
    receptor: &Receptor,
) -> Result<(), Error> {
    writer
        .write_event(Event::Start(BytesStart::new("Receptor")))
        .map_err(|e| Error::Xml(e.to_string()))?;

    write_element(writer, "RUTRecep", &receptor.rut_recep.to_string())?;
    write_element(writer, "RznSocRecep", &receptor.rzn_soc_recep)?;

    if let Some(giro) = &receptor.giro_recep {
        write_element(writer, "GiroRecep", giro)?;
    }
    if let Some(dir) = &receptor.dir_recep {
        write_element(writer, "DirRecep", dir)?;
    }
    if let Some(cmna) = &receptor.cmna_recep {
        write_element(writer, "CmnaRecep", cmna)?;
    }
    if let Some(ciudad) = &receptor.ciudad_recep {
        write_element(writer, "CiudadRecep", ciudad)?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("Receptor")))
        .map_err(|e| Error::Xml(e.to_string()))?;
    Ok(())
}

fn write_totales<W: std::io::Write>(
    writer: &mut Writer<W>,
    totales: &Totales,
) -> Result<(), Error> {
    writer
        .write_event(Event::Start(BytesStart::new("Totales")))
        .map_err(|e| Error::Xml(e.to_string()))?;

    if let Some(neto) = totales.mnt_neto {
        write_element(writer, "MntNeto", &neto.to_string())?;
    }
    if let Some(exe) = totales.mnt_exe {
        write_element(writer, "MntExe", &exe.to_string())?;
    }
    if let Some(tasa) = totales.tasa_iva {
        write_element(writer, "TasaIVA", &tasa.to_string())?;
    }
    if let Some(iva) = totales.iva {
        write_element(writer, "IVA", &iva.to_string())?;
    }
    write_element(writer, "MntTotal", &totales.mnt_total.to_string())?;

    writer
        .write_event(Event::End(BytesEnd::new("Totales")))
        .map_err(|e| Error::Xml(e.to_string()))?;
    Ok(())
}

fn write_detalle<W: std::io::Write>(
    writer: &mut Writer<W>,
    detalle: &Detalle,
) -> Result<(), Error> {
    writer
        .write_event(Event::Start(BytesStart::new("Detalle")))
        .map_err(|e| Error::Xml(e.to_string()))?;

    write_element(writer, "NroLinDet", &detalle.nro_lin_det.to_string())?;

    for cdg in &detalle.cdg_item {
        writer
            .write_event(Event::Start(BytesStart::new("CdgItem")))
            .map_err(|e| Error::Xml(e.to_string()))?;
        write_element(writer, "TpoCodigo", &cdg.tpo_codigo)?;
        write_element(writer, "VlrCodigo", &cdg.vlr_codigo)?;
        writer
            .write_event(Event::End(BytesEnd::new("CdgItem")))
            .map_err(|e| Error::Xml(e.to_string()))?;
    }

    if let Some(ind) = detalle.ind_exe {
        write_element(writer, "IndExe", &ind.to_string())?;
    }

    write_element(writer, "NmbItem", &detalle.nmb_item)?;

    if let Some(dsc) = &detalle.dsc_item {
        write_element(writer, "DscItem", dsc)?;
    }
    if let Some(qty) = detalle.qty_item {
        write_element(writer, "QtyItem", &qty.to_string())?;
    }
    if let Some(unmd) = &detalle.unmd_item {
        write_element(writer, "UnmdItem", unmd)?;
    }
    if let Some(prc) = detalle.prc_item {
        write_element(writer, "PrcItem", &prc.to_string())?;
    }

    write_element(writer, "MontoItem", &detalle.monto_item.to_string())?;

    writer
        .write_event(Event::End(BytesEnd::new("Detalle")))
        .map_err(|e| Error::Xml(e.to_string()))?;
    Ok(())
}

fn write_referencia<W: std::io::Write>(
    writer: &mut Writer<W>,
    referencia: &Referencia,
) -> Result<(), Error> {
    writer
        .write_event(Event::Start(BytesStart::new("Referencia")))
        .map_err(|e| Error::Xml(e.to_string()))?;

    write_element(writer, "NroLinRef", &referencia.nro_lin_ref.to_string())?;
    write_element(writer, "TpoDocRef", &referencia.tpo_doc_ref)?;
    if let Some(ind) = referencia.ind_global {
        write_element(writer, "IndGlobal", &ind.to_string())?;
    }
    write_element(writer, "FolioRef", &referencia.folio_ref)?;
    if let Some(rut) = &referencia.rut_otr {
        write_element(writer, "RUTOtr", rut)?;
    }
    write_element(writer, "FchRef", &format_date(&referencia.fch_ref))?;
    if let Some(cod) = referencia.cod_ref {
        write_element(writer, "CodRef", &cod.to_string())?;
    }
    if let Some(razon) = &referencia.razon_ref {
        write_element(writer, "RazonRef", razon)?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("Referencia")))
        .map_err(|e| Error::Xml(e.to_string()))?;
    Ok(())
}
