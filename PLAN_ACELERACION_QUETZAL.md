# Plan de aceleración del lenguaje Quetzal

## 1. Objetivo y límites

Acelerar `quetzal ejecutar` sin cambiar programas `.qz`, mensajes de error, permisos ni efectos observables. Quetzal seguirá siendo interpretado: el JIT solo traduce funciones elegibles a memoria ejecutable durante la ejecución; nunca exige compilación previa ni genera ejecutables del programa.

Prioridad: mejorar tiempos reales de ejecución y arranque, no aumentar infraestructura de pruebas. Se reutilizarán las suites y los benchmarks existentes. Solo se añadirá una regresión si aparece un defecto nuevo que no esté cubierto.

Terminología: se usa **referencia esperada** para salidas y comportamientos congelados de versiones anteriores.

## 2. Punto de partida medido

- Bucle entero caliente: aproximadamente 22,6× con JIT frente a VM optimizada sin JIT.
- Script corto, funciones pequeñas y ruta no elegible: sin regresión estable superior al 2 % en las últimas mediciones.
- Llamadas pequeñas repetidas: mejora actual pequeña; la ABI y las llamadas indirectas todavía cuestan demasiado.
- Carga de módulos: generación y verificación pueden repetir trabajo al entrar en la VM.
- Memoria prolongada: la purga de identidades muertas ya evita crecimiento no acotado; no se reabrirá ese trabajo salvo evidencia nueva.

## 3. Orden de ejecución

Cada cambio debe ser pequeño y medido por separado con los benchmarks existentes. Se descarta cualquier optimización que altere semántica, aumente más del 5 % los programas cortos o no compense su costo.

### Etapa 1 — Eliminar trabajo redundante en llamadas JIT

**Archivos:** `crates/compilador_jit/src/traductor.rs`, `analisis.rs`, `contexto.rs`.

1. Detectar llamadas directas cuyo destino esté probado por el análisis estático.
2. Incrustar primero funciones sin efectos de la forma `RetornarLocal(parámetro)`.
3. Reutilizar el valor ya presente en la pila: no invocar ABI, no reservar buffer de argumentos y no escribir `SalidaEntera`.
4. No compilar dependencias que solo existen para ser incrustadas, si el análisis demuestra que no se llaman de otra forma.
5. Ampliar después a expresiones puras de una sola instrucción: negación y aritmética entera con las mismas comprobaciones de desbordamiento/división por cero.
6. No incrustar recursión, objetos, globales mutables, llamadas nativas, `esperar`, excepciones ni control de flujo hasta tener un modelo de estado probado.

**Medida:** `ejecucion/llamadas_pequenas`, `llamada_nativa_pequena`, recursión y cargas con más de cuatro argumentos.

### Etapa 2 — Reducir costo de compilación JIT

**Archivos:** `crates/compilador_jit/src/contexto.rs`, `generador.rs`, `traductor.rs`.

1. Registrar por función el tiempo de análisis, generación y finalización, solo con métricas habilitadas.
2. Evitar crear estructuras de análisis cuando el estado ya sea `NoElegible` o `NoDisponible`.
3. Reutilizar buffers de traducción y tablas de bloques sin retener módulos.
4. Evaluar un propietario compartido de código nativo por VM, sin punteros colgantes y manteniendo límites de memoria.
5. Conservar compilación diferida; no reintroducir compilación general al cargar.

**Medida:** primera llamada caliente, carga de proyecto grande, bytes de código y tiempo total de proceso.

### Etapa 3 — Evitar verificación duplicada sin debilitar seguridad

**Archivos:** `crates/bytecode/src/verificador.rs`, `crates/motor/src/cargador.rs`, `crates/maquina_virtual/src/vm.rs`, `crates/paquetes/src/cache.rs`.

1. Mantener verificación en toda frontera no confiable: cache, deserialización, bytecode embebido y API pública.
2. Representar con un tipo `ModuloVerificado` que un módulo ya pasó el verificador; no usar un booleano falsificable.
3. Hacer que generador y cache devuelvan ese tipo solo tras verificar.
4. Aceptar el tipo en la ruta interna de la VM sin repetir análisis; conservar `cargar_modulo` verificando para entradas arbitrarias.
5. No propagar el estado de confianza entre módulos importados sin verificar cada uno.

**Medida:** arranque frío/caliente, carga de muchos módulos y verificación de entradas corruptas existente.

### Etapa 4 — Reducir costo del despacho de la VM

**Archivos:** `crates/maquina_virtual/src/vm.rs`, `codigo.rs`, `valores.rs`.

1. Perfilar antes de tocar representaciones: frecuencia de instrucciones, clones de `Valor`, accesos a locales y búsquedas de `RefCell`.
2. Revisar las operaciones calientes restantes en `ejecutar_instruccion` frente a `correr_hasta_modo::<true>`.
3. Mover a la ruta rápida solo operaciones con equivalencia ya cubierta: retorno, comparaciones, guardas y acceso a locales.
4. Reservar pila y marcos una vez por llamada cuando `max_pila` lo permita; no eliminar validaciones sin una precondición del verificador.
5. Evitar clones de `Rc` en instrucciones enteras; conservar propiedad compartida en objetos, textos y listas.
6. Mantener el mismo límite de recursión y restauración de marcos tras excepciones.

