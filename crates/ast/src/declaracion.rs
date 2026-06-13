//! Declaraciones de funciones del Lenguaje Quetzal.

use nucleo::Ubicacion;

use crate::sentencia::Bloque;
use crate::tipos::Tipo;

/// Parámetro de una función: `texto var palabra`.
#[derive(Debug, Clone, PartialEq)]
pub struct Parametro {
    pub tipo: Tipo,
    pub mutable: bool,
    pub nombre: String,
    pub ubicacion: Ubicacion,
}

/// Declaración de función: `entero sumar(entero a, entero b) { ... }`.
#[derive(Debug, Clone, PartialEq)]
pub struct Funcion {
    pub nombre: String,
    pub tipo_retorno: Tipo,
    pub parametros: Vec<Parametro>,
    pub cuerpo: Bloque,
    pub asincrona: bool,
    pub ubicacion: Ubicacion,
}
