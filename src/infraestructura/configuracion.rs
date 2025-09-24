//! Capa de configuración y permisos para el intérprete Quetzal.
//!
//! Este módulo se encarga de leer el archivo `quetzal.json` (si existe)
//! y traducirlo a una estructura interna que representa los permisos de
//! ejecución concedidos al programa. La filosofía por defecto es de cero
//! permisos: si no se encuentra el archivo o si un permiso no se declara
//! explícitamente, el intérprete asumirá que la acción está prohibida.

use crate::infraestructura::errores::{ErrorQuetzal, ResultadoQuetzal};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Representa el alcance permitido para el acceso al sistema de archivos.
#[derive(Debug, Clone)]
pub enum AlcanceSistemaArchivos {
    /// No existe permiso alguno para interactuar con el sistema de archivos.
    Denegado,
    /// El intérprete puede acceder a cualquier ruta del sistema.
    Todo,
    /// Solo se puede acceder a los directorios listados (y su contenido).
    Directorios(Vec<PathBuf>),
}

/// Permisos resultantes luego de analizar `quetzal.json`.
#[derive(Debug, Clone)]
pub struct PermisosEjecucion {
    permite_red: bool,
    sistema_archivos: AlcanceSistemaArchivos,
}

impl PermisosEjecucion {
    /// Construye un conjunto de permisos sin privilegios.
    pub fn sin_permisos() -> Self {
        PermisosEjecucion {
            permite_red: false,
            sistema_archivos: AlcanceSistemaArchivos::Denegado,
        }
    }

    /// Carga los permisos tomando como referencia la ruta principal del programa.
    pub fn desde_configuracion(ruta_principal: &str) -> ResultadoQuetzal<Self> {
        let ruta_programa = Path::new(ruta_principal);
        let ruta_canonica = ruta_programa
            .canonicalize()
            .unwrap_or_else(|_| ruta_programa.to_path_buf());
        let directorio_base = ruta_canonica
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        let ruta_config = directorio_base.join("quetzal.json");

        if !ruta_config.exists() {
            return Ok(Self::sin_permisos());
        }

        let contenido =
            fs::read_to_string(&ruta_config).map_err(|error| ErrorQuetzal::ErrorInterno {
                mensaje: format!(
                    "No se pudo leer el archivo de configuración `{}`: {}",
                    ruta_config.display(),
                    error
                ),
            })?;

        let datos: ArchivoConfiguracion =
            serde_json::from_str(&contenido).map_err(|error| ErrorQuetzal::ErrorInterno {
                mensaje: format!(
                    "No se pudo interpretar el archivo de configuración `{}`: {}",
                    ruta_config.display(),
                    error
                ),
            })?;

        Self::interpretar_permisos(&datos, &directorio_base)
    }

    /// Indica si la red está habilitada.
    pub fn permite_red(&self) -> bool {
        self.permite_red
    }

    /// Devuelve el alcance actual del sistema de archivos.
    pub fn sistema_archivos(&self) -> &AlcanceSistemaArchivos {
        &self.sistema_archivos
    }

    /// Verifica que exista permiso general para usar el sistema de archivos.
    pub fn verificar_uso_sistema_archivos(
        &self,
        linea: usize,
        accion: &str,
    ) -> ResultadoQuetzal<()> {
        match &self.sistema_archivos {
            AlcanceSistemaArchivos::Denegado => Err(ErrorQuetzal::PermisoDenegado {
                linea,
                detalle: format!(
                    "No hay permisos para {} porque el acceso al sistema de archivos está deshabilitado en quetzal.json.",
                    accion
                ),
            }),
            _ => Ok(()),
        }
    }

    /// Verifica que una ruta concreta se encuentre dentro de los permisos concedidos.
    pub fn verificar_acceso_a_ruta(
        &self,
        ruta: &Path,
        linea: usize,
        accion: &str,
    ) -> ResultadoQuetzal<()> {
        self.verificar_uso_sistema_archivos(linea, accion)?;

        match &self.sistema_archivos {
            AlcanceSistemaArchivos::Denegado => unreachable!(),
            AlcanceSistemaArchivos::Todo => Ok(()),
            AlcanceSistemaArchivos::Directorios(directorios) => {
                let permitido = directorios
                    .iter()
                    .any(|permitido| ruta.starts_with(permitido));

                if permitido {
                    Ok(())
                } else {
                    Err(ErrorQuetzal::PermisoDenegado {
                        linea,
                        detalle: format!(
                            "No hay permisos para {} en `{}`. Actualiza las rutas autorizadas en quetzal.json.",
                            accion,
                            ruta.display()
                        ),
                    })
                }
            }
        }
    }

