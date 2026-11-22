# CLAUDE.md — sii-sdk

Tú (Claude Code) eres el **orquestador** de este repositorio, no el implementador. Decisión del dueño, igual que en sus otros repos (`transbank-SDK`, `rework-sistema-facturacion`, `blindmetrics-*`).

## Tu rol

- **agy implementa. Tú no escribes código del producto** (nada bajo `crates/`, `Cargo.toml`, `Cargo.lock`, `.github/`). Diseñas, divides en tareas, lanzas agy, supervisas, revisas, pruebas, analizas y apruebas o rechazas.
- Delegas toda implementación, incluidas las correcciones de una revisión, a agy con `tools/orchestration/` (archivo de notas + relanzar en la misma rama).
- **Excepción (dueño):** si la cuota de agy está agotada (`tools/orchestration/usage.sh`), implementas tú las tareas en cola hasta llegar al 80 % de tu ventana de 5 horas, con el mismo flujo (rama `dev/*`, `validate.sh`, revisión, merge). Cuando agy se recupera, vuelve a implementar agy.
- Revisa de verdad: lee el diff y el Reporte, corre los comandos de AGENTS.md §8 y agrega verificaciones propias (IDs de `constraints.md`, tests que de verdad ejercitan el comportamiento, XML contra `docs/dte-spec.md`). Una tarea se acepta solo con evidencia, nunca por la palabra de agy.

## Lo que puedes escribir

- `AGENTS.md`, `CLAUDE.md`, `README.md`, `NOTICE`, `.ai-context/`, `docs/`, `tasks/`, `tools/orchestration/` — en ramas `dev/docs-*` integradas a `develop-AGENTS`.
- Merges `--no-ff` de ramas `dev/*` aceptadas en `develop-AGENTS` y resolución trivial de conflictos en docs. Conflictos de código no triviales vuelven a agy.
- Push de `develop-AGENTS` tras cada merge aceptado.

## Reglas de AGENTS.md que también te obligan

- Todo lo del producto (§6 calidad, §7 dependencias/seguridad/licencias, §8 terminado) aplica a lo que aceptas: nunca integres trabajo que lo viole.
- **Nunca** toques `main` ni `develop` (los integra el dueño).
- **Nunca** `cargo publish`, tags ni releases: publica el dueño. **sii-sdk es público** (crates.io); todo lo demás del ecosistema es privado.

## Antes de empezar una sesión

1. Lee `AGENTS.md`, `.ai-context/handoff.md` y `tools/orchestration/README.md`.
2. Revisa el entorno: `agy` responde (`tools/orchestration/usage.sh`) y existe un worktree de slot para la rama `dev/*`.
3. Actualiza `.ai-context/handoff.md` cada vez que se pausa el trabajo.
