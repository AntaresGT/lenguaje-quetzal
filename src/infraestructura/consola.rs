// Módulo de consola para el lenguaje Quetzal
// Proporciona funciones para interactuar con la consola

use crate::datos::tipos_datos::Valor;
use colored::Colorize;
use inquire::{Password, Text};

/// Objeto global de consola disponible en Quetzal
pub struct Consola;

impl Consola {
    /// Imprime un mensaje normal en la consola
    pub fn imprimir(&self, mensaje: &str) {
        println!("{}", mensaje);
    }

    /// Nuevo nombre: mostrar (alias de imprimir)
    pub fn mostrar(&self, mensaje: &str) {
        self.imprimir(mensaje);
    }

    /// Imprime un valor de Quetzal en la consola
    #[allow(dead_code)]
    pub fn imprimir_valor(&self, valor: &Valor) {
        println!("{}", valor.a_cadena());
    }

    /// Imprime un mensaje de error en rojo
    pub fn imprimir_error(&self, mensaje: &str) {
        eprintln!("{}", mensaje.red().bold());
    }

    /// Nuevo nombre: mostrar_error (alias)
    pub fn mostrar_error(&self, mensaje: &str) {
        self.imprimir_error(mensaje);
    }

    /// Imprime un mensaje de advertencia en amarillo
    pub fn imprimir_advertencia(&self, mensaje: &str) {
        println!("{}", mensaje.yellow().bold());
    }

    /// Nuevo nombre: mostrar_advertencia (alias)
    pub fn mostrar_advertencia(&self, mensaje: &str) {
        self.imprimir_advertencia(mensaje);
    }

    /// Imprime un mensaje de información en azul
    pub fn imprimir_informacion(&self, mensaje: &str) {
        println!("{}", mensaje.blue().bold());
    }

    /// Nuevo nombre: mostrar_informacion (alias)
    pub fn mostrar_informacion(&self, mensaje: &str) {
        self.imprimir_informacion(mensaje);
    }

    /// Imprime un mensaje de depuración en magenta
    pub fn imprimir_depurar(&self, mensaje: &str) {
        println!("{}", mensaje.magenta());
    }

    /// Nuevo nombre: mostrar_depurar (alias)
    pub fn mostrar_depurar(&self, mensaje: &str) {
        self.imprimir_depurar(mensaje);
    }

    /// Imprime un mensaje de éxito en verde
    pub fn imprimir_exito(&self, mensaje: &str) {
        println!("{}", mensaje.green().bold());
    }

    /// Nuevo nombre: mostrar_exito (alias)
    pub fn mostrar_exito(&self, mensaje: &str) {
        self.imprimir_exito(mensaje);
    }

    /// Imprime un mensaje de alerta en amarillo con fondo
    pub fn imprimir_alerta(&self, mensaje: &str) {
        println!("{}", mensaje.on_yellow().black().bold());
    }

    /// Nuevo nombre: mostrar_alerta (alias)
    pub fn mostrar_alerta(&self, mensaje: &str) {
        self.imprimir_alerta(mensaje);
    }

    /// Imprime un mensaje de confirmación en verde con fondo
    pub fn imprimir_confirmacion(&self, mensaje: &str) {
        println!("{}", mensaje.on_green().black().bold());
    }

    /// Nuevo nombre: mostrar_confirmacion (alias)
    pub fn mostrar_confirmacion(&self, mensaje: &str) {
        self.imprimir_confirmacion(mensaje);
    }

    /// Pide entrada al usuario mostrando un mensaje
    pub fn pedir(&self, mensaje: &str) -> String {
        match Text::new(mensaje).prompt() {
            Ok(entrada) => entrada,
            Err(_) => {
                self.imprimir_error("Error al leer la entrada del usuario.");
                String::new()
            }
        }
    }

    /// Pide entrada secreta al usuario (contraseña) sin mostrar el texto ingresado
    pub fn pedir_secreto(&self, mensaje: &str) -> String {
        match Password::new(mensaje).prompt() {
            Ok(secreto_entrada) => secreto_entrada,
            Err(_) => {
                self.imprimir_error("Error al leer la entrada del usuario.");
                String::new()
            }
        }
    }
}

/// Instancia global de la consola
pub static CONSOLA_GLOBAL: Consola = Consola;
