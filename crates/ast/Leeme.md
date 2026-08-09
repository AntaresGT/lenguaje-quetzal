# ast

Árbol de sintaxis abstracta (AST) del Lenguaje Quetzal: expresiones, sentencias, declaraciones (variables, funciones, objetos, prototipos), módulos (importar/exportar) y tipos. Cada nodo guarda su `Ubicacion` para diagnósticos precisos.

Este crate solo define datos; el parser vive en `sintaxis` y el consumo del AST en `semantica` y `bytecode`.
