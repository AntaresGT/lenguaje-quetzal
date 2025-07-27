// Manejador de módulos para el lenguaje Quetzal
// Gestiona la importación y exportación de módulos entre archivos

use crate::analizador_lexico::AnalizadorLexico;
use crate::analizador_sintactico::{AnalizadorSintactico, Nodo, ElementoImportar};
use crate::evaluador::{Evaluador, Entorno, FuncionDefinida};
use crate::errores::{ErrorQuetzal, ResultadoQuetzal};
use crate::tipos_datos::Variable;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use std::rc::Rc;
use std::cell::RefCell;

/// Elemento exportado de un módulo (puede ser variable o función)
#[derive(Debug, Clone)]
pub enum ElementoExportado {
    Variable(Variable),
    Funcion(FuncionDefinida),
}

/// Estructura que maneja el sistema de módulos
#[derive(Debug)]
pub struct ManejadorModulos {
    /// Caché de módulos ya cargados para evitar cargas múltiples
    modulos_cargados: HashMap<String, ModuloInfo>,
    /// Ruta del archivo principal (punto de entrada)
    ruta_principal: PathBuf,
    /// Entorno global compartido entre módulos
    entorno_global: Rc<RefCell<Entorno>>,
}

/// Información de un módulo cargado
#[derive(Debug, Clone)]
pub struct ModuloInfo {
    /// Ruta absoluta del módulo
    pub ruta: PathBuf,
    /// Variables y funciones exportadas por el módulo
    pub exportaciones: HashMap<String, ElementoExportado>,
    /// AST del módulo (para debug y análisis)
    pub ast: Nodo,
}

impl ManejadorModulos {
    /// Crea un nuevo manejador de módulos
    pub fn nuevo(ruta_principal: &str, entorno_global: Rc<RefCell<Entorno>>) -> ResultadoQuetzal<Self> {
        let ruta_principal = Path::new(ruta_principal)
            .canonicalize()
            .map_err(|error| ErrorQuetzal::ErrorInterno {
                mensaje: format!("No se pudo resolver la ruta principal '{}': {}", ruta_principal, error),
            })?;
        
        Ok(ManejadorModulos {
            modulos_cargados: HashMap::new(),
            ruta_principal,
            entorno_global,
        })
    }
    
    /// Procesa una declaración de importación
    pub fn procesar_importacion(
        &mut self,
        elementos: &[ElementoImportar],
        ruta_modulo: &str,
        evaluador: &mut Evaluador,
        linea: usize,
    ) -> ResultadoQuetzal<()> {
        // Resolver la ruta del módulo
        let ruta_absoluta = self.resolver_ruta_modulo(ruta_modulo, linea)?;
        
        // Cargar el módulo si no está en caché
        let ruta_str = ruta_absoluta.to_string_lossy().to_string();
        
        if !self.modulos_cargados.contains_key(&ruta_str) {
            self.cargar_modulo(&ruta_absoluta, evaluador, linea)?;
        }
        
        // Obtener las exportaciones del módulo
        let modulo = self.modulos_cargados
            .get(&ruta_str)
            .ok_or_else(|| ErrorQuetzal::ErrorCargaModulo {
                linea,
                ruta: ruta_modulo.to_string(),
                detalle: "El módulo no se encuentra en el caché después de ser cargado".to_string(),
            })?;
        
        // Importar los elementos solicitados
        for elemento in elementos {
            let nombre_real = &elemento.nombre;
            let nombre_local = elemento.alias.as_ref().unwrap_or(nombre_real);
            
            if let Some(elemento_exportado) = modulo.exportaciones.get(nombre_real) {
                // Agregar el elemento al entorno actual del evaluador
                match elemento_exportado {
                    ElementoExportado::Variable(variable) => {
                        evaluador.definir_variable_global(nombre_local.clone(), variable.clone())?;
                    },
                    ElementoExportado::Funcion(funcion) => {
                        evaluador.definir_funcion_global(nombre_local.clone(), funcion.clone())?;
                    },
                }
            } else {
                // Crear sugerencia de elementos disponibles
                let elementos_disponibles: Vec<String> = modulo.exportaciones.keys().cloned().collect();
                let sugerencia = if elementos_disponibles.is_empty() {
                    "El módulo no exporta ningún elemento. Verifica las declaraciones 'exportar' en el módulo.".to_string()
                } else if elementos_disponibles.len() <= 5 {
                    format!("Elementos disponibles: {}", elementos_disponibles.join(", "))
                } else {
                    format!("Elementos disponibles: {} y {} más", 
                        elementos_disponibles[..3].join(", "), 
                        elementos_disponibles.len() - 3)
                };
                
                return Err(ErrorQuetzal::ElementoNoExportado {
                    linea,
                    elemento: nombre_real.clone(),
                    modulo: ruta_modulo.to_string(),
                    sugerencia,
                });
            }
        }
        
        Ok(())
    }
    
