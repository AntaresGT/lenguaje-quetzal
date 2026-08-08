# Módulos

> Sistema de módulos del Lenguaje Quetzal: `importar`, `exportar`,
> alias, rutas relativas y módulos nativos.

## 1. `importar`

### 1.1 Sintaxis

```ebnf
importar = "importar" , "{" , lista_importaciones , "}" , "desde" , CADENA ;
```

### 1.2 Import básico

```quetzal
importar { sumar, restar } desde "./aplicacion/calculadora.qz"
```

### 1.3 Import con alias

```quetzal
importar {
    sumar,
    Usuario,
    instancia_usuario,
    saludo como texto_saludo       // alias con "como"
} desde "exportar.qz"
```

### 1.4 Múltiples imports

```quetzal
importar { sumar, restar } desde "./calculadora.qz"
importar { ExpresiónRegular } desde "quetzal/motor"
importar { Matemática } desde "quetzal/matemática"
importar { Tiempo } desde "quetzal/tiempo"
```

## 2. `exportar`

### 2.1 Sintaxis

```ebnf
exportar = "exportar" , "{" , lista_identificadores , "}" ;
```

### 2.2 Ejemplo

```quetzal
exportar {
    sumar_entero,           // función
    Usuario,                // definición de objeto
    instancia_usuario,      // instancia
    saludo                  // variable
}
```

Se puede exportar:
- Funciones
- Definiciones de `objeto`
- Definiciones de `prototipo`
- Instancias
- Variables (inmutables por defecto, como siempre)

## 3. Rutas de import

### 3.1 Rutas relativas

```quetzal
importar { X } desde "./mismo_directorio/modulo.qz"
importar { Y } desde "../padre/otro.qz"
importar { Z } desde "./subdir/otro.qz"
```

### 3.2 Rutas a módulos nativos

Los módulos nativos viven bajo el prefijo `quetzal/`:

```quetzal
importar { ExpresiónRegular } desde "quetzal/motor"
importar { Matemática }       desde "quetzal/matemática"
importar { Tiempo }           desde "quetzal/tiempo"
```

> **Nota**: las tildes en los nombres de módulos nativos son válidas
> (`quetzal/matemática` y `quetzal/tiempo` se observan ambos, con y sin
> tilde, en distintos archivos).

## 4. Manifiesto del módulo

La estructura de un proyecto multi-archivo viene dada por el archivo
`quetzal.json` (ver `manifiesto.md`):

```json
{
  "version": "0.1.0",
  "aplicacion": "mi_proyecto",
  "entrada": "principal.qz",
  "tipo": "aplicacion",
  "quetzal": "0.2.0"
}
```

> Los `quetzal.json` reales usan campos **sin tildes** (`version`,
> `aplicacion`, `tipo`). El JSON Schema oficial usa campos **con
> tildes** (`versión`, `aplicación`). Ver `anomalias.md`.

## 5. Formas de import NO soportadas (no se observan)

| Forma | Soporte |
|---|---|
| `importar * desde "modulo"` | No observado |
| `importar X desde "modulo"` (sin llaves) | No observado |
| Import default | No observado |
| Import dinámico (`importar(...)` en runtime) | No observado |
| Re-exports (`exportar { X } desde "..."`) | No observado |

## 6. Resumen de keywords

| Keyword | Equivalente inglés |
|---|---|
| `importar` | `import` |
| `exportar` | `export` |
| `desde` | `from` |
| `como` | `as` (alias) |
