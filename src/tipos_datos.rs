// Tipos de datos del lenguaje Quetzal
// Define todos los tipos de datos soportados por el lenguaje

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Valor que puede contener cualquier tipo de dato de Quetzal
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Valor {
    /// Tipo vacío - sin valor
    Vacio,
    /// Número entero
    Entero(i64),
    /// Número decimal/flotante  
    Numero(f64),
    /// Cadena de texto
    Cadena(String),
    /// Valor booleano
    Bool(bool),
    /// Lista de valores
    Lista(Vec<Valor>),
    /// Objeto JSON
    Json(HashMap<String, Valor>),
}

impl Valor {
    /// Convierte el valor a una cadena de texto
    pub fn a_cadena(&self) -> String {
        match self {
            Valor::Vacio => String::new(),
            Valor::Entero(n) => n.to_string(),
            Valor::Numero(n) => n.to_string(),
            Valor::Cadena(s) => s.clone(),
            Valor::Bool(b) => if *b { "verdadero".to_string() } else { "falso".to_string() },
            Valor::Lista(lista) => {
                let elementos: Vec<String> = lista.iter().map(|v| v.a_cadena()).collect();
                format!("[{}]", elementos.join(", "))
            },
            Valor::Json(objeto) => {
                let pares: Vec<String> = objeto.iter()
                    .map(|(clave, valor)| format!("{}: {}", clave, valor.a_cadena()))
                    .collect();
                format!("{{{}}}", pares.join(", "))
            }
        }
    }
    
    /// Convierte el valor a número entero
    pub fn a_entero(&self) -> Result<i64, String> {
        match self {
            Valor::Entero(n) => Ok(*n),
            Valor::Numero(n) => Ok(*n as i64),
            Valor::Cadena(s) => s.parse::<i64>()
                .map_err(|_| format!("No se puede convertir '{}' a entero", s)),
            Valor::Bool(b) => Ok(if *b { 1 } else { 0 }),
            _ => Err("Tipo no convertible a entero".to_string()),
        }
    }
    
    /// Convierte el valor a número decimal
    pub fn a_numero(&self) -> Result<f64, String> {
        match self {
            Valor::Entero(n) => Ok(*n as f64),
            Valor::Numero(n) => Ok(*n),
            Valor::Cadena(s) => s.parse::<f64>()
                .map_err(|_| format!("No se puede convertir '{}' a número", s)),
            Valor::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
            _ => Err("Tipo no convertible a número".to_string()),
        }
    }
    
    /// Convierte el valor a booleano
    pub fn a_bool(&self) -> bool {
        match self {
            Valor::Vacio => false,
            Valor::Entero(n) => *n != 0,
            Valor::Numero(n) => *n != 0.0,
            Valor::Cadena(s) => !s.is_empty() && s != "falso",
            Valor::Bool(b) => *b,
            Valor::Lista(lista) => !lista.is_empty(),
            Valor::Json(objeto) => !objeto.is_empty(),
        }
    }
    
    /// Obtiene el tipo del valor como cadena
    pub fn tipo_como_cadena(&self) -> &'static str {
        match self {
            Valor::Vacio => "vacio",
            Valor::Entero(_) => "entero",
            Valor::Numero(_) => "número",
            Valor::Cadena(_) => "cadena",
            Valor::Bool(_) => "bool",
            Valor::Lista(_) => "lista",
            Valor::Json(_) => "jsn",
        }
    }
}

impl fmt::Display for Valor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.a_cadena())
    }
}

/// Tipo de variable (mutable o inmutable)
#[derive(Debug, Clone, PartialEq)]
pub enum TipoVariable {
    Inmutable,
    Mutable,
}

/// Información sobre una variable
#[derive(Debug, Clone)]
pub struct Variable {
    pub nombre: String,
    pub valor: Valor,
    pub tipo_variable: TipoVariable,
    pub tipo_dato: String,
}

impl Variable {
    /// Crea una nueva variable
    pub fn nueva(nombre: String, valor: Valor, tipo_variable: TipoVariable, tipo_dato: String) -> Self {
        Variable {
            nombre,
            valor,
            tipo_variable,
            tipo_dato,
        }
    }
    
    /// Verifica si la variable es mutable
    pub fn es_mutable(&self) -> bool {
        matches!(self.tipo_variable, TipoVariable::Mutable)
    }
}
