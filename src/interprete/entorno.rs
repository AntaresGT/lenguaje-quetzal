use crate::interprete::valores::Valor;
use crate::nativos::interfaz::ModuloNativo;
use crate::configuracion::permisos::SistemaPermisos;
use std::collections::HashMap;
use std::sync::Arc;

/// Entorno de ejecución con stack de ámbitos
pub struct Entorno {
    variables: Vec<HashMap<String, Variable>>,
    funciones: Vec<HashMap<String, Funcion>>,
    objetos: Vec<HashMap<String, Objeto>>,
    objetos_nativos: HashMap<String, Arc<dyn ModuloNativo>>,
    archivo_actual: Option<String>,
    permisos: Option<SistemaPermisos>,
}

/// Variable en el entorno
#[derive(Debug, Clone)]
struct Variable {
    valor: Valor,
    mutable: bool,
}

/// Función en el entorno
#[derive(Debug, Clone)]
pub struct Funcion {
    pub nombre: String,
    pub parametros: Vec<String>,
    pub cuerpo: Box<crate::nucleo::sintactico::ast::NodoAst>,
}

/// Objeto en el entorno
#[derive(Debug, Clone)]
pub struct Objeto {
    pub nombre: String,
    pub propiedades: HashMap<String, Valor>,
}

impl Entorno {
    /// Crea un nuevo entorno
    pub fn nuevo() -> Self {
        Self {
            variables: vec![HashMap::new()],
            funciones: vec![HashMap::new()],
            objetos: vec![HashMap::new()],
            objetos_nativos: HashMap::new(),
            archivo_actual: None,
            permisos: None,
        }
    }
    
    /// Crea un nuevo entorno con archivo actual
    pub fn con_archivo(archivo: String) -> Self {
        Self {
            variables: vec![HashMap::new()],
            funciones: vec![HashMap::new()],
            objetos: vec![HashMap::new()],
            objetos_nativos: HashMap::new(),
            archivo_actual: Some(archivo),
            permisos: None,
        }
    }
    
    /// Obtiene el archivo actual
    pub fn archivo_actual(&self) -> Option<String> {
        self.archivo_actual.clone()
    }
    
    /// Establece el archivo actual
    pub fn establecer_archivo(&mut self, archivo: String) {
        self.archivo_actual = Some(archivo);
    }
    
    /// Establece el sistema de permisos
    pub fn establecer_permisos(&mut self, permisos: SistemaPermisos) {
        self.permisos = Some(permisos);
    }
    
    /// Obtiene el sistema de permisos
    pub fn permisos(&self) -> Option<&SistemaPermisos> {
        self.permisos.as_ref()
    }
    
    /// Verifica si se tiene permiso para acceder a un archivo
    pub fn verificar_permiso_archivo(&self, ruta: &std::path::PathBuf) -> crate::errores::Resultado<()> {
        if let Some(permisos) = &self.permisos {
            permisos.puede_acceder_archivo(ruta)
        } else {
            // Sin sistema de permisos configurado, permitir todo por defecto
            Ok(())
        }
    }
    
    /// Verifica si se tiene permiso para acceder a la red
    pub fn verificar_permiso_red(&self) -> crate::errores::Resultado<()> {
        if let Some(permisos) = &self.permisos {
            permisos.puede_acceder_red()
        } else {
            // Sin sistema de permisos configurado, permitir todo por defecto
            Ok(())
        }
    }
    
    /// Obtiene todas las variables del ámbito global
    pub fn obtener_variables_globales(&self) -> HashMap<String, Valor> {
        if let Some(ambito_global) = self.variables.first() {
            ambito_global
                .iter()
                .map(|(k, v)| (k.clone(), v.valor.clone()))
                .collect()
        } else {
            HashMap::new()
        }
    }
    
    /// Registra un objeto nativo global
    pub fn registrar_objeto_nativo(&mut self, nombre: String, modulo: Arc<dyn ModuloNativo>) {
        self.objetos_nativos.insert(nombre, modulo);
    }
    
    /// Obtiene un objeto nativo global
    pub fn obtener_objeto_nativo(&self, nombre: &str) -> Option<&Arc<dyn ModuloNativo>> {
        self.objetos_nativos.get(nombre)
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
    
    /// Define una variable
    pub fn definir_variable(&mut self, nombre: String, valor: Valor, mutable: bool) -> Result<(), String> {
        let ambito_actual = self.variables.last_mut().unwrap();
        
        if ambito_actual.contains_key(&nombre) {
            return Err(format!("variable '{}' ya está definida", nombre));
        }
        
        ambito_actual.insert(nombre, Variable { valor, mutable });
        Ok(())
    }
    
    /// Asigna un valor a una variable existente
    pub fn asignar_variable(&mut self, nombre: &str, valor: Valor) -> Result<(), String> {
        for ambito in self.variables.iter_mut().rev() {
            if let Some(variable) = ambito.get_mut(nombre) {
                if !variable.mutable {
                    return Err(format!("variable '{}' no es mutable", nombre));
                }
                variable.valor = valor;
                return Ok(());
            }
        }
        
        Err(format!("variable '{}' no está definida", nombre))
    }
    
    /// Obtiene el valor de una variable
    pub fn obtener_variable(&self, nombre: &str) -> Option<&Valor> {
        for ambito in self.variables.iter().rev() {
            if let Some(variable) = ambito.get(nombre) {
                return Some(&variable.valor);
            }
        }
        None
    }
    
    /// Obtiene información sobre una variable (valor y mutabilidad)
    pub fn obtener_info_variable(&self, nombre: &str) -> Option<(&Valor, bool)> {
        for ambito in self.variables.iter().rev() {
            if let Some(variable) = ambito.get(nombre) {
                return Some((&variable.valor, variable.mutable));
            }
        }
        None
    }
    
    /// Define una función
    pub fn definir_funcion(&mut self, nombre: String, parametros: Vec<String>, cuerpo: Box<crate::nucleo::sintactico::ast::NodoAst>) -> Result<(), String> {
        let ambito_actual = self.funciones.last_mut().unwrap();
        
        if ambito_actual.contains_key(&nombre) {
            return Err(format!("función '{}' ya está definida", nombre));
        }
        
        ambito_actual.insert(nombre.clone(), Funcion {
            nombre,
            parametros,
            cuerpo,
        });
        
        Ok(())
    }
    
    /// Obtiene una función
    pub fn obtener_funcion(&self, nombre: &str) -> Option<&Funcion> {
        for ambito in self.funciones.iter().rev() {
            if let Some(funcion) = ambito.get(nombre) {
                return Some(funcion);
            }
        }
        None
    }
    
    /// Define un objeto
    pub fn definir_objeto(&mut self, nombre: String, propiedades: HashMap<String, Valor>) -> Result<(), String> {
        let ambito_actual = self.objetos.last_mut().unwrap();
        
        if ambito_actual.contains_key(&nombre) {
            return Err(format!("objeto '{}' ya está definido", nombre));
        }
        
        ambito_actual.insert(nombre.clone(), Objeto {
            nombre,
            propiedades,
        });
        
        Ok(())
    }
    
    /// Obtiene un objeto
    pub fn obtener_objeto(&self, nombre: &str) -> Option<&Objeto> {
        for ambito in self.objetos.iter().rev() {
            if let Some(objeto) = ambito.get(nombre) {
                return Some(objeto);
            }
        }
        None
    }
}
