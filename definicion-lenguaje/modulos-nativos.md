# Módulos nativos (biblioteca estándar)

> Módulos que forman parte de la biblioteca estándar de Lenguaje
> Quetzal. Se importan desde rutas con el prefijo `quetzal/`.

## Índice

1. [`consola`](#1-consola) — entrada/salida (no requiere import)
2. [`quetzal/matemática`](#2-quetzalmatemática) — funciones matemáticas
3. [`quetzal/tiempo`](#3-quetzaltiempo) — fechas, horas, zonas
4. [`quetzal/motor`](#4-quetzalmotor) — expresiones regulares
5. [`quetzal/sistema_archivos`](#5-quetzalsistema_archivos) — archivos, directorios, datos binarios, flujos y observadores
6. [`quetzal/red`](#6-quetzalred) — servidores HTTP, cliente HTTP y códigos de estado

---

## 1. `consola`

> No requiere import — está disponible globalmente en todo programa.

```quetzal
consola.mostrar("texto")            // imprime en color por defecto
consola.mostrar_error("texto")      // imprime en rojo
consola.mostrar_advertencia("...")  // imprime en amarillo
consola.mostrar_exito("...")        // imprime en verde
consola.mostrar_informacion("...")  // imprime en azul

texto nombre = consola.pedir("¿Cómo te llamas? ")
texto secreto = consola.pedir_secreto("Contraseña: ")
```

| Método | Descripción |
|---|---|
| `consola.mostrar(texto)` | imprime línea (color por defecto) |
| `consola.mostrar_error(texto)` | imprime en rojo |
| `consola.mostrar_advertencia(texto)` | imprime en amarillo |
| `consola.mostrar_exito(texto)` | imprime en verde |
| `consola.mostrar_informacion(texto)` | imprime en azul |
| `consola.pedir(texto)` | lee una línea desde stdin (eco) |
| `consola.pedir_secreto(texto)` | lee sin eco (para passwords) |

---

## 2. `quetzal/matemática`

```quetzal
importar { Matemática } desde "quetzal/matemática"
```

### 2.1 Constantes

| Nombre | Valor | Descripción |
|---|---|---|
| `PI` | `3.141592653589793` | π |
| `TAU` | `6.283185307179586` | 2π |
| `E` | `2.718281828459045` | número de Euler |

### 2.2 Aritmética básica

| Función | Firma | Descripción |
|---|---|---|
| `sumar(a, b)` | `(número, número) → número` | a + b |
| `restar(a, b)` | `(número, número) → número` | a − b |
| `multiplicar(a, b)` | `(número, número) → número` | a · b |
| `dividir(a, b)` | `(número, número) → número` | a / b |
| `potencia(base, exp)` | `(número, número) → número` | base^exp |
| `absoluto(x)` | `(número) → número` | \|x\| |

### 2.3 Trigonometría

| Función | Descripción |
|---|---|
| `seno(rad)` | sin(rad) |
| `coseno(rad)` | cos(rad) |
| `tangente(rad)` | tan(rad) |
| `grados_a_radianes(g)` | g · π/180 |
| `radianes_a_grados(r)` | r · 180/π |
| `hipotenusa(c1, c2)` | √(c1² + c2²) |

### 2.4 Logaritmos y exponenciales

| Función | Descripción |
|---|---|
| `logaritmo(x)` | ln(x) |
| `logaritmo_base(x, base)` | log_base(x) |
| `exponencial(x)` | e^x |
| `raiz_cuadrada(x)` | √x |

### 2.5 Redondeo

| Función | Descripción |
|---|---|
| `redondear(x, decimales)` | redondea a n decimales |
| `piso(x)` | floor(x) |
| `techo(x)` | ceil(x) |

### 2.6 Estadística (sobre listas)

| Función | Descripción |
|---|---|
| `promedio(lista)` | media aritmética |
| `maximo(lista)` | valor máximo |
| `minimo(lista)` | valor mínimo |
| `suma_total(lista)` | suma de elementos |
| `producto_total(lista)` | producto de elementos |

### 2.7 Aleatorios

| Función | Descripción |
|---|---|
| `aleatorio()` | número aleatorio en [0.0, 1.0) |
| `aleatorio_rango(min, max)` | entero aleatorio en [min, max] |

### 2.8 Ejemplo

```quetzal
importar { Matemática } desde "quetzal/matemática"

número area = Matemática.PI * Matemática.potencia(5, 2)
consola.mostrar("Área del círculo: " + area.texto())

entero dado = Matemática.aleatorio_rango(1, 6)
```

---

## 3. `quetzal/tiempo`

```quetzal
importar { Tiempo } desde "quetzal/tiempo"
```

### 3.1 Constructores

```quetzal
Tiempo ahora = nuevo Tiempo()                        // momento actual
Tiempo desde_ms = nuevo Tiempo(1700000000000)        // desde epoch ms
Tiempo desde_texto = nuevo Tiempo("2024-01-15")      // desde texto
Tiempo fecha = nuevo Tiempo(2024, 1, 15)              // año, mes, día
Tiempo completo = nuevo Tiempo(2024, 1, 15, 14, 30, 0)               // y hora
Tiempo con_zona = nuevo Tiempo(2024, 1, 15, 14, 30, 0, "UTC")       // y zona
```

### 3.2 Funciones libres

| Función | Descripción |
|---|---|
| `Tiempo.ahora()` | momento actual en zona local |
| `Tiempo.ahora_utc()` | momento actual en UTC |
| `Tiempo.ahora_en(zona)` | momento actual en zona específica |
| `Tiempo.hoy()` | inicio del día actual |
| `Tiempo.marca()` | timestamp actual en ms |
| `Tiempo.desde_marca(ms)` | construye desde epoch ms |
| `Tiempo.analizar(texto, patron)` | parsea texto según patrón |
| `Tiempo.zona_local()` | nombre de la zona local |
| `Tiempo.zonas([region])` | lista de zonas disponibles |
| `Tiempo.es_bisiesto(año)` | true si el año es bisiesto |
| `Tiempo.dias_en_mes(año, mes)` | cantidad de días en el mes |

### 3.3 Componentes

| Método | Retorna | Descripción |
|---|---|---|
| `año()` | `entero` | año |
| `mes()` | `entero` | mes (1-12) |
| `dia()` | `entero` | día del mes (1-31) |
| `hora()` | `entero` | hora (0-23) |
| `minuto()` | `entero` | minuto (0-59) |
| `segundo()` | `entero` | segundo (0-59) |
| `milisegundo()` | `entero` | ms (0-999) |
| `dia_semana()` | `entero` | día de la semana (1-7) |
| `dia_año()` | `entero` | día del año (1-366) |
| `marca()` | `entero` | timestamp en ms |

### 3.4 Texto

| Método | Descripción |
|---|---|
| `texto()` | serialización completa |
| `texto_fecha()` | solo fecha |
| `texto_hora()` | solo hora |
| `formatear(patron)` | según patrón (similar a strftime) |
| `nombre_dia()` | nombre del día de la semana |
| `nombre_mes()` | nombre del mes |

### 3.5 Aritmética de fechas

| Método | Descripción |
|---|---|
| `agregar_años(n)` | añade n años |
| `agregar_meses(n)` | añade n meses |
| `agregar_dias(n)` | añade n días |
| `agregar_horas(n)` | añade n horas |
| `agregar_minutos(n)` | añade n minutos |
| `agregar_segundos(n)` | añade n segundos |
| `diferencia(otro)` | diferencia con otro Tiempo |

### 3.6 Comparación

| Método | Descripción |
|---|---|
| `es_antes(otro)` | true si es anterior |
| `es_despues(otro)` | true si es posterior |
| `es_mismo_instante(otro)` | true si es el mismo instante |
| `es_mismo_dia(otro)` | true si es el mismo día |

### 3.7 Zonas horarias

| Método | Descripción |
|---|---|
| `zona()` | zona actual |
| `desfase()` | offset UTC en minutos |
| `en_zona(zona)` | retorna copia en otra zona |
| `en_utc()` | retorna copia en UTC |
| `en_local()` | retorna copia en zona local |

### 3.8 Ejemplo

```quetzal
importar { Tiempo } desde "quetzal/tiempo"

Tiempo hoy = Tiempo.hoy()
consola.mostrar("Hoy es: " + hoy.texto_fecha())

Tiempo mañana = hoy.agregar_dias(1)
número diff = hoy.diferencia(mañana)
```

---

## 4. `quetzal/motor`

```quetzal
importar { ExpresiónRegular } desde "quetzal/motor"
```

> Provee la clase `ExpresiónRegular` y funciones de fábrica.

### 4.1 Constructor

```quetzal
ExpresiónRegular re = nuevo ExpresiónRegular("^\\d{4}-\\d{4}$")
```

### 4.2 Funciones libres (fábricas)

| Función | Descripción |
|---|---|
| `ExpresiónRegular.escapar(texto)` | escapa metacaracteres regex |
| `ExpresiónRegular.nueva_desde_comodín(gl)` | convierte glob → regex |
| `ExpresiónRegular.nueva_solo_dígitos()` | solo dígitos |
| `ExpresiónRegular.nueva_solo_letras()` | solo letras |
| `ExpresiónRegular.nueva_correo()` | validador de email |
| `ExpresiónRegular.nueva_url()` | validador de URL |
| `ExpresiónRegular.nueva_ipv4()` | validador de IPv4 |

### 4.3 Coincidencia

| Método | Retorna | Descripción |
|---|---|---|
| `coincide(texto)` | `log` | true si hay match en cualquier parte |
| `coincide_desde_inicio(texto)` | `log` | true si matchea desde el inicio |
| `coincide_completo(texto)` | `log` | true si matchea todo el texto |

### 4.4 Búsqueda

| Método | Retorna | Descripción |
|---|---|---|
| `buscar(texto)` | `texto\|nulo` | primer match o nulo |
| `buscar_posicion(texto)` | `(inicio, fin)` o nulo | posición del match |
| `buscar_todo(texto)` | `lista` | todos los matches |
| `buscar_grupos(texto)` | `jsn` | grupos capturados |
| `contar(texto)` | `entero` | cantidad de matches |

### 4.5 Reemplazo y división

| Método | Descripción |
|---|---|
| `reemplazar(texto, nuevo)` | reemplaza primer match |
| `reemplazar_todo(texto, nuevo)` | reemplaza todos |
| `dividir(texto)` | divide según el patrón |

### 4.6 Introspección

| Método | Retorna | Descripción |
|---|---|---|
| `es_válida()` | `log` | true si el patrón compila |
| `diagnosticar()` | `texto` | mensaje de error si inválida |
| `patrón()` | `texto` | el patrón original |
| `grupos_nombrados()` | `lista` | nombres de grupos |

### 4.7 Banderas (lectura)

| Método | Descripción |
|---|---|
| `ignorar_mayúsculas()` | true si flag `i` activa |
| `multilínea()` | true si flag `m` activa |
| `punto_total()` | true si flag `s` activa |
| `unicode()` | true si flag `u` activa |

### 4.8 Banderas (inmutabilidad)

Como el objeto `ExpresiónRegular` es inmutable, las banderas se
activan retornando una **nueva instancia**:

| Método | Descripción |
|---|---|
| `con_ignorar_mayúsculas(log)` | retorna copia con flag `i` |
| `con_multilínea(log)` | retorna copia con flag `m` |
| `con_punto_total(log)` | retorna copia con flag `s` |
| `con_unicode(log)` | retorna copia con flag `u` |

### 4.9 Ejemplo

```quetzal
importar { ExpresiónRegular } desde "quetzal/motor"

ExpresiónRegular re = nuevo ExpresiónRegular("\\d+")
log tiene_numeros = re.coincide("abc123")
texto primero = re.buscar("abc 456 def")     // "456"
lista todos = re.buscar_todo("a1 b22 c333")  // ["1", "22", "333"]
```

---

## 5. `quetzal/sistema_archivos`

```quetzal
importar { SistemaArchivos, Archivo, Bits, Flujo, Observador, EventoArchivo }
    desde "quetzal/sistema_archivos"
```

> Provee seis objetos:
>
> - `SistemaArchivos` — funciones `libre` (no se instancia) para operar
>   archivos y directorios.
> - `Archivo` — referencia a un archivo o directorio con metadatos y
>   operaciones propias (equivalente a `FileInfo` de C#).
> - `Bits` — secuencia de bytes en memoria para entrada/salida binaria.
> - `Flujo` — archivo abierto con **posición (cursor) persistente**: se
>   lee y se escribe por partes desde donde quedó.
> - `Observador` — vigila una ruta y notifica cambios del sistema de
>   archivos.
> - `EventoArchivo` — cada notificación que entrega un `Observador`.
>
> `quetzal/bits` se mantiene como ruta **legada** que reexporta `Bits`;
> la API canónica vive en `quetzal/sistema_archivos`.

### 5.1 Formas síncrona y asíncrona

Toda operación que toca el sistema tiene **dos** formas: la síncrona
(bloquea) y la asíncrona con sufijo `_asincrono` (se usa con `esperar`).

| Síncrona | Asíncrona |
|---|---|
| `SistemaArchivos.leer(ruta)` | `esperar SistemaArchivos.leer_asincrono(ruta)` |
| `archivo.escribir(texto)` | `esperar archivo.escribir_asincrono(texto)` |

```quetzal
asincrono vacio guardar() {
    esperar SistemaArchivos.escribir_asincrono("./salida/datos.txt", "hola")
}
```

> `esperar` solo es válido dentro de una función `asincrono` o en el
> scope global (ver [`funciones.md`](./funciones.md) §5).

Las operaciones de `Bits` que solo manipulan memoria **no** tienen forma
asíncrona ni requieren permisos.

### 5.2 Permisos requeridos

El módulo exige el permiso `sistema-archivos` en `quetzal.json`
(ver [`manifiesto.md`](./manifiesto.md) §6):

```json
{
  "permisos": [
    {
      "tipo": "sistema-archivos",
      "habilitado": true,
      "directorios": [
        { "ruta": "./", "permiso": "lectura" },
        { "ruta": "./salida", "permiso": "escritura" },
        { "ruta": "C:/directorio1/directorio2", "permiso": "lectura" },
        { "ruta": "/tmp", "permiso": "todo" }
      ]
    }
  ]
}
```

| Nivel | Operaciones autorizadas |
|---|---|
| `lectura` | listar directorios y archivos, leer contenido, comprobar existencia, consultar metadatos y abrir un `Flujo` de solo lectura |
| `escritura` | crear directorios (simple y recursivo), escribir, anexar y editar archivos, copiar, mover, renombrar y borrar (archivos y directorios), abrir un `Flujo` de escritura o anexado y crear un `Observador` |
| `todo` | `lectura` + `escritura` |

- Las rutas relativas se resuelven desde la raíz del proyecto.
- `"*"` en `ruta` actúa como comodín (cualquier ruta).
- En `copiar` y `mover` el origen necesita `lectura` y el destino
  `escritura`.
- Un `Flujo` en modo `lectura_escritura` necesita **ambos** niveles (o
  `todo`).

### 5.3 `SistemaArchivos` — funciones libres

Todas las funciones aceptan la ruta como `texto` y todas tienen su par
`_asincrono`.

| Función | Retorna | Permiso | Descripción |
|---|---|---|---|
| `leer(ruta)` | `texto` | lectura | lee el archivo como UTF-8 |
| `leer_bits(ruta)` | `Bits` | lectura | lee el archivo como binario |
| `escribir(ruta, contenido)` | `vacio` | escritura | crea o sobrescribe con `texto` |
| `escribir_bits(ruta, contenido)` | `vacio` | escritura | crea o sobrescribe con `Bits` |
| `anexar(ruta, contenido)` | `vacio` | escritura | agrega `texto` al final; crea el archivo si no existe |
| `anexar_bits(ruta, contenido)` | `vacio` | escritura | agrega `Bits` al final; crea el archivo si no existe |
| `abrir_flujo(ruta, modo)` | `Flujo` | según modo | abre el archivo con cursor persistente (§5.6) |
| `observar(ruta, recursivo)` | `Observador` | escritura | crea un observador de cambios (§5.7) |
| `crear_directorio(ruta)` | `Archivo` | escritura | crea un nivel; falla si el padre no existe o si ya existe |
| `crear_directorios(ruta)` | `Archivo` | escritura | crea la ruta completa; no falla si ya existe |
| `listar_directorios(ruta)` | `lista<texto>` | lectura | nombres de los subdirectorios (sin ruta) |
| `listar_archivos(ruta)` | `lista<texto>` | lectura | nombres de los archivos (sin ruta) |
| `existe(ruta)` | `log` | lectura | true si existe archivo o directorio |
| `es_archivo(ruta)` | `log` | lectura | true si existe y es archivo |
| `es_directorio(ruta)` | `log` | lectura | true si existe y es directorio |
| `abrir(ruta)` | `Archivo` | lectura | referencia con metadatos (no abre un flujo) |
| `borrar(ruta)` | `vacio` | escritura | borra un archivo o un directorio **vacío** |
| `borrar_recursivo(ruta)` | `vacio` | escritura | borra un directorio y todo su contenido |
| `renombrar(ruta, nueva)` | `vacio` | escritura | renombra dentro del mismo directorio |
| `copiar(origen, destino)` | `vacio` | escritura | copia archivo o directorio |
| `mover(origen, destino)` | `vacio` | escritura | mueve archivo o directorio |

> `escribir` siempre sobrescribe. Para agregar al final sin perder lo
> anterior está `anexar` (o un `Flujo` en modo `"anexar"`, §5.6).

### 5.4 `Archivo`

Se obtiene con `SistemaArchivos.abrir(ruta)` o como retorno de
`crear_directorio` / `crear_directorios`.

```quetzal
Archivo informe = SistemaArchivos.abrir("./salida/informe.txt")
consola.mostrar(t"{informe.nombre()} pesa {informe.tamaño()} bytes")
```

#### 5.4.1 Metadatos (permiso `lectura`)

| Método | Retorna | Descripción |
|---|---|---|
| `ruta()` | `texto` | ruta tal como se abrió |
| `ruta_completa()` | `texto` | ruta absoluta normalizada |
| `nombre()` | `texto` | nombre con extensión |
| `nombre_sin_extension()` | `texto` | nombre sin extensión |
| `extension()` | `texto` | extensión sin punto (vacía si no tiene) |
| `directorio_padre()` | `texto` | ruta del directorio contenedor |
| `tamaño()` | `entero` | tamaño en bytes (`0` en directorios) |
| `existe()` | `log` | true si sigue existiendo en disco |
| `es_archivo()` | `log` | true si es archivo |
| `es_directorio()` | `log` | true si es directorio |
| `es_vacio()` | `log` | archivo de 0 bytes o directorio sin entradas |
| `fecha_creacion()` | `Tiempo` | instante de creación |
| `fecha_modificacion()` | `Tiempo` | última modificación |
| `fecha_acceso()` | `Tiempo` | último acceso |

> Las fechas devuelven instancias del objeto `Tiempo`
> (ver §3), por lo que admiten `formatear`, comparación y aritmética.

#### 5.4.2 Operaciones (cada una con su par `_asincrono`)

| Método | Permiso | Descripción |
|---|---|---|
| `leer()` | lectura | contenido como `texto` (UTF-8) |
| `leer_bits()` | lectura | contenido como `Bits` |
| `escribir(contenido)` | escritura | sobrescribe con `texto` |
| `escribir_bits(contenido)` | escritura | sobrescribe con `Bits` |
| `copiar(destino)` | escritura | copia a otra ruta |
| `mover(destino)` | escritura | mueve a otra ruta |
| `renombrar(nuevo_nombre)` | escritura | renombra en el mismo directorio |
| `borrar()` | escritura | borra el archivo o el directorio vacío |
| `borrar_recursivo()` | escritura | borra el directorio con su contenido |
| `refrescar()` | lectura | relee los metadatos desde disco |
| `listar_archivos()` | lectura | `lista<texto>` con los nombres (solo directorios) |
| `listar_directorios()` | lectura | `lista<texto>` con los nombres (solo directorios) |

> `mover` y `renombrar` actualizan la ruta interna de la instancia.

### 5.5 `Bits`

Secuencia de bytes (valores `0`–`255`) para leer y escribir archivos
binarios. Es un tipo en memoria: no requiere permisos.

#### 5.5.1 Funciones libres

| Función | Retorna | Descripción |
|---|---|---|
| `Bits.vacío()` | `Bits` | secuencia de longitud 0 |
| `Bits.desde_lista(lista<entero>)` | `Bits` | cada elemento debe estar en 0–255 |
| `Bits.desde_texto(texto)` | `Bits` | codifica el texto en UTF-8 |
| `Bits.combinar(lista<Bits>)` | `Bits` | concatena varias secuencias |

#### 5.5.2 Métodos de instancia

| Método | Retorna | Descripción |
|---|---|---|
| `longitud()` | `entero` | cantidad de bytes |
| `obtener(indice)` | `entero` | byte en la posición dada |
| `establecer(indice, byte)` | `Bits` | copia con el byte reemplazado |
| `trozo(inicio, fin)` | `Bits` | subsecuencia `[inicio, fin)` |
| `concatenar(otro)` | `Bits` | copia con `otro` al final |
| `texto()` | `texto` | decodifica como UTF-8 |
| `lista()` | `lista<entero>` | bytes como lista de enteros |
| `es_igual(otro)` | `log` | compara por contenido |

> Un índice fuera de rango, un byte fuera de `0`–`255` o un contenido
> que no es UTF-8 válido en `texto()` lanzan una excepción.

### 5.6 `Flujo`

Un `Flujo` es un archivo abierto con **posición (cursor) persistente**:
mientras esté abierto recuerda en qué byte quedó, de modo que se puede
leer o escribir por partes sin recorrer el archivo completo.

```quetzal
Flujo bitacora = SistemaArchivos.abrir_flujo("./salida/bitacora.txt", "lectura_escritura")
bitacora.ir_a(0)
texto cabecera = bitacora.leer(16)
consola.mostrar(t"El cursor quedó en {bitacora.posicion()}")
bitacora.cerrar()
```

#### 5.6.1 Modos de apertura

| Modo | Permiso | Comportamiento |
|---|---|---|
| `"lectura"` | lectura | solo lee; el archivo debe existir |
| `"escritura"` | escritura | crea o trunca el archivo; solo escribe |
| `"lectura_escritura"` | lectura + escritura | crea si no existe; conserva el contenido |
| `"anexar"` | escritura | crea si no existe; toda escritura va al final |

> El cursor se mide en **bytes** desde el inicio, empieza en `0` (en
> `"anexar"`, al final) y avanza solo lo que se lee o se escribe.

#### 5.6.2 Cursor

| Método | Retorna | Descripción |
|---|---|---|
| `posicion()` | `entero` | byte actual del cursor |
| `longitud()` | `entero` | tamaño del archivo en bytes |
| `ir_a(byte)` | `entero` | mueve el cursor a una posición absoluta |
| `ir_al_inicio()` | `entero` | equivale a `ir_a(0)` |
| `ir_al_final()` | `entero` | mueve el cursor al final del archivo |
| `avanzar(bytes)` | `entero` | mueve el cursor hacia adelante |
| `retroceder(bytes)` | `entero` | mueve el cursor hacia atrás |

> Los movimientos devuelven la nueva posición. Retroceder más allá del
> inicio o usar una posición negativa lanza excepción.

#### 5.6.3 Entrada/salida (cada una con su par `_asincrono`)

| Método | Retorna | Permiso | Descripción |
|---|---|---|---|
| `leer(bytes)` | `texto` | lectura | lee hasta `bytes` bytes como UTF-8 |
| `leer_bits(bytes)` | `Bits` | lectura | lee hasta `bytes` bytes binarios |
| `leer_todo()` | `texto` | lectura | lee desde el cursor hasta el final |
| `escribir(contenido)` | `entero` | escritura | escribe `texto`; devuelve bytes escritos |
| `escribir_bits(contenido)` | `entero` | escritura | escribe `Bits`; devuelve bytes escritos |

> Al llegar al final del archivo, `leer` devuelve menos bytes de los
> pedidos (o texto vacío). Cortar un carácter UTF-8 a la mitad lanza
> excepción: para datos binarios usa `leer_bits`.

#### 5.6.4 Ciclo de vida

| Método | Retorna | Descripción |
|---|---|---|
| `cerrar()` | `vacio` | libera el archivo; es idempotente |
| `esta_cerrado()` | `log` | true si ya se cerró |
| `ruta()` | `texto` | ruta con la que se abrió |
| `modo()` | `texto` | modo de apertura |

> Operar un flujo cerrado (leer, escribir o mover el cursor) lanza una
> excepción capturable. Cierra siempre el flujo cuando termines.

### 5.7 `Observador` y `EventoArchivo`

Un `Observador` vigila una ruta (archivo o directorio) y entrega los
cambios como instancias de `EventoArchivo`. El ciclo lo controla el
desarrollador: nada queda vigilando por su cuenta.

```quetzal
Observador vigia = SistemaArchivos.observar("./salida", verdadero)
vigia.iniciar()

SistemaArchivos.escribir("./salida/nuevo.txt", "hola")

EventoArchivo evento = vigia.esperar_evento(2000)
si (evento != nulo) {
    consola.mostrar(t"{evento.tipo()} → {evento.ruta()}")
}

vigia.detener()
```

#### 5.7.1 `Observador`

| Método | Retorna | Descripción |
|---|---|---|
| `iniciar()` | `vacio` | empieza a vigilar; mientras esté activo el programa sigue vivo |
| `detener()` | `vacio` | deja de vigilar y libera el recurso; es idempotente |
| `esta_activo()` | `log` | true entre `iniciar` y `detener` |
| `ruta()` | `texto` | ruta vigilada |
| `es_recursivo()` | `log` | true si incluye subdirectorios |
| `esperar_evento(limite_ms)` | `EventoArchivo` \| `nulo` | espera el próximo evento; `nulo` si vence el límite |
| `esperar_evento_asincrono(limite_ms)` | `EventoArchivo` \| `nulo` | igual, sin bloquear el bucle de eventos |
| `eventos_pendientes()` | `entero` | eventos ya recibidos y sin consumir |

> `limite_ms` es el tiempo máximo de espera en milisegundos; `0`
> significa esperar indefinidamente. Consultar eventos (`esperar_evento`,
> `esperar_evento_asincrono`, `eventos_pendientes`) sobre un observador
> detenido lanza excepción.

#### 5.7.2 `EventoArchivo`

| Método | Retorna | Descripción |
|---|---|---|
| `tipo()` | `texto` | `"creado"`, `"modificado"`, `"borrado"` o `"renombrado"` |
| `ruta()` | `texto` | ruta afectada |
| `ruta_anterior()` | `texto` \| `nulo` | ruta previa en un renombrado |

> El segundo parámetro de `observar(ruta, recursivo)` decide si se
> vigilan también los subdirectorios.

### 5.8 Errores

Todas las fallas del módulo se reportan como **excepciones capturables**
con `intentar` / `capturar` (ver [`control-flujo.md`](./control-flujo.md) §4):

- permiso `sistema-archivos` deshabilitado en `quetzal.json`;
- ruta fuera de los directorios declarados;
- nivel insuficiente (por ejemplo, escribir con permiso `lectura`);
- errores del sistema operativo (no existe, sin espacio, sin acceso);
- contenido no UTF-8 al leer como `texto`;
- operar un `Flujo` cerrado o un `Observador` detenido;
- modo de apertura desconocido en `abrir_flujo`.

```quetzal
intentar {
    texto contenido = SistemaArchivos.leer("./secreto.txt")
    consola.mostrar(contenido)
} capturar (excepcion e) {
    consola.mostrar_error(e.mensaje)
}
```

### 5.9 Ejemplo

```quetzal
importar { SistemaArchivos, Archivo, Bits } desde "quetzal/sistema_archivos"

// Preparar el directorio de salida (recursivo, idempotente).
SistemaArchivos.crear_directorios("./salida/reportes")

// Escribir y leer texto.
SistemaArchivos.escribir("./salida/reportes/hoy.txt", "ventas: 120")
texto reporte = SistemaArchivos.leer("./salida/reportes/hoy.txt")

// Listar nombres (relativos al directorio consultado).
lista<texto> archivos = SistemaArchivos.listar_archivos("./salida/reportes")
consola.mostrar(t"Reportes: {archivos.longitud()}")

// Metadatos y operaciones desde una instancia de Archivo.
Archivo hoy = SistemaArchivos.abrir("./salida/reportes/hoy.txt")
consola.mostrar(t"{hoy.nombre()} — {hoy.tamaño()} bytes")
hoy.copiar("./salida/reportes/respaldo.txt")

// Datos binarios.
Bits cabecera = Bits.desde_lista([137, 80, 78, 71])
SistemaArchivos.escribir_bits("./salida/reportes/marca.bin", cabecera)
Bits leidos = SistemaArchivos.leer_bits("./salida/reportes/marca.bin")
consola.mostrar(t"Bytes leídos: {leidos.longitud()}")
```

```quetzal
asincrono vacio procesar() {
    texto datos = esperar SistemaArchivos.leer_asincrono("./entrada/datos.csv")
    esperar SistemaArchivos.escribir_asincrono("./salida/copia.csv", datos)
}
```

```quetzal
// Anexar a una bitácora y releer solo la última línea con un flujo.
SistemaArchivos.anexar("./salida/bitacora.log", "inicio\n")
SistemaArchivos.anexar("./salida/bitacora.log", "fin\n")

Flujo bitacora = SistemaArchivos.abrir_flujo("./salida/bitacora.log", "lectura")
bitacora.ir_a(7)
consola.mostrar(bitacora.leer_todo())
bitacora.cerrar()
```

---

## 6. `quetzal/red`

```quetzal
importar {
    ServidorHttp, Enrutador, Ruta,
    PeticionEntrante, RespuestaSaliente, Continuacion, ErrorHttp,
    ClienteHttp, RespuestaHttp, ProgresoPeticion,
    Formulario, ParteArchivo,
    HttpCodigos
} desde "quetzal/red"
```

> Provee dos mitades y dos utilidades:
>
> - **Servidor** — `ServidorHttp` (la aplicación), `Enrutador` (rutas
>   montables), `Ruta` (varios métodos sobre un mismo camino),
>   `PeticionEntrante`, `RespuestaSaliente`, `Continuacion` (el paso al
>   siguiente eslabón de la cadena) y `ErrorHttp`. El modelo es el de
>   Express 5 adaptado a Quetzal, con el término **interceptor** en lugar
>   de *middleware*.
> - **Cliente** — `ClienteHttp` (instancia con configuración por omisión,
>   al estilo de Axios), `RespuestaHttp` y `ProgresoPeticion`.
> - **`Formulario` y `ParteArchivo`** — formularios `multipart/form-data`
>   (RFC 7578) para enviar y recibir campos y archivos en la misma
>   petición (ver §6.11).
> - **`HttpCodigos`** — descripciones en español de los códigos de estado
>   según MDN.

### 6.1 Métodos HTTP en español

La API se escribe en español; por el cable siempre viajan los verbos
estándar. `consultar` es el método **QUERY** del
[RFC 10008](https://www.rfc-editor.org/rfc/rfc10008.html): seguro e
idempotente como `obtener`, pero con la consulta en el **cuerpo**.

| Español | Verbo | Seguro | Idempotente | Lleva cuerpo |
|---|---|---|---|---|
| `obtener` | `GET` | sí | sí | no |
| `publicar` | `POST` | no | no | sí |
| `poner` | `PUT` | no | sí | sí |
| `parchear` | `PATCH` | no | no | sí |
| `borrar` | `DELETE` | no | sí | sí |
| `cabecera` | `HEAD` | sí | sí | no |
| `opciones` | `OPTIONS` | sí | sí | no |
| `consultar` | `QUERY` | sí | sí | sí |
| `rastrear` | `TRACE` | sí | sí | no |
| `conectar` | `CONNECT` | no | no | no |

Todos existen tanto en el enrutador (`servidor.publicar(...)`) como en el
cliente (`cliente.publicar(...)`), y en el cliente además con la forma
`_asincrono`.

### 6.2 Permisos requeridos

El módulo exige el permiso `red` en `quetzal.json`
(ver [`manifiesto.md`](./manifiesto.md) §6):

```json
{
  "permisos": [
    {
      "tipo": "red",
      "habilitado": true
    }
  ]
}
```

Es un único interruptor: con `"habilitado": true` el programa puede tanto
conectarse (`ClienteHttp`) como escuchar puertos (`ServidorHttp.escuchar`).
Sin él, cada operación lanza una excepción capturable con un mensaje que
dice qué se intentaba hacer y cómo habilitarlo.

### 6.3 Callbacks: solo funciones nombradas

Quetzal no tiene funciones anónimas. Manejadores, interceptores y avisos
de progreso se declaran fuera y se pasan por su nombre
(ver [`funciones.md`](./funciones.md) §6):

| Rol | Firma |
|---|---|
| Manejador de ruta | `(PeticionEntrante, RespuestaSaliente)` |
| Interceptor | `(PeticionEntrante, RespuestaSaliente, Continuacion)` |
| Manejador de errores | `(ErrorHttp, PeticionEntrante, RespuestaSaliente, Continuacion)` |
| Manejador de parámetro | `(PeticionEntrante, RespuestaSaliente, Continuacion, texto)` |
| Interceptor de petición (cliente) | `(jsn var configuracion) -> jsn` |
| Interceptor de respuesta (cliente) | `(RespuestaHttp) -> RespuestaHttp` |
| Progreso | `(ProgresoPeticion)` |

Un manejador o interceptor puede ser `asincrono` y usar `esperar` dentro.

### 6.4 `ServidorHttp` y `Enrutador`

`ServidorHttp` es la aplicación; `Enrutador` es un grupo de rutas que se
monta bajo un prefijo. Ambos comparten los mismos métodos de enrutado.

| Método | Descripción |
|---|---|
| `obtener(camino, ...manejadores)` … `consultar(...)` | registra la cadena para ese método |
| `todos(camino, ...manejadores)` | cualquier método sobre ese camino |
| `usar([camino], interceptor \| Enrutador)` | interceptor global o montaje de un enrutador |
| `ruta(camino)` | devuelve una `Ruta` para encadenar métodos sobre el mismo camino |
| `parametro(nombre, manejador)` | se ejecuta cuando la ruta trae ese parámetro |
| `estaticos([camino], directorio)` | sirve archivos del directorio (requiere permiso de lectura) |

Solo en `ServidorHttp`:

| Método | Retorna | Descripción |
|---|---|---|
| `escuchar(puerto[, alArrancar])` | `ServidorHttp` | empieza a aceptar conexiones; `0` deja que el sistema elija el puerto |
| `puerto()` | `entero` | puerto real en el que quedó escuchando |
| `esta_escuchando()` | `log` | si el servidor está activo |
| `cerrar()` | `log` | deja de escuchar y libera el bucle de eventos |
| `manejar_errores(manejador)` | `ServidorHttp` | registra un manejador de errores (va al final) |
| `enrutador()` | `Enrutador` | el enrutador raíz de la aplicación |
| `configurar(clave, valor)` / `configuracion(clave)` | | ajustes propios de la aplicación |
| `habilitar(clave)` / `deshabilitar(clave)` | | ajustes booleanos |
| `esta_habilitado(clave)` / `esta_deshabilitado(clave)` | `log` | consulta de esos ajustes |

Los caminos aceptan segmentos literales, parámetros (`/usuarios/:id`),
parámetros opcionales (`/informes/:año?`) y comodín final (`/archivos/*`).

### 6.5 `PeticionEntrante`

| Método | Retorna | Descripción |
|---|---|---|
| `metodo()` | `texto` | verbo HTTP (`"QUERY"`) |
| `metodo_espanol()` | `texto` | nombre en español (`"consultar"`) |
| `url()` / `url_original()` | `texto` | camino con consulta, relativo al montaje y completo |
| `ruta()` / `ruta_base()` | `texto` | camino sin consulta y prefijo donde se montó |
| `parametros()` / `parametro(nombre)` | `jsn` / `texto` | parámetros del camino |
| `consulta()` / `consulta_valor(nombre)` | `jsn` / `texto` | cadena de consulta ya decodificada |
| `cabeceras()` / `cabecera(nombre)` | `jsn` / `texto` | cabeceras (búsqueda sin distinguir mayúsculas) |
| `galletas()` / `galleta(nombre)` | `jsn` / `texto` | cookies de la petición |
| `cuerpo()` | `jsn` \| `texto` \| `Bits` \| `Formulario` | cuerpo ya interpretado según `Content-Type` |
| `cuerpo_texto()` / `cuerpo_bits()` | `texto` / `Bits` | cuerpo crudo |
| `tipo_contenido()` | `texto` | `Content-Type` tal como llegó |
| `nombre_archivo()` | `texto`? | `filename` del `Content-Disposition` (`nulo` si no viene) |
| `tipo_es(tipo)` / `acepta(tipo)` | `log` | negociación de contenido |
| `protocolo()` / `ip()` / `anfitrion()` / `version()` | `texto` | datos de la conexión |
| `es_seguro()` / `es_idempotente()` | `log` | semántica del método (RFC 9110 y RFC 10008) |
| `locales()` | `jsn` | datos que los interceptores dejan para la cadena |

### 6.6 `RespuestaSaliente`

Todos los métodos devuelven la propia respuesta, así que se encadenan.

| Método | Descripción |
|---|---|
| `estado(codigo)` | fija el código de estado |
| `jsn(valor)` | responde JSON (`application/json`) |
| `texto(valor)` | responde texto plano |
| `enviar(valor[, opciones])` | responde adivinando el tipo (`jsn`, `texto`, `Bits`, `Archivo` o `Formulario`); las `opciones` son el jsn de §6.11 |
| `enviar_estado(codigo)` | responde solo con el código y su frase |
| `cabecera(nombre[, valor])` / `establecer(...)` | lee o fija una cabecera |
| `agregar(nombre, valor)` | añade un valor a una cabecera existente |
| `tipo(extension_o_mime)` | fija `Content-Type` |
| `ubicacion(destino)` / `redirigir([codigo,] destino)` | fija `Location` / redirige (302 por omisión) |
| `galleta(nombre, valor[, opciones])` / `borrar_galleta(nombre)` | cookies (`ruta`, `dominio`, `expira`, `edad_maxima`, `solo_http`, `segura`, `mismo_sitio`) |
| `variar(cabecera)` | añade a `Vary` |
| `local(nombre[, valor])` / `locales()` | datos por petición |
| `terminar()` / `esta_terminada()` | cierra la respuesta sin cuerpo / si ya se respondió |

### 6.7 `Continuacion` y `ErrorHttp`

| Método | Descripción |
|---|---|
| `siguiente()` | pasa al siguiente interceptor o manejador |
| `siguiente_ruta()` | abandona la ruta actual y sigue buscando |
| `siguiente_con_error(mensaje)` | salta a los manejadores de errores |
| `ErrorHttp.mensaje()` / `ErrorHttp.estado()` | qué falló y con qué código |

Si un interceptor responde y **no** llama a `siguiente()`, la cadena
termina ahí. Lo que lance un manejador llega como `ErrorHttp` al
manejador de errores; si no hay ninguno, el servidor responde `500`.

### 6.8 `ClienteHttp`

```quetzal
ClienteHttp cliente = nuevo ClienteHttp({
    base_url: "https://api.ejemplo.com",
    cabeceras: { "Accept": "application/json" },
    tiempo_limite: 5000
})
```

| Método | Descripción |
|---|---|
| `<verbo>(url[, datos[, configuracion]])` | un método por verbo: `obtener`, `publicar`, `poner`, `parchear`, `borrar`, `cabecera`, `opciones`, `consultar`, `rastrear`, `conectar` |
| `solicitar(configuracion)` | petición armada por completo desde un `jsn` |
| `<método>_asincrono(...)` | la misma petición sobre el bucle de eventos (se usa con `esperar`) |
| `configuracion()` / `configurar(clave, valor)` | valores por omisión de la instancia |
| `interceptar_peticion(funcion)` / `interceptar_respuesta(funcion)` | cadenas de interceptores |

Todos los verbos comparten la misma firma: el segundo argumento es el
**cuerpo** (`texto`, `jsn`, `lista`, `Formulario`, `Bits` o `Archivo`) y el
tercero, la **configuración** de esa petición.

```quetzal
cliente.obtener("/usuarios/42")
cliente.publicar("/usuarios", { nombre: "Ana" })
cliente.publicar("/documentos", formulario, { tiempo_limite: 10000 })
cliente.publicar("/soap", cuerpoXml, { cabeceras: { "SOAPAction": "\"...\"" } })
```

> Con un solo argumento después de la url, un `jsn` que traiga alguna clave
> de `ConfiguracionPeticion` (`cabeceras`, `parametros`, `validar_estado`,
> ...) se toma como configuración; cualquier otro valor es el cuerpo. Para
> enviar un `jsn` con esos nombres como cuerpo, pásalo en la clave `datos`.

> Al unir `base_url` con una url relativa vacía o `"/"` el resultado es la
> `base_url` tal cual: no se agrega una barra final que cambiaría el recurso.

Claves de `ConfiguracionPeticion`:

| Clave | Tipo | Descripción |
|---|---|---|
| `base_url` | `texto` | prefijo de las urls relativas |
| `parametros` | `jsn` | cadena de consulta (se codifica sola) |
| `cabeceras` | `jsn` | se combinan con las de la instancia |
| `datos` | `jsn` \| `lista` \| `texto` \| `Bits` \| `Archivo` \| `Formulario` | cuerpo; cada forma fija su `Content-Type` (ver §6.11) |
| `tiempo_limite` | `entero` | milisegundos |
| `maximo_redirecciones` | `entero` | `0` desactiva el seguimiento |
| `validar_estado` | `log` | `falso` devuelve la respuesta en vez de lanzar fuera de 2xx/3xx |
| `tipo_respuesta` | `texto` | fuerza `"jsn"`, `"texto"`, `"bits"` o `"formulario"` |
| `autenticacion` | `jsn` | `{ usuario, clave }` para HTTP Basic |
| `al_progreso_subida` / `al_progreso_descarga` | `funcion` | avisos de progreso |
| `tipo_contenido` | `texto` | fuerza el `Content-Type` de un cuerpo binario |
| `nombre_archivo` | `texto` | fuerza el `filename` del `Content-Disposition` |
| `disposicion` | `texto` | `"adjunto"` o `"inline"` |

> La forma síncrona bloquea el programa hasta recibir la respuesta. Si el
> mismo programa además atiende un `ServidorHttp`, hay que usar la forma
> `_asincrono`: es la que deja al bucle de eventos seguir trabajando.

### 6.9 `RespuestaHttp` y `ProgresoPeticion`

| `RespuestaHttp` | Retorna | Descripción |
|---|---|---|
| `estado()` / `razon()` / `descripcion()` | `entero` / `texto` / `texto` | código, frase estándar y descripción en español |
| `ok()` | `log` | si el estado está en 2xx |
| `datos()` | `jsn` \| `texto` \| `Bits` \| `Formulario` | cuerpo ya interpretado: JSON como `jsn`; texto, XML y SOAP como `texto`; multipart como `Formulario`; el resto como `Bits` |
| `cuerpo_texto()` / `bits()` | `texto` / `Bits` | cuerpo crudo |
| `cabeceras()` / `cabecera(nombre)` | `jsn` / `texto` | cabeceras de la respuesta |
| `tipo_contenido()` | `texto` | `Content-Type` de la respuesta |
| `nombre_archivo()` | `texto`? | `filename` del `Content-Disposition` (`nulo` si no viene) |
| `url()` / `metodo()` | `texto` | url final (tras redirecciones) y verbo usado |

| `ProgresoPeticion` | Retorna | Descripción |
|---|---|---|
| `direccion()` / `es_subida()` | `texto` / `log` | `"subida"` o `"descarga"` |
| `cargado()` / `total()` | `entero` / `entero?` | bytes transferidos y esperados (`nulo` si no se anuncian) |
| `bytes()` | `entero` | bytes de este aviso |
| `progreso()` / `porcentaje()` | `número?` | avance en 0..1 y en 0..100 |

### 6.10 `HttpCodigos`

| Función | Retorna | Descripción |
|---|---|---|
| `descripcion(codigo)` | `texto` | descripción en español (MDN) |
| `razon(codigo)` | `texto` | frase de razón estándar (`"Not Found"`) |
| `categoria(codigo)` | `texto` | `informativo`, `exitoso`, `redirección`, `error del cliente`, `error del servidor` |
| `existe(codigo)` | `log` | si el código está en la tabla |
| `es_informativo` / `es_exitoso` / `es_redireccion` / `es_error_cliente` / `es_error_servidor` / `es_error` | `log` | clasificación por rango |
| `todos()` | `lista<jsn>` | tabla completa (`{codigo, razon, descripcion}`) |

Constantes: `OK`, `CREADO`, `ACEPTADO`, `SIN_CONTENIDO`,
`MOVIDO_PERMANENTEMENTE`, `NO_MODIFICADO`, `PETICION_INCORRECTA`,
`NO_AUTENTICADO`, `PROHIBIDO`, `NO_ENCONTRADO`, `METODO_NO_PERMITIDO`,
`CONFLICTO`, `CONTENIDO_NO_PROCESABLE`, `DEMASIADAS_PETICIONES`,
`ERROR_INTERNO`, `SERVICIO_NO_DISPONIBLE`, entre otras.

### 6.11 Archivos y datos binarios

Los archivos viajan por los **mismos verbos** (`publicar`, `poner`, … y
sus formas `_asincrono`): no hay funciones aparte para subir o bajar. Lo
que cambia es el valor que se pasa como cuerpo.

| Cuerpo | Qué viaja por el cable |
|---|---|
| `Bits` | bytes con `Content-Type: application/octet-stream` |
| `Archivo` | los bytes del archivo, MIME por extensión y `Content-Disposition` con su nombre |
| `Formulario` | `multipart/form-data; boundary=…` (RFC 7578) |
| `jsn` / `lista` / `texto` | JSON o texto, como siempre |

Un `Archivo` se lee a memoria antes de transmitirse, así que además del
permiso `red` hace falta `sistema-archivos` de lectura sobre esa ruta.

Las opciones del cuerpo binario van en el **jsn de configuración** del
cliente (`tipo_contenido`, `nombre_archivo`, `disposicion`) o en el jsn
opcional de `RespuestaSaliente.enviar`:

```quetzal
Bits marca = Bits.desde_lista([137, 80, 78, 71])
esperar cliente.publicar_asincrono("/documentos", marca, {
    nombre_archivo: "marca.png",
    tipo_contenido: "image/png"
})
```

Del otro lado, el cuerpo binario se lee con `peticion.cuerpo_bits()` (o
`respuesta.bits()` en el cliente) y sus metadatos con `tipo_contenido()`
y `nombre_archivo()`.

#### 6.11.1 `Formulario` y `ParteArchivo`

`Formulario` es la única forma de armar o leer un multipart: no existe
una versión con `jsn`. Sirve para las dos direcciones —se construye en el
cliente y llega ya analizado en `peticion.cuerpo()` del servidor.

| Método de `Formulario` | Retorna | Descripción |
|---|---|---|
| `campo(nombre, valor)` | `Formulario` | agrega un campo de texto |
| `archivo(nombre, Archivo \| Bits[, opciones])` | `Formulario` | agrega un archivo; opciones: `nombre`, `tipo` |
| `campo_texto(nombre)` | `texto`? | valor de un campo (`nulo` si falta) |
| `tiene(nombre)` | `log` | si existe un campo o archivo con ese nombre |
| `campos()` | `jsn` | todos los campos de texto |
| `archivo_parte(nombre)` | `ParteArchivo`? | archivo de ese campo |
| `archivos()` | `lista<ParteArchivo>` | todos los archivos |

| Método de `ParteArchivo` | Retorna | Descripción |
|---|---|---|
| `campo()` | `texto` | nombre del campo del formulario |
| `nombre()` | `texto` | nombre del archivo (`filename`) |
| `tipo()` | `texto` | `Content-Type` de esa parte |
| `bits()` | `Bits` | contenido |

```quetzal
// Cliente: armar y enviar.
Formulario expediente = nuevo Formulario()
expediente.campo("titulo", "informe trimestral")
expediente.archivo("documento", SistemaArchivos.abrir("./informe.txt"))
expediente.archivo("miniatura", miniatura, { nombre: "mini.png", tipo: "image/png" })

RespuestaHttp creado = esperar cliente.publicar_asincrono("/expedientes", expediente)

// Servidor: leer.
vacio recibir(PeticionEntrante peticion, RespuestaSaliente respuesta) {
    Formulario formulario = peticion.cuerpo()
    texto titulo = formulario.campo_texto("titulo")
    ParteArchivo documento = formulario.archivo_parte("documento")
    respuesta.jsn({ titulo: titulo, bytes: documento.bits().longitud() })
}
```

Por el cable solo hay HTTP estándar: el multipart que produce el cliente
lo entienden `curl`, `fetch` o `multer`, y el servidor acepta el que
manden ellos.

Ejemplos completos: [`ejemplos/red_archivos_binarios`](../ejemplos/red_archivos_binarios/principal.qz)
y [`ejemplos/red_formulario_multipart`](../ejemplos/red_formulario_multipart/principal.qz).

### 6.12 Errores

Como en el resto de módulos nativos, todo se reporta como excepción
capturable: permiso `red` ausente, puerto ocupado, url inválida, host
inalcanzable, tiempo agotado y estado fuera de rango cuando
`validar_estado` no está en `falso`. Mandar un `Archivo` sin permiso de
`sistema-archivos` también lanza, aunque la red esté habilitada.

### 6.13 Ejemplo

```quetzal
importar {
    ServidorHttp, Enrutador, PeticionEntrante, RespuestaSaliente,
    Continuacion, ErrorHttp, HttpCodigos
} desde "quetzal/red"

vacio bitacora(PeticionEntrante peticion, RespuestaSaliente respuesta, Continuacion siguiente) {
    consola.mostrar(t"{peticion.metodo_espanol()} {peticion.ruta()}")
    siguiente.siguiente()
}

vacio verUsuario(PeticionEntrante peticion, RespuestaSaliente respuesta) {
    respuesta.jsn({ id: peticion.parametro("id") })
}

// QUERY: la consulta viaja en el cuerpo.
vacio buscar(PeticionEntrante peticion, RespuestaSaliente respuesta) {
    jsn criterio = peticion.cuerpo()
    respuesta.jsn({ termino: criterio.termino, resultados: ["Ana"] })
}

vacio atenderError(ErrorHttp fallo, PeticionEntrante peticion, RespuestaSaliente respuesta, Continuacion siguiente) {
    respuesta.estado(HttpCodigos.ERROR_INTERNO).jsn({ error: fallo.mensaje() })
}

ServidorHttp servidor = nuevo ServidorHttp()
Enrutador api = nuevo Enrutador()

api.usar(bitacora)
api.obtener("/usuarios/:id", verUsuario)
api.consultar("/usuarios", buscar)

servidor.usar("/api", api)
servidor.manejar_errores(atenderError)
servidor.escuchar(3000)
```

```quetzal
importar { ClienteHttp, RespuestaHttp, ProgresoPeticion } desde "quetzal/red"

vacio alDescargar(ProgresoPeticion progreso) {
    consola.mostrar(t"{progreso.porcentaje()}% ({progreso.cargado()} bytes)")
}

ClienteHttp cliente = nuevo ClienteHttp({ base_url: "http://127.0.0.1:3000" })

RespuestaHttp usuario = esperar cliente.obtener_asincrono("/api/usuarios/42", {
    al_progreso_descarga: alDescargar
})
consola.mostrar(t"{usuario.estado()}: {usuario.datos()}")
```
