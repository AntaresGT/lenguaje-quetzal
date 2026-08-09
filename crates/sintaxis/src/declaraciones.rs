//! Parseo de tipos, variables y funciones.

use ast::{Funcion, NodoSentencia, Parametro, Sentencia, Tipo};
use lexico::TipoToken;
use nucleo::ErrorQuetzal;

use crate::parser::{ClaseDeclaracion, Parser};

impl Parser<'_> {
    /// Mira hacia adelante para decidir si lo que sigue es una declaración
    /// de función, de variable, o ninguna de las dos. No consume tokens.
    pub(crate) fn clasificar_inicio_declaracion(&mut self) -> ClaseDeclaracion {
        let posicion_guardada = self.posicion;
        let clase = self.clasificar_inicio_declaracion_interno();
        self.posicion = posicion_guardada;
        clase
    }

    fn clasificar_inicio_declaracion_interno(&mut self) -> ClaseDeclaracion {
        if !self.es_inicio_de_tipo() {
            return ClaseDeclaracion::Ninguna;
        }
        if self.parsear_tipo().is_err() {
            return ClaseDeclaracion::Ninguna;
        }
        match self.tipo_actual() {
            TipoToken::Var => ClaseDeclaracion::Variable,
            TipoToken::Identificador(_) => {
                if matches!(self.siguiente().tipo, TipoToken::ParentesisIzquierdo) {
                    ClaseDeclaracion::Funcion
                } else {
                    ClaseDeclaracion::Variable
                }
            }
            _ => ClaseDeclaracion::Ninguna,
        }
    }

    /// Si el token actual puede iniciar un tipo (`entero`, `lista`, `Usuario`, ...).
    pub(crate) fn es_inicio_de_tipo(&self) -> bool {
        matches!(
            self.tipo_actual(),
            TipoToken::TipoEntero
                | TipoToken::TipoNumero
                | TipoToken::TipoTexto
                | TipoToken::TipoLog
                | TipoToken::TipoJsn
                | TipoToken::TipoVacio
                | TipoToken::TipoLista
                | TipoToken::Identificador(_)
        )
    }

    /// Parsea un tipo: `entero`, `número`, `lista`, `lista<entero>`, `Usuario`, ...
    pub(crate) fn parsear_tipo(&mut self) -> Result<Tipo, ErrorQuetzal> {
        let tipo = match self.tipo_actual().clone() {
            TipoToken::TipoEntero => {
                self.avanzar();
                Tipo::Entero
            }
            TipoToken::TipoNumero => {
                self.avanzar();
                Tipo::Numero
            }
            TipoToken::TipoTexto => {
                self.avanzar();
                Tipo::Texto
            }
            TipoToken::TipoLog => {
                self.avanzar();
                Tipo::Log
            }
            TipoToken::TipoJsn => {
                self.avanzar();
                Tipo::Jsn
            }
            TipoToken::TipoVacio => {
                self.avanzar();
                Tipo::Vacio
            }
            TipoToken::TipoLista => {
                self.avanzar();
                if self.consumir_si(&TipoToken::Menor) {
                    let interior = self.parsear_tipo()?;
                    self.esperar(&TipoToken::Mayor, "'>' para cerrar el tipo de la lista")?;
                    Tipo::Lista(Some(Box::new(interior)))
                } else {
                    Tipo::Lista(None)
                }
            }
            TipoToken::Identificador(nombre) => {
                self.avanzar();
                Tipo::Nombrado(nombre)
            }
            _ => return Err(self.error_token_inesperado("un tipo")),
        };
        Ok(tipo)
    }

    /// Parsea una declaración de variable: `entero var contador = 0`.
    pub(crate) fn parsear_declaracion_variable(&mut self) -> Result<Sentencia, ErrorQuetzal> {
        let inicio = self.ubicacion_actual();
        let tipo = self.parsear_tipo()?;
        let mutable = self.consumir_si(&TipoToken::Var);
        let (nombre, ubicacion_nombre) = self.esperar_identificador("el nombre de la variable")?;

        let valor = if self.consumir_si(&TipoToken::Asignar) {
            Some(self.parsear_expresion()?)
        } else {
            None
        };

        let fin = valor
            .as_ref()
            .map(|expresion| expresion.ubicacion)
            .unwrap_or(ubicacion_nombre);
        Ok(Sentencia::nueva(
            NodoSentencia::DeclaracionVariable {
                tipo,
                mutable,
                nombre,
                valor,
            },
            inicio.unir(fin),
        ))
    }

    /// Parsea una declaración de función completa (sin el `asincrono`, que
    /// consume quien llama).
    pub(crate) fn parsear_funcion(&mut self, asincrona: bool) -> Result<Funcion, ErrorQuetzal> {
        let inicio = self.ubicacion_actual();
        let tipo_retorno = self.parsear_tipo()?;
        let (nombre, _) = self.esperar_identificador("el nombre de la función")?;
        let parametros = self.parsear_parametros()?;
        let cuerpo = self.parsear_bloque()?;

        Ok(Funcion {
            nombre,
            tipo_retorno,
            parametros,
            cuerpo,
            asincrona,
            ubicacion: inicio,
        })
    }

    /// Parsea la lista de parámetros: `(texto var palabra, entero edad)`.
    pub(crate) fn parsear_parametros(&mut self) -> Result<Vec<Parametro>, ErrorQuetzal> {
        self.esperar(
            &TipoToken::ParentesisIzquierdo,
            "'(' para abrir los parámetros",
        )?;
        let mut parametros = Vec::new();

        if !self.comprobar(&TipoToken::ParentesisDerecho) {
            loop {
                let inicio = self.ubicacion_actual();
                let tipo = self.parsear_tipo()?;
                let mutable = self.consumir_si(&TipoToken::Var);
                let (nombre, ubicacion_nombre) =
                    self.esperar_identificador("el nombre del parámetro")?;
                parametros.push(Parametro {
                    tipo,
                    mutable,
                    nombre,
                    ubicacion: inicio.unir(ubicacion_nombre),
                });
                if !self.consumir_si(&TipoToken::Coma) {
                    break;
                }
            }
        }

        self.esperar(
            &TipoToken::ParentesisDerecho,
            "')' para cerrar los parámetros",
        )?;
        Ok(parametros)
    }
}
