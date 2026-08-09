//! Instalación de dependencias en `.quetzal/bibliotecas/<nombre>/<version>/`.
//!
//! Esta fase resuelve dependencias locales (`ruta:./...`). Las dependencias
//! remotas (registro o git) quedan declaradas pero producen un error claro.

use std::path::{Path, PathBuf};

use nucleo::{CategoriaError, ErrorQuetzal, ResultadoQuetzal};
use sha2::{Digest, Sha256};

use crate::bloqueo::{Bloqueo, DependenciaBloqueada};
use crate::manifiesto::Manifiesto;

/// Instala las dependencias del proyecto y actualiza `quetzal.bloquear`.
///
/// Devuelve los nombres de las dependencias instaladas.
pub fn instalar_dependencias(directorio: &Path) -> ResultadoQuetzal<Vec<String>> {
    let manifiesto = Manifiesto::leer_de_directorio(directorio)?;
    let mut bloqueo = Bloqueo::nuevo();
    let mut instaladas = Vec::new();

    for (nombre, declaracion) in &manifiesto.dependencias {
        if let Some(ruta_relativa) = declaracion.strip_prefix("ruta:") {
            let (version, integridad) = instalar_local(directorio, nombre, ruta_relativa)?;
            bloqueo.dependencias.insert(
                nombre.clone(),
                DependenciaBloqueada {
                    version,
                    origen: declaracion.clone(),
                    integridad,
                },
            );
            instaladas.push(nombre.clone());
        } else {
            return Err(ErrorQuetzal::nuevo(
                "E0602",
                CategoriaError::Paquetes,
                format!(
                    "la dependencia remota '{nombre}' ({declaracion}) todavía no se puede instalar"
                ),
            )
            .con_ayuda(
                "por ahora solo se instalan dependencias locales con la forma \"ruta:./bibliotecas/mi_biblioteca\"",
            ));
        }
    }

    bloqueo.guardar(directorio)?;
    Ok(instaladas)
}

/// Copia una dependencia local dentro de `.quetzal/bibliotecas`.
fn instalar_local(
    directorio: &Path,
    nombre: &str,
    ruta_relativa: &str,
) -> ResultadoQuetzal<(String, String)> {
    let origen = directorio.join(ruta_relativa);
    if !origen.is_dir() {
        return Err(ErrorQuetzal::nuevo(
            "E0602",
            CategoriaError::Paquetes,
            format!(
                "la dependencia '{nombre}' apunta a '{}' y ese directorio no existe",
                origen.display()
            ),
        ));
    }

    // La versión sale del quetzal.json de la dependencia (o 0.0.0).
    let version = Manifiesto::leer_de_directorio(&origen)
        .map(|manifiesto| manifiesto.version)
        .unwrap_or_else(|_| "0.0.0".to_string());

    let destino = directorio
        .join(".quetzal")
        .join("bibliotecas")
        .join(nombre)
        .join(&version);
    if destino.exists() {
        std::fs::remove_dir_all(&destino).map_err(|error| {
            error_instalacion(format!(
                "no se pudo limpiar '{}': {error}",
                destino.display()
            ))
        })?;
    }
    copiar_directorio(&origen, &destino)?;

    let integridad = hash_directorio(&destino)?;
    Ok((version, integridad))
}

/// Copia recursiva, omitiendo `.quetzal` para no anidar instalaciones.
fn copiar_directorio(origen: &Path, destino: &Path) -> ResultadoQuetzal<()> {
    std::fs::create_dir_all(destino).map_err(|error| {
        error_instalacion(format!("no se pudo crear '{}': {error}", destino.display()))
    })?;
    let entradas = std::fs::read_dir(origen).map_err(|error| {
        error_instalacion(format!("no se pudo leer '{}': {error}", origen.display()))
    })?;
    for entrada in entradas {
        let entrada = entrada
            .map_err(|error| error_instalacion(format!("error al listar archivos: {error}")))?;
        let nombre = entrada.file_name();
        if nombre == ".quetzal" {
            continue;
        }
        let ruta_origen = entrada.path();
        let ruta_destino = destino.join(&nombre);
        if ruta_origen.is_dir() {
            copiar_directorio(&ruta_origen, &ruta_destino)?;
        } else {
            std::fs::copy(&ruta_origen, &ruta_destino).map_err(|error| {
                error_instalacion(format!(
                    "no se pudo copiar '{}': {error}",
                    ruta_origen.display()
                ))
            })?;
        }
    }
    Ok(())
}

/// SHA-256 estable de todos los archivos de un directorio (rutas + contenido).
pub fn hash_directorio(directorio: &Path) -> ResultadoQuetzal<String> {
    let mut archivos = Vec::new();
    recolectar_archivos(directorio, &mut archivos)?;
    archivos.sort();

    let mut hasher = Sha256::new();
    for archivo in archivos {
        let relativa = archivo
            .strip_prefix(directorio)
            .unwrap_or(&archivo)
            .display()
            .to_string()
            .replace('\\', "/");
        hasher.update(relativa.as_bytes());
        let contenido = std::fs::read(&archivo).map_err(|error| {
            error_instalacion(format!("no se pudo leer '{}': {error}", archivo.display()))
        })?;
        hasher.update(&contenido);
    }
    Ok(format!("sha256-{}", hex::encode(hasher.finalize())))
}

fn recolectar_archivos(directorio: &Path, destino: &mut Vec<PathBuf>) -> ResultadoQuetzal<()> {
    let entradas = std::fs::read_dir(directorio).map_err(|error| {
        error_instalacion(format!(
            "no se pudo leer '{}': {error}",
            directorio.display()
        ))
    })?;
    for entrada in entradas {
        let entrada = entrada
            .map_err(|error| error_instalacion(format!("error al listar archivos: {error}")))?;
        let ruta = entrada.path();
        if ruta.is_dir() {
            recolectar_archivos(&ruta, destino)?;
        } else {
            destino.push(ruta);
        }
    }
    Ok(())
}

fn error_instalacion(mensaje: impl Into<String>) -> ErrorQuetzal {
    ErrorQuetzal::nuevo("E0602", CategoriaError::Paquetes, mensaje)
}
