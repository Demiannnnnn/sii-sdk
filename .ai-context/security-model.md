# Modelo de seguridad — sii-sdk

El SDK maneja material criptográfico sensible: la **llave privada RSA del CAF** y la **llave privada del
certificado digital** del emisor. Comprometerlas permite emitir DTE fraudulentos a nombre del
contribuyente. Reglas:

## 1. Secretos nunca en el repo (SEC-01)

- NUNCA se versiona un certificado real, un CAF real ni una llave privada real.
- Los tests **autogeneran** llaves RSA de prueba y arman CAF/certificados de juguete en memoria o en `tests/fixtures/` evidentemente falsos (ADR-05). Un CAF de prueba lleva un comentario que dice que es ficticio.
- `.gitignore` incluye `*.pfx`, `*.p12`, `*.pem`, `*.key`, `caf*.xml` reales, `.env`.

## 2. Secretos nunca en logs ni errores (SEC-02)

- Ni la llave del CAF ni la del certificado aparecen en `tracing`, mensajes de `Error`, `Display` ni `Debug`.
- Los tipos que contienen una llave privada implementan `Debug` **redactando** el campo (`CafKey(<redacted>)`).
- La feature `tracing` registra eventos de alto nivel (tipo de DTE, folio, track id, estado) pero **nunca** llaves, el XML firmado completo ni datos personales del receptor más allá del RUT.

## 3. Entrada de secretos reales (SEC-03)

- En la CLI, el certificado y su clave se leen de un archivo indicado por el usuario y una variable de entorno para la contraseña (`SII_CERT_PASSWORD`), nunca de un argumento en texto plano ni hardcodeados.
- El CAF real lo entrega el host/usuario; el SDK no lo descarga del SII en el alcance inicial.

## 4. Red y ambientes

- El núcleo no hace red. `HttpTransport` (feature `http`) usa TLS (rustls) contra el SII.
- `Ambiente` es explícito (`Certificacion`/`Produccion`); no hay default a producción (ADR-07). La CLI exige elegirlo.
- En tests nunca se toca el SII real: solo `sii-sim` o `MemoryTransport` (QA-01).

## 5. Validación de confianza

- El CAF se valida (rango + firma `FRMA` del SII cuando se dispone de la llave pública del ambiente) antes de usar sus folios (CAF-01). Un CAF no confiable no timbra.
- La verificación de firma XMLDSig es real (SIG-02); el SDK puede verificar un DTE/Envío recibido.
