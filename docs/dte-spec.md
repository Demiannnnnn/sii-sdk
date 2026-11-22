# Especificación de formato DTE — sii-sdk (normativa)

Esta es la referencia normativa del SDK. Cuando el código y una tarea discrepan, mandan esta spec y
`.ai-context/`. **Los campos exactos de cada esquema XML provienen de la documentación oficial del SII**
(esquemas `DTE_v10.xsd`, `EnvioDTE_v10.xsd`, `SiiTypes_v10.xsd`, `DTE_Cesion`, etc., y el formato del CAF y
del TED). Esta spec fija el **alcance, la estructura y las reglas** que el SDK debe cumplir; los nombres y la
cardinalidad precisos de cada elemento se toman del esquema oficial y se fijan con **fixtures** validados.

> Regla de oro (LEG-01, handoff): si un detalle del esquema no está aquí ni en un fixture oficial, **no se
> inventa**: se reporta y el orquestador lo agrega citando la fuente del SII. Nunca se copia código de otras
> librerías; sí se pueden usar ejemplos de XML publicados por el SII, citando el origen en el fixture.

## 1. Fuentes oficiales (referencia de lectura)

- SII — "Formato Documentos Tributarios Electrónicos" (esquemas XSD de DTE, EnvioDTE, EnvioBOLETA).
- SII — "Formato Timbre Electrónico" (TED) y "Instructivo técnico del Código de Autorización de Folios (CAF)".
- SII — "Formato de autenticación" (semilla/token) y "Envío de DTE" (WSDL/endpoints por ambiente).
- SII — "Formato Información Electrónica de Compras y Ventas (IECV)".

Los fixtures del repo (`crates/sii/tests/fixtures/`) deben derivarse de estos esquemas/ejemplos, con un
comentario que cite la fuente. Nunca usan datos reales del dueño.

## 2. Ambientes

| Ambiente | Nombre SII | Uso |
|:--|:--|:--|
| `Certificacion` | maullin | pruebas y certificación |
| `Produccion` | palena | real |

El SDK no fija un default a producción. Los endpoints por ambiente los documenta TRANSPORT-01.

## 3. Documento (DTE)

Un `<DTE version="1.0">` contiene un `<Documento ID="...">` con, a grandes rasgos:

- `Encabezado`:
  - `IdDoc`: `TipoDTE` (33/34/39/41/52/56/61), `Folio`, `FchEmis`, y campos propios del tipo (p. ej. `IndServicio`, `FchVenc`).
  - `Emisor`: `RUTEmisor`, `RznSoc`, `GiroEmis`, `Acteco`, dirección, comuna.
  - `Receptor`: `RUTRecep`, `RznSocRecep`, dirección (en boleta el receptor puede ir mínimo).
  - `Totales`: `MntNeto`, `IVA` (19 % en afecto), `MntExe` (exento), `MntTotal`. Para 34/41 no hay IVA.
- `Detalle` (1..n): `NroLinData`, `NmbItem`, `QtyItem`, `PrcItem`, `MontoItem`, indicador de exención si aplica.
- `Referencia` (0..n): obligatoria en nota de crédito (61) y débito (56): `TpoDocRef`, `FolioRef`, `FchRef`, `CodRef`, `RazonRef`.
- `TED` (ver §4).
- `TmstFirma`.
- `Signature` XMLDSig del documento (ver §6).

**Reglas que el SDK valida (DTE-02):**
- 33 (afecta): `IVA = round(MntNeto * 0.19)`, `MntTotal = MntNeto + IVA + otros`.
- 34/41 (exenta): sin `IVA`; montos en `MntExe`.
- 56/61 (ND/NC): al menos una `Referencia` válida.
- 39/41 (boletas): van en `EnvioBOLETA`, no en `EnvioDTE`.
- Montos enteros CLP (DTE-03); RUT con DV válido (DTE-04).

Cada tipo tiene un fixture `dte-<tipo>.xml` y un test de serialización que compara el XML canónico generado
contra él (DTE-01).

## 4. Timbre Electrónico (TED)