    /// Procesa una declaración de exportación
    pub fn procesar_exportacion(
        &mut self,
        elementos: &[String],
        evaluador: &Evaluador,
        ruta_actual: &str,
        linea: usize,
    ) -> ResultadoQuetzal<()> {
        let mut exportaciones = HashMap::new();
        
        // Obtener todas las variables definidas en el entorno actual
        let variables_definidas = evaluador.obtener_todas_las_variables();
        
        // Verificar que todos los elementos a exportar existan en el entorno actual
        for nombre in elementos {
            // Intentar obtener como variable primero
            if let Some(variable) = evaluador.obtener_variable(nombre) {
                exportaciones.insert(nombre.clone(), ElementoExportado::Variable(variable));
            }
            // Si no es una variable, intentar obtener como función
            else if let Some(funcion) = evaluador.obtener_funcion(nombre) {
                exportaciones.insert(nombre.clone(), ElementoExportado::Funcion(funcion));
            }
            // Si no es ni variable ni función, dar error
            else {
                // Crear sugerencia con variables similares o disponibles
                let mut sugerencias = Vec::new();
                
                // Buscar variables con nombres similares
                for var_nombre in &variables_definidas {
                    if var_nombre.contains(nombre) || nombre.contains(var_nombre) {
                        sugerencias.push(var_nombre.clone());
                    }
                }
                
                let sugerencia = if sugerencias.is_empty() {
                    if variables_definidas.is_empty() {
                        "No hay variables ni funciones definidas en este contexto. Define el elemento antes de exportarlo.".to_string()
                    } else if variables_definidas.len() <= 5 {
                        format!("Variables disponibles: {}", variables_definidas.join(", "))
                    } else {
                        format!("Variables disponibles: {} y {} más", 
                            variables_definidas[..3].join(", "), 
                            variables_definidas.len() - 3)
                    }
                } else {
                    format!("¿Quizás quisiste decir: {}?", sugerencias.join(", "))
                };
                
                return Err(ErrorQuetzal::ErrorExportacion {
                    linea,
                    elemento: nombre.clone(),
                    sugerencia,
                });
            }
        }
        
        // Resolver la ruta actual
        let ruta_absoluta = Path::new(ruta_actual)
            .canonicalize()
            .map_err(|error| ErrorQuetzal::ErrorCargaModulo {
                linea,
                ruta: ruta_actual.to_string(),
                detalle: format!("No se pudo resolver la ruta del módulo: {}", error),
            })?;
        
        // Actualizar las exportaciones del módulo
        if let Some(modulo) = self.modulos_cargados.get_mut(&ruta_absoluta.to_string_lossy().to_string()) {
            modulo.exportaciones.extend(exportaciones);
        }
        
        Ok(())
    }
    
