// Módulo de manejo de errores para el intérprete Quetzal
// Define todos los tipos de errores que puede generar el intérprete

use thiserror::Error;
use colored::Colorize;

/// Tipos de errores del intérprete Quetzal
#[derive(Error, Debug, Clone)]
pub enum ErrorQuetzal {
    #[error("Error de sintaxis en línea {linea}: {mensaje}")]
    ErrorSintaxis { linea: usize, mensaje: String },
    
    #[error("Error de tipo en línea {linea}: {mensaje}")]
    ErrorTipo { linea: usize, mensaje: String },
    
    #[error("Error de ejecución en línea {linea}: {mensaje}")]
    ErrorEjecucion { linea: usize, mensaje: String },
    
    #[error("Variable no definida en línea {linea}: '{nombre}'")]
    VariableNoDefinida { linea: usize, nombre: String },
    
    #[error("Función no definida en línea {linea}: '{nombre}'")]
    FuncionNoDefinida { linea: usize, nombre: String },
    
    #[error("Intento de asignación a variable inmutable en línea {linea}: '{nombre}'")]
    #[allow(dead_code)]
    VariableInmutable { linea: usize, nombre: String },
    
    #[error("División por cero en línea {linea}")]
    DivisionPorCero { linea: usize },
    
    #[error("Índice fuera de rango en línea {linea}: índice {indice} en lista de tamaño {tamanio}")]
    #[allow(dead_code)]
    IndiceFueraDeRango { linea: usize, indice: usize, tamanio: usize },
    
    #[error("Conversión de tipo inválida en línea {linea}: no se puede convertir {tipo_origen} a {tipo_destino}")]
    #[allow(dead_code)]
    ConversionInvalida { linea: usize, tipo_origen: String, tipo_destino: String },
    
    #[error("Error de conversión en línea {linea}: {mensaje}")]
    ErrorConversion { linea: usize, mensaje: String },
    
    #[error("Número incorrecto de argumentos en línea {linea}: se esperaban {esperados}, se recibieron {recibidos}")]
    ArgumentosIncorrectos { linea: usize, esperados: usize, recibidos: usize },
    
    #[error("Token inesperado en línea {linea}: '{token}'")]
    TokenInesperado { linea: usize, token: String },
    
    #[error("Fin de archivo inesperado")]
    #[allow(dead_code)]
    FinArchivoInesperado,
    
    #[error("Error de importación: {mensaje}")]
    #[allow(dead_code)]
    ErrorImportacion { mensaje: String },
    
    #[error("Error interno del intérprete: {mensaje}")]
    #[allow(dead_code)]
    ErrorInterno { mensaje: String },
}

impl ErrorQuetzal {
    /// Muestra el error con formato coloreado en la consola
    pub fn mostrar_error(&self) {
        match self {
            ErrorQuetzal::ErrorSintaxis { linea, mensaje } => {
                eprintln!("{} {}: {}", 
                    "Error de Sintaxis".red().bold(),
                    format!("línea {}", linea).yellow(),
                    mensaje
                );
            },
            ErrorQuetzal::ErrorTipo { linea, mensaje } => {
                eprintln!("{} {}: {}", 
                    "Error de Tipo".red().bold(),
                    format!("línea {}", linea).yellow(),
                    mensaje
                );
            },
            ErrorQuetzal::ErrorEjecucion { linea, mensaje } => {
                eprintln!("{} {}: {}", 
                    "Error de Ejecución".red().bold(),
                    format!("línea {}", linea).yellow(),
                    mensaje
                );
            },
            ErrorQuetzal::VariableNoDefinida { linea, nombre } => {
                eprintln!("{} {}: La variable '{}' no está definida", 
                    "Error".red().bold(),
                    format!("línea {}", linea).yellow(),
                    nombre.cyan()
                );
            },
            ErrorQuetzal::FuncionNoDefinida { linea, nombre } => {
                eprintln!("{} {}: La función '{}' no está definida", 
                    "Error".red().bold(),
                    format!("línea {}", linea).yellow(),
                    nombre.cyan()
                );
            },
            ErrorQuetzal::VariableInmutable { linea, nombre } => {
                eprintln!("{} {}: No se puede modificar la variable inmutable '{}'", 
                    "Error".red().bold(),
                    format!("línea {}", linea).yellow(),
                    nombre.cyan()
                );
            },
            ErrorQuetzal::DivisionPorCero { linea } => {
                eprintln!("{} {}: División por cero", 
                    "Error Matemático".red().bold(),
                    format!("línea {}", linea).yellow()
                );
            },
            ErrorQuetzal::IndiceFueraDeRango { linea, indice, tamanio } => {
                eprintln!("{} {}: Índice {} fuera de rango (tamaño de lista: {})", 
                    "Error de Índice".red().bold(),
                    format!("línea {}", linea).yellow(),
                    indice,
                    tamanio
                );
            },
            ErrorQuetzal::ConversionInvalida { linea, tipo_origen, tipo_destino } => {
                eprintln!("{} {}: No se puede convertir {} a {}", 
                    "Error de Conversión".red().bold(),
                    format!("línea {}", linea).yellow(),
                    tipo_origen.cyan(),
                    tipo_destino.cyan()
                );
            },
            ErrorQuetzal::ArgumentosIncorrectos { linea, esperados, recibidos } => {
                eprintln!("{} {}: Se esperaban {} argumentos, pero se recibieron {}", 
                    "Error de Argumentos".red().bold(),
                    format!("línea {}", linea).yellow(),
                    esperados,
                    recibidos
                );
            },
            ErrorQuetzal::TokenInesperado { linea, token } => {
                eprintln!("{} {}: Token inesperado '{}'", 
                    "Error de Sintaxis".red().bold(),
                    format!("línea {}", linea).yellow(),
                    token.cyan()
                );
            },
            ErrorQuetzal::FinArchivoInesperado => {
                eprintln!("{}: Fin de archivo inesperado", 
                    "Error de Sintaxis".red().bold()
                );
            },
            ErrorQuetzal::ErrorImportacion { mensaje } => {
                eprintln!("{}: {}", 
                    "Error de Importación".red().bold(),
                    mensaje
                );
            },
            ErrorQuetzal::ErrorInterno { mensaje } => {
                eprintln!("{}: {}", 
                    "Error Interno".red().bold(),
                    mensaje
                );
            },
            ErrorQuetzal::ErrorConversion { linea, mensaje } => {
                eprintln!("{} línea {}: {}", 
                    "Error de Conversión".red().bold(),
                    linea,
                    mensaje
                );
            },
        }
    }
}

/// Tipo de resultado estándar para el intérprete
pub type ResultadoQuetzal<T> = Result<T, ErrorQuetzal>;
