//! REPL interactivo del Lenguaje Quetzal.
//!
//! Construido sobre `reedline`: estado persistente entre líneas, entrada
//! multilínea cuando un bloque no cierra, historial y comandos especiales
//! (`:ayuda`, `:limpiar`, `:cargar`, `:modulos`, `:estado`, `salir`).

mod multilinea;
mod prompt;

use diagnosticos::colores::Paleta;
use motor::{ConfiguracionMotor, SesionInteractiva, VERSION_QUETZAL};
use reedline::{FileBackedHistory, Reedline, Signal};

use crate::multilinea::ValidadorBloques;
use crate::prompt::PromptQuetzal;

/// Inicia el REPL interactivo. Devuelve el código de salida del proceso.
pub fn iniciar() -> i32 {
    let mut sesion = match SesionInteractiva::nueva(ConfiguracionMotor::por_defecto()) {
        Ok(sesion) => sesion,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };

    println!("Quetzal v{VERSION_QUETZAL}");
    println!("Escribe código o 'salir' para terminar.");

    // Sin terminal interactiva (tubería, redirección) se leen las líneas
    // directamente; reedline necesita una terminal real.
    if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        return iniciar_sin_terminal(&mut sesion);
    }

    let mut editor = crear_editor();
    let prompt = PromptQuetzal;
    let paleta = Paleta::default();

    loop {
        match editor.read_line(&prompt) {
            Ok(Signal::Success(linea)) => {
                let entrada = linea.trim();
                if entrada.is_empty() {
                    continue;
                }
                match procesar(&mut sesion, &mut editor, entrada, &paleta) {
                    Accion::Continuar => {}
                    Accion::Salir => return 0,
                }
            }
            // Ctrl-C limpia la línea actual; Ctrl-D cierra la sesión.
            Ok(Signal::CtrlC) => continue,
            Ok(Signal::CtrlD) => return 0,
            Err(error) => {
                eprintln!("error de la terminal: {error}");
                return 1;
            }
        }
    }
}

enum Accion {
    Continuar,
    Salir,
}

/// Bucle simple para entrada no interactiva: acumula líneas hasta que el
/// fragmento queda completo y lo evalúa.
fn iniciar_sin_terminal(sesion: &mut SesionInteractiva) -> i32 {
    let paleta = Paleta::default();
    let mut pendiente = String::new();

    for linea in std::io::stdin().lines() {
        let Ok(linea) = linea else { break };
        pendiente.push_str(&linea);
        pendiente.push('\n');
        if multilinea::entrada_incompleta(&pendiente) {
            continue;
        }

        let entrada = std::mem::take(&mut pendiente);
        let entrada = entrada.trim();
        if entrada.is_empty() {
            continue;
        }
        // Sin editor, `:limpiar` no tiene pantalla que limpiar.
        if entrada == ":limpiar" {
            continue;
        }
        match procesar_comando(sesion, entrada, &paleta) {
            Some(Accion::Salir) => return 0,
            Some(Accion::Continuar) => continue,
            None => reportar(sesion.evaluar(entrada), sesion, &paleta),
        }
    }
    0
}

/// Procesa comandos que no necesitan el editor. Devuelve `None` si la
/// entrada no es un comando y debe evaluarse como código.
fn procesar_comando(
    sesion: &mut SesionInteractiva,
    entrada: &str,
    paleta: &Paleta,
) -> Option<Accion> {
    match entrada {
        "salir" => return Some(Accion::Salir),
        ":ayuda" => {
            mostrar_ayuda();
            return Some(Accion::Continuar);
        }
        ":modulos" => {
            let modulos = sesion.modulos();
            if modulos.is_empty() {
                println!("(no hay módulos cargados)");
            } else {
                for modulo in modulos {
                    println!("{modulo}");
                }
            }
            return Some(Accion::Continuar);
        }
        ":estado" => {
            let estado = sesion.estado();
            if estado.is_empty() {
                println!("(la sesión no tiene variables)");
            } else {
                for (nombre, tipo, valor) in estado {
                    println!("{tipo} {nombre} = {valor}");
                }
            }
            return Some(Accion::Continuar);
        }
        _ => {}
    }

    if let Some(ruta) = entrada.strip_prefix(":cargar") {
        let ruta = ruta.trim();
        if ruta.is_empty() {
            println!("uso: :cargar ruta/al/archivo.qz");
        } else {
            reportar(sesion.cargar_archivo(ruta), sesion, paleta);
        }
        return Some(Accion::Continuar);
    }
    if entrada.starts_with(':') {
        println!("comando desconocido '{entrada}'; escribe :ayuda para ver los comandos");
        return Some(Accion::Continuar);
    }
    None
}

fn crear_editor() -> Reedline {
    let mut editor = Reedline::create().with_validator(Box::new(ValidadorBloques));

    // Historial persistente entre sesiones (mejor esfuerzo).
    if let Some(ruta) = ruta_historial()
        && let Ok(historial) = FileBackedHistory::with_file(500, ruta)
    {
        editor = editor.with_history(Box::new(historial));
    }
    editor
}

fn ruta_historial() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
    Some(std::path::PathBuf::from(base).join(".quetzal_historial"))
}

fn procesar(
    sesion: &mut SesionInteractiva,
    editor: &mut Reedline,
    entrada: &str,
    paleta: &Paleta,
) -> Accion {
    if entrada == ":limpiar" {
        if editor.clear_screen().is_err() {
            eprintln!("no se pudo limpiar la pantalla");
        }
        return Accion::Continuar;
    }
    if let Some(accion) = procesar_comando(sesion, entrada, paleta) {
        return accion;
    }
    reportar(sesion.evaluar(entrada), sesion, paleta);
    Accion::Continuar
}

/// Muestra el valor de una expresión suelta o los errores sin cerrar la sesión.
fn reportar(
    resultado: Result<Option<String>, Vec<nucleo::ErrorQuetzal>>,
    sesion: &SesionInteractiva,
    paleta: &Paleta,
) {
    match resultado {
        Ok(Some(valor)) => println!("{valor}"),
        Ok(None) => {}
        Err(errores) => {
            for error in errores {
                eprint!(
                    "{}",
                    diagnosticos::reportar(&error, sesion.ultima_fuente(), paleta)
                );
            }
        }
    }
}

fn mostrar_ayuda() {
    println!("Comandos del REPL de Quetzal:");
    println!("  salir            termina la sesión");
    println!("  :ayuda           muestra esta ayuda");
    println!("  :limpiar         limpia la pantalla");
    println!("  :cargar ARCHIVO  ejecuta un archivo .qz dentro de la sesión");
    println!("  :modulos         lista los módulos cargados");
    println!("  :estado          lista las variables de la sesión");
    println!();
    println!("Cualquier otra entrada se evalúa como código Quetzal.");
    println!("Los bloques sin cerrar continúan en la línea siguiente.");
}
