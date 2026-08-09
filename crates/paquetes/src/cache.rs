//! Cache de bytecode en `.quetzal/cache/`.
//!
//! La clave de cada entrada combina el hash del código fuente, la versión de
//! Quetzal y la versión del formato de bytecode: cualquier cambio invalida la
//! entrada automáticamente.

use std::path::{Path, PathBuf};

use bytecode::{ModuloCompilado, modulo::VERSION_FORMATO_BYTECODE};
use sha2::{Digest, Sha256};

/// Cache de módulos compilados de un proyecto.
pub struct CacheBytecode {
    directorio: PathBuf,
}

impl CacheBytecode {
    /// Cache dentro de `.quetzal/cache/` del directorio del proyecto.
    pub fn del_proyecto(raiz: &Path) -> Self {
        Self {
            directorio: raiz.join(".quetzal").join("cache"),
        }
    }

    /// Clave estable para un archivo fuente.
    pub fn clave(contenido_fuente: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(contenido_fuente.as_bytes());
        hasher.update(nucleo::VERSION_QUETZAL.as_bytes());
        hasher.update(VERSION_FORMATO_BYTECODE.to_le_bytes());
        hex::encode(hasher.finalize())
    }

    /// Recupera un módulo compilado si hay una entrada válida.
    pub fn buscar(&self, contenido_fuente: &str) -> Option<ModuloCompilado> {
        let ruta = self.ruta_entrada(contenido_fuente);
        let datos = std::fs::read_to_string(ruta).ok()?;
        // Una entrada corrupta o de un formato viejo simplemente se ignora.
        serde_json::from_str(&datos).ok()
    }

    /// Guarda un módulo compilado (mejor esfuerzo: si falla, se recompila
    /// la próxima vez).
    pub fn guardar(&self, contenido_fuente: &str, modulo: &ModuloCompilado) {
        if std::fs::create_dir_all(&self.directorio).is_err() {
            return;
        }
        if let Ok(datos) = serde_json::to_string(modulo) {
            let _ = std::fs::write(self.ruta_entrada(contenido_fuente), datos);
        }
    }

    /// Borra todas las entradas de la cache.
    pub fn limpiar(&self) -> std::io::Result<()> {
        if self.directorio.is_dir() {
            std::fs::remove_dir_all(&self.directorio)?;
        }
        Ok(())
    }

    fn ruta_entrada(&self, contenido_fuente: &str) -> PathBuf {
        self.directorio
            .join(format!("{}.bytecode.json", Self::clave(contenido_fuente)))
    }
}
