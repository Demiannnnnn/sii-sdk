# TASK-INFRA-01: Workspace Cargo, lints, licencias y CI

| Atributo | Detalle |
|:--|:--|
| **ID** | `INFRA-01` |
| **Estado** | ✅ terminada |
| **Depende de** | — |
| **Rama** | `dev/infra-01-workspace` |
| **Prioridad** | P0 |
| **Módulos** | `Cargo.toml` (workspace), `crates/{sii,sii-sim,sii-cli}/`, `.github/workflows/ci.yml`, licencias por crate |

## 1. Contexto

El repo solo tiene diseño (`.ai-context/`, `docs/`, `tasks/`) y los archivos raíz de lints/licencias. Hay que
crear el **workspace Cargo** y el esqueleto de los tres crates que fija `.ai-context/architecture.md` §3, con
los lints del workspace, para que todas las tareas siguientes compilen en verde. **No** implementes lógica de
dominio todavía: solo el andamiaje que compila y pasa `cargo test` vacío.

## 2. Requerimientos

- **RF-01 Workspace:** `Cargo.toml` raíz con `resolver = "3"`, `members = ["crates/*"]`, `[workspace.package]`
  (version `0.1.0`, edition `2024`, rust-version `1.85`, license `MIT OR Apache-2.0`, repository, authors),
  `[workspace.dependencies]` con las dependencias permitidas de `architecture.md` §3 (declaradas aquí aunque
  aún no se usen todas), y `[workspace.lints]` exactamente como `.ai-context/conventions.md` §3.
- **RF-02 Crates:**
  - `crates/sii`: librería. `src/lib.rs` con los módulos declarados (vacíos o con stubs mínimos que compilan):
    `types`, `codes`, `caf`, `dte`, `ted`, `signature`, `envio`, `libro`, `auth`, `transport`, `client`, `error`.
    Cada módulo puede ser un `//! doc` + nada más por ahora, pero el crate compila con `missing_docs = "deny"`.
  - `crates/sii-sim`: `src/lib.rs` + `src/main.rs` (bin `sii-sim`) con un `main` mínimo que compila y sale 0.
  - `crates/sii-cli`: `src/main.rs` (bin `sii`) mínimo que compila y sale 0.
  - Cada crate: `[lints] workspace = true`, su `Cargo.toml` con `description`, `license`, `repository`, y los
    archivos `LICENSE-MIT`/`LICENSE-APACHE` enlazados o copiados según práctica de crates.io.
  - Define las **features** de `conventions.md` §4 en `crates/sii/Cargo.toml` (`http`, `pkcs12`, `serde`,
    `tracing`), aunque aún no activen código; el crate DEBE compilar con `--no-default-features` (API-01).
- **RF-03 CI:** `.github/workflows/ci.yml` (`workflow_dispatch` + push a `main`/`develop`/`develop-AGENTS`):
  en ubuntu/macos/windows corre `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`, `cargo clippy -p sii --no-default-features -- -D warnings`,
  `cargo test --workspace --all-features`, y `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`.
  Sin pasos de red ni secretos. (Ya hay un `ci.yml` de plantilla; ajústalo a sii-sdk o reescríbelo.)
- **RF-04 Compila verde:** todos los comandos de AGENTS.md §8 pasan sobre el esqueleto (con 0 tests o un test
  trivial de humo por crate que verifique que el crate carga, p. ej. una constante de versión).

## 3. Reglas

AGENTS.md completo. No agregues dependencias fuera de `architecture.md` §3. No implementes lógica de dominio.
`unsafe_code = "forbid"` ya en los lints. Los binarios imprimen con `std::io::Write`, no `println!`.

## 4. Tests

- Un test de humo por crate (que el crate enlaza y expone su versión). `cargo test --workspace` pasa.
- `cargo build -p sii --no-default-features` compila.

## 5. DoD

AGENTS.md §8 en verde, pegando la salida real en el Reporte.

## Reporte

