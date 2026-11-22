# sii-sim

[![Crates.io](https://img.shields.io/crates/v/sii-sim.svg)](https://crates.io/crates/sii-sim)
[![Documentation](https://docs.rs/sii-sim/badge.svg)](https://docs.rs/sii-sim)
[![License](https://img.shields.io/crates/l/sii-sim.svg)](https://github.com/Demiannnnnn/sii-sdk#license)

**Unofficial local simulator and mock server for Chilean SII DTE endpoints and Web Services.**

> [!IMPORTANT]
> This simulator is an independent development and testing tool. It is **not** affiliated with or maintained by the Chilean Internal Revenue Service (*Servicio de Impuestos Internos*, SII).

Part of the [sii-sdk](https://github.com/Demiannnnnn/sii-sdk) workspace.

---

## Overview

Developing electronic invoicing software for Chile normally requires valid digital certificates, authorized CAF folio files, and access to the SII certification environment (`maullin`). `sii-sim` provides a zero-setup, deterministic simulation of the SII's communication lifecycle:

1. **In-Memory Transport (`SimTransport`):** Fast, zero-network unit and integration testing directly embedded in Rust test suites.
2. **Local HTTP Server (`HttpServer`):** A standalone TCP HTTP server bound strictly to loopback `127.0.0.1` (QA-01) for testing external applications, CLIs, or services without network dependencies.
3. **Deterministic Scenarios:** Test happy paths, transient failures, token expiration, and rejection responses reliably.

---

## Simulated Endpoints

| Service / Endpoint | Method | Description |
|:---|:---:|:---|
| `/DTEWS/CrSeed.jws` | `GET` | Returns an XML seed document (`CrSeedReturn`) with `ESTADO 00` or error. |
| `/DTEWS/GetTokenFromSeed.jws` | `POST` | Validates signed seed XML and returns session `TOKEN`. |
| `/cgi_dte/UPL/DTEUpload` | `POST` | Receives multipart envelope upload and returns `RECEPCIONDTE` with a Track ID. |
| `/DTEWS/QueryEstUp.jws` | `POST` | Responds to SOAP `getEstUp` queries with processing status XML. |

---

## Scenarios

- `aceptado` (default): Successful flow; returns `EstadoEnvio::Aceptado`.
- `rechazado`: Envelope rejected by SII rules; returns `EstadoEnvio::Rechazado` (ADR-04).
- `en_proceso`: Intermediate processing state (`EstadoEnvio::EnProceso`).
- `token_invalido`: Token exchange failure (`ESTADO 11`).
- `upload_rechazado`: Immediate upload failure (`STATUS != 0`).
- `semilla_error`: Seed acquisition error (`ESTADO 01`).
- Custom JSON scenario: Configured via `--scenario-file`.

---

## Usage

### As a Library in Rust Tests

```rust
use std::sync::Arc;
use sii::SiiClient;
use sii::codes::Ambiente;
use sii_sim::{Scenario, SiiSimulator, SimTransport};

#[test]
fn test_flow_against_simulator() {
    // 1. Create a simulator with a deterministic scenario
    let simulator = Arc::new(SiiSimulator::new().with_scenario(Scenario::Aceptado));

    // 2. Wrap in an in-memory transport
    let transport = SimTransport::direct(simulator);

    // 3. Connect client without any network sockets
    let client = SiiClient::new(Ambiente::Certificacion, transport);

    let seed = client.obtener_semilla().unwrap();
    assert!(!seed.as_str().is_empty());
}
```

### Running the CLI Mock Server

```bash
# Run simulator listening on 127.0.0.1:8080
cargo run -p sii-sim -- --listen 127.0.0.1:8080 --scenario aceptado

# Run with a custom scenario file
cargo run -p sii-sim -- --listen 127.0.0.1:8080 --scenario-file scenarios.json
```

---

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/Demiannnnnn/sii-sdk/blob/main/LICENSE-APACHE))
- MIT license ([LICENSE-MIT](https://github.com/Demiannnnnn/sii-sdk/blob/main/LICENSE-MIT))

at your option.
