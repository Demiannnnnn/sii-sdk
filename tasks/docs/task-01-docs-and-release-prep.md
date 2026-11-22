# TASK-DOCS-01: Docs de crates, release.md, CHANGELOG, preparación crates.io

| Atributo | Detalle |
|:--|:--|
| **ID** | `DOCS-01` |
| **Estado** | ✅ terminada |
| **Depende de** | CLI-01 |
| **Rama** | `dev/docs-01-release-prep` |
| **Prioridad** | P2 |

## 1. Contexto
sii-sdk es **público**: destino crates.io. Preparar la documentación y los metadatos para que el dueño publique.
## 2. Requerimientos
- **RF-01 README por crate** (inglés, enlace al README del repo), `keywords`/`categories`, `description`, `license`, `repository`, `readme` en cada `Cargo.toml`.
- **RF-02 `docs/release.md`:** pasos de publicación (`cargo publish --dry-run`, orden de crates, versión), que ejecuta el **dueño** (no el agente).
- **RF-03 `CHANGELOG.md`:** Keep a Changelog, versión inicial.
- **RF-04 Doctests/ejemplos** de los flujos principales en el rustdoc público.
## 3. Tests
`cargo doc` sin warnings; `cargo publish --dry-run -p sii` pasa; doctests verdes.
## 4. DoD
AGENTS.md §8. No publicar (lo hace el dueño).
## Reporte

### Resumen
Se completó la preparación de documentación y publicación para crates.io de todo el workspace `sii-sdk` según `DOCS-01`:
1. **RF-01 README por crate:** Se crearon README completos y detallados en inglés para los 3 crates (`crates/sii/README.md`, `crates/sii-sim/README.md`, `crates/sii-cli/README.md`), con descripción, badges, tablas de features, ejemplos de código y enlaces al README y repositorio principal. Se verificaron metadatos en `Cargo.toml` (`description`, `readme`, `keywords`, `categories`, `license`, `repository`). Se configuró `doc = false` en `[[bin]]` de `crates/sii-cli` para resolver la colisión de nombres en `cargo doc` entre la librería `sii` y el binario `sii`.
2. **RF-02 `docs/release.md`:** Se creó la guía de release en español para el dueño del repositorio detallando los pasos de verificación previa (DoD), preparación de versión, simulación `--dry-run`, orden estricto de publicación (1. `sii` -> 2. `sii-sim` -> 3. `sii-cli`) debido a dependencias entre crates en crates.io, creación de tags en Git y releases en GitHub, y verificación post-publicación.
3. **RF-03 `CHANGELOG.md`:** Se creó el archivo de cambios en la raíz con formato Keep a Changelog y SemVer 2.0.0, detallando todas las funcionalidades añadidas en la versión inicial `0.1.0` para los tres crates.
4. **RF-04 Doctests:** Se agregaron ejemplos ejecutables (doctests) en `crates/sii/src/lib.rs` (construcción de factura, aritmética monetaria con RUT, consulta de estado con `MemoryTransport`), `types.rs` (`compute_dv`, `Rut::parse`, `Monto::from_pesos`, `Monto::iva_19`), `dte.rs` (`FacturaAfectaBuilder`, `NotaCreditoBuilder`), `envio.rs` (`EnvioDte`), `libro.rs` (`LibroCompraVenta`), `signature.rs` (`verify`) y `crates/sii-sim/src/lib.rs` (flujo simulador + cliente). Todos los 14 doctests pasan en verde.

### Archivos
- `crates/sii-cli/Cargo.toml`
- `crates/sii/README.md`
- `crates/sii-sim/README.md`
- `crates/sii-cli/README.md`
- `docs/release.md`
- `CHANGELOG.md`
- `crates/sii/src/lib.rs`
- `crates/sii/src/types.rs`
- `crates/sii/src/dte.rs`
- `crates/sii/src/envio.rs`
- `crates/sii/src/libro.rs`
- `crates/sii/src/signature.rs`
- `crates/sii-sim/src/lib.rs`
- `tasks/docs/task-01-docs-and-release-prep.md`

