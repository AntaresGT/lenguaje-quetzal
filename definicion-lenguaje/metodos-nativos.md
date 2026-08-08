# Métodos nativos (built-in)

> Métodos incorporados en los tipos primitivos y compuestos de
> Lenguaje Quetzal, organizados por tipo receptor.

---

## 1. Métodos de `texto`

### 1.1 Tamaño y verificación

| Método | Retorna | Descripción |
|---|---|---|
| `longitud()` | `entero` | cantidad de caracteres |
| `esta_vacia()` | `log` | true si la cadena está vacía |

### 1.2 Mayúsculas / minúsculas

| Método | Descripción |
|---|---|
| `mayusculas()` | convierte todo a mayúsculas |
| `minusculas()` | convierte todo a minúsculas |
| `capitalizar()` | primera letra en mayúscula |
| `titulo()` | cada palabra capitalizada |

### 1.3 Recorte (trim)

| Método | Descripción |
|---|---|
| `recortar()` | quita espacios al inicio y final |
| `recortar_inicio()` | solo al inicio |
| `recortar_final()` | solo al final |

### 1.4 Búsqueda y coincidencia

| Método | Descripción |
|---|---|
| `contiene(texto)` | true si contiene la subcadena |
| `empieza_con(texto)` | true si comienza con |
| `termina_con(texto)` | true si termina con |
| `encontrar(texto)` | posición de la primera ocurrencia o -1 |
| `buscar_ultimo(texto)` | posición de la última ocurrencia o -1 |
| `contar(texto)` | cantidad de ocurrencias |

### 1.5 Reemplazo

| Método | Descripción |
|---|---|
| `reemplazar(viejo, nuevo)` | reemplaza todas las ocurrencias |
| `reemplazar_primero(viejo, nuevo)` | solo la primera |

### 1.6 División

| Método | Descripción |
|---|---|
| `dividir(separador)` | divide en lista de subcadenas |
| `partir_lineas()` | divide por saltos de línea |

### 1.7 Subcadenas

| Método | Descripción |
|---|---|
| `subtexto(inicio, fin)` | subcadena entre índices |
| `izquierda(n)` | primeros n caracteres |
| `derecha(n)` | últimos n caracteres |
| `[indice]` | carácter en posición (también `[-1]` desde el final) |

### 1.8 Repetición y transformación

| Método | Descripción |
|---|---|
| `repetir(n)` | repite la cadena n veces |
| `invertir()` | invierte el orden de los caracteres |

### 1.9 Validación

| Método | Descripción |
|---|---|
| `es_numero()` | true si es parseable como número |
| `es_entero()` | true si es parseable como entero |
| `es_alfanumerico()` | true si solo letras y dígitos |
| `igual_sin_caso(texto)` | comparación case-insensitive |

### 1.10 Codificación

| Método | Descripción |
|---|---|
| `a_base64()` | codifica en base64 |
| `decodificar_base64()` | decodifica de base64 |
| `a_enlace()` | URL-encode |
| `decodificar_enlace()` | URL-decode |

### 1.11 Conversión

| Método | Convierte a |
|---|---|
| `entero()` | `entero` |
| `numero()` / `número()` | `número` |
| `logico()` | `log` |
| `texto()` | `texto` (identidad) |
| `lista()` | `lista<texto>` (split por coma) |

---

## 2. Métodos de `lista` / `lista<T>`

### 2.1 Tamaño y verificación

| Método | Retorna |
|---|---|
| `longitud()` | `entero` |
| `esta_vacia()` | `log` |

### 2.2 Acceso

| Método/Sintaxis | Descripción |
|---|---|
| `[indice]` | elemento en posición (negativos desde el final) |
| `primero()` | primer elemento |
| `ultimo()` | último elemento |

### 2.3 Modificación (sobre listas mutables)

| Método | Descripción |
|---|---|
| `agregar(elem)` | añade al final |
| `insertar(pos, elem)` | inserta en posición |
| `remover(elem)` | elimina por valor |
| `quitar_en(pos)` | elimina por posición |
| `limpiar()` | vacía la lista |

### 2.4 Búsqueda

| Método | Descripción |
|---|---|
| `contiene(elem)` | true si contiene |
| `buscar(elem)` | primera posición o -1 |
| `buscar_ultimo(elem)` | última posición o -1 |
| `contar(elem)` | cantidad de ocurrencias |

### 2.5 Orden

| Método | Descripción |
|---|---|
| `ordenar()` | ordena in-place (ascendente) |
| `ordenar_descendente()` | ordena in-place (descendente) |
| `ordenado()` | retorna nueva lista ordenada |
| `invertir()` | invierte in-place |

### 2.6 Slice (sublistas)

| Método | Descripción |
|---|---|
| `tomar(n)` | primeros n elementos |
| `saltar(n)` | salta los primeros n |
| `sublista(inicio, fin)` | sub-array entre índices |

### 2.7 Agregado

| Método | Descripción |
|---|---|
| `concatenar(otra)` | retorna nueva lista concatenada |
| `extender(otra)` | añade elementos in-place |

### 2.8 Numéricos (solo listas numéricas)

| Método | Descripción |
|---|---|
| `sumar()` | suma de elementos |
| `promedio()` | media aritmética |
| `maximo()` | valor máximo |
| `minimo()` | valor mínimo |

### 2.9 Conversión a texto

| Método | Descripción |
|---|---|
| `unir(separador)` | une en un texto con separador |
| `texto()` | serializa |
| `json()` | serializa como JSON |
| `logico()` | convierte a booleano |

### 2.10 Función global `rango`

```quetzal
lista r = rango(1, 10)   // [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
```

---

## 3. Métodos de `jsn` (objeto/JSON)

| Método | Descripción |
|---|---|
| `contiene_clave(clave)` | true si la clave existe |
| `claves()` | retorna lista de claves |
| `valores()` | retorna lista de valores |
| `establecer(clave, valor)` | crea o modifica propiedad |
| `eliminar(clave)` | borra la propiedad |
| `fusionar(otro)` | combina in-place |
| `texto()` | serializa |
| `texto_formateado()` | serializa con formato legible |
| `jsn()` | parsea desde texto |

---

## 4. Métodos de `log` (booleano)

| Método | Descripción |
|---|---|
| `texto()` | "verdadero" o "falso" |

---

## 5. Métodos de `entero` y `número`

| Método | Retorna | Descripción |
|---|---|---|
| `texto()` | `texto` | convierte a string |
| `logico()` | `log` | 0 → falso, !=0 → verdadero |
| `absoluto()` | mismo | valor absoluto |
| `entero()` | `entero` | trunca a entero |
| `numero()` / `número()` | `número` | identidad / conversión |

> Los literales numéricos pueden llamar métodos directamente:
> `1234.texto()`, `(3.14).absoluto()`.

---

## 6. Resumen de conversiones cruzadas

```
texto   → entero    : "1234".entero()
texto   → número    : "3.14".número()
texto   → log       : "verdadero".log()
texto   → jsn       : "{\"a\":1}".jsn()
texto   → lista     : "1,2,3".lista()

entero  → texto     : 1234.texto()
número  → texto     : 3.14.texto()
log     → texto     : verdadero.texto()
jsn     → texto     : mi_obj.texto()
lista   → texto     : mi_lista.texto()
```

> **Regla general**: los métodos de conversión están disponibles como
> **métodos del valor fuente** (no como funciones globales de cast).
