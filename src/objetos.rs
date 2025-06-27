use std::collections::HashMap;
use crate::valores::Valor;

#[derive(Clone, Debug)]
pub struct DefObjeto {
    pub nombre: String,
    pub campos: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Instancia {
    pub tipo: String,
    pub campos: HashMap<String, Valor>,
}
