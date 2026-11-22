# TASK-CORE-01: Tipos base: Rut, Monto, TipoDte, Ambiente, Error

| Atributo | Detalle |
|:--|:--|
| **ID** | `CORE-01` |
| **Estado** | ⬜ pendiente |
| **Depende de** | INFRA-01 |
| **Rama** | `dev/core-01-types` |
| **Prioridad** | P0 |

## 1. Contexto
Base tipada que usan todos los módulos. Ver `.ai-context/public-api.md` (Tipos base), `constraints.md` (DTE-03, DTE-04), `domain.md` §2.

## 2. Requerimientos
- **RF-01 `Rut`:** construcción validando el dígito verificador (módulo 11). `Rut::parse("76.123.456-7")` acepta con/sin puntos y con guion; DV `k`/`K` válido; RUT inválido → `Err(Error::InvalidRut)`. `Display` canónico. Sin pánico.
- **RF-02 `Monto`:** entero CLP (`i64`/`u64` documentado), aritmética segura (checked), sin `f64` (DTE-03). Soporta suma/resta/porcentaje de IVA sin perder exactitud.
- **RF-03 `TipoDte`:** enum con los códigos de `domain.md` §2 y conversión `u16 <-> TipoDte`; conocer si va en `EnvioDTE` o `EnvioBOLETA`.
- **RF-04 `Ambiente`:** `Certificacion`/`Produccion`, sin default a producción (ADR-07).
- **RF-05 `Error`:** enum `thiserror` con las variantes de `conventions.md` §6 (al menos `InvalidRut`, `InvalidMonto`); se irá ampliando. Sin exponer secretos.

## 3. Tests
- `Rut`: casos válidos/ inválidos, DV `k`, formato con/sin puntos; `proptest` del módulo 11.
- `Monto`: overflow seguro, IVA 19 % exacto sobre varios netos.
- `TipoDte`: round-trip u16; clasificación boleta vs DTE.

## 4. DoD
AGENTS.md §8 en verde (pegar salida). Compila con `--no-default-features`.

## Reporte
_(lo completa el agente)_
