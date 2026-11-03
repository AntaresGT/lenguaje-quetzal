//! Almacén de documentos abiertos, indexado por URI.

use std::collections::HashMap;

use super::DocumentoAbierto;

/// Colección de documentos abiertos con control de versiones.
#[derive(Debug, Default)]
pub struct AlmacenDocumentos {
    documentos: HashMap<String, DocumentoAbierto>,
}

impl AlmacenDocumentos {
    /// Abre o reemplaza un documento usando el texto enviado por el editor.
    pub fn abrir(&mut self, documento: DocumentoAbierto) {
        self.documentos.insert(documento.uri.clone(), documento);
    }

    /// Reemplaza todo el contenido si la versión es más reciente.
    ///
    /// Devuelve `false` si el documento no está abierto o la versión es vieja.
    pub fn cambiar_completo(
        &mut self,
        uri: &str,
        version: i32,
        contenido: impl Into<String>,
    ) -> bool {
        let Some(documento) = self.documentos.get_mut(uri) else {
            return false;
        };
        if version <= documento.version {
            return false;
        }
        documento.version = version;
        documento.contenido = contenido.into();
        documento.modificado = true;
        true
    }

    /// Marca el documento como guardado y conserva su versión en memoria.
    pub fn guardar(&mut self, uri: &str, contenido: Option<String>) -> bool {
        let Some(documento) = self.documentos.get_mut(uri) else {
            return false;
        };
        if let Some(contenido) = contenido {
            documento.contenido = contenido;
        }
        documento.modificado = false;
        true
    }

    /// Elimina un documento cerrado y devuelve su última versión.
    pub fn cerrar(&mut self, uri: &str) -> Option<DocumentoAbierto> {
        self.documentos.remove(uri)
    }

    /// Consulta un documento sin acudir al sistema de archivos.
    pub fn obtener(&self, uri: &str) -> Option<&DocumentoAbierto> {
        self.documentos.get(uri)
    }

    /// Número de documentos abiertos actualmente.
    pub fn cantidad(&self) -> usize {
        self.documentos.len()
    }
}
