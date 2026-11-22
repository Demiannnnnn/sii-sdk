# Estado de implementación — sii-sdk

Qué existe en el código hoy vs. lo que el SDK debe ser. Lo mantiene el orquestador al integrar cada tarea.

> **2026-11-21 — Repositorio recién creado.** Solo existe el andamiaje de diseño (`AGENTS.md`, `.ai-context/`,
> `docs/dte-spec.md`, `tasks/`, licencias, lints, CI). **Aún no hay código del producto** (`crates/` vacío):
> lo crea INFRA-01 y lo llenan las tareas siguientes. Todo lo de `public-api.md` es objetivo, no realidad.

## Mapa tarea → módulo

| Módulo / artefacto | Tarea que lo crea | Estado |
|:--|:--|:--|
| Workspace Cargo, lints, licencias por crate, CI, skeleton de crates | INFRA-01 | ⬜ |
| `types`, `codes` (`Rut`, `Monto`, `TipoDte`, `Ambiente`, `Error`) | CORE-01 | ⬜ |
| `caf` (parse, rango, llave, FRMA) | CAF-01 | ⬜ |
| `dte` modelo + XML por tipo (33/34/39/41/52/56/61) | DTE-01, DTE-02 | ⬜ |
| `ted` (DD, FRMT RSA-SHA1, PDF417 data) | TED-01 | ⬜ |
| `signature` (XMLDSig, Signer/Certificate, C14N) | SIG-01 | ⬜ |
| `envio` (`EnvioDTE`/`EnvioBOLETA`, carátula) | ENVIO-01 | ⬜ |
| `libro` (compra/venta IECV) | LIBRO-01 | ⬜ |
| `auth` (semilla/token) | AUTH-01 | ⬜ |
| `transport` + `client` + `HttpTransport` (feature `http`) | TRANSPORT-01 | ⬜ |
| `sii-sim` (simulador del SII) | SIM-01 | ⬜ |
| `sii-cli` (bin `sii`) | CLI-01 | ⬜ |
| Carga PKCS#12/PEM del certificado (feature `pkcs12`) | CERT-LOADER-01 | ⬜ |
| Docs públicas + preparación de release | DOCS-01 | ⬜ |
| Certificación contra el SII real (manual, dueño) | CERT-01 | ⬜ (dueño) |

## Brechas conocidas

- Ninguna aún; el repo está vacío de código. La primera brecha real se registra cuando INFRA-01 se integre.

## Decisiones de alcance pendientes del dueño

- Tipos de DTE adicionales (46 factura de compra, exportación 110/111/112): fuera del alcance inicial.
- Descarga automática de CAF desde el SII: fuera del alcance inicial (el host entrega el CAF).