    /// Determina si se puede inspeccionar un directorio sin producir error.
    pub fn puede_listar_directorio(&self, ruta: &Path) -> bool {
        match &self.sistema_archivos {
            AlcanceSistemaArchivos::Denegado => false,
            AlcanceSistemaArchivos::Todo => true,
            AlcanceSistemaArchivos::Directorios(directorios) => directorios
                .iter()
                .any(|permitido| ruta.starts_with(permitido)),
        }
    }

    /// Traduce el archivo de configuración a permisos efectivos.
    fn interpretar_permisos(
        datos: &ArchivoConfiguracion,
        directorio_base: &Path,
    ) -> ResultadoQuetzal<Self> {
        let mut permite_red = false;
        let mut acceso_total = false;
        let mut directorios_permitidos: Vec<PathBuf> = Vec::new();

        for permiso in &datos.permisos {
            match permiso {
                EntradaPermiso::Simple(nombre) => {
                    let nombre_normalizado = nombre.to_lowercase();
                    if nombre_normalizado == "red" {
                        permite_red = true;
                    } else if nombre_normalizado == "sistema-archivos" {
                        acceso_total = true;
                    }
                }
                EntradaPermiso::Detallado(detalle) => {
                    let nombre_normalizado = detalle.nombre.to_lowercase();
                    if nombre_normalizado == "red" {
                        permite_red = true;
                        continue;
                    }

                    if nombre_normalizado != "sistema-archivos" {
                        continue;
                    }

                    let alcance = detalle
                        .alcance
                        .as_deref()
                        .unwrap_or("directorios")
                        .to_lowercase();

                    match alcance.as_str() {
                        "todo" => {
                            acceso_total = true;
                        }
                        "directorio" | "directorios" => {
                            if detalle.directorios.is_empty() {
                                return Err(ErrorQuetzal::ErrorInterno {
                                    mensaje: "El permiso 'sistema-archivos' con alcance de directorios requiere la lista 'directorios' en quetzal.json.".to_string(),
                                });
                            }

                            for ruta_texto in &detalle.directorios {
                                let ruta = Self::resolver_directorio(directorio_base, ruta_texto)?;
                                if !directorios_permitidos
                                    .iter()
                                    .any(|existente| existente == &ruta)
                                {
                                    directorios_permitidos.push(ruta);
                                }
                            }
                        }
                        otro => {
                            return Err(ErrorQuetzal::ErrorInterno {
                                mensaje: format!(
                                    "Alcance desconocido '{}' para 'sistema-archivos' en quetzal.json. Usa 'todo' o 'directorios'.",
                                    otro
                                ),
                            });
                        }
                    }
                }
            }
        }

        let sistema_archivos = if acceso_total {
            AlcanceSistemaArchivos::Todo
        } else if !directorios_permitidos.is_empty() {
            AlcanceSistemaArchivos::Directorios(directorios_permitidos)
        } else {
            AlcanceSistemaArchivos::Denegado
        };

        Ok(PermisosEjecucion {
            permite_red,
            sistema_archivos,
        })
    }

    /// Convierte el texto de un directorio en una ruta absoluta canonizada.
    fn resolver_directorio(base: &Path, entrada: &str) -> ResultadoQuetzal<PathBuf> {
        let ruta_base = if Path::new(entrada).is_absolute() {
            PathBuf::from(entrada)
        } else {
            base.join(entrada)
        };

        ruta_base
            .canonicalize()
            .map_err(|error| ErrorQuetzal::ErrorInterno {
                mensaje: format!(
                    "No se pudo resolver el directorio permitido '{}': {}",
                    entrada, error
                ),
            })
    }
}

/// Representación del archivo `quetzal.json` utilizada durante la deserialización.
#[derive(Debug, Deserialize)]
struct ArchivoConfiguracion {
    #[serde(rename = "versión")]
    #[allow(dead_code)]
    version: Option<String>,
    #[serde(rename = "aplicación")]
    #[allow(dead_code)]
    aplicacion: Option<String>,
    #[serde(default)]
    permisos: Vec<EntradaPermiso>,
}

/// Permite interpretar tanto permisos simples como estructuras detalladas.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum EntradaPermiso {
    /// Permiso representado únicamente por su nombre (por ejemplo "red").
    Simple(String),
    /// Permiso con información adicional.
    Detallado(PermisoDetallado),
}

/// Estructura detallada de un permiso dentro de `permisos`.
#[derive(Debug, Deserialize)]
struct PermisoDetallado {
    nombre: String,
    #[serde(default)]
    alcance: Option<String>,
    #[serde(default)]
    #[serde(alias = "rutas")]
    directorios: Vec<String>,
}
