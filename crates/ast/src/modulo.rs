//! Módulos del Lenguaje Quetzal: un archivo `.qz` con sus elementos.

use nucleo::Ubicacion;

use crate::declaracion::Funcion;
use crate::objeto::DefinicionObjeto;
use crate::prototipo::DefinicionPrototipo;
use crate::sentencia::Sentencia;

/// Un archivo `.qz` parseado completo.
#[derive(Debug, Clone, PartialEq)]
pub struct Modulo {
    /// Nombre del archivo, por ejemplo `aplicacion/principal.qz`.
    pub nombre: String,
    pub elementos: Vec<Elemento>,
}

/// Elemento de primer nivel de un módulo.
#[derive(Debug, Clone, PartialEq)]
pub enum Elemento {
    Importacion(Importacion),
    /// `exportar { sumar, Usuario, saludo }`
    Exportacion {
        simbolos: Vec<String>,
        ubicacion: Ubicacion,
    },
    Funcion(Funcion),
    Objeto(DefinicionObjeto),
    Prototipo(DefinicionPrototipo),
    /// Cualquier sentencia de nivel superior (declaraciones, llamadas, ...).
    Sentencia(Sentencia),
}

/// `importar { sumar, saludo como alias } desde "./calculadora.qz"`
#[derive(Debug, Clone, PartialEq)]
pub struct Importacion {
    pub simbolos: Vec<SimboloImportado>,
    /// Origen: `quetzal/matematica`, `./archivo.qz` o nombre de dependencia.
    pub origen: String,
    pub ubicacion: Ubicacion,
}

/// Símbolo importado con alias opcional (`saludo como texto_saludo`).
#[derive(Debug, Clone, PartialEq)]
pub struct SimboloImportado {
    pub nombre: String,
    pub alias: Option<String>,
}

impl SimboloImportado {
    /// Nombre con el que el símbolo queda visible en el módulo.
    pub fn nombre_local(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.nombre)
    }
}
