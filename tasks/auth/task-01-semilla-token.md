# TASK-AUTH-01: Semilla y token del SII

| Atributo | Detalle |
|:--|:--|
| **ID** | `AUTH-01` |
| **Estado** | ⬜ pendiente |
| **Depende de** | SIG-01 |
| **Rama** | `dev/auth-01-semilla-token` |
| **Prioridad** | P1 |

## 1. Contexto
`docs/dte-spec.md` §7 (autenticación). Modelo sans-IO: construir la solicitud de token firmando la semilla; el transporte lo hace TRANSPORT-01.
## 2. Requerimientos
- **RF-01 Semilla:** tipos para la respuesta de semilla del SII y su parseo.
- **RF-02 Firma semilla:** `firmar_semilla(semilla, &dyn Signer) -> SolicitudToken` (XMLDSig sobre `getToken/item/Semilla`), reproducible.
- **RF-03 Token:** parseo de la respuesta de token; tipo `Token` con su valor (tratado como secreto en logs, SEC-02).
## 3. Tests
Firma de semilla reproducible y verificable; parseo de respuestas de token ok y con error.
## 4. DoD
AGENTS.md §8. `--no-default-features` ok (sans-IO).
## Reporte
_(lo completa el agente)_
