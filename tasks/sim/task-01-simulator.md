# TASK-SIM-01: Simulador del SII (sii-sim)

| Atributo | Detalle |
|:--|:--|
| **ID** | `SIM-01` |
| **Estado** | ✅ terminada |
| **Depende de** | TRANSPORT-01 |
| **Rama** | `dev/sim-01-simulator` |
| **Prioridad** | P1 |

## 1. Contexto
Gemelo del simulador de transbank-SDK. Imita los endpoints del SII para desarrollar y testear sin el SII real.
## 2. Requerimientos
- **RF-01 Endpoints:** semilla, token, upload (devuelve TrackID), consulta de estado. Respuestas deterministas y configurables por escenario (aceptado, rechazado, en proceso, token inválido).
- **RF-02 lib + bin:** `sii-sim` como lib (para tests, implementando/complementando `Transport`) y bin `sii-sim` (servidor local TCP/HTTP sobre 127.0.0.1).
- **RF-03 Escenarios:** definidos en datos (JSON/código), documentados en `docs/md/simulator-spec.md` (crear este doc).
- **RF-04 Sin red externa:** solo 127.0.0.1 (QA-01). El bin imprime con `std::io::Write`.
## 3. Tests
`client` contra el simulador cubriendo aceptado/rechazado/en proceso; estado por TrackID.
## 4. DoD
AGENTS.md §8.
## Reporte

### Resumen
Se implementó completamente el simulador de servicios del SII `sii-sim` (biblioteca sans-IO y binario servidor HTTP local sobre `127.0.0.1`) según los requerimientos RF-01..04 de SIM-01:
- **RF-01 Endpoints:** Se implementaron los 4 endpoints principales del SII (`CrSeed.jws` GET, `GetTokenFromSeed.jws` POST, `DTEUpload` POST multipart con cookie, `QueryEstUp.jws` POST SOAP). Las respuestas son deterministas y fieles a los esquemas y fixtures oficiales del SII, con respuestas configurables por escenario (aceptado, rechazado, en proceso, token inválido, error de subida, error de semilla).
- **RF-02 lib + bin:** `sii-sim` exporta `SiiSimulator` que implementa `sii::transport::Transport` directamente (sans-IO para tests inmediatos en memoria), `SimTransport` (modo directo o HTTP con reescritura de endpoints), `HttpServer` (servidor TCP/HTTP sobre loopback) y el binario CLI `sii-sim`.
- **RF-03 Escenarios:** Enum `Scenario` con soporte de parsing de CLI y deserialización, `ScenarioConfig` con soporte JSON completo (`from_json` / `to_json_pretty`) y sobreescritura de estados por Track ID (`track_statuses`). Se documentó la especificación técnica completa en `docs/md/simulator-spec.md`.
- **RF-04 Sin red externa:** Validación estricta que rechaza cualquier dirección que no sea loopback `127.0.0.1` (QA-01). El binario imprime con `std::io::Write` sobre `stdout().lock()` sin macros de impresión directa.

### Archivos
- `Cargo.lock`
- `crates/sii-sim/Cargo.toml`
- `crates/sii-sim/src/lib.rs`
- `crates/sii-sim/src/main.rs`
- `crates/sii-sim/src/error.rs`
- `crates/sii-sim/src/scenario.rs`
- `crates/sii-sim/src/simulator.rs`
- `crates/sii-sim/src/http.rs`
- `crates/sii-sim/src/transport.rs`
- `crates/sii-sim/tests/simulator_tests.rs`
- `crates/sii-sim/tests/smoke.rs`
- `docs/md/simulator-spec.md`
- `tasks/sim/task-01-simulator.md`

### Resultado real de cada comando de AGENTS.md §8

1. `cargo fmt --all --check`:
```text
Exit code: 0 (formato limpio sin diferencias)
```

2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`:
```text
    Checking sii-sim v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w1/crates/sii-sim)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.03s
Exit code: 0
```

3. `cargo clippy -p sii --no-default-features -- -D warnings`:
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s
Exit code: 0
```

