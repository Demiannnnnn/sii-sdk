# TASK-LIBRO-01: Libro de compra/venta (IECV)

| Atributo | Detalle |
|:--|:--|
| **ID** | `LIBRO-01` |
| **Estado** | ✅ terminada |
| **Depende de** | SIG-01 |
| **Rama** | `dev/libro-01-compra-venta` |
| **Prioridad** | P2 |

## 1. Contexto
`docs/dte-spec.md` §8. Resumen mensual de documentos emitidos/recibidos, firmado.
## 2. Requerimientos
- **RF-01 Modelo:** `LibroCompraVenta` con carátula (periodo, RUT, tipo de operación) y detalle/resumen.
- **RF-02 XML + firma:** serialización al formato IECV del SII y firma XMLDSig (SIG-01).
- **RF-03 Fixture:** un libro de prueba con su test.
## 3. Tests
Serialización vs fixture; totales correctos; libro firmado que verifica.
## 4. DoD
AGENTS.md §8.
## Reporte

### Resumen
Implementación completa del modelo de Información Electrónica de Compras y Ventas (IECV) del SII de Chile (`LibroCompraVenta`), serialización a XML conforme al esquema oficial `LibroCV_v10.xsd`, y firma digital enveloped `XMLDSig` sobre el elemento `<EnvioLibro ID="...">`.
- **RF-01 Modelo:** Implementados `LibroCompraVenta`, `CaratulaLibro`, `PeriodoTributario` (formato YYYY-MM con validación), `TipoOperacion` (`Compra`/`Venta`), `TipoLibro` (`Mensual`/`Especial`/`Rectifica`), `TipoEnvio` (`Total`/`Parcial`/`Final`/`Ajuste`), `TotalesPeriodo` y `DetalleLibro`. Soporta agregación automática de totales por tipo de DTE o totales manuales, y métodos para anexar directamente DTEs (`agregar_dte`, `agregar_dte_timbrado`).
- **RF-02 XML + firma:** Serialización a XML ISO-8859-1 con carátula, resumen de período opcional/calculado, detalles individuales de documentos, y marca de tiempo `TmstFirma` inyectada mediante `Clock`. Firma enveloped `XMLDSig` que referencia el ID del `<EnvioLibro>` e inserta la firma como hermano antes de `</LibroCompraVenta>`.
- **RF-03 Fixture y tests:** Creado fixture oficial `crates/sii/tests/fixtures/libro-venta.xml` y suite de tests de integración en `crates/sii/tests/libro.rs`, validando serialización canónica byte a byte contra el fixture, agregación matemática de totales, verificación criptográfica de la firma del libro, y detección de manipulación (tamper).

### Archivos
- `crates/sii/src/libro.rs`: Implementación completa de los tipos y métodos del modelo y serialización de `LibroCompraVenta`.
- `crates/sii/src/signature.rs`: Inclusión de `</LibroCompraVenta>` como candidato de cierre en `insert_signature_into_xml`.
- `crates/sii/src/lib.rs`: Re-export público de los tipos de `libro` en el API público del crate.
- `crates/sii/tests/fixtures/libro-venta.xml`: Fixture canónico para pruebas de IECV citando fuente oficial del SII.
- `crates/sii/tests/libro.rs`: Suite completa de tests de integración para `LibroCompraVenta`.
- `tasks/libro/task-01-compra-venta.md`: Actualización de estado y reporte de tarea.

### Resultado real de cada comando de AGENTS.md §8
1. `cargo fmt --all --check`:
```
(salida vacía, código 0)
```

2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`:
```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.18s
```

3. `cargo clippy -p sii --no-default-features -- -D warnings`:
```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.09s
```

4. `cargo test --workspace --all-features`:
```
test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.50s (sii unit tests)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.17s (tests/auth.rs)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s (tests/caf.rs)
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s (tests/dte_model.rs)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.58s (tests/envio.rs)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.05s (tests/libro.rs)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.63s (tests/signature.rs)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s (tests/smoke.rs)
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s (tests/ted.rs)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s (sii-cli smoke.rs)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s (sii-sim smoke.rs)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s (doc-tests sii)
```

5. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features`:
```
 Documenting sii v0.1.0 (/Users/demian/Documents/Github/sii-sdk-pro/crates/sii)
 Documenting sii-sim v0.1.0 (/Users/demian/Documents/Github/sii-sdk-pro/crates/sii-sim)
 Documenting sii-cli v0.1.0 (/Users/demian/Documents/Github/sii-sdk-pro/crates/sii-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.93s
   Generated /Users/demian/Documents/Github/sii-sdk-pro/target/doc/sii/index.html and 2 other files
```

### Decisiones
- El orden de las etiquetas XML sigue rigurosamente el esquema normativo `LibroCV_v10.xsd`: en `<TotalesPeriodo>` los montos van en orden `TotMntExe`, `TotMntNeto`, `TotMntIVA`, `TotMntTotal`. En `<Detalle>` los montos siguen `MntExe`, `MntNeto`, `MntIVA`, `MntTotal`.
- Si un libro tiene detalles y no se especificaron totales manuales, `calcular_totales()` calcula y agrupa automáticamente los totales por tipo de DTE en orden numérico ascendente de código de tipo de documento. Los documentos anulados incrementan `tot_doc` y `tot_anulado` pero no suman a los montos netos/exentos/IVA del período.
- La firma XMLDSig utiliza `sign_targeting_before` referenciando el `ID` del `<EnvioLibro>` y situando `<Signature>` antes del tag de cierre `</LibroCompraVenta>`, coincidiendo con el estándar del SII para libros electrónicos.
- Para interoperabilidad con `serde` bajo la feature `serde`, `CaratulaLibro` y `DetalleLibro` implementan serde custom serializando `time::Date` mediante el formato canónico `YYYY-MM-DD` sin depender de crates externos.

### Pendientes / bloqueos
- Ninguno. La tarea está completamente terminada y todos los criterios de aceptación y verificación están cumplidos.

### Desviaciones de la spec
- Ninguna._
