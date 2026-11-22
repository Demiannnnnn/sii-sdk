# TASK-SIG-01: Firma XMLDSig (Signer/Certificate, C14N)

| Atributo | Detalle |
|:--|:--|
| **ID** | `SIG-01` |
| **Estado** | ✅ terminada |
| **Depende de** | DTE-01 |
| **Rama** | `dev/sig-01-xmldsig` |
| **Prioridad** | P0 |

## 1. Contexto
`docs/md/dte-spec.md` §6, `constraints.md` (SIG-01..03), ADR-02. La firma se inyecta vía trait; el núcleo no abre certificados.

## 2. Requerimientos
- **RF-01 Traits:** `trait Signer` (firma bytes con la llave del emisor) y `trait Certificate` (expone el X.509). El núcleo no toca el SO; un impl de prueba (llave RSA de juguete) vive en tests.
- **RF-02 Firmar:** firma **enveloped** XMLDSig sobre `<Documento>` con C14N determinista, digest y `SignatureMethod` que acepta el SII. Fija el algoritmo exacto por ambiente desde `docs/md/dte-spec.md` §6 y déjalo configurable; `KeyInfo` con el certificado. Reproducible con llave+hora fijas.
- **RF-03 Verificar:** `verify(&[u8]) -> Result<bool, Error>` real; firma/digest alterados → `false` (SIG-02).
- **RF-04 Fixtures:** documento firmado de prueba; test de ida y vuelta (firmar→verificar) y de detección de alteración.

## 3. Tests
Firma reproducible; verificación ok; alteración detectada; C14N estable ante reordenamiento irrelevante de espacios.

## 4. DoD
AGENTS.md §8.

## Reporte

### Resumen
Implementación completa de la especificación de firma digital XMLDSig y canonicalización C14N 1.0 (REC-xml-c14n-20010315) para Documentos Tributarios Electrónicos (DTE) del SII:
- Traits sans-IO `Signer` (firma digital abstracta) y `Certificate` (exposición de X.509 y llave pública).
- Estructura `X509Certificate` para deserialización de certificados en formatos DER y PEM.
- Enums y opciones configurables `DigestAlgorithm` (`Sha1`, `Sha256`), `SignatureAlgorithm` (`RsaSha1`, `RsaSha256`), y `SignatureOptions::for_ambiente`.
- Canonicalización determinista (`canonicalize`, `canonicalize_node`) con ordenamiento lexicográfico de atributos y namespaces, expansión de elementos vacíos a pares inicio-fin (`<tag></tag>`), normalización de caracteres/escapes y soporte de la transformación `enveloped-signature`.
- Firma `enveloped` sobre `<Documento ID="...">` (`sign`, `sign_dte`, `sign_dte_with_options`) generando bloques estándar `<Signature>`, `<SignedInfo>`, `<SignatureValue>` y `<KeyInfo>` con `<KeyValue>` y `<X509Data>`.
- Verificación real (`verify(&[u8]) -> Result<bool, Error>`) de integridad de digest y firma criptográfica RSA.
- Fixtures de prueba (`dte-33-signed.xml`, `test-cert.txt`) y suite de pruebas de integración (`crates/sii/tests/signature.rs`) validando reproducibilidad, verificación exitosa, detección de alteración de payload/digest/firma y estabilidad ante reordenamientos de C14N.

### Archivos
- `Cargo.lock`
- `crates/sii/Cargo.toml`
- `crates/sii/src/error.rs`
- `crates/sii/src/lib.rs`
- `crates/sii/src/signature.rs`
- `crates/sii/tests/fixtures/dte-33-signed.xml`
- `crates/sii/tests/fixtures/test-cert.txt`
- `crates/sii/tests/signature.rs`

### Resultado real de cada comando de AGENTS.md §8

#### 1. `cargo fmt --all --check`
```
$ cargo fmt --all --check
(exit code: 0, sin diferencias de formato)
```

#### 2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
```
$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.12s
```

#### 3. `cargo clippy -p sii --no-default-features -- -D warnings`
```
$ cargo clippy -p sii --no-default-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s
```

