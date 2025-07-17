// Módulo principal del intérprete Quetzal
// Coordina el análisis léxico, sintáctico y la evaluación del código

use crate::analizador_lexico::AnalizadorLexico;
use crate::analizador_sintactico::AnalizadorSintactico;
use crate::evaluador::Evaluador;
use crate::errores::{ErrorQuetzal, ResultadoQuetzal};
use crate::tipos_datos::Valor;

/// Función principal que interpreta código Quetzal
/// 
/// Esta función toma el código fuente como cadena de texto y lo procesa
/// en tres etapas principales:
/// 1. Análisis léxico: convierte el texto en tokens
/// 2. Análisis sintáctico: convierte los tokens en un AST
/// 3. Evaluación: ejecuta el AST y produce el resultado
pub fn interpretar(codigo: &str) -> ResultadoQuetzal<Valor> {
    // Etapa 1: Análisis léxico
    // Convierte el código fuente en una secuencia de tokens
    let mut analizador_lexico = AnalizadorLexico::nuevo(codigo);
    let tokens = analizador_lexico.analizar().inspect_err(|error| {
        error.mostrar_error();
    })?;
    
    // Filtrar tokens de nueva línea para simplificar el análisis sintáctico
    let tokens_filtrados: Vec<_> = tokens
        .into_iter()
        .filter(|token| !matches!(token.tipo, crate::analizador_lexico::TipoToken::NuevaLinea))
        .collect();
    
    // Debug: mostrar tokens si hay una variable de entorno activada
    if std::env::var("QUETZAL_DEBUG_TOKENS").is_ok() {
        println!("=== TOKENS GENERADOS ===");
        for token in &tokens_filtrados {
            println!("{:?}", token);
        }
        println!("========================");
    }
    
    // Etapa 2: Análisis sintáctico
    // Convierte los tokens en un Árbol de Sintaxis Abstracta (AST)
    let mut analizador_sintactico = AnalizadorSintactico::nuevo(tokens_filtrados);
    let ast = analizador_sintactico.analizar().inspect_err(|error| {
        error.mostrar_error();
    })?;
    
    // Debug: mostrar AST si hay una variable de entorno activada
    if std::env::var("QUETZAL_DEBUG_AST").is_ok() {
        println!("=== AST GENERADO ===");
        println!("{:#?}", ast);
        println!("====================");
    }
    
    // Etapa 3: Evaluación
    // Ejecuta el AST y produce el resultado final
    let mut evaluador = Evaluador::nuevo();
    let (resultado, control_flujo) = evaluador.evaluar(&ast).inspect_err(|error| {
        error.mostrar_error();
    })?;
    
    // Debug: mostrar resultado si hay una variable de entorno activada
    if std::env::var("QUETZAL_DEBUG_RESULTADO").is_ok() {
        println!("=== RESULTADO ===");
        println!("Valor: {:?}", resultado);
        println!("Control de flujo: {:?}", control_flujo);
        println!("=================");
    }
    
    Ok(resultado)
}

/// Interpreta código Quetzal desde un archivo
/// 
/// Esta es una función de conveniencia que lee un archivo y lo interpreta
#[allow(dead_code)]
pub fn interpretar_archivo(ruta: &str) -> ResultadoQuetzal<Valor> {
    interpretar_archivo_con_modulos(ruta)
}

/// Interpreta código Quetzal desde un archivo con soporte completo de módulos
/// 
/// Esta función usa el evaluador avanzado que soporta importación y exportación de módulos
pub fn interpretar_archivo_con_modulos(ruta: &str) -> ResultadoQuetzal<Valor> {
    use std::fs;
    
    let codigo = fs::read_to_string(ruta)
        .map_err(|error| ErrorQuetzal::ErrorInterno {
            mensaje: format!("No se pudo leer el archivo '{}': {}", ruta, error),
        })?;
    
    // Etapa 1: Análisis léxico
    let mut analizador_lexico = AnalizadorLexico::nuevo(&codigo);
    let tokens = analizador_lexico.analizar()?;
    
    // Filtrar tokens de nueva línea
    let tokens_filtrados: Vec<_> = tokens
        .into_iter()
        .filter(|token| !matches!(token.tipo, crate::analizador_lexico::TipoToken::NuevaLinea))
        .collect();
    
    // Etapa 2: Análisis sintáctico
    let mut analizador_sintactico = AnalizadorSintactico::nuevo(tokens_filtrados);
    let ast = analizador_sintactico.analizar()?;
    
    // Etapa 3: Evaluación con soporte de módulos
    let mut evaluador = Evaluador::nuevo_con_modulos(ruta)?;
    let (resultado, _) = evaluador.evaluar(&ast)?;
    
    Ok(resultado)
}

