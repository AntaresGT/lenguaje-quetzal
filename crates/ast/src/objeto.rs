//! Definiciones de objetos del Lenguaje Quetzal.

use nucleo::Ubicacion;

use crate::declaracion::Funcion;
use crate::expresion::Expresion;
use crate::tipos::Tipo;

/// Visibilidad de un miembro de objeto o prototipo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibilidad {
    /// Si no se especifica etiqueta, los miembros son públicos.
    #[default]
    Publico,
    Privado,
}

/// `objeto Nombre hereda A, B como C implementa P, Q { ... }`
#[derive(Debug, Clone, PartialEq)]
pub struct DefinicionObjeto {
    pub nombre: String,
    /// Objetos padres (`hereda A, B`). La herencia múltiple se valida en
    /// fases posteriores, pero la gramática ya la acepta.
    pub padres: Vec<String>,
    /// `como EntidadAuditable`: composición/extensión declarada con `como`.
    pub extiende_como: Vec<String>,
    /// Prototipos implementados (`implementa P, Q`).
    pub prototipos: Vec<String>,
    pub miembros: Vec<MiembroObjeto>,
    pub ubicacion: Ubicacion,
}

/// Miembro de un objeto con su visibilidad y modificadores.
#[derive(Debug, Clone, PartialEq)]
pub struct MiembroObjeto {
    pub visibilidad: Visibilidad,
    /// `libre`: accesible sin instancia (no puede usar miembros de instancia).
    pub libre: bool,
    pub clase: ClaseMiembro,
}

/// Las clases de miembro que puede tener un objeto.
#[derive(Debug, Clone, PartialEq)]
pub enum ClaseMiembro {
    /// `texto var nombre` o `libre número var saldo = 0`
    Atributo {
        tipo: Tipo,
        mutable: bool,
        nombre: String,
        valor_inicial: Option<Expresion>,
        ubicacion: Ubicacion,
    },
    /// Función con nombre igual al objeto: `Usuario(texto nombre, ...) { }`
    Constructor(Funcion),
    /// Método normal del objeto.
    Metodo(Funcion),
}
