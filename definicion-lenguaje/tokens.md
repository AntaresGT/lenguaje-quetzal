# Tokens y análisis léxico

> Referencia completa de las unidades léxicas (tokens) reconocidas por
> el lexer de Lenguaje Quetzal.

## 1. Comentarios

| Estilo | Soporte | Ejemplo |
|---|---|---|
| Línea `//` | **Sí (único estilo observado)** | `// comentario hasta fin de línea` |
| Bloque `/* */` | No observado en ejemplos | — |
| Hash `#` | No observado | — |

## 2. Palabras reservadas

Las palabras reservadas están en **español** y el lexer **normaliza
tildes**, por lo que las versiones con y sin tilde son equivalentes.

### 2.1 Tipos primitivos

| Keyword | Significado | Variantes |
|---|---|---|
| `entero` | Entero de 64 bits (i64) | — |
| `número` | Decimal de coma flotante | `numero` |
| `texto` | Cadena Unicode | — |
| `log` | Booleano (verdadero/falso) | — |
| `lista` | Arreglo, admite genérico `<T>` | — |
| `jsn` | Objeto/JSON | — |
| `vacío` | Tipo unitario (solo en retornos) | `vacio` |

### 2.2 Control de flujo

| Keyword | Equivalente inglés |
|---|---|
| `si` | `if` |
| `sino` | `else` |
| `sino si` | `else if` |
| `mientras` | `while` |
| `para` | `for` |
| `hacer` | `do` |
| `en` | `in` (iteración) |
| `cada` | `forEach` (sinónimo de `en`) |
| `romper` | `break` |
| `continuar` | `continue` |
| `retornar` | `return` |

### 2.3 Excepciones

| Keyword | Equivalente inglés |
|---|---|
| `intentar` | `try` |
| `capturar` | `catch` |
| `finalmente` | `finally` |
| `lanzar` | `throw` |
| `excepcion` | `exception` (tipo de la variable capturada) — `excepción` |

### 2.4 Programación orientada a objetos

| Keyword | Significado |
|---|---|
| `objeto` | `class` |
| `prototipo` | `interface` (con miembros opcionales) |
| `hereda` | `extends` |
| `implementa` | `implements` |
| `como` | alias de padre / de import |
| `nuevo` | `new` (instanciación) |
| `esto` | `this` |
| `padre` | `super` |
| `publico` | `public` — `público` |
| `privado` | `private` |
| `libre` | `static` (miembro sin instancia) |
| `opcional` | marca miembros opcionales en prototipos |

### 2.5 Asincronía

| Keyword | Equivalente | Variantes |
|---|---|---|
| `asincrono` | `async` | `asincróno` |
| `esperar` | `await` | — |

### 2.6 Módulos

| Keyword | Equivalente inglés |
|---|---|
| `importar` | `import` |
| `exportar` | `export` |
| `desde` | `from` |

### 2.7 Mutabilidad y literales

| Keyword | Significado |
|---|---|
| `var` | Marca una declaración como mutable |
| `verdadero` | Literal booleano `true` |
| `falso` | Literal booleano `false` |
| `nulo` | Literal `null` (asignable a cualquier tipo) |

### 2.8 Operadores lógicos como palabras

> **Nota:** Aunque el lexer define tokens para `y`, `o`, `no`, los
> ejemplos no los ejercitan. La forma estándar es usar `&&`, `||`, `!`.

| Keyword | Equivalente símbolo |
|---|---|
| `y` | `&&` |
| `o` | `\|\|` |
| `no` | `!` |

## 3. Operadores

### 3.1 Aritméticos

| Operador | Token | Ejemplo |
|---|---|---|
| `+` | `Mas` | `a + b` |
| `-` | `Menos` | `a - b` |
| `*` | `Por` | `a * b` |
| `/` | `Entre` | `a / b` |
| `%` | `Modulo` | `valor % 2` |
| `++` | `Incremento` | `i++` |
| `--` | `Decremento` | `valor--` |

> **No existe `**` para potencia** — se usa `Matemática.potencia(2, 10)`.

### 3.2 Asignación

| Operador | Token | Ejemplo |
|---|---|---|
| `=` | `Asignar` | `x = 5` |
| `+=` | `MasIgual` | `x += 3` |
| `-=` | `MenosIgual` | `x -= 1` |
| `*=` | `PorIgual` | `x *= 2` |
| `/=` | `EntreIgual` | `x /= 4` |
| `%=` | `ModuloIgual` | `x %= 3` |

### 3.3 Comparación

| Operador | Token |
|---|---|
| `==` | `IgualIgual` |
| `!=` | `Diferente` |
| `<` | `Menor` |
| `>` | `Mayor` |
| `<=` | `MenorOIgual` |
| `>=` | `MayorOIgual` |

### 3.4 Lógicos

| Operador | Token |
|---|---|
| `!` | `NoLogico` |
| `&&` | `YLogico` |
| `\|\|` | `OLogico` |

### 3.5 Ternario

| Operador | Uso |
|---|---|
| `? :` | `cond ? expr_si : expr_sino` |

## 4. Delimitadores y puntuación

| Símbolo | Uso |
|---|---|
| `( )` | Parámetros, agrupación, condiciones |
| `{ }` | Bloques de código, cuerpos de objetos/prototipos, bloques `exportar { }` |
| `[ ]` | Literales de lista, acceso por índice, genéricos `lista<T>`, acceso JSON `obj["clave"]` |
| `,` | Separador de parámetros/elementos/imports |
| `;` | Separador de sentencias (en `for` clásico y al final de algunas expresiones) |
| `:` | Pares clave-valor en JSON, etiquetas `privado:`/`publico:`, parte del ternario |
| `.` | Acceso a miembros, números decimales, llamadas a métodos en literales (`1234.texto()`) |
| `< >` | Genéricos en tipos: `lista<entero>` |

## 5. Literales

### 5.1 Números

```quetzal
1234           // entero
3.1416         // decimal
-10            // negativo (prefijo unario)
lista[-1]      // índice desde el final
```

### 5.2 Cadenas

```quetzal
"texto simple"            // comillas dobles (única forma observada)
t"hola {nombre}!"         // template string con interpolación
t"línea 1\nlínea 2"       // escapes \n, \t, \", \\, \{, \}
"clave con espacio"       // sin tildes
```

> **No se observan**: comillas simples, backticks.

### 5.3 Booleanos

```quetzal
verdadero   // true
falso       // false
```

### 5.4 Nulo

```quetzal
nulo   // null — asignable a cualquier tipo
```

### 5.5 Listas

```quetzal
[1, 2, 3]                          // sin tipo (en contexto se infiere)
["mixto", 1, verdadero, [2, 3]]    // heterogénea
[]                                 // vacía
```

### 5.6 Objetos

```quetzal
{
    clave: "valor",                  // clave sin comillas
    "clave con espacios": 42,        // clave con comillas
    "123clave": "ok",                // clave con número al inicio
    símbolo_ñáéíóú: "unicode"        // clave con tildes/ñ
}
```

### 5.7 Expresiones regulares

> No tienen sintaxis literal propia. Se construyen vía
> `nuevo ExpresiónRegular("patrón")` y se modifican con métodos
> `con_ignorar_mayúsculas(true)`, etc.

## 6. Identificadores

```ebnf
IDENTIFICADOR = ( letra | "_" ) , { letra | digito | "_" } ;
letra         = A-Z | a-z | áéíóúÁÉÍÓÚñÑ ;
```

> Los identificadores pueden contener **letras con tildes y ñ**.
> Ejemplo: `valor_ñáéíóú`, `MiClase`.
