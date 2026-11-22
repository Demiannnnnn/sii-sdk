# TASK-CORE-01: Tipos base: Rut, Monto, TipoDte, Ambiente, Error

| Atributo | Detalle |
|:--|:--|
| **ID** | `CORE-01` |
| **Estado** | ✅ terminada |
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

### Resumen
Se implementaron los tipos base del dominio tributario chileno para el crate `sii` según los requerimientos de CORE-01:
- `Rut`: Representación tipada con validación estricta de módulo 11, parsing de formatos canónicos y con puntos (`"12345678-5"`, `"12.345.678-5"`), soporte de DV `'k'`/`'K'`, formateo `Display` canónico (`"12345678-5"`) y formateo con puntos (`format_with_dots()`).
- `Monto`: Tipo seguro en pesos chilenos enteros (`i64`, DTE-03) sin uso de punto flotante (`f64`). Aritmética comprobada con detección de overflow (`checked_add`, `checked_sub`, `checked_mul`, `checked_div`, `add_monto`, `sub_monto`, `mul_scalar`), y cálculo exacto de IVA al 19 % con redondeo estándar según normativa del SII (`iva_19`, `total_con_iva`).
- `TipoDte`: Enum con los tipos DTE del alcance inicial (33, 34, 39, 41, 52, 56, 61), conversión bidireccional segura con `u16` (`code()`, `TryFrom<u16>`), métodos de clasificación (`is_boleta()`, `is_envio_boleta()`, `is_envio_dte()`, `requires_iva()`, `requires_reference()`).
- `Ambiente`: Enum explícito (`Certificacion`, `Produccion`) sin valor por defecto (ADR-07), con nombre interno SII (`sii_name()`).
- `Error`: Enum `thiserror` con variantes tipadas (`InvalidRut`, `InvalidMonto`, `InvalidTipoDte`, `InvalidAmbiente`), sin exposición de secretos.
- Pruebas unitarias completas y property-based testing con `proptest` para el algoritmo módulo 11 del RUT.

### Archivos
- `crates/sii/Cargo.toml`: dependencias `thiserror`, `serde` opcional y `proptest` en `dev-dependencies`.
- `crates/sii/src/error.rs`: definición del enum `Error` y alias `Result`.
- `crates/sii/src/codes.rs`: constantes DTE, enums `TipoDte` y `Ambiente`, con traits y pruebas unitarias.
- `crates/sii/src/types.rs`: structs `Rut` y `Monto`, funciones auxiliares, `proptest` y pruebas unitarias.
- `crates/sii/src/lib.rs`: re-exportación de tipos principales a la raíz del crate.
- `Cargo.lock`: actualización de dependencias generada automáticamente por cargo.
- `tasks/core/task-01-types-rut-monto-errors.md`: actualización de estado y reporte.

### Resultado real de cada comando de AGENTS.md §8

1. `cargo fmt --all --check`
```text
(exited with 0, sin diferencias)
```

2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.15s
(exited with 0, 0 warnings)
```

3. `cargo clippy -p sii --no-default-features -- -D warnings`
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.01s
(exited with 0, 0 warnings)
```

4. `cargo test --workspace --all-features`
```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.07s
     Running unittests src/lib.rs (target/debug/deps/sii-b94827fe17b0dcd5)

running 14 tests
test types::tests::test_monto_iva_19_exact ... ok
test codes::tests::test_tipo_dte_invalid_code ... ok
test codes::tests::test_tipo_dte_u16_roundtrip ... ok
test codes::tests::test_ambiente_methods ... ok
test codes::tests::test_tipo_dte_boleta_vs_dte ... ok
test codes::tests::test_tipo_dte_requires_reference ... ok
test types::tests::test_monto_overflow_safety ... ok
test codes::tests::test_ambiente_from_str ... ok
test types::tests::test_monto_parsing ... ok
test types::tests::test_rut_canonical_display_and_dots ... ok
test types::tests::test_rut_invalid_cases ... ok
test types::tests::test_rut_valid_cases ... ok
test types::tests::test_rut_modulo_11_proptest ... ok
test types::tests::test_rut_invalid_dv_rejected_proptest ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/smoke.rs (target/debug/deps/smoke-3dd5f338ba50974e)

running 1 test
test smoke_test_version ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/sii-9fcd7b62b22fe458)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/smoke.rs (target/debug/deps/smoke-e04e114e7c5f31ec)

running 1 test
test smoke_test_cli_runs ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s

     Running unittests src/lib.rs (target/debug/deps/sii_sim-984194aad89fda9a)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/sii_sim-b44ec86c4eaed9d6)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/smoke.rs (target/debug/deps/smoke-58ad806b7c4ae620)

running 2 tests
test smoke_test_sim_version ... ok
test smoke_test_sim_bin_runs ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s

   Doc-tests sii

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests sii_sim

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

5. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features`
```text
 Documenting sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-pro/crates/sii)
 Documenting sii-sim v0.1.0 (/Users/demian/Documents/Github/sii-sdk-pro/crates/sii-sim)
 Documenting sii-cli v0.1.0 (/Users/demian/Documents/Github/sii-sdk-pro/crates/sii-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s
   Generated /Users/demian/Documents/Github/sii-sdk-pro/target/doc/sii/index.html and 2 other files
```

### Decisiones
- Se utilizó `i64` para el almacenamiento interno de `Monto` garantizando ausencia total de tipos de punto flotante (`f64`), cumpliendo con la directiva DTE-03.
- Se implementó el redondeo entero `round(neto * 0.19)` con `(producto + 50) / 100` para valores positivos y `(producto - 50) / 100` para negativos, validado contra los casos de prueba normativos del SII.
- En `Rut`, se exige exactamente un guion `-` que separe el cuerpo del dígito verificador, aceptando puntos de separación de miles si están correctamente formateados.
- Se omitió la implementación de `Default` para `Ambiente`, forzando a los consumidores a elegir explícitamente el ambiente según ADR-07.

### Pendientes/bloqueos
- Ninguno. La tarea CORE-01 está completa y lista para ser integrada.

### Desviaciones de la spec
- Ninguna. Se siguió estrictamente la especificación en `docs/dte-spec.md` y `.ai-context/`.
