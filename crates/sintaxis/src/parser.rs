//! Núcleo del parser: cursor de tokens y parseo de un módulo completo.

use ast::{Elemento, Modulo};
use lexico::{TipoToken, Token, tokenizar};
use nucleo::{ErrorQuetzal, Fuente, Ubicacion};

/// Parsea un archivo `.qz` completo y devuelve su [`Modulo`].
pub fn parsear_modulo(fuente: &Fuente) -> Result<Modulo, ErrorQuetzal> {
    Parser::nuevo(fuente)?.parsear_modulo()
}

/// Parser recursivo descendente del Lenguaje Quetzal.
pub struct Parser<'a> {
    pub(crate) fuente: &'a Fuente,
    pub(crate) tokens: Vec<Token>,
    pub(crate) posicion: usize,
}

impl<'a> Parser<'a> {
    /// Crea un parser tokenizando el archivo fuente.
    pub fn nuevo(fuente: &'a Fuente) -> Result<Self, ErrorQuetzal> {
        let tokens = tokenizar(fuente)?;
        Ok(Self {
            fuente,
            tokens,
            posicion: 0,
        })
    }

    /// Parsea el módulo completo hasta el fin del archivo.
    pub fn parsear_modulo(mut self) -> Result<Modulo, ErrorQuetzal> {
        let mut elementos = Vec::new();
        while !self.comprobar(&TipoToken::Fin) {
            elementos.push(self.parsear_elemento()?);
        }
        Ok(Modulo {
            nombre: self.fuente.nombre.clone(),
            elementos,
        })
    }

    /// Parsea un elemento de primer nivel del módulo.
    fn parsear_elemento(&mut self) -> Result<Elemento, ErrorQuetzal> {
        match self.tipo_actual() {
            TipoToken::Importar => Ok(Elemento::Importacion(self.parsear_importacion()?)),
            TipoToken::Exportar => self.parsear_exportacion(),
            TipoToken::Objeto => Ok(Elemento::Objeto(self.parsear_objeto()?)),
            TipoToken::Prototipo => Ok(Elemento::Prototipo(self.parsear_prototipo()?)),
            TipoToken::Asincrono => {
                self.avanzar();
                Ok(Elemento::Funcion(self.parsear_funcion(true)?))
            }
            _ => match self.clasificar_inicio_declaracion() {
                ClaseDeclaracion::Funcion => Ok(Elemento::Funcion(self.parsear_funcion(false)?)),
                ClaseDeclaracion::Variable | ClaseDeclaracion::Ninguna => {
                    Ok(Elemento::Sentencia(self.parsear_sentencia()?))
                }
            },
        }
    }

    // ----- Utilidades de cursor -----

    pub(crate) fn actual(&self) -> &Token {
        // El último token siempre es Fin, así que la posición nunca se pasa.
        &self.tokens[self.posicion.min(self.tokens.len() - 1)]
    }

    pub(crate) fn tipo_actual(&self) -> &TipoToken {
        &self.actual().tipo
    }

    pub(crate) fn siguiente(&self) -> &Token {
        let indice = (self.posicion + 1).min(self.tokens.len() - 1);
        &self.tokens[indice]
    }

    pub(crate) fn ubicacion_actual(&self) -> Ubicacion {
        self.actual().ubicacion
    }

    pub(crate) fn avanzar(&mut self) -> Token {
        let token = self.actual().clone();
        if self.posicion < self.tokens.len() - 1 {
            self.posicion += 1;
        }
        token
    }

    pub(crate) fn comprobar(&self, tipo: &TipoToken) -> bool {
        self.tipo_actual() == tipo
    }

    /// Si el token actual es del tipo dado, lo consume y devuelve `true`.
    pub(crate) fn consumir_si(&mut self, tipo: &TipoToken) -> bool {
        if self.comprobar(tipo) {
            self.avanzar();
            true
        } else {
            false
        }
    }

    /// Exige un token concreto o produce un error sintáctico.
    pub(crate) fn esperar(
        &mut self,
        tipo: &TipoToken,
        descripcion: &str,
    ) -> Result<Token, ErrorQuetzal> {
        if self.comprobar(tipo) {
            Ok(self.avanzar())
        } else {
            Err(self.error_token_inesperado(descripcion))
        }
    }

    /// Texto original de un token (conserva tildes, útil para nombres).
    pub(crate) fn texto_de(&self, token: &Token) -> String {
        self.fuente.fragmento(token.ubicacion).to_string()
    }

    /// Consume un identificador y devuelve su nombre.
    pub(crate) fn esperar_identificador(
        &mut self,
        descripcion: &str,
    ) -> Result<(String, Ubicacion), ErrorQuetzal> {
        match self.tipo_actual().clone() {
            TipoToken::Identificador(nombre) => {
                let token = self.avanzar();
                Ok((nombre, token.ubicacion))
            }
            _ => Err(self.error_token_inesperado(descripcion)),
        }
    }
}

/// Resultado de la mirada hacia adelante para distinguir declaraciones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClaseDeclaracion {
    /// `tipo nombre ( ...` → declaración de función.
    Funcion,
    /// `tipo [var] nombre [= ...]` → declaración de variable.
    Variable,
    /// Cualquier otra cosa (expresión, control de flujo, ...).
    Ninguna,
}
