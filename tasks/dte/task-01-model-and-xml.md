# TASK-DTE-01: Modelo DTE + XML para factura afecta (33) y exenta (34)

| Atributo | Detalle |
|:--|:--|
| **ID** | `DTE-01` |
| **Estado** | ⬜ pendiente |
| **Depende de** | CORE-01 |
| **Rama** | `dev/dte-01-model-xml` |
| **Prioridad** | P0 |

## 1. Contexto
`docs/dte-spec.md` §3, `constraints.md` (DTE-01..04). Primer corte del modelo y la serialización XML, con los dos tipos más comunes.

## 2. Requerimientos
- **RF-01 Modelo:** structs tipados para `Encabezado` (IdDoc, Emisor, Receptor, Totales), `Detalle` (1..n) y, para esta tarea, tipos 33 y 34. Builder que valide reglas: 33 calcula IVA 19 % (`IVA = round(MntNeto*0.19)`, `MntTotal` coherente); 34 sin IVA, montos en `MntExe` (DTE-02). Montos `Monto` (DTE-03), RUT validado (DTE-04).
- **RF-02 XML:** `Dte::to_xml() -> Result<Vec<u8>, Error>` produce el `<DTE><Documento>` del SII (sin TED ni firma todavía), con `quick-xml`. Nombres/estructura exactos del esquema oficial; lo que no esté claro se reporta, no se inventa (LEG-01).
- **RF-03 Fixtures:** `tests/fixtures/dte-33.xml` y `dte-34.xml` derivados del esquema oficial (citar fuente), y test que compara el XML canónico generado contra el fixture (DTE-01).

## 3. Tests
Serialización de 33 y 34 vs fixture; IVA exacto; exenta sin IVA; RUT inválido rechazado; montos no-f64.

## 4. DoD
AGENTS.md §8. `--no-default-features` ok.

## Reporte
_(lo completa el agente)_
