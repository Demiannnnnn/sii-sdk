# Dominio — sii-sdk

Lenguaje ubicuo del SDK para Documentos Tributarios Electrónicos (DTE) del SII de Chile. Términos en
es-CL; identificadores Rust en inglés (`conventions.md` §1).

## 1. Qué es un DTE

Un **Documento Tributario Electrónico** es un documento (factura, boleta, nota, guía) en **XML** con la
estructura que define el SII, **timbrado** con un folio autorizado y **firmado** digitalmente por el
emisor. El SII valida el XML, el timbre y la firma.

Cadena de confianza: el SII autoriza rangos de folios (**CAF**) → el emisor arma el DTE con un folio de
ese rango → calcula el **TED** (timbre) y lo firma con la llave del CAF → firma el DTE completo con su
**certificado digital** → agrupa DTE en un **Envío** firmado → lo sube al SII → consulta el **estado**.

## 2. Tipos de DTE (alcance inicial)

| Código | Nombre | Notas |
|:--|:--|:--|
| 33 | Factura electrónica (afecta a IVA) | con IVA 19 % |
| 34 | Factura electrónica exenta | sin IVA |
| 39 | Boleta electrónica | consumidor final; se envía en `EnvioBOLETA`, no en `EnvioDTE` |
| 41 | Boleta exenta electrónica | |
| 52 | Guía de despacho electrónica | traslado de mercadería |
| 56 | Nota de débito electrónica | aumenta un documento previo |
| 61 | Nota de crédito electrónica | anula/corrige un documento previo (referencia obligatoria) |

Fuera de alcance inicial (se puede agregar luego): 46 (factura de compra), 43 (liquidación factura), 110/111/112 (exportación). El modelo debe dejar espacio para extenderse por código de tipo.

## 3. Conceptos clave

- **Folio:** número correlativo único por tipo de DTE y emisor. Sale de un rango autorizado (CAF). No se repite ni se salta sin registro.
- **CAF (Código de Autorización de Folios):** archivo XML que entrega el SII. Contiene: el tipo de DTE, el rango de folios autorizado (desde–hasta), la **llave privada RSA** para firmar el TED, la llave pública y la firma del SII (`FRMA`). Es un **secreto**: contiene una llave privada.
- **TED (Timbre Electrónico DTE):** bloque `<TED>` dentro del DTE con los datos mínimos del documento (`DD`: emisor, folio, fecha, receptor, monto total, primer ítem) y su firma RSA (`FRMT`) hecha con la llave del CAF. Se representa también como código de barras **PDF417** en la impresión.
- **Certificado digital del emisor:** certificado X.509 + llave privada (archivo `.pfx`/`.p12` con clave, o PEM) de la persona/empresa autorizada. Firma el DTE completo y el Envío con **XMLDSig**. Es un **secreto**.
- **DD / FRMT:** `DD` = datos a timbrar (dentro del TED); `FRMT` = firma RSA-SHA1 de `DD` con la llave del CAF (algoritmo que exige el SII para el timbre).
- **Envío (`EnvioDTE` / `EnvioBOLETA`):** sobre XML que agrupa uno o más DTE del mismo emisor, con una carátula (`Caratula`: RUT emisor, RUT receptor/SII, resumen por tipo) y su propia firma XMLDSig.
- **Semilla y Token:** para enviar al SII el emisor pide una **semilla**, la firma, y obtiene un **token** de sesión. Todo el envío va autenticado con ese token.
- **Estado de envío:** tras subir, el SII entrega un **Track ID**; con él se consulta si el envío fue aceptado, rechazado o está en proceso, y el detalle por DTE.
- **Libro de compra/venta (IECV):** resumen XML mensual de documentos emitidos/recibidos, firmado y enviado al SII.
- **Ambientes:** **certificación** (maullin, pruebas) y **producción** (palena). El SDK nunca fija uno por defecto que apunte a producción; el host elige.

## 4. Actores

- **Emisor:** contribuyente que emite DTE. Tiene certificado digital y CAF.
- **Receptor:** quien recibe el DTE (otra empresa o consumidor final).
- **SII:** autoridad que autoriza folios, recibe envíos y valida.
- **Host:** el software que usa el SDK (p. ej. `rework-sistema-facturacion`).

## 5. Alcance — qué NO es

- **No** es un sistema de facturación completo: no maneja clientes, inventario ni contabilidad. Eso es `rework`. El SDK construye/firma/envía DTE y nada más.
- **No** genera el PDF/impresión del DTE: expone los **datos del PDF417** del TED; el render lo hace el host.
- **No** obtiene el CAF automáticamente del SII en el alcance inicial (el host lo descarga del portal SII y lo entrega al SDK). Se puede agregar luego.
- **No** guarda estado ni base de datos: es una librería sans-IO (ver `architecture.md`). La persistencia de folios la hace el host.
- **No** incluye certificados, CAF ni llaves reales en el repo (AGENTS.md §7).
