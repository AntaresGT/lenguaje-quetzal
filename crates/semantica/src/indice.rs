//! Índice persistente de símbolos para herramientas de edición.

use ast::Visibilidad;
use nucleo::{ErrorQuetzal, Ubicacion};

use crate::TipoSemantico;

/// Función o categoría de un símbolo dentro del programa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaseSimbolo {
    Variable,
    Parametro,
    Funcion,
    Objeto,
    Prototipo,
    Importacion,
    Atributo,
    Metodo,
    Constructor,
    Nativo,
}

/// Clase de ámbito conservada durante el análisis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaseAmbito {
    Global,
    Funcion,
    Bloque,
    Bucle,
    Captura,
    Objeto,
    Metodo,
}

/// Ámbito y su relación con el ámbito padre.
#[derive(Debug, Clone)]
pub struct AmbitoAnalizado {
    pub id: u32,
    pub padre: Option<u32>,
    pub clase: ClaseAmbito,
}

/// Parámetro con nombre y tipo dentro de una firma.
#[derive(Debug, Clone)]
pub struct ParametroAnalizado {
    pub nombre: String,
    pub tipo: TipoSemantico,
    pub mutable: bool,
}

/// Firma estática de una función, método o constructor.
#[derive(Debug, Clone)]
pub struct FirmaAnalizada {
    pub parametros: Vec<ParametroAnalizado>,
    pub retorno: TipoSemantico,
    pub asincrona: bool,
}

/// Declaración persistente de un símbolo.
#[derive(Debug, Clone)]
pub struct SimboloAnalizado {
    pub id: u32,
    pub nombre: String,
    pub tipo: TipoSemantico,
    pub ubicacion: Ubicacion,
    pub mutable: bool,
    pub ambito: u32,
    pub clase: ClaseSimbolo,
    pub visibilidad: Option<Visibilidad>,
    pub libre: bool,
    pub firma: Option<FirmaAnalizada>,
}

/// Uso de un símbolo resuelto por identidad, no por coincidencia textual.
#[derive(Debug, Clone)]
pub struct ReferenciaAnalizada {
    pub simbolo: u32,
    pub ubicacion: Ubicacion,
}

/// Importación y símbolos locales que declara.
#[derive(Debug, Clone)]
pub struct ImportacionAnalizada {
    pub origen: String,
    pub ubicacion: Ubicacion,
    pub simbolos: Vec<u32>,
}

/// Exportación y declaraciones que publica.
#[derive(Debug, Clone)]
pub struct ExportacionAnalizada {
    pub simbolos: Vec<u32>,
    pub ubicacion: Ubicacion,
}

/// Relaciones de herencia, composición y miembros de un objeto.
#[derive(Debug, Clone)]
pub struct ObjetoAnalizado {
    pub simbolo: u32,
    pub nombre: String,
    pub padres: Vec<String>,
    pub extiende_como: Vec<String>,
    pub prototipos: Vec<String>,
    pub miembros: Vec<u32>,
}

/// Resultado semántico consultable sin ejecutar el programa.
#[derive(Debug, Default)]
pub struct ResultadoAnalisis {
    pub simbolos: Vec<SimboloAnalizado>,
    pub referencias: Vec<ReferenciaAnalizada>,
    pub ambitos: Vec<AmbitoAnalizado>,
    pub importaciones: Vec<ImportacionAnalizada>,
    pub exportaciones: Vec<ExportacionAnalizada>,
    pub objetos: Vec<ObjetoAnalizado>,
    pub diagnosticos: Vec<ErrorQuetzal>,
}

impl ResultadoAnalisis {
    /// Busca una declaración por su identificador interno.
    pub fn simbolo(&self, id: u32) -> Option<&SimboloAnalizado> {
        self.simbolos.get(id as usize)
    }

    /// Itera sobre todas las referencias de una declaración.
    pub fn referencias_de(&self, id: u32) -> impl Iterator<Item = &ReferenciaAnalizada> {
        self.referencias
            .iter()
            .filter(move |referencia| referencia.simbolo == id)
    }
}
