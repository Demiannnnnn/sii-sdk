# TASK-LIBRO-01: Libro de compra/venta (IECV)

| Atributo | Detalle |
|:--|:--|
| **ID** | `LIBRO-01` |
| **Estado** | ⬜ pendiente |
| **Depende de** | SIG-01 |
| **Rama** | `dev/libro-01-compra-venta` |
| **Prioridad** | P2 |

## 1. Contexto
`docs/dte-spec.md` §8. Resumen mensual de documentos emitidos/recibidos, firmado.
## 2. Requerimientos
- **RF-01 Modelo:** `LibroCompraVenta` con carátula (periodo, RUT, tipo de operación) y detalle/resumen.
- **RF-02 XML + firma:** serialización al formato IECV del SII y firma XMLDSig (SIG-01).
- **RF-03 Fixture:** un libro de prueba con su test.
## 3. Tests
Serialización vs fixture; totales correctos; libro firmado que verifica.
## 4. DoD
AGENTS.md §8.
## Reporte
_(lo completa el agente)_
