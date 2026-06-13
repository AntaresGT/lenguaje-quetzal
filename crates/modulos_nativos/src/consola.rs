//! Módulo nativo `consola` (global, no requiere importación).
//!
//! Según `ejemplos/consola.qz`: `mostrar`, `mostrar_error`,
//! `mostrar_advertencia`, `mostrar_exito`, `mostrar_informacion`, `pedir` y
//! `pedir_secreto`.

use std::io::Write;

use maquina_virtual::valores::texto_de_valor;
use maquina_virtual::{Fallo, RegistroNativos, Valor};

const ROJO: &str = "\x1b[31m";
const AMARILLO: &str = "\x1b[33m";
const VERDE: &str = "\x1b[32m";
const AZUL: &str = "\x1b[34m";
const RESTAURAR: &str = "\x1b[0m";

/// Registra todas las funciones de `consola`.
pub fn registrar(registro: &mut RegistroNativos) {
    registro.registrar_modulo("consola");

    registro.registrar_funcion(
        "consola.mostrar",
        Box::new(|argumentos| {
            println!("{}", unir_argumentos(argumentos));
            Ok(Valor::Nulo)
        }),
    );
    registro.registrar_funcion(
        "consola.mostrar_error",
        Box::new(|argumentos| {
            eprintln!("{ROJO}{}{RESTAURAR}", unir_argumentos(argumentos));
            Ok(Valor::Nulo)
        }),
    );
    registro.registrar_funcion(
        "consola.mostrar_advertencia",
        Box::new(|argumentos| {
            println!("{AMARILLO}{}{RESTAURAR}", unir_argumentos(argumentos));
            Ok(Valor::Nulo)
        }),
    );
    registro.registrar_funcion(
        "consola.mostrar_exito",
        Box::new(|argumentos| {
            println!("{VERDE}{}{RESTAURAR}", unir_argumentos(argumentos));
            Ok(Valor::Nulo)
        }),
    );
    registro.registrar_funcion(
        "consola.mostrar_informacion",
        Box::new(|argumentos| {
            println!("{AZUL}{}{RESTAURAR}", unir_argumentos(argumentos));
            Ok(Valor::Nulo)
        }),
    );
    registro.registrar_funcion("consola.pedir", Box::new(pedir));
    // `pedir_secreto` ocultará la entrada en la fase 7; por ahora se comporta
    // como `pedir` para no bloquear los ejemplos.
    registro.registrar_funcion("consola.pedir_secreto", Box::new(pedir));
}

fn unir_argumentos(argumentos: &[Valor]) -> String {
    argumentos
        .iter()
        .map(texto_de_valor)
        .collect::<Vec<_>>()
        .join(" ")
}

fn pedir(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    if !argumentos.is_empty() {
        print!("{} ", unir_argumentos(argumentos));
        if std::io::stdout().flush().is_err() {
            return Err(error_consola("no se pudo escribir en la consola"));
        }
    }
    let mut entrada = String::new();
    match std::io::stdin().read_line(&mut entrada) {
        Ok(_) => Ok(Valor::texto(entrada.trim_end_matches(['\r', '\n']))),
        Err(_) => Err(error_consola("no se pudo leer la entrada de la consola")),
    }
}

fn error_consola(mensaje: &str) -> Fallo {
    Fallo::excepcion("E0404", mensaje, Vec::new(), None)
}
