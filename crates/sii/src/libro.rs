//! Electronic purchase and sales ledger (IECV, LIBRO-01).
//!
//! Provides the data model, XML serialization and digital signing for the Chilean SII
//! Electronic Information of Purchase and Sales Ledgers (*Información Electrónica de
//! Libros de Compra y Venta*, IECV), conforming to schema `LibroCV_v10.xsd` and
//! `docs/dte-spec.md` §8.
//!
//! A [`LibroCompraVenta`] encapsulates an `<EnvioLibro>` containing:
//! - [`CaratulaLibro`]: Header metadata (tax period, issuer RUT, sender RUT, operation kind).
//! - [`TotalesPeriodo`]: Optional summary totals per document type (`ResumenPeriodo`).
//! - [`DetalleLibro`]: Detailed per-document lines (`Detalle`).
//! - `TmstFirma`: Timestamp from the injected [`Clock`].
//!
//! The document is signed using enveloped `XMLDSig` over the `<EnvioLibro ID="...">`
//! element, with the signature placed right before the closing `</LibroCompraVenta>` tag.

use crate::codes::TipoDte;
use crate::dte::{Dte, DteTimbrado, format_date, parse_date};
use crate::error::Error;
use crate::signature::{Certificate, SignatureOptions, Signer, sign_targeting_before};
use crate::types::{Clock, Monto, Rut, format_timestamp};
use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Write as _;

/// Operation type (`TipoOperacion` in SII IECV schema).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TipoOperacion {
    /// Purchase ledger (`COMPRA`).
    Compra,
    /// Sales ledger (`VENTA`).
    Venta,
}

impl TipoOperacion {
    /// Returns the exact SII XML representation (`COMPRA` or `VENTA`).
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Compra => "COMPRA",
            Self::Venta => "VENTA",
        }
    }
}

impl fmt::Display for TipoOperacion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Ledger classification (`TipoLibro` in SII IECV schema).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TipoLibro {
    /// Standard monthly ledger (`MENSUAL`).
    #[default]
    Mensual,
    /// Special ledger (`ESPECIAL`).
    Especial,
    /// Rectifying ledger (`RECTIFICA`).
    Rectifica,
}

impl TipoLibro {
    /// Returns the exact SII XML representation (`MENSUAL`, `ESPECIAL`, or `RECTIFICA`).
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Mensual => "MENSUAL",
            Self::Especial => "ESPECIAL",
            Self::Rectifica => "RECTIFICA",
        }
    }
}

impl fmt::Display for TipoLibro {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Submission kind (`TipoEnvio` in SII IECV schema).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TipoEnvio {
    /// Complete period submission (`TOTAL`).
    #[default]
    Total,
    /// Partial period submission (`PARCIAL`).
    Parcial,
    /// Final period submission (`FINAL`).
    Final,
    /// Adjustment submission (`AJUSTE`).
    Ajuste,
}

impl TipoEnvio {
    /// Returns the exact SII XML representation (`TOTAL`, `PARCIAL`, `FINAL`, or `AJUSTE`).
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Total => "TOTAL",
            Self::Parcial => "PARCIAL",
            Self::Final => "FINAL",
            Self::Ajuste => "AJUSTE",
        }
    }
}

impl fmt::Display for TipoEnvio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Tax period (`PeriodoTributario` in SII schema) formatted as `YYYY-MM`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PeriodoTributario {
    year: i32,
    month: u8,
}

impl PeriodoTributario {
    /// Creates a new `PeriodoTributario`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DteValidation`] if `year` is outside 2000..=2099 or `month` is outside 1..=12.
    pub fn new(year: i32, month: u8) -> Result<Self, Error> {
        if !(2000..=2099).contains(&year) {
            return Err(Error::DteValidation(format!(
                "invalid year in tax period: {year} (expected 2000..=2099)"
            )));
        }
        if !(1..=12).contains(&month) {
            return Err(Error::DteValidation(format!(
                "invalid month in tax period: {month} (expected 1..=12)"
            )));
        }
        Ok(Self { year, month })
    }

