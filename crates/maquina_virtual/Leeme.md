# maquina_virtual

Máquina virtual de pila del Lenguaje Quetzal. Ejecuta el bytecode generado por `bytecode` con: pila de valores, marcos de ejecución, tabla de constantes, tabla de módulos cargados, manejo de excepciones (`intentar`/`capturar`) y errores de runtime con ubicación.

Los enteros usan operaciones verificadas (`checked_*`, sin overflow silencioso) y el tipo `número` usa decimales de precisión exacta (`0.1 + 0.2 == 0.3`).
