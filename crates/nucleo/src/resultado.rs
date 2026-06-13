//! Alias de resultado usado en todo el lenguaje.

use crate::errores::ErrorQuetzal;

/// Resultado estándar de las operaciones del lenguaje.
pub type ResultadoQuetzal<T> = Result<T, ErrorQuetzal>;

/// Resultado de una etapa que puede acumular varios errores (por ejemplo el
/// análisis semántico reporta todos los errores antes de detenerse).
pub type ResultadoMultiple<T> = Result<T, Vec<ErrorQuetzal>>;
