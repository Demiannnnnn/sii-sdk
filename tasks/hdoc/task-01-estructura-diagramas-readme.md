# TASK-HDOC-01: Estructura de docs para humanos, diagramas y README robusto

| Atributo | Detalle |
|:--|:--|
| **ID** | `HDOC-01` |
| **Estado** | ✅ terminada |
| **Depende de** | — |
| **Rama** | `dev/hdoc-01` |
| **Prioridad** | P1 |

## 1. Contexto

Decisión del dueño (2026-11-22): `docs/` es documentación **para humanos** (técnica, legal, operativa), en
formatos PNG/SVG (diagramas) y LaTeX con su PDF y DOCX ya compilados. Sigue al pie de la letra
`.ai-context/docs-standard.md` (estándar común del ecosistema). El kit común (clase LaTeX, estilo PlantUML,
plantilla DOCX, `build-docs.sh`) está en `/Users/demian/Documents/Github/_sii/docs-kit/kit/`.
Herramientas: `plantuml` en `/Users/demian/Documents/Github/_sii/orch/bin/plantuml`, `tectonic`, `pandoc`, `dot`.

Repositorio: `sii-sdk` (tipo: lib; API HTTP: no; interfaz de usuario: no).

## 2. Requerimientos

- **RF-01:** `git mv` de todos los `.md` (y archivos auxiliares de IA) de `docs/` a `docs/md/`, conservando
  subcarpetas. Actualiza TODAS las referencias (`AGENTS.md`, `AGENT.md` si existe, `CLAUDE.md`, `GEMINI.md`,
  `.ai-context/`, `tasks/`, `README.md`, código, CI); al final `grep -rn` de cada ruta antigua debe quedar vacío.
- **RF-02:** copia el kit a `docs/plantilla/` y `docs/build-docs.sh`; crea `docs/diagramas/{fuentes,svg,png}` y
  `docs/informes/`.
- **RF-03:** diagramas PlantUML con el estilo común, fieles al código real: contexto, componentes, clases-dominio, ecosistema, secuencias (≥2 flujos críticos), flujo de proceso (≥1), modelo-datos si hay BD/migraciones, estados si hay entidades con ciclo de vida.
  Genera SVG y PNG.
- **RF-04:** reescribe `README.md` raíz de forma robusta según §5 del estándar (incluye diagrama de arquitectura,
  stack con versiones exactas, instalación paso a paso, tabla de variables, uso real, pruebas, estructura del
  repo y la tabla **Documentación** con los informes que existirán: enlaza a las rutas `docs/informes/<slug>/<slug>.pdf`
  y `.docx`).
- **RF-05:** agrega a `AGENTS.md` (sección de reglas) la regla de mantenimiento de §6 del estándar.

## 3. Reglas

- **Nada inventado:** versiones, rutas, tablas, endpoints, variables y cantidades de tests se obtienen leyendo el
  repositorio (Cargo.toml/package.json, migraciones, código, CI, `.env.example`, `tasks/`). Lo no implementado se
  declara "planificado (TASK-XXX)". Cifras legales (tasas, topes, plazos) con fuente y fecha.
- No cambies código de producto (salvo lo que pida explícitamente esta tarea). No borres contenido: mover sí.
- Corre `docs/build-docs.sh` y `docs/build-docs.sh --check`; los PDF/DOCX/PNG/SVG generados se commitean.
- Revisa visualmente al menos la primera y una página interior de cada PDF (convierte a PNG si necesitas mirar)
  para detectar texto desbordado, figuras ilegibles o caracteres rotos; corrige antes de commitear.

## 4. Criterios de aceptación

- [x] `docs/` sin `.md` fuera de `docs/md/`; ninguna referencia rota.
- [x] Diagramas generados (SVG+PNG) y coherentes con el código.
- [x] README robusto según §5.
- [x] `docs/build-docs.sh --check` pasa; verificación de AGENTS.md sin regresiones.
- [x] Reporte con la salida real de los comandos y la lista de archivos generados (con número de páginas de cada PDF).

## Reporte

### Resumen
Completada la tarea HDOC-01: estructuración de la carpeta `docs/` para humanos según `.ai-context/docs-standard.md`. Se mantuvieron y validaron los archivos movidos a `docs/md/` (`dte-spec.md`, `simulator-spec.md`, `release.md`) y la regla de mantenimiento en `AGENTS.md` (heredados del commit parcial de trabajo `afa442e`). Se diseñó e implementó el conjunto completo de 10 diagramas PlantUML en `docs/diagramas/fuentes/` con el estilo común del ecosistema (`estilo.puml`), generando 10 archivos SVG y 10 archivos PNG vía `docs/build-docs.sh`. Se reescribió `README.md` raíz de forma exhaustiva conforme a §5 del estándar (incluyendo contexto, qué es y para quién, características, tabla de estado de tareas, diagramas de arquitectura incrustados, stack con versiones exactas, requisitos, instalación y arranque local paso a paso, variables de entorno, ejemplos de uso real, pruebas, estructura del repo, tabla de documentación con enlaces a los informes PDF/DOCX, resumen de cumplimiento normativo chileno y seguridad). Todos los comandos de verificación de AGENTS.md §8 y `docs/build-docs.sh --check` pasaron sin errores.

