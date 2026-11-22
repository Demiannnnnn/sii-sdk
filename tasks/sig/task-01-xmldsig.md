# TASK-SIG-01: Firma XMLDSig (Signer/Certificate, C14N)

| Atributo | Detalle |
|:--|:--|
| **ID** | `SIG-01` |
| **Estado** | ⬜ pendiente |
| **Depende de** | DTE-01 |
| **Rama** | `dev/sig-01-xmldsig` |
| **Prioridad** | P0 |

## 1. Contexto
`docs/dte-spec.md` §6, `constraints.md` (SIG-01..03), ADR-02. La firma se inyecta vía trait; el núcleo no abre certificados.

## 2. Requerimientos
- **RF-01 Traits:** `trait Signer` (firma bytes con la llave del emisor) y `trait Certificate` (expone el X.509). El núcleo no toca el SO; un impl de prueba (llave RSA de juguete) vive en tests.
- **RF-02 Firmar:** firma **enveloped** XMLDSig sobre `<Documento>` con C14N determinista, digest y `SignatureMethod` que acepta el SII. Fija el algoritmo exacto por ambiente desde `docs/dte-spec.md` §6 y déjalo configurable; `KeyInfo` con el certificado. Reproducible con llave+hora fijas.
- **RF-03 Verificar:** `verify(&[u8]) -> Result<bool, Error>` real; firma/digest alterados → `false` (SIG-02).
- **RF-04 Fixtures:** documento firmado de prueba; test de ida y vuelta (firmar→verificar) y de detección de alteración.

## 3. Tests
Firma reproducible; verificación ok; alteración detectada; C14N estable ante reordenamiento irrelevante de espacios.

## 4. DoD
AGENTS.md §8.

## Reporte
_(lo completa el agente)_
