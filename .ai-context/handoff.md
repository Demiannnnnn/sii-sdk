# Handoff — sii-sdk

Léelo al retomar en otra sesión o equipo. Resumen vivo del estado y la forma de trabajo.

## Estado (2026-11-22)

- **11 tareas de código integradas** en `develop-AGENTS` (128 tests): INFRA-01, CORE-01, CAF-01, DTE-01, DTE-02,
  TED-01, SIG-01, AUTH-01, ENVIO-01, CERT-LOADER-01, LIBRO-01, TRANSPORT-01. Ver `implementation-status.md`.
- **Siguiente:** SIM-01 (contrato del simulador documentado en `implementation-status.md`) → CLI-01 → DOCS-01.
  CERT-99 lo hace el dueño con certificado y CAF reales.
- ENVIO-01 y CERT-LOADER-01 los implementó Claude por el respaldo de cuota (CLAUDE.md); el resto, agy.
- Lecciones de merge: ramas paralelas chocan de forma aditiva en `lib.rs`, `error.rs`, `Cargo.toml` y
  `Cargo.lock` — combinar sin duplicar variantes/exports y regenerar el lock con `cargo generate-lockfile`.
  Si el choque es estructural (métodos en `impl` distintos), relanzar la tarea desde el `develop-AGENTS` actual.
- El hook de gitleaks bloquea llaves en commits: los tests guardan material de prueba como base64 DER y
  reconstruyen el PEM en runtime.

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