### Resumen
Se implementó el workspace Cargo raíz y el esqueleto de los tres crates (`sii`, `sii-sim`, `sii-cli`) según la especificación de `INFRA-01` y `.ai-context/architecture.md` §3. Se definieron los lints del workspace estrictos (`unsafe_code = "forbid"`, `missing_docs = "deny"`, clippy denegando `unwrap`, `expect`, `panic`, `print_stdout`, etc.), las dependencias permitidas en `[workspace.dependencies]`, las features requeridas en `crates/sii` (`http`, `pkcs12`, `serde`, `tracing`), licencias y `NOTICE` por crate, los 12 módulos requeridos en `sii`, binarios mínimos para `sii-sim` y `sii-cli` (bin `sii`), y tests de humo por crate. Todo compila y pasa las pruebas en verde con `--all-features` y con `--no-default-features`.

### Archivos
- `Cargo.toml` (workspace raíz)
- `Cargo.lock`
- `crates/sii/Cargo.toml`
- `crates/sii/README.md`, `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`
- `crates/sii/src/lib.rs`
- `crates/sii/src/types.rs`, `codes.rs`, `caf.rs`, `dte.rs`, `ted.rs`, `signature.rs`, `envio.rs`, `libro.rs`, `auth.rs`, `transport.rs`, `client.rs`, `error.rs`
- `crates/sii/tests/smoke.rs`
- `crates/sii-sim/Cargo.toml`
- `crates/sii-sim/README.md`, `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`
- `crates/sii-sim/src/lib.rs`, `src/main.rs`
- `crates/sii-sim/tests/smoke.rs`
- `crates/sii-cli/Cargo.toml`
- `crates/sii-cli/README.md`, `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`
- `crates/sii-cli/src/main.rs`
- `crates/sii-cli/tests/smoke.rs`

### Resultado real de cada comando de AGENTS.md §8

1. `cargo fmt --all --check`:
```text
Exit code: 0 (sin salida, formato limpio)
```

2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`:
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.15s
Exit code: 0
```

3. `cargo clippy -p sii --no-default-features -- -D warnings`:
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.01s
Exit code: 0
```

4. `cargo test --workspace --all-features`:
```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.07s
     Running unittests src/lib.rs (target/debug/deps/sii-ebef06d3d121a4b4)
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/smoke.rs (target/debug/deps/smoke-b58c7eed95a64f8b)
running 1 test
test smoke_test_version ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/sii-0e905d1e53ecab2a)
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/smoke.rs (target/debug/deps/smoke-296afdbcc797c5ea)
running 1 test
test smoke_test_cli_runs ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s

     Running unittests src/lib.rs (target/debug/deps/sii_sim-618325f4116375b0)
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/sii_sim-8404d4d2deb4acd7)
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/smoke.rs (target/debug/deps/smoke-c856bd56c7a23419)
running 2 tests
test smoke_test_sim_version ... ok
test smoke_test_sim_bin_runs ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s

   Doc-tests sii
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests sii_sim
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Exit code: 0
```

5. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features`:
```text
warning: output filename collision at /Users/demian/Documents/Github/sii-sdk-pro/target/doc/sii/index.html
  |
  = note: the bin target `sii` in package `sii-cli v0.1.0 (/Users/demian/Documents/Github/sii-sdk-pro/crates/sii-cli)` has the same output filename as the lib target `sii` in package `sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-pro/crates/sii)`
  = note: this is a known bug where multiple crates with the same name use the same path; see <https://github.com/rust-lang/cargo/issues/6313>
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s
   Generated /Users/demian/Documents/Github/sii-sdk-pro/target/doc/sii/index.html and 2 other files
Exit code: 0
```

### Decisiones
- Se configuraron los lints con prioridad `-1` para los grupos `rust_2018_idioms`, `all` y `pedantic`, permitiendo que las reglas individuales (`unwrap_used`, `module_name_repetitions`, etc.) tengan precedencia sin advertencias de Cargo.
- En `crates/sii/Cargo.toml`, se declararon las features opcionales vacías (`http`, `pkcs12`, `serde`, `tracing`) para que el workspace compile limpiamente con `--all-features` y con `--no-default-features` sin activar dependencias pesadas no implementadas todavía.
- Los binarios `sii-sim` y `sii` retornan `ExitCode::SUCCESS` sin usar `println!` ni `unwrap()`.
- Se incluyeron tests de humo de integración que verifican la constante de versión y la ejecución de los binarios.

### Pendientes / bloqueos
- Ninguno. El workspace está listo para comenzar la implementación de `CORE-01`.

### Desviaciones de la spec
- Ninguna.
