# motor

Motor del Lenguaje Quetzal. Orquesta el pipeline completo (léxico → sintaxis → semántica → bytecode → máquina virtual) y expone la API pública embebible para usar Quetzal desde otro programa Rust:

```rust
use motor::{ConfiguracionMotor, MotorQuetzal};

let motor = MotorQuetzal::nuevo(ConfiguracionMotor::por_defecto())?;
motor.ejecutar_texto(r#"consola.mostrar("Hola desde Quetzal embebido")"#)?;
```

La CLI (`lnc`) usa exactamente esta misma API; no existe lógica de ejecución duplicada.
