# Instrucciones de Desarrollo - Intérprete Quetzal v0.0.2

## Objetivo General

Desarrollar un intérprete para el lenguaje de programación Quetzal v0.0.2, implementado en Rust, que sea capaz de interpretar y ejecutar código Quetzal de manera eficiente.

## Características del Intérprete

### Tecnologías Base
- **Lenguaje de implementación**: Rust
- **Tipo**: Intérprete en tiempo de ejecución
- **Extensión de archivos**: `.qz`
- **Archivo de entrada**: `principal.rs`

### Capacidades Requeridas
- Tipado fuerte con verificación en tiempo de ejecución y compilación
- Compatibilidad nativa con JSON
- Sintaxis completamente en español
- Gestión automática de memoria
- Sistema de manejo de errores con códigos específicos (E0001-E0999)

## Reglas de Desarrollo

### Código en Español
- **Funciones**: Usar snake_case (`nombre_funcion`, `calcular_promedio`)
- **Variables**: Usar snake_case (`mi_variable`, `contador_principal`)
- **Archivos**: Nombres en español (`principal.rs`, `utileria.rs`)
- **Directorios**: Nombres en español (`analizador/`, `tipos/`)
- **Comentarios**: Todo en español
- **Excepción**: Solo `fn main()` puede mantener sintaxis en inglés

### Estructura de Archivos Recomendada
```
src/
├── principal.rs              // Punto de entrada
├── analizador/
│   ├── lexico.rs             // Análisis léxico
│   ├── sintactico.rs         // Análisis sintáctico
│   └── semantico.rs          // Análisis semántico
├── tipos/
│   ├── valor.rs              // Tipos de datos
│   ├── entero.rs             // Tipo entero
│   ├── numero.rs             // Tipo número (flotante)
│   ├── texto.rs              // Tipo texto/cadena
│   ├── logico.rs             // Tipo lógico/booleano
│   ├── lista.rs              // Tipo lista/array
│   └── json.rs               // Tipo jsn/JSON
├── evaluador/
│   └── interprete.rs         // Motor de ejecución
├── errores/
│   └── codigos.rs            // Sistema de códigos de error
└── utileria/
    └── consola.rs            // Objeto consola global
```

## Librerías Recomendadas

### Análisis Léxico y Sintáctico
- `logos` - Análisis léxico eficiente
- `pest` - Parser generator para análisis sintáctico

### Manejo de Datos
- `serde` - Serialización/deserialización
- `serde_json` - Manejo de JSON nativo

### Interfaz de Usuario
- `clap` - Argumentos de línea de comandos
- `rustyline` - Entrada interactiva de consola
- `colored` - Salida con colores en consola

### Manejo de Errores
- `anyhow` - Manejo simplificado de errores
- `thiserror` - Errores personalizados

## Sintaxis del Lenguaje Quetzal

### Reglas Fundamentales
- Sin punto y coma (`;`) al final de instrucciones
- Sensible a mayúsculas y minúsculas
- Palabras reservadas siempre en minúsculas
- Objetos en PascalCase, variables/funciones en snake_case

### Tipos de Datos Básicos
```qz
entero variable_entera = 42
número variable_decimal = 3.14159
texto variable_texto = "Hola mundo"
log variable_booleana = verdadero
lista variable_lista = [1, 2, 3]
jsn variable_json = {clave: "valor"}
```

### Mutabilidad
```qz
// Constante (por defecto)
entero constante = 10

// Variable (mutable)
entero var variable = 10
variable = 20  // Permitido
```

### Funciones
```qz
// Sin retorno
vacio mi_funcion(entero parametro) {
    // código
}

// Con retorno
entero calcular_suma(entero a, entero b) {
    retornar a + b
}

// Asíncrona
asincrono texto obtener_datos() {
    // código asíncrono
    retornar "datos"
}
```

