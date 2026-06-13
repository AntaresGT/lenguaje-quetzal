//! Definición de los argumentos de la CLI `quetzal`.

use clap::{Parser, Subcommand};

/// CLI del Lenguaje Quetzal.
#[derive(Debug, Parser)]
#[command(
    name = "quetzal",
    version,
    disable_help_flag = true,
    disable_help_subcommand = true,
    disable_version_flag = true,
    about = "Lenguaje Quetzal: lenguaje de programación interpretado en español"
)]
pub struct Cli {
    #[command(subcommand)]
    pub comando: Option<Comando>,
}

#[derive(Debug, Subcommand)]
pub enum Comando {
    /// Ejecuta un archivo o proyecto Quetzal.
    Ejecutar { archivo: Option<String> },
    /// Analiza el código sin ejecutarlo.
    Revisar { archivo: Option<String> },
    /// Crea un nuevo proyecto.
    Nuevo { nombre: String },
    /// Instala dependencias del proyecto.
    Instalar { paquete: Option<String> },
    /// Administra la cache de bytecode y análisis.
    Cache { accion: Option<String> },
    /// Muestra la versión.
    Version,
    /// Muestra esta ayuda.
    Ayuda,
}

/// Traduce los aliases tipo flag a subcomandos antes de pasar por clap.
///
/// Aliases aceptados por compatibilidad:
/// - `quetzal archivo.qz`            → `quetzal ejecutar archivo.qz`
/// - `quetzal --version | --versión | -v` → `quetzal version`
/// - `quetzal --ayuda | -a`          → `quetzal ayuda`
/// - `quetzal --nuevo NOMBRE`        → `quetzal nuevo NOMBRE`
/// - `quetzal --instalar | -i [PAQ]` → `quetzal instalar [PAQ]`
pub fn traducir_aliases(mut argumentos: Vec<String>) -> Vec<String> {
    if argumentos.len() < 2 {
        return argumentos;
    }

    let primero = argumentos[1].clone();
    match primero.as_str() {
        "--version" | "--versión" | "-v" | "version" | "versión" => {
            argumentos[1] = "version".to_string();
        }
        "--ayuda" | "-a" | "--help" | "-h" => {
            argumentos[1] = "ayuda".to_string();
        }
        "--nuevo" => {
            argumentos[1] = "nuevo".to_string();
        }
        "--instalar" | "-i" => {
            argumentos[1] = "instalar".to_string();
        }
        _ => {
            // Un archivo .qz como primer argumento ejecuta directamente.
            if primero.ends_with(".qz") {
                argumentos.insert(1, "ejecutar".to_string());
            }
        }
    }

    argumentos
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn args(lista: &[&str]) -> Vec<String> {
        lista.iter().map(|texto| texto.to_string()).collect()
    }

    #[test]
    fn traducir_aliases_deberia_convertir_archivo_qz_en_ejecutar() {
        let resultado = traducir_aliases(args(&["quetzal", "programa.qz"]));
        assert_eq!(resultado, args(&["quetzal", "ejecutar", "programa.qz"]));
    }

    #[test]
    fn traducir_aliases_deberia_aceptar_version_con_tilde() {
        let resultado = traducir_aliases(args(&["quetzal", "--versión"]));
        assert_eq!(resultado, args(&["quetzal", "version"]));
    }

    #[test]
    fn traducir_aliases_deberia_convertir_nuevo_flag_en_subcomando() {
        let resultado = traducir_aliases(args(&["quetzal", "--nuevo", "mi-proyecto"]));
        assert_eq!(resultado, args(&["quetzal", "nuevo", "mi-proyecto"]));
    }
}
