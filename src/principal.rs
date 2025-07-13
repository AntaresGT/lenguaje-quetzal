
// Intérprete del Lenguaje Quetzal v0.0.2
// Desarrollado en Rust para máximo rendimiento
// Autor: Desarrollado siguiendo las especificaciones del lenguaje Quetzal

use std::env;
use std::fs;
use std::path::Path;
use colored::Colorize;

// Módulos del intérprete
mod interprete;
mod analizador_lexico;
mod analizador_sintactico;
mod tipos_datos;
mod evaluador;
mod errores;
mod consola;

// Módulo de pruebas
#[cfg(test)]
mod pruebas;

const VERSION: &str = "0.0.2";

/// Función principal del intérprete Quetzal
fn main() {
    // Configurar codificación UTF-8 en Windows
    #[cfg(windows)]
    {
        // Habilitar soporte UTF-8 en la consola de Windows
        unsafe {
            use std::ffi::c_void;
            extern "system" {
                fn SetConsoleOutputCP(wCodePageID: u32) -> i32;
                fn SetConsoleCP(wCodePageID: u32) -> i32;
            }
            SetConsoleOutputCP(65001); // UTF-8
            SetConsoleCP(65001); // UTF-8
        }
    }
    
    let argumentos: Vec<String> = env::args().collect();
    
    // Si no hay argumentos o se pide ayuda
    if argumentos.len() <= 1 {
        mostrar_ayuda(&argumentos[0]);
        return;
    }
    
    match argumentos[1].as_str() {
        "--ayuda" | "-h" => mostrar_ayuda(&argumentos[0]),
        "--version" | "-v" => mostrar_version(),
        archivo => ejecutar_archivo(archivo),
    }
}

/// Muestra la versión del intérprete
fn mostrar_version() {
    println!("{}", format!("Quetzal v{}", VERSION).green().bold());
    println!("Un lenguaje de programación en español");
}

/// Muestra la ayuda del intérprete
fn mostrar_ayuda(_programa: &str) {
    println!("{}", "USO:".cyan().bold());
    println!("    quetzal <archivo.qz>");
    println!("    quetzal --version");
    println!("    quetzal --ayuda");
    println!();
    println!("{}", "OPCIONES:".cyan().bold());
    println!("    --version       Muestra la versión del intérprete");
    println!("    --ayuda         Muestra esta información de ayuda");
    println!();
    println!("{}", "EJEMPLOS:".cyan().bold());
    println!("    quetzal programa.qz");
    println!("    quetzal directorio/ejemplo.qz");
}

/// Ejecuta un archivo .qz
fn ejecutar_archivo(ruta_archivo: &str) {
    // Verificar que el archivo tenga extensión .qz
    if !ruta_archivo.ends_with(".qz") {
        eprintln!("{}", "Error: El archivo debe tener extensión .qz".red());
        return;
    }
    
    // Verificar que el archivo existe
    if !Path::new(ruta_archivo).exists() {
        eprintln!("{}", format!("Error: No se pudo encontrar el archivo '{}'", ruta_archivo).red());
        return;
    }
    
    // Leer el contenido del archivo
    match fs::read_to_string(ruta_archivo) {
        Ok(contenido) => {
            // Interpretar el código
            match interprete::interpretar(&contenido) {
                Ok(_) => {
                    // Ejecución exitosa
                },
                Err(error) => {
                    eprintln!("{}", format!("Error de ejecución: {}", error).red());
                }
            }
        },
        Err(error) => {
            eprintln!("{}", format!("Error al leer el archivo: {}", error).red());
        }
    }
}