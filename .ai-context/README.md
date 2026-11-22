# AI Context — sii-sdk

Lee este directorio antes de tocar código. Es la fuente de verdad de **lo que el SDK debe ser**; el código es la fuente de verdad de **lo que existe hoy**. Si difieren, `implementation-status.md` dice qué brechas se conocen; cualquier otra diferencia es un bug en el código o en estos documentos, y debes reportarla.

Estos archivos son **de solo lectura para los agentes** (AGENTS.md §5). Si algo está mal o falta, dilo en el reporte de la tarea; el orquestador los actualiza.

## Orden de lectura

| # | Archivo | Léelo cuando |
|:--|:--|:--|
| 1 | [`domain.md`](domain.md) | Siempre. Lenguaje ubicuo del SII, tipos de DTE, folios/CAF, timbre, envío, alcance (qué NO es). |
| 2 | [`architecture.md`](architecture.md) | Siempre. Workspace, capas, stack y versiones, dependencias permitidas, decisiones (ADR). |
| 3 | [`constraints.md`](constraints.md) | Siempre. Reglas duras con ID (`DTE-*`, `SIG-*`, `CAF-*`, `API-*`, `SAFE-*`, `SEC-*`, `LEG-*`, `QA-*`). |
| 4 | [`conventions.md`](conventions.md) | Siempre. Idioma, **reglas de ramas**, estilo Rust, errores, tests, formato del reporte. |
| 5 | [`security-model.md`](security-model.md) | Tocas firma, llaves, certificados, CAF, logs, la CLI o el simulador por red. |
| 6 | [`public-api.md`](public-api.md) | Tocas cualquier tipo o función pública. Es el contrato con los usuarios del crate. |
| 7 | [`implementation-status.md`](implementation-status.md) | Planificas trabajo: qué existe, qué falta, qué tarea lo hace. |
| 8 | [`handoff.md`](handoff.md) | Retomas en otra sesión/equipo: estado, pendientes, entorno y forma de trabajo. |

## Documentos relacionados

| Documento | Ubicación |
|:--|:--|
| Formato DTE (normativo: tipos, esquema XML, timbre, firma, folios, envío, libros, auth) | [`docs/dte-spec.md`](../docs/dte-spec.md) |
| Simulador del SII (endpoints, escenarios) | [`docs/simulator-spec.md`](../docs/simulator-spec.md) *(lo crea SIM-01)* |
| Certificación contra el ambiente real del SII (manual, dueño) | [`docs/certification.md`](../docs/certification.md) *(lo crea CERT-01)* |
| Publicación en crates.io | [`docs/release.md`](../docs/release.md) *(lo crea DOCS-01)* |
| Plan de tareas | [`tasks/README.md`](../tasks/README.md) |
| Proyecto que consumirá el SDK (solo lectura) | `../rework-sistema-facturacion/` (facturación electrónica) |
| Gemelo de referencia (estructura, no copiar dominio) | `../transbank-SDK/` |

## Nombres

| Concepto | Nombre aquí |
|:--|:--|
| Este repo | `sii-sdk` |
| Crates | `sii` (lib, `use sii`), `sii-sim` (lib + bin `sii-sim`), `sii-cli` (bin `sii`) |
| Documento Tributario Electrónico | DTE |
| Autorización de folios (XML del SII con llave RSA) | CAF |
| Sello digital embebido en cada DTE (PDF417) | TED (timbre electrónico) |
| Sobre que agrupa DTE para enviar al SII | Envío (`EnvioDTE` / `EnvioBOLETA`) |
| Quien emite el DTE | Emisor (contribuyente) |
| Quien lo recibe | Receptor |
| El computador/servicio que usa el SDK | host |
