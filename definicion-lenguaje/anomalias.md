# Anomalías, inconsistencias y observaciones

> Hallazgos del análisis de los archivos de ejemplo que podrían
> requerir atención al implementar parsers, herramientas o
> documentación externa.

## 1. Manifiesto `quetzal.json`: los archivos reales no cumplen el esquema canónico

**Conclusión**: el esquema canónico (con tildes) es la fuente de verdad.
Los `quetzal.json` reales del repositorio que usan campos sin tildes
**deben corregirse** para cumplir el esquema.

| Campo | Esquema canónico (`definicion-lenguaje/esquema_quetzal.json`) | Archivos reales (ej: `ejemplos/hola_mundo/quetzal.json`) |
|---|---|---|
| Versión del proyecto | `versión` (con tilde) | `version` (sin tilde) ❌ |
| Nombre del proyecto | `aplicación` (con tilde) | `aplicacion` (sin tilde) ❌ |
| Tipo de proyecto (enum) | `aplicación` \| `biblioteca` | `aplicacion` \| `biblioteca` ❌ |
| Versión del lenguaje | `quetzal` | `quetzal` ✓ |
| Entrada | `entrada` | `entrada` ✓ |

**Ejemplo real** (`ejemplos/hola_mundo/quetzal.json`) — **NO CUMPLE**:
```json
{
  "version": "0.1.0",
  "aplicacion": "hola_mundo",
  "entrada": "principal.qz",
  "tipo": "aplicacion",
  "quetzal": "0.2.0"
}
```

**Forma canónica correcta**:
```json
{
  "versión": "0.1.0",
  "aplicación": "hola_mundo",
  "entrada": "principal.qz",
  "tipo": "aplicación",
  "quetzal": "0.2.0"
}
```

> **Acción recomendada**:
> 1. Corregir los `quetzal.json` reales del repositorio para usar tildes.
> 2. Cualquier parser/validador debe usar la versión canónica como referencia.
> 3. Si por compatibilidad se quieren aceptar ambas formas durante una
>    transición, normalizar al cargar.

## 2. Operadores lógicos como palabras: definidos pero no usados

- El lexer define tokens para `y` (`YLogico`), `o` (`OLogico`) y `no`
  (`NoLogico`).
- Sin embargo, los ejemplos **solo usan símbolos** `&&`, `||`, `!`.
- El único operador lógico como palabra que se observa en ejemplos
  no es lógico sino de iteración: `cada` (sinónimo de `en`).

> **Implicación**: probablemente se planeó soportar tanto `y`/`o`/`no`
> como `&&`/`||`/`!` pero solo se documentó/exhortó la forma simbólica.

## 3. Operador ternario no documentado formalmente

- En `objetos.qz:66` aparece: `retornar valor < 0 ? -valor : valor`
- El token `?` (Interrogacion) sí está en el lexer.
- No hay evidencia en los archivos `.qz` de cadenas de `if/else`
  complejas que serían candidatas a refactorizarse con ternario.

> **Implicación**: el ternario parece soportado pero poco usado;
> convendría documentarlo explícitamente.

## 4. Llamada a constructor padre con `padre.Clase(...)`

Convención no estándar: en lugar de `super(...)`, Quetzal usa
`padre.Animal(...)`, `padre.Mamifero(...)`, etc. Esto sugiere que
`padre` no es una keyword que invoca un constructor especial, sino un
**objeto** con referencias a las clases padre.

```quetzal
objeto Perro hereda Animal {
    publico:
        Perro(texto n) {
            padre.Animal(n, "Canis")   // llamada como método
        }
}
```

> **Implicación**: la sintaxis es legal y útil, pero rompe la
> convención `super(args)` de la mayoría de lenguajes. Un parser debe
> permitir identificadores arbitrarios después de `padre.`.

## 5. JSON con claves con y sin comillas

```quetzal
jsn valor_json = {
    clave: "valor",                  // sin comillas (válido)
    "clave con espacio": "valor",    // con comillas (válido)
    "123clave": 456,                 // número al inicio (válido)
    "valorEspecial!@#": "símbolos"   // símbolos (válido)
}
```

> **Implicación**: el lexer debe tratar las claves de objeto con
> reglas distintas a las de identificadores comunes (aceptar más
> caracteres cuando están entre comillas).

## 6. Tildes en keywords y nombres de módulos nativos

Hay variabilidad en el uso de tildes en los nombres de módulos
nativos:

```quetzal
importar { ExpresiónRegular } desde "quetzal/motor"        // sin tilde en módulo
importar { Matemática } desde "quetzal/matemática"           // con tilde
importar { Tiempo } desde "quetzal/tiempo"                  // sin tilde
```

> **Implicación**: el resolver de módulos debe normalizar tildes en
> las rutas, o ambos formatos son aceptados explícitamente.

## 7. `nuevo` obligatorio en instanciación

No se observa instanciación sin la keyword `nuevo`:

```quetzal
Usuario u = nuevo Usuario("Ana", 30)      // siempre presente
```

Contraste con lenguajes como Kotlin o Scala donde se puede omitir.
Esta es una convención **consistente** en los ejemplos, no una
inconsistencia, pero conviene documentarlo explícitamente.

## 8. Inmutabilidad por defecto y `var`

- La palabra `var` aparece en posiciones **no estándar** comparada
  con C#/Java/Kotlin/Swift (donde va antes del nombre):
  ```quetzal
  entero var contador = 0   // después del tipo
  ```
- Es la posición de Kotlin (`var x: Int`) pero sin los dos puntos
  separadores de tipo.

