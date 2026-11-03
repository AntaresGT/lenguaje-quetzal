//! Representación en memoria de un documento abierto.

/// Versión actual del texto que mantiene el editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentoAbierto {
    pub uri: String,
    pub version: i32,
    pub contenido: String,
    pub modificado: bool,
}

impl DocumentoAbierto {
    /// Crea un documento cuyo contenido todavía no tiene cambios posteriores.
    pub fn nuevo(uri: impl Into<String>, version: i32, contenido: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            version,
            contenido: contenido.into(),
            modificado: false,
        }
    }
}
