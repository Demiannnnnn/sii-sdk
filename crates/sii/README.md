# sii

[![Crates.io](https://img.shields.io/crates/v/sii.svg)](https://crates.io/crates/sii)
[![Documentation](https://docs.rs/sii/badge.svg)](https://docs.rs/sii)
[![License](https://img.shields.io/crates/l/sii.svg)](https://github.com/Demiannnnnn/sii-sdk#license)

**Unofficial Rust SDK for Chilean Electronic Tax Documents (DTE - Documentos Tributarios Electrónicos).**

> [!IMPORTANT]
> This SDK is an independent, open-source project and is **not** officially affiliated with, endorsed by, or sponsored by the Chilean Internal Revenue Service (*Servicio de Impuestos Internos*, SII).

Part of the [sii-sdk](https://github.com/Demiannnnnn/sii-sdk) workspace.

---

## Features

- **Sans-IO Architecture (ADR-01):** The core library does zero direct network I/O, filesystem access, or system clock calls. Deterministic, portable, and easily testable.
- **Exact SII XML Conformity (ADR-03):** Verified against official Chilean SII schemas (`DTE_v10.xsd`, `LibroCV_v10.xsd`, `EnvioDTE_v10.xsd`, `EnvioBOLETA_v11.xsd`).
- **Complete Document Coverage:**
  - Factura Electrónica (afecta a IVA 19 %) — Tipo 33
  - Factura No Afecta o Exenta Electrónica — Tipo 34
  - Boleta Electrónica — Tipo 39
  - Boleta Exenta Electrónica — Tipo 41
  - Guía de Despacho Electrónica — Tipo 52
  - Nota de Débito Electrónica — Tipo 56
  - Nota de Crédito Electrónica — Tipo 61 (with mandatory reference validation)
- **Financial Accuracy (DTE-03):** All monetary amounts use integer Chilean Pesos (`Monto`) without floating-point math hazards.
- **Folio Authorization (CAF):** Parse authorization files, validate folio ranges, extract RSA authorization keys, and verify the SII's `FRMA` digital signature.
- **Electronic Tax Stamp (TED):** Canonical `<DD>` construction and RSA-SHA1 digital signing (`<FRMT>`) conforming strictly to SII requirements (TED-01), with serialized PDF417 payload extraction (ADR-06).
- **XMLDSig (W3C Canonical XML):** Enveloped digital signing and verification using `Signer` and `Certificate` traits (ADR-02).
- **Envelopes (`EnvioDTE` & `EnvioBOLETA`):** Multi-document batching with automatic `Caratula` cover metadata and summary totals (`SubTotDTE`).
- **Purchase & Sales Ledgers (IECV):** Monthly `LibroCompraVenta` building, aggregation, and signing.
- **SII Web Services Client (`SiiClient`):** Complete communication lifecycle: seed acquisition (`CrSeed`), token retrieval (`GetTokenFromSeed`), multipart envelope upload (`DTEUpload`), and processing status queries (`QueryEstUp`). Rejections are handled as domain values, not errors (`EstadoEnvio::Rechazado`, ADR-04).
- **Strict Security (SEC-01..03):** Private keys are redacted in `Debug` and `Display` implementations and never exposed in errors or logs.

---

## Cargo Features

| Feature | Default | Description |
|:---|:---:|:---|
| *(core)* | Yes | Core sans-IO domain models, XML serialization, CAF parsing, TED, XMLDSig, envelopes, and ledgers. |
| `http` | No | Enables `HttpTransport` with `ureq` and TLS for network communication with SII or simulators. |
| `pkcs12` | No | Enables `cert_loader::SiiIdentity` for loading certificates from `.p12`/`.pfx` or PEM files. |
| `serde` | No | Derives `serde::Serialize` and `serde::Deserialize` for public domain models. |
| `tracing` | No | Emits operational trace events (without logging secret keys or sensitive payloads). |

---

## Quick Start

Add `sii` to your `Cargo.toml`:

```toml
[dependencies]
sii = { version = "0.1", features = ["http", "pkcs12"] }
```

### 1. Basic Example: Build and Stamp a Factura

```rust
use sii::codes::TipoDte;
use sii::dte::{Detalle, Emisor, FacturaAfectaBuilder, Receptor};
use sii::types::{ManualClock, Monto, Rut};
use time::macros::date;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Define taxpayer RUTs
    let emisor_rut = Rut::new(76_123_456, '7')?;
    let receptor_rut = Rut::new(55_555_555, '5')?;

    // 2. Build a Factura Electrónica (Tipo 33)
    let dte = FacturaAfectaBuilder::new()
        .folio(1)
        .fch_emis(date!(2026-11-22))
        .emisor(Emisor {
            rut_emisor: emisor_rut,
            rzn_soc: "Empresa Emisora SpA".into(),
            giro_emis: "Servicios Informáticos".into(),
            acteco: vec![620100],
            cdg_sii_sucur: None,
            dir_origen: "Av. Providencia 1234".into(),
            cmna_origen: "Providencia".into(),
            ciudad_origen: Some("Santiago".into()),
        })
        .receptor(Receptor::minimo(receptor_rut, "Cliente S.A."))
        .detalle(
            Detalle::new(1, "Desarrollo de Software", Monto::from_pesos(100_000))
                .qty(1, "UN")
                .precio(Monto::from_pesos(100_000)),
        )
        .build()?;

    // Inspect totals: $100,000 net + $19,000 IVA = $119,000 total
    assert_eq!(dte.totales().mnt_total, Monto::from_pesos(119_000));
    assert_eq!(dte.totales().iva, Some(Monto::from_pesos(19_000)));

    // 3. Serialize to canonical SII XML
    let xml = dte.to_xml()?;
    assert!(xml.starts_with(b"<DTE"));

    Ok(())
}
```

### 2. Client Example: Query Status

```rust
use sii::client::SiiClient;
use sii::codes::Ambiente;
use sii::transport::MemoryTransport;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport = MemoryTransport::new();
    let client = SiiClient::new(Ambiente::Certificacion, transport);

    // High-level client manages authentication tokens and status queries
    assert_eq!(client.ambiente(), Ambiente::Certificacion);

    Ok(())
}
```

---

## Sibling Crates

- [`sii-sim`](https://crates.io/crates/sii-sim): In-memory and local HTTP simulator for SII Web Services and upload endpoints.
- [`sii-cli`](https://crates.io/crates/sii-cli): Command-line utility (`sii`) for building, stamping, signing, uploading, and querying DTE documents.

---

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/Demiannnnnn/sii-sdk/blob/main/LICENSE-APACHE))
- MIT license ([LICENSE-MIT](https://github.com/Demiannnnnn/sii-sdk/blob/main/LICENSE-MIT))

at your option.
