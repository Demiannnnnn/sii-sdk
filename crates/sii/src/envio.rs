//! Envelope structures (`EnvioDTE`, `EnvioBOLETA`) and batching (ENVIO-01).
//!
//! An envelope groups one or more already-signed DTEs from the same issuer, adds a
//! `Caratula` (cover page with the per-type summary) and carries its own enveloped
//! `XMLDSig` signature over the `SetDTE` element (RF-01..RF-04, `docs/dte-spec.md` §7).
//!
//! Boletas (types 39/41) travel in [`EnvioBoleta`]; every other DTE travels in
//! [`EnvioDte`]. Mixing a boleta into [`EnvioDte`] (or vice versa) is rejected.

use crate::codes::TipoDte;
use crate::dte::format_date;
use crate::error::Error;
use crate::signature::{Certificate, SignatureOptions, Signer, sign_targeting_before};
use crate::types::{Clock, Rut, format_timestamp};
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// Receptor RUT of the SII itself, used in every envelope `Caratula`.
pub const RUT_RECEPTOR_SII: &str = "60803000-K";

const SET_DTE_ID: &str = "SetDoc";

/// A single signed DTE held inside an envelope.
struct EnvelopeDoc {
    tipo: TipoDte,
    /// The signed `<DTE>...</DTE>` XML, without any XML declaration.
    xml: Vec<u8>,
}

/// Builder for an `EnvioDTE` envelope (every DTE type except boletas 39/41).
pub struct EnvioDte {
    rut_emisor: Rut,
    rut_envia: Rut,
    fch_resol: time::Date,
    nro_resol: u32,
    documentos: Vec<EnvelopeDoc>,
}

/// Builder for an `EnvioBOLETA` envelope (boletas, types 39 and 41).
pub struct EnvioBoleta {
    rut_emisor: Rut,
    rut_envia: Rut,
    fch_resol: time::Date,
    nro_resol: u32,
    documentos: Vec<EnvelopeDoc>,
}

/// Strips a leading `<?xml ... ?>` declaration and surrounding whitespace from a DTE.
fn strip_xml_declaration(xml: &[u8]) -> Result<Vec<u8>, Error> {
    let s = std::str::from_utf8(xml).map_err(|e| Error::Xml(e.to_string()))?;
    let trimmed = s.trim_start();
    let body = if trimmed.starts_with("<?xml") {
        let end = trimmed
            .find("?>")
            .ok_or_else(|| Error::Xml("malformed XML declaration in DTE".into()))?;
        trimmed
            .get(end.saturating_add(2)..)
            .ok_or_else(|| Error::Xml("internal error stripping XML declaration".into()))?
            .trim_start()
    } else {
        trimmed
    };
    if !body.starts_with("<DTE") {
        return Err(Error::DteValidation(
            "signed document does not start with <DTE>".into(),
        ));
    }
    Ok(body.as_bytes().to_vec())
}

/// Common envelope metadata used to build the `Caratula`.
struct EnvelopeMeta<'a> {
    rut_emisor: &'a Rut,
    rut_envia: &'a Rut,
    fch_resol: time::Date,
    nro_resol: u32,
    tmst_firma: &'a str,
}

/// Static identity of an envelope kind (root element and schema reference).
struct EnvelopeKind {
    root: &'static str,
    schema: &'static str,
}

/// Builds the per-type `<SubTotDTE>` summary ordered by ascending type code.
fn sub_tot_dte(out: &mut String, documentos: &[EnvelopeDoc]) {
    let mut counts: BTreeMap<u16, usize> = BTreeMap::new();
    for doc in documentos {
        *counts.entry(doc.tipo.code()).or_insert(0) += 1;
    }
    for (tpo, nro) in counts {
        // Writing to a String is infallible.
        let _ = write!(
            out,
            "<SubTotDTE><TpoDTE>{tpo}</TpoDTE><NroDTE>{nro}</NroDTE></SubTotDTE>"
        );
    }
}

