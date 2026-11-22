# Estado de implementación — sii-sdk

Qué existe en el código hoy vs. lo que el SDK debe ser. Lo mantiene el orquestador al integrar cada tarea.

> **2026-11-22 — 11 de 15 tareas de código integradas en `develop-AGENTS`, 128 tests en verde.**
> Falta la cadena final: SIM-01 → CLI-01 → DOCS-01. CERT-99 es manual del dueño.

## Mapa tarea → módulo

| Módulo / artefacto | Tarea | Estado | Notas |
|:--|:--|:--|:--|
| Workspace, lints, CI, 3 crates | INFRA-01 | ✅ | |
| `types`, `codes` (`Rut` DV mód. 11, `Monto` i64, `TipoDte`, `Ambiente` + URLs por ambiente) | CORE-01 | ✅ | |
| `caf` (parse, rango, llave RSA, FRMA) | CAF-01 | ✅ | fixture CAF ficticio |
| `dte` 33/34 | DTE-01 | ✅ | XML byte a byte vs fixture |
| `dte` 39/41/52/56/61 | DTE-02 | ✅ | notas exigen `Referencia` |
| `ted` (DD, FRMT RSA-SHA1, datos PDF417) | TED-01 | ✅ | |
| `signature` (XMLDSig, C14N, `sign`, `sign_dte`, `sign_enveloped`, `sign_targeting_before`, `verify`, `verify_by_reference`) | SIG-01 (+ENVIO-01) | ✅ | |
| `auth` (semilla/token) | AUTH-01 | ✅ | |
| `envio` (`EnvioDte`/`EnvioBoleta`, carátula, `SubTotDTE`, firma sobre `SetDoc`) | ENVIO-01 | ✅ | implementada por Claude (respaldo) |
| `cert_loader` (`SiiIdentity` PEM + PKCS#12, feature `pkcs12`, crate `p12`) | CERT-LOADER-01 | ✅ | implementada por Claude (respaldo) |
| `libro` (IECV) | LIBRO-01 | ✅ | |
| `transport` + `client` (`SiiClient`, `MemoryTransport`, `HttpTransport` feature `http` con `ureq`) | TRANSPORT-01 | ✅ | `EstadoEnvio` como valor (ADR-04) |
| `sii-sim` (simulador) | SIM-01 | ⬜ | crate solo con esqueleto |
| `sii-cli` (bin `sii`) | CLI-01 | ⬜ | crate solo con esqueleto |
| README por crate, `docs/md/release.md`, CHANGELOG | DOCS-01 | ⬜ | |
| Certificación real contra maullin | CERT-99 | ⬜ (dueño) | |

## Contrato del simulador (insumo para SIM-01)

El simulador implementa `sii::Transport` y responde por sufijo de URL (válido para ambos ambientes):

| Endpoint | Método | Respuesta que el cliente sabe parsear |
|:--|:--|:--|
| `…/DTEWS/CrSeed.jws` | GET | `SII:RESPUESTA` con `ESTADO 00` y `SEMILLA` (ver `tests/fixtures/semilla-resp-ok.xml`) |
| `…/DTEWS/GetTokenFromSeed.jws` | POST (`<getToken><item><Semilla>` firmado) | `SII:RESPUESTA` con `TOKEN` o `ESTADO 11` (fixtures `token-resp-*`) |
| `…/cgi_dte/UPL/DTEUpload` | POST multipart (campo `archivo`, cookie `TOKEN=`) | `RECEPCIONDTE` con `STATUS 0` + `TRACKID`, o `STATUS≠0` + `DETAIL/ERROR` |
| `…/DTEWS/QueryEstUp.jws` | POST SOAP `getEstUp` (`TrackId`, `Token`) | SOAP con `getEstUpReturn` escapado que contiene `ESTADO` (`EPR`+`ACEPTADOS`/`RECHAZADOS`, `REC`…), clasificado por `client::classify_estado` |

## Brechas conocidas

- `cargo doc` avisa colisión de nombre entre la lib `sii` y el bin `sii` (bug conocido de cargo); resolver en DOCS-01.
- El cliente usa URLs fijas por ambiente; CLI-01 necesita poder apuntar al simulador local (override de URL base o transporte del simulador).

## Decisiones de alcance pendientes del dueño

- Tipos de DTE adicionales (46, 110/111/112) y descarga automática de CAF: fuera del alcance inicial.
