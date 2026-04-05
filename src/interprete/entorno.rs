use crate::interprete::valores::Valor;
use crate::modulos::CargadorModulos;
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
    definiciones_objetos: HashMap<String, DefinicionObjeto>,
    archivo_actual: Option<String>,
    permisos: Option<SistemaPermisos>,
    cargador_modulos: CargadorModulos,
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

/// Definición de un objeto (clase) con información de herencia
#[derive(Debug, Clone)]
pub struct DefinicionObjeto {
    pub nombre: String,
    pub padres: Vec<String>,
    pub miembros: Vec<crate::nucleo::sintactico::ast::MiembroObjetoAst>,
    pub constructor: Option<crate::nucleo::sintactico::ast::NodoAst>,
}

impl Entorno {
    /// Crea un nuevo entorno
    pub fn nuevo() -> Self {
        Self {
            variables: vec![HashMap::new()],
            funciones: vec![HashMap::new()],
            objetos: vec![HashMap::new()],
            objetos_nativos: HashMap::new(),
            definiciones_objetos: HashMap::new(),
            archivo_actual: None,
            permisos: Some(SistemaPermisos::nuevo()),
            cargador_modulos: CargadorModulos::nuevo(),
        }
    }
    
    /// Crea un nuevo entorno con archivo actual
    pub fn con_archivo(archivo: String) -> Self {
        let cargador_modulos = CargadorModulos::desde_archivo_actual(std::path::Path::new(&archivo));
        Self {
            variables: vec![HashMap::new()],
            funciones: vec![HashMap::new()],
            objetos: vec![HashMap::new()],
            objetos_nativos: HashMap::new(),
            definiciones_objetos: HashMap::new(),
            archivo_actual: Some(archivo),
            permisos: Some(SistemaPermisos::nuevo()),
            cargador_modulos,
        }
    }
    
    /// Obtiene el archivo actual
    pub fn archivo_actual(&self) -> Option<String> {
        self.archivo_actual.clone()
    }
    
    /// Establece el archivo actual
    pub fn establecer_archivo(&mut self, archivo: String) {
        self.cargador_modulos = CargadorModulos::desde_archivo_actual(std::path::Path::new(&archivo));
        self.archivo_actual = Some(archivo);
    }

    pub fn establecer_cargador_modulos(&mut self, cargador: CargadorModulos) {
        self.cargador_modulos = cargador;
    }

    pub fn cargador_modulos(&self) -> CargadorModulos {
        self.cargador_modulos.clone()
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
            SistemaPermisos::nuevo().puede_acceder_archivo(ruta)
        }
    }
    
    /// Verifica si se tiene permiso para acceder a la red
    pub fn verificar_permiso_red(&self) -> crate::errores::Resultado<()> {
        if let Some(permisos) = &self.permisos {
            permisos.puede_acceder_red()
        } else {
            SistemaPermisos::nuevo().puede_acceder_red()
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
    
    /// Define una nueva variable en el ámbito base (nivel 1, por encima del global)
    /// Útil para variables ambiente.X que deben persistir a través de bloques anidados
    pub fn definir_variable_en_base(&mut self, nombre: String, valor: Valor, mutable: bool) -> Result<(), String> {
        // Buscar si la variable ya existe en algún ámbito
        for ambito in self.variables.iter_mut().rev() {
            if ambito.contains_key(&nombre) {
                ambito.insert(nombre, Variable { valor, mutable });
                return Ok(());
            }
        }
        
        // Si no existe, crear en el ámbito justo por encima del global (índice 1)
        // Esto asegura que persista durante toda la construcción del objeto
        let indice = if self.variables.len() > 1 { 1 } else { 0 };
        
        if let Some(ambito) = self.variables.get_mut(indice) {
            ambito.insert(nombre, Variable { valor, mutable });
            Ok(())
        } else {
            Err("no hay ámbito disponible".to_string())
        }
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
    
    /// Registra una definición de objeto (clase)
    pub fn registrar_definicion_objeto(&mut self, definicion: DefinicionObjeto) {
        self.definiciones_objetos.insert(definicion.nombre.clone(), definicion);
    }
    
    /// Obtiene una definición de objeto (clase)
    pub fn obtener_definicion_objeto(&self, nombre: &str) -> Option<&DefinicionObjeto> {
        self.definiciones_objetos.get(nombre)
    }
    
    /// Obtiene todas las variables que comienzan con un prefijo específico
    pub fn obtener_variables_con_prefijo(&self, prefijo: &str) -> Vec<(String, Valor)> {
        let mut resultado = Vec::new();
        for ambito in self.variables.iter().rev() {
            for (nombre, variable) in ambito {
                if nombre.starts_with(prefijo) {
                    resultado.push((nombre.clone(), variable.valor.clone()));
                }
            }
        }
        resultado
    }
}
