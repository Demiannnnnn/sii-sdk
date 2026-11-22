# API pública — sii-sdk

Contrato con quien usa el crate `sii`. Mantener estable; cambios incompatibles solo con bump mayor y nota en
`CHANGELOG.md`. Lo que no está aquí es `pub(crate)`. Esto es el **objetivo**; `implementation-status.md` dice
qué existe hoy.

## Tipos base (`sii::types`, `sii::codes`)

- `Rut` — RUT chileno validado (DV módulo 11). `Rut::parse("76.123.456-7") -> Result<Rut, Error>`.
- `Monto` — entero CLP. Aritmética segura (sin `f64`).
- `TipoDte` — enum (`FacturaAfecta=33`, `FacturaExenta=34`, `Boleta=39`, `BoletaExenta=41`, `GuiaDespacho=52`, `NotaDebito=56`, `NotaCredito=61`).
- `Ambiente` — `Certificacion` | `Produccion`.
- `Error` — error de la librería (`thiserror`), variantes tipadas.

## CAF (`sii::caf`)

- `Caf::parse(xml: &[u8]) -> Result<Caf, Error>` — valida rango y firma del SII.
- `Caf::rango() -> (u64, u64)`, `Caf::tipo() -> TipoDte`, `Caf::contiene(folio) -> bool`.
- `Caf` expone un `Signer` del TED (la llave RSA del CAF) sin revelar la llave.

## DTE (`sii::dte`)

- Builders tipados por tipo, p. ej. `Dte::factura_afecta() -> FacturaBuilder`, que validan reglas (DTE-02) y producen un `Dte`.
- `Dte::to_xml() -> Result<Vec<u8>, Error>` — XML del SII (sin firmar aún).
- `Dte::timbrar(&Caf, folio, clock) -> Result<DteTimbrado, Error>` — agrega el `<TED>`.
- `DteTimbrado::firmar(&dyn Signer) -> Result<DteFirmado, Error>` — firma XMLDSig del DTE.
- `DteTimbrado::ted_pdf417_data() -> &[u8]` — datos para el PDF417 (el host genera el bitmap).

## Firma (`sii::signature`)

- `trait Signer` — firma bytes con la llave del emisor (lo implementa un cargador PKCS#12 o uno de prueba).
- `trait Certificate` — expone el certificado X.509 para incrustarlo en la firma.
- `verify(xml: &[u8]) -> Result<bool, Error>` — verifica una firma XMLDSig.

## Envío y libros (`sii::envio`, `sii::libro`)

- `EnvioDte::nuevo(emisor, receptor, ambiente)` → agrega `DteFirmado` → `firmar(&dyn Signer)` → `to_xml()`.
- `EnvioBoleta` análogo para boletas (39/41).
- `LibroCompraVenta` — resumen mensual IECV.

## Auth y cliente (`sii::auth`, `sii::client`, feature `http`)

- `auth::firmar_semilla(semilla, &dyn Signer) -> SolicitudToken`.
- `SiiClient::<T: Transport>::new(ambiente, transport)` con:
  - `obtener_token(&dyn Signer) -> Result<Token, Error>`,
  - `enviar(&EnvioDte, &Token) -> Result<TrackId, Error>`,
  - `estado(TrackId, &Token) -> Result<EstadoEnvio, Error>` (donde `EstadoEnvio` incluye `Aceptado`/`Rechazado`/`EnProceso` — ADR-04).
- `trait Transport` — `MemoryTransport` (tests), `HttpTransport` (feature `http`).
