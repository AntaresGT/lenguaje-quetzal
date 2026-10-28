//! Reportes de errores con estilo parecido a Rust para Lenguaje Quetzal.

#![forbid(unsafe_code)]

pub mod codigos;
pub mod colores;
pub mod reportes;

pub use reportes::{reportar, reportar_varios};
