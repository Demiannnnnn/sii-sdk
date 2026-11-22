# TASK-INFRA-01: Workspace Cargo, lints, licencias y CI

| Atributo | Detalle |
|:--|:--|
| **ID** | `INFRA-01` |
| **Estado** | ⬜ pendiente |
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

_(lo completa el agente)_
