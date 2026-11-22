//! Integration tests for envelopes (ENVIO-01): `EnvioDTE` / `EnvioBOLETA`.

use rsa::RsaPrivateKey;
use rsa::RsaPublicKey;
use rsa::pkcs1v15::SigningKey;
use rsa::rand_core::OsRng;
use rsa::signature::{SignatureEncoding, Signer as _};
use sha1::Sha1;
use sii::codes::TipoDte;
use sii::dte::{Detalle, Dte, Emisor, Receptor};
use sii::signature::{
    Certificate, SignatureOptions, Signer, X509Certificate, sign_dte, verify_by_reference,
};
use sii::types::{ManualClock, Monto, Rut};
use sii::{EnvioBoleta, EnvioDte, RUT_RECEPTOR_SII};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

struct TestSigner {
    private_key: RsaPrivateKey,
}

impl Signer for TestSigner {
    fn sign(&self, data: &[u8]) -> sii::Result<Vec<u8>> {
        let key = SigningKey::<Sha1>::new(self.private_key.clone());
        Ok(key.sign(data).to_vec())
    }
}

fn test_signer_and_cert() -> Result<(TestSigner, X509Certificate), Box<dyn std::error::Error>> {
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

fn emisor() -> Result<Emisor, Box<dyn std::error::Error>> {
    Ok(Emisor::new(
        Rut::parse("76.086.428-5")?,
        "DISTRIBUIDORA EJEMPLO SPA",
        "VENTA AL POR MAYOR",
        465_900,
        "AVENIDA LIBERTADOR 1234",
        "SANTIAGO",
    )
    .ciudad_origen("SANTIAGO"))
}

fn receptor() -> Result<Receptor, Box<dyn std::error::Error>> {
    Ok(Receptor::new(
        Rut::parse("77.777.777-7")?,
        "CLIENTE DE PRUEBAS SA",
        "SERVICIOS",
        "CALLE COMERCIAL 567",
        "PROVIDENCIA",
    ))
}

fn factura_afecta(folio: u64) -> Result<Dte, Box<dyn std::error::Error>> {
    Ok(Dte::factura_afecta()
        .folio(folio)
        .fch_emis_str("2026-10-21")?
        .emisor(emisor()?)
        .receptor(receptor()?)
        .item(
            Detalle::new(1, "Servidor de aplicaciones", Monto::from_pesos(100_000))
                .qty(1)
                .prc(Monto::from_pesos(100_000)),
        )
        .build()?)
}

fn boleta(folio: u64) -> Result<Dte, Box<dyn std::error::Error>> {
    Ok(Dte::boleta()
        .folio(folio)
        .fch_emis_str("2026-10-21")?
        .emisor(emisor()?)
        .receptor(receptor()?)
        .item(
            Detalle::new(1, "Producto de prueba", Monto::from_pesos(5_000))
                .qty(1)
                .prc(Monto::from_pesos(5_000)),
        )
        .build()?)
}

fn clock() -> Result<ManualClock, Box<dyn std::error::Error>> {
    let date = time::Date::from_calendar_date(2026, time::Month::October, 21)?;
    let t = time::Time::from_hms(12, 30, 0)?;
    Ok(ManualClock::new(date.with_time(t).assume_utc()))
}

fn resol_date() -> Result<time::Date, Box<dyn std::error::Error>> {
    Ok(time::Date::from_calendar_date(
        2014,
        time::Month::August,
        22,
    )?)
}

fn signed(
    dte: &Dte,
    signer: &dyn Signer,
    cert: &dyn Certificate,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let xml = dte.to_xml()?;
    Ok(sign_dte(&xml, signer, cert)?)
}

#[test]
fn test_envio_dte_signed_verifies_by_reference() -> TestResult {
    let (signer, cert) = test_signer_and_cert()?;
    let d1 = signed(&factura_afecta(60)?, &signer, &cert)?;
    let d2 = signed(&factura_afecta(61)?, &signer, &cert)?;

    let rut_e = Rut::parse("76.086.428-5")?;
    let rut_v = Rut::parse("11.111.111-1")?;
    let mut envio = EnvioDte::nuevo(rut_e, rut_v, resol_date()?, 0);
    envio.agregar(TipoDte::FacturaAfecta, &d1)?;
    envio.agregar(TipoDte::FacturaAfecta, &d2)?;
    assert_eq!(envio.len(), 2);

    let signed_env = envio.firmar(&clock()?, &signer, &cert, SignatureOptions::default())?;

    // The envelope signature targets SetDoc and must verify against it.
    assert!(
        verify_by_reference(&signed_env, "SetDoc")?,
        "envelope signature over SetDTE must verify"
    );

    let text = String::from_utf8(signed_env)?;
    assert!(text.contains(&format!("<RutReceptor>{RUT_RECEPTOR_SII}</RutReceptor>")));
    // Two type-33 documents summarized together.
    assert!(text.contains("<SubTotDTE><TpoDTE>33</TpoDTE><NroDTE>2</NroDTE></SubTotDTE>"));
    assert!(text.contains("ID=\"SetDoc\""));
    assert!(text.trim_end().ends_with("</EnvioDTE>"));
    Ok(())
}

#[test]
fn test_envelope_signature_tamper_is_detected() -> TestResult {
    let (signer, cert) = test_signer_and_cert()?;
    let d1 = signed(&factura_afecta(60)?, &signer, &cert)?;
    let rut_e = Rut::parse("76.086.428-5")?;
    let mut envio = EnvioDte::nuevo(rut_e, rut_e, resol_date()?, 0);
    envio.agregar(TipoDte::FacturaAfecta, &d1)?;
    let signed_env = envio.firmar(&clock()?, &signer, &cert, SignatureOptions::default())?;

    // Tamper with the Caratula resolution number inside the signed SetDTE.
    let text = String::from_utf8(signed_env)?;
    let tampered = text.replace("<NroResol>0</NroResol>", "<NroResol>999</NroResol>");
    assert_ne!(text, tampered, "tamper must change the document");
    assert!(
        !verify_by_reference(tampered.as_bytes(), "SetDoc")?,
        "tampered envelope must fail verification"
    );
    Ok(())
}

#[test]
fn test_envio_boleta_groups_boletas() -> TestResult {
    let (signer, cert) = test_signer_and_cert()?;
    let b1 = signed(&boleta(5)?, &signer, &cert)?;
    let rut_e = Rut::parse("76.086.428-5")?;
    let mut envio = EnvioBoleta::nuevo(rut_e, rut_e, resol_date()?, 0);
    envio.agregar(TipoDte::Boleta, &b1)?;
    let signed_env = envio.firmar(&clock()?, &signer, &cert, SignatureOptions::default())?;
    assert!(verify_by_reference(&signed_env, "SetDoc")?);
    let text = String::from_utf8(signed_env)?;
    assert!(text.contains("<EnvioBOLETA"));
    assert!(text.contains("<SubTotDTE><TpoDTE>39</TpoDTE><NroDTE>1</NroDTE></SubTotDTE>"));
    assert!(text.trim_end().ends_with("</EnvioBOLETA>"));
    Ok(())
}

#[test]
fn test_boleta_rejected_in_envio_dte() -> TestResult {
    let (signer, cert) = test_signer_and_cert()?;
    let b1 = signed(&boleta(5)?, &signer, &cert)?;
    let rut_e = Rut::parse("76.086.428-5")?;
    let mut envio = EnvioDte::nuevo(rut_e, rut_e, resol_date()?, 0);
    let err = envio
        .agregar(TipoDte::Boleta, &b1)
        .expect_err("boleta must be rejected in EnvioDTE");
    assert!(format!("{err}").contains("EnvioBOLETA"));
    Ok(())
}

#[test]
fn test_factura_rejected_in_envio_boleta() -> TestResult {
    let (signer, cert) = test_signer_and_cert()?;
    let d1 = signed(&factura_afecta(60)?, &signer, &cert)?;
    let rut_e = Rut::parse("76.086.428-5")?;
    let mut envio = EnvioBoleta::nuevo(rut_e, rut_e, resol_date()?, 0);
    let err = envio
        .agregar(TipoDte::FacturaAfecta, &d1)
        .expect_err("factura must be rejected in EnvioBOLETA");
    assert!(format!("{err}").contains("EnvioBOLETA"));
    Ok(())
}

#[test]
fn test_empty_envelope_fails_to_serialize() -> TestResult {
    let rut_e = Rut::parse("76.086.428-5")?;
    let envio = EnvioDte::nuevo(rut_e, rut_e, resol_date()?, 0);
    assert!(envio.is_empty());
    assert!(envio.to_xml(&clock()?).is_err());
    Ok(())
}
