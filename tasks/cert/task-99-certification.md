# TASK-CERT-99: Certificación contra el SII real (manual, dueño)

| Atributo | Detalle |
|:--|:--|
| **ID** | `CERT-99` |
| **Estado** | ⬜ pendiente |
| **Depende de** | DOCS-01 |
| **Rama** | `dev/cert-99-certification` |
| **Prioridad** | P3 |

## 1. Contexto
Tarea **manual del dueño**, no de agy: certificar el SDK contra el ambiente real de certificación del SII (maullin) con un certificado y CAF reales. Requiere secretos reales → fuera del alcance de los agentes (AGENTS.md §7).
## 2. Pasos (dueño)
- Obtener certificado digital y CAF reales de certificación.
- Configurar `SII_CERT_PASSWORD` y rutas por variable de entorno (nada se versiona).
- Correr el set de pruebas de certificación del SII con la CLI contra maullin.
- Registrar resultados y ajustes necesarios como tareas nuevas.
## 3. Estado
⬜ (dueño). No la ejecuta ningún agente.
## Reporte
_(lo completa el dueño)_
