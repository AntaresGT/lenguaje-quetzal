//! Recuperación de errores para editores y código incompleto.

use ast::Modulo;
use lexico::{TipoToken, Token, tokenizar_para_editor};
use nucleo::{ErrorQuetzal, Fuente};

use crate::parser::{ClaseDeclaracion, Parser};

/// AST parcial, tokens y errores obtenidos durante una edición.
#[derive(Debug)]
pub struct ResultadoParseo {
    pub modulo: Option<Modulo>,
    pub errores: Vec<ErrorQuetzal>,
    pub completo: bool,
    pub tokens: Vec<Token>,
}

/// Parsea código posiblemente incompleto sin cambiar el modo estricto.
pub fn parsear_modulo_para_editor(fuente: &Fuente) -> ResultadoParseo {
    let resultado = tokenizar_para_editor(fuente);
    let mut parser = Parser {
        fuente,
        tokens: resultado.tokens,
        posicion: 0,
        modo_editor: true,
        errores: resultado.errores,
    };
    let mut elementos = Vec::new();

    while !parser.comprobar(&TipoToken::Fin) {
        let posicion_anterior = parser.posicion;
        match parser.parsear_elemento() {
            Ok(elemento) => elementos.push(elemento),
            Err(error) => {
                parser.errores.push(error);
                parser.sincronizar_elemento();
            }
        }
        if parser.posicion == posicion_anterior {
            parser.avanzar();
        }
    }

    let completo = parser.errores.is_empty();
    ResultadoParseo {
        modulo: Some(Modulo {
            nombre: fuente.nombre.clone(),
            elementos,
        }),
        errores: parser.errores,
        completo,
        tokens: parser.tokens,
    }
}

impl Parser<'_> {
    pub(crate) fn sincronizar_elemento(&mut self) {
        self.avanzar();
        while !self.comprobar(&TipoToken::Fin) && !self.es_inicio_de_elemento() {
            self.avanzar();
        }
    }

    pub(crate) fn sincronizar_sentencia(&mut self) {
        self.avanzar();
        while !self.comprobar(&TipoToken::Fin)
            && !self.comprobar(&TipoToken::LlaveDerecha)
            && !self.es_inicio_de_sentencia()
        {
            self.avanzar();
        }
    }

    fn es_inicio_de_elemento(&mut self) -> bool {
        matches!(
            self.tipo_actual(),
            TipoToken::Importar
                | TipoToken::Exportar
                | TipoToken::Objeto
                | TipoToken::Prototipo
                | TipoToken::Asincrono
        ) || self.es_inicio_de_sentencia()
    }

    fn es_inicio_de_sentencia(&mut self) -> bool {
        matches!(
            self.tipo_actual(),
            TipoToken::Si
                | TipoToken::Mientras
                | TipoToken::Hacer
                | TipoToken::Para
                | TipoToken::Intentar
                | TipoToken::Lanzar
                | TipoToken::Romper
                | TipoToken::Continuar
                | TipoToken::Retornar
        ) || self.clasificar_inicio_declaracion() != ClaseDeclaracion::Ninguna
    }
}
