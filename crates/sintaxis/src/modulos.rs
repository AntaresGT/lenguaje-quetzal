//! Parseo de `importar` y `exportar`.

use ast::{Elemento, Importacion, SimboloImportado};
use lexico::TipoToken;
use nucleo::ErrorQuetzal;

use crate::parser::Parser;

impl Parser<'_> {
    /// `importar { sumar, saludo como alias } desde "./calculadora.qz"`
    pub(crate) fn parsear_importacion(&mut self) -> Result<Importacion, ErrorQuetzal> {
        let inicio = self.esperar(&TipoToken::Importar, "'importar'")?.ubicacion;
        self.esperar(&TipoToken::LlaveIzquierda, "'{' después de 'importar'")?;

        let mut simbolos = Vec::new();
        while !self.comprobar(&TipoToken::LlaveDerecha) {
            let (nombre, _) = self.esperar_identificador("el nombre a importar")?;
            let alias = if self.consumir_si(&TipoToken::Como) {
                let (alias, _) = self.esperar_identificador("el alias del símbolo")?;
                Some(alias)
            } else {
                None
            };
            simbolos.push(SimboloImportado { nombre, alias });
            if !self.consumir_si(&TipoToken::Coma) {
                break;
            }
        }

        self.esperar(&TipoToken::LlaveDerecha, "'}' para cerrar la importación")?;
        self.esperar(&TipoToken::Desde, "'desde'")?;

        let token = self.actual().clone();
        let origen = match token.tipo {
            TipoToken::LiteralTexto(ruta) => {
                self.avanzar();
                ruta
            }
            _ => return Err(self.error_token_inesperado("la ruta del módulo entre comillas")),
        };

        Ok(Importacion {
            simbolos,
            origen,
            ubicacion: inicio.unir(token.ubicacion),
        })
    }

    /// `exportar { sumar, Usuario, saludo }`
    pub(crate) fn parsear_exportacion(&mut self) -> Result<Elemento, ErrorQuetzal> {
        let inicio = self.esperar(&TipoToken::Exportar, "'exportar'")?.ubicacion;
        self.esperar(&TipoToken::LlaveIzquierda, "'{' después de 'exportar'")?;

        let mut simbolos = Vec::new();
        while !self.comprobar(&TipoToken::LlaveDerecha) {
            let (nombre, _) = self.esperar_identificador("el nombre a exportar")?;
            simbolos.push(nombre);
            if !self.consumir_si(&TipoToken::Coma) {
                break;
            }
        }

        let fin = self
            .esperar(&TipoToken::LlaveDerecha, "'}' para cerrar la exportación")?
            .ubicacion;

        Ok(Elemento::Exportacion {
            simbolos,
            ubicacion: inicio.unir(fin),
        })
    }
}
