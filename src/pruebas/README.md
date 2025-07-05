# Pruebas Unitarias del Intérprete Quetzal

Este directorio contiene todas las pruebas unitarias para el intérprete del lenguaje Quetzal. Las pruebas están organizadas por funcionalidad para facilitar el mantenimiento y la comprensión.

## Estructura de Pruebas

### 1. `variables_y_tipos.rs`
- Declaración de variables básicas
- Tipos de datos (entero, número, cadena, bool, vacio)
- Variables mutables e inmutables
- Validación de nombres de variables
- Palabras reservadas

### 2. `operadores.rs`
- Operadores aritméticos (+, -, *, /, %)
- Operadores de comparación (>, <, >=, <=, ==, !=)
- Operadores lógicos (&&, ||, y, o, !)
- Asignación compuesta (+=, -=, *=, /=, %=)
- Operaciones con tipos mixtos

### 3. `condicionales.rs`
- Estructuras si/sino
- Condicionales anidados
- Expresiones booleanas complejas
- Comparaciones de diferentes tipos

### 4. `bucles.rs`
- Bucles "para" (for)
- Bucles "mientras" (while)
- Bucles "hacer-mientras" (do-while)
- Bucles "foreach"
- Control de flujo (break, continue)

### 5. `funciones.rs`
- Declaración de funciones
- Funciones con y sin parámetros
- Funciones con y sin retorno
- Llamadas a funciones
- Funciones recursivas
- Alcance de variables
- Parámetros múltiples

### 6. `funciones_encadenadas.rs`
- Métodos encadenados (.cadena(), .numero())
- Funciones encadenadas en varias líneas
- Manejo de comentarios en expresiones encadenadas
- Conversiones de tipos encadenadas
- Casos de error en encadenamiento

### 7. `listas_y_json.rs`
- Creación y manipulación de listas
- Acceso a elementos por índice
- Operaciones con objetos JSON
- Anidamiento de estructuras
- Métodos de listas

### 8. `cadenas.rs`
- Concatenación de cadenas
- Operaciones con cadenas vacías
- Escape de caracteres
- Cadenas con caracteres especiales
- Interpolación de variables

### 9. `entrada_salida.rs`
- Función imprimir()
- Funciones imprimir_exito(), imprimir_error(), etc.
- Impresión de diferentes tipos de datos
- Conversión automática para impresión
- Manejo de salida formateada

### 10. `conversiones.rs`
- Conversión de entero a cadena
- Conversión de decimal a cadena
- Conversión de cadena a número
- Conversión de booleanos
- Conversiones encadenadas
- Manejo de errores en conversiones

### 11. `manejo_errores.rs`
- División por cero
- Variables no declaradas
- Tipos incompatibles
- Sintaxis inválida
- Overflow de números
- Nombres de variables inválidos
- Errores de asignación

### 12. `sintaxis_y_comentarios.rs`
- Comentarios de línea (//)
- Comentarios de bloque (/* */)
- Manejo de espacios y saltos de línea
- Estructura del código
- Expresiones multilínea
- Validación de sintaxis

## Ejecución de Pruebas

Para ejecutar todas las pruebas:
```bash
cargo test
```

Para ejecutar pruebas de un módulo específico:
```bash
cargo test variables_y_tipos
cargo test funciones_encadenadas
cargo test manejo_errores
# etc.
```

Para ejecutar una prueba específica:
```bash
cargo test test_declaracion_variables_basicas
```

## Cobertura de Pruebas

Las pruebas cubren:
- ✅ Declaración y uso de variables
- ✅ Todos los tipos de datos básicos
- ✅ Operadores aritméticos, lógicos y de comparación
- ✅ Estructuras de control (if/else, bucles)
- ✅ Funciones simples y con parámetros
- ✅ Funciones encadenadas (.cadena(), .numero())
- ✅ Listas y objetos JSON
- ✅ Manipulación de cadenas
- ✅ Entrada/salida básica
- ✅ Conversiones de tipos
- ✅ Manejo de errores y validaciones
- ✅ Sintaxis y comentarios

## Convenciones

- Cada prueba debe tener un nombre descriptivo que indique qué funcionalidad está probando
- Las pruebas que deben fallar deben usar `assert!(result.is_err())`
- Las pruebas que deben pasar deben usar `assert!(result.is_ok())`
- Incluir casos límite y situaciones de error
- Mantener el código de prueba simple y fácil de entender

## Agregar Nuevas Pruebas

1. Identificar el módulo apropiado o crear uno nuevo si es necesario
2. Agregar la prueba siguiendo las convenciones establecidas
3. Actualizar este README si se agrega un nuevo módulo
4. Ejecutar las pruebas para verificar que funcionen correctamente

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
