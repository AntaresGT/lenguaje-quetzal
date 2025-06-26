use crate::utileria::leer_archivo;
use crate::lexico::{analizar_lexico, Token};

/// Representa al intérprete de Quetzal
pub struct Interprete {}

impl Interprete {
    /// Crea una nueva instancia del intérprete
    pub fn nuevo() -> Self {
        Interprete {}
    }

    /// Ejecuta un archivo de Quetzal y devuelve la lista de tokens
    pub fn ejecutar_archivo(&self, ruta: &str) -> Result<Vec<Token>, String> {
        let contenido = leer_archivo(ruta).map_err(|e| e.to_string())?;
        let tokens = analizar_lexico(&contenido);
        Ok(tokens)
    }
}
