use crate::errores::codigos::CodigoError;
use thiserror::Error;

/// Tipo de resultado estándar para operaciones que pueden fallar
pub type Resultado<T> = std::result::Result<T, Error>;

/// Error principal del intérprete de Quetzal
#[derive(Error, Debug)]
pub enum Error {
    /// Error de análisis léxico o sintáctico
    #[error("error[{codigo}]: {mensaje}")]
    Analisis {
        codigo: String,
        mensaje: String,
        archivo: Option<String>,
        linea: Option<usize>,
        columna: Option<usize>,
        ayuda: Option<String>,
    },
    
    /// Error semántico
    #[error("error[{codigo}]: {mensaje}")]
    Semantico {
        codigo: String,
        mensaje: String,
        archivo: Option<String>,
        linea: Option<usize>,
        columna: Option<usize>,
        ayuda: Option<String>,
    },
    
    /// Error en tiempo de ejecución
    #[error("error[{codigo}]: {mensaje}")]
    Ejecucion {
        codigo: String,
        mensaje: String,
        archivo: Option<String>,
        linea: Option<usize>,
        columna: Option<usize>,
        ayuda: Option<String>,
        pila_llamadas: Vec<String>,
    },
    
    /// Error de módulo
    #[error("error[{codigo}]: {mensaje}")]
    Modulo {
        codigo: String,
        mensaje: String,
        ruta: Option<String>,
        causa: Option<Box<Error>>,
    },
    
    /// Error de sistema/IO
    #[error("error[{codigo}]: {mensaje}")]
    Sistema {
        codigo: String,
        mensaje: String,
        causa: Option<String>,
    },
}

impl Error {
    /// Crea un error de análisis
    pub fn analisis(
        codigo: CodigoError,
        mensaje: impl Into<String>,
        archivo: Option<String>,
        linea: Option<usize>,
        columna: Option<usize>,
    ) -> Self {
        Error::Analisis {
            codigo: codigo.codigo().to_string(),
            mensaje: mensaje.into(),
            archivo,
            linea,
            columna,
            ayuda: codigo.ayuda().map(|s| s.to_string()),
        }
    }
    
    /// Crea un error semántico
    pub fn semantico(
        codigo: CodigoError,
        mensaje: impl Into<String>,
        archivo: Option<String>,
        linea: Option<usize>,
        columna: Option<usize>,
    ) -> Self {
        Error::Semantico {
            codigo: codigo.codigo().to_string(),
            mensaje: mensaje.into(),
            archivo,
            linea,
            columna,
            ayuda: codigo.ayuda().map(|s| s.to_string()),
        }
    }
    
    /// Crea un error de ejecución
    pub fn ejecucion(
        codigo: CodigoError,
        mensaje: impl Into<String>,
        archivo: Option<String>,
        linea: Option<usize>,
        columna: Option<usize>,
    ) -> Self {
        Error::Ejecucion {
            codigo: codigo.codigo().to_string(),
            mensaje: mensaje.into(),
            archivo,
            linea,
            columna,
            ayuda: codigo.ayuda().map(|s| s.to_string()),
            pila_llamadas: Vec::new(),
        }
    }
    
    /// Crea un error de ejecución con pila de llamadas
    pub fn ejecucion_con_pila(
        codigo: CodigoError,
        mensaje: impl Into<String>,
        archivo: Option<String>,
        linea: Option<usize>,
        columna: Option<usize>,
        pila_llamadas: Vec<String>,
    ) -> Self {
        Error::Ejecucion {
            codigo: codigo.codigo().to_string(),
            mensaje: mensaje.into(),
            archivo,
            linea,
            columna,
            ayuda: codigo.ayuda().map(|s| s.to_string()),
            pila_llamadas,
        }
    }
    
    /// Crea un error de módulo
    pub fn modulo(
        codigo: CodigoError,
        mensaje: impl Into<String>,
        ruta: Option<String>,
    ) -> Self {
        Error::Modulo {
            codigo: codigo.codigo().to_string(),
            mensaje: mensaje.into(),
            ruta,
            causa: None,
        }
    }
    
    /// Crea un error de sistema
    pub fn sistema(
        codigo: CodigoError,
        mensaje: impl Into<String>,
        causa: Option<String>,
    ) -> Self {
        Error::Sistema {
            codigo: codigo.codigo().to_string(),
            mensaje: mensaje.into(),
            causa,
        }
    }
    
    /// Obtiene el código de error
    pub fn codigo(&self) -> &str {
        match self {
            Error::Analisis { codigo, .. }
            | Error::Semantico { codigo, .. }
            | Error::Ejecucion { codigo, .. }
            | Error::Modulo { codigo, .. }
            | Error::Sistema { codigo, .. } => codigo,
        }
    }
    
    /// Obtiene el mensaje de error
    pub fn mensaje(&self) -> &str {
        match self {
            Error::Analisis { mensaje, .. }
            | Error::Semantico { mensaje, .. }
            | Error::Ejecucion { mensaje, .. }
            | Error::Modulo { mensaje, .. }
            | Error::Sistema { mensaje, .. } => mensaje,
        }
    }
    
    /// Obtiene la ayuda asociada al error
    pub fn ayuda(&self) -> Option<&str> {
        match self {
            Error::Analisis { ayuda, .. }
            | Error::Semantico { ayuda, .. }
            | Error::Ejecucion { ayuda, .. } => ayuda.as_deref(),
            _ => None,
        }
    }
    
    /// Obtiene la información de ubicación
    pub fn ubicacion(&self) -> Option<(Option<String>, Option<usize>, Option<usize>)> {
        match self {
            Error::Analisis { archivo, linea, columna, .. }
            | Error::Semantico { archivo, linea, columna, .. }
            | Error::Ejecucion { archivo, linea, columna, .. } => {
                Some((archivo.clone(), *linea, *columna))
            }
            _ => None,
        }
    }
    
    /// Obtiene la pila de llamadas (si existe)
    pub fn pila_llamadas(&self) -> Option<&[String]> {
        match self {
            Error::Ejecucion { pila_llamadas, .. } => Some(pila_llamadas),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::sistema(
            CodigoError::ErrorLecturaArchivo,
            format!("error de entrada/salida: {}", err),
            Some(err.to_string()),
        )
    }
}
