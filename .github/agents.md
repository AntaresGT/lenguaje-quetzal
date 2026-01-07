# Instrucciones para Agentes de GitHub

Este documento proporciona instrucciones para agentes de IA que trabajan en este proyecto.

## Estructura del Proyecto

El proyecto Lenguaje Quetzal está organizado en las siguientes secciones principales:

- `src/nucleo/` - Análisis del código fuente (léxico, sintáctico, semántico)
- `src/interprete/` - Ejecución del código (evaluador)
- `src/nativos/` - Módulos nativos extensibles
- `src/errores/` - Sistema de errores completo

## Reglas de Desarrollo

1. **Código en español**: Todo el código debe estar en español, incluyendo funciones, variables y archivos.
2. **Comentarios en español**: Todos los comentarios deben estar en español.
3. **Nombres descriptivos**: Use nombres descriptivos para variables, funciones y objetos.
4. **Manejo de errores**: Use el sistema de errores definido en `src/errores/`.
5. **Pruebas**: Cree pruebas unitarias para cada nueva funcionalidad.

## Agregar Nuevos Módulos Nativos

Para agregar un nuevo módulo nativo:

1. Cree un archivo en `src/nativos/` implementando el trait `ModuloNativo`.
2. Agregue una línea en `src/nativos/registro.rs` para registrarlo.

Ejemplo:
```rust
// src/nativos/mi_modulo.rs
impl ModuloNativo for MiModulo { ... }

// src/nativos/registro.rs
let modulos: Vec<Box<dyn ModuloNativo>> = vec![
    ...
    Box::new(MiModulo::nuevo()),
];
```

## Sistema de Errores

Todos los errores deben usar los códigos definidos en `src/errores/codigos.rs` (E0001-E0999).

## Más Información

Consulte el plan de implementación en `.cursor/plans/` para más detalles sobre la arquitectura.
