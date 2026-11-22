# Arquitectura — sii-sdk

## 1. Vista general

```
 Software del emisor (rework-sistema-facturacion, caja, ERP…)
        │  API pública (public-api.md)
        ▼
┌──────────────────────────────────── sii ──────────────────────────────────────┐
│  client      SiiClient<T>  (flujo alto nivel: semilla→token→envío→estado)      │
│     │        sans-IO salvo el Transport inyectado                              │
│  envio       EnvioDTE / EnvioBOLETA: carátula, agrupación, serialización        │
│  dte         modelo tipado por tipo (33/34/39/52/56/61) → XML del SII           │
│  ted         Timbre: arma DD, firma RSA-SHA1 con la llave del CAF, PDF417 data  │
│  caf         parse del XML CAF: rango de folios + llave RSA + validación FRMA   │
│  signature   XMLDSig: firma/verifica con un Signer (trait) — sans-IO            │
│  libro       libro de compra/venta (IECV)                                       │
│  auth        semilla/token del SII (modelo + firma de la semilla)               │
│  folio       rango, siguiente folio, agotamiento (el host persiste el estado)   │
│  codes/types Rut, Monto, TipoDte, Ambiente, errores                             │
│     │                                                                            │
│  transport   trait Transport  ── MemoryTransport (tests)                         │
│                               ── HttpTransport (feature "http", contra sim/SII)  │
└──────────────────────────────────────────┬─────────────────────────────────────┘
                                           │ XML + HTTP(S)
                         sii-sim (TCP/HTTP local) ── o ── SII real (dueño, CERT-01)
```

- **`sii-sim`** imita los endpoints del SII (semilla, token, upload, consulta de estado) con escenarios deterministas. **`sii-cli`** (bin `sii`) arma, firma, envía (contra el simulador o el SII real) y consulta estado.

## 2. Principios de diseño

1. **Sans-IO primero (ADR-01):** el núcleo (`dte`, `ted`, `caf`, `signature`, `envio`, `libro`, `auth`) no hace E/S ni usa el reloj del sistema directamente. La red vive detrás del trait `Transport`; la hora y el azar (si hacen falta) se inyectan. Así todo es testeable sin red ni certificados reales.
2. **La firma se inyecta (ADR-02):** `signature` define un trait `Signer`/`Certificate`; el crate no abre archivos `.pfx` por su cuenta en el núcleo. Un cargador concreto (PKCS#12/PEM) vive detrás de una feature y lo usa la CLI.
3. **XML exacto del SII (ADR-03):** cada tipo de DTE y cada envío tienen un test que compara el XML generado contra un fixture validado. La canonicalización para la firma (C14N) es determinista.
4. **Rechazo ≠ error (ADR-04):** que el SII rechace un envío es un resultado del dominio (`EstadoEnvio::Rechazado`), no un `Err`. `Err` se reserva para fallos de transporte, XML inválido o datos mal formados.
5. **Sin pánicos ni `unsafe`** (AGENTS.md §6).
6. **Tests deterministas** con `sii-sim` y certificados/CAF **de prueba autogenerados**; nunca red ni secretos reales (QA-*).
7. **Secretos nunca en el repo** (SEC-*): CAF y certificados de prueba se generan en tiempo de test o son fixtures evidentes; los reales van por variable de entorno o archivo ignorado.

## 3. Stack — CERRADO

- **Rust edition 2024**, MSRV **1.85**, `resolver = "3"`.
- Workspace `members = ["crates/*"]`: `sii`, `sii-sim`, `sii-cli`.
- `[workspace.lints.rust] unsafe_code = "forbid"`, `missing_docs = "deny"`; clippy a `-D warnings`.
- **Dependencias permitidas** (no agregar otras sin que la tarea lo pida):
  - XML: `quick-xml` (serde opcional) para leer/escribir; canonicalización C14N propia o mínima documentada.
  - Cripto: `rsa`, `sha1` (exigido por el SII para TED/semilla), `sha2`, `x509-cert`/`der` para el certificado, `pkcs12`/`pkcs8` para cargar llaves (detrás de feature `pkcs12`), `base64`.
  - PDF417 del TED: solo se exponen los **datos** a codificar; la generación del bitmap es del host (no se agrega una dependencia de PDF417 en el núcleo).
  - `thiserror` (errores), `serde` + `serde_json` (CLI/escenarios), `time` o `chrono` (una sola, para fechas; sin usar `now()` en el núcleo), `tracing`.
  - HTTP (feature `http`): un cliente mínimo (`ureq` o `reqwest` con rustls — se decide en TRANSPORT-01 y se fija aquí). No en el núcleo.
  - CLI: `clap`. Simulador: lo que exija su transporte local.
  - Tests: `proptest`, `insta` (snapshots de XML) si la tarea lo aprueba.
- **Features:** `sii` default = núcleo sans-IO. `http` = `HttpTransport`. `pkcs12` = cargar certificado desde `.pfx`. `serde` = derivar serde en los tipos públicos. El núcleo DEBE compilar con `--no-default-features`.

## 4. Capas y responsabilidad

| Módulo (`crates/sii/src`) | Responsabilidad | No hace |
|:--|:--|:--|
| `types`, `codes` | `Rut` (con DV), `Monto`, `TipoDte`, `Ambiente`, enums de estado | lógica de red |
| `caf` | parsear CAF, exponer rango + llave RSA, validar FRMA del SII | descargar CAF |
| `dte` | modelo por tipo, validación de reglas (IVA, referencias NC/ND), XML | firmar |
| `ted` | construir `DD`, firmar con llave del CAF (RSA-SHA1), datos PDF417 | render |
| `signature` | trait `Signer`/`Certificate`, firmar/verificar XMLDSig, C14N | abrir archivos |
| `envio` | `EnvioDTE`/`EnvioBOLETA`, carátula, resumen, firma del sobre | subir |
| `libro` | libro de compra/venta (IECV) | subir |
| `auth` | modelo semilla/token, firma de la semilla | HTTP |
| `client` | orquesta el flujo con un `Transport` inyectado | parsear XML a mano |
| `transport` | trait `Transport`; `MemoryTransport`; `HttpTransport` (feature) | lógica DTE |

## 5. Decisiones (ADR)

- **ADR-01 Sans-IO.** Testabilidad y portabilidad; la red y el reloj se inyectan.
- **ADR-02 Firma inyectada.** El núcleo no toca el sistema de archivos ni el almacén de certificados del SO.
- **ADR-03 XML exacto.** Fixtures por tipo; la firma depende de bytes canónicos estables.
- **ADR-04 Rechazo ≠ error.** El rechazo del SII es un valor de dominio.
- **ADR-05 CAF/cert de prueba autogenerados.** Los tests generan sus propias llaves RSA de juguete; jamás secretos reales (AGENTS.md §7, SEC-*).
- **ADR-06 PDF417 fuera del núcleo.** Se exponen los datos; el bitmap lo hace el host (evita dependencia pesada y mantiene el núcleo sans-IO).
- **ADR-07 Dos ambientes explícitos.** `Ambiente::Certificacion` / `Ambiente::Produccion`; sin default a producción.
