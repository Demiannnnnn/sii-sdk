# TASK-CLI-01: CLI sii (armar, timbrar, firmar, enviar, estado)

| Atributo | Detalle |
|:--|:--|
| **ID** | `CLI-01` |
| **Estado** | ✅ terminada |
| **Depende de** | SIM-01 |
| **Rama** | `dev/cli-01-cli` |
| **Prioridad** | P2 |

## 1. Contexto
Bin `sii` (crate `sii-cli`). Herramienta de línea de comandos sobre el crate `sii`, contra el simulador o el SII real.
## 2. Requerimientos
- **RF-01 Comandos:** `dte build` (desde JSON/args), `dte timbrar` (con CAF), `dte firmar` (con certificado), `envio enviar` (contra sim o ambiente), `envio estado <trackid>`, `libro build`. Salida humana en español; `--json` opcional (claves en inglés).
- **RF-02 Secretos:** certificado desde archivo + `SII_CERT_PASSWORD` (env), CAF desde archivo; nunca en argumentos en claro (SEC-03). Ambiente obligatorio (`--ambiente certificacion|produccion`), sin default a producción.
- **RF-03 Salida/errores:** `std::io::Write` (no `println!`), `ExitCode` estables, errores en español sin filtrar secretos.
## 3. Tests
Comandos principales contra el simulador (end-to-end de armar→timbrar→firmar→enviar→estado) con certificado/CAF de prueba.
## 4. DoD
AGENTS.md §8.
## Reporte

### Resumen
Implementación completa de la CLI `sii` (crate `sii-cli`, binario `sii`) para Documentos Tributarios Electrónicos (DTE) del SII de Chile:
- Subcomandos implementados bajo `sii`:
  - `dte build`: Construye DTEs desde argumentos CLI o archivo JSON estructurado, exportando a XML o JSON.
  - `dte timbrar`: Aplica Timbre Electrónico DTE (`<TED>`) con verificación de rango de folios CAF y firma RSA-SHA1.
  - `dte firmar`: Firma documentos DTE individuales con XMLDSig enveloped (`<Signature>`).
  - `envio enviar`: Construye el sobre de envío (`EnvioDTE` o `EnvioBOLETA`), lo firma, obtiene token de autenticación del SII con certificado y sube el sobre (soportando simulación en memoria `--sim`, simulación HTTP loopback `--sim-url <URL>` o ambiente real).
  - `envio estado <trackid>`: Consulta el estado de procesamiento de un envío al SII autenticándose con certificado.
  - `libro build`: Construye y firma libros de compra/venta electrónicos (IECV / `LibroCompraVenta`), incorporando DTEs individuales como detalles.
- Formato de salida: Salida humana en español chileno y formato `--json` estructurado con claves en inglés.
- Seguridad y manejo de secretos (SEC-02, SEC-03, RF-02, RF-03): Contraseña de certificados leída exclusivamente vía `SII_CERT_PASSWORD`. Sin filtrado de claves ni contraseñas en logs ni mensajes de error.
- Códigos de salida (`ExitCode`): `0` en éxito, `1` en error con mensaje descriptivo a `stderr`.

### Archivos
- `crates/sii-cli/Cargo.toml`
- `crates/sii-cli/src/main.rs`
- `crates/sii-cli/src/args.rs`
- `crates/sii-cli/src/error.rs`
- `crates/sii-cli/src/output.rs`
- `crates/sii-cli/src/commands/mod.rs`
- `crates/sii-cli/src/commands/cert.rs`
- `crates/sii-cli/src/commands/transport.rs`
- `crates/sii-cli/src/commands/xml_utils.rs`
- `crates/sii-cli/src/commands/dte.rs`
- `crates/sii-cli/src/commands/envio.rs`
- `crates/sii-cli/src/commands/libro.rs`
- `crates/sii-cli/tests/smoke.rs`
- `crates/sii-cli/tests/cli_tests.rs`
- `tasks/cli/task-01-cli.md`
- `Cargo.lock`

### Resultado real de cada comando de AGENTS.md §8

1. `cargo fmt --all --check`
```
Exit code: 0
(sin diferencias de formato)
```

2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
```
   Compiling sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w1/crates/sii)
   Compiling sii-sim v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w1/crates/sii-sim)
   Compiling sii-cli v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w1/crates/sii-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.85s
Exit code: 0
```

3. `cargo clippy -p sii --no-default-features -- -D warnings`
```
    Checking sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w1/crates/sii)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.54s
Exit code: 0
```

4. `cargo test --workspace --all-features`
```
     Running unittests src/lib.rs (sii)
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.22s

     Running tests/auth.rs (sii)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s

     Running tests/caf.rs (sii)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/cert_loader.rs (sii)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

     Running tests/client_transport.rs (sii)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/dte.rs (sii)
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s

     Running tests/envio.rs (sii)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

     Running tests/fixtures.rs (sii)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/libro.rs (sii)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

     Running tests/signature.rs (sii)
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s

     Running tests/ted.rs (sii)
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

     Running unittests src/main.rs (sii-cli)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/cli_tests.rs (sii-cli)
test test_cli_errors_and_exit_codes ... ok
test test_cli_libro_build_and_sign ... ok
test test_cli_live_http_server_simulation ... ok
test test_cli_end_to_end_pipeline ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.68s

     Running tests/smoke.rs (sii-cli)
test smoke_test_cli_runs ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src/lib.rs (sii-sim)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/simulator_tests.rs (sii-sim)
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.82s

     Running tests/smoke.rs (sii-sim)
test smoke_test_sim_version ... ok
test smoke_test_sim_bin_runs ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.85s

   Doc-tests sii
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

Exit code: 0
```

5. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features`
```
 Documenting sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w1/crates/sii)
 Documenting sii-sim v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w1/crates/sii-sim)
 Documenting sii-cli v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w1/crates/sii-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.30s
Exit code: 0
```

### Decisiones
- Implementación de salidas mediante bloqueos explícitos de `stdout().lock()` y `stderr().lock()` (RF-03) garantizando rendimiento y aislamiento de streams.
- Detección automática del contenedor de certificado (PKCS#12 vs PEM) según el prefijo `-----BEGIN`. Claves privadas se extraen automáticamente de archivos PEM autocontenidos o mediante `--cert-key`. Contraseña PKCS#12 leída obligatoriamente de la variable `SII_CERT_PASSWORD` sin opciones de línea de comandos en texto claro (SEC-03).
- Soporte para simulador en memoria (`--sim`), simulador HTTP local (`--sim-url <URL>`) y endpoints oficiales del SII (`--ambiente certificacion|produccion`).
- Parámetro `--ambiente` es obligatorio para operaciones de red o firma; no se define un valor por defecto a producción para prevenir envíos no intencionados.

### Pendientes/bloqueos
- Ninguno. Todos los criterios de aceptación y tests pasan exitosamente.

### Desviaciones de la spec
- Ninguna. Cumplimiento exacto de los requerimientos RF-01, RF-02 y RF-03.
