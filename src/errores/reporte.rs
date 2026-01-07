#![allow(dead_code)]
#![allow(unused_assignments)]

use crate::errores::tipos::Error;
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

/// Convierte un Error de Quetzal en un Diagnostic de miette
#[derive(Error, Diagnostic, Debug)]
#[error("{mensaje}")]
pub struct ErrorDiagnostico {
    #[source_code]
    pub codigo: Option<String>,
    
    #[label("{mensaje}")]
    pub span: Option<SourceSpan>,
    
    pub mensaje: String,
    pub codigo_error: String,
    pub archivo: Option<String>,
    pub ayuda: Option<String>,
}

/// Reporta un error usando miette con formato similar a Rust
pub fn reportar_error(error: &Error, codigo_fuente: Option<&str>) {
    match error {
        Error::Analisis {
            codigo,
            mensaje,
            archivo,
            linea,
            columna,
            ayuda,
        }
        | Error::Semantico {
            codigo,
            mensaje,
            archivo,
            linea,
            columna,
            ayuda,
        }
        | Error::Ejecucion {
            codigo,
            mensaje,
            archivo,
            linea,
            columna,
            ayuda,
            ..
        } => {
            // Construir el mensaje principal
            let mut mensaje_completo = format!("error[{}]: {}", codigo, mensaje);
            
            // Agregar información de ubicación
            if let (Some(archivo), Some(linea), Some(columna)) = (archivo, linea, columna) {
                mensaje_completo.push_str(&format!("\n --> {}:{}:{}", archivo, linea, columna));
            } else if let Some(archivo) = archivo {
                mensaje_completo.push_str(&format!("\n --> {}", archivo));
            }
            
            // Mostrar el código fuente si está disponible
            if let Some(codigo) = codigo_fuente {
                if let (Some(linea), Some(columna)) = (linea, columna) {
                    let lineas: Vec<&str> = codigo.lines().collect();
                    if let Some(linea_texto) = lineas.get(linea.saturating_sub(1)) {
                        mensaje_completo.push_str(&format!("\n  |\n{} | {}", linea, linea_texto));
                        
                        // Agregar marcador de posición
                        let posicion = columna.saturating_sub(1);
                        let espacios = " ".repeat(posicion);
                        mensaje_completo.push_str(&format!("\n  | {}^ aquí está el error", espacios));
                    }
                }
            }
            
            // Agregar ayuda si está disponible
            if let Some(ayuda) = ayuda {
                mensaje_completo.push_str(&format!("\n  |\n  = ayuda: {}", ayuda));
            }
            
            eprintln!("{}", mensaje_completo);
        }
        
        Error::Modulo {
            codigo,
            mensaje,
            ruta,
            causa,
        } => {
            let mut mensaje_completo = format!("error[{}]: {}", codigo, mensaje);
            
            if let Some(ruta) = ruta {
                mensaje_completo.push_str(&format!("\n --> módulo: {}", ruta));
            }
            
            if let Some(causa) = causa {
                mensaje_completo.push_str(&format!("\n   causa: {}", causa));
            }
            
            eprintln!("{}", mensaje_completo);
        }
        
        Error::Sistema {
            codigo,
            mensaje,
            causa,
        } => {
            let mut mensaje_completo = format!("error[{}]: {}", codigo, mensaje);
            
            if let Some(causa) = causa {
                mensaje_completo.push_str(&format!("\n   causa: {}", causa));
            }
            
            eprintln!("{}", mensaje_completo);
        }
    }
}

/// Reporta un error simple sin código fuente
pub fn reportar_error_simple(error: &Error) {
    reportar_error(error, None);
}
