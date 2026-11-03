//! Navegación desde una referencia hasta su declaración.

use lsp_types::{Location, Uri};
use semantica::indice::ResultadoAnalisis;

use crate::documentos::ConvertidorPosiciones;

use super::simbolos::{rango_nombre, simbolo_en_offset};

pub fn ubicacion_definicion(
    uri: &Uri,
    analisis: &ResultadoAnalisis,
    contenido: &str,
    offset: usize,
) -> Option<Location> {
    let (simbolo, _) = simbolo_en_offset(analisis, contenido, offset)?;
    let nombre = rango_nombre(simbolo, contenido)?;
    let rango = ConvertidorPosiciones::nuevo(contenido).ubicacion_a_rango(nombre);
    Some(Location::new(uri.clone(), rango))
}
