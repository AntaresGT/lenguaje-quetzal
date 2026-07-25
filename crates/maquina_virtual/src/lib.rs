//! Máquina virtual de pila del Lenguaje Quetzal.
//!
//! Ejecuta el bytecode generado por el crate `bytecode` con pila de valores,
//! marcos de ejecución, ámbitos, manejo de excepciones y errores de runtime
//! con ubicación. Los enteros usan operaciones verificadas (overflow es un
//! error claro) y el tipo `número` usa decimales exactos
//! (`0.1 + 0.2 == 0.3`).

pub mod bucle_eventos;
pub mod errores;
pub mod nativos;
pub mod valores;
pub mod vm;

pub use bucle_eventos::{BucleEventos, CargaNativa, ManijaBucle, Mensaje};
pub use errores::{DatosExcepcion, Fallo};
pub use nativos::{FuncionNativa, FuncionNativaConVm, RegistroNativos, normalizar_nombre};
pub use valores::{
    DatosInstanciaNativa, EstadoTareaNativa, Valor, Variable, carga_a_valor, texto_de_valor,
    valor_a_carga,
};
pub use vm::Vm;
