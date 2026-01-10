# Sistema de Importación/Exportación de Módulos

Este documento describe el sistema de importación y exportación de módulos implementado en Lenguaje Quetzal con soporte completo para aliases.

## Características

### Exportación de Símbolos

Los módulos pueden exportar variables, funciones y objetos usando la palabra clave `exportar`:

```qz
// modulo.qz
cadena saludo = "¡Hola!"
entero edad = 25

entero sumar(entero a, entero b) {
    retornar a + b
}

// Exportar símbolos
exportar saludo
exportar edad, sumar
```

### Importación Básica

Importar símbolos de otros módulos:

```qz
importar saludo desde modulo
imprimir(saludo)  // ¡Hola!
```

### Importación con Alias (Sintaxis "como")

Usar un nombre diferente para el símbolo importado:

```qz
importar saludo como mensaje desde modulo
imprimir(mensaje)  // ¡Hola!
```

### Importaciones Múltiples

Importar varios símbolos en una sola línea:

```qz
importar saludo, edad, sumar desde modulo
```

### Importaciones Múltiples con Aliases Mixtos

Combinar importaciones con y sin aliases:

```qz
importar saludo, edad como años, sumar como suma desde modulo
```

## Validaciones y Seguridad

### Palabras Reservadas
No se pueden usar palabras reservadas como aliases:

```qz
importar saludo como si desde modulo  // ❌ Error
```

### Símbolos No Exportados
Solo se pueden importar símbolos que hayan sido exportados:

```qz
importar simbolo_privado desde modulo  // ❌ Error si no está exportado
```

### Importaciones Circulares
El sistema detecta y previene importaciones circulares:

```qz
// modulo_a.qz
importar x desde modulo_b

// modulo_b.qz
importar y desde modulo_a  // ❌ Error: Importación circular detectada
```

### Protección de Ruta
Solo se permiten rutas relativas sin navegación hacia directorios superiores:

```qz
importar saludo desde ../modulo  // ❌ Error: No se permite '..'
importar saludo desde /modulo    // ❌ Error: No se permiten rutas absolutas
```

## Resolución de Archivos

- Los módulos deben estar en el mismo directorio o subdirectorios
- La extensión `.qz` es opcional al importar
- Ejemplos válidos:
  - `importar x desde modulo` → busca `modulo.qz`
  - `importar x desde modulo.qz` → busca `modulo.qz`
  - `importar x desde subdir/modulo` → busca `subdir/modulo.qz`

## Ejemplos Completos

### Ejemplo 1: Módulo de Matemáticas

```qz
// matematicas.qz
entero PI = 3

entero multiplicar(entero a, entero b) {
    retornar a * b
}

entero dividir(entero a, entero b) {
    retornar a / b
}

exportar PI, multiplicar, dividir
```

```qz
// programa.qz
importar PI como CONSTANTE_PI, multiplicar como mult desde matematicas

entero resultado = mult(5, 3)
imprimir("5 * 3 = " + resultado.cadena())
imprimir("PI = " + CONSTANTE_PI.cadena())
```

### Ejemplo 2: Módulo de Utilidades

```qz
// utilidades.qz
cadena saludar(cadena nombre) {
    retornar "¡Hola, " + nombre + "!"
}

cadena despedirse(cadena nombre) {
    retornar "¡Adiós, " + nombre + "!"
}

exportar saludar, despedirse
```

```qz
// programa.qz
importar saludar como saludo, despedirse como adios desde utilidades

cadena msg1 = saludo("María")
cadena msg2 = adios("Juan")

imprimir(msg1)  // ¡Hola, María!
imprimir(msg2)  // ¡Adiós, Juan!
```

## Notas Técnicas

- Los módulos se cargan una sola vez por ejecución
- Cada módulo tiene su propio entorno (scope) independiente
- Los símbolos importados se añaden al entorno actual con su nombre o alias
- La detección de importaciones circulares usa thread-local storage
- Las rutas se resuelven de forma segura para prevenir ataques de path traversal

## Mejores Prácticas

1. **Exportar explícitamente**: Solo exporta lo que necesitas exponer
2. **Usar aliases descriptivos**: Los aliases deben mejorar la legibilidad
3. **Agrupar importaciones**: Agrupa importaciones relacionadas del mismo módulo
4. **Evitar nombres conflictivos**: Usa aliases cuando haya conflictos de nombres
5. **Documentar dependencias**: Comenta qué módulos son necesarios

## Limitaciones Conocidas

- Los módulos no pueden tener dependencias cíclicas
- Los módulos deben estar en el mismo árbol de directorios
- No hay soporte para módulos remotos o paquetes
- No hay versionado de módulos

## Futuras Mejoras (Roadmap)

- [ ] Sistema de paquetes
- [ ] Módulos de la biblioteca estándar
- [ ] Importación selectiva con wildcard
- [ ] Caché de módulos compilados
- [ ] Soporte para módulos en subdirectorios profundos
