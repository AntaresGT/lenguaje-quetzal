use crate::errores::{Error, CodigoError, Resultado};
use crate::nucleo::sintactico::Parser;
use crate::nucleo::sintactico::ast::NodoAst;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::fs;

/// Cargador de módulos con resolución de rutas y detección de dependencias circulares
pub struct CargadorModulos {
    /// Cache de módulos ya cargados (ruta absoluta -> AST)
    modulos_cargados: HashMap<PathBuf, Vec<NodoAst>>,
    /// Elementos exportados por cada módulo (ruta absoluta -> elementos)
    exportaciones: HashMap<PathBuf, HashMap<String, NodoAst>>,
    /// Módulos actualmente en proceso de carga (para detectar ciclos)
    en_proceso: HashSet<PathBuf>,
    /// Directorio base para resolver rutas relativas
    directorio_base: Option<PathBuf>,
}

impl CargadorModulos {
    /// Crea un nuevo cargador de módulos
    pub fn nuevo() -> Self {
        Self {
            modulos_cargados: HashMap::new(),
            exportaciones: HashMap::new(),
            en_proceso: HashSet::new(),
            directorio_base: None,
        }
    }
    
    /// Crea un nuevo cargador con un directorio base específico
    pub fn con_directorio_base(directorio: PathBuf) -> Self {
        Self {
            modulos_cargados: HashMap::new(),
            exportaciones: HashMap::new(),
            en_proceso: HashSet::new(),
            directorio_base: Some(directorio),
        }
    }
    
    /// Establece el directorio base para resolver rutas relativas
    pub fn establecer_directorio_base(&mut self, directorio: PathBuf) {
        self.directorio_base = Some(directorio);
    }
    
    /// Resuelve una ruta de módulo a una ruta absoluta
    pub fn resolver_ruta(&self, ruta: &str) -> Resultado<PathBuf> {
        let ruta_con_extension = if ruta.ends_with(".qz") {
            ruta.to_string()
        } else {
            format!("{}.qz", ruta)
        };
        
        let path = Path::new(&ruta_con_extension);
        
        // Si es una ruta absoluta, usarla directamente
        if path.is_absolute() {
            return Ok(path.to_path_buf());
        }
        
        // Si hay un directorio base, resolver relativo a él
        if let Some(ref base) = self.directorio_base {
            let ruta_completa = base.join(&ruta_con_extension);
            if ruta_completa.exists() {
                return Ok(ruta_completa.canonicalize().unwrap_or(ruta_completa));
            }
        }
        
        // Intentar resolver desde el directorio actual
        let ruta_actual = PathBuf::from(&ruta_con_extension);
        if ruta_actual.exists() {
            return Ok(ruta_actual.canonicalize().unwrap_or(ruta_actual));
        }
        
        // Si no se encuentra, devolver la ruta tal como está para mejor mensaje de error
        Err(Error::modulo(
            CodigoError::ModuloNoEncontrado,
            format!("no se encontró el módulo '{}'", ruta),
            Some(ruta.to_string()),
        ))
    }
    
