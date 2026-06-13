//! Árbol de sintaxis abstracta del Lenguaje Quetzal.
//!
//! Solo define estructuras de datos; el parser vive en `sintaxis` y el
//! consumo del AST en `semantica` y `bytecode`. Cada nodo guarda su
//! [`nucleo::Ubicacion`] para diagnósticos precisos.

pub mod declaracion;
pub mod expresion;
pub mod modulo;
pub mod objeto;
pub mod prototipo;
pub mod sentencia;
pub mod tipos;

pub use declaracion::{Funcion, Parametro};
pub use expresion::{
    ClaveJsn, Expresion, NodoExpresion, OperadorBinario, OperadorUnario, SegmentoInterpolado,
};
pub use modulo::{Elemento, Importacion, Modulo, SimboloImportado};
pub use objeto::{ClaseMiembro, DefinicionObjeto, MiembroObjeto, Visibilidad};
pub use prototipo::{DefinicionPrototipo, FirmaMiembro, MiembroPrototipo};
pub use sentencia::{Bloque, Captura, NodoSentencia, OperadorAsignacion, Sentencia};
pub use tipos::Tipo;
