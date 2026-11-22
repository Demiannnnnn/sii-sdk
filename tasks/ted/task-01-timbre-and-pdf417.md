# TASK-TED-01: Timbre electrónico (DD, FRMT, PDF417)

| Atributo | Detalle |
|:--|:--|
| **ID** | `TED-01` |
| **Estado** | ✅ terminada |
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

### Resumen
Implementación completa del Timbre Electrónico DTE (TED), bloque canónico `<DD>`, firma RSA-SHA1 (`<FRMT>`) usando la llave privada provista en el CAF, verificación criptográfica con la llave pública del CAF, extracción de datos para PDF417 (`ted_pdf417_data()`, según ADR-06 y TED-02) y método de timbrado en DTE (`Dte::timbrar(&Caf, folio, clock) -> Result<DteTimbrado, Error>`). Todo desarrollado bajo arquitectura Sans-IO con inyección de reloj (`Clock`), sin `unsafe`, sin dependencias externas fuera de las aprobadas, y con cobertura integral de tests y fixtures byte a byte.

### Archivos
- `crates/sii/src/types.rs`: Definición del trait `Clock`, implementación `ManualClock`, funciones auxiliares de formato y parseo de marcas temporales (`format_timestamp`, `parse_timestamp`), e implementaciones para tipos de `time`.
- `crates/sii/src/caf.rs`: Exposición interna `pub(crate)` de `find_tag_slice` para extracción segura de fragmentos XML en `ted.rs`.
- `crates/sii/src/ted.rs`: Módulo `ted` implementando el formateo y truncado de textos (`clean_truncate`), estructuras `Dd` y `DdBuilder`, generación de XML canónico para `<DD>`, firma RSA-SHA1 con CAF para generar el bloque `<TED>`, métodos de verificación de firma (`verificar_firma`), parseo roundtrip y extracción de bytes para PDF417 (`pdf417_data()`).
- `crates/sii/src/dte.rs`: Método `Dte::timbrar`, estructura `DteTimbrado` con accesores (`dte`, `ted`, `folio`, `tipo`, `id`, `ted_pdf417_data`) y serialización XML completa que incrusta el nodo `<TED>`.
- `crates/sii/src/lib.rs`: Re-exportación pública de `Dd`, `Ted`, `DteTimbrado`, `Clock` y `ManualClock`.
- `crates/sii/tests/fixtures/dd-33.xml`: Fixture oficial canónico byte a byte para el elemento `<DD>` correspondiente al DTE 33 y CAF 33.
- `crates/sii/tests/fixtures/ted-33.xml`: Fixture oficial canónico para el nodo `<TED>` completo firmado.
- `crates/sii/tests/ted.rs`: Suite con 12 tests de integración (canonicidad byte a byte, verificación RSA-SHA1, PDF417 data, rangos válidos e inválidos de folios, rechazo de RUT o TipoDte dispares, detección de manipulación de datos o firma, y reproducibilidad con reloj inyectado).

### Resultado real de cada comando de AGENTS.md §8

1. `cargo fmt --all --check`
```
(salida vacía, exit code 0)
```

2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.30s
```

3. `cargo clippy -p sii --no-default-features -- -D warnings`
```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s
```

4. `cargo test --workspace --all-features`
```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.13s
     Running unittests src/lib.rs (target/debug/deps/sii-6fce74122b8a9380)

running 29 tests
test caf::tests::test_caf_parse_valid ... ok
test caf::tests::test_caf_invalid_ranges ... ok
test caf::tests::test_folio_range_containment ... ok
test codes::tests::test_ambiente_from_str ... ok
test codes::tests::test_ambiente_methods ... ok
test codes::tests::test_tipo_dte_boleta_vs_dte ... ok
test codes::tests::test_tipo_dte_invalid_code ... ok
test codes::tests::test_tipo_dte_requires_reference ... ok
test codes::tests::test_tipo_dte_u16_roundtrip ... ok
test ted::tests::test_clean_truncate ... ok
test ted::tests::test_clock_injection_reproducible_ted ... ok
test caf::tests::test_caf_parse_and_verify ... ok
test caf::tests::test_caf_tampered_da_fails_verification ... ok
test ted::tests::test_ted_signing_and_verification ... ok
test types::tests::test_monto_iva_19_exact ... ok
test types::tests::test_monto_overflow_safety ... ok
test types::tests::test_monto_parsing ... ok
test types::tests::test_rut_canonical_display_and_dots ... ok
test types::tests::test_rut_invalid_cases ... ok
test types::tests::test_rut_invalid_dv_rejected_proptest ... ok
test types::tests::test_rut_modulo_11_proptest ... ok
test types::tests::test_rut_valid_cases ... ok
test ted::tests::test_ted_tampered_fails_verification ... ok
test ted::tests::test_dd_canonical_xml_format ... ok
test caf::tests::test_caf_sign_ted_and_signer_trait ... ok
test caf::tests::test_caf_debug_redacts_private_key ... ok
test caf::tests::test_caf_tampered_signature_fails_verification ... ok
test caf::tests::test_caf_invalid_rut_or_tipo ... ok
test caf::tests::test_caf_keypair_mismatch ... ok

