//! Máquina virtual de pila del Lenguaje Quetzal.
//!
//! Ejecuta el bytecode generado por el crate `bytecode` con pila única de
//! valores, marcos livianos, ranuras locales, manejo de excepciones y
//! errores de runtime con ubicación.

#![forbid(unsafe_code)]

pub mod bucle_eventos;
pub mod codigo;
pub mod errores;
pub mod nativos;
pub mod valores;
pub mod vm;

pub use bucle_eventos::{BucleEventos, CargaNativa, ManijaBucle, Mensaje};
pub use errores::{DatosExcepcion, Fallo};
pub use nativos::{
    FuncionNativa, FuncionNativaConVm, IdNativo, RegistroNativos, normalizar_nombre,
};
pub use valores::{
    DatosFuncion, DatosInstanciaNativa, DatosMetodoEnlazado, DatosPadre, EstadoTareaNativa, Valor,
    Variable, carga_a_valor, texto_de_valor, valor_a_carga,
};
pub use vm::Vm;
