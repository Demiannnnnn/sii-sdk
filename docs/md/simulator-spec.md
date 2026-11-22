# Especificación del Simulador del SII — `sii-sim`

Simulador **no oficial** de los servicios y endpoints del Servicio de Impuestos Internos (SII) de Chile.
Diseñado para el desarrollo, pruebas automatizadas e integración de aplicaciones basadas en `sii` sin
necesidad de interactuar con los ambientes reales del SII (`maullin` o `palena`).

---

## 1. Arquitectura

El simulador se compone de tres capas desacopladas:

1. **Núcleo sans-IO (`SiiSimulator`):** procesa estructuras de solicitud [`sii::transport::Request`] y produce
   respuestas [`sii::transport::Response`]. Implementa el trait [`sii::transport::Transport`] directamente,
   permitiendo inyectarlo de forma inmediata en [`sii::SiiClient`] en tests sin crear sockets ni hilos.
2. **Servidor HTTP local (`HttpServer`):** servidor TCP HTTP/1.1 ultraliviano sobre `std::net::TcpListener`
   que vincula **exclusivamente interfaces loopback** (`127.0.0.1`), garantizando aislamiento total de red (QA-01, RF-04).
3. **Binario CLI (`sii-sim`):** servidor de línea de comandos para desarrollo local, pruebas con `curl` o
   integración con la CLI `sii`.

```
                    ┌──────────────────────────────┐
                    │      SiiClient / CLI sii     │
                    └──────────────┬───────────────┘
                                   │
             ┌─────────────────────┴─────────────────────┐
             │ (In-memory sans-IO)                       │ (HTTP loopback 127.0.0.1)
             ▼                                           ▼
┌─────────────────────────┐                 ┌─────────────────────────┐
│      SiiSimulator       │                 │       HttpServer        │
│   (impl Transport)      │                 │    (std::net 127.0.0.1) │
└────────────┬────────────┘                 └────────────┬────────────┘
             │                                           │
             └─────────────────────┬─────────────────────┘
                                   ▼
                      Escenarios configurables
             (Aceptado, Rechazado, EnProceso, etc.)
```

---

## 2. Endpoints imitados (RF-01)

El simulador detecta los endpoints por sufijo de URL o ruta relativa, funcionando de forma idéntica tanto
para peticiones dirigidas a `https://maullin.sii.cl/...`, `https://palena.sii.cl/...`, o `http://127.0.0.1:8080/...`.

### 2.1 Semilla (`.../DTEWS/CrSeed.jws`)

- **Método:** `GET`
- **Headers:** estándar.
- **Respuesta exitosa (HTTP 200):**
  ```xml
  <?xml version="1.0" encoding="UTF-8"?>
  <SII:RESPUESTA xmlns:SII="http://www.sii.cl/XMLSchema">
      <SII:RESP_HDR>
          <ESTADO>00</ESTADO>
          <GLOSA>Operacion Exitosa</GLOSA>
      </SII:RESP_HDR>
      <SII:RESP_BODY>
          <SEMILLA>0079009845665</SEMILLA>
      </SII:RESP_BODY>
  </SII:RESPUESTA>
  ```
- **Respuesta error (`SemillaError`):**
  ```xml
  <?xml version="1.0" encoding="UTF-8"?>
  <SII:RESPUESTA xmlns:SII="http://www.sii.cl/XMLSchema">
      <SII:RESP_HDR>
          <ESTADO>-1</ESTADO>
          <GLOSA>Error interno de base de datos</GLOSA>
      </SII:RESP_HDR>
  </SII:RESPUESTA>
  ```

### 2.2 Token (`.../DTEWS/GetTokenFromSeed.jws`)

- **Método:** `POST`
- **Headers:** `Content-Type: application/xml; charset=UTF-8`
- **Cuerpo:** documento XML firmado `<getToken><item><Semilla>...</Semilla></item><Signature>...</Signature></getToken>`.
- **Respuesta exitosa (HTTP 200):**
  ```xml
  <?xml version="1.0" encoding="UTF-8"?>
  <SII:RESPUESTA xmlns:SII="http://www.sii.cl/XMLSchema">
      <SII:RESP_HDR>
          <ESTADO>00</ESTADO>
          <GLOSA>Token Creado</GLOSA>
      </SII:RESP_HDR>
      <SII:RESP_BODY>
          <TOKEN>SIM_TOKEN_79009845</TOKEN>
      </SII:RESP_BODY>
  </SII:RESPUESTA>
  ```
- **Respuesta rechazo (`TokenInvalido`):**
  ```xml
  <?xml version="1.0" encoding="UTF-8"?>
  <SII:RESPUESTA xmlns:SII="http://www.sii.cl/XMLSchema">
      <SII:RESP_HDR>
          <ESTADO>11</ESTADO>
          <GLOSA>Firma invalida o certificado no autorizado</GLOSA>
      </SII:RESP_HDR>
  </SII:RESPUESTA>
  ```

