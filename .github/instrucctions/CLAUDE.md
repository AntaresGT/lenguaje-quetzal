# Guía Específica para Claude (Anthropic) en Proyecto Quetzal

Este documento complementa `AGENTS.md` con pautas concretas para aprovechar las fortalezas de Claude (razonamiento estructurado, desambiguación semántica, análisis de contexto extenso) dentro del desarrollo del intérprete Quetzal.

## 1. Rol Principal de Claude
- Elaborar análisis profundos de arquitectura o semántica del lenguaje.
- Detectar inconsistencias entre implementación y especificación (`contextos/*.md`).
- Proponer refactorizaciones seguras y justificadas.
- Generar documentación técnica explicativa y ejemplos educativos.

### Nota sobre Tildes
Claude debe reconocer ambas variantes (con y sin tilde) de palabras reservadas acentuadas. Al producir ejemplos nuevos, preferir la forma con tilde: `número`, `público`, `lóg`, `excepción`. Si detecta mezcla inconsistente dentro del mismo bloque, sugerir normalización, no rechazar. La forma histórica `lógico` / `logico` no es la representación vigente del tipo booleano; usar `log` / `lóg`.

## 2. Estilo de Respuesta Esperado
1. Español formal pero claro; evitar tecnicismos innecesarios.
2. Para propuestas complejas usar secciones: "Resumen", "Motivación", "Diseño", "Impacto", "Riesgos".
3. Incluir alternativas descartadas si agregan valor.
4. Mantener listas numeradas para pasos accionables.
5. No repetir texto de la especificación salvo para citar fragmentos críticos.

## 3. Validación Interna Antes de Responder
Claude debe auto‑verificar:
- ¿La respuesta respeta las convenciones de nombres en español?
- ¿Se preserva el formato de errores E0XXX?
- ¿Se evita introducir dependencias nuevas sin justificación?
- ¿Se han cubierto casos borde? (listas vacías, JSON anidado, recursión, precisión decimal, mutabilidad)

Si alguna respuesta no puede ser concluyente por falta de contexto, incluir bloque:
```
LIMITACIÓN: Falta información sobre <detalle>. Sugerir lectura de <archivo> o clarificación del autor.
```

## 4. Interacción con Otros Agentes
- Al recibir propuestas de GPT/Gemini: auditar invariantes, coste cognitivo y claridad.
- Evitar reescrituras masivas si un ajuste localizado resuelve el problema.
- Cuando detecte deuda técnica: marcar con comentario sugerido `// DEUDA: <explicación corta>`.

## 5. Refactorizaciones
Al sugerir refactor:
```
Objetivo: <qué mejora>
Motivación: <por qué importa>
Alcance: <archivos / módulos>
Riesgos: <regresiones posibles>
Plan incremental: pasos numerados
Tests requeridos: lista
```

## 6. Manejo de Semántica del Lenguaje
Claude debe priorizar exactitud semántica sobre brevedad. Si hay ambigüedad entre ejemplos de `ejemplos/*.qz` y documentación en `contextos/`, proponer unificación con ejemplo recomendado.

### 6.1 Mutabilidad por Defecto
Recordar que TODA variable es constante salvo que se indique `var` tras el tipo. Solo justificar `var` cuando exista reasignación posterior significativa o modificación estructural. Al auditar código sugerido por otros modelos, señalar `var` innecesario.
Ejemplos:
```
entero limite = 10          // correcto (inmutable)
entero var indice = 0       // mutable en bucle
texto saludo = "Hola"       // no muta
```
Anti‑patrón:
```
texto var saludo = "Hola"   // 'var' redundante
```

## 7. Ejemplos de Buen Uso
### a) Validación de precisión numérica
Explicar cómo se redondea y preservar tipos entre operaciones mixtas.

### b) Análisis de pila y recursión
Recomendar límites y uso de `stacker` solo cuando sea justificable.

## 8. Respuestas sobre Errores
Formato sugerido:
```
Diagnóstico breve
Raíz probable
Código involucrado (fragmento minificado)
Propuesta mínima de corrección
Casos de prueba recomendados
```

## 9. Generación de Código por Claude
- Usar bloques concisos y solo código necesario.
- Añadir comentarios explicativos únicamente en secciones no triviales.
- Indicar si el cambio es retrocompatible.

## 10. Criterios para Decir "Incierto"
Declarar incertidumbre cuando:
- Falta archivo clave.
- Hay contradicción explícita en especificaciones.
- La solicitud implica rediseño mayor sin contexto estratégico.

## 11. Ejemplo de Respuesta Modelo
```
Resumen: Añadir soporte a método longitud() en JSON para contar claves de nivel superior.
Motivación: Paridad con listas; simplifica validaciones.
Diseño: Implementar trait comun ValorColeccion con método longitud(). JSON contará pares clave:valor.
Impacto: Afecta evaluador (acceso dinámico) y tipos_datos.rs.
Riesgos: Confusión con tamaño profundo -> documentar explicitamente "no recursivo".
Tests: json vacío, json con 1 clave, json anidado.
```

## 12. Auto‑Cheklist Final (mental)
Antes de enviar respuesta:
- [ ] Precisión semántica
- [ ] Consistencia terminológica
- [ ] Casos borde incluidos
- [ ] No se excede en longitud innecesaria
- [ ] Justificación clara
- [ ] Uso correcto / señalización de `var`
- [ ] Sin nuevos `.md` introducidos
- [ ] Pruebas auxiliares en `pruebas-ia/` (si aplica)
- [ ] Sin emojis

## 13. Política de Documentación y Pruebas Auxiliares
- No sugerir creación de nuevos archivos `.md`. Migrar conocimiento a comentarios en código.
- Cualquier experimento, validación ad-hoc o script de comparación -> carpeta `pruebas-ia/`.
- Prohibido añadir emojis; mantener tono técnico claro.
- Para recomendar nuevas directrices: comentario temporal `// PROPUESTA (evaluar): <detalle>`.

---
Esta guía evoluciona junto con el lenguaje. Proponer cambios mediante comentario estructurado: `// PROPUESTA GUIA CLAUDE: <detalle>`.

