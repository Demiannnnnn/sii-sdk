# Restricciones — sii-sdk

Reglas duras con ID. Una tarea las referencia; una revisión las verifica. Violar una es motivo de rechazo.

## DTE — modelo y XML

- **DTE-01** Cada tipo soportado (33, 34, 39, 41, 52, 56, 61) genera XML que valida contra su fixture de referencia (`docs/md/dte-spec.md` §3). Diferencia de bytes significativa = bug.
- **DTE-02** Las reglas tributarias se cumplen: factura afecta (33) calcula IVA 19 % sobre el neto; exenta (34/41) no lleva IVA; nota de crédito/débito (61/56) **exige** al menos una referencia (`Referencia`) a un documento previo.
- **DTE-03** Los montos son enteros en pesos (CLP) sin decimales, salvo donde el SII permita decimales (cantidades, precios unitarios): ver `types::Monto`. Nunca `f64` para dinero.
- **DTE-04** El `Rut` se valida con su dígito verificador (módulo 11) al construirse; un RUT inválido es `Err`, no un timbre inválido.

## TED y folios

- **CAF-01** El CAF se parsea sin confiar en el emisor: se valida el rango de folios y, si está presente, la firma `FRMA` del SII contra la llave pública del SII del ambiente. Un folio fuera del rango del CAF es `Err`.
- **TED-01** El `DD` del timbre contiene exactamente los campos que exige el SII (RE, TD, F, FE, RR, RSR, MNT, IT1, CAF, TSTED) y `FRMT` es su firma **RSA-SHA1** con la llave privada del CAF. Un test compara el `DD` canónico byte a byte contra el fixture.
- **TED-02** Los datos del PDF417 son exactamente el `<TED>` serializado que exige el SII; el SDK no genera el bitmap (ADR-06).

## Firma

- **SIG-01** La firma del DTE y del Envío es **XMLDSig** enveloped, con canonicalización determinista; dos firmas del mismo documento con la misma llave y hora producen la misma salida (para testear) salvo la parte que el estándar exige variable, que se inyecta.
- **SIG-02** La verificación de firma es real (no un stub): un DTE con firma alterada falla la verificación en un test.
- **SIG-03** El núcleo nunca abre un archivo de certificado ni accede al almacén del SO; recibe un `Signer` (ADR-02).

## API y seguridad

- **API-01** El núcleo compila y pasa sus tests con `--no-default-features` (sans-IO puro).
- **API-02** `Rechazo`/`Aceptado`/`EnProceso` del SII son valores de dominio; `Err` solo para transporte/XML/datos (ADR-04).
- **SEC-01** NUNCA se versiona un certificado real, CAF real o llave privada real. Los tests autogeneran llaves RSA de prueba (ADR-05).
- **SEC-02** Ni la llave privada del CAF ni la del certificado se escriben en logs, trazas, mensajes de error ni `Debug`. `Debug` de tipos con secreto los redacta.
- **SEC-03** Los secretos reales (en la CLI) se leen de variable de entorno o archivo indicado por el usuario, nunca hardcodeados.

## Licencias y calidad

- **LEG-01** Se implementa desde `docs/md/dte-spec.md` y documentación pública del SII; NUNCA se copia código de LibreDTE u otras librerías (licencia incompatible o sin verificar).
- **QA-01** Cero red real y cero secretos reales en tests: solo `sii-sim`, `MemoryTransport` y fixtures/llaves de prueba.
- **QA-02** Los tests ejercitan comportamiento real (XML válido, firma que verifica, rechazo que se detecta), no asserts triviales.
- **QA-03** La hora no se toma de `SystemTime::now()` en el núcleo; se inyecta para que los TED/semillas sean reproducibles en test.
