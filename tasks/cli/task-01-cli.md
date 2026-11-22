# TASK-CLI-01: CLI sii (armar, timbrar, firmar, enviar, estado)

| Atributo | Detalle |
|:--|:--|
| **ID** | `CLI-01` |
| **Estado** | ⬜ pendiente |
| **Depende de** | SIM-01 |
| **Rama** | `dev/cli-01-cli` |
| **Prioridad** | P2 |

## 1. Contexto
Bin `sii` (crate `sii-cli`). Herramienta de línea de comandos sobre el crate `sii`, contra el simulador o el SII real.
## 2. Requerimientos
- **RF-01 Comandos:** `dte build` (desde JSON/args), `dte timbrar` (con CAF), `dte firmar` (con certificado), `envio enviar` (contra sim o ambiente), `envio estado <trackid>`, `libro build`. Salida humana en español; `--json` opcional (claves en inglés).
- **RF-02 Secretos:** certificado desde archivo + `SII_CERT_PASSWORD` (env), CAF desde archivo; nunca en argumentos en claro (SEC-03). Ambiente obligatorio (`--ambiente certificacion|produccion`), sin default a producción.
- **RF-03 Salida/errores:** `std::io::Write` (no `println!`), `ExitCode` estables, errores en español sin filtrar secretos.
## 3. Tests
Comandos principales contra el simulador (end-to-end de armar→timbrar→firmar→enviar→estado) con certificado/CAF de prueba.
## 4. DoD
AGENTS.md §8.
## Reporte
_(lo completa el agente)_
