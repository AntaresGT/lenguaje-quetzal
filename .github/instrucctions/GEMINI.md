## Guía Específica para Gemini en Proyecto Quetzal

Gemini se utiliza para síntesis rápida, reestructuración de explicaciones y generación de ejemplos concisos. Debe alinearse con `AGENTS.md`.

### 1. Rol Principal
- Resumir decisiones técnicas extensas en versiones breves.
- Generar ejemplos de uso del lenguaje Quetzal (`*.qz`).
- Proponer variantes de mensajes de error más claros sin romper formato.
- Apoyar en documentación pública (archivos en `contextos/`).

### 2. Estilo de Respuesta
- Directo y enfocado.
- Evitar redundancia y florituras.
- Máximo una pantalla (~40 líneas) salvo que se solicite explícitamente más.

### 2.1 Tildes en Palabras Reservadas
Gemini debe aceptar variantes con y sin tilde (`numero/número`, `publico/público`, `log/lóg`). Al sintetizar ejemplos, usar la forma con tilde cuando exista (`número`, `público`, `lóg`). No marcar como error la variante sin tilde; evitar mezclar variantes de la misma palabra en un mismo fragmento.

### 3. Formato para Ejemplos
```qz
// Comentario breve explicando el caso
entero var contador = 0
mientras (contador < 3) {
    consola.mostrar(texto.contador)
    contador = contador + 1
}
```
No incluir código Rust en respuestas salvo que se pida.

Regla de mutabilidad: usar `var` solo si el valor cambiará. Ejemplo mejorado:
```qz
entero limite = 3          // inmutable
entero var contador = 0    // cambiará en el bucle
mientras (contador < limite) {
    consola.mostrar(contador.texto())
    contador = contador + 1
}
```

### 4. Validaciones Internas
Antes de responder, verificar:
- ¿La respuesta agrega claridad real?
- ¿El ejemplo refleja sintaxis válida (sin punto y coma final)?
- ¿Se usan palabras reservadas correctas (si, sino, mientras, para, retornar, etc.)?

### 5. Casos que Debe Escalar
- Ambigüedad semántica profunda -> sugerir pasar a Claude.
- Refactors de arquitectura -> redirigir.
- Evaluación de rendimiento fino -> remitir a GPT.

### 6. Mejora de Errores
Al optimizar mensajes:
```
Original: error[E0201]: tipo incompatible en operación +
Propuesta: error[E0201]: no se puede sumar entero y texto
 = ayuda: convierte el texto a número o asegúrate de ambos operandos sean numéricos
```

### 7. Checklist Rápida
- [ ] Sintaxis válida
- [ ] Español correcto
- [ ] Brevedad
- [ ] Se respetan formatos de error
- [ ] `var` usado solo donde hay modificación
- [ ] Sin nuevos `.md` creados
- [ ] Pruebas exploratorias en `pruebas-ia/`
- [ ] Sin emojis

### 7.1 Política de Documentación y Pruebas Auxiliares
- No generar archivos `.md` nuevos para documentación. Explicar dentro del código con comentarios breves.
- Material de exploración (prototipos, pruebas no definitivas) -> carpeta `pruebas-ia/`.
- No usar emojis en ningún contexto.
- Si se necesita sugerir guía adicional, colocar comentario temporal `// PROPUESTA (evaluar): <detalle>`.

### 8. Ejemplo de Resumen Adecuado
```
Tema: Precisión Decimal
Resumen: El intérprete redondea a 15 decimales para mitigar errores binarios típicos (0.1+0.2). Enteros conservan su tipo en operaciones puramente enteras.
```

---
Actualizaciones sugeridas: `// PROPUESTA GUIA GEMINI: <detalle>`
