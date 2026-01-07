use crate::nucleo::semantico::tipos::Tipo;
use std::collections::HashMap;

/// Información sobre una variable
#[derive(Debug, Clone)]
pub struct Variable {
    pub nombre: String,
    pub tipo: Tipo,
    pub mutable: bool,
    pub inicializada: bool,
}

/// Información sobre una función
#[derive(Debug, Clone)]
pub struct Funcion {
    pub nombre: String,
    pub tipo_retorno: Tipo,
    pub parametros: Vec<ParametroFuncion>,
}

/// Parámetro de función
#[derive(Debug, Clone)]
pub struct ParametroFuncion {
    pub nombre: String,
    pub tipo: Tipo,
    pub mutable: bool,
}

/// Información sobre un objeto/clase
#[derive(Debug, Clone)]
pub struct Objeto {
    pub nombre: String,
    pub miembros: HashMap<String, MiembroObjeto>,
}

/// Miembro de un objeto
#[derive(Debug, Clone)]
pub enum MiembroObjeto {
    Variable {
        tipo: Tipo,
        mutable: bool,
        publico: bool,
        libre: bool,
    },
    Funcion {
        tipo_retorno: Tipo,
        parametros: Vec<ParametroFuncion>,
        publico: bool,
        libre: bool,
    },
}

/// Tabla de símbolos con soporte para ámbitos anidados
pub struct TablaSimbolos {
    variables: Vec<HashMap<String, Variable>>,
    funciones: Vec<HashMap<String, Funcion>>,
    objetos: Vec<HashMap<String, Objeto>>,
}

impl TablaSimbolos {
    /// Crea una nueva tabla de símbolos
    pub fn nueva() -> Self {
        Self {
            variables: vec![HashMap::new()],
            funciones: vec![HashMap::new()],
            objetos: vec![HashMap::new()],
        }
    }
    
    /// Entra en un nuevo ámbito
    pub fn entrar_ambito(&mut self) {
        self.variables.push(HashMap::new());
        self.funciones.push(HashMap::new());
        self.objetos.push(HashMap::new());
    }
    
    /// Sale del ámbito actual
    pub fn salir_ambito(&mut self) {
        if self.variables.len() > 1 {
            self.variables.pop();
        }
        if self.funciones.len() > 1 {
            self.funciones.pop();
        }
        if self.objetos.len() > 1 {
            self.objetos.pop();
        }
    }
    
    /// Declara una variable
    pub fn declarar_variable(&mut self, nombre: String, tipo: Tipo, mutable: bool) -> Result<(), String> {
        let ambito_actual = self.variables.last_mut().unwrap();
        
        if ambito_actual.contains_key(&nombre) {
            return Err(format!("variable '{}' ya está declarada", nombre));
        }
        
        ambito_actual.insert(nombre.clone(), Variable {
            nombre,
            tipo,
            mutable,
            inicializada: false,
        });
        
        Ok(())
    }
    
    /// Busca una variable en todos los ámbitos
    pub fn buscar_variable(&self, nombre: &str) -> Option<&Variable> {
        for ambito in self.variables.iter().rev() {
            if let Some(variable) = ambito.get(nombre) {
                return Some(variable);
            }
        }
        None
    }
    
    /// Declara una función
    pub fn declarar_funcion(&mut self, nombre: String, tipo_retorno: Tipo, parametros: Vec<ParametroFuncion>) -> Result<(), String> {
        let ambito_actual = self.funciones.last_mut().unwrap();
        
        if ambito_actual.contains_key(&nombre) {
            return Err(format!("función '{}' ya está declarada", nombre));
        }
        
        ambito_actual.insert(nombre.clone(), Funcion {
            nombre,
            tipo_retorno,
            parametros,
        });
        
        Ok(())
    }
    
    /// Busca una función
    pub fn buscar_funcion(&self, nombre: &str) -> Option<&Funcion> {
        for ambito in self.funciones.iter().rev() {
            if let Some(funcion) = ambito.get(nombre) {
                return Some(funcion);
            }
        }
        None
    }
    
    /// Declara un objeto
    pub fn declarar_objeto(&mut self, nombre: String, miembros: HashMap<String, MiembroObjeto>) -> Result<(), String> {
        let ambito_actual = self.objetos.last_mut().unwrap();
        
        if ambito_actual.contains_key(&nombre) {
            return Err(format!("objeto '{}' ya está declarado", nombre));
        }
        
        ambito_actual.insert(nombre.clone(), Objeto {
            nombre,
            miembros,
        });
        
        Ok(())
    }
    
    /// Busca un objeto
    pub fn buscar_objeto(&self, nombre: &str) -> Option<&Objeto> {
        for ambito in self.objetos.iter().rev() {
            if let Some(objeto) = ambito.get(nombre) {
                return Some(objeto);
            }
        }
        None
    }
}
