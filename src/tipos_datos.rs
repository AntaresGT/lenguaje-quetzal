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
    /// Valor nulo - representa ausencia de valor
    Nulo,
    /// Número entero
    Entero(i64),
    /// Número decimal/flotante  
    Numero(f64),
    /// Texto de caracteres
    Texto(String),
    /// Valor lógico (booleano)
    Log(bool),
    /// Lista de valores
    Lista(Vec<Valor>),
    /// Objeto JSON
    Json(HashMap<String, Valor>),
    /// Instancia de objeto Quetzal
    Objeto {
        clase: String,
        propiedades: HashMap<String, Valor>,
        propiedades_publicas: Vec<String>, // Lista de propiedades públicas
        metodos_publicos: Vec<String>, // Lista de métodos públicos
    },
}

impl Valor {
    /// Convierte el valor a un texto
    pub fn a_texto(&self) -> String {
        match self {
            Valor::Vacio => String::new(),
            Valor::Nulo => "nulo".to_string(),
            Valor::Entero(n) => n.to_string(),
            Valor::Numero(n) => n.to_string(),
            Valor::Texto(s) => s.clone(),
            Valor::Log(b) => if *b { "verdadero".to_string() } else { "falso".to_string() },
            Valor::Lista(lista) => {
                let elementos: Vec<String> = lista.iter().map(|v| v.a_texto()).collect();
                format!("[{}]", elementos.join(", "))
            },
            Valor::Json(objeto) => {
                let pares: Vec<String> = objeto.iter()
                    .map(|(clave, valor)| format!("{}: {}", clave, valor.a_texto()))
                    .collect();
                format!("{{{}}}", pares.join(", "))
            },
            Valor::Objeto { clase, propiedades, .. } => {
                let pares: Vec<String> = propiedades.iter()
                    .map(|(clave, valor)| format!("{}: {}", clave, valor.a_texto()))
                    .collect();
                format!("{}[{}]", clase, pares.join(", "))
            }
        }
    }
    
    /// Convierte el valor a número entero
    #[allow(dead_code)]
    pub fn a_entero(&self) -> Result<i64, String> {
        match self {
            Valor::Nulo => Err("No se puede convertir 'nulo' a entero".to_string()),
            Valor::Entero(n) => Ok(*n),
            Valor::Numero(n) => Ok(*n as i64),
            Valor::Texto(s) => s.parse::<i64>()
                .map_err(|_| format!("No se puede convertir '{}' a entero", s)),
            Valor::Log(b) => Ok(if *b { 1 } else { 0 }),
            _ => Err("Tipo no convertible a entero".to_string()),
        }
    }
    
    /// Convierte el valor a número decimal
    #[allow(dead_code)]
    pub fn a_numero(&self) -> Result<f64, String> {
        match self {
            Valor::Nulo => Err("No se puede convertir 'nulo' a número".to_string()),
            Valor::Entero(n) => Ok(*n as f64),
            Valor::Numero(n) => Ok(*n),
            Valor::Texto(s) => s.parse::<f64>()
                .map_err(|_| format!("No se puede convertir '{}' a número", s)),
            Valor::Log(b) => Ok(if *b { 1.0 } else { 0.0 }),
            _ => Err("Tipo no convertible a número".to_string()),
        }
    }
    
    /// Convierte el valor a lógico (booleano)
    pub fn a_log(&self) -> bool {
        match self {
            Valor::Vacio => false,
            Valor::Nulo => false,
            Valor::Entero(n) => *n != 0,
            Valor::Numero(n) => *n != 0.0,
            Valor::Texto(s) => !s.is_empty() && s != "falso",
            Valor::Log(b) => *b,
            Valor::Lista(lista) => !lista.is_empty(),
            Valor::Json(objeto) => !objeto.is_empty(),
            Valor::Objeto { propiedades, .. } => !propiedades.is_empty(),
        }
    }
    
    /// Obtiene el tipo del valor como texto
    pub fn tipo_como_texto(&self) -> &'static str {
        match self {
            Valor::Vacio => "vacio",
            Valor::Nulo => "nulo",
            Valor::Entero(_) => "entero",
            Valor::Numero(_) => "número",
            Valor::Texto(_) => "texto",
            Valor::Log(_) => "log",
            Valor::Lista(_) => "lista",
            Valor::Json(_) => "jsn",
            Valor::Objeto { .. } => "objeto",
        }
    }
    
    // === MÉTODOS DE COMPATIBILIDAD HACIA ATRÁS ===
    // Estos métodos mantienen compatibilidad mientras se migra el código
    
    /// Alias para a_texto() - mantiene compatibilidad
    pub fn a_cadena(&self) -> String {
        self.a_texto()
    }
    
    /// Alias para a_log() - mantiene compatibilidad
    pub fn a_bool(&self) -> bool {
        self.a_log()
    }
    
    /// Alias para tipo_como_texto() - mantiene compatibilidad
    pub fn tipo_como_cadena(&self) -> &'static str {
        self.tipo_como_texto()
    }
}

impl fmt::Display for Valor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.a_texto())
    }
}

/// Tipo de variable (variable o inmutable)
#[derive(Debug, Clone, PartialEq)]
pub enum TipoVariable {
    Inmutable,
    Variable,
}

/// Información sobre una variable
#[derive(Debug, Clone)]
pub struct Variable {
    #[allow(dead_code)]
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
    
    /// Verifica si la variable es variable (mutable)
    pub fn es_variable(&self) -> bool {
        matches!(self.tipo_variable, TipoVariable::Variable)
    }
    
    /// Alias para compatibilidad - verifica si la variable es mutable
    pub fn es_mutable(&self) -> bool {
        self.es_variable()
    }
}

// Implementación manual de Hash para Valor (necesario para la VM universal)
impl std::hash::Hash for Valor {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Valor::Vacio => 0u8.hash(state),
            Valor::Nulo => 1u8.hash(state),
            Valor::Entero(n) => {
                2u8.hash(state);
                n.hash(state);
            },
            Valor::Numero(f) => {
                3u8.hash(state);
                // Para f64, usamos la representación en bits
                f.to_bits().hash(state);
            },
            Valor::Texto(s) => {
                4u8.hash(state);
                s.hash(state);
            },
            Valor::Log(b) => {
                5u8.hash(state);
                b.hash(state);
            },
            Valor::Lista(lista) => {
                6u8.hash(state);
                lista.hash(state);
            },
            Valor::Json(mapa) => {
                7u8.hash(state);
                // Para HashMap, ordenamos las claves para hash consistente
                let mut items: Vec<_> = mapa.iter().collect();
                items.sort_by_key(|(k, _)| *k);
                items.hash(state);
            },
            Valor::Objeto { clase, propiedades, propiedades_publicas, metodos_publicos } => {
                8u8.hash(state);
                clase.hash(state);
                propiedades_publicas.hash(state);
                metodos_publicos.hash(state);
                // Para HashMap, ordenamos las claves para hash consistente
                let mut items: Vec<_> = propiedades.iter().collect();
                items.sort_by_key(|(k, _)| *k);
                items.hash(state);
            },
        }
    }
}
