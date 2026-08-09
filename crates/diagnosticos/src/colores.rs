//! Soporte de colores para los reportes en terminal.

use owo_colors::OwoColorize;

/// Paleta de colores de los reportes; puede desactivarse para pruebas
/// o terminales sin soporte de color.
#[derive(Debug, Clone, Copy)]
pub struct Paleta {
    pub activada: bool,
}

impl Paleta {
    pub fn nueva(activada: bool) -> Self {
        Self { activada }
    }

    pub fn error(&self, texto: &str) -> String {
        if self.activada {
            texto.red().bold().to_string()
        } else {
            texto.to_string()
        }
    }

    pub fn ayuda(&self, texto: &str) -> String {
        if self.activada {
            texto.cyan().bold().to_string()
        } else {
            texto.to_string()
        }
    }

    pub fn marco(&self, texto: &str) -> String {
        if self.activada {
            texto.blue().bold().to_string()
        } else {
            texto.to_string()
        }
    }

    pub fn enfasis(&self, texto: &str) -> String {
        if self.activada {
            texto.bold().to_string()
        } else {
            texto.to_string()
        }
    }
}

impl Default for Paleta {
    fn default() -> Self {
        Self { activada: true }
    }
}
