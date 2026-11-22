# TASK-DTE-02: Boletas (39/41), guía (52) y notas (56/61)

| Atributo | Detalle |
|:--|:--|
| **ID** | `DTE-02` |
| **Estado** | ⬜ pendiente |
| **Depende de** | DTE-01 |
| **Rama** | `dev/dte-02-notes-guides` |
| **Prioridad** | P1 |

## 1. Contexto
Extiende el modelo a los tipos restantes del alcance (`domain.md` §2). Las notas exigen referencia; las boletas van en otro sobre.

## 2. Requerimientos
- **RF-01 Boletas 39/41:** modelo + XML; marcar que se agrupan en `EnvioBOLETA` (lo usa ENVIO-01). Receptor puede ir mínimo.
- **RF-02 Guía 52:** modelo + XML con los campos de traslado.
- **RF-03 Notas 56/61:** exigen al menos una `Referencia` válida (TpoDocRef, FolioRef, FchRef, CodRef, RazonRef); sin referencia → `Err` (DTE-02).
- **RF-04 Fixtures:** un fixture por tipo (39, 41, 52, 56, 61) y su test de serialización.

## 3. Tests
Serialización por tipo vs fixture; nota sin referencia rechazada; clasificación boleta vs DTE para el sobre.

## 4. DoD
AGENTS.md §8.

## Reporte
_(lo completa el agente)_
