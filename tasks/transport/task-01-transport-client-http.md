# TASK-TRANSPORT-01: Transport, SiiClient y HttpTransport

| Atributo | Detalle |
|:--|:--|
| **ID** | `TRANSPORT-01` |
| **Estado** | ⬜ pendiente |
| **Depende de** | ENVIO-01, AUTH-01 |
| **Rama** | `dev/transport-01-client` |
| **Prioridad** | P1 |

## 1. Contexto
`architecture.md` §1/§3, `docs/dte-spec.md` §7, ADR-04. La red vive detrás de `trait Transport`; el núcleo sigue sans-IO.
## 2. Requerimientos
- **RF-01 Transport:** `trait Transport` (enviar request → response, sans-IO del lado del núcleo) + `MemoryTransport` para tests.
- **RF-02 SiiClient:** `SiiClient::<T: Transport>::new(ambiente, transport)` con `obtener_token(&dyn Signer)`, `enviar(&EnvioDte, &Token) -> Result<TrackId, Error>`, `estado(TrackId, &Token) -> Result<EstadoEnvio, Error>`. `EstadoEnvio` = `Aceptado`/`Rechazado`/`EnProceso` + detalle; **rechazo es valor, no `Err`** (ADR-04).
- **RF-03 HttpTransport (feature `http`):** cliente TLS (rustls) con los endpoints por ambiente documentados aquí (certificación/producción). Fuera de los tests. Decide y fija en `architecture.md` §3 el cliente HTTP (`ureq` o `reqwest`-rustls) y repórtalo.
- **RF-04 Endpoints:** documenta en este archivo (o en `docs/`) las URLs por ambiente; nunca default a producción.
## 3. Tests
Flujo completo semilla→token→envío→estado contra `MemoryTransport`; rechazo del SII como `EstadoEnvio::Rechazado`; sin red real (QA-01).
## 4. DoD
AGENTS.md §8, incluido `cargo clippy -p sii --no-default-features`.
## Reporte
_(lo completa el agente)_