#### 4. `cargo test --workspace --all-features`
```
$ cargo test --workspace --all-features
   Compiling sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii)
   Compiling sii-sim v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii-sim)
   Compiling sii-cli v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii-cli)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.63s
     Running unittests src/lib.rs (target/debug/deps/sii-7961c7452d7b78b1)

running 30 tests
test caf::tests::test_caf_invalid_ranges ... ok
test caf::tests::test_caf_invalid_rut_or_tipo ... ok
test caf::tests::test_caf_parse_and_verify ... ok
test codes::tests::test_ambiente_from_str ... ok
test codes::tests::test_ambiente_methods ... ok
test codes::tests::test_tipo_dte_boleta_vs_dte ... ok
test codes::tests::test_tipo_dte_invalid_code ... ok
test codes::tests::test_tipo_dte_requires_reference ... ok
test codes::tests::test_tipo_dte_u16_roundtrip ... ok
test signature::tests::test_c14n_namespace_propagation_on_signed_info ... ok
test signature::tests::test_c14n_whitespace_and_attribute_reordering ... ok
test caf::tests::test_folio_range_containment ... ok
test caf::tests::test_caf_debug_redacts_private_key ... ok
test caf::tests::test_caf_parse_valid ... ok
test caf::tests::test_caf_tampered_da_fails_verification ... ok
test types::tests::test_monto_iva_19_exact ... ok
test types::tests::test_monto_overflow_safety ... ok
test types::tests::test_monto_parsing ... ok
test types::tests::test_rut_canonical_display_and_dots ... ok
test types::tests::test_rut_invalid_cases ... ok
test types::tests::test_rut_invalid_dv_rejected_proptest ... ok
test types::tests::test_rut_modulo_11_proptest ... ok
test types::tests::test_rut_valid_cases ... ok
test signature::tests::test_sign_and_verify_roundtrip_sha1 ... ok
test caf::tests::test_caf_sign_ted_and_signer_trait ... ok
test caf::tests::test_caf_keypair_mismatch ... ok
test signature::tests::test_verify_detects_payload_tampering ... ok
test signature::tests::test_verify_detects_signature_tampering ... ok
test caf::tests::test_caf_tampered_signature_fails_verification ... ok
test signature::tests::test_sign_and_verify_roundtrip_sha256 ... ok

test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.81s

     Running tests/caf.rs (target/debug/deps/caf-b8e9a4f15f313178)

running 6 tests
test test_fixture_caf_33_parse_and_fields ... ok
test test_fixture_caf_33_debug_redaction ... ok
test test_fixture_caf_33_folio_range_check ... ok
test test_fixture_caf_33_tampered_fails ... ok
test test_fixture_caf_33_signature_verification ... ok
test test_fixture_caf_33_ted_signing ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/dte_model.rs (target/debug/deps/dte_model-2177460332713ca4)

running 9 tests
test test_builder_rejects_invalid_rut ... ok
test test_monto_no_f64_integer_precision ... ok
test test_factura_afecta_with_mixed_items ... ok
test test_quick_xml_roundtrip_verification ... ok
test test_builder_validation_errors ... ok
test test_serde_json_roundtrip ... ok
test test_serialize_dte_33_against_fixture ... ok
test test_serialize_dte_34_against_fixture ... ok
test test_factura_afecta_iva_exact_rounding ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/signature.rs (target/debug/deps/signature-24a15392c8a327ee)

running 8 tests
test test_fixture_cert_parsing_and_public_key ... ok
test test_verify_fixture_signed_dte ... ok
test test_tamper_signature_value_detection ... ok
test test_tamper_digest_value_detection ... ok
test test_c14n_insensitivity_to_irrelevant_whitespace ... ok
test test_sign_dte_reproducible_and_verifiable ... ok
test test_sha256_signature_algorithm ... ok
test test_tamper_payload_detection ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.01s

     Running tests/smoke.rs (target/debug/deps/smoke-bf46b271f41c8782)

running 1 test
test smoke_test_version ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/sii-a4f62bd789dfe9cd)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/smoke.rs (target/debug/deps/smoke-6f98ffd59483b357)

running 1 test
test smoke_test_cli_runs ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s

     Running unittests src/lib.rs (target/debug/deps/sii_sim-d7b013bc94c481c7)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/sii_sim-07e10c7872e35e75)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/smoke.rs (target/debug/deps/smoke-2de69f320e8f4292)

running 2 tests
test smoke_test_sim_version ... ok
test smoke_test_sim_bin_runs ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s

   Doc-tests sii

running 1 test
test crates/sii/src/caf.rs - caf (line 16) - compile ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

all doctests ran in 1.33s; merged doctests compilation took 0.50s
   Doc-tests sii_sim

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

#### 5. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features`
```
$ RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
    Checking sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii)
 Documenting sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii)
 Documenting sii-sim v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii-sim)
 Documenting sii-cli v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.51s
   Generated /Users/demian/Documents/Github/sii-sdk-plus/target/doc/sii/index.html and 2 other files
```

### Decisiones
- Se implementó C14N 1.0 (Canonical XML 20010315) canónico determinista para serializar y verificar tanto el elemento `<Documento ID="...">` como `<SignedInfo>`, omitiendo `<Signature>` enveloped cuando aplica.
- Se incluyeron en `<KeyInfo>` tanto `<KeyValue>` (con `RSAKeyValue` Modulus y Exponent) como `<X509Data>` (con `X509Certificate`), garantizando compatibilidad con esquemas SII y W3C XMLDSig.
- La verificación extrae la llave pública preferentemente desde `<X509Certificate>` y con fallback a `<RSAKeyValue>`.
- Las llaves privadas para tests se generan en memoria durante la ejecución (`generate_test_key_and_cert`), y el certificado X.509 de prueba ficticio se guarda en `test-cert.txt` (SEC-01, ADR-05).

### Pendientes/bloqueos
- Ninguno.

### Desviaciones de la spec
- Ninguna.
