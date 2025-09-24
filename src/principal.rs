// Intérprete del Lenguaje Quetzal v0.0.2
// Desarrollado en Rust para máximo rendimiento
// Autor: Desarrollado siguiendo las especificaciones del lenguaje Quetzal

use colored::Colorize;
use std::env;
use std::path::Path;
use std::thread;

// Organización principal del intérprete
mod analisis;
mod datos;
mod ejecucion;
mod infraestructura;
mod modulos;
mod nucleo;

// Reexportación local para facilitar el acceso desde este archivo
use nucleo::interprete;

// Módulo de pruebas
#[cfg(test)]
mod pruebas;

const VERSION: &str = "0.0.2";

/// Función principal del intérprete Quetzal
fn main() {
    // Crear un hilo con stack más grande para manejar recursión profunda
    let builder = thread::Builder::new().stack_size(16 * 1024 * 1024); // 16MB stack

    let handle = builder
        .spawn(|| ejecutar_principal())
        .expect("No se pudo crear el hilo principal");

    handle.join().expect("Error en el hilo principal");
}

fn ejecutar_principal() {
    // Configurar codificación UTF-8 en Windows
    #[cfg(windows)]
    {
        // Habilitar soporte UTF-8 en la consola de Windows
        unsafe {
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
        eprintln!(
            "{}[E0001]: el archivo debe tener extensión .qz",
            "error".red().bold()
        );
        eprintln!(" {} {}", "-->".blue().bold(), ruta_archivo);
        eprintln!(
            "  {} archivos de Quetzal deben terminar en `.qz`",
            "=".blue().bold()
        );
        return;
    }

    // Verificar que el archivo existe y convertir a ruta absoluta
    let ruta_path = Path::new(ruta_archivo);
    if !ruta_path.exists() {
        eprintln!(
            "{}[E0001]: no se pudo encontrar el archivo `{}`",
            "error".red().bold(),
            ruta_archivo
        );
        eprintln!(" {} {}", "-->".blue().bold(), ruta_archivo);
        eprintln!("  {} verifica que la ruta sea correcta", "=".blue().bold());
        return;
    }

    // Convertir a ruta absoluta para el sistema de módulos
    let ruta_absoluta = match ruta_path.canonicalize() {
        Ok(ruta) => ruta,
        Err(error) => {
            eprintln!(
                "{}[E0001]: error al resolver la ruta del archivo `{}`",
                "error".red().bold(),
                ruta_archivo
            );
            eprintln!(" {} {}", "-->".blue().bold(), ruta_archivo);
            eprintln!("  {} {}", "=".blue().bold(), error);
            return;
        }
    };

    let ruta_absoluta_str = ruta_absoluta.to_string_lossy();

    // Leer el contenido del archivo para poder pasarlo a los errores
    let codigo_fuente = match std::fs::read_to_string(&ruta_absoluta) {
        Ok(contenido) => contenido,
        Err(error) => {
            eprintln!(
                "{}[E0001]: error al leer el archivo `{}`",
                "error".red().bold(),
                ruta_archivo
            );
            eprintln!(" {} {}", "-->".blue().bold(), ruta_archivo);
            eprintln!("  {} {}", "=".blue().bold(), error);
            return;
        }
    };

    // Interpretar el archivo con soporte completo de módulos usando la ruta absoluta
    match interprete::interpretar_archivo_con_modulos(&ruta_absoluta_str) {
        Ok(_) => {
            // Ejecución exitosa
        }
        Err(error) => {
            // Mostrar error con formato estilo Rust y código fuente
            error.mostrar_error_con_codigo(Some(&codigo_fuente), Some(ruta_archivo));
        }
    }
}
