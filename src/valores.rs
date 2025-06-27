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
}

impl Valor {
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
        }
    }
}
