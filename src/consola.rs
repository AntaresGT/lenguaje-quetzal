// Módulo de consola para el lenguaje Quetzal
// Proporciona funciones para interactuar con la consola

use colored::Colorize;
use crate::tipos_datos::Valor;

/// Objeto global de consola disponible en Quetzal
pub struct Consola;

impl Consola {
    /// Imprime un mensaje normal en la consola
    pub fn imprimir(&self, mensaje: &str) {
        println!("{}", mensaje);
    }
    
    /// Imprime un valor de Quetzal en la consola
    pub fn imprimir_valor(&self, valor: &Valor) {
        println!("{}", valor.a_cadena());
    }
    
    /// Imprime un mensaje de error en rojo
    pub fn imprimir_error(&self, mensaje: &str) {
        eprintln!("{}", mensaje.red().bold());
    }
    
    /// Imprime un mensaje de advertencia en amarillo
    pub fn imprimir_advertencia(&self, mensaje: &str) {
        println!("{}", mensaje.yellow().bold());
    }
    
    /// Imprime un mensaje de información en azul
    pub fn imprimir_informacion(&self, mensaje: &str) {
        println!("{}", mensaje.blue().bold());
    }
    
    /// Imprime un mensaje de depuración en magenta
    pub fn imprimir_depurar(&self, mensaje: &str) {
        println!("{}", mensaje.magenta());
    }
    
    /// Imprime un mensaje de éxito en verde
    pub fn imprimir_exito(&self, mensaje: &str) {
        println!("{}", mensaje.green().bold());
    }
    
    /// Imprime un mensaje de alerta en amarillo con fondo
    pub fn imprimir_alerta(&self, mensaje: &str) {
        println!("{}", mensaje.on_yellow().black().bold());
    }
    
    /// Imprime un mensaje de confirmación en verde con fondo
    pub fn imprimir_confirmacion(&self, mensaje: &str) {
        println!("{}", mensaje.on_green().black().bold());
    }
}

/// Instancia global de la consola
pub static CONSOLA_GLOBAL: Consola = Consola;
