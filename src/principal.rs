mod interprete;
mod valores;
mod entorno;

use std::env;
use std::fs;

fn main() {
    let argumentos: Vec<String> = env::args().collect();
    if argumentos.len() < 2 {
        eprintln!("Uso: {} <archivo.qz>", argumentos[0]);
        std::process::exit(1);
    }

    let ruta_archivo = &argumentos[1];
    let contenido = match fs::read_to_string(ruta_archivo) {
        Ok(texto) => texto,
        Err(_) => {
            eprintln!("No se pudo leer el archivo {}", ruta_archivo);
            std::process::exit(1);
        }
    };

    if let Err(error) = interprete::interpretar(&contenido) {
        eprintln!("{}", error);
        std::process::exit(1);
    }
}