/// Modo interactivo del intérprete (REPL - Read-Eval-Print Loop)
/// 
/// Permite al usuario escribir código Quetzal línea por línea
/// y ver los resultados inmediatamente
#[allow(dead_code)]
pub fn modo_interactivo() {
    use colored::Colorize;
    
    println!("{}", "=== Intérprete Interactivo de Quetzal ===".cyan().bold());
    println!("{}", "Escribe código Quetzal y presiona Enter para ejecutarlo.".yellow());
    println!("{}", "Escribe 'salir' para terminar.".yellow());
    println!();
    
    let mut evaluador = Evaluador::nuevo();
    let mut numero_linea = 1;
    
    loop {
        // Mostrar prompt
        print!("{} ", format!("quetzal[{}]>", numero_linea).green().bold());
        use std::io::{self, Write};
        io::stdout().flush().unwrap();
        
        // Leer entrada del usuario
        let mut entrada = String::new();
        match io::stdin().read_line(&mut entrada) {
            Ok(_) => {
                let entrada = entrada.trim();
                
                // Verificar comandos especiales
                if entrada == "salir" || entrada == "exit" {
                    println!("{}", "¡Hasta luego!".green().bold());
                    break;
                }
                
                if entrada.is_empty() {
                    continue;
                }
                
                // Interpretar la línea
                match interpretar_linea_interactiva(entrada, &mut evaluador) {
                    Ok(valor) => {
                        if !matches!(valor, Valor::Vacio) {
                            println!("{} {}", "=>".blue().bold(), valor.a_cadena().cyan());
                        }
                    },
                    Err(_) => {
                        // Los errores ya se muestran dentro de interpretar()
                    }
                }
                
                numero_linea += 1;
            },
            Err(error) => {
                eprintln!("{}", format!("Error al leer entrada: {}", error).red());
                break;
            }
        }
    }
}

/// Interpreta una línea en modo interactivo
/// 
/// Reutiliza el evaluador existente para mantener el estado entre líneas
#[allow(dead_code)]
fn interpretar_linea_interactiva(codigo: &str, evaluador: &mut Evaluador) -> ResultadoQuetzal<Valor> {
    // Análisis léxico
    let mut analizador_lexico = AnalizadorLexico::nuevo(codigo);
    let tokens = analizador_lexico.analizar()?;
    
    // Filtrar tokens de nueva línea
    let tokens_filtrados: Vec<_> = tokens
        .into_iter()
        .filter(|token| !matches!(token.tipo, crate::analizador_lexico::TipoToken::NuevaLinea))
        .collect();
    
    // Análisis sintáctico
    let mut analizador_sintactico = AnalizadorSintactico::nuevo(tokens_filtrados);
    let ast = analizador_sintactico.analizar()?;
    
    // Evaluación
    let (resultado, _) = evaluador.evaluar(&ast)?;
    
    Ok(resultado)
}

/// Verifica la sintaxis de código Quetzal sin ejecutarlo
/// 
/// Útil para validación y herramientas de desarrollo
#[allow(dead_code)]
pub fn verificar_sintaxis(codigo: &str) -> ResultadoQuetzal<()> {
    // Solo análisis léxico y sintáctico, sin evaluación
    let mut analizador_lexico = AnalizadorLexico::nuevo(codigo);
    let tokens = analizador_lexico.analizar()?;
    
    let tokens_filtrados: Vec<_> = tokens
        .into_iter()
        .filter(|token| !matches!(token.tipo, crate::analizador_lexico::TipoToken::NuevaLinea))
        .collect();
    
    let mut analizador_sintactico = AnalizadorSintactico::nuevo(tokens_filtrados);
    let _ = analizador_sintactico.analizar()?;
    
    Ok(())
}

/// Obtiene información detallada sobre un error de Quetzal
/// 
/// Proporciona información adicional para ayudar en la depuración
#[allow(dead_code)]
pub fn analizar_error(error: &ErrorQuetzal) -> String {
    match error {
        ErrorQuetzal::ErrorSintaxis { linea, mensaje } => {
            format!(
                "Error de Sintaxis en línea {}:\n\
                Descripción: {}\n\
                Sugerencia: Revisa la sintaxis de Quetzal en la documentación.",
                linea, mensaje
            )
        },
        ErrorQuetzal::ErrorTipo { linea, mensaje } => {
            format!(
                "Error de Tipo en línea {}:\n\
                Descripción: {}\n\
                Sugerencia: Verifica que los tipos de datos sean compatibles.",
                linea, mensaje
            )
        },
        ErrorQuetzal::VariableNoDefinida { linea, nombre } => {
            format!(
                "Variable No Definida en línea {}:\n\
                Variable: '{}'\n\
                Sugerencia: Define la variable antes de usarla o verifica el nombre.",
                linea, nombre
            )
        },
        ErrorQuetzal::FuncionNoDefinida { linea, nombre } => {
            format!(
                "Función No Definida en línea {}:\n\
                Función: '{}'\n\
                Sugerencia: Define la función antes de llamarla o verifica el nombre.",
                linea, nombre
            )
        },
        ErrorQuetzal::DivisionPorCero { linea } => {
            format!(
                "División por Cero en línea {}:\n\
                Sugerencia: Verifica que el denominador no sea cero antes de dividir.",
                linea
            )
        },
        _ => error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_interpretar_basico() {
        let codigo = r#"
            entero numero = 42
            consola.imprimir("El número es: " + numero.texto())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }
    
    #[test]
    fn test_verificar_sintaxis() {
        let codigo = r#"
            entero var contador = 0
            para (entero i = 0; i < 10; i++) {
                contador = contador + 1
            }
        "#;
        
        let resultado = verificar_sintaxis(codigo);
        assert!(resultado.is_ok());
    }
    
    #[test]
    fn test_error_sintaxis() {
        let codigo = r#"
            entero numero = 
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_err());
    }
}
