//! Gestión de documentos abiertos por el editor.

mod almacen;
mod documento;
mod posiciones;

pub use almacen::AlmacenDocumentos;
pub use documento::DocumentoAbierto;
pub use posiciones::ConvertidorPosiciones;
