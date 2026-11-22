# sii-cli

[![Crates.io](https://img.shields.io/crates/v/sii-cli.svg)](https://crates.io/crates/sii-cli)
[![License](https://img.shields.io/crates/l/sii-cli.svg)](https://github.com/Demiannnnnn/sii-sdk#license)

**Unofficial Command-Line Interface (`sii`) for Chilean Electronic Tax Documents (DTE).**

> [!IMPORTANT]
> This CLI is an independent open-source tool. It is **not** officially affiliated with or maintained by the Chilean Internal Revenue Service (*Servicio de Impuestos Internos*, SII).

Part of the [sii-sdk](https://github.com/Demiannnnnn/sii-sdk) workspace.

---

## Installation

```bash
cargo install sii-cli
```

This installs the `sii` binary into your Cargo bin path.

---

## Capabilities & Commands

The `sii` CLI provides subcommands for every stage of Chilean electronic invoicing:

### 1. Document Operations (`sii dte`)

- `sii dte build`: Construct validated DTE XML (Factura Afecta 33, Factura Exenta 34, Boleta 39/41, Guía 52, Nota Débito 56, Nota Crédito 61).
- `sii dte timbrar`: Stamp DTE with an authorized CAF file, generating the canonical `<TED>` electronic stamp.
- `sii dte firmar`: Sign DTE with XMLDSig using an X.509 digital certificate (`.p12`/`.pfx` or PEM).

### 2. Envelope & Transmission Operations (`sii envio`)

- `sii envio enviar`: Assemble signed DTE documents into an envelope (`EnvioDTE` or `EnvioBOLETA`), sign the envelope, authenticate with the SII via seed/token, and upload.
- `sii envio estado`: Query processing status using a Track ID.

### 3. Electronic Ledgers (`sii libro`)

- `sii libro build`: Generate and sign monthly purchase and sales ledgers (IECV - *Información Electrónica de Libros de Compra y Venta*).

---

## Output Modes

- **Human-Friendly (default):** Clear terminal output in Chilean Spanish.
- **Machine-Readable (`--json`):** Formatted JSON output with stable English property keys, suitable for scripting, automation pipelines, and ERP integration.

---

## Security (SEC-03)

In accordance with strict security practices, private key passwords should never be passed as command-line arguments. Instead, set the `SII_CERT_PASSWORD` environment variable:

```bash
export SII_CERT_PASSWORD="mi_clave_secreta"
sii dte firmar --cert emisor.p12 --in dte_timbrado.xml --out dte_firmado.xml
```

---

## Examples

### Building a Factura Electrónica

```bash
sii dte build \
  --tipo 33 \
  --folio 120 \
  --emisor-rut 76123456-7 \
  --emisor-rzn "Empresa Emisora SpA" \
  --emisor-giro "Servicios Informáticos" \
  --emisor-acteco 620100 \
  --emisor-dir "Av. Providencia 1234" \
  --emisor-cmna "Providencia" \
  --receptor-rut 55555555-5 \
  --receptor-rzn "Cliente S.A." \
  --receptor-giro "Comercial" \
  --receptor-dir "Alameda 100" \
  --receptor-cmna "Santiago" \
  --detalle "1;Desarrollo Software;1;UN;100000;100000;0" \
  --out factura.xml
```

### Stamping with CAF

```bash
sii dte timbrar --caf caf_33.xml --in factura.xml --out factura_timbrada.xml
```

### Signing with Digital Certificate

```bash
export SII_CERT_PASSWORD="clave"
sii dte firmar --cert cert.p12 --in factura_timbrada.xml --out factura_firmada.xml
```

### Uploading Envelope to SII or Local Simulator

```bash
sii envio enviar \
  --ambiente certificacion \
  --cert cert.p12 \
  --emisor-rut 76123456-7 \
  --envia-rut 12345678-5 \
  --fch-resol 2026-01-15 \
  --nro-resol 0 \
  --dte factura_firmada.xml \
  --sim-url http://127.0.0.1:8080
```

---

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/Demiannnnnn/sii-sdk/blob/main/LICENSE-APACHE))
- MIT license ([LICENSE-MIT](https://github.com/Demiannnnnn/sii-sdk/blob/main/LICENSE-MIT))

at your option.