    /// Resuelve la ruta de un módulo relativa al punto de entrada
    fn resolver_ruta_modulo(&self, ruta_modulo: &str, linea: usize) -> ResultadoQuetzal<PathBuf> {
        // Validaciones básicas de la ruta
        if ruta_modulo.is_empty() {
            return Err(ErrorQuetzal::RutaModuloInvalida {
                linea,
                ruta: ruta_modulo.to_string(),
                razon: "La ruta del módulo no puede estar vacía".to_string(),
            });
        }
        
        // Verificar caracteres inválidos
        if ruta_modulo.contains("..") {
            return Err(ErrorQuetzal::RutaModuloInvalida {
                linea,
                ruta: ruta_modulo.to_string(),
                razon: "No se permite el uso de '..' en las rutas de módulos por seguridad".to_string(),
            });
        }
        
        // Si la ruta es absoluta, usarla directamente
        if Path::new(ruta_modulo).is_absolute() {
            let ruta = Path::new(ruta_modulo);
            if ruta.exists() {
                if ruta.extension() != Some(std::ffi::OsStr::new("qz")) {
                    return Err(ErrorQuetzal::RutaModuloInvalida {
                        linea,
                        ruta: ruta_modulo.to_string(),
                        razon: "Los archivos de módulos deben tener extensión .qz".to_string(),
                    });
                }
                return Ok(ruta.to_path_buf());
            } else {
                return Err(ErrorQuetzal::ModuloNoEncontrado {
                    linea,
                    ruta: ruta_modulo.to_string(),
                    detalle: "El archivo no existe en la ruta especificada".to_string(),
                });
            }
        }
        
        // Si la ruta es relativa, buscarla desde el directorio del archivo principal
        let directorio_principal = self.ruta_principal
            .parent()
            .ok_or_else(|| ErrorQuetzal::ErrorCargaModulo {
                linea,
                ruta: ruta_modulo.to_string(),
                detalle: "No se pudo obtener el directorio del archivo principal".to_string(),
            })?;
        
        let ruta_candidata = directorio_principal.join(ruta_modulo);
        
        if ruta_candidata.exists() {
            if ruta_candidata.extension() != Some(std::ffi::OsStr::new("qz")) {
                return Err(ErrorQuetzal::RutaModuloInvalida {
                    linea,
                    ruta: ruta_modulo.to_string(),
                    razon: "Los archivos de módulos deben tener extensión .qz".to_string(),
                });
            }
            
            ruta_candidata.canonicalize().map_err(|error| ErrorQuetzal::ErrorCargaModulo {
                linea,
                ruta: ruta_modulo.to_string(),
                detalle: format!("No se pudo resolver la ruta canónica: {}", error),
            })
        } else {
            // Generar sugerencias de archivos .qz cercanos
            let mut archivos_cercanos = Vec::new();
            if let Ok(entradas) = fs::read_dir(directorio_principal) {
                for entrada in entradas.flatten() {
                    if let Some(extension) = entrada.path().extension() {
                        if extension == "qz" {
                            if let Some(nombre) = entrada.file_name().to_str() {
                                archivos_cercanos.push(nombre.to_string());
                            }
                        }
                    }
                }
            }
            
            let detalle = if archivos_cercanos.is_empty() {
                format!("No se encontraron archivos .qz en el directorio '{}'", 
                    directorio_principal.display())
            } else if archivos_cercanos.len() <= 5 {
                format!("Archivos .qz disponibles en el directorio: {}", 
                    archivos_cercanos.join(", "))
            } else {
                format!("Archivos .qz disponibles: {} y {} más", 
                    archivos_cercanos[..3].join(", "), 
                    archivos_cercanos.len() - 3)
            };
            
            Err(ErrorQuetzal::ModuloNoEncontrado {
                linea,
                ruta: ruta_modulo.to_string(),
                detalle,
            })
        }
    }
    