### Resultado real de cada comando de AGENTS.md §8

1. `cargo fmt --all --check`:
```text
Exit code: 0 (código limpio, sin diffs)
```

2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`:
```text
    Checking sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w1/crates/sii)
    Checking sii-sim v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w1/crates/sii-sim)
    Checking sii-cli v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w1/crates/sii-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.88s
Exit code: 0
```

3. `cargo clippy -p sii --no-default-features -- -D warnings`:
```text
    Checking sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w1/crates/sii)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.23s
Exit code: 0
```

4. `cargo test --workspace --all-features`:
```text
     Running unittests src/lib.rs (target/debug/deps/sii-b1b2db1e9884a8b9)
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.13s
     Running tests/auth.rs (target/debug/deps/auth-b465498a04c42dc1)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.17s
     Running tests/caf.rs (target/debug/deps/caf-0514c646b452fc7e)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
     Running tests/cert_loader.rs (target/debug/deps/cert_loader-803c1133a08ef838)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
     Running tests/client_transport.rs (target/debug/deps/client_transport-a941179c8e8df5e0)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
     Running tests/dte_model.rs (target/debug/deps/dte_model-ffc9c99ec2ce38b4)
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/envio.rs (target/debug/deps/envio-aa7b2266e272fc5e)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.50s
     Running tests/libro.rs (target/debug/deps/libro-10f95d2b933a1a5e)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.89s
     Running tests/signature.rs (target/debug/deps/signature-f6f5ec1f9c24e279)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.20s
     Running tests/smoke.rs (target/debug/deps/smoke-8bc7d44697ff2128)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/ted.rs (target/debug/deps/ted-2223c21a4f0eb770)
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
     Running tests/cli_tests.rs (target/debug/deps/cli_tests-6c082715a6805f0e)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.73s
     Running tests/smoke.rs (target/debug/deps/smoke-5196869b1f15341e)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running unittests src/lib.rs (target/debug/deps/sii_sim-49f0dcefc595ba56)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/simulator_tests.rs (target/debug/deps/simulator_tests-6bdf48423418981b)
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 11.22s
     Running tests/smoke.rs (target/debug/deps/smoke-86fa4b09dd25e101)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
   Doc-tests sii
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
   Doc-tests sii_sim
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
Exit code: 0
```

5. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features`:
```text
 Documenting sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w1/crates/sii)
 Documenting sii-sim v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w1/crates/sii-sim)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.24s
   Generated target/doc/sii/index.html and 1 other file
Exit code: 0 (cero warnings)
```

6. `cargo publish --dry-run -p sii`:
```text
   Packaging sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w1/crates/sii)
   Verifying sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w1/target/package/sii-0.1.0)
   Compiling sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w1/target/package/sii-0.1.0)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.18s
   Uploading sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w1/crates/sii)
warning: aborting upload due to dry run
Exit code: 0
```

### Decisiones
- Se configuró `doc = false` en `[[bin]]` dentro de `crates/sii-cli/Cargo.toml` para resolver la advertencia conocida de colisión de nombre de archivo (`target/doc/sii/index.html`) entre la librería `sii` y el binario `sii`.
- Los doctests de `sii` y `sii-sim` se diseñaron para ejecutarse completamente en memoria utilizando `MemoryTransport` y `SimTransport::direct`, sin requerir archivos en disco ni conexión a red (cumpliendo estrictamente con ADR-01 y QA-01).
- `docs/release.md` destaca el orden estricto de publicación (1. `sii`, 2. `sii-sim`, 3. `sii-cli`), ya que `sii-sim` y `sii-cli` dependen de `sii` y crates.io debe indexar `sii` antes de que los demás puedan publicarse.

### Pendientes / bloqueos
- Ninguno en cuanto a código ni documentación. El workspace está listo para la certificación contra el ambiente real del SII (`tasks/cert/task-99-certification.md`), que corresponde ejecutar manualmente al dueño con su certificado digital y CAF reales.

### Desviaciones de la spec
- Ninguna.
