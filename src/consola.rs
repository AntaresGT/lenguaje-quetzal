use std::io::{self, Write};

pub fn imprimir_error(texto: &str) {
    imprimir_color(texto, "\x1b[31m");
}

pub fn imprimir_advertencia(texto: &str) {
    imprimir_color(texto, "\x1b[33m");
}

pub fn imprimir_informacion(texto: &str) {
    imprimir_color(texto, "\x1b[34m");
}

pub fn imprimir_depurar(texto: &str) {
    imprimir_color(texto, "\x1b[35m");
}

pub fn imprimir_exito(texto: &str) {
    imprimir_color(texto, "\x1b[32m");
}

pub fn imprimir_alerta(texto: &str) {
    imprimir_color(texto, "\x1b[31m");
}

pub fn imprimir_confirmacion(texto: &str) {
    imprimir_color(texto, "\x1b[32m");
}

fn imprimir_color(texto: &str, codigo: &str) {
    print!("{}{}\x1b[0m\n", codigo, texto);
    let _ = io::stdout().flush();
}
