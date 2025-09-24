## Guía Específica para Modelos GPT en Proyecto Quetzal

Los modelos GPT (OpenAI) se enfocan en generación de código robusto, expansión de pruebas y creación de prototipos rápidos manteniendo calidad.

### 1. Rol Principal
- Escribir implementaciones completas (lexer, parser, evaluador, VM, tipos).
- Generar suites de prueba adicionales cubriendo bordes.
- Integrar nueva funcionalidad siguiendo patrones existentes.
- Proponer optimizaciones con explicación de impacto.

### 2. Estilo de Respuesta
- Estructurado por secciones: "Contexto", "Cambio", "Justificación", "Impacto en Tests".
- Explicar invariantes mantenidas.
- Omitir verbosidad innecesaria.

#### 2.1 Política de Tildes
Al generar código, preferir siempre variante con tilde para palabras reservadas que la acepten (`número`, `público`, `lóg`, `excepción`). El parser debe soportar ambas (`numero`, `publico`, `log`, `excepcion`). No introducir variantes híbridas dentro de un mismo bloque. Tests nuevos deben incluir al menos un caso usando la forma alternativa sin tilde para validar tolerancia.

### 3. Reglas para Código Propuesto
1. No reescribir archivos completos salvo necesidad.
2. Mantener nombres en español y comentarios claros.
3. Validar mentalmente compilación (evitar imports huérfanos, tipos inconsistentes).
4. Añadir tests cuando se expone un nuevo método, palabra clave o regla sintáctica.
 5. Inmutabilidad por defecto: declarar variables sin `var` siempre que NO se reasignen. Usar `var` solo cuando exista al menos una operación posterior que cambie el valor o reemplace la estructura (ver `AGENTS.md` sección mutabilidad).

### 4. Manejo de Errores
Al introducir nuevo error:
```
// PROPUESTA: E02XY - descripción corta
```
Explicar categoría y ejemplos de activación.

### 5. Optimización Responsable
- Justificar con: complejidad actual, propuesta, mejora esperada.
- No micro-optimizar rutas no críticas.

### 6. Ejemplo de Plantilla de Respuesta
```
Contexto: Falta soporte para método longitud() en JSON.
Cambio: Implementar trait Longitud para listas y JSON.
Justificación: Unificar API y simplificar validaciones.
Impacto en Tests: Añadir casos json vacío, json simple, json anidado.
```

### 7. Casos a Rechazar / Escalar
- Requerimientos vagos -> pedir aclaración mínima.
- Cambios que colisionan con semántica establecida -> citar archivo fuente.

### 7.1 Política de Documentación y Pruebas Auxiliares
- No crear nuevos archivos `.md` de documentación. Toda explicación adicional debe ir como comentario en el código directamente sobre la funcionalidad afectada.
- Para pruebas exploratorias, benchmarks temporales o scripts no definitivos usar la carpeta raíz `pruebas-ia/`. No dejar prototipos sueltos en `src/` ni en otros directorios.
- Prohibido usar emojis en código, comentarios, mensajes de error o nombres.
- Propuestas conceptuales: usar comentario temporal `// PROPUESTA (evaluar): <detalle>` y retirar al consolidar.

### 8. Checklist Previa a Entrega
- [ ] Consistencia con `AGENTS.md`
- [ ] Código idiomático Rust
- [ ] Comentarios en español
- [ ] Errores categorizados correctamente
- [ ] Tests agregados/ajustados
- [ ] `var` usado únicamente donde hay reasignación real
- [ ] Sin nuevos `.md` creados
- [ ] Pruebas exploratorias en `pruebas-ia/`
- [ ] Sin emojis

### 9. Ejemplo de Mejora de Test
Antes:
```
assert_eq!(evaluar("1+2"), 3);
```
Después (más robusto):
```
// Verifica preservación de tipo entero
let r = evaluar("entero var a = 1\na + 2");
assert!(r.es_entero());
assert_eq!(r.entero(), 3);
```

### 10. Uso de Traits y Rasgos
- Prefiera traits pequeños y cohesionados.
- Evitar abuso de macros para simplificar lógica trivial.

### 11. Interoperabilidad JSON
Confirmar al modificar: acceso por clave, serialización y conversión a `.texto()`.

### 12. Ejemplo de Respuesta Modelo
```
Contexto: Falta método quitar() en listas mutables.
Cambio: Agregar función que remueve por índice y retorna valor.
Justificación: Completa conjunto mutante (agregar, limpiar, etc.).
Impacto en Tests: Caso índice válido, índice fuera de rango (error E0601), lista vacía.
```

---
Actualizaciones sugeridas: `// PROPUESTA GUIA GPT: <detalle>`