### 2.3 Carga de Envíos (`.../cgi_dte/UPL/DTEUpload`)

- **Método:** `POST`
- **Headers:**
  - `Content-Type: multipart/form-data; boundary=...`
  - `Cookie: TOKEN=<token>`
- **Partes form-data:** `rutSender`, `dvSender`, `rutCompany`, `dvCompany`, `archivo` (con el XML del sobre).
- **Respuesta exitosa (HTTP 200):**
  ```xml
  <?xml version="1.0" encoding="UTF-8"?>
  <RECEPCIONDTE>
      <STATUS>0</STATUS>
      <TRACKID>100000001</TRACKID>
  </RECEPCIONDTE>
  ```
- **Respuesta rechazo de token (`TokenInvalido`):**
  ```xml
  <RECEPCIONDTE>
      <STATUS>5</STATUS>
      <DETAIL>Token no autenticado o sesion caducada</DETAIL>
  </RECEPCIONDTE>
  ```
- **Respuesta rechazo de carátula (`UploadRechazado`):**
  ```xml
  <RECEPCIONDTE>
      <STATUS>1</STATUS>
      <DETAIL>Envio Rechazado por Error en Caratula</DETAIL>
  </RECEPCIONDTE>
  ```

### 2.4 Consulta de Estado (`.../DTEWS/QueryEstUp.jws`)

- **Método:** `POST`
- **Headers:** `Content-Type: text/xml; charset=UTF-8`, `SOAPAction: ""`
- **Cuerpo:** sobre SOAP con `<m:getEstUp><Rut>...</Rut><Dv>...</Dv><TrackId>...</TrackId><Token>...</Token></m:getEstUp>`.
- **Respuesta SOAP (HTTP 200):**
  Retorna un sobre SOAP cuyo elemento `<getEstUpReturn>` contiene la respuesta XML del SII con caracteres escapados (`&lt;`, `&gt;`).

  - **Aceptado (`Scenario::Aceptado`):**
    ```xml
    <soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/">
    <soapenv:Body>
    <ns1:getEstUpResponse xmlns:ns1="https://maullin.sii.cl/DTEWS/QueryEstUp.jws">
    <getEstUpReturn xsi:type="xsd:string">&lt;?xml version="1.0"?&gt;&lt;SII:RESPUESTA xmlns:SII="http://www.sii.cl/XMLSchema"&gt;&lt;SII:RESP_HDR&gt;&lt;TRACKID&gt;100000001&lt;/TRACKID&gt;&lt;ESTADO&gt;EPR&lt;/ESTADO&gt;&lt;GLOSA&gt;Envio Procesado&lt;/GLOSA&gt;&lt;/SII:RESP_HDR&gt;&lt;SII:RESP_BODY&gt;&lt;TIPO_DOCTO&gt;33&lt;/TIPO_DOCTO&gt;&lt;INFORMADOS&gt;1&lt;/INFORMADOS&gt;&lt;ACEPTADOS&gt;1&lt;/ACEPTADOS&gt;&lt;RECHAZADOS&gt;0&lt;/RECHAZADOS&gt;&lt;REPAROS&gt;0&lt;/REPAROS&gt;&lt;/SII:RESP_BODY&gt;&lt;/SII:RESPUESTA&gt;</getEstUpReturn>
    </ns1:getEstUpResponse>
    </soapenv:Body>
    </soapenv:Envelope>
    ```

  - **Rechazado (`Scenario::Rechazado`):**
    ```xml
    <soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/">
    <soapenv:Body>
    <ns1:getEstUpResponse xmlns:ns1="https://maullin.sii.cl/DTEWS/QueryEstUp.jws">
    <getEstUpReturn xsi:type="xsd:string">&lt;?xml version="1.0"?&gt;&lt;SII:RESPUESTA xmlns:SII="http://www.sii.cl/XMLSchema"&gt;&lt;SII:RESP_HDR&gt;&lt;TRACKID&gt;100000001&lt;/TRACKID&gt;&lt;ESTADO&gt;EPR&lt;/ESTADO&gt;&lt;GLOSA&gt;Envio Rechazado por Documentos&lt;/GLOSA&gt;&lt;/SII:RESP_HDR&gt;&lt;SII:RESP_BODY&gt;&lt;TIPO_DOCTO&gt;33&lt;/TIPO_DOCTO&gt;&lt;INFORMADOS&gt;1&lt;/INFORMADOS&gt;&lt;ACEPTADOS&gt;0&lt;/ACEPTADOS&gt;&lt;RECHAZADOS&gt;1&lt;/RECHAZADOS&gt;&lt;REPAROS&gt;0&lt;/REPAROS&gt;&lt;/SII:RESP_BODY&gt;&lt;/SII:RESPUESTA&gt;</getEstUpReturn>
    </ns1:getEstUpResponse>
    </soapenv:Body>
    </soapenv:Envelope>
    ```

  - **En proceso (`Scenario::EnProceso`):**
    ```xml
    <soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/">
    <soapenv:Body>
    <ns1:getEstUpResponse xmlns:ns1="https://maullin.sii.cl/DTEWS/QueryEstUp.jws">
    <getEstUpReturn xsi:type="xsd:string">&lt;?xml version="1.0"?&gt;&lt;SII:RESPUESTA xmlns:SII="http://www.sii.cl/XMLSchema"&gt;&lt;SII:RESP_HDR&gt;&lt;TRACKID&gt;100000001&lt;/TRACKID&gt;&lt;ESTADO&gt;REC&lt;/ESTADO&gt;&lt;GLOSA&gt;Envio Recibido Conforme, en cola de validacion&lt;/GLOSA&gt;&lt;/SII:RESP_HDR&gt;&lt;/SII:RESPUESTA&gt;</getEstUpReturn>
    </ns1:getEstUpResponse>
    </soapenv:Body>
    </soapenv:Envelope>
    ```

