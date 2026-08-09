//! Parseo de sentencias y bloques.

use ast::{Bloque, Captura, NodoSentencia, OperadorAsignacion, Sentencia};
use lexico::TipoToken;
use nucleo::ErrorQuetzal;

use crate::parser::{ClaseDeclaracion, Parser};

impl Parser<'_> {
    /// Parsea un bloque `{ ... }` con declaraciones y sentencias.
    pub(crate) fn parsear_bloque(&mut self) -> Result<Bloque, ErrorQuetzal> {
        self.esperar(&TipoToken::LlaveIzquierda, "'{' para abrir el bloque")?;
        let mut sentencias = Vec::new();
        while !self.comprobar(&TipoToken::LlaveDerecha) && !self.comprobar(&TipoToken::Fin) {
            sentencias.push(self.parsear_sentencia()?);
        }
        self.esperar(&TipoToken::LlaveDerecha, "'}' para cerrar el bloque")?;
        Ok(Bloque { sentencias })
    }

    /// Parsea una sentencia completa (incluidas declaraciones de variables).
    pub(crate) fn parsear_sentencia(&mut self) -> Result<Sentencia, ErrorQuetzal> {
        match self.tipo_actual() {
            TipoToken::Si => self.parsear_si(),
            TipoToken::Mientras => self.parsear_mientras(),
            TipoToken::Hacer => self.parsear_hacer_mientras(),
            TipoToken::Para => self.parsear_para(),
            TipoToken::Intentar => self.parsear_intentar(),
            TipoToken::Lanzar => {
                let inicio = self.avanzar().ubicacion;
                let expresion = self.parsear_expresion()?;
                let fin = expresion.ubicacion;
                Ok(Sentencia::nueva(
                    NodoSentencia::Lanzar(expresion),
                    inicio.unir(fin),
                ))
            }
            TipoToken::Romper => {
                let ubicacion = self.avanzar().ubicacion;
                Ok(Sentencia::nueva(NodoSentencia::Romper, ubicacion))
            }
            TipoToken::Continuar => {
                let ubicacion = self.avanzar().ubicacion;
                Ok(Sentencia::nueva(NodoSentencia::Continuar, ubicacion))
            }
            TipoToken::Retornar => {
                let inicio = self.avanzar().ubicacion;
                // `retornar` sin valor: el siguiente token cierra el bloque.
                if self.comprobar(&TipoToken::LlaveDerecha) || self.comprobar(&TipoToken::Fin) {
                    return Ok(Sentencia::nueva(NodoSentencia::Retornar(None), inicio));
                }
                let expresion = self.parsear_expresion()?;
                let fin = expresion.ubicacion;
                Ok(Sentencia::nueva(
                    NodoSentencia::Retornar(Some(expresion)),
                    inicio.unir(fin),
                ))
            }
            _ => {
                if self.clasificar_inicio_declaracion() == ClaseDeclaracion::Variable {
                    self.parsear_declaracion_variable()
                } else {
                    self.parsear_sentencia_simple()
                }
            }
        }
    }

    /// Sentencia simple: asignación, incremento/decremento o expresión.
    /// Se usa también como inicialización y paso del bucle `para`.
    pub(crate) fn parsear_sentencia_simple(&mut self) -> Result<Sentencia, ErrorQuetzal> {
        let expresion = self.parsear_expresion()?;
        let inicio = expresion.ubicacion;

        let operador = match self.tipo_actual() {
            TipoToken::Asignar => Some(OperadorAsignacion::Asignar),
            TipoToken::MasIgual => Some(OperadorAsignacion::Sumar),
            TipoToken::MenosIgual => Some(OperadorAsignacion::Restar),
            TipoToken::PorIgual => Some(OperadorAsignacion::Multiplicar),
            TipoToken::EntreIgual => Some(OperadorAsignacion::Dividir),
            TipoToken::ModuloIgual => Some(OperadorAsignacion::Modulo),
            TipoToken::Incremento => {
                self.avanzar();
                return Ok(Sentencia::nueva(
                    NodoSentencia::IncrementoDecremento {
                        objetivo: expresion,
                        incremento: true,
                    },
                    inicio,
                ));
            }
            TipoToken::Decremento => {
                self.avanzar();
                return Ok(Sentencia::nueva(
                    NodoSentencia::IncrementoDecremento {
                        objetivo: expresion,
                        incremento: false,
                    },
                    inicio,
                ));
            }
            _ => None,
        };

        if let Some(operador) = operador {
            self.avanzar();
            let valor = self.parsear_expresion()?;
            let fin = valor.ubicacion;
            return Ok(Sentencia::nueva(
                NodoSentencia::Asignacion {
                    objetivo: expresion,
                    operador,
                    valor,
                },
                inicio.unir(fin),
            ));
        }

        Ok(Sentencia::nueva(
            NodoSentencia::Expresion(expresion),
            inicio,
        ))
    }

    fn parsear_si(&mut self) -> Result<Sentencia, ErrorQuetzal> {
        let inicio = self.esperar(&TipoToken::Si, "'si'")?.ubicacion;
        self.esperar(&TipoToken::ParentesisIzquierdo, "'(' después de 'si'")?;
        let condicion = self.parsear_expresion()?;
        self.esperar(
            &TipoToken::ParentesisDerecho,
            "')' para cerrar la condición",
        )?;
        let entonces = self.parsear_bloque()?;

        let sino = if self.consumir_si(&TipoToken::Sino) {
            if self.comprobar(&TipoToken::Si) {
                // `sino si`: se parsea como un `si` anidado dentro del bloque.
                let anidado = self.parsear_si()?;
                Some(Bloque {
                    sentencias: vec![anidado],
                })
            } else {
                Some(self.parsear_bloque()?)
            }
        } else {
            None
        };

        Ok(Sentencia::nueva(
            NodoSentencia::Si {
                condicion,
                entonces,
                sino,
            },
            inicio,
        ))
    }

    fn parsear_mientras(&mut self) -> Result<Sentencia, ErrorQuetzal> {
        let inicio = self.esperar(&TipoToken::Mientras, "'mientras'")?.ubicacion;
        self.esperar(&TipoToken::ParentesisIzquierdo, "'(' después de 'mientras'")?;
        let condicion = self.parsear_expresion()?;
        self.esperar(
            &TipoToken::ParentesisDerecho,
            "')' para cerrar la condición",
        )?;
        let cuerpo = self.parsear_bloque()?;
        Ok(Sentencia::nueva(
            NodoSentencia::Mientras { condicion, cuerpo },
            inicio,
        ))
    }

    fn parsear_hacer_mientras(&mut self) -> Result<Sentencia, ErrorQuetzal> {
        let inicio = self.esperar(&TipoToken::Hacer, "'hacer'")?.ubicacion;
        let cuerpo = self.parsear_bloque()?;
        self.esperar(
            &TipoToken::Mientras,
            "'mientras' después del bloque 'hacer'",
        )?;
        self.esperar(&TipoToken::ParentesisIzquierdo, "'(' después de 'mientras'")?;
        let condicion = self.parsear_expresion()?;
        self.esperar(
            &TipoToken::ParentesisDerecho,
            "')' para cerrar la condición",
        )?;
        Ok(Sentencia::nueva(
            NodoSentencia::HacerMientras { cuerpo, condicion },
            inicio,
        ))
    }

    fn parsear_para(&mut self) -> Result<Sentencia, ErrorQuetzal> {
        let inicio = self.esperar(&TipoToken::Para, "'para'")?.ubicacion;
        self.esperar(&TipoToken::ParentesisIzquierdo, "'(' después de 'para'")?;

        // Ambas formas inician con: tipo [var] nombre
        let tipo_elemento = self.parsear_tipo()?;
        let mutable = self.consumir_si(&TipoToken::Var);
        let (nombre, _) = self.esperar_identificador("el nombre de la variable del bucle")?;

        match self.tipo_actual() {
            // `para (entero var valor en lista)` / `... cada lista`
            TipoToken::En | TipoToken::Cada => {
                self.avanzar();
                let iterable = self.parsear_expresion()?;
                self.esperar(&TipoToken::ParentesisDerecho, "')' para cerrar el 'para'")?;
                let cuerpo = self.parsear_bloque()?;
                Ok(Sentencia::nueva(
                    NodoSentencia::ParaEn {
                        tipo_elemento,
                        mutable,
                        nombre,
                        iterable,
                        cuerpo,
                    },
                    inicio,
                ))
            }
            // `para (entero var i = 0; i < 5; i++)`
            TipoToken::Asignar => {
                self.avanzar();
                let valor = self.parsear_expresion()?;
                let fin_valor = valor.ubicacion;
                let inicializacion = Sentencia::nueva(
                    NodoSentencia::DeclaracionVariable {
                        tipo: tipo_elemento,
                        mutable,
                        nombre,
                        valor: Some(valor),
                    },
                    inicio.unir(fin_valor),
                );
                self.esperar(
                    &TipoToken::PuntoYComa,
                    "';' después de la inicialización del 'para'",
                )?;
                let condicion = self.parsear_expresion()?;
                self.esperar(
                    &TipoToken::PuntoYComa,
                    "';' después de la condición del 'para'",
                )?;
                let paso = self.parsear_sentencia_simple()?;
                self.esperar(&TipoToken::ParentesisDerecho, "')' para cerrar el 'para'")?;
                let cuerpo = self.parsear_bloque()?;
                Ok(Sentencia::nueva(
                    NodoSentencia::ParaClasico {
                        inicializacion: Box::new(inicializacion),
                        condicion,
                        paso: Box::new(paso),
                        cuerpo,
                    },
                    inicio,
                ))
            }
            _ => Err(self.error_token_inesperado("'=', 'en' o 'cada' en el bucle 'para'")),
        }
    }

    fn parsear_intentar(&mut self) -> Result<Sentencia, ErrorQuetzal> {
        let inicio = self.esperar(&TipoToken::Intentar, "'intentar'")?.ubicacion;
        let bloque = self.parsear_bloque()?;

        let captura = if self.consumir_si(&TipoToken::Capturar) {
            self.esperar(&TipoToken::ParentesisIzquierdo, "'(' después de 'capturar'")?;
            self.esperar(&TipoToken::Excepcion, "'excepcion'")?;
            let (nombre, _) = self.esperar_identificador("el nombre de la excepción")?;
            self.esperar(&TipoToken::ParentesisDerecho, "')' después de la excepción")?;
            let bloque_captura = self.parsear_bloque()?;
            Some(Captura {
                nombre,
                bloque: bloque_captura,
            })
        } else {
            None
        };

        let finalmente = if self.consumir_si(&TipoToken::Finalmente) {
            Some(self.parsear_bloque()?)
        } else {
            None
        };

        if captura.is_none() && finalmente.is_none() {
            return Err(
                self.error_token_inesperado("'capturar' o 'finalmente' después de 'intentar'")
            );
        }

        Ok(Sentencia::nueva(
            NodoSentencia::Intentar {
                bloque,
                captura,
                finalmente,
            },
            inicio,
        ))
    }
}
