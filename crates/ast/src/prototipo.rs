//! Definiciones de prototipos (contratos/interfaces) del Lenguaje Quetzal.

use nucleo::Ubicacion;

use crate::declaracion::Parametro;
use crate::objeto::Visibilidad;
use crate::tipos::Tipo;

/// `prototipo Nombre { ... }`
#[derive(Debug, Clone, PartialEq)]
pub struct DefinicionPrototipo {
    pub nombre: String,
    pub miembros: Vec<MiembroPrototipo>,
    pub ubicacion: Ubicacion,
}

/// Miembro declarado dentro de un prototipo.
#[derive(Debug, Clone, PartialEq)]
pub struct MiembroPrototipo {
    pub visibilidad: Visibilidad,
    /// `opcional`: el objeto que implementa puede omitirlo.
    pub opcional: bool,
    pub firma: FirmaMiembro,
}

/// Firma de un miembro de prototipo (sin cuerpo).
#[derive(Debug, Clone, PartialEq)]
pub enum FirmaMiembro {
    /// `texto var atributoMutable`
    Atributo {
        tipo: Tipo,
        mutable: bool,
        nombre: String,
        ubicacion: Ubicacion,
    },
    /// `entero suma(entero a, entero b)`
    Metodo {
        tipo_retorno: Tipo,
        nombre: String,
        parametros: Vec<Parametro>,
        ubicacion: Ubicacion,
    },
}

impl FirmaMiembro {
    pub fn nombre(&self) -> &str {
        match self {
            FirmaMiembro::Atributo { nombre, .. } | FirmaMiembro::Metodo { nombre, .. } => nombre,
        }
    }
}
