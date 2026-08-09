# Estructuras de control

> Condicionales, bucles, flujo directo y excepciones.
> Formato: **Objetivo** · **Funciones** · **Funciones de la API**.
> Catálogo: [`catalogo.json`](./catalogo.json) · grafo: [`diagramas.md`](./diagramas.md).

```yaml
doc: control-flujo
keywords: [si, sino, mientras, hacer, para, en, cada, romper, continuar, retornar,
           intentar, capturar, finalmente, lanzar, excepcion]
```

---

## condicional `si`

### Objetivo
Ramificación condicional. Paréntesis y llaves **obligatorios**.

### Funciones
- `si (cond) { … }`
- `sino si (cond) { … }` (keywords concatenadas)
- `sino { … }`
- Ternario: `cond ? a : b` (precedencia más baja)

### Funciones de la API
Ninguna.

## 1. Condicionales

### 1.1 `si` básico

```quetzal
si (condicion) {
    // bloque si verdadero
}
```

### 1.2 `si / sino`

```quetzal
si (edad >= 18) {
    consola.mostrar("mayor de edad")
} sino {
    consola.mostrar("menor de edad")
}
```

### 1.3 `si / sino si / sino` (cadena)

```quetzal
si (edad > 60) {
    consola.mostrar("tercera edad")
} sino si (edad > 18) {
    consola.mostrar("mayor de edad")
} sino {
    consola.mostrar("menor de edad")
}
```

**Reglas**:
- Los paréntesis `( )` alrededor de la condición son **obligatorios**.
- Las llaves `{ }` del bloque son **obligatorias** (incluso para una
  sola sentencia).
- `sino si` se construye concatenando las dos keywords.

## bucles

### Objetivo
Repetición con cuatro formas + `romper` / `continuar`.

### Funciones
| Forma | Sintaxis |
|---|---|
| while | `mientras (cond) { }` |
| do-while | `hacer { } mientras (cond);` |
| for C | `para (init; cond; paso) { }` |
| for-in | `para (tipo var id en\|cada coll) { }` |
| break/continue | `romper` / `continuar` |

### Funciones de la API
Ninguna. `en` ≡ `cada`.

## 2. Bucles

### 2.1 `mientras` (while)

```quetzal
entero var i = 0
mientras (i < 10) {
    consola.mostrar("i = " + i.texto())
    i++
}
```

### 2.2 `hacer { ... } mientras (...)` (do-while)

```quetzal
entero var n = 0
hacer {
    consola.mostrar("se ejecuta al menos una vez")
    n++
} mientras (n < 3)
```

> El bloque se ejecuta **al menos una vez**, antes de evaluar la
> condición. La condición va seguida de `;`.

### 2.3 `para` estilo C (for)

```quetzal
para (entero var i = 0; i < 5; i++) {
    consola.mostrar("iteración " + i.texto())
}
```

Estructura: `para (init; condición; paso) { bloque }`

### 2.4 `para ... en` (for-in)

```quetzal
lista<entero> numeros = [10, 20, 30]
para (entero var n en numeros) {
    consola.mostrar("valor: " + n.texto())
}
```

### 2.5 `para ... cada` (forEach — sinónimo de `en`)

```quetzal
para (entero var n cada numeros) {
    consola.mostrar("valor: " + n.texto())
}
```

> `en` y `cada` producen el **mismo resultado**.

## 3. Control de flujo directo

### 3.1 `romper` (break)

```quetzal
mientras (verdadero) {
    si (condicion_salida) {
        romper    // sale del bucle más cercano
    }
}
```

### 3.2 `continuar` (continue)

```quetzal
para (entero var i = 0; i < 10; i++) {
    si (i % 2 == 0) {
        continuar    // salta a la siguiente iteración
    }
    consola.mostrar("impar: " + i.texto())
}
```

### 3.3 `retornar` (return)

```quetzal
número sumar(número a, número b) {
    retornar a + b
}

vacio saludar() {
    consola.mostrar("hola")
    retornar    // retorno temprano sin valor
}
```

## excepciones

### Objetivo
Errores capturables con limpieza opcional. Un solo tipo `excepcion`.

### Funciones
- `intentar { }` (obligatorio)
- `capturar (excepcion id) { }` (obligatorio)
- `finalmente { }` (opcional; siempre corre)
- `lanzar <expresión>` (cualquier valor; típico: texto)

### Funciones de la API
| Atributo | Tipo | Descripción |
|---|---|---|
| `e.mensaje` | `texto` | mensaje |
| `e.llamadas` | `lista` | call stack |

## 4. Manejo de excepciones

### 4.1 Sintaxis completa

```quetzal
intentar {
    // código que puede fallar
    número resultado = dividir(10, 0)
} capturar (excepcion e) {
    // manejo del error
    consola.mostrar(e.mensaje)
} finalmente {
    // limpieza (opcional, se ejecuta siempre)
    consola.mostrar("limpieza")
}
```

**Componentes**:
- `intentar { ... }` — bloque protegido (obligatorio).
- `capturar (excepcion id) { ... }` — manejador (obligatorio).
- `finalmente { ... }` — bloque de cleanup (opcional, se ejecuta
  incluso si no hubo excepción o si hubo `retornar`/`romper`).

### 4.2 `lanzar` (throw)

```quetzal
número dividir(número numerador, número denominador) {
    si (denominador == 0) {
        lanzar "División por cero no permitida"
    }
    retornar numerador / denominador
}
```

> `lanzar` acepta **cualquier expresión** (típicamente una cadena con
> el mensaje, pero puede ser un objeto, un número, etc.).

### 4.3 Atributos del objeto `excepcion`

| Atributo | Tipo | Descripción |
|---|---|---|
| `e.mensaje` | `texto` | Mensaje asociado al error |
| `e.llamadas` | `lista` | Pila de llamadas (call stack) |

> No se observa un sistema jerárquico de tipos de excepción (todas
> las excepciones son del tipo genérico `excepcion`).

## 5. Operador ternario

Quetzal tiene un operador ternario estilo C:

```quetzal
texto clasificar(entero valor) {
    retornar valor < 0 ? "negativo" : (valor == 0 ? "cero" : "positivo")
}
```

**Precedencia**: más bajo que cualquier operador binario (se evalúa al
final). Ver `gramatica.ebnf.md` §11.

## 6. Resumen de keywords de control

| Keyword | Uso |
|---|---|
| `si` | `if` |
| `sino` | `else` |
| `sino si` | `else if` |
| `mientras` | `while` |
| `hacer` | `do` (inicio de do-while) |
| `para` | `for` (admite 3 formas) |
| `en` | iterador `for-in` |
| `cada` | sinónimo de `en` |
| `romper` | `break` |
| `continuar` | `continue` |
| `retornar` | `return` |
| `intentar` | `try` |
| `capturar` | `catch` |
| `finalmente` | `finally` |
| `lanzar` | `throw` |
| `excepcion` | tipo del valor capturado en `catch` |