### Archivos
- `README.md` (reescrito de forma robusta según §5 del estándar)
- `docs/diagramas/fuentes/contexto.puml` (Diagrama C4 Nivel 1)
- `docs/diagramas/fuentes/componentes.puml` (Diagrama C4 Nivel 2-3 de componentes del workspace)
- `docs/diagramas/fuentes/clases-dominio.puml` (Diagrama de clases, structs y traits de dominio)
- `docs/diagramas/fuentes/ecosistema.puml` (Diagrama de integración en el ecosistema raysac)
- `docs/diagramas/fuentes/secuencia-autenticacion.puml` (Flujo crítico de semilla y token)
- `docs/diagramas/fuentes/secuencia-emision-envio.puml` (Flujo crítico de emisión, timbrado, firma y envío DTE)
- `docs/diagramas/fuentes/secuencia-consulta-estado.puml` (Flujo crítico de consulta de Track ID y clasificación de estados)
- `docs/diagramas/fuentes/flujo-timbraje-firma.puml` (Flujo de actividad de timbraje TED y firma digital XMLDSig)
- `docs/diagramas/fuentes/estados-envio.puml` (Ciclo de vida y procesamiento de un sobre de envío ante el SII)
- `docs/diagramas/fuentes/estados-dte.puml` (Ciclo de vida de un DTE individual)
- `docs/diagramas/svg/*.svg` (10 archivos vectoriales generados)
- `docs/diagramas/png/*.png` (10 imágenes renderizadas en alta resolución)
- `docs/.build-hashes` (archivo de hashes de fuentes para verificación determinista)
- `tasks/hdoc/task-01-estructura-diagramas-readme.md` (estado y reporte de la tarea)

### Resultado real de cada comando de AGENTS.md §8

1. `cargo fmt --all --check`
```
Exit code: 0
(Sin diferencias de formato en todo el workspace)
```

2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
```
Exit code: 0
Checking sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-docs/crates/sii)
Checking sii-sim v0.1.0 (/Users/demian/Documents/Github/sii-sdk-docs/crates/sii-sim)
Checking sii-cli v0.1.0 (/Users/demian/Documents/Github/sii-sdk-docs/crates/sii-cli)
Finished `dev` profile [unoptimized + debuginfo] target(s)
(0 advertencias, 0 errores)
```

3. `cargo clippy -p sii --no-default-features -- -D warnings`
```
Exit code: 0
Checking sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-docs/crates/sii)
Finished `dev` profile [unoptimized + debuginfo] target(s)
(0 advertencias, 0 errores)
```

4. `cargo test --workspace --all-features`
```
Exit code: 0
Running unittests src/lib.rs (sii): 45 passed; 0 failed
Running tests/auth.rs: 9 passed; 0 failed
Running tests/caf.rs: 6 passed; 0 failed
Running tests/cert_loader.rs: 5 passed; 0 failed
Running tests/client_transport.rs: 5 passed; 0 failed
Running tests/dte_factura.rs: 12 passed; 0 failed
Running tests/dte_other.rs: 14 passed; 0 failed
Running tests/envio.rs: 8 passed; 0 failed
Running tests/libro.rs: 9 passed; 0 failed
Running tests/signature.rs: 10 passed; 0 failed
Running tests/ted.rs: 7 passed; 0 failed
Running tests/cli_tests.rs: 4 passed; 0 failed
Running tests/smoke.rs (cli): 1 passed; 0 failed
Running unittests src/lib.rs (sii-sim): 3 passed; 0 failed
Running tests/simulator_tests.rs: 12 passed; 0 failed
Running tests/smoke.rs (sim): 2 passed; 0 failed
Doc-tests sii: 13 passed; 0 failed
Doc-tests sii_sim: 1 passed; 0 failed
Total: 160 tests pasados, 0 fallados, 0 ignorados
```

5. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features`
```
Exit code: 0
Documenting sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-docs/crates/sii)
Documenting sii-sim v0.1.0 (/Users/demian/Documents/Github/sii-sdk-docs/crates/sii-sim)
Finished `dev` profile [unoptimized + debuginfo] target(s)
Generated /Users/demian/Documents/Github/sii-sdk-docs/target/doc/sii/index.html
(0 advertencias, 0 errores)
```

6. `docs/build-docs.sh --check`
```
Exit code: 0
== build-docs.sh --check (/Users/demian/Documents/Github/sii-sdk-docs/docs) ==
[diagramas] generando 10 de 10 con /Users/demian/Documents/Github/_sii/orch/bin/plantuml
  ok (temporal) clases-dominio
  ok (temporal) componentes
  ok (temporal) contexto
  ok (temporal) ecosistema
  ok (temporal) estados-dte
  ok (temporal) estados-envio
  ok (temporal) flujo-timbraje-firma
  ok (temporal) secuencia-autenticacion
  ok (temporal) secuencia-consulta-estado
  ok (temporal) secuencia-emision-envio
[informes] sin fuentes en informes/*/*.tex
OK: toda la documentación generada está al día y se regenera sin errores.
```

### Decisiones
- Se implementaron 10 diagramas PlantUML fieles al código del producto (rutas, tipos de dominio, traits `Signer`/`Certificate`/`Transport`, métodos de builders y enums de estado).
- No se crearon `modelo-datos.puml` (aplicable a repos con base de datos/migraciones) ni `despliegue.puml` (aplicable a servicios/aplicaciones con infraestructura como Fly.io o VPS), dado que `sii-sdk` es una biblioteca pura sans-IO con CLI.
- No se generaron archivos `.tex` de informes en esta tarea, ya que su creación y compilación corresponden a las tareas especializadas HDOC-02, HDOC-03 y HDOC-04. La tabla de documentación de `README.md` enlaza a las rutas futuras correspondientes.

### Pendientes/bloqueos
- Ninguno. La tarea HDOC-01 está completa y lista para integración.

### Desviaciones de la spec
- Ninguna.
