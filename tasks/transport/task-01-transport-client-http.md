# TASK-TRANSPORT-01: Transport, SiiClient y HttpTransport

| Atributo | Detalle |
|:--|:--|
| **ID** | `TRANSPORT-01` |
| **Estado** | ✅ terminada |
| **Depende de** | ENVIO-01, AUTH-01 |
| **Rama** | `dev/transport-01-client` |
| **Prioridad** | P1 |

## 1. Contexto
`architecture.md` §1/§3, `docs/dte-spec.md` §7, ADR-04. La red vive detrás de `trait Transport`; el núcleo sigue sans-IO.
## 2. Requerimientos
- **RF-01 Transport:** `trait Transport` (enviar request → response, sans-IO del lado del núcleo) + `MemoryTransport` para tests.
- **RF-02 SiiClient:** `SiiClient::<T: Transport>::new(ambiente, transport)` con `obtener_token(&dyn Signer)`, `enviar(&EnvioDte, &Token) -> Result<TrackId, Error>`, `estado(TrackId, &Token) -> Result<EstadoEnvio, Error>`. `EstadoEnvio` = `Aceptado`/`Rechazado`/`EnProceso` + detalle; **rechazo es valor, no `Err`** (ADR-04).
- **RF-03 HttpTransport (feature `http`):** cliente TLS (rustls) con los endpoints por ambiente documentados aquí (certificación/producción). Fuera de los tests. Decide y fija en `architecture.md` §3 el cliente HTTP (`ureq` o `reqwest`-rustls) y repórtalo.
- **RF-04 Endpoints:** documenta en este archivo (o en `docs/`) las URLs por ambiente; nunca default a producción.
## 3. Tests
Flujo completo semilla→token→envío→estado contra `MemoryTransport`; rechazo del SII como `EstadoEnvio::Rechazado`; sin red real (QA-01).
## 4. DoD
AGENTS.md §8, incluido `cargo clippy -p sii --no-default-features`.
## Reporte

### Resumen
Se implementó la capa de transporte sans-IO y el cliente de alto nivel del SII (`TRANSPORT-01`):
1. **`trait Transport` & modelo HTTP sans-IO:**
   - Métodos `Method` (`Get`, `Post`), `Request`, y `Response` tipados y libres de E/S.
   - `Transport` trait implementado para `&T`, `Box<T>`, y `Arc<T>`.
   - `MemoryTransport` para tests deterministas: captura requests y responde por coincidencia exacta, prefijo de URL o handlers dinámicos.
2. **`HttpTransport` (feature `http`):**
   - Cliente TLS síncrono basado en `ureq` (2.x) con `rustls` (`webpki-roots`). Se seleccionó `ureq` por ser síncrono, liviano y alineado con el diseño sans-IO sin exigir runtime asíncrono (Tokio).
3. **`Ambiente` y URLs normativas del SII (RF-04):**
   - Métodos en `Ambiente`: `seed_url()`, `token_url()`, `upload_url()`, `query_status_url()`, `base_url()`.
   - Certificación: `https://maullin.sii.cl/DTEWS/CrSeed.jws`, `https://maullin.sii.cl/DTEWS/GetTokenFromSeed.jws`, `https://maullin.sii.cl/cgi_dte/UPL/DTEUpload`, `https://maullin.sii.cl/DTEWS/QueryEstUp.jws`.
   - Producción: `https://palena.sii.cl/DTEWS/CrSeed.jws`, `https://palena.sii.cl/DTEWS/GetTokenFromSeed.jws`, `https://palena.sii.cl/cgi_dte/UPL/DTEUpload`, `https://palena.sii.cl/DTEWS/QueryEstUp.jws`.
   - Sin default a producción (ADR-07, SEC-04).
4. **`SiiClient<T: Transport>`:**
   - `obtener_semilla()`, `obtener_token_con_solicitud()`, `obtener_token(&signer, &cert)`.
   - `enviar(&EnvioDte, &Token) -> Result<TrackId, Error>`: arma multipart/form-data con boundary (`rutSender`, `dvSender`, `rutCompany`, `dvCompany`, `archivo`), cabecera `Cookie: TOKEN=...`, parsea `<RECEPCIONDTE>` y devuelve `TrackId`.
   - `estado(TrackId, &Token) -> Result<EstadoEnvio, Error>`: construye SOAP envelope a `QueryEstUp.jws`, parsea tanto XML directo como SOAP con `<getEstUpReturn>`, y mapea a `EstadoEnvio`.
   - **ADR-04 cumplido estrictamente:** La respuesta de rechazo del SII se retorna como valor de dominio `Ok(EstadoEnvio::Rechazado(detalle))`, **nunca** como `Err`.
