# lexico

Analizador léxico del Lenguaje Quetzal. Convierte el código fuente `.qz` en una secuencia de tokens con su ubicación: palabras reservadas en español (`entero`, `si`, `mientras`, `objeto`, ...), identificadores Unicode (`año`, `función`), números, textos (incluidos textos interpolados `t"Hola {nombre}"`), operadores, delimitadores y comentarios.

Acepta variantes con y sin tilde de las palabras reservadas (`número`/`numero`, `asincróno`/`asincrono`) mediante normalización.
