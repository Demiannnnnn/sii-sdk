# TASK-AUTH-01: Semilla y token del SII

| Atributo | Detalle |
|:--|:--|
| **ID** | `AUTH-01` |
| **Estado** | ✅ terminada |
| **Depende de** | SIG-01 |
| **Rama** | `dev/auth-01-semilla-token` |
| **Prioridad** | P1 |

## 1. Contexto
`docs/md/dte-spec.md` §7 (autenticación). Modelo sans-IO: construir la solicitud de token firmando la semilla; el transporte lo hace TRANSPORT-01.
## 2. Requerimientos
- **RF-01 Semilla:** tipos para la respuesta de semilla del SII y su parseo.
- **RF-02 Firma semilla:** `firmar_semilla(semilla, &dyn Signer) -> SolicitudToken` (XMLDSig sobre `getToken/item/Semilla`), reproducible.
- **RF-03 Token:** parseo de la respuesta de token; tipo `Token` con su valor (tratado como secreto en logs, SEC-02).
## 3. Tests
Firma de semilla reproducible y verificable; parseo de respuestas de token ok y con error.
## 4. DoD
AGENTS.md §8. `--no-default-features` ok (sans-IO).
## Reporte

### Resumen
- Se implementó la capa de autenticación sans-IO (`sii::auth`) conforme a la especificación del SII (`docs/md/dte-spec.md` §7, AUTH-01).
- **Semilla (RF-01):** Nuevo tipo `Semilla` para representar y parsear la semilla retornada por `CrSeed.jws`/`crTicket.getSeed`. Método `parse_xml` y `RespuestaSemilla` que maneja estados de éxito ("00") y error del SII, tolerante a namespaces (`SII:` o raíz estándar).
- **Firma Semilla (RF-02):** Implementación de `firmar_semilla` y `Semilla::firmar`. Genera el documento XML `<getToken><item><Semilla>...</Semilla></item></getToken>` con firma digital enveloped XMLDSig conforme a las exigencias del SII (algoritmo RSA-SHA1, C14N 1.0, `Reference URI=""`, `KeyInfo` con `RSAKeyValue` y certificado X.509 en base64). La firma es 100 % determinista y reproducible dados la misma semilla y clave privada.
- **Token (RF-03, SEC-02):** Nuevo tipo `Token` y `RespuestaToken` para parsear la respuesta de `GetTokenFromSeed.jws`. `Token` implementa `fmt::Debug` y `fmt::Display` con valor redactado (`[REDACTED]`), previniendo fugas accidentales en logs o trazas. Para construir headers de transporte se proveen `.as_str()` y `.valor()`.
- **Firma enveloped general:** Se agregó el helper `sign_enveloped` en `signature.rs` para firmar documentos XML completos con `URI=""` e inserción antes de `</getToken>`, totalmente compatible con `signature::verify`.
- **Manejo de errores:** Se añadió la variante `Error::Auth(String)` a `sii::error::Error`.

### Archivos
- `crates/sii/src/auth.rs`: Módulo completo de autenticación (tipos `Semilla`, `Token`, `SolicitudToken`, `RespuestaSemilla`, `RespuestaToken`, función `firmar_semilla`).
- `crates/sii/src/signature.rs`: Adición de `sign_enveloped` y soporte de `</getToken>` en closing candidates.
- `crates/sii/src/error.rs`: Adición de variante `Error::Auth(String)`.
- `crates/sii/src/lib.rs`: Re-exportación de tipos y funciones de `auth`.
- `crates/sii/tests/fixtures/semilla-resp-ok.xml`: Fixture de respuesta de semilla exitosa del SII.
- `crates/sii/tests/fixtures/semilla-resp-error.xml`: Fixture de respuesta de semilla con error del SII.
- `crates/sii/tests/fixtures/token-resp-ok.xml`: Fixture de respuesta de token exitosa del SII.
- `crates/sii/tests/fixtures/token-resp-error.xml`: Fixture de respuesta de token con error del SII.
- `crates/sii/tests/auth.rs`: Suite de tests de integración para auth (9 tests).
- `tasks/auth/task-01-semilla-token.md`: Actualización de estado y reporte.

### Resultado real de cada comando de AGENTS.md §8

1. `cargo fmt --all --check`
```
(exit code 0, sin diferencias)
```

2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
```
    Checking sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii)
    Checking sii-sim v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii-sim)
    Checking sii-cli v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.11s
```

3. `cargo clippy -p sii --no-default-features -- -D warnings`
```
    Checking sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s
```