/// Assembles the unsigned envelope XML for the given kind, metadata and documents.
fn build_envelope(
    kind: &EnvelopeKind,
    meta: &EnvelopeMeta<'_>,
    documentos: &[EnvelopeDoc],
) -> Result<Vec<u8>, Error> {
    if documentos.is_empty() {
        return Err(Error::DteValidation(
            "envelope must contain at least one DTE".into(),
        ));
    }
    let mut out = String::new();
    // All writes to a String are infallible.
    let _ = write!(
        out,
        "<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><{root} xmlns=\"http://www.sii.cl/SiiDte\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:schemaLocation=\"http://www.sii.cl/SiiDte {schema}\" version=\"1.0\"><SetDTE ID=\"{SET_DTE_ID}\"><Caratula version=\"1.0\"><RutEmisor>{emisor}</RutEmisor><RutEnvia>{envia}</RutEnvia><RutReceptor>{sii}</RutReceptor><FchResol>{resol}</FchResol><NroResol>{nro}</NroResol><TmstFirmaEnv>{tmst}</TmstFirmaEnv>",
        root = kind.root,
        schema = kind.schema,
        emisor = meta.rut_emisor,
        envia = meta.rut_envia,
        sii = RUT_RECEPTOR_SII,
        resol = format_date(&meta.fch_resol),
        nro = meta.nro_resol,
        tmst = meta.tmst_firma,
    );
    sub_tot_dte(&mut out, documentos);
    out.push_str("</Caratula>");
    let mut bytes = out.into_bytes();
    for doc in documentos {
        bytes.extend_from_slice(&doc.xml);
    }
    bytes.extend_from_slice(b"</SetDTE>");
    bytes.extend_from_slice(format!("</{}>", kind.root).as_bytes());
    Ok(bytes)
}

macro_rules! envelope_impl {
    ($ty:ident, $root:literal, $schema:literal, $accepts:ident, $reject_msg:literal) => {
        impl $ty {
            /// Creates a new envelope builder.
            ///
            /// `nro_resol` is `0` in the certification environment; `fch_resol` / `nro_resol`
            /// come from the issuer's SII resolution authorizing electronic documents.
            #[must_use]
            pub fn nuevo(
                rut_emisor: Rut,
                rut_envia: Rut,
                fch_resol: time::Date,
                nro_resol: u32,
            ) -> Self {
                Self {
                    rut_emisor,
                    rut_envia,
                    fch_resol,
                    nro_resol,
                    documentos: Vec::new(),
                }
            }

            /// Adds a signed DTE of the given `tipo` to the envelope.
            ///
            /// `signed_dte_xml` is the output of [`crate::sign_dte`] (a `<DTE>` with its
            /// enveloped `<Signature>`), with or without an XML declaration.
            ///
            /// # Errors
            ///
            /// Returns [`Error::DteValidation`] if `tipo` does not belong in this envelope
            /// kind or the XML does not start with `<DTE>`.
            pub fn agregar(&mut self, tipo: TipoDte, signed_dte_xml: &[u8]) -> Result<(), Error> {
                if !tipo.$accepts() {
                    return Err(Error::DteValidation($reject_msg.into()));
                }
                let xml = strip_xml_declaration(signed_dte_xml)?;
                self.documentos.push(EnvelopeDoc { tipo, xml });
                Ok(())
            }

            /// Returns the number of documents currently in the envelope.
            #[must_use]
            pub fn len(&self) -> usize {
                self.documentos.len()
            }

            /// Returns `true` if the envelope holds no documents yet.
            #[must_use]
            pub fn is_empty(&self) -> bool {
                self.documentos.is_empty()
            }

            /// Serializes the unsigned envelope XML, timestamping `TmstFirmaEnv` from `clock`.
            ///
            /// # Errors
            ///
            /// Returns [`Error`] if the envelope is empty or serialization fails.
            pub fn to_xml(&self, clock: &dyn Clock) -> Result<Vec<u8>, Error> {
                let tmst = format_timestamp(&clock.now());
                build_envelope(
                    &EnvelopeKind {
                        root: $root,
                        schema: $schema,
                    },
                    &EnvelopeMeta {
                        rut_emisor: &self.rut_emisor,
                        rut_envia: &self.rut_envia,
                        fch_resol: self.fch_resol,
                        nro_resol: self.nro_resol,
                        tmst_firma: &tmst,
                    },
                    &self.documentos,
                )
            }

            /// Builds and signs the envelope: the `SetDTE` element is signed with an
            /// enveloped `XMLDSig` signature inserted as a sibling under the root.
            ///
            /// # Errors
            ///
            /// Returns [`Error`] if the envelope is empty, serialization, or signing fails.
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
                    SET_DTE_ID,
                    concat!("</", $root, ">"),
                    signer,
                    cert,
                    options,
                )
            }
        }
    };
}

envelope_impl!(
    EnvioDte,
    "EnvioDTE",
    "EnvioDTE_v10.xsd",
    is_envio_dte,
    "boletas (39/41) must be sent in an EnvioBOLETA, not EnvioDTE"
);
envelope_impl!(
    EnvioBoleta,
    "EnvioBOLETA",
    "EnvioBOLETA_v11.xsd",
    is_envio_boleta,
    "only boletas (39/41) may be sent in an EnvioBOLETA"
);
