//! Punto de entrada del binario `quetzal`.
//!
//! Este crate solo interpreta argumentos y delega en `motor`, `repl` y
//! `paquetes`; no contiene lógica del lenguaje.

mod argumentos;
mod comandos;

use argumentos::{Cli, Comando};
use clap::Parser;

fn main() {
    let argumentos_traducidos = argumentos::traducir_aliases(std::env::args().collect());

    let cli = match Cli::try_parse_from(argumentos_traducidos) {
        Ok(cli) => cli,
        Err(error) => {
            // clap ya formatea el mensaje (incluida la ayuda).
            let _ = error.print();
            std::process::exit(2);
        }
    };

    let codigo_salida = match cli.comando {
        Some(Comando::Ejecutar { archivo }) => comandos::ejecutar::ejecutar(archivo.as_deref()),
        Some(Comando::Revisar { archivo }) => comandos::revisar::revisar(archivo.as_deref()),
        Some(Comando::Nuevo { nombre }) => comandos::nuevo::nuevo(&nombre),
        Some(Comando::Instalar { paquete }) => comandos::instalar::instalar(paquete.as_deref()),
        Some(Comando::Cache { accion }) => comandos::cache::cache(accion.as_deref()),
        Some(Comando::Version) => comandos::version::version(),
        Some(Comando::Ayuda) => comandos::ayuda(),
        None => comandos::repl::repl(),
    };

    std::process::exit(codigo_salida);
}
