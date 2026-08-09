# lnc

CLI del Lenguaje Quetzal. Produce el binario `quetzal` con los comandos:

- `quetzal ejecutar [ARCHIVO]` / `quetzal archivo.qz`: ejecuta un archivo o proyecto.
- `quetzal revisar [ARCHIVO]`: analiza el código sin ejecutarlo.
- `quetzal nuevo NOMBRE`: crea un proyecto nuevo.
- `quetzal instalar [PAQUETE]`: instala dependencias.
- `quetzal version` / `--version` / `--versión`: muestra la versión.
- `quetzal` (sin argumentos): inicia el REPL.

Este crate solo parsea argumentos y delega en `motor`, `repl` y `paquetes`; no contiene lógica del lenguaje.
