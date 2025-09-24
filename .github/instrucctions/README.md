## Instrucciones Centralizadas para Agentes de IA — Proyecto Quetzal

Esta carpeta contiene guías específicas para la colaboración estructurada entre múltiples modelos de IA en el desarrollo del intérprete del lenguaje Quetzal.

### Archivos
- `AGENTS.md`: Regla base y marco común multi‑agente (LEER PRIMERO).
- `CLAUDE.md`: Guía enfocada en análisis profundo, refactors y semántica.
- `GEMINI.md`: Guía para síntesis breve, generación de ejemplos y documentación pública.
- `GPT.md`: Guía para generación de código, expansión de pruebas y optimizaciones justificadas.

### Cómo Usar Estas Instrucciones
1. Identifica el tipo de tarea (refactor, ejemplo, documentación, implementación nueva).
2. Consulta `AGENTS.md` para principios globales.
3. Aplica la guía específica del modelo que estás usando.
4. Si la tarea involucra múltiples aspectos (ej. diseño + tests): combinar enfoques respetando coherencia.
5. Documenta decisiones mayores con comentario: `// RDT: <fecha ISO> - <decisión>`.

### Cuando Hay Conflicto Entre Guías
Orden de precedencia:
1. Especificación del lenguaje (`contextos/*.md`)
2. `copilot-instructions.md`
3. `AGENTS.md`
4. Guía específica del modelo

### Proponer Cambios a las Guías
Incluir en el PR o diff un bloque:
```
PROPUESTA GUIA:
Archivo: <nombre>
Sección: <título>
Cambio: <descripción>
Motivación: <razón>
Impacto: <riesgos / beneficios>
```

### Principios Clave (Resumen)
- Código y comentarios en español.
- Cambios pequeños y atómicos.
- Sistema de errores consistente (E0001-E0999).
- Tests para nuevas rutas lógicas y errores.
- Evitar duplicación de lógica.
- Palabras reservadas con tilde aceptan también variante sin tilde (ver `SINTAXIS_REFERENCIA.md` sección Política de Tildes); preferir forma con tilde en ejemplos nuevos.
- Inmutabilidad por defecto: toda variable sin `var` es constante; usar `var` solo si habrá reasignación.
- Documentación nueva en código (no crear más `.md`); migrar notas existentes gradualmente.
- Experimentos y pruebas exploratorias van a `pruebas-ia/`.
- No usar emojis en código, comentarios, mensajes ni nombres.

### Checklist Rápido Antes de Confirmar Cambios
- [ ] Compila sin warnings innecesarios
- [ ] Nombres en español (snake_case / PascalCase según corresponda)
- [ ] Errores con código correcto
- [ ] Tests agregados/actualizados
- [ ] Documentación coherente
- [ ] Sin dependencias nuevas injustificadas
- [ ] Uso de `var` justificado
- [ ] Sin nuevos `.md` añadidos
- [ ] Pruebas auxiliares (si aplica) en `pruebas-ia/`
- [ ] Sin emojis

---
Esta carpeta es parte viva del proceso. Las guías deben reflejar el estado actual del lenguaje y evolucionar con él.
