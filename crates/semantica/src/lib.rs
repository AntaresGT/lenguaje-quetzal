//! Análisis semántico del Lenguaje Quetzal.
//!
//! Valida sobre el AST: símbolos no definidos, duplicados, reasignación de
//! constantes, tipos incompatibles, retornos, visibilidad y cumplimiento de
//! prototipos. Acumula todos los errores antes de reportar.

pub mod analizador;
pub mod mutabilidad;
pub mod permisos;
pub mod resolucion_imports;
pub mod tabla_simbolos;
pub mod tipos;
pub mod visibilidad;

pub use analizador::analizar_modulo;
pub use tipos::TipoSemantico;
