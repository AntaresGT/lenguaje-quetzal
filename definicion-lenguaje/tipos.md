# Tipos de datos

> Tipos primitivos y compuestos de Lenguaje Quetzal.
> Formato por feature: **Objetivo** · **Funciones** · **Funciones de la API**.
> Catálogo machine-readable: [`catalogo.json`](./catalogo.json).

```yaml
doc: tipos
pipeline: lexer → parser → semántica(tipos) → evaluador
keywords_tipo: [entero, número, numero, texto, log, vacío, vacio, lista, jsn, funcion]
```

---

## tipo `entero`

### Objetivo
Entero con signo de 64 bits (`i64`) para conteos, índices y aritmética exacta.

### Funciones
- Declaración: `entero nombre = expr`
- Literales: `42`, negativos unarios `-10`
- Ops: `+ - * / % ++ --` y comparación
- Asignable a `nulo`

### Funciones de la API
| Método | Retorna | Descripción |
|---|---|---|
| `texto()` | `texto` | conversión a cadena |
| `logico()` | `log` | `0` → falso, ≠0 → verdadero |
| `absoluto()` | `entero` | valor absoluto |
| `entero()` | `entero` | identidad |
| `numero()` / `número()` | `número` | a decimal |

```quetzal
entero edad = 41
entero var contador = 0
consola.mostrar(edad.texto())
```

---

## tipo `número` / `numero`

### Objetivo
Decimal IEEE 754 double para cálculo real.

### Funciones
- Declaración: `número x = 3.14` (o `numero`)
- Literales decimales `3.1416`
- Mismas ops aritméticas/comparación que `entero`
- Equivalencia de tilde: `número` ≡ `numero` (lexer)

### Funciones de la API
| Método | Retorna | Descripción |
|---|---|---|
| `texto()` | `texto` | a cadena |
| `logico()` | `log` | `0.0` → falso |
| `absoluto()` | `número` | \|x\| |
| `entero()` | `entero` | truncación |
| `numero()` / `número()` | `número` | identidad |

```quetzal
número pi = 3.1416
número area = pi * 5.0 * 5.0
```

---

## tipo `texto`

### Objetivo
Cadena Unicode UTF-8. Única forma de literal observada: comillas dobles.
Interpolación con prefijo `t`.

### Funciones
- Literal: `"hola"`
- Template: `t"hola {nombre}!"` con escapes `\n \t \" \\ \{ \}`
- Concatenación: `+`
- Índice: `s[0]`, `s[-1]` (desde el final)
- Sin comillas simples ni backticks

### Funciones de la API
Resumen (detalle completo: [`metodos-nativos.md`](./metodos-nativos.md) §1):

| Grupo | Métodos |
|---|---|
| Tamaño | `longitud()`, `esta_vacia()` |
| Caso | `mayusculas()`, `minusculas()`, `capitalizar()`, `titulo()` |
| Trim | `recortar()`, `recortar_inicio()`, `recortar_final()` |
| Búsqueda | `contiene`, `empieza_con`, `termina_con`, `encontrar`, `buscar_ultimo`, `contar` |
| Reemplazo | `reemplazar`, `reemplazar_primero` |
| División | `dividir`, `partir_lineas` |
| Subcadena | `subtexto`, `izquierda`, `derecha` |
| Transform | `repetir`, `invertir` |
| Validación | `es_numero`, `es_entero`, `es_alfanumerico`, `igual_sin_caso` |
| Codec | `a_base64`, `decodificar_base64`, `a_enlace`, `decodificar_enlace` |
| Conversión | `entero()`, `numero()`/`número()`, `logico()`, `texto()`, `lista()`, `jsn()` |

```quetzal
texto saludo = "hola"
texto msg = t"{saludo}, mundo"
texto may = saludo.mayusculas()
```

---

## tipo `log`

### Objetivo
Booleano de un bit.

### Funciones
- Literales: `verdadero`, `falso`
- Ops: `&&`, `||`, `!` (lexer también: `y`, `o`, `no`)
- Condiciones de `si` / bucles

### Funciones de la API
| Método | Retorna | Descripción |
|---|---|---|
| `texto()` | `texto` | `"verdadero"` / `"falso"` |

```quetzal
log activo = verdadero
si (activo && !falso) {
    consola.mostrar("ok")
}
```

---

## tipo `vacío` / `vacio`

### Objetivo
Tipo unitario: solo como retorno de función sin valor.

### Funciones
- Anotación: `vacio saludar() { … }`
- `retornar` sin expresión (opcional al final)
- No produce valor asignable útil

