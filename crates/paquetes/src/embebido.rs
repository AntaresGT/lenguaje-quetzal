//! Carga embebida al final de un ejecutable (`quetzal compilar`).
//!
//! Formato, de atrás hacia adelante:
//! magia (8) · longitud del payload (u64 LE) · sha256 (32) · payload postcard.
//! Todo con `std::fs`; cero `unsafe`.

use bytecode::{ModuloCompilado, verificar_modulo_inmutable};
use nucleo::{CategoriaError, ErrorQuetzal, ResultadoQuetzal};
use sha2::{Digest, Sha256};

use crate::permisos::Permisos;

/// Marca de 8 bytes al final del binario compilado.
pub const MAGIA: [u8; 8] = *b"QZEXE01\n";

/// Grafo de módulos ya verificados y los permisos del `quetzal.json`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CargaEmbebida {
    /// Clave del módulo de entrada (ruta relativa al proyecto o nombre).
    pub entrada: String,
    /// Módulos en orden de carga (dependencias primero).
    pub modulos: Vec<(String, ModuloCompilado)>,
    pub permisos: Permisos,
}

/// Anexa una carga verificada a una copia del binario anfitrión.
pub fn anexar(anfitrion: &[u8], carga: &CargaEmbebida) -> ResultadoQuetzal<Vec<u8>> {
    for (_, modulo) in &carga.modulos {
        verificar_modulo_inmutable(modulo).map_err(|error| {
            error_embebido(format!("bytecode inválido al embeber: {error}"))
        })?;
    }
    let payload = postcard::to_allocvec(carga).map_err(|error| {
        error_embebido(format!("no se pudo serializar la carga: {error}"))
    })?;
    let hash = Sha256::digest(&payload);
    let mut salida =
        Vec::with_capacity(anfitrion.len() + payload.len() + hash.len() + 8 + MAGIA.len());
    salida.extend_from_slice(anfitrion);
    salida.extend_from_slice(&payload);
    salida.extend_from_slice(&hash);
    salida.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    salida.extend_from_slice(&MAGIA);
    Ok(salida)
}

/// Lee una carga al final de un binario. `None` si no hay magia.
pub fn extraer(binario: &[u8]) -> ResultadoQuetzal<Option<CargaEmbebida>> {
    const COLA: usize = 8 + 8 + 32;
    if binario.len() < COLA {
        return Ok(None);
    }
    let magia = &binario[binario.len() - 8..];
    if magia != MAGIA {
        return Ok(None);
    }
    let len_bytes: [u8; 8] = binario[binario.len() - 16..binario.len() - 8]
        .try_into()
        .map_err(|_| error_embebido("cola de ejecutable corrupta"))?;
    let len = u64::from_le_bytes(len_bytes) as usize;
    if binario.len() < COLA + len {
        return Err(error_embebido(
            "la carga embebida declara un tamaño mayor al archivo",
        ));
    }
    let hash_fin = binario.len() - 16;
    let hash_inicio = hash_fin - 32;
    let payload_inicio = hash_inicio - len;
    let payload = &binario[payload_inicio..hash_inicio];
    let hash_almacenado = &binario[hash_inicio..hash_fin];
    let hash = Sha256::digest(payload);
    if hash.as_slice() != hash_almacenado {
        return Err(error_embebido(
            "el hash sha2 de la carga embebida no coincide (archivo manipulado)",
        ));
    }
    let carga: CargaEmbebida = postcard::from_bytes(payload).map_err(|error| {
        error_embebido(format!("no se pudo leer la carga embebida: {error}"))
    })?;
    for (_, modulo) in &carga.modulos {
        verificar_modulo_inmutable(modulo).map_err(|error| {
            error_embebido(format!("bytecode embebido inválido: {error}"))
        })?;
    }
    Ok(Some(carga))
}

/// Detecta carga embebida en el ejecutable actual.
///
/// Solo lee la cola (magia + longitudes) primero; si no hay magia, no carga
/// el binario entero (el `quetzal` de desarrollo no lleva payload).
pub fn desde_ejecutable_actual() -> ResultadoQuetzal<Option<CargaEmbebida>> {
    use std::io::{Read, Seek, SeekFrom};

    const COLA: u64 = 8 + 8 + 32;
    let ruta = std::env::current_exe().map_err(|error| {
        error_embebido(format!("no se pudo leer el ejecutable actual: {error}"))
    })?;
    let mut archivo = std::fs::File::open(&ruta).map_err(|error| {
        error_embebido(format!(
            "no se pudo abrir '{}': {error}",
            ruta.display()
        ))
    })?;
    let total = archivo.metadata().map_err(|error| {
        error_embebido(format!("no se pudo medir '{}': {error}", ruta.display()))
    })?
    .len();
    if total < COLA {
        return Ok(None);
    }
    archivo
        .seek(SeekFrom::End(-8))
        .map_err(|error| error_embebido(format!("no se pudo leer la magia: {error}")))?;
    let mut magia = [0u8; 8];
    archivo
        .read_exact(&mut magia)
        .map_err(|error| error_embebido(format!("no se pudo leer la magia: {error}")))?;
    if magia != MAGIA {
        return Ok(None);
    }
    let bytes = std::fs::read(&ruta).map_err(|error| {
        error_embebido(format!(
            "no se pudo leer '{}': {error}",
            ruta.display()
        ))
    })?;
    extraer(&bytes)
}

fn error_embebido(mensaje: impl Into<String>) -> ErrorQuetzal {
    ErrorQuetzal::nuevo("E0411", CategoriaError::Interno, mensaje)
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use bytecode::ModuloCompilado;

    #[test]
    fn anexar_y_extraer_deberia_redondear() {
        let carga = CargaEmbebida {
            entrada: "principal".into(),
            modulos: vec![("principal".into(), ModuloCompilado::default())],
            permisos: Permisos::default(),
        };
        let binario = anexar(b"HOST", &carga).expect("anexar");
        assert!(binario.starts_with(b"HOST"));
        let leida = extraer(&binario).expect("extraer").expect("carga");
        assert_eq!(leida.entrada, "principal");
        assert_eq!(leida.modulos.len(), 1);
    }

    #[test]
    fn extraer_sin_magia_deberia_devolver_none() {
        assert!(extraer(b"solo un binario normal").expect("ok").is_none());
    }
}
