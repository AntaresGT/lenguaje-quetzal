use rust_decimal::Decimal;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Valor en tiempo de ejecución
#[derive(Debug, Clone)]
pub enum Valor {
    Vacio,
    Entero(i64),
    Numero(Decimal),
    Texto(String),
    Logico(bool),
    Lista(Vec<Valor>),
    Json(JsonValue),
    Objeto {
        tipo: String,
        propiedades: HashMap<String, Valor>,
    },
    Funcion {
        nombre: String,
        parametros: Vec<String>,
        parametros_mutables: Vec<bool>,
        cuerpo: Box<crate::nucleo::sintactico::ast::NodoAst>,
        asincrono: bool,
    },
}

impl Valor {
    /// Convierte el valor a texto
    pub fn a_texto(&self) -> String {
        match self {
            Valor::Vacio => "nulo".to_string(),
            Valor::Entero(val) => val.to_string(),
            Valor::Numero(val) => val.to_string(),
            Valor::Texto(val) => val.clone(),
            Valor::Logico(true) => "verdadero".to_string(),
            Valor::Logico(false) => "falso".to_string(),
            Valor::Lista(elements) => {
                let elementos: Vec<String> = elements.iter().map(|e| e.a_texto()).collect();
                format!("[{}]", elementos.join(", "))
            }
            Valor::Json(val) => val.to_string(),
            Valor::Objeto { tipo, .. } => format!("{} {{}}", tipo),
            Valor::Funcion { nombre, .. } => format!("funcion {}", nombre),
        }
    }
    
    /// Verifica si el valor es verdadero (para condiciones)
    pub fn es_verdadero(&self) -> bool {
        match self {
            Valor::Vacio => false,
            Valor::Entero(val) => *val != 0,
            Valor::Numero(val) => *val != Decimal::ZERO,
            Valor::Texto(val) => !val.is_empty(),
            Valor::Logico(val) => *val,
            Valor::Lista(val) => !val.is_empty(),
            Valor::Json(_) => true,
            Valor::Objeto { .. } => true,
            Valor::Funcion { .. } => true,
        }
    }
    
    /// Compara dos valores (implementación básica)
    pub fn es_igual(&self, otro: &Valor) -> bool {
        match (self, otro) {
            (Valor::Vacio, Valor::Vacio) => true,
            (Valor::Entero(a), Valor::Entero(b)) => a == b,
            (Valor::Numero(a), Valor::Numero(b)) => a == b,
            (Valor::Texto(a), Valor::Texto(b)) => a == b,
            (Valor::Logico(a), Valor::Logico(b)) => a == b,
            (Valor::Lista(a), Valor::Lista(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.es_igual(y))
            }
            _ => false,
        }
    }
    
    /// Obtiene el tipo del valor como string
    pub fn tipo(&self) -> &'static str {
        match self {
            Valor::Vacio => "vacio",
            Valor::Entero(_) => "entero",
            Valor::Numero(_) => "número",
            Valor::Texto(_) => "texto",
            Valor::Logico(_) => "logico",
            Valor::Lista(_) => "lista",
            Valor::Json(_) => "jsn",
            Valor::Objeto { .. } => "objeto",
            Valor::Funcion { .. } => "funcion",
        }
    }
}

impl From<i64> for Valor {
    fn from(val: i64) -> Self {
        Valor::Entero(val)
    }
}

impl From<Decimal> for Valor {
    fn from(val: Decimal) -> Self {
        Valor::Numero(val)
    }
}

impl From<String> for Valor {
    fn from(val: String) -> Self {
        Valor::Texto(val)
    }
}

impl From<&str> for Valor {
    fn from(val: &str) -> Self {
        Valor::Texto(val.to_string())
    }
}

impl From<bool> for Valor {
    fn from(val: bool) -> Self {
        Valor::Logico(val)
    }
}

impl From<Vec<Valor>> for Valor {
    fn from(val: Vec<Valor>) -> Self {
        Valor::Lista(val)
    }
}

impl From<JsonValue> for Valor {
    fn from(val: JsonValue) -> Self {
        Valor::Json(val)
    }
}
