# Convenciones — sii-sdk

## 1. Idioma

| Qué | Idioma |
|:--|:--|
| Código: identificadores, archivos, commits, comentarios, mensajes de `Error`, rustdoc | Inglés |
| Términos del dominio en el código | Inglés (`issuer`, `folio`, `stamp`, `envelope`); se mantienen las siglas del SII en los tipos XML (`TED`, `DD`, `FRMT`, `Caratula`, `EnvioDTE`) |
| Nombres de elementos XML | Exactamente como los define el SII (no traducir) |
| Salida de la CLI y del simulador para humanos | Español de Chile (tuteo); `--json` con claves en inglés |
| Documentación (`docs/`, `.ai-context/`, `tasks/`, reportes) | Español |
| README de cada crate (crates.io) | Inglés, con enlace al README del repo |

## 2. Git y ramas (reglas del dueño — obligatorias)

| Rama | Quién | Regla |
|:--|:--|:--|
| `main` | **Nadie** | Prohibido: checkout para commitear, commit, merge, push, PR, rebase, borrar. |
| `develop` | **Nadie** | Igual que `main`. El dueño integra `develop-AGENTS → develop` manualmente. |
| `develop-AGENTS` | Solo el **orquestador** (Claude) | Rama de integración. Recibe merges `--no-ff` de `dev/*` validados. |
| `dev/*` | Agentes y orquestador | Libres. Ej.: `dev/infra-01-workspace`, `dev/dte-01-model`, `dev/fix-<tema>`, `dev/docs-<tema>`. |

- Cada tarea se trabaja en `dev/<id-en-minúsculas>-<tema>` creada desde `develop-AGENTS`.
- Commits: Conventional Commits en inglés con scope = módulo o crate, y referencia a la tarea en el cuerpo:
  `feat(dte): serialize factura afecta to SII XML` + `Refs: TASK-DTE-02`.
  Tipos: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `build`, `ci`, `perf`.
- Commits pequeños; nunca commitear `target/` ni secretos.

## 3. Rust

- Edición 2024, MSRV 1.85. `cargo fmt` por defecto.
- **Lints del workspace** (`[workspace.lints]`, cada crate con `[lints] workspace = true`):
  - rust: `unsafe_code = "forbid"`, `missing_docs = "deny"`, `rust_2018_idioms = "deny"`, `unreachable_pub = "warn"`.
  - clippy: `all = "deny"`, `pedantic = "warn"` (prioridad -1), `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `indexing_slicing`, `float_arithmetic`, `dbg_macro`, `print_stdout`, `print_stderr` = `"deny"`; `module_name_repetitions = "allow"` (único allow).
  - `clippy.toml`: `allow-unwrap-in-tests`, `allow-expect-in-tests`, `allow-panic-in-tests`, `allow-indexing-slicing-in-tests` = `true`.
  - Los binarios (CLI, simulador) imprimen vía `std::io::Write` sobre `stdout().lock()`, no `println!`.
- Errores: `thiserror` en la librería; nada de `Box<dyn Error>` en la API salvo `Error::Transport`. Los binarios convierten a mensaje y `ExitCode`.
- Dinero: tipo `Monto` entero (CLP); NUNCA `f64` para montos (DTE-03). El lint `float_arithmetic` lo refuerza.
- Visibilidad mínima: `pub(crate)` por defecto; `pub` solo lo de `public-api.md`.
- Rustdoc: cada ítem público con descripción, `# Errors` en funciones que devuelven `Result`, ejemplo en los principales.
- Nombres: tipos `PascalCase`, funciones `snake_case`, constantes `SCREAMING_SNAKE`. Códigos de tipo de DTE como `const`/enum en `codes`, nunca literales repartidos.
- Ninguna función de más de ~60 líneas; los parsers de XML usan helpers.
- **Secretos en `Debug`:** los tipos que contienen llave privada (CAF, certificado) implementan `Debug` redactando el secreto (SEC-02).

## 4. Features

| Feature | Default | Activa |
|:--|:--|:--|
| (núcleo) | sí | modelo DTE, TED, firma, envío, libro, auth — todo sans-IO |
| `http` | no | `HttpTransport` contra el simulador o el SII |
| `pkcs12` | no | cargar certificado/llave desde `.pfx`/`.p12` o PEM |
| `serde` | no | `Serialize`/`Deserialize` en los tipos públicos |
| `tracing` | no | eventos `tracing` (sin secretos, security-model) |

El crate `sii` DEBE compilar y pasar tests con `--no-default-features` (API-01).

## 5. Tests

| Capa | Dónde | Cómo |
|:--|:--|:--|
| types/codes (Rut, Monto, DV) | `#[cfg(test)] mod tests` junto al código | casos + `proptest` del módulo 11 |
| caf | `crates/sii/tests/caf.rs` + `tests/fixtures/caf-*.xml` | CAF de prueba autogenerado; rango, llave, FRMA |
| dte (XML por tipo) | `crates/sii/tests/dte_*.rs` + `tests/fixtures/dte-*.xml` | un fixture por tipo; comparación de XML canónico |
| ted | `crates/sii/tests/ted.rs` | `DD` byte a byte vs fixture; `FRMT` que verifica con la llave pública del CAF |
| signature | `crates/sii/tests/signature.rs` | firma que verifica; firma alterada que falla (SIG-02) |
| envio/libro | `crates/sii/tests/*.rs` + fixtures | sobre con carátula y resumen correctos |
| client | `crates/sii/tests/client_*.rs` (feature off: `MemoryTransport`; con `http`: contra `sii-sim`) | flujo semilla→token→envío→estado; rechazo como valor (ADR-04) |

- Hora y azar inyectados (`ManualClock`/semilla fija) para reproducibilidad (QA-03).
- Nunca `std::thread::sleep` para esperar; el simulador responde de inmediato o con tiempo simulado.

## 6. Mensajes de error y salida

- `Error` de la librería: variantes tipadas (`InvalidRut`, `FolioOutOfRange`, `CafSignature`, `Xml`, `Signature`, `Transport`, …), en inglés, sin exponer secretos.
- La CLL imprime en español, con `--json` opcional (claves en inglés) y códigos de salida estables.

## 7. Documentación y versión

- `CHANGELOG.md` Keep a Changelog; versión en `[workspace.package]`. La publicación la hace el dueño (`docs/md/release.md`).

## 8. Formato del reporte de tarea

En `## Reporte` del archivo de la tarea: **Resumen** · **Archivos** · **Resultado real de cada comando de AGENTS.md §8** (pegar la salida relevante) · **Decisiones** · **Pendientes/bloqueos** · **Desviaciones de la spec**. Nunca declarar "terminado" sin pegar la salida de los comandos.
