use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum Valor {
    Vacio,
    Entero(i64),
    Numero(f64),
    Cadena(String),
    Bool(bool),
    Lista(Vec<Valor>),
    Objeto(HashMap<String, Valor>),
    Instancia(String, HashMap<String, Valor>),
    Funcion(DefFuncion),
}

#[derive(Clone, Debug)]
pub struct DefFuncion {
    pub nombre: String,
    pub parametros: Vec<(String, String)>, // (nombre, tipo)
    pub tipo_retorno: String,
    pub cuerpo: Vec<String>,
}

impl Valor {
    pub fn valor_por_defecto(tipo: &str) -> Option<Valor> {
        match tipo {
            "vacio" => Some(Valor::Vacio),
            "entero" => Some(Valor::Entero(0)),
            "número" => Some(Valor::Numero(0.0)),
            "cadena" => Some(Valor::Cadena(String::new())),
            "bool" => Some(Valor::Bool(false)),
            "lista" => Some(Valor::Lista(Vec::new())),
            "jsn" => Some(Valor::Objeto(HashMap::new())),
            _ => None,
        }
    }

    pub fn a_cadena(&self) -> String {
        match self {
            Valor::Vacio => "vacio".to_string(),
            Valor::Entero(i) => i.to_string(),
            Valor::Numero(n) => n.to_string(),
            Valor::Cadena(c) => c.clone(),
            Valor::Bool(b) => {
                if *b { "verdadero".to_string() } else { "falso".to_string() }
            }
            Valor::Lista(lista) => {
                let partes: Vec<String> = lista.iter().map(|v| v.a_cadena()).collect();
                format!("[{}]", partes.join(", "))
            }
            Valor::Objeto(obj) => {
                let partes: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.a_cadena()))
                    .collect();
                format!("{{{}}}", partes.join(", "))
            }
            Valor::Instancia(nombre, campos) => {
                let partes: Vec<String> = campos
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.a_cadena()))
                    .collect();
                format!("{} {{ {} }}", nombre, partes.join(", "))
            }
            Valor::Funcion(func) => {
                format!("funcion {}({})", func.nombre, 
                    func.parametros.iter()
                        .map(|(nombre, tipo)| format!("{}: {}", nombre, tipo))
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
        }
    }

    pub fn convertir_a_entero(&self) -> Result<i64, String> {
        match self {
            Valor::Entero(i) => Ok(*i),
            Valor::Numero(n) => Ok(*n as i64),
            Valor::Cadena(s) => s.parse::<i64>().map_err(|_| "Conversión a entero inválida".to_string()),
            _ => Err("No se puede convertir a entero".to_string()),
        }
    }
    
    pub fn convertir_a_numero(&self) -> Result<f64, String> {
        match self {
            Valor::Numero(n) => Ok(*n),
            Valor::Entero(i) => Ok(*i as f64),
            Valor::Cadena(s) => s.parse::<f64>().map_err(|_| "Conversión a número inválida".to_string()),
            _ => Err("No se puede convertir a número".to_string()),
        }
    }
    
    pub fn convertir_a_bool(&self) -> Result<bool, String> {
        match self {
            Valor::Bool(b) => Ok(*b),
            Valor::Cadena(s) => match s.as_str() {
                "verdadero" => Ok(true),
                "falso" => Ok(false),
                _ => Err("Conversión a booleano inválida".to_string()),
            },
            Valor::Entero(i) => Ok(*i != 0),
            Valor::Numero(n) => Ok(*n != 0.0),
            _ => Err("No se puede convertir a booleano".to_string()),
        }
    }
}
