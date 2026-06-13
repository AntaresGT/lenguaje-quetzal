//! Construcción de errores sintácticos.

use nucleo::{CategoriaError, ErrorQuetzal};

use crate::parser::Parser;

impl Parser<'_> {
    /// Error genérico de token inesperado, indicando qué se esperaba.
    pub(crate) fn error_token_inesperado(&self, esperado: &str) -> ErrorQuetzal {
        let token = self.actual();
        ErrorQuetzal::nuevo(
            "E0101",
            CategoriaError::Sintactico,
            format!(
                "se esperaba {esperado}, pero se encontró {}",
                token.tipo.descripcion()
            ),
        )
        .con_ubicacion(token.ubicacion)
        .con_archivo(&self.fuente.nombre)
        .con_etiqueta(format!("se esperaba {esperado} aquí"))
    }

    /// Error de expresión esperada.
    pub(crate) fn error_se_esperaba_expresion(&self) -> ErrorQuetzal {
        let token = self.actual();
        ErrorQuetzal::nuevo(
            "E0102",
            CategoriaError::Sintactico,
            format!(
                "se esperaba una expresión, pero se encontró {}",
                token.tipo.descripcion()
            ),
        )
        .con_ubicacion(token.ubicacion)
        .con_archivo(&self.fuente.nombre)
        .con_etiqueta("se esperaba una expresión aquí")
    }
}
