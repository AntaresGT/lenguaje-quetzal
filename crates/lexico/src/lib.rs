//! Analizador léxico del Lenguaje Quetzal.
//!
//! Convierte código fuente `.qz` en tokens con ubicación: palabras
//! reservadas en español (con y sin tilde), identificadores Unicode,
//! números, textos (incluidos los interpolados `t"Hola {nombre}"`),
//! operadores, delimitadores y comentarios.

#![forbid(unsafe_code)]

pub mod lexer;
pub mod normalizacion;
pub mod palabras_reservadas;
pub mod token;

pub use lexer::{ResultadoTokens, resolver_escapes, tokenizar, tokenizar_para_editor};
pub use token::{TipoToken, Token};