test result: ok. 29 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.68s

     Running tests/caf.rs (target/debug/deps/caf-f7d770763b7ba595)

running 6 tests
test test_fixture_caf_33_parse_and_fields ... ok
test test_fixture_caf_33_folio_range_check ... ok
test test_fixture_caf_33_debug_redaction ... ok
test test_fixture_caf_33_tampered_fails ... ok
test test_fixture_caf_33_signature_verification ... ok
test test_fixture_caf_33_ted_signing ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/dte_model.rs (target/debug/deps/dte_model-123988ae6dc5fe73)

running 9 tests
test test_builder_rejects_invalid_rut ... ok
test test_monto_no_f64_integer_precision ... ok
test test_quick_xml_roundtrip_verification ... ok
test test_builder_validation_errors ... ok
test test_serde_json_roundtrip ... ok
test test_serialize_dte_34_against_fixture ... ok
test test_factura_afecta_with_mixed_items ... ok
test test_serialize_dte_33_against_fixture ... ok
test test_factura_afecta_iva_exact_rounding ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/smoke.rs (target/debug/deps/smoke-c7a278967aaeb19f)

running 1 test
test smoke_test_version ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/ted.rs (target/debug/deps/ted-31da5cf733c7dd85)

running 12 tests
test test_timbrar_caf_issuer_rut_mismatch_fails ... ok
test test_timbrar_caf_type_mismatch_fails ... ok
test test_timbrar_out_of_range_fails ... ok
test test_ted_parse_roundtrip_with_fixture ... ok
test test_stamped_dte_xml_serialization ... ok
test test_tampered_frmt_fails_verification ... ok
test test_frmt_verification_with_caf_public_key ... ok
test test_tampered_dd_fails_verification ... ok
test test_dd_canonical_byte_for_byte_against_fixture ... ok
test test_ted_pdf417_data_byte_for_byte_against_fixture ... ok
test test_timbre_reproducible_with_clock ... ok
test test_timbrar_within_range ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running unittests src/main.rs (target/debug/deps/sii-221e21ce97abd02c)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/smoke.rs (target/debug/deps/smoke-26d3b49900abe3e9)

running 1 test
test smoke_test_cli_runs ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.07s

     Running unittests src/lib.rs (target/debug/deps/sii_sim-70616e7ad37f15f7)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/sii_sim-8942a21c1e1323a8)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/smoke.rs (target/debug/deps/smoke-1b300c7caaac2247)

running 2 tests
test smoke_test_sim_version ... ok
test smoke_test_sim_bin_runs ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.65s

   Doc-tests sii

running 1 test
test crates/sii/src/caf.rs - caf (line 16) - compile ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

all doctests ran in 0.55s; merged doctests compilation took 0.30s
   Doc-tests sii_sim

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

5. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features`
```
warning: output filename collision at /Users/demian/Documents/Github/sii-sdk-pro/target/doc/sii/index.html
  |
  = note: the bin target `sii` in package `sii-cli v0.1.0 (/Users/demian/Documents/Github/sii-sdk-pro/crates/sii-cli)` has the same output filename as the lib target `sii` in package `sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-pro/crates/sii)`
  = note: this is a known bug where multiple crates with the same name use the same path; see <https://github.com/rust-lang/cargo/issues/6313>
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s
   Generated /Users/demian/Documents/Github/sii-sdk-pro/target/doc/sii/index.html and 2 other files
```

### Decisiones
- **Formato canónico de DD:** Se serializa estrictamente de forma lineal (`<DD><RE>...</RE><TD>...</TD>...<CAF version="1.0">...</CAF><TSTED>...</TSTED></DD>`) sin saltos de línea ni espacios extras entre tags, cumpliendo con la especificación del SII para garantizar que la firma RSA-SHA1 sea reproducible y verificable byte a byte.
- **Truncado de campos de texto en DD:** Los campos `RSR` (Razón Social Receptor) e `IT1` (Nombre/Descripción del primer ítem) se truncan limpiamente a un máximo de 40 caracteres sin espacios iniciales ni finales sobrantes, conforme a la norma técnica del SII.
- **PDF417 Data (ADR-06 / TED-02):** `Ted::pdf417_data()` y `DteTimbrado::ted_pdf417_data()` exponen los bytes del elemento XML `<TED>` (`&[u8]`). La generación gráfica / rasterización del código de barras se delega al consumidor o CLI para mantener el core sans-IO y libre de dependencias pesadas de imagen/código de barras en el modelo base.
- **Inyección de tiempo (QA-03):** El timestamp del timbre (`TSTED`) se genera mediante el trait `Clock`, permitiendo que en producción se use el reloj del sistema (`OffsetDateTime::now_utc()`) y en pruebas se inyecte un reloj determinista (`ManualClock`), facilitando tests reproducibles byte a byte.
- **Validaciones en `Dte::timbrar`:** Se verifica que el folio solicitado se encuentre en el rango `[desde, hasta]` del CAF, que el tipo de DTE coincida con el tipo autorizado en el CAF, y que el RUT emisor coincida con el RUT del CAF. Si alguna condición no se cumple, retorna `Error::FolioOutOfRange` o `Error::DteValidation`.

### Pendientes/bloqueos
- Ninguno.

### Desviaciones de la spec
- Ninguna.
