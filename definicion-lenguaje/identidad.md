# Identidad de Lenguaje Quetzal

> **Documento canónico.** Si se pierde contexto del proyecto, este archivo
> + `catalogo.json` + `gramatica.json` restauran qué es Quetzal.

```yaml
id: lenguaje-quetzal
nombre: Lenguaje Quetzal
corto: Quetzal
extensión: .qz
manifiesto: quetzal.json
implementación: intérprete Rust
versión_documentada: 0.0.2
idioma_superficie: español
```

## Objetivo

Lenguaje de programación **interpretado, de propósito general**, con
sintaxis y biblioteca en **español**. No es un juguete pedagógico: sirve
para apps, backends HTTP, FS y scripts reales, con tipado estricto y
permisos declarativos.

## Qué es (definición operativa)

| Propiedad | Valor |
|---|---|
| Paradigma | Imperativo + POO clásica + async |
| Tipado | Estático, **explícito**, sin inferencia |
| Mutabilidad | **Inmutable por defecto**; `var` tras el tipo |
| Nulos | `nulo` polimórfico (cualquier tipo) |
| Módulos | `importar {…} desde "…"`, `exportar {…}` |
| Seguridad | Permisos en `quetzal.json`: `red`, `sistema-archivos`, `ejecución` |
| Léxico | Keywords ES; lexer **normaliza tildes** (`número` ≡ `numero`) |

## Qué no es

- No es solo “lenguaje para principiantes”.
- No tiene lambdas / funciones anónimas (callbacks = nombres).
- No tiene inferencia de tipos.
- No es tipado nullable estilo TypeScript (`Option`); `nulo` entra en todo tipo.

## Pipeline (fuente → runtime)

```
.qz + quetzal.json
        ↓
   [lexer]  → tokens
        ↓
   [parser] → AST
        ↓
 [semántica] → tipos + imports + permisos
        ↓
  [cargador] → programa enlazado
        ↓
 [evaluador] ↔ stdlib (consola, matemática, tiempo, motor, FS, red)
```

Grafo sintaxis: [`diagramas.md`](./diagramas.md#0-grafo-maestro-pipeline).  
Arquitectura crates/intérprete: [`arquitectura.md`](./arquitectura.md).  
Inventario machine-readable: [`catalogo.json`](./catalogo.json).

## Contrato de superficie (mínimo)

```quetzal
// tipado + inmutable
entero edad = 30
texto var nombre = "Ana"

// control
si (edad >= 18) {
    consola.mostrar(t"Hola {nombre}")
}

// función
número sumar(número a, número b) {
    retornar a + b
}

// módulo
importar { Matemática } desde "quetzal/matemática"
```

## Fuentes de verdad (orden)

1. `identidad.md` — qué es
2. `catalogo.json` — inventario features/API
3. `gramatica.json` / `gramatica.ebnf.md` — sintaxis formal
4. `esquema_quetzal.json` — manifiesto
5. Docs temáticos `.md` — detalle humano
6. `anomalias.md` — desviaciones conocidas
7. `ejemplos/**/*.qz` — comportamiento observado

## Audiencias

| Quién | Cómo consumir |
|---|---|
| Humano | Este archivo → README → tipos → feature docs |
| IA | Prompt con `catalogo.json` + `gramatica.json` + este archivo |
| Máquina | Validar JSON Schema; generar parser desde `gramatica.json` |
