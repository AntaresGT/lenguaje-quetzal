// Permitir código muerto para módulos de infraestructura pendientes de integración
#![allow(dead_code)]

mod errores;
mod nucleo;
mod interprete;
mod modulos;
mod configuracion;
mod utilidades;
mod nativos;
mod comandos;
mod repl;

#[cfg(test)]
mod pruebas;

use clap::{Arg, Command};
use std::fs;

const VERSION: &str = env!("CARGO_PKG_VERSION");


#[tokio::main]
async fn main() {
    let matches = Command::new("quetzal")
        .version(VERSION)
        .about("Intérprete del Lenguaje Quetzal")
        .arg(
            Arg::new("archivo")
                .help("Archivo .qz a ejecutar")
                .index(1)
        )
        .arg(
            Arg::new("ayuda")
                .long("ayuda")
                .help("Muestra la ayuda")
                .action(clap::ArgAction::SetTrue)
        )
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new("version")
                .short('v')
                .long("version")
                .help("Muestra la versión")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("nuevo")
                .long("nuevo")
                .help("Crea un nuevo proyecto")
                .value_name("NOMBRE_PROYECTO")
        )
        .get_matches();
    
    if matches.get_flag("version") {
        println!("Quetzal v{}", VERSION);
        return;
    }
    
    if matches.get_flag("ayuda") {
        println!("Uso: quetzal [OPCIONES] [ARCHIVO]\n");
        println!("Opciones:");
        println!("  -v, --version          Muestra la versión");
        println!("  -h, --ayuda            Muestra esta ayuda");
        println!("  --nuevo NOMBRE         Crea un nuevo proyecto\n");
        println!("Ejemplos:");
        println!("  quetzal programa.qz");
        println!("  quetzal --nuevo mi-proyecto");
        println!("  quetzal                (modo REPL)");
        return;
    }
    
    if let Some(nombre_proyecto) = matches.get_one::<String>("nuevo") {
        match comandos::crear_proyecto(nombre_proyecto) {
            Ok(_) => println!("Proyecto '{}' creado exitosamente", nombre_proyecto),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }
    
    if let Some(archivo) = matches.get_one::<String>("archivo") {
        // Ejecutar archivo
        match ejecutar_archivo_con_reporte(archivo) {
            Ok(_) => {}
            Err(_) => {
                // El error ya fue reportado
                std::process::exit(1);
            }
        }
    } else {
        // Modo REPL
        println!("Quetzal REPL v{}", VERSION);
        println!("Escribe código o 'salir' para terminar\n");
        
        match repl::Repl::nuevo() {
            Ok(mut repl) => {
                loop {
                    use std::io::{self, Write};
                    print!("qz> ");
                    io::stdout().flush().unwrap();
                    
                    let mut entrada = String::new();
                    match io::stdin().read_line(&mut entrada) {
                        Ok(_) => {
                            let entrada = entrada.trim();
                            if entrada == "salir" || entrada.is_empty() {
                                break;
                            }
                            
                            match repl.evaluar(entrada) {
                                Ok(resultado) => {
                                    if !resultado.is_empty() {
                                        println!("{}", resultado);
                                    }
                                }
                                Err(e) => {
                                    // Reportar error con formato mejorado
                                    errores::reporte::reportar_error(&e, Some(entrada));
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
            Err(e) => {
                eprintln!("Error al inicializar REPL: {}", e);
                std::process::exit(1);
            }
        }
    }
}

/// Ejecuta un archivo con reporte de errores mejorado
fn ejecutar_archivo_con_reporte(ruta: &str) -> crate::errores::Resultado<()> {
    // Leer contenido primero para poder mostrarlo en errores
    let contenido = match fs::read_to_string(ruta) {
        Ok(c) => c,
        Err(e) => {
            let error = crate::errores::Error::sistema(
                crate::errores::CodigoError::ErrorLecturaArchivo,
                format!("no se pudo leer el archivo '{}': {}", ruta, e),
                None,
            );
            errores::reporte::reportar_error_simple(&error);
            return Err(error);
        }
    };
    
    // Ejecutar y reportar cualquier error con el codigo fuente
    match ejecutar_archivo_interno(ruta, &contenido) {
        Ok(_) => Ok(()),
        Err(e) => {
            errores::reporte::reportar_error(&e, Some(&contenido));
            Err(e)
        }
    }
}

/// Ejecuta un archivo (implementacion interna)
fn ejecutar_archivo_interno(ruta: &str, _contenido: &str) -> crate::errores::Resultado<()> {
    use std::path::Path;
    
    let ruta_path = Path::new(ruta);
    let ruta_absoluta = ruta_path.canonicalize().unwrap_or_else(|_| ruta_path.to_path_buf());
    let permisos = cargar_configuracion_permisos(ruta_absoluta.to_string_lossy().as_ref())?;

    let mut cargador = modulos::CargadorModulos::desde_archivo_actual(&ruta_absoluta);
    cargador.establecer_manifiesto_actual(
        ruta_absoluta
            .parent()
            .map(modulos::ManifiestoPaquete::buscar_desde_directorio_resultado)
            .transpose()?
            .flatten(),
    );

    let modulo_principal = cargador.obtener_modulo_compilado(
        ruta_absoluta.to_str().unwrap_or(ruta),
    )?;

    let mut verificador = nucleo::semantico::Verificador::nuevo();
    verificador.establecer_archivo_actual(ruta_absoluta.to_string_lossy().to_string());
    verificador.establecer_cargador_modulos(cargador.clone());
    verificador.verificar_programa_hir(&modulo_principal.hir)?;

    let mut entorno = interprete::entorno::Entorno::con_archivo(
        ruta_absoluta.to_string_lossy().to_string(),
    );
    entorno.establecer_cargador_modulos(cargador.clone());
    entorno.establecer_permisos(permisos);
    
    nativos::registro::registrar_modulos_nativos(&mut entorno)?;
    interprete::hir::ejecutar_programa_hir(&modulo_principal.hir, &mut entorno)?;
    
    Ok(())
}

/// Busca y carga la configuración de permisos desde quetzal.json
fn cargar_configuracion_permisos(ruta_archivo: &str) -> crate::errores::Resultado<configuracion::permisos::SistemaPermisos> {
    use std::path::Path;
    
    let archivo_path = Path::new(ruta_archivo);

    if let Some(dir) = archivo_path.parent() {
        if let Some(manifiesto) = modulos::ManifiestoPaquete::buscar_desde_directorio_resultado(dir)? {
            if let Some(permisos) = manifiesto.construir_permisos() {
                return Ok(permisos);
            }
        }
    }

    Ok(configuracion::permisos::SistemaPermisos::nuevo())
}
