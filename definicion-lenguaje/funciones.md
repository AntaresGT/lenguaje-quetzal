# Funciones

> Declaración, parámetros, retorno, async y callbacks nombrados.
> Formato: **Objetivo** · **Funciones** · **Funciones de la API**.

```yaml
doc: funciones
keywords: [retornar, asincrono, asincróno, esperar, funcion, var]
sin: [lambdas, funciones_anónimas, flecha_->]
```

---

## funciones

### Objetivo
Procedimientos con tipos explícitos en params y retorno. Primera clase vía nombre.

### Funciones
- Decl: `[asincrono] <tipo> <nombre>(<params>) { <cuerpo> }`
- Param: `<tipo> [var] <nombre>`
- Retorno: `retornar [<expr>]`
- Recursión nativa
- Async: `asincrono` + `esperar`
- Callbacks: tipo `funcion`; métodos `libre` / enlazados
- **Sin** funciones anónimas

### Funciones de la API
Ninguna built-in de “Function”; la API es la invocación `(args)` y stdlib que acepta `funcion`.

## 1. Declaración de función

### 1.1 Sintaxis general

```ebnf
funcion = [ "asincrono" ] , tipo , IDENTIFICADOR , "(" , [ parametros ] , ")" , bloque ;
```

> El modificador `asincrono` (o `asincróno`) es **opcional** y debe
> ir al principio.

### 1.2 Ejemplo mínimo

```quetzal
vacio saludar() {
    consola.mostrar("¡Hola, Quetzal!")
}
```

### 1.3 Función con tipo de retorno

```quetzal
número sumar(número a, número b) {
    retornar a + b
}
```

### 1.4 Función con retorno de lista tipada

```quetzal
lista<entero> obtener_numeros() {
    retornar [1, 2, 3, 4, 5]
}
```

### 1.5 Función con retorno de JSON

```quetzal
jsn obtener_datos() {
    retornar { nombre: "Ana", edad: 30 }
}
```

## 2. Parámetros

### 2.1 Sintaxis

```ebnf
parametros = parametro , { "," , parametro } ;
parametro  = tipo , [ "var" ] , IDENTIFICADOR ;
```

### 2.2 Ejemplos

```quetzal
// Parámetros inmutables (por defecto)
número sumar(número a, número b) {
    retornar a + b
}

// Parámetros mutables (con var)
texto agregar_sufijo(texto var palabra, texto sufijo) {
    palabra += sufijo              // modifica la copia local
    retornar palabra
}
```

**Reglas**:
- Cada parámetro lleva **anotación de tipo explícita**.
- `var` opcional marca el parámetro como **modificable localmente**.
- Sin `var`, intentar reasignar el parámetro produce error de tipo
  (inmutabilidad).

### 2.3 Lista de parámetros vacía

```quetzal
vacio funcion_sin_args() {
    consola.mostrar("nada")
}
```

## 3. Retorno

### 3.1 Con valor

```quetzal
entero doble(entero n) {
    retornar n * 2
}
```

### 3.2 Sin valor (void)

```quetzal
vacio imprimir(texto mensaje) {
    consola.mostrar(mensaje)
    retornar        // opcional en funciones vacio
}
```

### 3.3 Retorno temprano

```quetzal
texto clasificar(entero n) {
    si (n < 0) {
        retornar "negativo"      // sale inmediatamente
    }
    retornar "no negativo"
}
```

## 4. Recursividad

Quetzal soporta recursión nativa:

```quetzal
entero factorial(entero n) {
    si (n <= 1) {
        retornar 1
    }
    retornar n * factorial(n - 1)
}

// Versión concisa en una línea
entero factorial_compacta(entero n) {
    retornar n == 0 ? 1 : n * factorial_compacta(n - 1)
}
```

## 5. Funciones asíncronas

### 5.1 Declaración

```quetzal
asincrono número tarea(entero valor) {
    retornar valor * 2
}
```

