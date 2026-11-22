# TASK-DTE-01: Modelo DTE + XML para factura afecta (33) y exenta (34)

| Atributo | Detalle |
|:--|:--|
| **ID** | `DTE-01` |
| **Estado** | ✅ terminada |
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

### Resumen
Se implementó el modelo DTE tipado en `crates/sii/src/dte.rs` para los tipos 33 (Factura electrónica afecta a IVA 19 %) y 34 (Factura no afecta o exenta electrónica), con validación estricta de reglas tributarias (DTE-02, DTE-03, DTE-04). Se implementó la serialización XML canónica a través de `quick-xml` en `Dte::to_xml() -> Result<Vec<u8>, Error>`, produciendo la estructura `<DTE version="1.0"><Documento ID="...">` requerida por el SII. Se crearon los fixtures oficiales `tests/fixtures/dte-33.xml` y `dte-34.xml` (citando el esquema oficial `DTE_v10.xsd` del SII) y se desarrollaron 9 tests de integración en `crates/sii/tests/dte_model.rs` que verifican comparación byte a byte contra fixture, cálculo exacto de IVA (19 %), exención sin IVA, rechazo de RUT inválido, consistencia de líneas y precisión de montos en `Monto` entero sin `f64`.

### Archivos
- `crates/sii/Cargo.toml`: Agregadas dependencias permitidas `quick-xml`, `time` y dev-dependency `serde_json`.
- `Cargo.lock`: Actualizado por Cargo con dependencias locked.
- `crates/sii/src/error.rs`: Agregadas variantes `DteValidation`, `Xml`, `InvalidDate`.
- `crates/sii/src/dte.rs`: Modelo DTE (`Dte`, `Encabezado`, `IdDoc`, `Emisor`, `Receptor`, `Totales`, `Detalle`, `CdgItem`), builders `FacturaAfectaBuilder` / `FacturaBuilder` y `FacturaExentaBuilder`, helpers de fecha y serializador XML canónico con `quick-xml`.
- `crates/sii/src/lib.rs`: Re-exportados tipos públicos del módulo `dte`.
- `crates/sii/tests/fixtures/dte-33.xml` y `tests/fixtures/dte-33.xml`: Fixture oficial canónico para Factura Afecta (33).
- `crates/sii/tests/fixtures/dte-34.xml` y `tests/fixtures/dte-34.xml`: Fixture oficial canónico para Factura Exenta (34).
- `crates/sii/tests/dte_model.rs`: Suite completa de tests de integración para modelo, validaciones y serialización XML vs fixture.

### Resultado real de cada comando de AGENTS.md §8

1. `cargo fmt --all --check`
```
(salida vacía, exit code 0)
```

2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
```
    Checking sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.21s
```

3. `cargo clippy -p sii --no-default-features -- -D warnings`
```
    Checking sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.16s
```

4. `cargo test --workspace --all-features`
```
     Running unittests src/lib.rs (target/debug/deps/sii-5a23bf666cfcfbcb)

running 14 tests
test codes::tests::test_tipo_dte_boleta_vs_dte ... ok
test codes::tests::test_ambiente_methods ... ok
test codes::tests::test_tipo_dte_invalid_code ... ok
test codes::tests::test_tipo_dte_requires_reference ... ok
test codes::tests::test_ambiente_from_str ... ok
test codes::tests::test_tipo_dte_u16_roundtrip ... ok
test types::tests::test_monto_iva_19_exact ... ok
test types::tests::test_monto_overflow_safety ... ok
test types::tests::test_monto_parsing ... ok
test types::tests::test_rut_invalid_cases ... ok
test types::tests::test_rut_canonical_display_and_dots ... ok
test types::tests::test_rut_valid_cases ... ok
test types::tests::test_rut_modulo_11_proptest ... ok
test types::tests::test_rut_invalid_dv_rejected_proptest ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/dte_model.rs (target/debug/deps/dte_model-2275cb4412a7d4bf)

running 9 tests
test test_builder_rejects_invalid_rut ... ok
test test_factura_afecta_with_mixed_items ... ok
test test_monto_no_f64_integer_precision ... ok
test test_builder_validation_errors ... ok
test test_serde_json_roundtrip ... ok
test test_quick_xml_roundtrip_verification ... ok
test test_serialize_dte_33_against_fixture ... ok
test test_serialize_dte_34_against_fixture ... ok
test test_factura_afecta_iva_exact_rounding ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/smoke.rs (target/debug/deps/smoke-9d76ac494d2550b4)

running 1 test
test smoke_test_version ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/sii-10c3aa55fe865d4e)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/smoke.rs (target/debug/deps/smoke-52a5b3284cb345d2)

running 1 test
test smoke_test_cli_runs ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s

     Running unittests src/lib.rs (target/debug/deps/sii_sim-6ed49734f9393de8)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/sii_sim-24e11513e72a4b26)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/smoke.rs (target/debug/deps/smoke-bf7883f2324ebd1d)

running 2 tests
test smoke_test_sim_version ... ok
test smoke_test_sim_bin_runs ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s

   Doc-tests sii

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests sii_sim

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

5. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features`
```
    Checking sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii)
 Documenting sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii)
 Documenting sii-sim v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii-sim)
 Documenting sii-cli v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.34s
```

### Decisiones
- Se utilizó `NroLinDet` en lugar de `NroLinData` para el número secuencial de línea en `<Detalle>`, conforme al esquema oficial normativo del SII `DTE_v10.xsd`.
- El atributo `ID` en `<Documento>` se genera por defecto en el formato canónico `F{folio}T{tipo}` (ej. `F60T33`, `F100T34`), permitiendo también personalización mediante `documento_id(...)`.
- Se preservó el soporte opcional para `serde` serializando `Date` de forma compatible como strings ISO `YYYY-MM-DD`.
- Los fixtures fueron creados tanto en `crates/sii/tests/fixtures/` como en `tests/fixtures/` para garantizar acceso uniforme desde la raíz del workspace y desde el crate.

### Pendientes/bloqueos
- Ninguno. La tarea se completó al 100 %.

### Desviaciones de la spec
- En `docs/dte-spec.md` §3 línea 41 se menciona `NroLinData`, pero el esquema normativo `DTE_v10.xsd` del SII exige `NroLinDet`. Se implementó `NroLinDet` para cumplir estrictamente con el validador oficial del SII.
