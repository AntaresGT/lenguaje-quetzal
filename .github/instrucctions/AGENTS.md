# Guía General para Agentes de IA del Proyecto Quetzal

Este documento define reglas unificadas para TODAS las IAs colaboradoras (Claude, Gemini, GPT, Copilot u otras). Su propósito es asegurar consistencia en estilo, arquitectura, calidad y alineación con los principios del lenguaje Quetzal.

## 1. Objetivo del Proyecto
Construir e iterar sobre un intérprete del lenguaje Quetzal (v0.0.2+) en Rust, con sintaxis en español, tipado fuerte, soporte nativo para JSON, manejo estructurado de errores y enfoque educativo/documentado.

## 2. Principios Rectores
1. Claridad sobre complejidad: Implementaciones simples, extensibles y comentadas en español.
2. Coherencia terminológica: Siempre usar nombres y comentarios en español (snake_case para funciones/variables, PascalCase para tipos/objetos del lenguaje Quetzal cuando aplique).
3. Seguridad y robustez: Validar entradas, manejar errores con códigos formales (E0001–E0999) y evitar pánico innecesario.
4. Evolución incremental: Cambios pequeños, atómicos y justificables.
5. Trazabilidad: Explicar en commit o comentarios por qué (no solo qué) se modificó.
6. Rendimiento razonable: Optimizar solo tras medir o en rutas calientes claras (lexing, parsing, evaluación y estructuras de datos internas).

## 3. Estructura Esperada del Código
Ver `copilot-instructions.md` y directorio `src/`. Mantener consistencia con los módulos:
- `analizador_lexico.rs`
- `analizador_sintactico.rs`
- `evaluador.rs`
- `interprete.rs`
- `maquina_virtual.rs`
- `tipos_datos.rs`
- `consola.rs`
- `errores.rs`
- `manejador_modulos.rs`

Separar responsabilidades: no mezclar parsing con evaluación; no incrustar lógica de E/S en tipos de datos. Preferir funciones puras cuando sea posible.

## 4. Convenciones de Estilo
- Comentarios y documentación SIEMPRE en español.
- Evitar abreviaciones crípticas: preferir `indice_actual` vs `idx` salvo dentro de bucles claros.
- Explicar algoritmos no triviales: objetivo, complejidad, invariantes.
- Tests: nombre descriptivo en español (`prueba_convierte_texto_a_entero`).
- Errores: usar helper centralizado para formato uniforme.

### 4.2 Regla de Mutabilidad por Defecto
Todas las variables son CONSTANTES por defecto. Solo se vuelven mutables si se incluye explícitamente la palabra reservada `var` inmediatamente después del tipo:
```
entero contador = 0          // constante
entero var contador = 0      // mutable
número var valor_algo = 82.20
```
Buenas prácticas:
1. Usar `var` únicamente cuando el valor cambie lógicamente (estado acumulativo, iteradores, estructuras que se modifican).
2. Evitar `var` preventivo. Primero implementar inmutable; mutar solo si es necesario.
3. En funciones, parámetros marcados con `var` indican intención de modificación interna controlada.
4. No añadir `var` a tipos que nunca se re-asignan tras inicialización.
5. En ejemplos pedagógicos, mostrar ambos patrones (inmutable y mutable) para claridad.

### 4.3 Política de Documentación y Pruebas Auxiliares
1. Prohibido crear nueva documentación en archivos `.md` adicionales. La única documentación aceptada desde ahora será dentro del propio código fuente mediante comentarios claros y concisos.
2. Cualquier prueba, experimento, exploración o script generado por las IAs que NO sea parte directa del core del intérprete deberá ubicarse en la carpeta raíz `pruebas-ia/` (crear si no existe). Esto evita mezclar prototipos con el código estable.
3. No incluir emojis en el código, comentarios, mensajes de error, nombres de archivos ni commits.
4. Si se necesita proponer una guía o nota conceptual, hacerlo como comentario temporal en código con prefijo:
```
// PROPUESTA (migrar a código definitivo si se acepta): <descripción corta>
```
Eliminarlo una vez incorporado.
5. Si se detecta documentación obsoleta en `.md`, planear migración paulatina a comentarios en código y luego eliminar el archivo solo tras confirmación.

### 4.1 Política de Tildes en Palabras Reservadas
El lenguaje Quetzal debe aceptar y reconocer tanto la forma con tilde como la forma sin tilde de las palabras reservadas que en español llevan acento ortográfico. Ejemplos (pares equivalentes):
```
número / numero
vacio / vacío (solo se usa vacio en ejemplos; soporte futuro para "vacío" debe mantener ambas formas)
publico / público
privado (no varía: no lleva tilde)
log / lóg
excepcion / excepción
```
Reglas:
1. Forma CANÓNICA preferida en repositorio: con tilde cuando la palabra la lleva en español estándar (`número`, `público`, `lóg`, `excepción`). Parser debe aceptar variante sin tilde (`numero`, `publico`, `log`, `excepcion`).
2. Agentes deben NO rechazar código que use o no tilde mientras la palabra base coincida.
3. Al generar ejemplos nuevos, priorizar forma CON tilde cuando exista (promueve corrección lingüística): `número`, `público`, `lóg`, `excepción`.
4. Si se introduce nueva palabra reservada con tilde potencial, documentar ambos alias.
5. Nunca mezclar dentro de la MISMA función variantes distintas sin motivo; elegir una forma y mantenerla consistente.
6. Tests pueden incluir casos mixtos para asegurar tolerancia léxica.

