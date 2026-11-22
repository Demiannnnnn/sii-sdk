# TASK-CAF-01: Parseo y validación del CAF

| Atributo | Detalle |
|:--|:--|
| **ID** | `CAF-01` |
| **Estado** | ⬜ pendiente |
| **Depende de** | CORE-01 |
| **Rama** | `dev/caf-01-parse` |
| **Prioridad** | P0 |

## 1. Contexto
`docs/dte-spec.md` §5, `constraints.md` (CAF-01), `security-model.md`. El CAF trae el rango de folios y la llave RSA privada del timbre.

## 2. Requerimientos
- **RF-01 Parse:** `Caf::parse(&[u8]) -> Result<Caf, Error>` lee `AUTORIZACION/CAF`: `DA` (RE, RS, TD, RNG D/H, FA, RSAPK M/E, IDK), `FRMA`, `RSASK` (llave privada), `RSAPUBK`.
- **RF-02 Rango/tipo:** `rango() -> (u64,u64)`, `tipo() -> TipoDte`, `contiene(folio) -> bool`. Folio fuera de `[D,H]` lo rechaza quien timbra (`Err(FolioOutOfRange)`).
- **RF-03 Validar FRMA:** cuando se dispone de la llave pública del SII del ambiente, verifica la firma `FRMA` sobre `DA`. Si no se dispone, lo documenta y marca `caf` como "firma no verificada" sin fallar el parseo (configurable).
- **RF-04 Secreto:** `RSASK` queda encapsulada; `Debug` la redacta (SEC-02). Expone un `Signer` del TED sin revelar la llave.
- **RF-05 Fixtures:** CAF de prueba **autogenerado** (llave RSA de juguete) en `tests/fixtures/caf-*.xml` con comentario de que es ficticio (SEC-01).

## 3. Tests
Parse de un CAF de prueba; rango y tipo correctos; folio dentro/fuera; `FRMA` válida vs alterada; `Debug` no filtra la llave.

## 4. DoD
AGENTS.md §8. `--no-default-features` ok.

## Reporte
_(lo completa el agente)_