**Medida:** VM genérica frente a rápida, aritmética sin JIT, recursión y excepciones.

### Etapa 5 — Mejorar cachés en línea y miembros

**Archivos:** `crates/maquina_virtual/src/vm.rs`, `codigo.rs`, `valores.rs`.

1. Mantener validación por identidad de clase/layout en cada acceso especializado.
2. Reducir búsquedas de nombres y conversiones repetidas en sitios `Nativa`/`Externa` ya resueltos.
3. Revisar cachés polimórficas de dos entradas; evitar crecimiento por receptores cambiantes.
4. Volver a ruta genérica en tipos variables, sin reespecialización oscilante.
5. No aplicar JIT a objetos arbitrarios: la ganancia debe venir del despacho y de accesos directos seguros.

**Medida:** atributos, escrituras, métodos nativos, receptores monomórficos y polimórficos.

### Etapa 6 — Ajustar selección automática por costo amortizado

**Archivos:** `crates/maquina_virtual/src/codigo.rs`, `vm.rs`, `crates/compilador_jit/src/contexto.rs`.

1. Estimar costo de compilación por cantidad de instrucciones y tiempo observado.
2. Incorporar el beneficio de incrustación antes de decidir compilar una función o su dependencia.
3. Evitar compilar funciones que se ejecutan pocas veces o cuyo cuerpo cuesta menos que la transición.
4. Mantener 1 000 retrocesos/1 024 llamadas como punto medido inicial; cambiar umbrales solo con datos.
5. Presupuestos limitados de funciones, bytes y tiempo; fallback invisible al usuario.

**Medida:** cortos, una sola llamada, funciones pequeñas repetidas y bucles calientes.

### Etapa 7 — Acelerar valores sin cambiar precisión ni Unicode

**Archivos:** `crates/maquina_virtual/src/valores.rs`, `nativos.rs`, `crates/modulos_nativos/src/`.

1. Listas: evitar realocaciones cuando la longitud sea conocida; revisar iteradores y conversiones.
2. `jsn`: evitar búsquedas/clonaciones repetidas de claves y valores en caminos calientes.
3. Texto: revisar índices por carácter y construcción de cadenas; no indexar por bytes si cambia semántica UTF-8.
4. Decimales: no sustituir `Decimal` por `f64`; optimizar conversiones y asignaciones sin perder exactitud.
5. No extender JIT a estos valores hasta disponer de interoperabilidad segura.

**Medida:** benchmarks existentes de listas, `jsn`, decimales y Unicode.

### Etapa 8 — Revisar carga, asincronía y nativos

**Archivos:** `crates/motor/src/cargador.rs`, `crates/maquina_virtual/src/bucle_eventos.rs`, `nativos.rs`.

1. Evitar clones completos de módulos/entornos en imports y fragmentos REPL.
2. Mantener un solo bucle de eventos y drenado acotado.
3. Reducir conversiones entre `Valor` y cargas nativas en llamadas repetidas.
4. No mover red, archivos, callbacks ni suspensiones al JIT.
5. Conservar permisos antes de cualquier optimización de E/S.

**Medida:** proyectos con imports, tareas repetidas, temporizadores, servidores y finalización del bucle.

## 4. Secuencia de commits prevista

1. Incrustar `RetornarLocal(parámetro)` en llamadas directas JIT.
2. Incrustar operaciones puras simples, solo si la primera mejora es clara.
3. Reducir estructuras temporales de compilación JIT.
4. Introducir bytecode verificado por tipo para eliminar duplicación segura.
5. Aplicar una mejora de despacho VM confirmada por perfil.
6. Aplicar una mejora de miembros/cachés confirmada por perfil.
7. Ajustar selección por costo amortizado.
8. Optimizar listas/texto/`jsn`/decimales solo con una medición concreta.
9. Revisar imports, asincronía y conversiones nativas.
10. Cerrar documentación de rendimiento y plataforma; no publicar hasta que las puertas funcionales existentes pasen.

## 5. Criterios de aceptación y reversa

- Cero cambios en resultados, errores, permisos y orden de efectos.
- Sin regresión estable superior al 5 % en arranque/cortos ni al 10 % en un escenario principal.
- JIT solo cuando el ahorro supere compilación y transición.
- Sin crecimiento sostenido de memoria ni metadatos.
- Sin `unsafe` nuevo fuera del crate JIT y sin `unsafe` nuevo en VM/bytecode/motor.
- Revertir una optimización que no muestre beneficio neto, aunque sea técnicamente correcta.
- No crear más infraestructura de pruebas: ejecutar las suites y benchmarks existentes antes de continuar con la siguiente etapa.
