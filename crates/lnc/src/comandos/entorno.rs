//! Comando `quetzal entorno`.

use std::fs;
use std::path::Path;

use motor::VERSION_QUETZAL;

/// Muestra la versión requerida por el proyecto y la versión instalada.
///
/// La primera versión del comando es deliberadamente de solo lectura: ayuda
/// a detectar incompatibilidades sin descargar ni modificar nada.
pub fn ejecutar(accion: Option<&str>) -> i32 {
    match accion.unwrap_or("mostrar") {
        "mostrar" | "info" => mostrar(),
        "verificar" => verificar(),
        otro => {
            eprintln!("acción desconocida '{otro}'; usa 'mostrar' o 'verificar'");
            2
        }
    }
}

fn mostrar() -> i32 {
    let requerida = leer_version_requerida();
    println!("Versión instalada: {VERSION_QUETZAL}");
    match requerida {
        Some(version) => println!("Versión del proyecto: {version}"),
        None => println!("Versión del proyecto: no especificada"),
    }
    0
}

fn verificar() -> i32 {
    let Some(requerida) = leer_version_requerida() else {
        println!("No hay un archivo .quetzal-version en el directorio actual.");
        return 0;
    };
    if requerida == VERSION_QUETZAL || requerida == format!("v{VERSION_QUETZAL}") {
        println!("✓ El entorno coincide con Quetzal {requerida}");
        0
    } else {
        eprintln!(
            "✗ El proyecto requiere Quetzal {requerida}, pero está instalada la versión {VERSION_QUETZAL}"
        );
        1
    }
}

fn leer_version_requerida() -> Option<String> {
    let ruta = Path::new(".quetzal-version");
    fs::read_to_string(ruta)
        .ok()
        .map(|version| version.trim().to_string())
        .filter(|version| !version.is_empty())
}
