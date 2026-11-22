# sii-sdk

Librería Rust + simulador para Documentos Tributarios Electrónicos (DTE) del SII de Chile.

Gemela de [transbank-SDK](https://github.com/Demiannnnnn/transbank-SDK): librería pública y reutilizable, con **simulador** para desarrollar sin certificados reales del SII.

## Qué cubre (alcance inicial)
- Firma electrónica de DTE (XML + XMLDSig) y sobres de envío.
- Manejo de folios **CAF** (Código de Autorización de Folios).
- Envío al SII y consulta de estado del envío/DTE.
- Libro de compras y ventas.
- Boleta electrónica.
- Simulador local que imita las respuestas del SII para pruebas.

---

## Stack (decidido 2026-11-21)

**Crate Rust** (workspace + simulador + CLI), publicable en crates.io. **Público.** Sin app ni API: lo consumen otros productos (p. ej. rework). Estructura espejo de transbank-SDK.

## Lugar en el ecosistema raysac

Back-office para pymes chilenas junto a **blindmetrics** (asistencia R38 + telemetría), **rework-sistema-facturacion** (facturación / DTE), **sistema-remuneraciones** (sueldos) y **transbank-SDK** (POS). Hub: [`raysac-entreprise-web`](https://github.com/Demiannnnnn/raysac-entreprise-web).

## Estado

🌱 Repositorio recién creado. README para no perder la idea; la implementación viene después.

## Flujo de trabajo

Integración en `develop-AGENTS`; nunca commits directos a `main`. Secretos solo por variables de entorno; `.env` jamás se versiona.
