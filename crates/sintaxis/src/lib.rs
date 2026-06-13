//! Parser del Lenguaje Quetzal: convierte tokens en AST.
//!
//! Parser recursivo descendente con precedencia de operadores. Produce el
//! AST definido en el crate `ast` y reporta errores sintácticos como
//! `ErrorQuetzal` con ubicación exacta.

mod declaraciones;
mod errores;
mod expresiones;
mod modulos;
mod objetos;
mod parser;
mod sentencias;

pub use parser::{Parser, parsear_modulo};
