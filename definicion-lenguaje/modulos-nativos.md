# Módulos nativos (biblioteca estándar)

> Módulos que forman parte de la biblioteca estándar de Lenguaje
> Quetzal. Se importan desde rutas con el prefijo `quetzal/`.

## Índice

1. [`consola`](#1-consola) — entrada/salida (no requiere import)
2. [`quetzal/matemática`](#2-quetzalmatemática) — funciones matemáticas
3. [`quetzal/tiempo`](#3-quetzaltiempo) — fechas, horas, zonas
4. [`quetzal/motor`](#4-quetzalmotor) — expresiones regulares
5. [`quetzal/sistema_archivos`](#5-quetzalsistema_archivos) — archivos, directorios, datos binarios, flujos y observadores

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
