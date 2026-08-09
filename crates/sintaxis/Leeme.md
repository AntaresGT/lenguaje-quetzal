# sintaxis

Parser recursivo descendente del Lenguaje Quetzal. Consume los tokens de `lexico` y produce el AST definido en `ast`: declaraciones de variables y funciones, expresiones con precedencia, condicionales, bucles, excepciones, objetos, prototipos, herencia e importar/exportar.

Los errores sintácticos se reportan como `ErrorQuetzal` con ubicación exacta.
