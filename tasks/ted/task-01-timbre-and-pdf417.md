# TASK-TED-01: Timbre electrónico (DD, FRMT, PDF417)

| Atributo | Detalle |
|:--|:--|
| **ID** | `TED-01` |
| **Estado** | ⬜ pendiente |
| **Depende de** | CAF-01, DTE-01 |
| **Rama** | `dev/ted-01-timbre` |
| **Prioridad** | P0 |

## 1. Contexto
`docs/dte-spec.md` §4, `constraints.md` (TED-01, TED-02). El timbre se firma con la llave del CAF usando RSA-SHA1 (exigencia del SII, no se cambia).

## 2. Requerimientos
- **RF-01 DD:** construir la cadena canónica `DD` con exactamente los campos del SII (RE, TD, F, FE, RR, RSR, MNT, IT1, CAF, TSTED), byte a byte igual al formato oficial. Hora inyectada (QA-03).
- **RF-02 FRMT:** firmar `DD` con la llave privada del CAF, algoritmo RSA-SHA1. `Dte::timbrar(&Caf, folio, clock) -> Result<DteTimbrado, Error>`; folio fuera del rango del CAF → `Err` (CAF-01).
- **RF-03 PDF417 data:** `ted_pdf417_data()` devuelve el `<TED>` serializado que el host codifica como PDF417 (no se genera el bitmap, ADR-06).
- **RF-04 Fixtures:** `DD` de prueba vs fixture byte a byte; `FRMT` que verifica con la llave pública del CAF.

## 3. Tests
`DD` canónico exacto; `FRMT` válida y verificable; timbrar fuera de rango falla; timbre reproducible con hora fija.

## 4. DoD
AGENTS.md §8.

## Reporte
_(lo completa el agente)_
