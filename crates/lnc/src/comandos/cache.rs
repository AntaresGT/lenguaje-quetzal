//! Comando `quetzal cache [ACCION]`.

use paquetes::CacheBytecode;

/// Administra la cache de bytecode (`.quetzal/cache/`).
pub fn cache(accion: Option<&str>) -> i32 {
    let cache = CacheBytecode::del_proyecto(std::path::Path::new("."));
    match accion.unwrap_or("estado") {
        "limpiar" => match cache.limpiar() {
            Ok(()) => {
                println!("Cache de bytecode eliminada.");
                0
            }
            Err(error) => {
                eprintln!("No se pudo limpiar la cache: {error}");
                1
            }
        },
        "estado" => {
            let directorio = std::path::Path::new(".quetzal").join("cache");
            match std::fs::read_dir(&directorio) {
                Ok(entradas) => {
                    let total = entradas.count();
                    println!("Entradas en cache: {total}");
                }
                Err(_) => println!("No hay cache (se crea al ejecutar un proyecto)."),
            }
            0
        }
        otra => {
            eprintln!("Acción de cache desconocida: '{otra}'. Usa 'estado' o 'limpiar'.");
            1
        }
    }
}
