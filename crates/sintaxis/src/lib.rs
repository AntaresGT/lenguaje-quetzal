//! Parser del Lenguaje Quetzal: convierte tokens en AST.
//!
//! Parser recursivo descendente con precedencia de operadores. Produce el
//! AST definido en el crate `ast` y reporta errores sintácticos como
//! `ErrorQuetzal` con ubicación exacta.

#![forbid(unsafe_code)]

mod declaraciones;
mod editor;
mod errores;
mod expresiones;
mod modulos;
mod objetos;
mod parser;
mod sentencias;

pub use editor::{ResultadoParseo, parsear_modulo_para_editor};
pub use parser::{Parser, parsear_modulo};
