# Informe de estabilización VM/JIT — rama 0.0.3

## Veredicto

**NO ESTABILIZADO** como veredicto global multiplataforma.

**Linux x86_64 sí supera las puertas locales de estabilidad, compatibilidad y costo/beneficio.** En esa plataforma el build release normal activa `ModoJit::Automatico` sin flags; `--sin-jit` conserva la salida de emergencia y otras plataformas no reciben activación automática indiscriminada.

El veredicto global no se cierra porque la primera matriz Windows/CI detectó dos fallos y aún falta una nueva ejecución completa tras las correcciones. Tampoco se afirma haber cubierto una ejecución prolongada en cada plataforma. La divergencia de llamadas directas JIT en Windows quedó corregida y validada localmente, pero su confirmación en Windows sigue pendiente.

## Alcance y línea base

- Rama: `trabajo/estabilizar-vm-jit-0.0.3`, derivada de `647cfe73e035799393315835e8e025a52a9a0fd0`.
- Referencias: `v0.0.2 = 533807f3066ebd6ad21c5885b360fbf07dee5965`; rama 0.0.3 original = `647cfe73e035799393315835e8e025a52a9a0fd0`.
- Entorno: Rust 1.99.0, Linux x86_64, Intel i7-6700HQ, 15 GiB RAM; detalles en `pruebas/resultados/linea-base/entorno.txt`.
- La línea base completa pasó antes de editar. Logs y salidas originales: `pruebas/resultados/linea-base/`.
- Se congelaron 19 ejemplos deterministas en `pruebas/compatibilidad/casos.json` y `pruebas/resultados/linea-base/oraculo/`.
- `--sin-optimizar` desactiva especialización, cachés y JIT de ejecución; el generador de bytecode sigue optimizando y verificando. El oráculo crudo usa `generar_modulo_sin_procesar()` más `verificar_modulo()`.

## Optimizaciones y decisiones

| Optimización o mecanismo | Estado | Motivo y evidencia |
|---|---|---|
| Bytecode optimizado y verificado | Activado | Se conserva el pipeline existente y se añadió equivalencia original/optimizado. |
| Verificación en toda entrada de VM | Activado | `Vm::cargar_modulo()` rechaza módulos externos inválidos antes de ejecutar. |
| Flujo estructurado de excepciones | Activado | Se rechazan salidas sin región activa, uniones inválidas y limpiezas fuera de rango. |
| Revisión de caché del optimizador | Activado | Formato incrementado a 6 y revisión incluida en la clave; fría/caliente/corrupta probadas. |
| Despacho rápido, especialización y cachés | Activado | Comparación de cuatro niveles y pruebas mono/polimórficas. |
| Compilación anticipada al cargar | Rechazada por defecto | Evita costo de arranque; puede configurarse explícitamente con presupuesto. |
| JIT durante ejecución en Linux x86_64 | Activado, selectivo | Requiere 1 000 retrocesos o 1 024 llamadas en modo automático. |
| Umbrales antiguos de 4 llamadas | Solo `--jit` explícito | En automático producían +11,06 % en funciones pequeñas y +7,33 % en no elegible. |
| Copias `split_off`/`splice` en llamadas | Eliminada | Las rutas frías y guardas fallidas dejan intacta la pila. |
| Copias completas de módulos para identidad JIT | Eliminada | Identidad por `Rc` y referencias débiles; no se retienen módulos al compilarse. |
| `NoElegible` frente a `NoDisponible` | Activado | Presupuesto, indisponibilidad y rechazo permanente tienen estados diferenciados. |
| Métricas detalladas | Opt-in | Elegibles, descartes, guardas y ejecuciones nativas; no se imprimen en salida normal. |
| JIT de objetos arbitrarios, `jsn`, red, archivos o suspensiones | Rechazado | Esas funciones permanecen en el intérprete. |
| OSR/desoptimización a mitad de efectos | Diferido | Un llamado caliente continúa interpretado si no hay transición segura; no se duplican efectos. |
| Ejecutables nativos persistentes | Rechazado | Cranelift solo genera memoria ejecutable dentro del proceso. |
| Activación automática en Windows/otros objetivos | No activada | Falta validación de plataforma; CI quedó configurada. |

## Fallos encontrados y corregidos

1. **Módulos externos sin verificación de frontera.** La VM terminaba con `E9999`; ahora rechaza con `E0410` antes de ejecutar. Regresión: `pruebas/runtime/tests/verificacion.rs`.
2. **`SalirIntentar` sin región activa aceptado.** Se añadió análisis de profundidad de manejadores y uniones de flujo. Regresiones: `pruebas/bytecode/tests/verificador_adversarial.rs`.
3. **Compilación JIT anticipada.** Funciones con bucles se compilaban al cargar. La anticipación predeterminada ahora es falsa; `pruebas/runtime/tests/jit_frio.rs` lo impide.
4. **Retención de módulos completos por el JIT.** `ContextoJit` usaba `Rc::new(modulo.clone())`; ahora usa identidad y `Weak`, con prueba de liberación.
5. **Sobrecosto en llamadas frías.** Se eliminaron vectores y reinserción de argumentos en la ruta de pila.
6. **Fallback permanentemente contado.** Una función rechazada deja de pagar observación en cada llamada; el fallback se registra en la transición.
7. **Regresión por umbral automático demasiado bajo.** La primera medición falló las puertas; la selección por calor observado la corrigió. Evidencias antes/después: `INFORME_UMBRAL_4.md`, `resultados-seleccion-1.json` y `resultados.json`.
8. **ABI incorrecta en llamadas directas JIT en Windows.** La llamada indirecta forzaba `CallConv::SystemV` aunque el llamado usaba la convención del objetivo; ahora ambos usan `generador.func.signature.call_conv`. La regresión de 10 000 llamadas conserva el resultado y muestra el valor real si diverge. Linux vuelve a pasar toda la suite; falta confirmación del próximo CI en Windows.

