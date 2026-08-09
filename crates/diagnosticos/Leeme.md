# diagnosticos

Convierte un `ErrorQuetzal` (definido en `nucleo`) en un reporte legible con estilo parecido a Rust:

```text
error[E0007]: no puedes reasignar una variable constante
  --> aplicacion/principal.qz:3:1
   |
 3 | edad = 31
   | ^^^^ esta variable fue declarada como constante
   |
ayuda: declara la variable como mutable usando 'var'
```

Incluye soporte de colores (rojo para errores, cian para ayuda) que puede desactivarse para pruebas o terminales sin color.