4. `cargo test --workspace --all-features`
```
     Running unittests src/lib.rs (target/debug/deps/sii-7961c7452d7b78b1)
running 35 tests
test caf::tests::test_caf_parse_valid ... ok
test caf::tests::test_caf_parse_and_verify ... ok
test caf::tests::test_folio_range_containment ... ok
test codes::tests::test_ambiente_from_str ... ok
test codes::tests::test_ambiente_methods ... ok
test codes::tests::test_tipo_dte_boleta_vs_dte ... ok
test codes::tests::test_tipo_dte_invalid_code ... ok
test codes::tests::test_tipo_dte_requires_reference ... ok
test codes::tests::test_tipo_dte_u16_roundtrip ... ok
test signature::tests::test_c14n_namespace_propagation_on_signed_info ... ok
test signature::tests::test_c14n_whitespace_and_attribute_reordering ... ok
test caf::tests::test_caf_tampered_da_fails_verification ... ok
test caf::tests::test_caf_debug_redacts_private_key ... ok
test caf::tests::test_caf_keypair_mismatch ... ok
test signature::tests::test_sign_and_verify_roundtrip_sha1 ... ok
test ted::tests::test_clean_truncate ... ok
test signature::tests::test_verify_detects_payload_tampering ... ok
test caf::tests::test_caf_invalid_ranges ... ok
test signature::tests::test_sign_and_verify_roundtrip_sha256 ... ok
test caf::tests::test_caf_tampered_signature_fails_verification ... ok
test types::tests::test_monto_iva_19_exact ... ok
test types::tests::test_monto_overflow_safety ... ok
test types::tests::test_monto_parsing ... ok
test types::tests::test_rut_canonical_display_and_dots ... ok
test types::tests::test_rut_invalid_cases ... ok
test types::tests::test_rut_invalid_dv_rejected_proptest ... ok
test types::tests::test_rut_modulo_11_proptest ... ok
test types::tests::test_rut_valid_cases ... ok
test ted::tests::test_ted_signing_and_verification ... ok
test caf::tests::test_caf_invalid_rut_or_tipo ... ok
test caf::tests::test_caf_sign_ted_and_signer_trait ... ok
test ted::tests::test_clock_injection_reproducible_ted ... ok
test ted::tests::test_dd_canonical_xml_format ... ok
test signature::tests::test_verify_detects_signature_tampering ... ok
test ted::tests::test_ted_tampered_fails_verification ... ok
test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s

     Running tests/auth.rs (target/debug/deps/auth-7ad649947b1586bf)
running 9 tests
test test_malformed_xml_responses ... ok
test test_parse_token_fixture_error ... ok
test test_parse_semilla_fixture_error ... ok
test test_token_confidentiality_redaction_in_debug_and_display ... ok
test test_parse_token_fixture_ok ... ok
test test_parse_semilla_fixture_ok ... ok
test test_firmar_semilla_reproducible_and_verifiable ... ok
test test_firmar_semilla_tampered_fails_verification ... ok
test test_firmar_empty_seed_fails ... ok
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.80s

     Running tests/caf.rs (target/debug/deps/caf-b8e9a4f15f313178)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/dte_model.rs (target/debug/deps/dte_model-2177460332713ca4)
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/signature.rs (target/debug/deps/signature-24a15392c8a327ee)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.16s

     Running tests/smoke.rs (target/debug/deps/smoke-bf46b271f41c8782)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/ted.rs (target/debug/deps/ted-25bd66124444ba1c)
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/smoke.rs (target/debug/deps/smoke-6f98ffd59483b357)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s

     Running tests/smoke.rs (target/debug/deps/smoke-2de69f320e8f4292)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s

   Doc-tests sii
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

5. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features`
```
    Checking sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii)
 Documenting sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii)
 Documenting sii-sim v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii-sim)
 Documenting sii-cli v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.59s
   Generated /Users/demian/Documents/Github/sii-sdk-plus/target/doc/sii/index.html and 2 other files
```

### Decisiones
1. **Redacción de `Token` (SEC-02):** Tanto `fmt::Debug` como `fmt::Display` imprimen `[REDACTED]` para evitar que llamadas inadvertidas a logging, formateo de cadenas o `tracing` expongan el token de sesión. El acceso al valor real se efectúa a través de `.as_str()` o `.valor()`.
2. **Estructura XMLDSig de `getToken`:** La firma enveloped exigida por el SII para `getToken` utiliza `Reference URI=""` sobre la raíz del documento. Se añadió el método `sign_enveloped` en `signature.rs` para permitir firmas enveloped de documentos completos con C14N determinista y verificación completa vía `signature::verify`.
3. **Manejo de estados SII:** Respuestas con `ESTADO != "00"` son preservadas en `RespuestaSemilla` y `RespuestaToken` para inspección de dominio, mientras que los métodos de extracción (`.semilla()`, `.into_semilla()`, `.token()`, `.into_token()`) devuelven `Err(Error::Auth(...))` con la glosa y código de estado correspondientes.

### Pendientes / bloqueos
- Ninguno. La tarea está completamente terminada y lista para integración.

### Desviaciones de la spec
- Ninguna.
