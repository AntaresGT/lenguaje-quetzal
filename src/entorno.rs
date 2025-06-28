use std::collections::HashMap;
use crate::valores::{Valor, DefFuncion};
use crate::objetos::DefObjeto;

#[derive(Debug)]
pub struct Entorno {
    variables: HashMap<String, Valor>,
    objetos: HashMap<String, DefObjeto>,
    funciones: HashMap<String, DefFuncion>,
    pub padre: Option<Box<Entorno>>,
}

impl Entorno {
    pub fn nuevo() -> Self {
        Self { 
            variables: HashMap::new(), 
            objetos: HashMap::new(),
            funciones: HashMap::new(),
            padre: None,
        }
    }

    pub fn nuevo_con_padre(padre: Entorno) -> Self {
        Self {
            variables: HashMap::new(),
            objetos: HashMap::new(),
            funciones: HashMap::new(),
            padre: Some(Box::new(padre)),
        }
    }

    pub fn establecer(&mut self, nombre: &str, valor: Valor) {
        self.variables.insert(nombre.to_string(), valor);
    }

    pub fn obtener(&self, nombre: &str) -> Option<&Valor> {
        self.variables.get(nombre).or_else(|| {
            self.padre.as_ref().and_then(|p| p.obtener(nombre))
        })
    }

    pub fn definir_objeto(&mut self, def: DefObjeto) {
        self.objetos.insert(def.nombre.clone(), def);
    }

    pub fn obtener_objeto(&self, nombre: &str) -> Option<&DefObjeto> {
        self.objetos.get(nombre).or_else(|| {
            self.padre.as_ref().and_then(|p| p.obtener_objeto(nombre))
        })
    }

    pub fn definir_funcion(&mut self, def: DefFuncion) {
        self.funciones.insert(def.nombre.clone(), def);
    }

    pub fn obtener_funcion(&self, nombre: &str) -> Option<&DefFuncion> {
        self.funciones.get(nombre).or_else(|| {
            self.padre.as_ref().and_then(|p| p.obtener_funcion(nombre))
        })
    }
}
