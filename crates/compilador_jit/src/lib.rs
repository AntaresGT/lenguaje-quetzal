//! Compilación JIT aislada. La VM sigue siendo segura y opcionalmente invoca
//! las interfaces de este crate; el código Quetzal nunca aporta direcciones.

#![cfg_attr(not(feature = "nativo"), forbid(unsafe_code))]
#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(feature = "nativo")]
mod analisis;
mod compilados;
mod configuracion;
mod contexto;
mod enlaces;
mod estadisticas;
#[cfg(feature = "nativo")]
mod generador;
#[cfg(feature = "nativo")]
mod memoria;
#[cfg(feature = "nativo")]
mod traductor;

pub use compilados::FuncionNativa;
pub use configuracion::ConfiguracionJit;
pub use contexto::ContextoJit;
pub use enlaces::{
    ESTADO_DESBORDAMIENTO, ESTADO_DIVISION_CERO, ESTADO_EXITO, OPERACION_AJUSTE,
    OPERACION_DIVISION, OPERACION_MODULO, OPERACION_MULTIPLICACION, OPERACION_NEGACION,
    OPERACION_NINGUNA, OPERACION_RESTA, OPERACION_SUMA, SalidaEntera,
};
pub use estadisticas::EstadisticasJit;

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ErrorJit {
    #[error("el JIT nativo no está disponible en esta compilación o plataforma")]
    NoDisponible,
    #[error("la función nativa esperaba {esperados} argumentos y recibió {recibidos}")]
    Aridad { esperados: usize, recibidos: usize },
    #[error("se alcanzó el límite de recursos del JIT: {0}")]
    Limite(&'static str),
    #[error("bytecode inválido para el JIT: {0}")]
    Bytecode(String),
    #[error("la función no es elegible para el Baseline JIT: {0}")]
    NoElegible(String),
    #[error("la función nativa tiene otra signatura ABI")]
    FirmaIncorrecta,
    #[error("no se pudo generar código nativo: {0}")]
    Compilacion(String),
}
