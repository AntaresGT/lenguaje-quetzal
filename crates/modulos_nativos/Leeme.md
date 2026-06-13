# modulos_nativos

Módulos nativos del Lenguaje Quetzal escritos en Rust:

- `consola`: global, no necesita importarse.
- `matematica` (`quetzal/matematica` o `quetzal/matemática`): constantes y funciones matemáticas.
- Métodos de los tipos `texto`, `lista` y `jsn` (incluidas las conversiones `.texto()`, `.entero()`, ...).
- `tiempo`: expone el objeto `Tiempo` (instanciable con `nuevo Tiempo(...)`) con fechas, horas, aritmética, comparación y zonas horarias IANA.
- `red` y `sistema_archivos`: respetan el modelo de permisos de `runtime`.

Expone un registro que el `motor` instala en la máquina virtual.
