mod interprete;
mod valores;
mod entorno;
mod consola;
mod objetos;

use std::env;
use std::fs;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn mostrar_version() {
    println!("Quetzal v{}", VERSION);
    println!("Un lenguaje de programación en español");
}

fn mostrar_ayuda(_programa: &str) {
    println!("USO:");
    println!("    quetzal <archivo.qz>");
    println!("    quetzal --version");
    println!("    quetzal --ayuda");
    println!();
    println!("OPCIONES:");
    println!("    --version       Muestra la versión del intérprete");
    println!("    --ayuda         Muestra esta información de ayuda");
    println!();
    println!("EJEMPLOS:");
    println!("    quetzal programa.qz");
    println!("    quetzal directorio/ejemplo.qz");
}

fn main() {
    let argumentos: Vec<String> = env::args().collect();
    
    if argumentos.len() < 2 {
        eprintln!("Error: Se requiere un argumento.");
        eprintln!();
        mostrar_ayuda(&argumentos[0]);
        std::process::exit(1);
    }

    let argumento = &argumentos[1];
    
    match argumento.as_str() {
        "--version" => {
            mostrar_version();
            return;
        }
        "--ayuda" => {
            mostrar_ayuda(&argumentos[0]);
            return;
        }
        _ => {
            // Es un archivo
            if argumento.starts_with("--") {
                eprintln!("Error: Opción desconocida '{}'", argumento);
                eprintln!();
                mostrar_ayuda(&argumentos[0]);
                std::process::exit(1);
            }
        }
    }

    let ruta_archivo = &argumentos[1];
    let contenido = match fs::read_to_string(ruta_archivo) {
        Ok(texto) => texto,
        Err(error) => {
            eprintln!("Error: No se pudo leer el archivo '{}': {}", ruta_archivo, error);
            std::process::exit(1);
        }
    };

    if let Err(error) = interprete::interpretar(&contenido) {
        eprintln!("{}", error);
        std::process::exit(1);
    }
}
