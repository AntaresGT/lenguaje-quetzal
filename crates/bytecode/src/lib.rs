//! Bytecode del Lenguaje Quetzal: instrucciones y generador desde el AST.

#![forbid(unsafe_code)]

pub mod constantes;
pub mod generador;
pub mod instruccion;
pub mod modulo;
pub mod optimizar;
pub mod verificador;

pub use constantes::Constante;
pub use generador::generar_modulo;
pub use instruccion::{CodigoOp, Instruccion, Trozo};
pub use modulo::{
    AtributoCompilado, FuncionCompilada, ImportacionCompilada, MetodoCompilado, ModuloCompilado,
    ObjetoCompilado,
};
pub use optimizar::optimizar_modulo;
pub use verificador::{verificar_funcion_inmutable, verificar_modulo, verificar_modulo_inmutable};
