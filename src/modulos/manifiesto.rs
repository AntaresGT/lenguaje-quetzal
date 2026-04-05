use crate::configuracion::permisos::{ConfiguracionPermisos, SistemaPermisos};
use crate::modulos::esquema::validar_configuracion_quetzal;
use crate::errores::{CodigoError, Error, Resultado};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoPaquete {
    Aplicacion,
    Libreria,
}

#[derive(Debug, Clone)]
pub struct ManifiestoPaquete {
    pub nombre: String,
    pub version: String,
    pub tipo: TipoPaquete,
    pub entrada: Option<String>,
    pub biblioteca: Option<String>,
    pub dependencias: HashMap<String, String>,
    pub permisos: Option<ConfiguracionPermisos>,
    pub nativos: Vec<String>,
    pub ruta_directorio: PathBuf,
}

#[derive(Debug, Deserialize)]
struct ManifiestoCrudo {
    #[serde(default)]
    nombre: Option<String>,
    #[serde(default, alias = "versión")]
    version: Option<String>,
    #[serde(default)]
    tipo: Option<String>,
    #[serde(default)]
    entrada: Option<String>,
    #[serde(default)]
    biblioteca: Option<String>,
    #[serde(default, alias = "aplicación")]
    aplicacion: Option<String>,
    #[serde(default)]
    dependencias: HashMap<String, String>,
    #[serde(default)]
    permisos: Option<Vec<crate::configuracion::permisos::Permiso>>,
    #[serde(default)]
    nativos: Vec<String>,
}

impl ManifiestoPaquete {
    pub fn cargar_desde_archivo(ruta: &Path) -> Resultado<Self> {
        let contenido = fs::read_to_string(ruta).map_err(|e| {
            Error::sistema(
                CodigoError::ErrorLecturaArchivo,
                format!("no se pudo leer '{}': {}", ruta.display(), e),
                Some(e.to_string()),
            )
        })?;

        validar_configuracion_quetzal(ruta, &contenido)?;

        let crudo: ManifiestoCrudo = serde_json::from_str(&contenido).map_err(|e| {
            Error::analisis(
                CodigoError::SintaxisGeneral,
                format!("manifiesto inválido '{}': {}", ruta.display(), e),
                Some(ruta.display().to_string()),
                None,
                None,
            )
        })?;

        let ruta_directorio = ruta
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        let nombre = crudo
            .nombre
            .or(crudo.aplicacion)
            .unwrap_or_else(|| "quetzal-paquete".to_string());

        let version = crudo.version.unwrap_or_else(|| "0.1.0".to_string());
        let tipo = match crudo.tipo.as_deref() {
            Some("libreria") | Some("biblioteca") => TipoPaquete::Libreria,
            _ if crudo.biblioteca.is_some() => TipoPaquete::Libreria,
            _ => TipoPaquete::Aplicacion,
        };

        let permisos = crudo.permisos.map(|permisos| ConfiguracionPermisos { permisos });

        Ok(Self {
            nombre,
            version,
            tipo,
            entrada: crudo.entrada,
            biblioteca: crudo.biblioteca,
            dependencias: crudo.dependencias,
            permisos,
            nativos: crudo.nativos,
            ruta_directorio,
        })
    }

    pub fn buscar_desde_directorio_resultado(directorio: &Path) -> Resultado<Option<Self>> {
        let mut actual = Some(directorio);
        while let Some(dir) = actual {
            let ruta = dir.join("quetzal.json");
            if ruta.exists() {
                return Self::cargar_desde_archivo(&ruta).map(Some);
            }
            actual = dir.parent();
        }

        Ok(None)
    }

    pub fn buscar_desde_directorio(directorio: &Path) -> Option<Self> {
        Self::buscar_desde_directorio_resultado(directorio).ok().flatten()
    }

    pub fn punto_entrada_libreria(&self) -> Option<PathBuf> {
        let candidatos = [
            self.biblioteca.as_deref(),
            Some("aplicacion/lib.qz"),
            Some("lib.qz"),
            Some("aplicacion/principal.qz"),
            Some("principal.qz"),
        ];

        candidatos
            .iter()
            .flatten()
            .map(|ruta| self.ruta_directorio.join(ruta))
            .find(|ruta| ruta.exists())
    }

    pub fn construir_permisos(&self) -> Option<SistemaPermisos> {
        self.permisos
            .as_ref()
            .map(SistemaPermisos::cargar_desde_config)
    }

    pub fn resolver_dependencia(&self, nombre: &str) -> Resultado<Option<PathBuf>> {
        let Some(ruta) = self.dependencias.get(nombre) else {
            return Ok(None);
        };
        let base = self.ruta_directorio.join(ruta);

        if base.is_file() {
            return Ok(Some(base));
        }

        if base.is_dir() {
            if let Some(manifiesto) = Self::buscar_desde_directorio_resultado(&base)? {
                return Ok(manifiesto.punto_entrada_libreria().or(Some(base)));
            }

            for candidato in ["aplicacion/lib.qz", "lib.qz", "aplicacion/principal.qz", "principal.qz"] {
                let ruta_candidata = base.join(candidato);
                if ruta_candidata.exists() {
                    return Ok(Some(ruta_candidata));
                }
            }
        }

        Ok(Some(base))
    }
}
