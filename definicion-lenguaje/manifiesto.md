# Manifiesto de proyecto (`quetzal.json`)

> Formato del archivo `quetzal.json` que describe un proyecto Quetzal:
> metadatos, dependencias, permisos y punto de entrada.

## 1. Ubicación y formato

- **Nombre del archivo**: `quetzal.json` (en la raíz del proyecto).
- **Formato**: JSON (no JSON5 — se observa JSON estricto en los ejemplos).
- **Esquema de validación**: [`esquema_quetzal.json`](./esquema_quetzal.json) (ver §7).
- **Convención de nombres**: los campos del manifiesto usan **tildes**
  (`versión`, `aplicación`, enums `aplicación` / `biblioteca`,
  `sistema-archivos`, `ejecución`, `escritura`, etc.). Esta es la forma
  canónica exigida por el esquema oficial.

## 2. Ejemplo mínimo

```json
{
  "versión": "0.1.0",
  "aplicación": "hola_mundo",
  "entrada": "principal.qz",
  "tipo": "aplicación",
  "quetzal": "0.2.0"
}
```

## 3. Campos

| Campo | Tipo | Requerido | Descripción |
|---|---|---|---|
| `versión` | string (semver) | **Sí** | Versión del formato del manifiesto |
| `aplicación` | string | **Sí** | Nombre del proyecto o aplicación |
| `entrada` | string | **Sí** | Archivo de entrada (ej: `principal.qz`) |
| `tipo` | enum | **Sí** | `aplicación` o `biblioteca` (default: `aplicación`) |
| `quetzal` | string (semver) | **Sí** | Versión del lenguaje requerida |
| `autor` | string | No | Autor del proyecto |
| `email` | string (email) | No | Email de contacto |
| `sitioweb` | string (URL) | No | Sitio web del autor |
| `repositorio` | string (URL) | No | URL del repositorio |
| `licencia` | string | No | SPDX de la licencia (`MIT`, `GPL-3.0`, etc.) |
| `descripcion` | string | No | Descripción corta del proyecto |
| `palabras_clave` | array<string> | No | Keywords del proyecto |
| `dependencias` | object<string, string> | No | Mapa de nombre → versión o URL |
| `permisos` | array<object> | No | Lista de permisos solicitados |

> **Nota**: el campo `aplicación` se llama así en el esquema canónico
> (con tilde). El campo opcional `descripcion` está sin tilde
> (convención propia del esquema).

## 4. Tipos de proyecto

```json
{ "tipo": "aplicación" }     // app ejecutable
{ "tipo": "biblioteca" }     // librería importable
```

## 5. Dependencias

```json
{
  "dependencias": {
    "mi_libreria": "1.0.0",
    "otra_lib":    "https://git.example.com/repo.git"
  }
}
```

- Las versiones siguen **semver** (`MAJOR.MINOR.PATCH`).
- Alternativamente se puede usar una **URL git** como valor.

## 6. Permisos

```json
{
  "permisos": [
    {
      "tipo": "red",
      "habilitado": true
    },
    {
      "tipo": "sistema-archivos",
      "habilitado": true,
      "directorios": [
        { "ruta": "./", "permiso": "lectura" },
        { "ruta": "./salida", "permiso": "escritura" },
        { "ruta": "/tmp", "permiso": "todo" }
      ]
    },
    {
      "tipo": "ejecución",
      "habilitado": true,
      "ejecutables": ["git", "python3", "*"]
    }
  ]
}
```

### 6.1 Tipos de permiso

| `tipo` | Descripción |
|---|---|
| `red` | acceso a la red (HTTP, sockets) |
| `sistema-archivos` | acceso al filesystem |
| `ejecución` | ejecutar procesos externos |

> **Regla condicional** (definida en el esquema con `allOf`):
> - Si `tipo == "sistema-archivos"` → `directorios` es **requerido**.
> - Si `tipo == "ejecución"` → `ejecutables` es **requerido**.

### 6.2 Permisos de filesystem

| `permiso` | Descripción |
|---|---|
| `lectura` | listar directorios y archivos, leer contenido, comprobar existencia y consultar metadatos |
| `escritura` | crear directorios, escribir y editar archivos, copiar, mover, renombrar y borrar |
| `todo` | lectura + escritura |

> El borrado y el renombrado van incluidos en `escritura`: no existe un
> nivel aparte para ellos.

> El valor `"*"` en `ruta` o `ejecutables` actúa como **comodín**
> (permite todos).

Los objetos que consumen estos permisos (`SistemaArchivos`, `Archivo` y
`Bits`) se documentan en
[`modulos-nativos.md`](./modulos-nativos.md) §5.

## 7. JSON Schema oficial (canónico)

El archivo [`esquema_quetzal.json`](./esquema_quetzal.json) define
formalmente el formato. Es la **fuente de verdad** para validar
cualquier `quetzal.json`.

Resumen de su estructura:

| Sección | Contenido |
|---|---|
| Top-level | `$schema`, `$id`, `title`, `description` (Draft-07) |
| `required` | `versión`, `aplicación`, `entrada`, `tipo`, `quetzal` |
| `properties` | 15 campos documentados con `type`, `description`, `pattern`, `examples` |
| `permisos` | Array con `allOf` para reglas condicionales (si `tipo == "sistema-archivos"`, entonces `directorios` requerido, etc.) |

### 7.1 Bugs conocidos en el esquema actual

El esquema en `esquema_quetzal.json` contiene los siguientes
problemas menores (ver `anomalias.md` §2 para detalles):

1. **`ejecutables`**: usa `"array": {...}` en lugar de
   `"type": "array", "items": {...}`. Falta el campo `type`.
2. **`entrada`**: el `pattern` no permite `/` ni `.`, pero el ejemplo
   incluye `aplicación/principal.qz`.
3. **`descripcion` y `palabras_clave`**: el `pattern` no permite
   espacios, lo que hace imposible escribir descripciones reales.
4. **`permisos[].directorios[].permiso`**: usa `"enum": {...}` en
   lugar de `"type": "string", "enum": [...]`. Falta el campo `type`.

## 8. Ejemplo completo

```json
{
  "versión": "1.0.0",
  "aplicación": "mi-proyecto-quetzal",
  "entrada": "aplicación/principal.qz",
  "tipo": "aplicación",
  "quetzal": "0.2.0",
  "autor": "Ana López",
  "email": "ana@example.com",
  "sitioweb": "https://mi-proyecto.example.com",
  "repositorio": "https://github.com/usuario/mi-proyecto",
  "licencia": "MIT",
  "descripcion": "Una aplicación de demostración en Quetzal",
  "palabras_clave": ["demo", "quetzal", "español"],
  "dependencias": {
    "lib_http": "1.2.0"
  },
  "permisos": [
    { "tipo": "red", "habilitado": true },
    {
      "tipo": "sistema-archivos",
      "habilitado": true,
      "directorios": [
        { "ruta": "./", "permiso": "lectura" }
      ]
    }
  ]
}
```
