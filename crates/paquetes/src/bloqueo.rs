//! `quetzal.bloquear`: archivo de bloqueo con versiones y hashes.

use std::path::Path;

use indexmap::IndexMap;
use nucleo::{CategoriaError, ErrorQuetzal, ResultadoQuetzal};
use serde::{Deserialize, Serialize};

/// Contenido del archivo `quetzal.bloquear`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Bloqueo {
    /// Versión del formato del archivo de bloqueo.
    pub formato: u32,
    /// Versión de Quetzal con la que se generó.
    pub quetzal: String,
    /// Dependencias bloqueadas: nombre → entrada.
    pub dependencias: IndexMap<String, DependenciaBloqueada>,
}

/// Una dependencia fijada con su versión, origen y hash de integridad.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependenciaBloqueada {
    pub version: String,
    /// `ruta:./...` para locales; URL o registro para remotas (futuro).
    pub origen: String,
    /// SHA-256 del contenido instalado.
    pub integridad: String,
}

/// Versión actual del formato de `quetzal.bloquear`.
pub const FORMATO_BLOQUEO: u32 = 1;

impl Bloqueo {
    pub fn nuevo() -> Self {
        Self {
            formato: FORMATO_BLOQUEO,
            quetzal: nucleo::VERSION_QUETZAL.to_string(),
            dependencias: IndexMap::new(),
        }
    }

    /// Lee el `quetzal.bloquear` de un directorio si existe.
    pub fn leer(directorio: &Path) -> ResultadoQuetzal<Option<Bloqueo>> {
        let ruta = directorio.join("quetzal.bloquear");
        if !ruta.is_file() {
            return Ok(None);
        }
        let contenido = std::fs::read_to_string(&ruta).map_err(|error| {
            error_bloqueo(format!("no se pudo leer '{}': {error}", ruta.display()))
        })?;
        serde_json::from_str(&contenido)
            .map(Some)
            .map_err(|error| error_bloqueo(format!("'{}' está corrupto: {error}", ruta.display())))
    }

    /// Escribe el archivo de bloqueo en el directorio del proyecto.
    pub fn guardar(&self, directorio: &Path) -> ResultadoQuetzal<()> {
        let ruta = directorio.join("quetzal.bloquear");
        let contenido = serde_json::to_string_pretty(self)
            .map_err(|error| error_bloqueo(format!("no se pudo serializar el bloqueo: {error}")))?;
        std::fs::write(&ruta, contenido + "\n").map_err(|error| {
            error_bloqueo(format!("no se pudo escribir '{}': {error}", ruta.display()))
        })
    }
}

fn error_bloqueo(mensaje: impl Into<String>) -> ErrorQuetzal {
    ErrorQuetzal::nuevo("E0601", CategoriaError::Paquetes, mensaje)
}
