//! Fallos de ejecución de la VM.

use nucleo::{CategoriaError, ErrorQuetzal, Ubicacion};

/// Datos de una excepción de Quetzal (capturable con `intentar/capturar`).
#[derive(Debug, Clone)]
pub struct DatosExcepcion {
    pub mensaje: String,
    /// Pila de llamadas en el momento del lanzamiento.
    pub llamadas: Vec<String>,
    /// Código de error si la excepción proviene del runtime (E0401, ...).
    pub codigo: Option<String>,
    /// Ubicación donde se originó, si se conoce.
    pub ubicacion: Option<Ubicacion>,
}

/// Fallo durante la ejecución.
#[derive(Debug)]
pub enum Fallo {
    /// Excepción de Quetzal: capturable con `intentar/capturar`.
    Excepcion(DatosExcepcion),
    /// Error interno no capturable (bytecode corrupto, límite de pila, ...).
    Error(Box<ErrorQuetzal>),
}

impl Fallo {
    /// Crea una excepción de runtime con código.
    pub fn excepcion(
        codigo: &str,
        mensaje: impl Into<String>,
        llamadas: Vec<String>,
        ubicacion: Option<Ubicacion>,
    ) -> Self {
        Fallo::Excepcion(DatosExcepcion {
            mensaje: mensaje.into(),
            llamadas,
            codigo: Some(codigo.to_string()),
            ubicacion,
        })
    }

    /// Convierte el fallo en un `ErrorQuetzal` para reportarlo (excepción no
    /// capturada o error interno).
    pub fn a_error(self, archivo: &str) -> ErrorQuetzal {
        match self {
            Fallo::Error(error) => *error,
            Fallo::Excepcion(datos) => {
                let codigo = datos.codigo.unwrap_or_else(|| "E0405".to_string());
                let mut error = ErrorQuetzal::nuevo(
                    codigo,
                    CategoriaError::Runtime,
                    format!("excepción no capturada: {}", datos.mensaje),
                )
                .con_archivo(archivo)
                .con_ayuda(
                    "envuelve la operación en 'intentar { ... } capturar (excepcion e) { ... }'",
                );
                if let Some(ubicacion) = datos.ubicacion {
                    error = error
                        .con_ubicacion(ubicacion)
                        .con_etiqueta("aquí se lanzó la excepción");
                }
                error
            }
        }
    }
}