    /// Carga y evalúa un módulo
    fn cargar_modulo(&mut self, ruta: &Path, evaluador: &mut Evaluador, linea: usize) -> ResultadoQuetzal<()> {
        // Verificar que la ruta tenga extensión .qz
        if ruta.extension() != Some(std::ffi::OsStr::new("qz")) {
            return Err(ErrorQuetzal::RutaModuloInvalida {
                linea,
                ruta: ruta.display().to_string(),
                razon: "Los módulos deben tener extensión .qz".to_string(),
            });
        }
        
        // Leer el contenido del archivo
        let codigo = fs::read_to_string(ruta)
            .map_err(|error| {
                let detalle = match error.kind() {
                    std::io::ErrorKind::NotFound => "El archivo no existe".to_string(),
                    std::io::ErrorKind::PermissionDenied => "Sin permisos para leer el archivo".to_string(),
                    std::io::ErrorKind::InvalidData => "El archivo contiene datos inválidos o no es UTF-8".to_string(),
                    _ => format!("Error de E/S: {}", error),
                };
                
                ErrorQuetzal::ErrorCargaModulo {
                    linea,
                    ruta: ruta.display().to_string(),
                    detalle,
                }
            })?;
        
        // Verificar que el archivo no esté vacío
        if codigo.trim().is_empty() {
            return Err(ErrorQuetzal::ErrorCargaModulo {
                linea,
                ruta: ruta.display().to_string(),
                detalle: "El archivo del módulo está vacío".to_string(),
            });
        }
        
        // Análisis léxico
        let mut analizador_lexico = AnalizadorLexico::nuevo(&codigo);
        let tokens = analizador_lexico.analizar()
            .map_err(|error| ErrorQuetzal::ErrorCargaModulo {
                linea,
                ruta: ruta.display().to_string(),
                detalle: format!("Error en análisis léxico: {}", error),
            })?;
        
        // Filtrar tokens de nueva línea
        let tokens_filtrados: Vec<_> = tokens
            .into_iter()
            .filter(|token| !matches!(token.tipo, crate::analizador_lexico::TipoToken::NuevaLinea))
            .collect();
        
        // Verificar que hay tokens válidos después del filtrado
        if tokens_filtrados.is_empty() {
            return Err(ErrorQuetzal::ErrorCargaModulo {
                linea,
                ruta: ruta.display().to_string(),
                detalle: "El módulo no contiene código válido".to_string(),
            });
        }
        
        // Análisis sintáctico
        let mut analizador_sintactico = AnalizadorSintactico::nuevo(tokens_filtrados);
        let ast = analizador_sintactico.analizar()
            .map_err(|error| ErrorQuetzal::ErrorCargaModulo {
                linea,
                ruta: ruta.display().to_string(),
                detalle: format!("Error en análisis sintáctico: {}", error),
            })?;
        
        // Crear un entorno hijo para el módulo
        let entorno_modulo = Entorno::nuevo_hijo(self.entorno_global.clone());
        let entorno_anterior = evaluador.intercambiar_entorno(Rc::new(RefCell::new(entorno_modulo)));
        
        // Establecer la ruta del archivo actual para las exportaciones
        let ruta_anterior = evaluador.obtener_ruta_archivo_actual().map(|s| s.to_string());
        evaluador.establecer_ruta_archivo_actual(&ruta.to_string_lossy());
        
        // Evaluar el módulo
        let resultado_evaluacion = evaluador.evaluar(&ast);
        
        // Procesar resultado y manejar errores
        let exportaciones_encontradas = match resultado_evaluacion {
            Ok(_) => {
                // Procesar las exportaciones ANTES de restaurar el entorno
                let mut exportaciones = HashMap::new();
                match self.recopilar_exportaciones_del_ast(&ast, evaluador, ruta, &mut exportaciones) {
                    Ok(_) => exportaciones,
                    Err(error) => {
                        // Restaurar entorno y ruta antes de retornar error
                        if let Some(ruta_anterior) = ruta_anterior {
                            evaluador.establecer_ruta_archivo_actual(&ruta_anterior);
                        }
                        evaluador.intercambiar_entorno(entorno_anterior);
                        return Err(error);
                    }
                }
            }
            Err(error) => {
                // Restaurar entorno y ruta antes de retornar error
                if let Some(ruta_anterior) = ruta_anterior {
                    evaluador.establecer_ruta_archivo_actual(&ruta_anterior);
                }
                evaluador.intercambiar_entorno(entorno_anterior);
                return Err(ErrorQuetzal::ErrorCargaModulo {
                    linea,
                    ruta: ruta.display().to_string(),
                    detalle: format!("Error durante la evaluación del módulo: {}", error),
                });
            }
        };
        
        // Restaurar la ruta anterior
        if let Some(ruta_anterior) = ruta_anterior {
            evaluador.establecer_ruta_archivo_actual(&ruta_anterior);
        }
        
        // Restaurar el entorno anterior
        evaluador.intercambiar_entorno(entorno_anterior);
        
        // Crear información del módulo con las exportaciones recopiladas
        let modulo_info = ModuloInfo {
            ruta: ruta.to_path_buf(),
            exportaciones: exportaciones_encontradas,
            ast,
        };
        
        // Agregar al caché
        let ruta_cache = ruta.to_string_lossy().to_string();
        self.modulos_cargados.insert(ruta_cache, modulo_info);
        
        Ok(())
    }
    
