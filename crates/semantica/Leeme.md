# semantica

Análisis semántico del Lenguaje Quetzal sobre el AST: tabla de símbolos con ámbitos, verificación de tipos, mutabilidad (`var`), reasignación de constantes, retornos de funciones, visibilidad (`publico`/`privado`), cumplimiento de prototipos (`implementa`) y resolución de imports.

Acumula todos los errores y los reporta antes de ejecutar.

## Asincronía

El analizador también valida el contexto de `esperar` (regla E0213):

- `esperar` es válido dentro de una función `asincrono` (incluyendo métodos `asincrono` y métodos `libre asincrono` de objetos).
- `esperar` es válido a nivel de scope global (top-level), punto de entrada del programa.
- `esperar` es un error dentro de una función síncrona (incluidos constructores y métodos no `asincrono` de objetos), porque esas funciones no participan del bucle de eventos.
