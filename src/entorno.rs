use std::collections::HashMap;
use crate::valores::{Valor, DefFuncion};
use crate::objetos::DefObjeto;

#[derive(Debug)]
pub struct Entorno {
    variables: HashMap<String, Valor>,
    objetos: HashMap<String, DefObjeto>,
    pub funciones: HashMap<String, DefFuncion>,
    pub padre: Option<Box<Entorno>>,
    exportados: HashMap<String, ExportedItem>,
}

#[derive(Debug, Clone)]
pub enum ExportedItem {
    Variable(Valor),
    Funcion(DefFuncion),
    Objeto(DefObjeto),
}

impl Entorno {
    pub fn nuevo() -> Self {
        Self { 
            variables: HashMap::new(), 
            objetos: HashMap::new(),
            funciones: HashMap::new(),
            padre: None,
            exportados: HashMap::new(),
        }
    }

    pub fn nuevo_con_padre(padre: Entorno) -> Self {
        Self {
            variables: HashMap::new(),
            objetos: HashMap::new(),
            funciones: HashMap::new(),
            padre: Some(Box::new(padre)),
            exportados: HashMap::new(),
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

    pub fn exportar(&mut self, nombre: &str) -> Result<(), String> {
        if let Some(valor) = self.variables.get(nombre) {
            self.exportados.insert(nombre.to_string(), ExportedItem::Variable(valor.clone()));
            Ok(())
        } else if let Some(funcion) = self.funciones.get(nombre) {
            self.exportados.insert(nombre.to_string(), ExportedItem::Funcion(funcion.clone()));
            Ok(())
        } else if let Some(objeto) = self.objetos.get(nombre) {
            self.exportados.insert(nombre.to_string(), ExportedItem::Objeto(objeto.clone()));
            Ok(())
        } else {
            Err(format!("'{}' no está definido y no puede ser exportado", nombre))
        }
    }

    pub fn obtener_exportado(&self, nombre: &str) -> Option<&ExportedItem> {
        self.exportados.get(nombre)
    }

    pub fn obtener_todos_exportados(&self) -> &HashMap<String, ExportedItem> {
        &self.exportados
    }

    pub fn importar_desde(&mut self, nombre: &str, alias: &str, item: &ExportedItem) {
        let nombre_final = if alias.is_empty() { nombre } else { alias };
        match item {
            ExportedItem::Variable(valor) => {
                self.variables.insert(nombre_final.to_string(), valor.clone());
            }
            ExportedItem::Funcion(func) => {
                let mut func_alias = func.clone();
                func_alias.nombre = nombre_final.to_string();
                self.funciones.insert(nombre_final.to_string(), func_alias);
            }
            ExportedItem::Objeto(obj) => {
                let mut obj_alias = obj.clone();
                obj_alias.nombre = nombre_final.to_string();
                self.objetos.insert(nombre_final.to_string(), obj_alias);
            }
        }
    }
}
