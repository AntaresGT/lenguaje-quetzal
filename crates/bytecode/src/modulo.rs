//! Formato de un módulo compilado a bytecode.

use crate::constantes::Constante;
use crate::instruccion::Trozo;

/// Versión del formato de bytecode (invalida la cache al cambiar).
pub const VERSION_FORMATO_BYTECODE: u32 = 2;

/// Función compilada a bytecode.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FuncionCompilada {
    pub nombre: String,
    /// Parámetros: `(nombre, mutable)`.
    pub parametros: Vec<(String, bool)>,
    pub trozo: Trozo,
    pub asincrona: bool,
    /// Ranuras locales del marco, incluyendo parámetros (ocupan `0..n`).
    #[serde(default)]
    pub ranuras_locales: u16,
}

/// Método u operación de un objeto compilado.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MetodoCompilado {
    pub funcion: FuncionCompilada,
    pub publico: bool,
    pub libre: bool,
}

/// Atributo de un objeto compilado.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AtributoCompilado {
    pub nombre: String,
    pub mutable: bool,
    pub publico: bool,
    pub libre: bool,
    /// Código que produce el valor inicial, si fue declarado.
    pub inicial: Option<Trozo>,
}

/// Objeto compilado con sus miembros.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct ObjetoCompilado {
    pub nombre: String,
    pub padres: Vec<String>,
    pub extiende_como: Vec<String>,
    pub atributos: Vec<AtributoCompilado>,
    pub metodos: Vec<MetodoCompilado>,
    pub constructores: Vec<FuncionCompilada>,
}

/// Importación pendiente de resolver por el motor.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ImportacionCompilada {
    /// `(nombre, nombre_local)` de cada símbolo.
    pub simbolos: Vec<(String, String)>,
    pub origen: String,
}

/// Un módulo Quetzal compilado completo.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ModuloCompilado {
    pub nombre: String,
    pub constantes: Vec<Constante>,
    pub nombres: Vec<String>,
    /// Nombres de las globales del módulo, en el orden de sus índices.
    #[serde(default)]
    pub nombres_globales: Vec<String>,
    /// Mutabilidad de cada global, alineada con [`Self::nombres_globales`].
    #[serde(default)]
    pub globales_mutables: Vec<bool>,
    /// Ranuras locales del código principal (bloques de nivel superior).
    #[serde(default)]
    pub ranuras_locales_principal: u16,
    pub funciones: Vec<FuncionCompilada>,
    pub objetos: Vec<ObjetoCompilado>,
    pub importaciones: Vec<ImportacionCompilada>,
    pub exportaciones: Vec<String>,
    /// Código del nivel superior del módulo.
    pub principal: Trozo,
}