---

## 3. Escenarios (RF-01, RF-03)

Los escenarios se controlan mediante el enum `Scenario` y la estructura `ScenarioConfig`:

| Escenario | Alias CLI | Semilla | Token | Upload | Estado |
|:--|:--|:--|:--|:--|:--|
| `Aceptado` (default) | `aceptado`, `approve` | `ESTADO 00` | `ESTADO 00` | `STATUS 0` | `EPR` (aceptados: 1, rechazados: 0) |
| `Rechazado` | `rechazado`, `reject` | `ESTADO 00` | `ESTADO 00` | `STATUS 0` | `EPR` (aceptados: 0, rechazados: 1) |
| `EnProceso` | `en_proceso`, `in_process` | `ESTADO 00` | `ESTADO 00` | `STATUS 0` | `REC` (sin documentos aún) |
| `TokenInvalido` | `token_invalido` | `ESTADO 00` | `ESTADO 11` | `STATUS 5` | `-11` |
| `UploadRechazado` | `upload_rechazado` | `ESTADO 00` | `ESTADO 00` | `STATUS 1` | `RCT` |
| `SemillaError` | `semilla_error` | `ESTADO -1` | — | — | — |

### 3.1 Configuración basada en datos (JSON)

Se puede suministrar un archivo JSON con `--scenario-file` o parsearlo mediante `ScenarioConfig::from_json`:

```json
{
  "default_scenario": "aceptado",
  "fixed_seed": "0079009845665",
  "fixed_token": "SIM_TOKEN_79009845",
  "track_statuses": {
    "998877": {
      "estado": "EPR",
      "glosa": "Envio Rechazado por Documentos",
      "aceptados": 0,
      "rechazados": 1,
      "reparos": 0,
      "tipo_docto": 33
    },
    "998878": {
      "estado": "REC",
      "glosa": "Envio en cola"
    }
  }
}
```

---

## 4. Uso como biblioteca (Rust)

### 4.1 Uso directo sans-IO con `SiiClient`

```rust
use sii::{codes::Ambiente, SiiClient};
use sii_sim::{Scenario, SiiSimulator};

// Instanciar simulador con escenario deseado
let sim = SiiSimulator::new().with_scenario(Scenario::Aceptado);

// SiiClient usa directamente el simulador como Transport en memoria
let client = SiiClient::new(Ambiente::Certificacion, &sim);

// Flujo completo sin red ni sockets
let token = client.obtener_token(&signer, &cert)?;
let track_id = client.enviar(&envio, &token)?;
let estado = client.estado(track_id, &token)?;

assert!(estado.is_aceptado());
```

### 4.2 Servidor HTTP en segundo plano en tests

```rust
use std::sync::Arc;
use sii_sim::{HttpServer, Scenario, SiiSimulator};

let sim = Arc::new(SiiSimulator::new().with_scenario(Scenario::Aceptado));
let server = HttpServer::bind("127.0.0.1:0")?; // puerto dinámico
let mut handle = server.spawn(Arc::clone(&sim))?;

let url = handle.url("/DTEWS/CrSeed.jws");
// Realizar peticiones HTTP reales sobre loopback
handle.stop();
```

---

## 5. Binario CLI (`sii-sim`)

```bash
# Iniciar servidor por defecto en 127.0.0.1:8080 (escenario aceptado)
sii-sim

# Iniciar con escenario de rechazo en otro puerto loopback
sii-sim --listen 127.0.0.1:8888 --scenario rechazado

# Iniciar cargando configuración JSON
sii-sim --scenario-file escenario-pruebas.json

# Modo silencioso
sii-sim --quiet
```

### 5.1 Restricciones de seguridad (QA-01, RF-04)

- El servidor rechaza explícitamente cualquier dirección que no sea de loopback (`127.0.0.1` / `::1`).
- La salida para humanos se emite en UTF-8 utilizando `std::io::Write` sobre `stdout().lock()` (cumpliendo los lints de workspace sin macros de impresión directa).
