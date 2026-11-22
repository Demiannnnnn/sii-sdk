# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-11-22

### Added

#### `sii` (Core Library)
- **Sans-IO Architecture (ADR-01):** Core domain models, XML serialization, cryptographic signatures, envelopes, and ledgers compile with `--no-default-features` without direct network or system clock dependencies.
- **DTE Document Models (DTE-01, DTE-02):**
  - Factura Electrónica (afecta a IVA 19 %) — Tipo 33
  - Factura No Afecta o Exenta Electrónica — Tipo 34
  - Boleta Electrónica — Tipo 39
  - Boleta Exenta Electrónica — Tipo 41
  - Guía de Despacho Electrónica — Tipo 52
  - Nota de Débito Electrónica — Tipo 56
  - Nota de Crédito Electrónica — Tipo 61 (with mandatory reference validation)
- **Domain Primitives (CORE-01):**
  - `Rut`: Validated Chilean taxpayer identifier with modulo 11 check digit verification.
  - `Monto`: Safe integer arithmetic in Chilean Pesos (CLP) without floating-point inaccuracies (DTE-03).
  - `TipoDte` & `Ambiente`: Strongly typed domain enumerations with SII environment URLs (`Certificacion` and `Produccion`).
- **Folio Authorization (CAF-01):** Parser for Chilean SII CAF XML authorizations with folio range validation, RSA private key extraction, and SII `FRMA` signature verification.
- **Electronic Tax Stamp (TED-01, TED-02):** Canonical `<DD>` construction and RSA-SHA1 digital signing (`<FRMT>`) using CAF authorization keys, with PDF417 raw payload data extraction (ADR-06).
- **XMLDSig Digital Signatures (SIG-01..03):** W3C enveloped XML Digital Signature implementation with Canonical XML 1.0 (C14N) normalization, supporting `Signer` and `Certificate` traits (ADR-02).
- **Envelopes (ENVIO-01):** Batch envelope building for `EnvioDTE` and `EnvioBOLETA` with automatic `Caratula` cover metadata and `SubTotDTE` summary totals.
- **Electronic Ledgers (LIBRO-01):** Purchase and sales ledgers (IECV - *Información Electrónica de Libros de Compra y Venta*) with period totals aggregation and XMLDSig envelope signing.
- **SII Web Services Client (AUTH-01, TRANSPORT-01):** Complete communication lifecycle with seed acquisition (`CrSeed`), token exchange (`GetTokenFromSeed`), multipart envelope upload (`DTEUpload`), and status polling (`QueryEstUp`). Rejections are treated as domain values (`EstadoEnvio::Rechazado`, ADR-04).
- **Transports (ADR-01):** `MemoryTransport` for in-memory testing and `HttpTransport` (feature `http`) using `ureq` with TLS.
- **Certificate Loading (CERT-LOADER-01):** Optional `cert_loader::SiiIdentity` (feature `pkcs12`) for loading identity credentials from PKCS#12 (`.p12`/`.pfx`) or PEM files.
- **Security Protections (SEC-01..03):** Redacted `Debug` and `Display` representations for private key containers, zero secrets versioned in repository, and forbidden `unsafe` code.

#### `sii-sim` (Simulator)
- **Sans-IO In-Memory Simulator (SIM-01):** Local simulation core implementing `sii::transport::Transport` via `SimTransport` for zero-IO testing.
- **Loopback HTTP Server (QA-01):** Standalone HTTP server binding exclusively to loopback `127.0.0.1` mimicking official SII endpoints.
- **Deterministic Scenarios:** Pre-configured scenarios (`aceptado`, `rechazado`, `en_proceso`, `token_invalido`, `upload_rechazado`, `semilla_error`) and custom JSON scenario support.
- **CLI Binary (`sii-sim`):** Standalone executable for running mock server instances locally.

#### `sii-cli` (Command-Line Tool)
- **Command-Line Interface (`sii`, CLI-01):**
  - `sii dte build`: Construct validated DTE XML documents.
  - `sii dte timbrar`: Stamp documents with CAF authorization files.
  - `sii dte firmar`: Digitally sign documents with digital certificates.
  - `sii envio enviar`: Batch documents into envelopes, sign, and upload to SII or simulator.
  - `sii envio estado`: Query processing status by Track ID.
  - `sii libro build`: Build and sign monthly IECV purchase/sales ledgers.
- **Output Formats:** Human-friendly Chilean Spanish terminal output and machine-readable `--json` output with stable English property keys.
- **Secrets Management (SEC-03):** Certificate passwords read securely from `SII_CERT_PASSWORD` environment variable.

[Unreleased]: https://github.com/Demiannnnnn/sii-sdk/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Demiannnnnn/sii-sdk/releases/tag/v0.1.0
