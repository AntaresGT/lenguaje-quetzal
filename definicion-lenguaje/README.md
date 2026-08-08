# Definición del Lenguaje Quetzal

Especificación formal y visual de la sintaxis del lenguaje **Quetzal**,
derivada del análisis exhaustivo de los 21 archivos `.qz` de ejemplo
y de los manifiestos `quetzal.json` del repositorio.

## Índice de archivos

| Archivo | Contenido |
|---|---|
| **`gramatica.ebnf.md`** | Gramática formal en notación EBNF (Extended Backus–Naur Form). |
| **`gramatica.json`** | Gramática en JSON estructurado, optimizada para que un modelo de IA la parsee y genere parsers/highlighters. |
| **`diagramas.md`** | 25 diagramas de sintaxis en Mermaid (estilo railroad) cubriendo cada producción de la gramática. |
| `tokens.md` | Referencia léxica: palabras reservadas, operadores, delimitadores, literales y comentarios. |
| `tipos.md` | Tipos de datos primitivos y compuestos, mutabilidad, conversiones y el valor `nulo`. |
| `control-flujo.md` | Condicionales, bucles, control de flujo directo y manejo de excepciones. |
| `funciones.md` | Declaración, parámetros, retorno, recursividad, funciones asíncronas. |
| `poo.md` | `objeto`, `prototipo`, herencia, modificadores de visibilidad, miembros `libre`. |
| `modulos.md` | `importar`, `exportar`, alias `como`, rutas relativas y nativas. |
| `metodos-nativos.md` | API de cadenas, listas, JSON, booleanos y números. |
| `modulos-nativos.md` | Módulos `consola`, `quetzal/matemática`, `quetzal/tiempo`, `quetzal/motor`, `quetzal/sistema_archivos`. |
| `manifiesto.md` | Formato del archivo `quetzal.json` (manifiesto de proyecto). |
| `esquema_quetzal.json` | JSON Schema canónico (Draft-07) del manifiesto, con campos con tildes. |
| `anomalias.md` | Inconsistencias, ambigüedades y observaciones sobre la sintaxis. |

## Resumen ejecutivo

Quetzal es un lenguaje de programación:

- **En español** — keywords y API en español natural (`entero`, `texto`, `lista`, `objeto`).
- **Tipado estático y explícito** — sin inferencia; todo tipo se declara.
- **Inmutable por defecto** — `var` introduce mutabilidad.
- **Orientado a objetos clásico** — clases (`objeto`) con herencia simple/múltiple e interfaces (`prototipo`).
- **Con asincronía** — `asincrono` / `esperar`.
- **Con manejo de excepciones** — `intentar / capturar / finalmente` con variable `excepcion`.
- **Con módulos** — `importar` / `exportar` y alias `como`.
- **Con interpolación de cadenas** — prefijo `t"..."` similar a f-strings.
- **Con biblioteca estándar en español** — `consola`, `Matemática`, `Tiempo`, `ExpresiónRegular`, `SistemaArchivos` (incluye `Flujo` con cursor y `Observador` de cambios).
- **Seguro por defecto** — el acceso al filesystem, la red y la ejecución de procesos requiere permisos declarados en `quetzal.json`.

## Cómo usar esta documentación

1. **Para entender el lenguaje**: empezar por `tokens.md` → `tipos.md` → `diagramas.md` → secciones por característica.
2. **Para implementar un parser**: usar `gramatica.ebnf.md` o `gramatica.json` directamente.
3. **Para que una IA entienda la sintaxis**: darle `gramatica.json` en el prompt (es JSON nativo, no requiere pre-procesamiento).
4. **Para visualizar la sintaxis**: abrir `diagramas.md` en cualquier visor con soporte Mermaid (GitHub, GitLab, VS Code, Obsidian, etc.).

## Fuentes

Esta especificación fue generada a partir de:

- `ejemplos/*.qz` (21 archivos de código fuente Quetzal)
- `ejemplos/hola_mundo/quetzal.json`
- `ejemplos/modulos/quetzal.json`
- El JSON Schema canónico del manifiesto (guardado como
  `esquema_quetzal.json` en esta carpeta, con campos con tildes).
