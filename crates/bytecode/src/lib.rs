//! Bytecode del Lenguaje Quetzal: instrucciones y generador desde el AST.

pub mod constantes;
pub mod generador;
pub mod instruccion;
pub mod modulo;
pub mod optimizar;

pub use constantes::Constante;
pub use generador::generar_modulo;
pub use instruccion::{Instruccion, Trozo};
pub use modulo::{
    AtributoCompilado, FuncionCompilada, ImportacionCompilada, MetodoCompilado, ModuloCompilado,
    ObjetoCompilado,
};
pub use optimizar::optimizar_modulo;