### Funciones de la API
Ninguna.

```quetzal
vacio saludar() {
    consola.mostrar("hola")
}
```

---

## tipo `lista` / `lista<T>`

### Objetivo
Arreglo ordenado. Genérico opcional: tipado estático o heterogéneo.

### Funciones
- Literales: `[1, 2, 3]`, `[]`
- Tipado: `lista<entero> xs = [1, 2]`
- Heterogéneo: `lista mixta = [1, "a", verdadero]`
- Índice: `xs[0]`, `xs[-1]`
- Iteración: `para (entero var n en xs)` / `cada`
- Mutación de contenido si la variable es `var`

### Funciones de la API
Detalle: [`metodos-nativos.md`](./metodos-nativos.md) §2.

| Grupo | Métodos |
|---|---|
| Tamaño | `longitud()`, `esta_vacia()`, `primero()`, `ultimo()` |
| Mutación | `agregar`, `insertar`, `remover`, `quitar_en`, `limpiar` |
| Búsqueda | `contiene`, `buscar`, `buscar_ultimo`, `contar` |
| Orden | `ordenar`, `ordenar_descendente`, `ordenado`, `invertir` |
| Slice | `tomar`, `saltar`, `sublista` |
| Agregado | `concatenar`, `extender` |
| Numérico | `sumar`, `promedio`, `maximo`, `minimo` |
| Texto | `unir`, `texto`, `json`, `logico` |

**Global:** `rango(inicio, fin)` → `lista` de enteros inclusiva.

```quetzal
lista<entero> var nums = [1, 2, 3]
nums.agregar(4)
lista r = rango(1, 5)   // [1,2,3,4,5]
```

---

## tipo `jsn`

### Objetivo
Objeto JSON nativo (mapa clave → valor).

### Funciones
- Literal: `{ nombre: "Ana", edad: 30 }`
- Claves con o sin comillas (ver [`tokens.md`](./tokens.md) §5.6)
- Acceso: `obj.nombre` / `obj["clave con espacio"]`

### Funciones de la API
| Método | Descripción |
|---|---|
| `contiene_clave(clave)` | existencia |
| `claves()` / `valores()` | listas |
| `establecer(clave, valor)` | upsert |
| `eliminar(clave)` | borra |
| `fusionar(otro)` | merge in-place |
| `texto()` / `texto_formateado()` | serializa |
| `jsn()` | parsea desde texto (en receptor texto) |

```quetzal
jsn persona = { nombre: "Ana", edad: 30 }
persona.establecer("ciudad", "GT")
```

---

## tipo `funcion`

### Objetivo
Referencia a función de primera clase para callbacks. **Sin lambdas.**

### Funciones
- Parámetro/variable: `funcion accion`
- Pasar nombre sin `()`: `aplicar(doble, 21)`
- `Objeto.metodo` = método `libre`
- `instancia.metodo` = método enlazado (conserva `esto`)
- Firma no fijada en el tipo; se verifica al invocar

### Funciones de la API
Invocación: `accion(args…)`. Ver [`funciones.md`](./funciones.md) §6.

```quetzal
entero doble(entero v) { retornar v * 2 }
entero aplicar(funcion f, entero v) { retornar f(v) }
entero r = aplicar(doble, 21)   // 42
```

---

## valor `nulo`

### Objetivo
Ausencia polimórfica. No existe `null` / `undefined` / `indefinido`.

### Funciones
- Asignable a **cualquier** tipo sin coerción
- Comparación con `==` / `!=`

### Funciones de la API
Ninguna propia.

```quetzal
entero x = nulo
texto z = nulo
```

---

## mutabilidad `var`

### Objetivo
Inmutable por defecto. `var` tras el tipo habilita reasignación.

### Funciones
- Sintaxis: `<tipo> [var] <nombre> = <expresión>`
- También en parámetros: `texto var palabra`
- Sin `var` ≡ const efectiva

### Funciones de la API
Ninguna.

```quetzal
entero contador = 0        // inmutable
entero var total = 0       // mutable
total = total + 1
```

---

## Conversiones cruzadas

Regla: conversión = **método del valor fuente** (no cast global).

```
texto → entero/número/log/jsn/lista : .entero() .número() .log() .jsn() .lista()
*     → texto                       : .texto()
```

## Inferencia

**No existe.** Toda declaración lleva tipo explícito.

## Equivalencias de tilde

| Canónico | Alternativa |
|---|---|
| `número` | `numero` |
| `vacío` | `vacio` |
| `público` | `publico` |
| `asincróno` | `asincrono` |
| `excepción` | `excepcion` |
