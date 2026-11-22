# Plan maestro de tareas — sii-sdk

Mismo formato que `transbank-SDK` y `rework-sistema-facturacion`: cada tarea tiene contexto, requerimientos
(`RF-*`), reglas, tests, DoD y una sección `## Reporte` que completa el agente.

> Antes de tomar una tarea lee `AGENTS.md`, todo `.ai-context/` y [`../.ai-context/implementation-status.md`](../.ai-context/implementation-status.md). Si el código y una tarea discrepan, `.ai-context/` y `docs/dte-spec.md` mandan sobre el pseudocódigo de la tarea; repórtalo.

## 1. Principios

1. **XML exacto del SII** (DTE-01): cada tipo y cada envío tienen un test contra un fixture oficial.
2. **Sans-IO primero** (ADR-01): núcleo sin E/S ni reloj; la red detrás de `Transport`.
3. **Firma inyectada** (ADR-02): el núcleo no abre certificados ni toca el SO.
4. **Rechazo ≠ error** (ADR-04): el rechazo del SII es un valor de dominio.
5. **Sin pánicos ni `unsafe`** (AGENTS.md §6).
6. **Secretos jamás en el repo** (SEC-01): CAF/certificados de prueba autogenerados.
7. **Cada tarea deja el workspace verde** (AGENTS.md §8).

## 2. Estructura

```
tasks/
├── README.md
├── infra/   task-01-workspace-and-ci
├── core/    task-01-types-rut-monto-errors
├── caf/     task-01-parse-and-validate
├── dte/     task-01-model-and-xml · task-02-rules-notes-guides
├── ted/     task-01-timbre-and-pdf417
├── sig/     task-01-xmldsig
├── envio/   task-01-envio-dte-boleta
├── libro/   task-01-compra-venta
├── auth/    task-01-semilla-token
├── transport/ task-01-transport-client-http
├── sim/     task-01-simulator
├── cli/     task-01-cli
├── cert/    task-01-loader-pkcs12 · task-99-certification  (CERT-99 manual, dueño)
└── docs/    task-01-docs-and-release-prep
```

## 3. Orden de ejecución

| # | ID | Tarea | Depende de | Prioridad | Estado |
|:--|:--|:--|:--|:--|:--|
| 1 | [INFRA-01](infra/task-01-workspace-and-ci.md) | Workspace Cargo, lints, licencias por crate, CI, skeleton de 3 crates | — | P0 | ✅ |
| 2 | [CORE-01](core/task-01-types-rut-monto-errors.md) | `Rut` (DV), `Monto`, `TipoDte`, `Ambiente`, `Error` | INFRA-01 | P0 | ✅ |
| 3 | [CAF-01](caf/task-01-parse-and-validate.md) | Parse del CAF: rango, llave RSA, validación FRMA | CORE-01 | P0 | ✅ |
| 4 | [DTE-01](dte/task-01-model-and-xml.md) | Modelo DTE + XML para 33 y 34 (factura afecta/exenta) | CORE-01 | P0 | ✅ |
| 5 | [DTE-02](dte/task-02-rules-notes-guides.md) | Boletas (39/41), guía (52), notas (56/61) con referencias | DTE-01 | P1 | ✅ |
| 6 | [TED-01](ted/task-01-timbre-and-pdf417.md) | Timbre: `DD`, `FRMT` RSA-SHA1 con el CAF, datos PDF417 | CAF-01, DTE-01 | P0 | ✅ |
| 7 | [SIG-01](sig/task-01-xmldsig.md) | XMLDSig: `Signer`/`Certificate`, firmar/verificar, C14N | DTE-01 | P0 | ✅ |
| 8 | [ENVIO-01](envio/task-01-envio-dte-boleta.md) | `EnvioDTE`/`EnvioBOLETA`, carátula, firma del sobre | SIG-01, TED-01 | P1 | ✅ |
| 9 | [AUTH-01](auth/task-01-semilla-token.md) | Semilla/token del SII (modelo + firma de la semilla) | SIG-01 | P1 | ✅ |
| 10 | [TRANSPORT-01](transport/task-01-transport-client-http.md) | `Transport`, `SiiClient`, `HttpTransport` (feature `http`), estado | ENVIO-01, AUTH-01 | P1 | ✅ |
| 11 | [SIM-01](sim/task-01-simulator.md) | `sii-sim`: simulador de los endpoints del SII | TRANSPORT-01 | P1 | ⬜ |
| 12 | [LIBRO-01](libro/task-01-compra-venta.md) | Libro de compra/venta (IECV) | SIG-01 | P2 | ✅ |
| 13 | [CLI-01](cli/task-01-cli.md) | `sii-cli` (bin `sii`): armar, timbrar, firmar, enviar, estado | SIM-01 | P2 | ⬜ |
| 14 | [CERT-LOADER-01](cert/task-01-loader-pkcs12.md) | Cargar certificado/llave desde PKCS#12/PEM (feature `pkcs12`) | SIG-01 | P2 | ✅ |
| 15 | [DOCS-01](docs/task-01-docs-and-release-prep.md) | README de crates, `docs/release.md`, CHANGELOG, preparación crates.io | CLI-01 | P2 | ⬜ |
| — | [CERT-99](cert/task-99-certification.md) | Certificación contra el SII real (manual, **dueño**) | DOCS-01 | P3 | ⬜ (dueño) |

## 4. Reglas de ejecución

1. Elige la primera tarea `⬜` cuyas dependencias estén `✅`.
2. El orquestador crea la rama `dev/<id>-<tema>` desde `develop-AGENTS` y la deja checkouteada.
3. Lee el archivo de la tarea y sus referencias de `.ai-context/` y `docs/`.
4. Implementa SOLO esa tarea. Corre los comandos de AGENTS.md §8.
5. Actualiza la línea **Estado** de tu archivo y la sección `## Reporte`. Commitea en tu rama `dev/*`.
6. El orquestador revisa, integra en `develop-AGENTS` y marca la fila como ✅.


## Documentación para humanos (estándar del ecosistema, 2026-11-22)

| ID | Tarea | Depende de | Estado |
|:--|:--|:--|:--|
| [HDOC-01](hdoc/task-01-estructura-diagramas-readme.md) | Estructura de docs para humanos, diagramas y README robusto | — | ⬜ |
| [HDOC-02](hdoc/task-02-informe-tecnico.md) | Informe técnico e informe de diagramas | HDOC-01 | ⬜ |
| [HDOC-03](hdoc/task-03-legal-seguridad.md) | Informe legal e informe de seguridad | HDOC-01 | ⬜ |
| [HDOC-04](hdoc/task-04-despliegue-pruebas-usuario.md) | Despliegue/integración, pruebas | HDOC-01 | ⬜ |
