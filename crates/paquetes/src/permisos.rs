//! Permisos declarados en `quetzal.json` (seguro por defecto).
//!
//! Acepta dos formas:
//! - Objeto: `{"red": {"habilitado": true}, "sistema_archivos": {...}}`
//! - Lista (esquema original): `[{"tipo": "red", "habilitado": true}, ...]`

use nucleo::{CategoriaError, ErrorQuetzal, ResultadoQuetzal};

use crate::manifiesto::normalizar_clave;

/// Permisos del proyecto; todo deshabilitado por defecto.
#[derive(Debug, Clone, Default)]
pub struct Permisos {
    pub red: PermisoRed,
    pub sistema_archivos: PermisoSistemaArchivos,
    pub ejecucion: PermisoEjecucion,
}

/// Acceso a la red: un único interruptor.
///
/// `{"habilitado": true}` autoriza todas las operaciones de red del
/// programa (conectarse como cliente y escuchar como servidor).
#[derive(Debug, Clone, Default)]
pub struct PermisoRed {
    pub habilitado: bool,
}

/// Acceso al sistema de archivos por directorio.
#[derive(Debug, Clone, Default)]
pub struct PermisoSistemaArchivos {
    pub habilitado: bool,
    pub directorios: Vec<DirectorioPermitido>,
}

/// Un directorio permitido con su nivel de acceso.
#[derive(Debug, Clone)]
pub struct DirectorioPermitido {
    pub ruta: String,
    pub acceso: Acceso,
}

/// Nivel de acceso a un directorio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acceso {
    Lectura,
    Escritura,
    Todo,
}

/// Ejecución de programas externos por lista blanca.
#[derive(Debug, Clone, Default)]
pub struct PermisoEjecucion {
    pub habilitado: bool,
    pub ejecutables: Vec<String>,
}

impl Permisos {
    /// Interpreta el campo `permisos` de quetzal.json.
    pub fn desde_json(valor: &serde_json::Value) -> ResultadoQuetzal<Permisos> {
        match valor {
            serde_json::Value::Object(mapa) => {
                let mut permisos = Permisos::default();
                for (clave, valor) in mapa {
                    permisos.aplicar(&normalizar_clave(clave), valor)?;
                }
                Ok(permisos)
            }
            serde_json::Value::Array(lista) => {
                let mut permisos = Permisos::default();
                for entrada in lista {
                    let tipo = entrada
                        .get("tipo")
                        .and_then(|tipo| tipo.as_str())
                        .ok_or_else(|| error_permisos("cada permiso necesita un campo 'tipo'"))?;
                    permisos.aplicar(&normalizar_clave(tipo), entrada)?;
                }
                Ok(permisos)
            }
            serde_json::Value::Null => Ok(Permisos::default()),
            _ => Err(error_permisos(
                "el campo 'permisos' debe ser un objeto o una lista",
            )),
        }
    }

    fn aplicar(&mut self, tipo: &str, valor: &serde_json::Value) -> ResultadoQuetzal<()> {
        let habilitado = valor
            .get("habilitado")
            .and_then(|habilitado| habilitado.as_bool())
            .unwrap_or(false);
        match tipo {
            "red" => {
                self.red.habilitado = habilitado;
            }
            "sistema_archivos" => {
                self.sistema_archivos.habilitado = habilitado;
                if let Some(serde_json::Value::Array(directorios)) = valor.get("directorios") {
                    for directorio in directorios {
                        let ruta = directorio
                            .get("ruta")
                            .and_then(|ruta| ruta.as_str())
                            .ok_or_else(|| {
                                error_permisos("cada directorio permitido necesita 'ruta'")
                            })?;
                        let acceso = directorio
                            .get("permiso")
                            .and_then(|permiso| permiso.as_str())
                            .unwrap_or("lectura");
                        let acceso = match normalizar_clave(acceso).as_str() {
                            "lectura" => Acceso::Lectura,
                            "escritura" => Acceso::Escritura,
                            "todo" => Acceso::Todo,
                            otro => {
                                return Err(error_permisos(format!(
                                    "permiso de directorio desconocido: '{otro}' (usa lectura, escritura o todo)"
                                )));
                            }
                        };
                        self.sistema_archivos.directorios.push(DirectorioPermitido {
                            ruta: ruta.to_string(),
                            acceso,
                        });
                    }
                }
            }
            "ejecucion" => {
                self.ejecucion.habilitado = habilitado;
                if let Some(serde_json::Value::Array(ejecutables)) = valor.get("ejecutables") {
                    for ejecutable in ejecutables {
                        if let Some(ruta) = ejecutable.as_str() {
                            self.ejecucion.ejecutables.push(ruta.to_string());
                        }
                    }
                }
            }
            otro => {
                return Err(error_permisos(format!(
                    "permiso desconocido: '{otro}' (usa red, sistema_archivos o ejecucion)"
                )));
            }
        }
        Ok(())
    }
}

fn error_permisos(mensaje: impl Into<String>) -> ErrorQuetzal {
    ErrorQuetzal::nuevo("E0601", CategoriaError::Paquetes, mensaje)
        .con_ayuda("ejemplo: \"permisos\": {\"red\": {\"habilitado\": false}}")
}
