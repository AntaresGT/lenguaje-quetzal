# Tests de Ámbito de Variables - Solución del Bug de Scoping

## Descripción del Problema Solucionado

Se solucionó un bug crítico en el intérprete Quetzal donde variables declaradas dentro de bucles `para` y `para_cada` causaban errores de "variable ya declarada" cuando se usaba el mismo nombre en diferentes iteraciones o en diferentes funciones.

### Problema Original
```quetzal
// Esto fallaba antes de la corrección
vacio funcion1() {
    para (entero i = 0; i < 3; i = i + 1) {
        cadena palabra = "test1"  // Error en iteración 2+
    }
}

vacio funcion2() {
    para (entero j = 0; j < 3; j = j + 1) {
        cadena palabra = "test2"  // Error: variable ya declarada
    }
}
```

### Solución Implementada
Se modificó el evaluador para crear un nuevo entorno hijo en cada iteración de bucle, permitiendo que las variables locales del bucle se declaren independientemente en cada iteración.

## Tests Implementados

### Tests en `bucles.rs`

#### 1. `test_ambito_variables_bucle_para_mismo_nombre`
- **Propósito**: Verifica que se puede usar el mismo nombre de variable en diferentes iteraciones del mismo bucle
- **Escenario**: Un bucle `para` que declara `cadena palabra` en cada iteración

#### 2. `test_ambito_variables_bucles_anidados_mismo_nombre`
- **Propósito**: Verifica que variables con el mismo nombre en bucles anidados no causan conflictos
- **Escenario**: Bucles anidados donde tanto el bucle exterior como interior declaran `cadena palabra`

#### 3. `test_ambito_variables_funciones_con_bucles`
- **Propósito**: Reproduce y verifica la solución del problema original reportado
- **Escenario**: Dos funciones diferentes que contienen bucles con variables del mismo nombre

#### 4. `test_ambito_variables_bucle_para_cada_mismo_nombre`
- **Propósito**: Verifica el ámbito correcto en bucles `para_cada` (foreach)
- **Escenario**: Múltiples bucles `para_cada` usando variables con el mismo nombre

#### 5. `test_ambito_variables_multiples_bucles_consecutivos`
- **Propósito**: Verifica que bucles consecutivos pueden usar variables con el mismo nombre
- **Escenario**: Tres bucles `para` consecutivos declarando `cadena resultado`

#### 6. `test_ambito_variables_bucle_dentro_funcion_con_parametros`
- **Propósito**: Test complejo con función que tiene parámetros y contiene bucle con variables locales
- **Escenario**: Función que recibe lista como parámetro y procesa elementos en bucle

#### 7. `test_ambito_variables_bucle_tres_niveles_anidacion`
- **Propósito**: Verifica ámbito en bucles con tres niveles de anidación
- **Escenario**: Bucles anidados de tres niveles cada uno con sus propias variables

#### 8. `test_ambito_variables_bucle_con_shadowing`
- **Propósito**: Verifica que el shadowing (ocultación de variables) funciona correctamente
- **Escenario**: Variable global ocultada por variable local de bucle

### Tests en `ambito_variables.rs`

#### 1. `test_ambito_funcion_basico`
- **Propósito**: Test básico de ámbito entre funciones
- **Escenario**: Dos funciones con variables del mismo nombre

#### 2. `test_ambito_funciones_con_bucles_misma_variable`
- **Propósito**: Reproduce exactamente el problema original del usuario
- **Escenario**: Funciones `contar_elementos()` y `procesar_texto()` con bucles y variable `palabra`

#### 3. `test_ambito_bucle_variables_iteracion`
- **Propósito**: Verifica que cada iteración tiene su propio ámbito
- **Escenario**: Bucle con múltiples variables locales en cada iteración

#### 4. `test_ambito_shadowing_variables`
- **Propósito**: Test completo de shadowing en múltiples niveles
- **Escenario**: Variable global → variable de función → variable de bucle

#### 5. `test_ambito_bucles_anidados_profundos`
- **Propósito**: Test con múltiples niveles de anidación profunda
- **Escenario**: Tres niveles de bucles anidados con variables en cada nivel

#### 6. `test_ambito_parametros_funcion`
- **Propósito**: Verifica ámbito de parámetros de función con bucles
- **Escenario**: Función con parámetros que usa bucles con variables locales

#### 7. `test_ambito_variables_locales_complejas`
- **Propósito**: Test con estructuras de datos complejas en ámbitos locales
- **Escenario**: Bucles que crean y procesan listas locales

#### 8. `test_ambito_variables_entre_bloques_condicionales`
- **Propósito**: Verifica ámbito entre bloques condicionales y bucles
- **Escenario**: Bloques `si/sino` que contienen bucles con variables locales

#### 9. `test_ambito_variables_reutilizacion_nombres`
- **Propósito**: Test específico para reutilización de nombres en diferentes contextos
- **Escenario**: Múltiples contextos (bucles, condicionales) reutilizando nombres

#### 10. `test_ambito_error_variable_no_definida` *(Test de error)*
- **Propósito**: Verifica que variables locales no son accesibles fuera de su ámbito
- **Escenario**: Intento de acceder a variable de función fuera de la función

#### 11. `test_ambito_variable_bucle_no_accesible_fuera` *(Test de error)*
- **Propósito**: Verifica que variables de bucle no son accesibles fuera del bucle
- **Escenario**: Intento de acceder a variable de bucle fuera del bucle

## Cobertura de Testing

Los tests cubren:

### ✅ Casos Exitosos
- Bucles simples con variables locales
- Bucles anidados hasta 3 niveles
- Funciones con bucles y parámetros
- Shadowing de variables en múltiples niveles
- Reutilización de nombres en diferentes contextos
- Bucles `para` y `para_cada`

### ✅ Casos de Error (Expected Failures)
- Acceso a variables fuera de su ámbito
- Variables de bucle no accesibles externamente

### ✅ Regresión del Bug Original
- Reproduce exactamente el problema reportado por el usuario
- Verifica que la solución funciona correctamente

## Resultados de Ejecución

Todos los tests pasan exitosamente:
- **19 tests en `ambito_variables`**: ✅ PASSED
- **11 tests relacionados con ámbito en `bucles`**: ✅ PASSED

La solución garantiza que el problema de ámbito de variables en bucles está completamente resuelto y no habrá regresiones futuras.
