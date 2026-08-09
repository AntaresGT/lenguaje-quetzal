---
name: definir-spec
description: >
  Define una especificación a partir de una historia de usuario: rellena huecos,
  lista asunciones funcionales/no técnicas, y refina una a una con opciones y
  barra de progreso. Use when the user says "definir spec", "definir una spec",
  "historia de usuario", "rellenar especificación", or invokes /definir-spec.
disable-model-invocation: true
---

# Definir spec

Flujo guiado para convertir una historia de usuario en una especificación lista
para escribirse. No escribas la spec final hasta que el usuario lo pida después
del cierre de este flujo.

## Fase 1 — Historia y borrador

1. El usuario entrega una historia de usuario (y contexto opcional).
2. Rellena los espacios en blanco: completa lo implícito o ausente para que la
   historia sea usable como base de una spec.
3. Presenta un borrador breve de la historia completada (solo lo necesario para
   que el usuario vea cómo quedaron los huecos).

## Fase 2 — Listado de asunciones

Muestra **todas** las asunciones **no técnicas o funcionales** que usaste al
rellenar (reglas de negocio, alcance, UX, criterios de aceptación, datos,
permisos, flujos, etc.). No listes aquí detalles de implementación técnica
(lenguaje, librerías, estructura de código).

Formato obligatorio:

```markdown
## Asunciones

1. ...
2. ...
3. ...
```

Luego pide: que indique los **números** de las asunciones que no le gustan.
Si no rechaza ninguna, salta a la Fase 4.

## Fase 3 — Refinar asunciones rechazadas

Pregunta **una asunción a la vez**, en el orden de los números que indicó el
usuario. No adelantes la siguiente hasta que responda la actual.

### Barra de progreso

En cada pregunta muestra progreso así (ajusta `actual` y `total`):

```text
Progreso: [████░░░░] 2/5
```

- `actual` = número de pregunta en curso (1-based).
- `total` = cantidad de asunciones a refinar.
- Rellena la barra en proporción (`actual/total`), con bloques `█` y `░`.
- Indica también cuántas faltan: `Faltan: N`.

### Opciones por pregunta

Para la asunción en curso:

1. Recuerda en una línea qué asunción se está reemplazando.
2. Ofrece **exactamente 4** asunciones alternativas plausibles, numeradas 1–4.
3. Añade una **5.ª opción: `otra`** — si el usuario elige `otra` (o 5), debe
   escribir su definición; úsala tal cual.
4. Espera la respuesta antes de pasar a la siguiente.

Plantilla:

```markdown
Progreso: [██░░░░░░] 1/4
Faltan: 3

**Asunción original (#N):** ...

Elige la nueva definición:
1. ...
2. ...
3. ...
4. ...
5. otra (especifica tu respuesta)
```

Tras cada respuesta, actualiza internamente esa asunción y continúa.

## Fase 4 — Cierre

Cuando no queden asunciones por refinar, di exactamente que ya estás listo
para crear la especificación. No generes la spec en ese mismo mensaje salvo
que el usuario lo pida de inmediato.

Ejemplo de cierre:

> Ya me encuentro listo para crear la especificación.

## Reglas

- Español en toda la interacción.
- Una pregunta a la vez en Fase 3; sin agrupar.
- Siempre 4 alternativas + `otra`.
- Siempre barra de progreso con hechas/total y faltantes.
- Solo asunciones no técnicas o funcionales en el listado.
- No inventes pasos extra (diseño técnico, código, archivos) dentro de este flujo.