No se eliminaron ni ignoraron tests para obtener suites verdes.

## Pruebas y compatibilidad

- Cuatro niveles en `pruebas/runtime/tests/diferencial.rs`: original verificado + VM genérica, optimizado sin especialización, VM optimizada y JIT.
- 38 pruebas diferenciales, incluidos errores, efectos, Unicode, objetos, asincronía y 32 semillas generativas.
- Equivalencia del optimizador: `pruebas/bytecode/tests/equivalencia_optimizador.rs`.
- Bytecode corrupto y 512 mutaciones reproducibles sin pánicos: `verificador_adversarial.rs`.
- Caché fría/caliente/corrupta y bytecode serializado inválido: `pruebas/paquetes/tests/cache_bytecode.rs`.
- CLI real con procesos, tiempo límite y temporales: `crates/lnc/tests/`.
- Los 19 oráculos pasan en modo normal y con `--sin-jit`, con y sin feature nominal.
- Memoria: 24 ciclos de módulos calientes, presupuesto de 256 funciones/16 MiB, liberación de entornos y código al soltar VM.
- `unsafe` sigue prohibido en VM, bytecode y motor; las llamadas nativas aisladas del JIT conservan contratos de aridad, vida del módulo y buffers.

## Rendimiento medido

Dos ejecuciones release del medidor validaron salidas idénticas antes de comparar tiempos. Última repetición; 31 muestras en cortos y 7 en cargas pesadas:

| Escenario | Sin JIT | Automático | Aceleración | Δ |
|---|---:|---:|---:|---:|
| Script corto | 3,686 ms | 3,641 ms | 1,012× | −1,22 % |
| Funciones pequeñas | 3,709 ms | 3,651 ms | 1,016× | −1,58 % |
| Bucles cortos | 3,754 ms | 3,682 ms | 1,020× | −1,92 % |
| Llamadas repetidas | 40,812 ms | 39,810 ms | 1,025× | −2,45 % |
| Bucle caliente | 143,972 ms | 6,378 ms | 22,572× | −95,57 % |
| No elegible/recursión | 15,531 ms | 15,249 ms | 1,019× | −1,82 % |

La primera repetición independiente dio resultados equivalentes: cortos entre −0,79 % y +0,29 %, y 22,608× en el bucle caliente. No se observó regresión superior al 5 % en cortos ni al 10 % en escenarios principales.

Histórico del bucle: 912,902 ms en `v0.0.2`, 143,741 ms en 0.0.3 original, 6,378 ms con el modo automático actual. La comparación central de aprobación es VM actual sin JIT frente a automático, no Python.

RSS máximo: 11 372 KiB sin JIT y 11 356/11 368 KiB en automático para corto/caliente. Caché fría: 3,346 ms automático; caliente: 3,570 ms. Datos, dispersión y muestras: `pruebas/resultados/rendimiento/resultados.json`, `resultados.csv` e `INFORME_RENDIMIENTO.md`.

## Validación final ejecutada

Todos estos registros terminaron con código 0 (`pruebas/resultados/validacion-final/estados.txt`):

- `cargo fmt --all -- --check`
- `cargo check --workspace --locked`
- `cargo test --workspace --locked --no-fail-fast`
- `cargo test -p pruebas_runtime --features jit --locked --no-fail-fast`
- `cargo test -p compilador_jit --features nativo --locked --no-fail-fast`
- `cargo test -p lnc --features jit --locked --no-fail-fast`
- Clippy de todos los crates afectados, todos los targets/features y `-D warnings`.
- `cargo build -p lnc --release --features jit --locked`
- `cargo build -p lnc --release --locked` sin feature explícita.

El binario release normal superó ocho ejecuciones: `quetzal ejecutar`, `quetzal ejecutar programa.qz`, `quetzal programa.qz`, `--sin-jit`, `--sin-optimizar`, `--jit` y caliente con/sin JIT. Salidas, errores y estados están en `pruebas/resultados/validacion-final/cli.json`.

## Trabajo restante para cerrar el veredicto global

1. Repetir la matriz Linux/Windows para confirmar la corrección de ABI y resolver la divergencia restante de `compatibilidad_cli`; activar automáticamente solo los objetivos que la superen.
2. Repetir estrés prolongado y compatibilidad completa en cada plataforma soportada; las 24 iteraciones y RSS local no sustituyen esa campaña.
3. Si aparece una divergencia generativa, conservar el caso mínimo como regresión permanente y repetir las cuatro rutas antes de habilitar más funciones JIT.

No se cambió la versión pública, no se crearon etiquetas/publicaciones y no se modificó `quetzal compilar` ni la generación de ejecutables Quetzal.
