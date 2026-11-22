# Handoff — sii-sdk

Léelo al retomar en otra sesión o equipo. Resumen vivo del estado y la forma de trabajo.

## Estado (2026-11-21)

- Repositorio **recién creado** por el orquestador. Hecho: andamiaje de diseño completo
  (`AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, `.ai-context/` completo, `docs/dte-spec.md`, `tasks/` con el plan
  maestro y los archivos de tarea, licencias MIT/Apache, `rustfmt.toml`, `clippy.toml`, `.gitignore`, CI,
  `tools/orchestration/`). Rama de integración: `develop-AGENTS`.
- **Pendiente:** todo el código del producto. Siguiente tarea: **INFRA-01** (crear el workspace Cargo y los
  tres crates con lints y licencias). Luego CORE-01 → CAF-01 → DTE-01/02 → TED-01 → SIG-01 → ENVIO-01 →
  AUTH-01 → TRANSPORT-01 → SIM-01 → LIBRO-01 → CLI-01 → CERT-LOADER-01 → DOCS-01. Ver `tasks/README.md`.

## Forma de trabajo

- **Claude orquesta, agy implementa** (`CLAUDE.md`). Claude no escribe código del producto.
- Flujo por tarea: el orquestador crea `dev/<id>-<tema>` desde `develop-AGENTS` en un worktree de slot, lanza
  agy con `tools/orchestration/`, revisa (diff + Reporte + comandos de AGENTS.md §8 + IDs de `constraints.md`),
  y si pasa, hace merge `--no-ff` en `develop-AGENTS` y push.
- Si una revisión encuentra problemas, se devuelve a agy con un archivo de notas en la misma rama (una ronda),
  y se re-revisa.

## Entorno

- `tools/orchestration/usage.sh` para la cuota de agy; `pipeline.sh`/`review_agy.sh`/`validate.sh` como en los
  repos hermanos. Sin PostgreSQL ni servicios: es una librería pura, los tests corren con `cargo test`.
- **sii-sdk es público** (destino crates.io). Todo lo demás del ecosistema raysac es privado.

## Riesgos / cosas a vigilar

- Fidelidad del XML del SII: los fixtures deben venir de ejemplos oficiales del SII (citar la fuente), no
  inventados. Si un detalle del esquema no está en `docs/dte-spec.md`, agy debe reportarlo, no inventarlo.
- RSA-SHA1 para el TED y la semilla es exigencia del SII (no es elección de seguridad del SDK): no "mejorar" a
  SHA-256 ahí. La firma del DTE/Envío sí usa el algoritmo que el SII acepte (ver `docs/dte-spec.md`).
- Certificación real contra el SII: la hace el dueño (CERT-01); nunca se automatiza con secretos reales.