    /// Procesa las exportaciones encontradas en el AST del módulo
    fn procesar_exportaciones_del_ast(
        &mut self,
        ast: &Nodo,
        evaluador: &Evaluador,
        ruta: &Path,
    ) -> ResultadoQuetzal<()> {
        self.buscar_exportaciones_en_nodo(ast, evaluador, ruta)
    }
    
    /// Recopila las exportaciones encontradas en el AST del módulo
    fn recopilar_exportaciones_del_ast(
        &mut self,
        ast: &Nodo,
        evaluador: &Evaluador,
        ruta: &Path,
        exportaciones: &mut HashMap<String, ElementoExportado>,
    ) -> ResultadoQuetzal<()> {
        self.recopilar_exportaciones_en_nodo(ast, evaluador, ruta, exportaciones)
    }
    
    /// Busca recursivamente las declaraciones de exportación en un nodo
    fn buscar_exportaciones_en_nodo(
        &mut self,
        nodo: &Nodo,
        evaluador: &Evaluador,
        ruta: &Path,
    ) -> ResultadoQuetzal<()> {
        match nodo {
            Nodo::Programa(declaraciones) => {
                for declaracion in declaraciones {
                    self.buscar_exportaciones_en_nodo(declaracion, evaluador, ruta)?;
                }
            },
            Nodo::DeclaracionExportar { elementos, linea } => {
                // Procesar esta exportación
                self.procesar_exportacion(elementos, evaluador, &ruta.to_string_lossy(), *linea)?;
            },
            // Para otros tipos de nodos, no hay nada que hacer
            _ => {},
        }
        Ok(())
    }
    
    /// Recopila recursivamente las declaraciones de exportación en un nodo
    fn recopilar_exportaciones_en_nodo(
        &mut self,
        nodo: &Nodo,
        evaluador: &Evaluador,
        ruta: &Path,
        exportaciones: &mut HashMap<String, ElementoExportado>,
    ) -> ResultadoQuetzal<()> {
        match nodo {
            Nodo::Programa(declaraciones) => {
                for declaracion in declaraciones {
                    self.recopilar_exportaciones_en_nodo(declaracion, evaluador, ruta, exportaciones)?;
                }
            },
            Nodo::DeclaracionExportar { elementos, linea } => {
                // Recopilar esta exportación
                for elemento in elementos {
                    // Intentar obtener como variable primero
                    if let Some(variable) = evaluador.obtener_variable(elemento) {
                        exportaciones.insert(elemento.clone(), ElementoExportado::Variable(variable));
                    }
                    // Si no es una variable, intentar obtener como función
                    else if let Some(funcion) = evaluador.obtener_funcion(elemento) {
                        exportaciones.insert(elemento.clone(), ElementoExportado::Funcion(funcion));
                    }
                    // Si no es ni variable ni función, dar error
                    else {
                        return Err(ErrorQuetzal::ErrorExportacion {
                            linea: *linea,
                            elemento: elemento.clone(),
                            sugerencia: format!("El elemento '{}' no está definido en el entorno actual (ni como variable ni como función)", elemento),
                        });
                    }
                }
            },
            // Para otros tipos de nodos, no hay nada que hacer
            _ => {},
        }
        Ok(())
    }
    
    /// Obtiene la lista de módulos cargados
    pub fn obtener_modulos_cargados(&self) -> Vec<&str> {
        self.modulos_cargados.keys().map(|s| s.as_str()).collect()
    }
    
    /// Verifica si un módulo está cargado
    pub fn esta_modulo_cargado(&self, ruta: &str) -> bool {
        self.modulos_cargados.contains_key(ruta)
    }
}