Nota: Esta política no autoriza crear variantes arbitrarias; solo las diferencias de acentuación reconocidas en español estándar.

## 5. Sistema de Errores
- Rango y categorías según documentación (`contextos/10_codigos_error.md`).
- Formato:
  ```
  error[E0XYZ]: descripción clara
	--> archivo.qz:línea:columna
	 | código relevante
	 | ^ indicador
	 = ayuda: sugerencia
  ```
- No inventar códigos fuera de rango. Si se requiere nuevo, documentar propuesta en comentario `// PROPUESTA: E0ABC - descripcion`.

## 6. Tipos y Conversión
Todos los tipos de valor deben soportar conversiones documentadas (`.texto()`, `.entero()`, `.numero()`, `.log()`, `.lista()`, `.jsn()`). Conservar semántica de precisión y evitar pérdida silenciosa; retornar error tipado cuando la conversión no sea válida.

## 7. Reglas para Generación de Código por IA
Antes de proponer un cambio:
1. Leer archivo relevante (evitar sobrescrituras ciegas).
2. Explicar breve intención de modificación.
3. Mantener imports mínimos y ordenados.
4. No introducir dependencias sin justificar su necesidad.
5. Mantener compilación limpia (sin warnings evitables).

## 8. Flujo Colaborativo Multi-Agente
1. Un solo agente debe asumir rol de ORQUESTADOR por interacción (coordina y sintetiza).
2. Otros agentes pueden:
	- Auditar lógica
	- Proponer optimizaciones
	- Generar tests adicionales
3. Resolución de conflicto: preferir solución más simple que pase tests y mantenga semántica definida.
4. Si hay ambigüedad semántica del lenguaje, registrar en comentario `// PENDIENTE: aclarar comportamiento de X`.

## 9. Pruebas
- Ubicación: `src/pruebas/`
- Incluir casos: camino feliz + bordes (nulo, lista vacía, número grande, recursión profunda, JSON anidado).
- Probar errores esperados: usar patrón de asserts sobre mensaje/código.
- Evitar dependencias externas en tests (pureza y reproducibilidad).

## 10. Rendimiento
- Evitar clonaciones innecesarias (`to_string()` redundante, copias grandes de JSON/listas).
- Usar referencias y slices cuando posible.
- Considerar `bumpalo` o pools solo en rutas realmente calientes (ya presente en dependencias).

## 11. Módulos y Carga
- `manejador_modulos.rs` debe centralizar resolución de rutas y caché.
- Evitar lógica de interpretación duplicada en otros módulos.

## 12. Seguridad y Robustez
- Validar índices de listas/matrices.
- Limitar profundidad recursiva (usar `stacker` si necesario) y documentar límites.
- Sanitizar entrada interactiva (no confiar en formato JSON sin parsing seguro).

## 13. JSON y Listas
- Acceso híbrido: notación punto y `[...]` deben coexistir.
- Mantener distinción entre lista y objeto JSON.
- Preservar orden de inserción si el modelo lo asume (documentar si cambia).

## 14. Consola Global
`consola.rs` centraliza estilos de salida. No replicar lógica de color o formato en otros archivos.

## 15. Documentación Interna
Para cada función pública compleja incluir:
```rust
/// Descripción breve
/// Parámetros: ...
/// Retorna: ...
/// Errores: lista de códigos relevantes
```

## 16. Estrategia ante Errores de IA
Si un modelo sugiere código incompatible:
1. No aplicar automáticamente.
2. Producir diff explicativo.
3. Justificar rechazo o adaptación.

## 17. Qué NO Hacer
- No traducir palabras clave de Rust al español.
- No introducir macros complejas sin necesidad.
- No mezclar responsabilidades (Single Responsibility Principle adaptado).
- No cambiar formato de errores ya estandarizado.

## 18. Ejemplo de Aporte Correcto
1. Agregar método `esta_vacia()` a tipo lista:
	- Test en `metodos_listas.rs` cubriendo lista vacía y no vacía.
	- Implementación documentada evitando copiar toda la lista.
	- Actualizar documentación si expone nueva API al usuario del lenguaje.

## 19. Registro de Decisiones Técnicas (RDT)
Agregar comentario `// RDT: <fecha ISO> - decisión y motivación` en cambios arquitectónicos o semánticos clave.

## 20. Actualización de estas Instrucciones
Puede ampliarse solo si:
1. Se alcanza una nueva versión del lenguaje.
2. Se introduce un subsistema (optimizador, depurador, JIT, etc.).
3. Cambia el formato de errores.

Registrar cambios con entrada en commit: `docs(instructions): ...`.

---
Esta guía es la referencia canónica para colaboración multi-IA en Quetzal. Toda desviación debe justificarse y documentarse.

