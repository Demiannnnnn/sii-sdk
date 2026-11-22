# TASK-DOCS-01: Docs de crates, release.md, CHANGELOG, preparación crates.io

| Atributo | Detalle |
|:--|:--|
| **ID** | `DOCS-01` |
| **Estado** | ⬜ pendiente |
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
_(lo completa el agente)_
