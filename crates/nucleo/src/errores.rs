//! Error central del Lenguaje Quetzal.
//!
//! Todas las etapas (léxico, sintaxis, semántica, runtime, paquetes) producen
//! [`ErrorQuetzal`]; el crate `diagnosticos` se encarga de mostrarlo con
//! estilo parecido a Rust.

use crate::ubicacion::Ubicacion;

/// Categoría de un error del lenguaje.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoriaError {
    Lexico,
    Sintactico,
    Semantico,
    Permisos,
    Runtime,
    Modulos,
    Paquetes,
    /// Errores del propio intérprete (E/S, configuración, etc.).
    Interno,
}

impl CategoriaError {
    pub fn nombre(&self) -> &'static str {
        match self {
            CategoriaError::Lexico => "léxico",
            CategoriaError::Sintactico => "sintáctico",
            CategoriaError::Semantico => "semántico",
            CategoriaError::Permisos => "permisos",
            CategoriaError::Runtime => "runtime",
            CategoriaError::Modulos => "módulos",
            CategoriaError::Paquetes => "paquetes",
            CategoriaError::Interno => "interno",
        }
    }
}

/// Error central del lenguaje con código, mensaje, ubicación y ayuda opcional.
#[derive(Debug, Clone, thiserror::Error)]
#[error("error[{codigo}]: {mensaje}")]
pub struct ErrorQuetzal {
    /// Código estable del error, por ejemplo `E0007`.
    pub codigo: String,
    /// Mensaje principal del error.
    pub mensaje: String,
    /// Categoría del error.
    pub categoria: CategoriaError,
    /// Ubicación del problema dentro del archivo fuente, si se conoce.
    pub ubicacion: Option<Ubicacion>,
    /// Nombre del archivo donde ocurrió, si se conoce.
    pub archivo: Option<String>,
    /// Etiqueta corta que se muestra bajo el caret `^^^^`.
    pub etiqueta: Option<String>,
    /// Texto de ayuda con una posible solución.
    pub ayuda: Option<String>,
}

impl ErrorQuetzal {
    pub fn nuevo(
        codigo: impl Into<String>,
        categoria: CategoriaError,
        mensaje: impl Into<String>,
    ) -> Self {
        Self {
            codigo: codigo.into(),
            mensaje: mensaje.into(),
            categoria,
            ubicacion: None,
            archivo: None,
            etiqueta: None,
            ayuda: None,
        }
    }

    pub fn con_ubicacion(mut self, ubicacion: Ubicacion) -> Self {
        self.ubicacion = Some(ubicacion);
        self
    }

    pub fn con_archivo(mut self, archivo: impl Into<String>) -> Self {
        self.archivo = Some(archivo.into());
        self
    }

    pub fn con_etiqueta(mut self, etiqueta: impl Into<String>) -> Self {
        self.etiqueta = Some(etiqueta.into());
        self
    }

    pub fn con_ayuda(mut self, ayuda: impl Into<String>) -> Self {
        self.ayuda = Some(ayuda.into());
        self
    }

    /// Error interno sin código específico, para fallas de E/S y similares.
    pub fn interno(mensaje: impl Into<String>) -> Self {
        Self::nuevo("E9999", CategoriaError::Interno, mensaje)
    }
}
