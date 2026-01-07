# Instrucciones para Agentes de Cursor

Este documento proporciona instrucciones específicas para agentes de IA que trabajan en Cursor en este proyecto.

## Contexto del Proyecto

Lenguaje Quetzal es un lenguaje de programación interpretado en tiempo de ejecución, programado en Rust. Los archivos de Quetzal se caracterizan por la terminación `.qz`.

## Arquitectura Principal

### Núcleo (`src/nucleo/`)
- **lexico/**: Analizador léxico con logos
- **sintactico/**: Analizador sintáctico con pest
- **semantico/**: Verificación semántica y tipos

### Intérprete (`src/interprete/`)
- **valores.rs**: Representación de valores en tiempo de ejecución
- **entorno.rs**: Entorno de ejecución con stack de scopes
- **expresiones.rs**: Evaluación de expresiones
- **declaraciones.rs**: Evaluación de declaraciones
- **objetos.rs**: Sistema de objetos
- **asincrono.rs**: Funciones asíncronas

### Módulos Nativos (`src/nativos/`)
Sistema extensible para agregar módulos nativos fácilmente:
- Implementar `ModuloNativo` trait
- Registrar en `registro.rs`

## Convenciones de Código

1. **Español obligatorio**: Todo el código debe estar en español
2. **snake_case**: Variables y funciones
3. **PascalCase**: Objetos/clases
4. **4 espacios**: Indentación

## Agregar Funcionalidad

### Nuevo Módulo Nativo
1. Crear `src/nativos/mi_modulo.rs`
2. Implementar `ModuloNativo`
3. Agregar a `src/nativos/registro.rs`

### Nuevo Tipo de Error
1. Agregar código en `src/errores/codigos.rs`
2. Usar en el código correspondiente

## Pruebas

- Pruebas humanas: `pruebas/humanos/`
- Pruebas IA: `pruebas/ia/`

## Comandos Útiles

```bash
cargo check          # Verificar compilación
cargo build          # Compilar
cargo test           # Ejecutar pruebas
cargo run -- --ayuda # Ver ayuda de CLI
```

## Referencias

- Plan completo: `.cursor/plans/intérprete_lenguaje_quetzal_v0.2.0_d805e07e.plan.md`
- Especificación: Ver documentación del usuario
