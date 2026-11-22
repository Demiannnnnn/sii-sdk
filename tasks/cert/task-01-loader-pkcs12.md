# TASK-CERT-LOADER-01: Cargar certificado/llave (PKCS#12/PEM)

| Atributo | Detalle |
|:--|:--|
| **ID** | `CERT-LOADER-01` |
| **Estado** | ✅ terminada |
| **Depende de** | SIG-01 |
| **Rama** | `dev/cert-loader-01-pkcs12` |
| **Prioridad** | P2 |

## 1. Contexto
`architecture.md` ADR-02, feature `pkcs12`. Implementa `Signer`/`Certificate` cargando un certificado real del emisor fuera del núcleo.
## 2. Requerimientos
- **RF-01 PKCS#12:** cargar `.pfx`/`.p12` con contraseña (de env/parametro, nunca hardcodeada) → `Signer` + `Certificate`.
- **RF-02 PEM:** cargar certificado + llave en PEM.
- **RF-03 Seguridad:** la llave nunca se loguea ni aparece en `Debug` (SEC-02); detrás de la feature `pkcs12`; el núcleo sigue compilando sin ella.
## 3. Tests
Cargar un `.pfx` de prueba autogenerado y firmar/verificar; contraseña incorrecta → `Err`; `Debug` no filtra.
## 4. DoD
AGENTS.md §8, incluido `--no-default-features` del núcleo.
## Reporte
_(lo completa el agente)_
