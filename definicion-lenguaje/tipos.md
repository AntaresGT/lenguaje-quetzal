# Tipos de datos

> Tipos primitivos y compuestos del Lenguaje Quetzal, con su sintaxis
> de declaración, mutabilidad, conversiones y manejo del valor `nulo`.

## 1. Tipos primitivos

| Tipo | Keyword | Tamaño/representación | Ejemplo |
|---|---|---|---|
| Entero | `entero` | 64 bits con signo (i64) | `42` |
| Decimal | `número` | IEEE 754 double | `3.1416` |
| Texto | `texto` | Unicode (UTF-8) | `"hola"` |
| Booleano | `log` | 1 bit (`verdadero`/`falso`) | `verdadero` |
| Vacío | `vacío` | Tipo unitario (solo retornos) | (no produce valor) |

> **Variantes sin tilde**: `numero` ≡ `número`, `vacio` ≡ `vacío`.

## 2. Tipos compuestos

### 2.1 Lista

```quetzal
lista vacia = []                           // sin tipo (en contexto)
lista<entero> numeros = [1, 2, 3]          // tipada
lista mixta = [1, "texto", verdadero]      // heterogénea
```

- Admite **genérico opcional** `<T>`.
- Si se especifica, los elementos se verifican estáticamente.
- Si no se especifica, la lista es heterogénea.

### 2.2 JSON (objeto)

```quetzal
jsn persona = { nombre: "Ana", edad: 30 }
```

Las claves pueden ir con o sin comillas (ver `tokens.md` §5.6).

## 3. Declaración y mutabilidad

**Sintaxis**: `<tipo> [var] <nombre> = <expresión>`

```quetzal
entero contador = 0              // inmutable (por defecto)
entero var total = 0             // mutable (por palabra 'var')

texto saludo = "hola"            // inmutable
texto var buffer = ""            // mutable

lista var items = []             // lista mutable (se puede agregar/quitar)
lista items_constantes = [1,2,3] // la lista en sí no se puede reasignar
                                 //   (pero los items podrían ser var)
```

**Reglas**:
- `var` se coloca **después del tipo** y **antes del nombre**.
- Sin `var`, la variable es efectivamente `const`.
- `var` se puede aplicar también a **parámetros de función** para
  permitir mutarlos dentro del cuerpo.

## 4. El valor `nulo`

`nulo` es el valor de ausencia. Es **polimórfico** — puede asignarse a
cualquier tipo:

```quetzal
entero  x = nulo
número  y = nulo
texto   z = nulo
log     w = nulo
lista   l = nulo
jsn     o = nulo
```

> **No se observa** `null` (inglés) ni `indefinido` / `undefined`.

## 5. Conversión de tipos

Quetzal no usa casting estilo `int(x)`. En su lugar, las conversiones
son **métodos de extensión** disponibles sobre cualquier valor:

```quetzal
// Desde texto
entero  i = "1234".entero()              // 1234
número  n = "1234.56".número()          // 1234.56
log     b = "verdadero".log()            // verdadero
jsn     j = "{\"a\":1}".jsn()            // {a: 1}
lista<entero> l = "1,2,3".lista()       // [1, 2, 3]

// Desde número/otros a texto
texto t1 = 1234.texto()                  // "1234"
texto t2 = 3.14.texto()                  // "3.14"
texto t3 = verdadero.texto()             // "verdadero"
texto t4 = mi_lista.texto()              // "[1, 2, 3]"
texto t5 = mi_json.texto()               // serialización JSON
```

> Ver `metodos-nativos.md` para la lista completa de conversiones
> por tipo (cadenas, números, listas, JSON, booleanos).

## 6. Inferencia de tipos

**No se observa inferencia de tipos**. Todas las declaraciones llevan
anotación de tipo explícita:

```quetzal
entero x = 5            // tipo explícito
número y = 3.14         // tipo explícito
texto  s = "hola"       // tipo explícito
```

## 7. Anotación de tipos en parámetros y retornos

Siempre obligatorias en signaturas:

```quetzal
número sumar(número a, número b) {       // tipo en parámetros y retorno
    retornar a + b
}

vacio saludar() {                        // void
    consola.mostrar("hola")
}

texto formatear(entero id) -> texto {    // (sintaxis -> no observada;
    retornar t"id={id}"                  //  en la práctica solo se usa
}                                        //  la sintaxis sin flecha)
```

> **Observación**: en los ejemplos revisados, la sintaxis siempre es
> `<tipo> nombre(params) { ... }` con el tipo antes del nombre.
> Una sintaxis con `->` no se observa.

## 8. Resumen: equivalencias importantes

| Forma canónica | Formas alternativas (todas válidas) |
|---|---|
| `número` | `numero` |
| `vacío` | `vacio` |
| `público` | `publico` |
| `asincróno` | `asincrono` |
| `excepción` | `excepcion` |

Esta equivalencia se debe a la **normalización Unicode** que el lexer
aplica a identificadores y keywords.
