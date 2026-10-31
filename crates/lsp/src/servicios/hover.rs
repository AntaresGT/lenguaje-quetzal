//! Información contextual de símbolos bajo el cursor.

use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};
use semantica::indice::ResultadoAnalisis;

use crate::documentos::ConvertidorPosiciones;

use super::simbolos::{describir_simbolo, simbolo_en_offset};

pub fn informacion(analisis: &ResultadoAnalisis, contenido: &str, offset: usize) -> Option<Hover> {
    let (simbolo, ubicacion) = simbolo_en_offset(analisis, contenido, offset)?;
    let rango = ConvertidorPosiciones::nuevo(contenido).ubicacion_a_rango(ubicacion);
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("```quetzal\n{}\n```", describir_simbolo(simbolo)),
        }),
        range: Some(rango),
    })
}
