# TASK-DTE-02: Boletas (39/41), guía (52) y notas (56/61)

| Atributo | Detalle |
|:--|:--|
| **ID** | `DTE-02` |
| **Estado** | ✅ terminada |
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

### Resumen
Implementación completa de modelos DTE, validaciones, generadores XML y builders para Boletas Electrónicas (39 afecta y 41 exenta), Guía de Despacho Electrónica (52) y Notas de Débito y Crédito Electrónicas (56 y 61):
- **RF-01 Boletas 39/41**: modelo y serialización XML con receptor mínimo (`Receptor::minimo`) o consumidor final (`Receptor::consumidor_final` con RUT `66666666-6`). Métodos inspectores `is_boleta()`, `is_envio_boleta()` y `is_envio_dte()` para agrupamiento en sobre `EnvioBOLETA`.
- **RF-02 Guía 52**: soporte de `TipoDespacho` (1..=3), `IndTraslado` (1..=9, obligatorio) y bloque opcional `Transporte` con `Patente`, `RUTTrans` y `Chofer` (`RUTChofer`, `NombreChofer`).
- **RF-03 Notas 56/61**: validación estricta de referencias (`validate_referencias`), requiriendo al menos una `Referencia` válida (`NroLinRef`, `TpoDocRef`, `FolioRef`, `FchRef`, `CodRef`, `RazonRef`). Rechazo con `Error::DteValidation` si no contiene referencias o campos requeridos están vacíos o fuera de rango.
- **RF-04 Fixtures**: creación de 5 fixtures XML canónicos (`dte-39.xml`, `dte-41.xml`, `dte-52.xml`, `dte-56.xml`, `dte-61.xml`) basados en la especificación y esquemas oficiales del SII (`DTE_v10.xsd`), con suite de tests de serialización que verifican concordancia byte-a-byte en forma canónica y roundtrip con `quick-xml`.

### Archivos
- `crates/sii/src/dte.rs`: modelos `Referencia`, `Transporte`, `Chofer`; extensión de `IdDoc`, `Receptor`, `Encabezado`, `Dte`, `DteTimbrado`; builders `BoletaBuilder`, `BoletaExentaBuilder`, `GuiaDespachoBuilder`, `NotaDebitoBuilder`, `NotaCreditoBuilder`; validaciones y generadores XML.
- `crates/sii/src/lib.rs`: re-exportación de los nuevos modelos y builders.
- `crates/sii/tests/dte_model.rs`: 19 tests de integración para Boletas, Guía, Notas, validaciones de referencia y clasificación de sobres.
- `crates/sii/tests/fixtures/dte-39.xml`, `dte-41.xml`, `dte-52.xml`, `dte-56.xml`, `dte-61.xml`: fixtures XML canónicos de prueba.
- `tests/fixtures/dte-39.xml`, `dte-41.xml`, `dte-52.xml`, `dte-56.xml`, `dte-61.xml`: fixtures a nivel de workspace.
- `tasks/dte/task-02-rules-notes-guides.md`: actualización de estado y reporte.

### Resultado real de cada comando de AGENTS.md §8

1. `cargo fmt --all --check`:
```
(salida vacía, código de salida 0)
```

2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`:
```
    Checking sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w3/crates/sii)
    Checking sii-sim v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w3/crates/sii-sim)
    Checking sii-cli v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w3/crates/sii-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.51s
```

3. `cargo clippy -p sii --no-default-features -- -D warnings`:
```
    Checking sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w3/crates/sii)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.66s
```

4. `cargo test --workspace --all-features`:
```
     Running unittests src/lib.rs (target/debug/deps/sii-7961c7452d7b78b1)
test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.30s

     Running tests/caf.rs (target/debug/deps/caf-b8e9a4f15f313178)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/dte_model.rs (target/debug/deps/dte_model-2177460332713ca4)
running 19 tests
test test_builder_rejects_invalid_rut ... ok
test test_monto_no_f64_integer_precision ... ok
test test_envelope_classification_all_types ... ok
test test_guia_despacho_requires_ind_traslado ... ok
test test_builder_validation_errors ... ok
test test_factura_afecta_with_mixed_items ... ok
test test_nota_credito_requires_reference ... ok
test test_nota_debito_requires_reference ... ok
test test_nota_credito_text_correction_zero_amount ... ok
test test_quick_xml_roundtrip_verification ... ok
test test_serialize_dte_33_against_fixture ... ok
test test_serde_json_roundtrip ... ok
test test_serialize_dte_34_against_fixture ... ok
test test_factura_afecta_iva_exact_rounding ... ok
test test_serialize_dte_39_against_fixture ... ok
test test_serialize_dte_41_against_fixture ... ok
test test_serialize_dte_56_against_fixture ... ok
test test_serialize_dte_52_against_fixture ... ok
test test_serialize_dte_61_against_fixture ... ok
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/signature.rs (target/debug/deps/signature-24a15392c8a327ee)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.84s

     Running tests/smoke.rs (target/debug/deps/smoke-bf46b271f41c8782)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/ted.rs (target/debug/deps/ted-25bd66124444ba1c)
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

     Running unittests src/main.rs (target/debug/deps/sii-a4f62bd789dfe9cd)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/smoke.rs (target/debug/deps/smoke-6f98ffd59483b357)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s

     Running unittests src/lib.rs (target/debug/deps/sii_sim-d7b013bc94c481c7)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/sii_sim-07e10c7872e35e75)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/smoke.rs (target/debug/deps/smoke-2de69f320e8f4292)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s

   Doc-tests sii
test crates/sii/src/caf.rs - caf (line 16) - compile ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests sii_sim
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

5. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features`:
```
 Documenting sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w3/crates/sii)
 Documenting sii-sim v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w3/crates/sii-sim)
 Documenting sii-cli v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w3/crates/sii-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.74s
```

### Decisiones
- Se valida en `Referencia` que `cod_ref` pertenezca a los códigos 1, 2 o 3 admitidos por el SII.
- En Guía de Despacho (52), `IndTraslado` es obligatorio con valores en rango 1..=9 según `DTE_v10.xsd`.
- Se añadieron `is_boleta()`, `is_envio_boleta()` e `is_envio_dte()` tanto a `Dte` como a `DteTimbrado` para desacoplar y simplificar la lógica de armado de sobre en `ENVIO-01`.

### Pendientes/bloqueos
- Ninguno.

### Desviaciones de la spec
- Ninguna.