4. `cargo test --workspace --all-features`:
```text
     Running unittests src/lib.rs (target/debug/deps/sii-b1b2db1e9884a8b9)
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.88s

     Running tests/auth.rs (target/debug/deps/auth-b465498a04c42dc1)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.81s

     Running tests/caf.rs (target/debug/deps/caf-0514c646b452fc7e)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/cert_loader.rs (target/debug/deps/cert_loader-803c1133a08ef838)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

     Running tests/client_transport.rs (target/debug/deps/client_transport-a941179c8e8df5e0)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

     Running tests/dte_model.rs (target/debug/deps/dte_model-6fbe36365ae76f80)
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/envio.rs (target/debug/deps/envio-aa7b2266e272fc5e)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.22s

     Running tests/libro.rs (target/debug/deps/libro-10f95d2b933a1a5e)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 15.44s

     Running tests/signature.rs (target/debug/deps/signature-f6f5ec1f9c24e279)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.07s

     Running tests/smoke.rs (target/debug/deps/smoke-59a003c9fd437373)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/ted.rs (target/debug/deps/ted-ed6ff1c9ce3db21e)
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running unittests src/main.rs (target/debug/deps/sii-f26be68829ae8903)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/smoke.rs (target/debug/deps/smoke-cd91e8dffd563c73)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.43s

     Running unittests src/lib.rs (target/debug/deps/sii_sim-49f0dcefc595ba56)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/sii_sim-5ef7ede8c9924669)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/simulator_tests.rs (target/debug/deps/simulator_tests-6bdf48423418981b)
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.57s

     Running tests/smoke.rs (target/debug/deps/smoke-86fa4b09dd25e101)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s

   Doc-tests sii
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests sii_sim
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

Total general del workspace: 142 tests unitarios y de integración + 1 doctest, todos en verde (0 fallos).
Exit code: 0
```

5. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features`:
```text
 Documenting sii-sim v0.1.0 (/Users/demian/Documents/Github/sii-sdk-w1/crates/sii-sim)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.91s
Exit code: 0
```

6. Verificación de features por defecto de `sii-sim`:
```text
$ cargo clippy -p sii-sim -- -D warnings
Exit code: 0
$ cargo test -p sii-sim
Exit code: 0 (16 tests pasados: 3 lib, 0 main, 11 tests integración, 2 smoke tests)
```

### Decisiones
- `SiiSimulator` implementa directamente `sii::transport::Transport`, permitiendo que cualquier `SiiClient` lo use directamente en tests sin sobrecoste de red ni creación de sockets.
- `HttpServer` usa `std::net::TcpListener` nativo sin frameworks HTTP pesados, manteniendo el stack cerrado y 100% libre de `unsafe`.
- Se valida y rechaza explícitamente cualquier dirección que no sea de loopback (`127.0.0.1` / `::1`), garantizando el cumplimiento de QA-01 y RF-04.
- Los endpoints se resuelven por coincidencia de sufijo de ruta, funcionando transparentemente con URLs canónicas del SII (`https://maullin.sii.cl/...`) o relativas/locales (`/DTEWS/...`).
- En `crates/sii-sim/tests/smoke.rs`, se agregó `.arg("--help")` a `smoke_test_sim_bin_runs` para que el test de humo de binario verifique la compilación y ejecución de la CLI sin bloquearse esperando conexiones en el listener de red.
- En `crates/sii-sim/src/transport.rs`, se anotó `#[cfg(any(feature = "http", test))]` sobre `extract_url_path` para evitar advertencias de dead code al compilar con features por defecto (`default = []`).
- En `crates/sii-sim/tests/simulator_tests.rs`, se condicionó `test_http_server_live_network` con `#[cfg(feature = "http")]` y se añadieron pruebas sans-IO para `SimTransport::memory()` y `SimTransport::direct(...)`.

### Pendientes/bloqueos
- Ninguno. La tarea está terminada y todos los criterios de aceptación y verificación están cumplidos.

### Desviaciones de la spec
- Ninguna. Se respetaron estrictamente todos los requisitos y restricciones de AGENTS.md y TASK-SIM-01.
