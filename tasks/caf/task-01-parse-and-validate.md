# TASK-CAF-01: Parseo y validación del CAF

| Atributo | Detalle |
|:--|:--|
| **ID** | `CAF-01` |
| **Estado** | ✅ terminada |
| **Depende de** | CORE-01 |
| **Rama** | `dev/caf-01-parse` |
| **Prioridad** | P0 |

## 1. Contexto
`docs/md/dte-spec.md` §5, `constraints.md` (CAF-01), `security-model.md`. El CAF trae el rango de folios y la llave RSA privada del timbre.

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

### Resumen
Implementación completa de `CAF-01` (parseo y validación de CAF):
- Implementación del struct `Caf` en `crates/sii/src/caf.rs` con métodos `parse`, `parse_and_verify`, `verificar_firma`, `is_firma_verificada`, `rango`, `tipo`, `contiene`, `verificar_folio`, `emisor_rut`, `razon_social`, `fecha_autorizacion`, `idk`, `algoritmo_firma`, `public_key`, `sign_ted`, `signer`, `raw_da`, `raw_xml`.
- Extracción y validación de `DA` (RUT con validación módulo 11, `TipoDte`, rango de folios con `1 <= D <= H`, fecha de autorización, clave pública `RSAPK` con módulo `M` y exponente `E` en Base64, e identificador `IDK`).
- Encapsulación de la llave privada RSA (`RSASK`) con redacción estricta en `Debug` (`"<redacted>"` según SEC-02), verificación de consistencia entre `RSASK` y `RSAPK`, e implementación del trait `Signer` (`crates/sii/src/signature.rs`) para timbrar el `TED` con RSA-SHA1.
- Verificación criptográfica de firma `FRMA` (SHA1withRSA) sobre el bloque `<DA>` canónico/raw utilizando la llave pública del SII.
- Creación de fixture de prueba `crates/sii/tests/fixtures/caf-33.xml` y `crates/sii/tests/fixtures/sii-test-pubkey.pub` con llaves RSA ficticias de juguete y comentarios explícitos de test (SEC-01).
- Tests unitarios y de integración completos en `crates/sii/src/caf.rs` y `crates/sii/tests/caf.rs`, incluyendo detección de alteraciones en `DA` y `FRMA`, verificación de rango, y proptests.

### Archivos
- `Cargo.lock`: dependencias resueltas para `quick-xml`, `rsa`, `sha1`, `base64`.
- `crates/sii/Cargo.toml`: adición de dependencias permitidas (`quick-xml`, `rsa`, `sha1` con feature `oid`, `base64`).
- `crates/sii/src/lib.rs`: exportación pública de `Caf` y `Signer`.
- `crates/sii/src/error.rs`: variantes `FolioOutOfRange`, `CafSignature`, `InvalidCaf`, `Xml`, `Crypto`.
- `crates/sii/src/signature.rs`: definición del trait `Signer`.
- `crates/sii/src/caf.rs`: modelo `Caf`, parser XML, validaciones, operaciones criptográficas y tests unitarios.
- `crates/sii/tests/fixtures/caf-33.xml`: fixture de prueba de CAF para tipo 33.
- `crates/sii/tests/fixtures/sii-test-pubkey.pub`: llave pública de prueba del SII para verificar `caf-33.xml`.
- `crates/sii/tests/caf.rs`: suite de tests de integración para `Caf`.
- `tasks/caf/task-01-parse-and-validate.md`: actualización de estado y reporte.

### Resultado real de cada comando de AGENTS.md §8
1. `cargo fmt --all --check`
```
(Exit code: 0)
```

2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
```
    Checking sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-pro/crates/sii)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.24s
```

3. `cargo clippy -p sii --no-default-features -- -D warnings`
```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s
```

4. `cargo test --workspace --all-features`
```
     Running unittests src/lib.rs (target/debug/deps/sii-152ea84a72cb8d8a)

running 24 tests
test caf::tests::test_caf_parse_valid ... ok
test caf::tests::test_caf_parse_and_verify ... ok
test caf::tests::test_folio_range_containment ... ok
test codes::tests::test_ambiente_from_str ... ok
test codes::tests::test_ambiente_methods ... ok
test codes::tests::test_tipo_dte_boleta_vs_dte ... ok
test codes::tests::test_tipo_dte_invalid_code ... ok
test codes::tests::test_tipo_dte_requires_reference ... ok
test codes::tests::test_tipo_dte_u16_roundtrip ... ok
test types::tests::test_monto_iva_19_exact ... ok
test types::tests::test_monto_overflow_safety ... ok
test types::tests::test_monto_parsing ... ok
test types::tests::test_rut_canonical_display_and_dots ... ok
test types::tests::test_rut_invalid_cases ... ok
test types::tests::test_rut_invalid_dv_rejected_proptest ... ok
test types::tests::test_rut_modulo_11_proptest ... ok
test types::tests::test_rut_valid_cases ... ok
test caf::tests::test_caf_invalid_ranges ... ok
test caf::tests::test_caf_debug_redacts_private_key ... ok
test caf::tests::test_caf_invalid_rut_or_tipo ... ok
test caf::tests::test_caf_tampered_da_fails_verification ... ok
test caf::tests::test_caf_keypair_mismatch ... ok
test caf::tests::test_caf_sign_ted_and_signer_trait ... ok
test caf::tests::test_caf_tampered_signature_fails_verification ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.66s

     Running tests/caf.rs (target/debug/deps/caf-2f71474bfb58d6d0)

running 6 tests
test test_fixture_caf_33_parse_and_fields ... ok
test test_fixture_caf_33_debug_redaction ... ok
test test_fixture_caf_33_folio_range_check ... ok
test test_fixture_caf_33_tampered_fails ... ok
test test_fixture_caf_33_signature_verification ... ok
test test_fixture_caf_33_ted_signing ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/smoke.rs (target/debug/deps/smoke-55369ca95524e34c)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/smoke.rs (target/debug/deps/smoke-f91cdbb1969d2477)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s

     Running tests/smoke.rs (target/debug/deps/smoke-3c89d4656b79ae60)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s

   Doc-tests sii
test crates/sii/src/caf.rs - caf (line 16) - compile ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

5. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features`
```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s
   Generated /Users/demian/Documents/Github/sii-sdk-pro/target/doc/sii/index.html and 2 other files
```

### Decisiones
1. **Preservación de bytes raw de `<DA>` y `<CAF>`:** Para verificar `FRMA` se preserva el slice exacto de bytes de `<DA>...</DA>` tal como viene en el XML, garantizando que diferencias sutiles de espacios no rompan la firma digital del SII. Análogamente, se preserva el bloque `<CAF>` para su posterior incrustación textual en el `<TED>`.
2. **Compatibilidad con codificación ISO-8859-1 y UTF-8:** El lector decodifica texto probando primero UTF-8 válido y cayendo a mapeo byte a byte (Latin-1) si contiene caracteres acentuados o especiales chilenos sin UTF-8.
3. **Llave pública del SII para tests:** Dado que `.gitignore` excluye `*.pem`, la llave pública de prueba del SII se nombró `sii-test-pubkey.pub` para permitir su versionado limpio como fixture sin violar las reglas de exclusión de secretos.

### Pendientes / bloqueos
- Ninguno. La tarea se completó en su totalidad.

### Desviaciones de la spec
- Ninguna. Cumple estrictamente con `docs/md/dte-spec.md` §5, `constraints.md` (CAF-01, SEC-01, SEC-02) y `public-api.md`.
