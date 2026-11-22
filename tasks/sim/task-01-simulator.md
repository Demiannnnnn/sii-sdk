# TASK-SIM-01: Simulador del SII (sii-sim)

| Atributo | Detalle |
|:--|:--|
| **ID** | `SIM-01` |
| **Estado** | ⬜ pendiente |
| **Depende de** | TRANSPORT-01 |
| **Rama** | `dev/sim-01-simulator` |
| **Prioridad** | P1 |

## 1. Contexto
Gemelo del simulador de transbank-SDK. Imita los endpoints del SII para desarrollar y testear sin el SII real.
## 2. Requerimientos
- **RF-01 Endpoints:** semilla, token, upload (devuelve TrackID), consulta de estado. Respuestas deterministas y configurables por escenario (aceptado, rechazado, en proceso, token inválido).
- **RF-02 lib + bin:** `sii-sim` como lib (para tests, implementando/complementando `Transport`) y bin `sii-sim` (servidor local TCP/HTTP sobre 127.0.0.1).
- **RF-03 Escenarios:** definidos en datos (JSON/código), documentados en `docs/simulator-spec.md` (crear este doc).
- **RF-04 Sin red externa:** solo 127.0.0.1 (QA-01). El bin imprime con `std::io::Write`.
## 3. Tests
`client` contra el simulador cubriendo aceptado/rechazado/en proceso; estado por TrackID.
## 4. DoD
AGENTS.md §8.
## Reporte
_(lo completa el agente)_
