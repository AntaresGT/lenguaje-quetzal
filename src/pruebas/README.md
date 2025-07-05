# Pruebas Unitarias del Intérprete Quetzal

Esta carpeta contiene todas las pruebas unitarias organizadas por funcionalidad.

## Estructura

```
src/pruebas/
├── mod.rs                    # Declaración del módulo de pruebas
├── funciones_encadenadas.rs  # Pruebas para funciones encadenadas
└── README.md                 # Este archivo
```

## Cómo ejecutar las pruebas

### Ejecutar todas las pruebas:
```bash
cargo test
```

### Ejecutar pruebas específicas de funciones encadenadas:
```bash
cargo test funciones_encadenadas
```

### Ejecutar una prueba específica:
```bash
cargo test test_funciones_encadenadas_basicas
```

## Agregar nuevas pruebas

Para agregar nuevos módulos de pruebas:

1. Crear un nuevo archivo `.rs` en esta carpeta
2. Agregarlo al archivo `mod.rs`
3. Usar la estructura estándar de pruebas de Rust con `#[cfg(test)]`

## Cobertura actual

- ✅ **Funciones encadenadas**: 11 pruebas
  - Conversiones básicas entre tipos
  - Encadenamiento múltiple de métodos
  - Manejo de saltos de línea
  - Validación de errores

## Futuras pruebas a implementar

- Pruebas de sintaxis básica
- Pruebas de operadores
- Pruebas de estructuras de control (bucles, condicionales)
- Pruebas de manejo de errores
- Pruebas de objetos y clases
- Pruebas de módulos e importaciones
