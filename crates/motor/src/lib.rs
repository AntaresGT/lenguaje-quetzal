//! Motor del Lenguaje Quetzal: orquesta el pipeline completo y expone la
//! API pública embebible para usar Quetzal desde otro programa Rust.

#![forbid(unsafe_code)]

mod cargador;
pub mod configuracion;
pub mod motor;
pub mod sesion;

pub use configuracion::ConfiguracionMotor;
pub use motor::{MotorQuetzal, interrumpir_ejecucion, reiniciar_interrupcion};
pub use nucleo::{ErrorQuetzal, ResultadoMultiple, ResultadoQuetzal, VERSION_QUETZAL};
pub use sesion::SesionInteractiva;
