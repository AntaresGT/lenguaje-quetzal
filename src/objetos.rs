use std::collections::HashMap;
use crate::valores::Valor;

pub type TipoMetodo = fn(&mut HashMap<String, Valor>, Vec<Valor>) -> Option<Valor>;

#[derive(Clone, Debug)]
pub struct DefObjeto {
    pub nombre: String,
    pub campos: Vec<String>,
    pub metodos: HashMap<String, TipoMetodo>,
}

#[derive(Clone, Debug)]
pub struct Instancia {
    pub tipo: String,
    pub campos: HashMap<String, Valor>,
}