> No es una inconsistencia, solo una **nota** para quien venga de
> otros lenguajes.

## 9. Comentarios solo `//`

No se observan `/* */` ni `#`. Si el lexer los rechaza formalmente,
no hay forma documentada de hacer comentarios de bloque.

> **Implicación**: si se quieren agregar banners de documentación
> grandes, hay que usar muchas líneas de `//`.

## 10. Genéricos solo en listas y JSON

`lista<T>` y `jsn` (sin genérico) son los únicos tipos parametrizados.
No se observa `Mapa<K, V>`, `Conjunto<T>`, tuplas, etc.

> **Implicación**: para pares clave-valor se usa `jsn`; para conjuntos
> no hay tipo dedicado.

## 11. Constructores sin tipo de retorno

```quetzal
objeto Usuario {
    publico:
        Usuario(texto n) {       // sin tipo de retorno
            ...
        }
}
```

Convención Java/C++. **Consistente** en los ejemplos.

## 12. Operadores de incremento `++` y `--`

Se observan como **prefijo** (no se observa uso posfijo en los
ejemplos revisados), aunque la gramática los acepta en ambos:

```quetzal
iterador_mientras++    // post-incremento (observado en bucles.qz)
valor--                // post-decremento (observado en operadores.qz)
```

> **Implicación**: la sintaxis es simétrica pero los ejemplos solo
> muestran una variante.

## 13. `nulo` polimórfico

`nulo` puede asignarse a **cualquier tipo** sin coerción. Esto
implica que todos los tipos tienen un valor "vacío" implícito, lo que
es poderoso pero menos seguro que los nullable types de TypeScript o
los `Option<T>` de Rust.

## 14. Resumen de acciones recomendadas

| Hallazgo | Acción sugerida |
|---|---|
| Tildes en `quetzal.json` | Corregir los archivos reales para usar tildes (el esquema es la verdad) |
| `y`/`o`/`no` no usados | Decidir: deprecarlos o promoverlos |
| Operador ternario | Documentarlo en la referencia |
| `padre.Clase()` | Documentar convención de herencia |
| Claves JSON flexibles | Documentar reglas de claves |
| Sin comentarios de bloque | Decidir: agregar `/* */` o documentar limitación |
| Tildes en rutas de import | Documentar normalización |

## 15. Bugs en el esquema canónico (`esquema_quetzal.json`)

El esquema canónico (guardado en `definicion-lenguaje/esquema_quetzal.json`)
contiene los siguientes **bugs menores** que no afectan a la mayoría de
archivos pero conviene corregir:

### 15.1 `permisos[].ejecutables` — falta el `type`

**Bug**: el campo está definido como `"array": {...}` en vez de
`"type": "array", "items": {...}`.

```json
// Incorrecto (esquema actual):
"ejecutables": {
  "array": {                                  // ← debería ser "type": "array"
    "items": {
      "type": "string",
      "description": "Ruta del ejecutable"
    }
  },
  "examples": [...]
}

// Correcto:
"ejecutables": {
  "type": "array",
  "items": {
    "type": "string",
    "description": "Ruta del ejecutable"
  },
  "examples": [...]
}
```

### 15.2 `permisos[].directorios[].permiso` — falta el `type`

**Bug**: usa `"enum": {...}` en lugar de `"type": "string", "enum": [...]`.

```json
// Incorrecto:
"permiso": {
  "enum": {                                   // ← falta "type": "string"
    "description": "Permiso del directorio",
    "enum": ["lectura", "escritura", "todo"]
  }
}

// Correcto:
"permiso": {
  "type": "string",
  "description": "Permiso del directorio",
  "enum": ["lectura", "escritura", "todo"]
}
```

### 15.3 `entrada` — `pattern` demasiado restrictivo

El campo `entrada` define:
```json
"pattern": "^[a-zA-Z0-9_-ñÑáéíóúÁÉÍÓÚ]+$"
```

Pero el ejemplo es `aplicación/principal.qz`, que **contiene `/` y `.`**
y por tanto **no cumple el patrón**. Solución: ampliar el patrón a:
```regex
^[a-zA-Z0-9_/.\-ñÑáéíóúÁÉÍÓÚ]+$
```

### 15.4 `descripcion` y `palabras_clave` — no permiten espacios

El campo `descripcion` y los items de `palabras_clave` usan:
```json
"pattern": "^[a-zA-Z0-9_-ñÑáéíóúÁÉÍÓÚ]+$"
```

Esto **no permite espacios**, lo que hace imposible escribir una
descripción natural como:

> *"Mi proyecto Quetzal es un lenguaje de programación interpretado
> en español"*

Solución: relajar el patrón o permitir texto libre.

### 15.5 `licencia` — no permite nombres humanos

Similar al anterior: el patrón no permite espacios, así que `"MIT"`
funciona pero `"MIT License"` o `"Apache License 2.0"` no.

### 15.6 `autor` y `email` — patrón inconsistente con ejemplos

- El patrón de `autor` no permite espacios, pero los ejemplos son
  `"Juan Pérez"` y `"Maria Gómez"`. **No cumplen** el patrón.
- El campo `email` permite espacios por la regla `+`, lo cual es
  técnicamente inválido para emails.

---

> **Nota metodológica**: estas observaciones se basan en los 21
> archivos `.qz` analizados. Puede haber sintaxis adicional que no
> esté ejercitada en los ejemplos pero que sí esté implementada
> formalmente en el lexer/parser del repositorio.
