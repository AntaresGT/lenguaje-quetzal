//! Lectura y validación de `quetzal.json`.
//!
//! Acepta las claves con tilde del prompt original pero normaliza
//! internamente: `versión`→`version`, `aplicación`→`aplicacion`,
//! `ejecución`→`ejecucion`, `sistema-archivos`→`sistema_archivos`.

use std::path::Path;

use indexmap::IndexMap;
use nucleo::{CategoriaError, ErrorQuetzal, ResultadoQuetzal};

use crate::permisos::Permisos;

/// Contenido validado de un `quetzal.json`.
#[derive(Debug, Clone)]
pub struct Manifiesto {
    pub version: String,
    pub aplicacion: String,
    pub entrada: String,
    /// `aplicacion` o `biblioteca` (normalizado sin tilde).
    pub tipo: String,
    /// Versión del lenguaje que requiere el proyecto.
    pub quetzal: String,
    pub autor: Option<String>,
    /// Dependencias: nombre → versión o `ruta:./...`.
    pub dependencias: IndexMap<String, String>,
    pub permisos: Permisos,
}

impl Manifiesto {
    /// Lee y valida el `quetzal.json` de un directorio.
    pub fn leer_de_directorio(directorio: &Path) -> ResultadoQuetzal<Manifiesto> {
        Self::leer_archivo(&directorio.join("quetzal.json"))
    }

    pub fn leer_archivo(ruta: &Path) -> ResultadoQuetzal<Manifiesto> {
        let contenido = std::fs::read_to_string(ruta).map_err(|error| {
            error_manifiesto(format!("no se pudo leer '{}': {error}", ruta.display()))
        })?;
        Self::leer_texto(&contenido)
            .map_err(|error| error.con_archivo(ruta.display().to_string().replace('\\', "/")))
    }

    /// Valida el contenido JSON del manifiesto.
    pub fn leer_texto(contenido: &str) -> ResultadoQuetzal<Manifiesto> {
        let json: serde_json::Value = serde_json::from_str(contenido).map_err(|error| {
            error_manifiesto(format!("quetzal.json no es JSON válido: {error}"))
        })?;
        let serde_json::Value::Object(mapa) = json else {
            return Err(error_manifiesto("quetzal.json debe ser un objeto JSON"));
        };

        // Normalización de claves con tilde.
        let mapa: serde_json::Map<String, serde_json::Value> = mapa
            .into_iter()
            .map(|(clave, valor)| (normalizar_clave(&clave), valor))
            .collect();

        let version = campo_texto(&mapa, "version")?;
        validar_version(&version, "version")?;
        let quetzal = campo_texto(&mapa, "quetzal")?;
        validar_version(&quetzal, "quetzal")?;
        let aplicacion = campo_texto(&mapa, "aplicacion")?;
        let entrada = campo_texto(&mapa, "entrada")?;
        let tipo = normalizar_clave(&campo_texto(&mapa, "tipo")?);
        if tipo != "aplicacion" && tipo != "biblioteca" {
            return Err(error_manifiesto(format!(
                "el campo 'tipo' debe ser 'aplicacion' o 'biblioteca', no '{tipo}'"
            )));
        }

        let mut dependencias = IndexMap::new();
        if let Some(valor) = mapa.get("dependencias") {
            let serde_json::Value::Object(objeto) = valor else {
                return Err(error_manifiesto(
                    "el campo 'dependencias' debe ser un objeto",
                ));
            };
            for (nombre, version) in objeto {
                let serde_json::Value::String(version) = version else {
                    return Err(error_manifiesto(format!(
                        "la dependencia '{nombre}' debe declarar una versión o ruta como texto"
                    )));
                };
                dependencias.insert(nombre.clone(), version.clone());
            }
        }

        let permisos = match mapa.get("permisos") {
            Some(valor) => Permisos::desde_json(valor)?,
            None => Permisos::default(),
        };

        Ok(Manifiesto {
            version,
            aplicacion,
            entrada,
            tipo,
            quetzal,
            autor: mapa
                .get("autor")
                .and_then(|valor| valor.as_str())
                .map(str::to_string),
            dependencias,
            permisos,
        })
    }
}

/// Normaliza una clave: quita tildes y cambia `-` por `_`.
pub fn normalizar_clave(clave: &str) -> String {
    clave
        .chars()
        .map(|caracter| match caracter {
            'á' | 'à' => 'a',
            'é' | 'è' => 'e',
            'í' | 'ì' => 'i',
            'ó' | 'ò' => 'o',
            'ú' | 'ù' => 'u',
            'Á' => 'A',
            'É' => 'E',
            'Í' => 'I',
            'Ó' => 'O',
            'Ú' => 'U',
            '-' => '_',
            otro => otro,
        })
        .collect()
}

fn campo_texto(
    mapa: &serde_json::Map<String, serde_json::Value>,
    campo: &str,
) -> ResultadoQuetzal<String> {
    match mapa.get(campo) {
        Some(serde_json::Value::String(texto)) if !texto.is_empty() => Ok(texto.clone()),
        Some(_) => Err(error_manifiesto(format!(
            "el campo '{campo}' debe ser un texto no vacío"
        ))),
        None => Err(error_manifiesto(format!(
            "falta el campo obligatorio '{campo}'"
        ))),
    }
}

fn validar_version(version: &str, campo: &str) -> ResultadoQuetzal<()> {
    let valida = version.split('.').count() == 3
        && version
            .split('.')
            .all(|parte| !parte.is_empty() && parte.chars().all(|c| c.is_ascii_digit()));
    if valida {
        Ok(())
    } else {
        Err(error_manifiesto(format!(
            "el campo '{campo}' debe tener formato X.Y.Z, no '{version}'"
        )))
    }
}

fn error_manifiesto(mensaje: impl Into<String>) -> ErrorQuetzal {
    ErrorQuetzal::nuevo("E0601", CategoriaError::Paquetes, mensaje).con_ayuda(
        "revisa el formato de quetzal.json (campos: version, aplicacion, entrada, tipo, quetzal)",
    )
}