`<TED version="1.0">` contiene:

- `DD` (datos del documento a timbrar): `RE` (RUT emisor), `TD` (tipo), `F` (folio), `FE` (fecha emisión),
  `RR` (RUT receptor), `RSR` (razón social receptor), `MNT` (monto total), `IT1` (nombre del primer ítem),
  `CAF` (el bloque `<CAF>` del archivo CAF, tal cual), `TSTED` (timestamp del timbre).
- `FRMT algoritmo="SHA1withRSA"`: firma de la **cadena canónica del `DD`** con la **llave privada RSA del
  CAF**. El algoritmo RSA-SHA1 lo **exige el SII** para el TED; no se cambia (handoff).

El `DD` canónico se timbra byte a byte igual a como lo define el SII; hay un test que compara el `DD`
generado contra un fixture y verifica `FRMT` con la llave pública del CAF (TED-01). Los **datos del PDF417**
son el `<TED>` serializado; el SDK no genera el bitmap (TED-02, ADR-06).

## 5. CAF

El archivo CAF es un `<AUTORIZACION><CAF version="1.0">` con:

- `DA`: `RE` (RUT emisor), `RS`, `TD` (tipo DTE), `RNG` (`D` desde, `H` hasta), `FA` (fecha autorización),
  `RSAPK` (llave pública del timbre: `M`, `E`), `IDK`.
- `FRMA algoritmo="SHA1withRSA"`: firma del SII sobre `DA`.
- `RSASK`: **llave privada RSA** para firmar el TED (secreto).
- `RSAPUBK`: llave pública correspondiente.

El SDK (CAF-01): parsea el CAF, expone el rango y el tipo, usa `RSASK` como `Signer` del TED **sin revelar la
llave**, y valida `FRMA` contra la llave pública del SII del ambiente cuando está disponible. Un folio fuera
de `[D, H]` es `Err` (CAF-01).

## 6. Firma XMLDSig (DTE y Envío)

- Firma **enveloped** sobre el `<Documento>` (DTE) o el `<SetDTE>` (Envío), con canonicalización
  C14N determinista, digest y `SignatureMethod` que el SII acepta (ver esquema oficial; históricamente
  RSA-SHA1, con migración del SII a SHA-256 en curso — la tarea SIG-01 fija el algoritmo exacto por ambiente
  desde la documentación vigente y lo deja configurable, con fixtures).
- `KeyInfo` incluye el certificado X.509 del emisor.
- Verificación real (SIG-02): un documento con la firma o el digest alterados falla.

## 7. Envío al SII

- `EnvioDTE`: `<SetDTE ID="...">` con `Caratula` (`RutEmisor`, `RutEnvia`, `RutReceptor` = SII, `FchResol`,
  `NroResol`, `TmstFirmaEnv`, `SubTotDTE` por tipo) y 1..n `<DTE>`; firmado (§6). Límite de DTE por envío
  según el SII.
- `EnvioBOLETA`: análogo para 39/41.
- **Autenticación (AUTH-01):** `GET` semilla → firmar la semilla con el certificado → `POST` para obtener
  token → usar el token en el envío.
- **Carga (TRANSPORT-01):** `POST` del XML del envío al endpoint del ambiente → respuesta con `TrackID`.
- **Estado:** consultar por `TrackID` → `Aceptado` | `Rechazado` | `EnProceso` + detalle (ADR-04: el rechazo
  es un valor, no un `Err`).

## 8. Libro de compra/venta (IECV, LIBRO-01)

`<LibroCompraVenta>` con `EnvioLibro`/`Caratula` (periodo, RUT, tipo de operación) y el detalle o resumen de
documentos, firmado (§6). Alcance inicial: construcción y firma del libro; el envío usa el mismo transporte.

## 9. Qué NO cubre esta spec

- La impresión/representación gráfica del DTE (la hace el host con los datos del PDF417).
- La obtención del CAF desde el portal del SII (el host lo entrega).
- Tipos de DTE fuera de §3 (se agregan como tareas nuevas, extendiendo `TipoDte`).
