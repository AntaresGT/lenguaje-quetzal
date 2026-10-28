//! Compilación JIT aislada. La VM sigue siendo segura y opcionalmente invoca
//! las interfaces de este crate; el código Quetzal nunca aporta direcciones.

#![cfg_attr(not(feature = "nativo"), forbid(unsafe_code))]
#![deny(unsafe_op_in_unsafe_fn)]

mod compilados;
mod configuracion;
mod contexto;
mod estadisticas;
#[cfg(feature = "nativo")]
mod generador;
#[cfg(feature = "nativo")]
mod memoria;

pub use compilados::FuncionNativa;
pub use configuracion::ConfiguracionJit;
pub use contexto::ContextoJit;
pub use estadisticas::EstadisticasJit;

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ErrorJit {
    #[error("el JIT nativo no está disponible en esta compilación o plataforma")]
    NoDisponible,
    #[error("la función nativa esperaba {esperados} argumentos y recibió {recibidos}")]
    Aridad { esperados: usize, recibidos: usize },
    #[error("se alcanzó el límite de recursos del JIT: {0}")]
    Limite(&'static str),
    #[error("no se pudo generar código nativo: {0}")]
    Compilacion(String),
}