    /// Carga un módulo desde una ruta
    pub fn cargar_modulo(&mut self, ruta: &str) -> Resultado<Vec<NodoAst>> {
        let ruta_absoluta = self.resolver_ruta(ruta)?;
        
        // Si ya está cargado, retornar del cache
        if let Some(modulo) = self.modulos_cargados.get(&ruta_absoluta) {
            return Ok(modulo.clone());
        }
        
        // Verificar dependencia circular
        if self.en_proceso.contains(&ruta_absoluta) {
            return Err(Error::modulo(
                CodigoError::DependenciaCircular,
                format!("dependencia circular detectada al cargar '{}'", ruta),
                Some(ruta.to_string()),
            ));
        }
        
        // Marcar como en proceso
        self.en_proceso.insert(ruta_absoluta.clone());
        
        // Leer el archivo
        let contenido = fs::read_to_string(&ruta_absoluta)
            .map_err(|e| Error::modulo(
                CodigoError::ErrorCargarModulo,
                format!("no se pudo leer el archivo '{}': {}", ruta_absoluta.display(), e),
                Some(ruta.to_string()),
            ))?;
        
        // Actualizar directorio base temporalmente para resolver importaciones anidadas
        let directorio_modulo = ruta_absoluta.parent().map(|p| p.to_path_buf());
        let directorio_anterior = self.directorio_base.clone();
        
        if let Some(dir) = directorio_modulo {
            self.directorio_base = Some(dir);
        }
        
        // Parsear el contenido
        let ast = Parser::parsear(&contenido)?;
        
        // Procesar exportaciones
        let mut exports = HashMap::new();
        for nodo in &ast {
            self.procesar_exportacion(nodo, &mut exports);
        }
        
        // Restaurar directorio base
        self.directorio_base = directorio_anterior;
        
        // Quitar de en proceso
        self.en_proceso.remove(&ruta_absoluta);
        
        // Guardar en cache
        self.modulos_cargados.insert(ruta_absoluta.clone(), ast.clone());
        self.exportaciones.insert(ruta_absoluta, exports);
        
        Ok(ast)
    }
    
    /// Procesa un nodo para extraer exportaciones
    fn procesar_exportacion(&self, nodo: &NodoAst, exports: &mut HashMap<String, NodoAst>) {
        match nodo {
            NodoAst::DeclaracionFuncion { nombre, .. } => {
                // Por defecto, las funciones de nivel superior son exportadas
                exports.insert(nombre.clone(), nodo.clone());
            }
            NodoAst::DeclaracionVariable { nombre, .. } => {
                // Por defecto, las variables de nivel superior son exportadas
                exports.insert(nombre.clone(), nodo.clone());
            }
            NodoAst::DeclaracionObjeto { nombre, .. } => {
                // Por defecto, los objetos de nivel superior son exportados
                exports.insert(nombre.clone(), nodo.clone());
            }
            _ => {}
        }
    }
    
    /// Obtiene los elementos exportados de un módulo
    pub fn obtener_exportaciones(&mut self, ruta: &str) -> Resultado<HashMap<String, NodoAst>> {
        let ruta_absoluta = self.resolver_ruta(ruta)?;
        
        // Si no está cargado, cargarlo primero
        if !self.modulos_cargados.contains_key(&ruta_absoluta) {
            self.cargar_modulo(ruta)?;
        }
        
        self.exportaciones
            .get(&ruta_absoluta)
            .cloned()
            .ok_or_else(|| Error::modulo(
                CodigoError::ErrorCargarModulo,
                format!("no se pudieron obtener las exportaciones de '{}'", ruta),
                Some(ruta.to_string()),
            ))
    }
    
    /// Obtiene elementos específicos exportados de un módulo
    pub fn obtener_elementos(&mut self, ruta: &str, elementos: &[String]) -> Resultado<HashMap<String, NodoAst>> {
        let exportaciones = self.obtener_exportaciones(ruta)?;
        let mut resultado = HashMap::new();
        
        for elemento in elementos {
            if let Some(nodo) = exportaciones.get(elemento) {
                resultado.insert(elemento.clone(), nodo.clone());
            } else {
                return Err(Error::modulo(
                    CodigoError::ElementoImportadoNoEncontrado,
                    format!("'{}' no está exportado en el módulo '{}'", elemento, ruta),
                    Some(ruta.to_string()),
                ));
            }
        }
        
        Ok(resultado)
    }
    
    /// Verifica si un módulo ya está cargado
    pub fn esta_cargado(&self, ruta: &str) -> bool {
        if let Ok(ruta_absoluta) = self.resolver_ruta(ruta) {
            self.modulos_cargados.contains_key(&ruta_absoluta)
        } else {
            false
        }
    }
}
