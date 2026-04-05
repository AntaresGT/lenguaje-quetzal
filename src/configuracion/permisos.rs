use crate::errores::{Error, CodigoError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuración de permisos
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfiguracionPermisos {
    pub permisos: Vec<Permiso>,
}

/// Permiso individual
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permiso {
    pub tipo: String,
    pub habilitado: bool,
    pub alcance: Option<AlcancePermiso>,
    pub directorios: Option<Vec<String>>,
}

/// Alcance configurado para un permiso.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AlcancePermiso {
    Texto(String),
    Operaciones(Vec<String>),
}

/// Sistema de permisos
pub struct SistemaPermisos {
    permisos_archivos: bool,
    permisos_red: bool,
    directorios_permitidos: Vec<PathBuf>,
}

impl SistemaPermisos {
    /// Crea un sistema de permisos con configuración por defecto (segura)
    pub fn nuevo() -> Self {
        Self {
            permisos_archivos: false,
            permisos_red: false,
            directorios_permitidos: Vec::new(),
        }
    }
    
    /// Carga permisos desde quetzal.json
    pub fn cargar_desde_config(config: &ConfiguracionPermisos) -> Self {
        let mut sistema = Self::nuevo();
        
        for permiso in &config.permisos {
            match permiso.tipo.as_str() {
                "sistema-archivos" if permiso.habilitado => {
                    sistema.permisos_archivos = true;
                    if let Some(dirs) = &permiso.directorios {
                        sistema.directorios_permitidos = dirs.iter()
                            .map(|d| PathBuf::from(d))
                            .collect();
                    }
                }
                "red" if permiso.habilitado => {
                    sistema.permisos_red = true;
                }
                _ => {}
            }
        }
        
        sistema
    }
    
    /// Verifica si se puede acceder a un archivo
    pub fn puede_acceder_archivo(&self, ruta: &PathBuf) -> Result<(), Error> {
        if !self.permisos_archivos {
            return Err(Error::sistema(
                CodigoError::ViolacionSeguridad,
                "acceso a archivos no permitido",
                Some("configura permisos en quetzal.json".to_string()),
            ));
        }
        
        if !self.directorios_permitidos.is_empty() {
            let permitido = self.directorios_permitidos.iter()
                .any(|dir| ruta.starts_with(dir));
            
            if !permitido {
                return Err(Error::sistema(
                    CodigoError::ViolacionSeguridad,
                    format!("acceso a '{}' no permitido", ruta.display()),
                    Some("el directorio no está en la lista de permitidos".to_string()),
                ));
            }
        }
        
        Ok(())
    }
    
    /// Verifica si se puede acceder a la red
    pub fn puede_acceder_red(&self) -> Result<(), Error> {
        if !self.permisos_red {
            Err(Error::sistema(
                CodigoError::ViolacionSeguridad,
                "acceso a red no permitido",
                Some("configura permisos en quetzal.json".to_string()),
            ))
        } else {
            Ok(())
        }
    }
}
