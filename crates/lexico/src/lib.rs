//! Analizador léxico del Lenguaje Quetzal.
//!
//! Convierte código fuente `.qz` en tokens con ubicación: palabras
//! reservadas en español (con y sin tilde), identificadores Unicode,
//! números, textos (incluidos los interpolados `t"Hola {nombre}"`),
//! operadores, delimitadores y comentarios.

pub mod lexer;
pub mod normalizacion;
pub mod palabras_reservadas;
pub mod token;

pub use lexer::{resolver_escapes, tokenizar};
pub use token::{TipoToken, Token};
