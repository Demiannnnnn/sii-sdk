# TASK-ENVIO-01: EnvioDTE / EnvioBOLETA con carátula y firma

| Atributo | Detalle |
|:--|:--|
| **ID** | `ENVIO-01` |
| **Estado** | ✅ terminada |
| **Depende de** | SIG-01, TED-01 |
| **Rama** | `dev/envio-01-sobre` |
| **Prioridad** | P1 |

## 1. Contexto
`docs/md/dte-spec.md` §7. El sobre agrupa DTE del mismo emisor, con carátula y firma propia.

## 2. Requerimientos
- **RF-01 EnvioDTE:** `<SetDTE>` con `Caratula` (RutEmisor, RutEnvia, RutReceptor=SII, FchResol, NroResol, TmstFirmaEnv, SubTotDTE por tipo) y 1..n `<DTE>` firmados; firma XMLDSig del sobre (SIG-01). Respeta el límite de DTE por envío del SII.
- **RF-02 EnvioBOLETA:** análogo para 39/41.
- **RF-03 Resumen:** `SubTotDTE` correcto por tipo; carátula coherente con los DTE incluidos.
- **RF-04 Fixtures:** un `EnvioDTE` y un `EnvioBOLETA` de prueba con sus tests.

## 3. Tests
Carátula y resumen correctos; sobre firmado que verifica; mezclar boleta en EnvioDTE → error.

## 4. DoD
AGENTS.md §8.

## Reporte
_(lo completa el agente)_