### Objetos
```qz
objeto MiObjeto {
    publico:
        entero var valor_publico
        
        MiObjeto(entero valor_inicial) {
            ambiente.valor_publico = valor_inicial
        }
        
        entero obtener_valor() {
            retornar ambiente.valor_publico
        }
    
    privado:
        texto valor_privado = "secreto"
}
```

### Control de Flujo
```qz
// Condicionales
si (condicion) {
    // código
} sino si (otra_condicion) {
    // código
} sino {
    // código
}

// Bucles
mientras (condicion) {
    // código
}

para (entero var i = 0; i < 10; i++) {
    // código
}

para (elemento cada lista) {
    // código
}
```

### Manejo de Excepciones
```qz
intentar {
    // código que puede fallar
} capturar (Excepción e) {
    // manejo del error
} finalmente {
    // código que siempre se ejecuta
}
```

## Sistema de Errores

### Rangos de Códigos
- **E0001-E0099**: Errores léxicos y sintácticos
- **E0100-E0199**: Errores de declaración
- **E0200-E0299**: Errores de tipos
- **E0300-E0399**: Errores de control de flujo
- **E0400-E0499**: Errores de módulos
- **E0500-E0599**: Errores de objetos
- **E0600-E0699**: Errores de listas/estructuras
- **E0700-E0799**: Errores de JSON
- **E0800-E0899**: Errores de funciones
- **E0900-E0999**: Errores de sistema

### Formato de Error
```
error[E####]: descripción del error
 --> archivo.qz:línea:columna
  |
línea | código que causó el error
      | ^ aquí está el error
  |
  = ayuda: sugerencia para resolver el error
```

## Funcionalidades Especiales

### Precisión Decimal
- Manejo automático de precisión para evitar errores como `0.1 + 0.2 ≠ 0.3`
- Redondeo automático a 15 decimales
- Preservación de tipos enteros en operaciones entre enteros

### Objeto Consola Global
```qz
consola.mostrar("Mensaje normal")
consola.mostrar_error("Mensaje de error")
consola.mostrar_advertencia("Mensaje de advertencia")
consola.mostrar_exito("Mensaje de éxito")

texto entrada = consola.pedir("Ingresa tu nombre: ")
texto secreto = consola.pedir_secreto("Contraseña: ")
```

### Sistema de Módulos
```qz
// Exportar desde módulo
exportar {
    mi_funcion,
    mi_variable,
    MiObjeto
}

// Importar en archivo principal
importar {
    mi_funcion,
    mi_variable como variable_importada,
    MiObjeto
} desde "ruta/al/modulo.qz"
```

## Comandos del Intérprete

### Uso Básico
```bash
quetzal archivo.qz
```

### Opciones
```bash
quetzal --version    # Mostrar versión
quetzal --ayuda      # Mostrar ayuda
```

## Notas de Implementación

### Conversiones de Tipo
- Todos los tipos deben implementar `.texto()` para conversión a string
- Conversiones específicas: `.entero()`, `.numero()`, `.log()`, `.lista()`, `.jsn()`

### Métodos de Listas
- Métodos de consulta: `.longitud()`, `.esta_vacia()`, `.primero()`, `.ultimo()`
- Métodos mutantes: `.agregar()`, `.quitar()`, `.limpiar()` (requieren `var`)
- Soporte completo para matrices multidimensionales

### Compatibilidad JSON
- Parsing nativo de JSON a tipo `jsn`
- Conversión bidireccional entre objetos Quetzal y JSON
- Acceso a propiedades con `.` y `["clave"]`

## Objetivos de Rendimiento

- Ejecución eficiente gracias a implementación en Rust
- Gestión óptima de memoria
- Manejo eficiente de estructuras de datos grandes
- Optimizaciones específicas para operaciones comunes

## Documentación

Todo el código debe estar documentado en español, incluyendo:
- Comentarios explicativos para funciones complejas
- Documentación de estructuras de datos
- Ejemplos de uso en comentarios
- Explicaciones de algoritmos no triviales
