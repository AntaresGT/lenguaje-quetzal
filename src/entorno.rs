use std::collections::HashMap;
use crate::valores::Valor;

#[derive(Debug)]
pub struct Entorno {
    variables: HashMap<String, Valor>,
}

impl Entorno {
    pub fn nuevo() -> Self {
        Self { variables: HashMap::new() }
    }

    pub fn establecer(&mut self, nombre: &str, valor: Valor) {
        self.variables.insert(nombre.to_string(), valor);
    }

    pub fn obtener(&self, nombre: &str) -> Option<&Valor> {
        self.variables.get(nombre)
    }
}