    /// Parses a `YYYY-MM` string into a `PeriodoTributario`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DteValidation`] if format is invalid or values are out of range.
    pub fn parse(s: &str) -> Result<Self, Error> {
        let trimmed = s.trim();
        let parts: Vec<&str> = trimmed.split('-').collect();
        if parts.len() != 2 {
            return Err(Error::DteValidation(format!(
                "expected YYYY-MM format, got: {trimmed}"
            )));
        }
        let year_str = parts
            .first()
            .ok_or_else(|| Error::DteValidation("missing year in period".into()))?;
        let month_str = parts
            .get(1)
            .ok_or_else(|| Error::DteValidation("missing month in period".into()))?;

        let year: i32 = year_str
            .parse()
            .map_err(|e| Error::DteValidation(format!("invalid year: {e}")))?;
        let month: u8 = month_str
            .parse()
            .map_err(|e| Error::DteValidation(format!("invalid month: {e}")))?;

        Self::new(year, month)
    }

    /// Returns the year component.
    #[must_use]
    pub const fn year(&self) -> i32 {
        self.year
    }

    /// Returns the month component (1..=12).
    #[must_use]
    pub const fn month(&self) -> u8 {
        self.month
    }

    /// Formats the period compactly as `YYYYMM`.
    #[must_use]
    pub fn to_compact_str(&self) -> String {
        format!("{:04}{:02}", self.year, self.month)
    }
}

impl fmt::Display for PeriodoTributario {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}", self.year, self.month)
    }
}

#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize)]
struct CaratulaLibroHelper {
    rut_emisor_libro: Rut,
    rut_envia: Rut,
    periodo_tributario: PeriodoTributario,
    fch_resol: String,
    nro_resol: u32,
    tipo_operacion: TipoOperacion,
    tipo_libro: TipoLibro,
    tipo_envio: TipoEnvio,
    folio_notificacion: Option<u64>,
}

/// Cover metadata (`Caratula` in SII IECV schema).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaratulaLibro {
    /// Tax ID of the taxpayer to whom this ledger belongs (`RutEmisorLibro`).
    pub rut_emisor_libro: Rut,
    /// Tax ID of the person/entity submitting the file (`RutEnvia`).
    pub rut_envia: Rut,
    /// Tax period of the ledger (`PeriodoTributario`).
    pub periodo_tributario: PeriodoTributario,
    /// Date of the SII resolution authorizing electronic documents (`FchResol`).
    pub fch_resol: time::Date,
    /// Resolution number (`NroResol`, 0 for certification).
    pub nro_resol: u32,
    /// Operation kind (`TipoOperacion`: purchase or sales).
    pub tipo_operacion: TipoOperacion,
    /// Ledger kind (`TipoLibro`: monthly, special, rectifying).
    pub tipo_libro: TipoLibro,
    /// Submission kind (`TipoEnvio`: total, partial, etc.).
    pub tipo_envio: TipoEnvio,
    /// Optional notification folio (`FolioNotificacion`).
    pub folio_notificacion: Option<u64>,
}

