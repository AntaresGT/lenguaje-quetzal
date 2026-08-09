//! Tipos base compartidos por todo el Lenguaje Quetzal.
//!
//! Este crate define la representación del código fuente, las ubicaciones
//! dentro de él y el error central del lenguaje. No depende de ningún otro
//! crate del workspace.

pub mod errores;
pub mod fuente;
pub mod resultado;
pub mod ubicacion;

pub use errores::{CategoriaError, ErrorQuetzal};
pub use fuente::Fuente;
pub use resultado::{ResultadoMultiple, ResultadoQuetzal};
pub use ubicacion::{Posicion, Ubicacion};

/// Versión actual del Lenguaje Quetzal.
pub const VERSION_QUETZAL: &str = env!("CARGO_PKG_VERSION");