> Variante con tilde: `asincróno número tarea(entero valor) { ... }`

### 5.2 Llamada con `esperar`

```quetzal
asincrono vacio ejecutar() {
    número resultado = esperar tarea(10)
    consola.mostrar("Resultado: " + resultado.texto())
}
```

### 5.3 `esperar` fuera de función asíncrona

`esperar` también puede aparecer en el nivel superior de un programa
asíncrono, o en el archivo de entrada:

```quetzal
número resultado = esperar tarea(3)   // a nivel de módulo
```

## 6. Funciones como valores (callbacks)

Toda función declarada es un **valor de primera clase**: su nombre, sin
paréntesis, es una referencia a la función y puede pasarse como
argumento. Quetzal **no tiene funciones anónimas**: un callback siempre
se declara fuera y se pasa por su nombre.

### 6.1 El tipo `funcion`

El tipo `funcion` declara un parámetro (o variable) que recibe otra
función. No fija la firma: la cantidad y el tipo de los argumentos se
verifican al invocarla.

```quetzal
entero doble(entero valor) {
    retornar valor * 2
}

entero aplicar(funcion accion, entero valor) {
    retornar accion(valor)
}

entero resultado = aplicar(doble, 21)   // 42
```

### 6.2 Callbacks con tipos de módulos nativos

Los parámetros del callback pueden ser tipos que exporta un módulo
nativo. Es la forma en que se registran manejadores e interceptores en
`quetzal/red`:

```quetzal
importar { ServidorHttp, PeticiónEntrante, RespuestaSaliente } desde "quetzal/red"

asincrono vacio saludar(PeticiónEntrante petición, RespuestaSaliente respuesta) {
    respuesta.jsn({ mensaje: "hola" })
}

ServidorHttp servidor = nuevo ServidorHttp()
servidor.obtener("/saludo", saludar)
```

### 6.3 Callbacks asíncronos

Un callback declarado `asincrono` puede usar `esperar` dentro de su
cuerpo. Quien lo invoca (el módulo nativo o `esperar`) resuelve la tarea
antes de continuar.

### 6.4 Métodos como valores

Los métodos también se referencian con la notación de punto, sin
paréntesis.

`Objeto.metodo` referencia un método `libre`: se invoca sin instancia.

```quetzal
objeto Util {
    libre entero doble(entero valor) {
        retornar valor * 2
    }
}

entero resultado = aplicar(Util.doble, 21)   // 42
```

`instancia.metodo` produce un **método enlazado**: el valor lleva
consigo la instancia, así que `esto` sigue apuntando a ella cuando el
callback se ejecuta más tarde.

```quetzal
objeto Contador {
    privado:
        entero var total = 0
    publico:
        vacio sumar(entero valor) {
            esto.total = esto.total + valor
        }
        entero leer() {
            retornar esto.total
        }
}

Contador contador = nuevo Contador()
funcion sumar = contador.sumar
sumar(5)
entero total = contador.leer()   // 5
```

Es la forma de registrar manejadores que viven dentro de un objeto:

```quetzal
esto.m_enrutador.obtener("/tipo-cambio", tipocambioct.obtener_tipo_cambio)
```

Un método enlazado `asincrono` se comporta igual que una función
`asincrono`: al invocarlo devuelve una tarea que se resuelve con
`esperar`.

## 7. Resumen

| Concepto | Sintaxis |
|---|---|
| Declaración | `[asincrono] <tipo> <nombre>(<params>) { <cuerpo> }` |
| Parámetro | `<tipo> [var] <nombre>` |
| Retorno | `retornar [<expresión>] ;` |
| Función asíncrona | anteponer `asincrono` / `asincróno` |
| Esperar resultado | `esperar <expresión_llamada>` |
| Callback | `funcion <nombre>` como tipo de parámetro |
| Método `libre` como valor | `Objeto.metodo` |
| Método enlazado | `instancia.metodo` |