impl CaratulaLibro {
    /// Creates a new `CaratulaLibro` with default [`TipoLibro::Mensual`] and [`TipoEnvio::Total`].
    #[must_use]
    pub fn new(
        rut_emisor_libro: Rut,
        rut_envia: Rut,
        periodo_tributario: PeriodoTributario,
        fch_resol: time::Date,
        nro_resol: u32,
        tipo_operacion: TipoOperacion,
    ) -> Self {
        Self {
            rut_emisor_libro,
            rut_envia,
            periodo_tributario,
            fch_resol,
            nro_resol,
            tipo_operacion,
            tipo_libro: TipoLibro::default(),
            tipo_envio: TipoEnvio::default(),
            folio_notificacion: None,
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for CaratulaLibro {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let helper = CaratulaLibroHelper {
            rut_emisor_libro: self.rut_emisor_libro,
            rut_envia: self.rut_envia,
            periodo_tributario: self.periodo_tributario,
            fch_resol: format_date(&self.fch_resol),
            nro_resol: self.nro_resol,
            tipo_operacion: self.tipo_operacion,
            tipo_libro: self.tipo_libro,
            tipo_envio: self.tipo_envio,
            folio_notificacion: self.folio_notificacion,
        };
        helper.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for CaratulaLibro {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = CaratulaLibroHelper::deserialize(deserializer)?;
        let fch_resol = parse_date(&helper.fch_resol).map_err(serde::de::Error::custom)?;
        Ok(Self {
            rut_emisor_libro: helper.rut_emisor_libro,
            rut_envia: helper.rut_envia,
            periodo_tributario: helper.periodo_tributario,
            fch_resol,
            nro_resol: helper.nro_resol,
            tipo_operacion: helper.tipo_operacion,
            tipo_libro: helper.tipo_libro,
            tipo_envio: helper.tipo_envio,
            folio_notificacion: helper.folio_notificacion,
        })
    }
}

#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize)]
struct DetalleLibroHelper {
    tipo_doc: TipoDte,
    nro_doc: u64,
    anulado: bool,
    tasa_imp: Option<u32>,
    fch_doc: Option<String>,
    rut_doc: Option<Rut>,
    rzn_soc: Option<String>,
    mnt_exe: Option<Monto>,
    mnt_neto: Option<Monto>,
    mnt_iva: Option<Monto>,
    mnt_total: Option<Monto>,
}

/// Single document record within a purchase or sales ledger (`Detalle` in SII schema).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetalleLibro {
    /// Document type code (`TpoDoc`).
    pub tipo_doc: TipoDte,
    /// Document folio / identifier (`NroDoc`).
    pub nro_doc: u64,
    /// Whether the document is annulled (`Anulado = A`).
    pub anulado: bool,
    /// Optional tax rate percentage (`TasaImp`, e.g. 19 for 19%).
    pub tasa_imp: Option<u32>,
    /// Document issue date (`FchDoc`).
    pub fch_doc: Option<time::Date>,
    /// Counterparty tax ID (`RUTDoc`: buyer in sales, supplier in purchases).
    pub rut_doc: Option<Rut>,
    /// Counterparty legal name (`RznSoc`).
    pub rzn_soc: Option<String>,
    /// Exempt amount (`MntExe`).
    pub mnt_exe: Option<Monto>,
    /// Net amount subject to IVA (`MntNeto`).
    pub mnt_neto: Option<Monto>,
    /// Value-added tax amount (`MntIVA`).
    pub mnt_iva: Option<Monto>,
    /// Total document amount (`MntTotal`).
    pub mnt_total: Option<Monto>,
}

impl DetalleLibro {
    /// Creates a new document detail with required fields.
    #[must_use]
    pub fn new(tipo_doc: TipoDte, nro_doc: u64) -> Self {
        Self {
            tipo_doc,
            nro_doc,
            anulado: false,
            tasa_imp: None,
            fch_doc: None,
            rut_doc: None,
            rzn_soc: None,
            mnt_exe: None,
            mnt_neto: None,
            mnt_iva: None,
            mnt_total: None,
        }
    }

    /// Constructs a `DetalleLibro` from an existing [`Dte`].
    ///
    /// Automatically extracts counterparty information (receptor for sales, issuer for purchases),
    /// issue date, and financial amounts.
    #[must_use]
    pub fn from_dte(dte: &Dte, operacion: TipoOperacion) -> Self {
        let (rut, rzn) = match operacion {
            TipoOperacion::Venta => (
                Some(dte.encabezado.receptor.rut_recep),
                Some(dte.encabezado.receptor.rzn_soc_recep.clone()),
            ),
            TipoOperacion::Compra => (
                Some(dte.encabezado.emisor.rut_emisor),
                Some(dte.encabezado.emisor.rzn_soc.clone()),
            ),
        };

        Self {
            tipo_doc: dte.encabezado.id_doc.tipo_dte,
            nro_doc: dte.encabezado.id_doc.folio,
            anulado: false,
            tasa_imp: dte.encabezado.totales.tasa_iva,
            fch_doc: Some(dte.encabezado.id_doc.fch_emis),
            rut_doc: rut,
            rzn_soc: rzn,
            mnt_exe: dte.encabezado.totales.mnt_exe,
            mnt_neto: dte.encabezado.totales.mnt_neto,
            mnt_iva: dte.encabezado.totales.iva,
            mnt_total: Some(dte.encabezado.totales.mnt_total),
        }
    }

    /// Sets whether the document is annulled (`<Anulado>A</Anulado>`).
    #[must_use]
    pub fn anulado(mut self, anulado: bool) -> Self {
        self.anulado = anulado;
        self
    }

    /// Sets the tax rate percentage.
    #[must_use]
    pub fn tasa_imp(mut self, tasa: u32) -> Self {
        self.tasa_imp = Some(tasa);
        self
    }

    /// Sets the document date.
    #[must_use]
    pub fn fch_doc(mut self, date: time::Date) -> Self {
        self.fch_doc = Some(date);
        self
    }

    /// Parses and sets the document date from a `YYYY-MM-DD` string.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidDate`] if the date string is malformed.
    pub fn fch_doc_str(mut self, s: &str) -> Result<Self, Error> {
        self.fch_doc = Some(parse_date(s)?);
        Ok(self)
    }

    /// Sets the counterparty RUT.
    #[must_use]
    pub fn rut_doc(mut self, rut: Rut) -> Self {
        self.rut_doc = Some(rut);
        self
    }

    /// Sets the counterparty legal name.
    #[must_use]
    pub fn rzn_soc(mut self, rzn: impl Into<String>) -> Self {
        self.rzn_soc = Some(rzn.into());
        self
    }

    /// Sets the exempt amount.
    #[must_use]
    pub fn mnt_exe(mut self, m: Monto) -> Self {
        self.mnt_exe = Some(m);
        self
    }

    /// Sets the net amount subject to IVA.
    #[must_use]
    pub fn mnt_neto(mut self, m: Monto) -> Self {
        self.mnt_neto = Some(m);
        self
    }

    /// Sets the IVA amount.
    #[must_use]
    pub fn mnt_iva(mut self, m: Monto) -> Self {
        self.mnt_iva = Some(m);
        self
    }

    /// Sets the explicit total amount.
    #[must_use]
    pub fn mnt_total(mut self, m: Monto) -> Self {
        self.mnt_total = Some(m);
        self
    }

    /// Returns the effective total amount, calculating `mnt_neto + mnt_iva + mnt_exe`
    /// if `mnt_total` was not explicitly set.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidMonto`] if calculating total overflows.
    pub fn mnt_total_efectivo(&self) -> Result<Monto, Error> {
        if let Some(total) = self.mnt_total {
            return Ok(total);
        }
        let mut total = Monto::ZERO;
        if let Some(neto) = self.mnt_neto {
            total = total.add_monto(neto)?;
        }
        if let Some(iva) = self.mnt_iva {
            total = total.add_monto(iva)?;
        }
        if let Some(exe) = self.mnt_exe {
            total = total.add_monto(exe)?;
        }
        Ok(total)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for DetalleLibro {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let helper = DetalleLibroHelper {
            tipo_doc: self.tipo_doc,
            nro_doc: self.nro_doc,
            anulado: self.anulado,
            tasa_imp: self.tasa_imp,
            fch_doc: self.fch_doc.as_ref().map(format_date),
            rut_doc: self.rut_doc,
            rzn_soc: self.rzn_soc.clone(),
            mnt_exe: self.mnt_exe,
            mnt_neto: self.mnt_neto,
            mnt_iva: self.mnt_iva,
            mnt_total: self.mnt_total,
        };
        helper.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for DetalleLibro {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = DetalleLibroHelper::deserialize(deserializer)?;
        let fch_doc = match helper.fch_doc {
            Some(s) => Some(parse_date(&s).map_err(serde::de::Error::custom)?),
            None => None,
        };
        Ok(Self {
            tipo_doc: helper.tipo_doc,
            nro_doc: helper.nro_doc,
            anulado: helper.anulado,
            tasa_imp: helper.tasa_imp,
            fch_doc,
            rut_doc: helper.rut_doc,
            rzn_soc: helper.rzn_soc,
            mnt_exe: helper.mnt_exe,
            mnt_neto: helper.mnt_neto,
            mnt_iva: helper.mnt_iva,
            mnt_total: helper.mnt_total,
        })
    }
}

/// Summary totals for a single document type within a tax period (`TotalesPeriodo` in SII schema).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TotalesPeriodo {
    /// Document type summarized (`TpoDoc`).
    pub tpo_doc: TipoDte,
    /// Total count of documents of this type (`TotDoc`).
    pub tot_doc: u32,
    /// Count of annulled documents (`TotAnulado`).
    pub tot_anulado: Option<u32>,
    /// Total exempt amount (`TotMntExe`).
    pub tot_mnt_exe: Monto,
    /// Total net amount (`TotMntNeto`).
    pub tot_mnt_neto: Monto,
    /// Total IVA amount (`TotMntIVA`).
    pub tot_mnt_iva: Monto,
    /// Total amount (`TotMntTotal`).
    pub tot_mnt_total: Monto,
}

impl TotalesPeriodo {
    /// Creates a new `TotalesPeriodo` initialized to zero amounts.
    #[must_use]
    pub fn new(tpo_doc: TipoDte, tot_doc: u32) -> Self {
        Self {
            tpo_doc,
            tot_doc,
            tot_anulado: None,
            tot_mnt_exe: Monto::ZERO,
            tot_mnt_neto: Monto::ZERO,
            tot_mnt_iva: Monto::ZERO,
            tot_mnt_total: Monto::ZERO,
        }
    }

    /// Sets the count of annulled documents.
    #[must_use]
    pub fn tot_anulado(mut self, n: u32) -> Self {
        self.tot_anulado = Some(n);
        self
    }

    /// Sets total exempt amount.
    #[must_use]
    pub fn tot_mnt_exe(mut self, m: Monto) -> Self {
        self.tot_mnt_exe = m;
        self
    }

    /// Sets total net amount.
    #[must_use]
    pub fn tot_mnt_neto(mut self, m: Monto) -> Self {
        self.tot_mnt_neto = m;
        self
    }

    /// Sets total IVA amount.
    #[must_use]
    pub fn tot_mnt_iva(mut self, m: Monto) -> Self {
        self.tot_mnt_iva = m;
        self
    }

    /// Sets total amount.
    #[must_use]
    pub fn tot_mnt_total(mut self, m: Monto) -> Self {
        self.tot_mnt_total = m;
        self
    }
}

/// Accumulator used internally to aggregate totals per document type.
struct TotalesAccumulator {
    tipo: TipoDte,
    tot_doc: u32,
    tot_anulado: u32,
    tot_mnt_exe: Monto,
    tot_mnt_neto: Monto,
    tot_mnt_iva: Monto,
    tot_mnt_total: Monto,
}

impl TotalesAccumulator {
    fn new(tipo: TipoDte) -> Self {
        Self {
            tipo,
            tot_doc: 0,
            tot_anulado: 0,
            tot_mnt_exe: Monto::ZERO,
            tot_mnt_neto: Monto::ZERO,
            tot_mnt_iva: Monto::ZERO,
            tot_mnt_total: Monto::ZERO,
        }
    }

    fn add_document(&mut self, doc: &DetalleLibro) -> Result<(), Error> {
        self.tot_doc = self
            .tot_doc
            .checked_add(1)
            .ok_or_else(|| Error::DteValidation("tot_doc overflow".into()))?;

        if doc.anulado {
            self.tot_anulado = self
                .tot_anulado
                .checked_add(1)
                .ok_or_else(|| Error::DteValidation("tot_anulado overflow".into()))?;
            return Ok(());
        }

        if let Some(exe) = doc.mnt_exe {
            self.tot_mnt_exe = self.tot_mnt_exe.add_monto(exe)?;
        }
        if let Some(neto) = doc.mnt_neto {
            self.tot_mnt_neto = self.tot_mnt_neto.add_monto(neto)?;
        }
        if let Some(iva) = doc.mnt_iva {
            self.tot_mnt_iva = self.tot_mnt_iva.add_monto(iva)?;
        }
        let total = doc.mnt_total_efectivo()?;
        self.tot_mnt_total = self.tot_mnt_total.add_monto(total)?;
        Ok(())
    }

    fn into_totales(self) -> TotalesPeriodo {
        TotalesPeriodo {
            tpo_doc: self.tipo,
            tot_doc: self.tot_doc,
            tot_anulado: if self.tot_anulado > 0 {
                Some(self.tot_anulado)
            } else {
                None
            },
            tot_mnt_exe: self.tot_mnt_exe,
            tot_mnt_neto: self.tot_mnt_neto,
            tot_mnt_iva: self.tot_mnt_iva,
            tot_mnt_total: self.tot_mnt_total,
        }
    }
}

/// Electronic Purchase and Sales Ledger (`LibroCompraVenta` in SII IECV schema).
///
/// Wraps an `<EnvioLibro>` containing `<Caratula>`, optional `<ResumenPeriodo>`,
/// `<Detalle>` records, and carries an enveloped `XMLDSig` signature over the
/// `<EnvioLibro ID="...">` element (RF-01, RF-02, `docs/dte-spec.md` §8).
///
/// # Example
///
/// ```
/// use sii::libro::{LibroCompraVenta, PeriodoTributario, TipoOperacion};
/// use sii::types::Rut;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let emisor_rut = Rut::new(76_086_428, '5')?;
/// let envia_rut = Rut::new(12_345_678, '5')?;
/// let resol_date = sii::dte::parse_date("2026-01-15")?;
///
/// let libro = LibroCompraVenta::nuevo(
///     emisor_rut,
///     envia_rut,
///     PeriodoTributario::new(2026, 11)?,
///     resol_date,
///     0,
///     TipoOperacion::Venta,
/// );
/// assert_eq!(libro.caratula().rut_emisor_libro, emisor_rut);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct LibroCompraVenta {
    caratula: CaratulaLibro,
    envio_id: String,
    detalles: Vec<DetalleLibro>,
    totales_manuales: Vec<TotalesPeriodo>,
}

impl LibroCompraVenta {
    /// Creates a new `LibroCompraVenta` builder.
    ///
    /// Automatically generates a default `envio_id` as `Libro_{emisor}_{periodo}`.
    #[must_use]
    pub fn nuevo(
        rut_emisor: Rut,
        rut_envia: Rut,
        periodo: PeriodoTributario,
        fch_resol: time::Date,
        nro_resol: u32,
        tipo_operacion: TipoOperacion,
    ) -> Self {
        let default_id = format!("Libro_{}_{}", rut_emisor.body(), periodo.to_compact_str());
        Self {
            caratula: CaratulaLibro::new(
                rut_emisor,
                rut_envia,
                periodo,
                fch_resol,
                nro_resol,
                tipo_operacion,
            ),
            envio_id: default_id,
            detalles: Vec::new(),
            totales_manuales: Vec::new(),
        }
    }

    /// Sets the ledger kind ([`TipoLibro::Mensual`], [`TipoLibro::Especial`], [`TipoLibro::Rectifica`]).
    #[must_use]
    pub fn tipo_libro(mut self, tipo: TipoLibro) -> Self {
        self.caratula.tipo_libro = tipo;
        self
    }

    /// Sets the submission kind ([`TipoEnvio::Total`], [`TipoEnvio::Parcial`], etc.).
    #[must_use]
    pub fn tipo_envio(mut self, tipo: TipoEnvio) -> Self {
        self.caratula.tipo_envio = tipo;
        self
    }

    /// Sets the optional notification folio.
    #[must_use]
    pub fn folio_notificacion(mut self, folio: u64) -> Self {
        self.caratula.folio_notificacion = Some(folio);
        self
    }

    /// Sets a custom XML ID for the `<EnvioLibro ID="...">` element.
    #[must_use]
    pub fn envio_id(mut self, id: impl Into<String>) -> Self {
        self.envio_id = id.into();
        self
    }

    /// Returns a reference to the ledger's cover metadata.
    #[must_use]
    pub const fn caratula(&self) -> &CaratulaLibro {
        &self.caratula
    }

    /// Returns the ID attribute value used on the `<EnvioLibro>` element.
    #[must_use]
    pub fn envio_id_str(&self) -> &str {
        &self.envio_id
    }

    /// Adds a single document record to the ledger.
    pub fn agregar_detalle(&mut self, detalle: DetalleLibro) -> &mut Self {
        self.detalles.push(detalle);
        self
    }

    /// Converts and adds an existing [`Dte`] to the ledger according to the ledger's operation kind.
    pub fn agregar_dte(&mut self, dte: &Dte) -> &mut Self {
        let detalle = DetalleLibro::from_dte(dte, self.caratula.tipo_operacion);
        self.agregar_detalle(detalle)
    }

    /// Converts and adds an existing stamped [`DteTimbrado`] to the ledger.
    pub fn agregar_dte_timbrado(&mut self, dte: &DteTimbrado) -> &mut Self {
        self.agregar_dte(dte.dte())
    }

    /// Adds manual summary totals for a document type.
    ///
    /// When manual totals are supplied, auto-aggregation from detail rows is skipped.
    pub fn agregar_totales(&mut self, totales: TotalesPeriodo) -> &mut Self {
        self.totales_manuales.push(totales);
        self
    }

    /// Returns a slice of all document details currently held in the ledger.
    #[must_use]
    pub fn detalles(&self) -> &[DetalleLibro] {
        &self.detalles
    }

    /// Returns the number of document detail lines in the ledger.
    #[must_use]
    pub fn len(&self) -> usize {
        self.detalles.len()
    }

    /// Returns `true` if the ledger contains no document detail lines.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.detalles.is_empty()
    }

    /// Calculates or returns the summary totals for the tax period.
    ///
    /// If manual totals were added with [`Self::agregar_totales`], those are returned.
    /// Otherwise, totals are dynamically computed from [`Self::detalles`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidMonto`] or [`Error::DteValidation`] if summation overflows.
    pub fn calcular_totales(&self) -> Result<Vec<TotalesPeriodo>, Error> {
        if !self.totales_manuales.is_empty() {
            return Ok(self.totales_manuales.clone());
        }

        let mut groups: BTreeMap<u16, TotalesAccumulator> = BTreeMap::new();
        for doc in &self.detalles {
            let entry = groups
                .entry(doc.tipo_doc.code())
                .or_insert_with(|| TotalesAccumulator::new(doc.tipo_doc));
            entry.add_document(doc)?;
        }

        Ok(groups
            .into_values()
            .map(TotalesAccumulator::into_totales)
            .collect())
    }

    /// Serializes the unsigned ledger XML, timestamping `TmstFirma` from `clock`.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if serialization fails or calculations overflow.
    pub fn to_xml(&self, clock: &dyn Clock) -> Result<Vec<u8>, Error> {
        let mut out = String::new();
        let _ = write!(
            out,
            "<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><LibroCompraVenta xmlns=\"http://www.sii.cl/SiiDte\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://www.sii.cl/SiiDte LibroCV_v10.xsd\" version=\"1.0\"><EnvioLibro ID=\"{}\">",
            self.envio_id
        );
        write_caratula(&mut out, &self.caratula);
        let totales = self.calcular_totales()?;
        write_resumen_periodo(&mut out, &totales);
        for d in &self.detalles {
            write_detalle(&mut out, d)?;
        }
        let tmst = format_timestamp(&clock.now());
        let _ = write!(
            out,
            "<TmstFirma>{tmst}</TmstFirma></EnvioLibro></LibroCompraVenta>"
        );
        Ok(out.into_bytes())
    }

    /// Builds and digitally signs the ledger document.
    ///
    /// Signs the `<EnvioLibro ID="...">` element with an enveloped `XMLDSig` signature
    /// inserted right before the closing `</LibroCompraVenta>` tag (RF-02, SIG-01).
    ///
    /// # Errors
    ///
    /// Returns [`Error`] if serialization or signing fails.
    pub fn firmar(
        &self,
        clock: &dyn Clock,
        signer: &dyn Signer,
        cert: &dyn Certificate,
        options: SignatureOptions,
    ) -> Result<Vec<u8>, Error> {
        let xml = self.to_xml(clock)?;
        sign_targeting_before(
            &xml,
            &self.envio_id,
            "</LibroCompraVenta>",
            signer,
            cert,
            options,
        )
    }
}

fn write_caratula(out: &mut String, c: &CaratulaLibro) {
    let _ = write!(
        out,
        "<Caratula><RutEmisorLibro>{emisor}</RutEmisorLibro><RutEnvia>{envia}</RutEnvia><PeriodoTributario>{periodo}</PeriodoTributario><FchResol>{resol}</FchResol><NroResol>{nro}</NroResol><TipoOperacion>{operacion}</TipoOperacion><TipoLibro>{libro}</TipoLibro><TipoEnvio>{envio}</TipoEnvio>",
        emisor = c.rut_emisor_libro,
        envia = c.rut_envia,
        periodo = c.periodo_tributario,
        resol = format_date(&c.fch_resol),
        nro = c.nro_resol,
        operacion = c.tipo_operacion.as_str(),
        libro = c.tipo_libro.as_str(),
        envio = c.tipo_envio.as_str(),
    );
    if let Some(folio) = c.folio_notificacion {
        let _ = write!(out, "<FolioNotificacion>{folio}</FolioNotificacion>");
    }
    out.push_str("</Caratula>");
}

fn write_resumen_periodo(out: &mut String, totales: &[TotalesPeriodo]) {
    if totales.is_empty() {
        return;
    }
    out.push_str("<ResumenPeriodo>");
    for t in totales {
        let _ = write!(
            out,
            "<TotalesPeriodo><TpoDoc>{tpo}</TpoDoc><TotDoc>{tot_doc}</TotDoc>",
            tpo = t.tpo_doc.code(),
            tot_doc = t.tot_doc,
        );
        if let Some(anulado) = t.tot_anulado {
            if anulado > 0 {
                let _ = write!(out, "<TotAnulado>{anulado}</TotAnulado>");
            }
        }
        let _ = write!(
            out,
            "<TotMntExe>{exe}</TotMntExe><TotMntNeto>{neto}</TotMntNeto><TotMntIVA>{iva}</TotMntIVA><TotMntTotal>{total}</TotMntTotal></TotalesPeriodo>",
            exe = t.tot_mnt_exe.as_pesos(),
            neto = t.tot_mnt_neto.as_pesos(),
            iva = t.tot_mnt_iva.as_pesos(),
            total = t.tot_mnt_total.as_pesos(),
        );
    }
    out.push_str("</ResumenPeriodo>");
}

fn write_detalle(out: &mut String, d: &DetalleLibro) -> Result<(), Error> {
    let _ = write!(
        out,
        "<Detalle><TpoDoc>{tpo}</TpoDoc><NroDoc>{nro}</NroDoc>",
        tpo = d.tipo_doc.code(),
        nro = d.nro_doc,
    );
    if d.anulado {
        out.push_str("<Anulado>A</Anulado>");
    }
    if let Some(tasa) = d.tasa_imp {
        let _ = write!(out, "<TasaImp>{tasa}</TasaImp>");
    }
    if let Some(fch) = d.fch_doc {
        let _ = write!(out, "<FchDoc>{}</FchDoc>", format_date(&fch));
    }
    if let Some(ref rut) = d.rut_doc {
        let _ = write!(out, "<RUTDoc>{rut}</RUTDoc>");
    }
    if let Some(ref rzn) = d.rzn_soc {
        let escaped = quick_xml::escape::escape(rzn);
        let _ = write!(out, "<RznSoc>{escaped}</RznSoc>");
    }
    if let Some(exe) = d.mnt_exe {
        let _ = write!(out, "<MntExe>{}</MntExe>", exe.as_pesos());
    }
    if let Some(neto) = d.mnt_neto {
        let _ = write!(out, "<MntNeto>{}</MntNeto>", neto.as_pesos());
    }
    if let Some(iva) = d.mnt_iva {
        let _ = write!(out, "<MntIVA>{}</MntIVA>", iva.as_pesos());
    }
    if let Some(total) = d.mnt_total {
        let _ = write!(out, "<MntTotal>{}</MntTotal>", total.as_pesos());
    } else if d.mnt_neto.is_some() || d.mnt_exe.is_some() || d.mnt_iva.is_some() {
        let total = d.mnt_total_efectivo()?;
        let _ = write!(out, "<MntTotal>{}</MntTotal>", total.as_pesos());
    }
    out.push_str("</Detalle>");
    Ok(())
}
