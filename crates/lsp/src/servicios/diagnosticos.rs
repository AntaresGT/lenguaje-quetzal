//! Conversión de errores de Quetzal a diagnósticos LSP.

use lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString, Uri,
};
use nucleo::ErrorQuetzal;

use crate::documentos::ConvertidorPosiciones;

/// Convierte errores del compilador usando las posiciones negociadas.
pub fn convertir_diagnosticos(
    uri: &Uri,
    contenido: &str,
    errores: Vec<ErrorQuetzal>,
) -> Vec<Diagnostic> {
    let convertidor = ConvertidorPosiciones::nuevo(contenido);
    errores
        .into_iter()
        .map(|error| {
            let ubicacion = error.ubicacion.unwrap_or_default();
            let rango = convertidor.ubicacion_a_rango(ubicacion);
            let relacionados = error.ayuda.map(|ayuda| {
                vec![DiagnosticRelatedInformation {
                    location: Location::new(uri.clone(), rango),
                    message: ayuda,
                }]
            });
            Diagnostic {
                range: rango,
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(NumberOrString::String(error.codigo)),
                source: Some("quetzal".to_string()),
                message: error.mensaje,
                related_information: relacionados,
                ..Diagnostic::default()
            }
        })
        .collect()
}
