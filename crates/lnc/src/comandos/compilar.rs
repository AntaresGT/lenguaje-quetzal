//! Comando `quetzal compilar <entrada> -o app`.
//!
//! Copia el binario `quetzal` y anexa bytecode verificado + permisos.

use std::path::Path;

use motor::{ConfiguracionMotor, MotorQuetzal};
use paquetes::CargaEmbebida;

use super::ejecutar::reportar_errores;

/// Compila un archivo o proyecto a un ejecutable único.
pub fn compilar(entrada: &str, salida: Option<&str>) -> i32 {
    let motor = match MotorQuetzal::nuevo(ConfiguracionMotor::por_defecto()) {
        Ok(motor) => motor,
        Err(error) => return reportar_errores(None, &[error]),
    };
    let carga = match motor.compilar_carga(entrada) {
        Ok(carga) => carga,
        Err(errores) => return reportar_errores(Some(&motor), &errores),
    };
    let destino = salida.map(Path::new).map(Path::to_path_buf).unwrap_or_else(|| {
        let origen = Path::new(entrada);
        let nombre = origen
            .file_stem()
            .and_then(|nombre| nombre.to_str())
            .unwrap_or("app");
        Path::new(nombre).to_path_buf()
    });
    match escribir_ejecutable(&carga, &destino) {
        Ok(()) => {
            println!("compilado: {}", destino.display());
            0
        }
        Err(error) => reportar_errores(Some(&motor), &[error]),
    }
}

/// Arranca una carga detectada en el propio binario.
pub fn ejecutar_embebido(carga: CargaEmbebida) -> i32 {
    let motor = match MotorQuetzal::nuevo(ConfiguracionMotor::por_defecto()) {
        Ok(motor) => motor,
        Err(error) => return reportar_errores(None, &[error]),
    };
    match motor.ejecutar_carga(&carga) {
        Ok(()) => 0,
        Err(errores) => reportar_errores(Some(&motor), &errores),
    }
}

fn escribir_ejecutable(
    carga: &CargaEmbebida,
    destino: &Path,
) -> Result<(), nucleo::ErrorQuetzal> {
    let anfitrion = std::env::current_exe().map_err(|error| {
        nucleo::ErrorQuetzal::nuevo(
            "E0411",
            nucleo::CategoriaError::Interno,
            format!("no se pudo ubicar el binario quetzal: {error}"),
        )
    })?;
    let bytes = std::fs::read(&anfitrion).map_err(|error| {
        nucleo::ErrorQuetzal::nuevo(
            "E0411",
            nucleo::CategoriaError::Interno,
            format!("no se pudo leer '{}': {error}", anfitrion.display()),
        )
    })?;
    let combinado = paquetes::embebido::anexar(&bytes, carga)?;
    if let Some(padre) = destino.parent()
        && !padre.as_os_str().is_empty()
    {
        std::fs::create_dir_all(padre).map_err(|error| {
            nucleo::ErrorQuetzal::nuevo(
                "E0411",
                nucleo::CategoriaError::Interno,
                format!("no se pudo crear '{}': {error}", padre.display()),
            )
        })?;
    }
    std::fs::write(destino, combinado).map_err(|error| {
        nucleo::ErrorQuetzal::nuevo(
            "E0411",
            nucleo::CategoriaError::Interno,
            format!("no se pudo escribir '{}': {error}", destino.display()),
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permisos = std::fs::metadata(destino)
            .map_err(|error| {
                nucleo::ErrorQuetzal::nuevo(
                    "E0411",
                    nucleo::CategoriaError::Interno,
                    format!("no se pudo leer permisos de '{}': {error}", destino.display()),
                )
            })?
            .permissions();
        permisos.set_mode(permisos.mode() | 0o111);
        std::fs::set_permissions(destino, permisos).map_err(|error| {
            nucleo::ErrorQuetzal::nuevo(
                "E0411",
                nucleo::CategoriaError::Interno,
                format!(
                    "no se pudo marcar ejecutable '{}': {error}",
                    destino.display()
                ),
            )
        })?;
    }
    Ok(())
}
