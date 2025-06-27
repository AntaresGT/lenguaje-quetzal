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
