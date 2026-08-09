# pruebas

Pruebas del Lenguaje Quetzal organizadas por etapa del pipeline:

- `lexico/`: tokenización de código fuente.
- `sintaxis/`: parser y AST.
- `semantica/`: tipos, mutabilidad y símbolos.
- `bytecode/`: generación de instrucciones.
- `runtime/`: ejecución en la máquina virtual y módulos nativos.
- `paquetes/`: quetzal.json, lock e instalación.

Cada subdirectorio es un paquete de pruebas del workspace (solo contiene tests de integración que ejercitan los crates de `crates/`). Se ejecutan con `cargo test`.
