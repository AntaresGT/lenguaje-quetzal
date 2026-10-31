# lsp

Servidor de lenguaje (Language Server Protocol) oficial del Lenguaje Quetzal. Analiza código sin ejecutar programas ni depender de la máquina virtual o del JIT.

## Ejecución

```bash
quetzal-lsp --stdio
```

La ejecución sin argumentos también usa `stdio`. La salida estándar contiene exclusivamente mensajes LSP; los errores de arranque se escriben en la salida estándar de errores.

## Estado

Implementado: inicialización, negociación de UTF-16, respuesta a métodos desconocidos, `shutdown`, `exit`, gestión de documentos abiertos, conversión UTF-8 ↔ UTF-16 y diagnósticos léxicos, sintácticos y semánticos reales. Los cambios sin guardar permanecen en memoria; las versiones anteriores se descartan.

Pendiente: autocompletado, hover y navegación. El servidor no anuncia capacidades que todavía no implementa.
