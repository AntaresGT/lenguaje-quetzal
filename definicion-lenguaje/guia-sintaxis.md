# Guía completa de sintaxis de Quetzal

> Todo lo que necesitas para escribir programas en Quetzal: tipos, operadores,
> control de flujo, funciones, objetos, módulos, excepciones y más. Con ejemplos
> verificados contra el intérprete oficial (v0.0.5).
> Proyecto: [github.com/AntaresGT/lenguaje-quetzal](https://github.com/AntaresGT/lenguaje-quetzal).

---

## 1. ¿Qué es Quetzal?

Quetzal es un lenguaje de programación interpretado, escrito en Rust, con
palabras clave y biblioteca estándar completamente en español. No es solo para
aprender: busca ser de propósito general (servidores, archivos, red, consola).

Sus reglas básicas:

- Tipado explícito: toda variable y todo parámetro llevan su tipo. No hay
  inferencia.
- Immutable por defecto: para poder reasignar una variable escribe `var`
  después del tipo.
- Llaves y paréntesis obligatorios en condiciones y bloques.
- Sin punto y coma al final de cada línea.
- Tildes opcionales: `número` y `numero`, `vacío` y `vacio`, `publico` y
  `público` son equivalentes.
- Los archivos usan la extensión `.qz`.

### Instalación y ejecución

Necesitas [Rust](https://rustup.rs/) y Git. Compila desde el código fuente:

```bash
git clone https://github.com/AntaresGT/lenguaje-quetzal.git
cd lenguaje-quetzal
cargo build --release
# Binario: target/release/quetzal
```

| Comando | Qué hace |
|---|---|
| `quetzal archivo.qz` | Ejecuta un archivo |
| `quetzal ejecutar [ARCHIVO]` | Ejecuta un archivo o el proyecto (`quetzal.json`) |
| `quetzal revisar [ARCHIVO]` | Analiza el código sin ejecutarlo |
| `quetzal compilar ENTRADA -o APP` | Genera un ejecutable único |
| `quetzal nuevo NOMBRE` | Crea un proyecto nuevo |
| `quetzal instalar [PAQUETE]` | Instala dependencias |
| `quetzal` | Abre el REPL interactivo |

También existe una extensión oficial para Visual Studio Code con resaltado y
diagnósticos.

### Tu primer programa

```qz
consola.mostrar("¡Hola, Quetzal!")
```

Salida:

```text
¡Hola, Quetzal!
```

## 2. Comentarios

Solo existe el comentario de línea con `//`.

```qz
// Esto es un comentario
entero total = 10   // también al final de una línea
```

## 3. Variables y tipos

Sintaxis: `tipo [var] nombre = valor`.

| Tipo | Descripción | Ejemplo |
|---|---|---|
| `entero` | Entero de 64 bits | `42`, `-10` |
| `número` | Decimal (coma flotante) | `3.1416` |
| `texto` | Cadena Unicode | `"hola"` |
| `log` | Lógico | `verdadero`, `falso` |
| `lista` / `lista<T>` | Arreglo ordenado | `[1, 2, 3]` |
| `jsn` | Objeto JSON nativo | `{ clave: "valor" }` |
| `vacío` | Sin valor (solo retornos) | `vacio f() { }` |
| `funcion` | Referencia a función | `aplicar(doble, 2)` |

```qz
// Variables y tipos
entero edad = 30
número precio = 19.99
texto nombre = "Ana"
log activo = verdadero
lista<entero> notas = [90, 85, 100]
jsn persona = { nombre: "Luis", ciudad: "Guatemala" }
entero var contador = 0
contador = contador + 1
texto vacio_aun = nulo
consola.mostrar(t"{nombre} tiene {edad} años")
consola.mostrar("Contador: " + contador.texto())
consola.mostrar(persona.ciudad)
consola.mostrar(notas[-1].texto())
```

Salida:

```text
Ana tiene 30 años
Contador: 1
Guatemala
100
```

Puntos clave:

- `nulo` se puede asignar a cualquier tipo.
- Los textos con prefijo `t` interpolan expresiones entre llaves:
  `t"Hola {nombre}"`. Escapes: `\n` `\t` `\"` `\\` `\{` `\}`.
- Solo comillas dobles para textos.
- Índices negativos cuentan desde el final: `lista[-1]`.
- Los identificadores admiten tildes y `ñ`: `año_de_ingreso`.
- Conversión con métodos del valor: `.texto()`, `.entero()`, `.número()`,
  `.logico()`, `.jsn()`, `.lista()`.

## 4. Operadores

| Grupo | Operadores |
|---|---|
| Aritméticos | `+ - * / %` · `++ --` |
| Asignación | `= += -= *= /= %=` |
| Comparación | `== != < > <= >=` |
| Lógicos | `&&` (y) · `\|\|` (o) · `!` (no) |
| Ternario | `condición ? si_verdadero : si_falso` |
| Concatenación | `+` entre textos |

No hay operador de potencia `**`: usa `Matemática.potencia(base, exp)`.

```qz
entero var x = 10
x += 5
x -= 3
x *= 2
x /= 4
x %= 4
x++
consola.mostrar("x = " + x.texto())
consola.mostrar((7 % 3).texto())
log a = verdadero
log b = falso
consola.mostrar((a && !b).texto())
consola.mostrar((a || b).texto())
consola.mostrar((5 >= 3).texto())
texto r = x > 2 ? "grande" : "pequeño"
consola.mostrar(r)
```

Salida:

```text
x = 3
1
verdadero
verdadero
verdadero
grande
```

## 5. Condicionales y bucles

`si / sino si / sino`; bucles `mientras`, `hacer … mientras`, `para` estilo C y
`para … en` (o su sinónimo `cada`). `romper` sale del bucle y `continuar` salta
a la siguiente vuelta.

```qz
entero edad = 20
si (edad > 60) {
    consola.mostrar("tercera edad")
} sino si (edad >= 18) {
    consola.mostrar("mayor de edad")
} sino {
    consola.mostrar("menor de edad")
}
entero var i = 0
mientras (i < 3) {
    consola.mostrar("mientras " + i.texto())
    i++
}
entero var n = 0
hacer {
    n++
} mientras (n < 3)
consola.mostrar("hacer: " + n.texto())
para (entero var k = 0; k < 6; k++) {
    si (k % 2 == 0) {
        continuar
    }
    si (k == 5) {
        romper
    }
    consola.mostrar("impar " + k.texto())
}
lista<entero> numeros = [10, 20, 30]
para (entero var v en numeros) {
    consola.mostrar("en " + v.texto())
}
para (entero var v cada numeros) {
    consola.mostrar("cada " + v.texto())
}
```

Salida:

```text
mayor de edad
mientras 0
mientras 1
mientras 2
hacer: 3
impar 1
impar 3
en 10
en 20
en 30
cada 10
cada 20
cada 30
```

## 6. Funciones

Sintaxis: `[asincrono] tipo_retorno nombre(tipo param, …) { … }`. Los
parámetros son inmutables salvo que lleven `var`. Las funciones son valores:
pasa su nombre (sin paréntesis) a un parámetro de tipo `funcion`. No hay
funciones anónimas. Para asincronía usa `asincrono` y `esperar`.

```qz
número sumar(número a, número b) {
    retornar a + b
}
texto agregar_sufijo(texto var palabra, texto sufijo) {
    palabra += sufijo
    retornar palabra
}
entero factorial(entero n) {
    retornar n <= 1 ? 1 : n * factorial(n - 1)
}
vacio saludar(texto quien) {
    consola.mostrar(t"¡Hola, {quien}!")
}
entero doble(entero v) {
    retornar v * 2
}
entero aplicar(funcion f, entero v) {
    retornar f(v)
}
asincrono número tarea(entero valor) {
    retornar valor * 2
}
consola.mostrar(sumar(2.5, 1.5).texto())
consola.mostrar(agregar_sufijo("Quetz", "al"))
consola.mostrar(factorial(5).texto())
saludar("Guatemala")
consola.mostrar(aplicar(doble, 21).texto())
número res = esperar tarea(10)
consola.mostrar(res.texto())
```

Salida:

```text
4
Quetzal
120
¡Hola, Guatemala!
42
20
```

## 7. Estructuras de datos y métodos

```qz
lista<entero> var nums = [5, 3, 8]
nums.agregar(1)
consola.mostrar(nums.longitud().texto())
consola.mostrar(nums.ordenado().texto())
consola.mostrar(nums.sumar().texto())
consola.mostrar(nums.contiene(8).texto())
lista r = rango(1, 5)
consola.mostrar(r.texto())
lista<texto> frutas = ["mango", "piña", "banano"]
consola.mostrar(frutas.unir(", "))
jsn var persona = { nombre: "Ana", edad: 30, "año de ingreso": 2024 }
persona.establecer("ciudad", "Antigua")
consola.mostrar(persona.nombre)
consola.mostrar(persona["año de ingreso"].texto())
consola.mostrar(persona.contiene_clave("ciudad").texto())
consola.mostrar(persona.claves().texto())
texto s = "  Hola Quetzal  "
consola.mostrar(s.recortar().mayusculas())
consola.mostrar("a,b,c".dividir(",").texto())
consola.mostrar("123".entero() + 1)
consola.mostrar("quetzal".contiene("tz").texto())
```

Salida:

```text
4
[1, 3, 5, 8]
17
verdadero
[1, 2, 3, 4, 5]
mango, piña, banano
Ana
2024
verdadero
[nombre, edad, año de ingreso, ciudad]
HOLA QUETZAL
[a, b, c]
124
verdadero
```

| Tipo | Métodos principales |
|---|---|
| `texto` | longitud, esta_vacia, mayusculas, minusculas, capitalizar, titulo, recortar, contiene, empieza_con, termina_con, encontrar, contar, reemplazar, dividir, partir_lineas, subtexto, izquierda, derecha, repetir, invertir, es_numero, es_entero, a_base64 |
| `lista` | longitud, esta_vacia, primero, ultimo, agregar, insertar, remover, quitar_en, limpiar, contiene, buscar, contar, ordenar, ordenado, ordenar_descendente, invertir, tomar, saltar, sublista, concatenar, extender, sumar, promedio, maximo, minimo, unir |
| `jsn` | contiene_clave, claves, valores, establecer, eliminar, fusionar, texto, texto_formateado |
| `entero` / `número` | texto, logico, absoluto, entero, número |
| Global | `rango(inicio, fin)`: lista de enteros, ambos extremos incluidos |

## 8. Objetos y prototipos

`objeto` = clase; `prototipo` = interfaz (miembros opcionales no son
obligatorios). Secciones `publico:` y `privado:`. El constructor se llama igual
que el objeto y no lleva tipo de retorno. Dentro de los métodos, accede a los
atributos con `esto.atributo`. `hereda` admite varios padres, `implementa`,
`padre.Clase(...)` para el constructor padre. `libre` = miembro estático.
Instancia siempre con `nuevo`.

```qz
prototipo Describible {
    publico:
    texto describir()
}
objeto Animal implementa Describible {
    publico:
    texto nombre
    texto especie
    Animal(texto n, texto e) {
        esto.nombre = n
        esto.especie = e
    }
    texto describir() {
        retornar t"{esto.nombre} ({esto.especie})"
    }
}
objeto Perro hereda Animal {
    publico:
    Perro(texto n) {
        padre.Animal(n, "Canis familiaris")
    }
    texto ladrar() {
        retornar "¡Guau!"
    }
}
objeto Cuenta {
    privado:
    número var saldo
    publico:
    Cuenta(número inicial) {
        esto.saldo = inicial
    }
    vacio depositar(número monto) {
        si (monto <= 0) {
            lanzar "El monto debe ser positivo"
        }
        esto.saldo += monto
    }
    número ver_saldo() {
        retornar esto.saldo
    }
}
objeto Util {
    libre entero absoluto(entero v) {
        retornar v < 0 ? -v : v
    }
}
Perro p = nuevo Perro("Firulais")
consola.mostrar(p.describir())
consola.mostrar(p.ladrar())
Cuenta c = nuevo Cuenta(100.0)
c.depositar(50.0)
consola.mostrar(c.ver_saldo().texto())
consola.mostrar(Util.absoluto(-10).texto())
```

Salida:

```text
Firulais (Canis familiaris)
¡Guau!
150
10
```

## 9. Módulos

Exporta con `exportar { … }` e importa con `importar { … } desde "ruta"`. Usa
`como` para renombrar. Las rutas pueden ser relativas (`./`, `../`) o nativas
(`quetzal/…`).

**calculadora.qz**

```qz
entero sumar(entero a, entero b) {
    retornar a + b
}
texto saludo = "Hola desde el módulo"
exportar { sumar, saludo }
```

**principal.qz**

```qz
importar { sumar, saludo como mensaje } desde "./calculadora.qz"
consola.mostrar(mensaje)
consola.mostrar(sumar(2, 3).texto())
```

Salida:

```text
Hola desde el módulo
5
```

### Proyecto con `quetzal.json`

Un proyecto de varios archivos se describe en `quetzal.json`, incluidos los
permisos de seguridad (archivos, red, entorno):

```json
{
  "versión": "1.0.0",
  "aplicación": "configuracion_entorno",
  "entrada": "principal.qz",
  "tipo": "aplicación",
  "quetzal": "0.0.5",
  "permisos": [
    { "tipo": "entorno", "habilitado": true }
  ]
}
```

## 10. Errores y excepciones

`intentar / capturar (excepcion e) / finalmente` (opcional, siempre se
ejecuta). `lanzar` acepta cualquier expresión. La excepción expone `e.mensaje`
y `e.llamadas` (pila de llamadas).

```qz
número dividir(número a, número b) {
    si (b == 0) {
        lanzar "División por cero no permitida"
    }
    retornar a / b
}
intentar {
    número r = dividir(10, 0)
    consola.mostrar(r.texto())
} capturar (excepcion e) {
    consola.mostrar_error("Error: " + e.mensaje)
} finalmente {
    consola.mostrar("Fin")
}
```

Salida:

```text
Error: División por cero no permitida
Fin
```

## 11. Entrada y salida

`consola` está disponible sin importar nada.

| Método | Descripción |
|---|---|
| `consola.mostrar(x)` | Imprime una línea |
| `consola.mostrar_error(x)` | Imprime en rojo |
| `consola.mostrar_advertencia(x)` | Imprime en amarillo |
| `consola.mostrar_exito(x)` | Imprime en verde |
| `consola.mostrar_informacion(x)` | Imprime en azul |
| `consola.pedir(mensaje)` | Lee una línea del teclado |
| `consola.pedir_secreto(mensaje)` | Lee sin mostrar lo escrito (contraseñas) |

```qz
texto nombre = consola.pedir("¿Cómo te llamas? ")
consola.mostrar_exito(t"¡Bienvenido, {nombre}!")
```

## 12. Biblioteca estándar

| Módulo | Importación | Para qué |
|---|---|---|
| `consola` (global) | — | Entrada y salida |
| `Matemática` | `importar { Matemática } desde "quetzal/matemática"` | PI, E, potencia, raiz_cuadrada, redondear, trigonometría, aleatorios |
| `Tiempo` | `importar { Tiempo } desde "quetzal/tiempo"` | Fechas, horas y zonas horarias |
| `ExpresiónRegular` | `importar { ExpresiónRegular } desde "quetzal/motor"` | Expresiones regulares |
| `sistema_archivos` | `importar { ... } desde "quetzal/sistema_archivos"` | Archivos (permiso `sistema-archivos`) |
| `red` | `importar { ... } desde "quetzal/red"` | Cliente y servidor HTTP (permiso `red`) |
| `entorno` | `importar { entorno } desde "quetzal/entorno"` | Variables de sistema y archivos `.env` (permiso `entorno`) |

```qz
importar { Matemática } desde "quetzal/matemática"
importar { Tiempo } desde "quetzal/tiempo"
consola.mostrar(Matemática.potencia(2, 10).texto())
consola.mostrar(Matemática.raiz_cuadrada(81).texto())
```

Salida:

```text
1024
9
```

### Configuración con `.env` (módulo entorno)

Requiere el permiso `entorno` en `quetzal.json`. Funciones: `cargar()`,
`obtener(nombre)` (devuelve `nulo` si no existe), `obtener_o(nombre, defecto)`
y `contiene(nombre)`. No subas tu `.env` real al repositorio: publica solo un
`.env.example`.

```qz
// Ejemplo de configuración externa para una aplicación Quetzal.
importar { entorno } desde "quetzal/entorno"

entorno.cargar()
texto modo = entorno.obtener_o("MODO", "desarrollo")
texto puerto = entorno.obtener_o("PUERTO", "3000")
consola.mostrar("Modo: " + modo)
consola.mostrar("Puerto: " + puerto)
```

Salida:

```text
Modo: desarrollo
Puerto: 3000
```

## 13. Palabras reservadas

| Palabra | Significado | Palabra | Significado |
|---|---|---|---|
| `entero` | entero 64 bits | `capturar` | catch |
| `número` / `numero` | decimal | `finalmente` | finally |
| `texto` | cadena | `lanzar` | throw |
| `log` | lógico | `excepcion` | tipo del error |
| `lista` | arreglo | `objeto` | clase |
| `jsn` | objeto JSON | `prototipo` | interfaz |
| `vacío` / `vacio` | sin valor | `hereda` | extiende |
| `funcion` | referencia a función | `implementa` | implementa |
| `var` | mutable | `nuevo` | instanciar |
| `verdadero` / `falso` | lógicos | `esto` | this |
| `nulo` | sin valor | `padre` | super |
| `si` / `sino` | if / else | `publico` / `privado` | visibilidad |
| `mientras` | while | `libre` | estático |
| `hacer` | do | `opcional` | miembro opcional |
| `para` | for | `como` | alias |
| `en` / `cada` | in | `asincrono` | async |
| `romper` | break | `esperar` | await |
| `continuar` | continue | `importar` | import |
| `retornar` | return | `exportar` | export |
| `intentar` | try | `desde` | from |

## 14. Ejemplo completo

Un pequeño inventario que combina objetos, listas, bucles y excepciones:

```qz
// Inventario de una tienda
objeto Producto {
    publico:
    texto nombre
    número precio
    entero var existencias
    Producto(texto n, número p, entero e) {
        esto.nombre = n
        esto.precio = p
        esto.existencias = e
    }
    vacio vender(entero cantidad) {
        si (cantidad > esto.existencias) {
            lanzar t"No hay suficiente {esto.nombre}"
        }
        esto.existencias -= cantidad
    }
}
lista<Producto> inventario = [
    nuevo Producto("Café", 45.0, 10),
    nuevo Producto("Pan", 2.5, 3)
]
intentar {
    inventario[0].vender(4)
    inventario[1].vender(5)
} capturar (excepcion e) {
    consola.mostrar_error(e.mensaje)
}
número var total = 0.0
para (Producto var p en inventario) {
    número valor = p.precio * p.existencias
    total += valor
    consola.mostrar(t"{p.nombre}: {p.existencias} u. = Q{valor}")
}
consola.mostrar(t"Valor total: Q{total}")
```

Salida:

```text
Café: 6 u. = Q270
Pan: 3 u. = Q7.5
Valor total: Q277.5
```

## 15. Novedades recientes

- **Interrupción con Ctrl+C**: la ejecución de un programa (y el REPL) se
  puede detener limpiamente con Ctrl+C.
- **Módulo `quetzal/entorno`**: lectura de variables de sistema y archivos
  `.env`, protegida por el nuevo permiso `entorno` en `quetzal.json`.

Todos los ejemplos de esta guía se ejecutaron contra el intérprete oficial de
Quetzal (rama `main`, v0.0.5, octubre de 2026).