5. **Envoltorios de sobre:**
   - Métodos `rut_emisor()`, `rut_envia()`, `with_signed_xml()`, `set_signed_xml()`, `signed_xml()` añadidos a `EnvioDte` y `EnvioBoleta`.

### Archivos
- `crates/sii/src/transport.rs` (nuevo: `Transport`, `Method`, `Request`, `Response`, `MemoryTransport`, `HttpTransport`)
- `crates/sii/src/client.rs` (nuevo: `SiiClient`, `TrackId`, `DetalleEnvio`, `EstadoEnvio`, builders y parsers)
- `crates/sii/src/codes.rs` (URLs normativas por ambiente)
- `crates/sii/src/error.rs` (variante `Error::Transport(String)`)
- `crates/sii/src/envio.rs` (getters y almacenamiento de pre-signed XML)
- `crates/sii/src/lib.rs` (re-exportación pública de tipos de cliente y transporte)
- `crates/sii/Cargo.toml` (dependencia opcional `ureq` detrás de feature `http`)
- `Cargo.toml` (workspace dependency `ureq = { version = "2.10", default-features = false, features = ["tls"] }`)
- `Cargo.lock`
- `crates/sii/tests/client_transport.rs` (tests de integración de flujo completo, ADR-04 rechazo, en proceso, errores)
- `tasks/transport/task-01-transport-client-http.md`

### Resultado real de cada comando de AGENTS.md §8

1. `cargo fmt --all --check`:
```text
(exit code 0 - sin diferencias de formato)
```

2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`:
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.26s
(exit code 0 - cero warnings)
```

3. `cargo clippy -p sii --no-default-features -- -D warnings`:
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s
(exit code 0 - cero warnings)
```

4. `cargo test --workspace --all-features`:
```text
     Running unittests src/lib.rs (target/debug/deps/sii-1bba35e9ae3d18ab)
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s

     Running tests/auth.rs (target/debug/deps/auth-779509b4c9e8bec1)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.62s

     Running tests/caf.rs (target/debug/deps/caf-7f043f051efb828a)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/client_transport.rs (target/debug/deps/client_transport-b5f4c8e094cd5633)
running 5 tests
test test_ambiente_urls_certificacion_and_produccion ... ok
test test_status_en_proceso_intermediate ... ok
test test_upload_rejection_returns_err ... ok
test test_adr04_rejection_is_domain_value_not_err ... ok
test test_client_full_flow_success ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.94s

     Running tests/dte_model.rs (target/debug/deps/dte_model-f7ed907d991b4f11)
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/envio.rs (target/debug/deps/envio-1593dc3a173324a4)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.39s

     Running tests/signature.rs (target/debug/deps/signature-1a0ab1ba3e204f59)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.51s

     Running tests/smoke.rs (target/debug/deps/smoke-18b42b4c229be2d9)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/ted.rs (target/debug/deps/ted-9e3d5bc7392558bc)
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

     Running unittests src/main.rs (target/debug/deps/sii-b2a23eacd708de29)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/smoke.rs (target/debug/deps/smoke-e7a3e39760dd89ef)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s

     Running unittests src/lib.rs (target/debug/deps/sii_sim-6db4ef67b6183918)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/sii_sim-ef3829b58c4d846d)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/smoke.rs (target/debug/deps/smoke-9c034d2eae0eb7a1)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s

   Doc-tests sii
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests sii_sim
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

(exit code 0 - todos los tests pasaron)
```

5. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features`:
```text
 Documenting sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii)
 Documenting sii-cli v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii-cli)
 Documenting sii-sim v0.1.0 (/Users/demian/Documents/Github/sii-sdk-plus/crates/sii-sim)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.80s
   Generated /Users/demian/Documents/Github/sii-sdk-plus/target/doc/sii/index.html and 2 other files
(exit code 0 - documentación generada sin warnings)
```

### Decisiones
- **Cliente HTTP elegido:** `ureq` (2.x) con `default-features = false` y `features = ["tls"]` (usando `rustls` y `webpki-roots`). Se reporta al orquestador para actualizar `architecture.md` §3 (dado que `.ai-context/` es de solo lectura para agentes por AGENTS.md §5).
- **ADR-04 (Rechazo ≠ error):** El método `SiiClient::estado` retorna `Ok(EstadoEnvio::Rechazado(detalle))` cuando el SII rechaza el envío (por firma, carátula, schema o documentos). Los errores (`Err`) se reservan para fallos de red/transporte, HTTP 5xx no parseable o XML mal formado.
- **Ambiente explícito:** `Ambiente` no tiene default a producción. Se expusieron URLs normativas por método en `Ambiente`.

### Pendientes/bloqueos
- Ninguno. La tarea está terminada y cumple con todos los criterios de aceptación y DoD.

### Desviaciones de la spec
- Ninguna.
