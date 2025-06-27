use std::collections::HashMap;
use crate::valores::Valor;
use crate::objetos::DefObjeto;

#[derive(Debug)]
pub struct Entorno {
    variables: HashMap<String, Valor>,
    objetos: HashMap<String, DefObjeto>,
}

impl Entorno {
    pub fn nuevo() -> Self {
        Self { variables: HashMap::new(), objetos: HashMap::new() }
    }

    pub fn establecer(&mut self, nombre: &str, valor: Valor) {
        self.variables.insert(nombre.to_string(), valor);
    }

    pub fn obtener(&self, nombre: &str) -> Option<&Valor> {
        self.variables.get(nombre)
    }

    pub fn definir_objeto(&mut self, def: DefObjeto) {
        self.objetos.insert(def.nombre.clone(), def);
    }

    pub fn obtener_objeto(&self, nombre: &str) -> Option<&DefObjeto> {
        self.objetos.get(nombre)
    }
}
