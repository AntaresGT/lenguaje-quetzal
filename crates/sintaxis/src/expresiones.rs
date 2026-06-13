//! Parseo de expresiones con precedencia de operadores.

use ast::{
    ClaveJsn, Expresion, NodoExpresion, OperadorBinario, OperadorUnario, SegmentoInterpolado,
};
use lexico::{TipoToken, resolver_escapes};
use nucleo::{ErrorQuetzal, Fuente, Ubicacion};

use crate::parser::Parser;

impl Parser<'_> {
    /// Parsea una expresión completa (la precedencia más baja es el ternario).
    pub(crate) fn parsear_expresion(&mut self) -> Result<Expresion, ErrorQuetzal> {
        self.parsear_ternaria()
    }

    fn parsear_ternaria(&mut self) -> Result<Expresion, ErrorQuetzal> {
        let condicion = self.parsear_o_logico()?;
        if !self.consumir_si(&TipoToken::Interrogacion) {
            return Ok(condicion);
        }
        let si_verdadero = self.parsear_expresion()?;
        self.esperar(&TipoToken::DosPuntos, "':' en el operador ternario")?;
        let si_falso = self.parsear_expresion()?;
        let ubicacion = condicion.ubicacion.unir(si_falso.ubicacion);
        Ok(Expresion::nueva(
            NodoExpresion::Ternaria {
                condicion: Box::new(condicion),
                si_verdadero: Box::new(si_verdadero),
                si_falso: Box::new(si_falso),
            },
            ubicacion,
        ))
    }

    fn parsear_o_logico(&mut self) -> Result<Expresion, ErrorQuetzal> {
        let mut izquierda = self.parsear_y_logico()?;
        while self.consumir_si(&TipoToken::OLogico) {
            let derecha = self.parsear_y_logico()?;
            izquierda = combinar(izquierda, OperadorBinario::O, derecha);
        }
        Ok(izquierda)
    }

    fn parsear_y_logico(&mut self) -> Result<Expresion, ErrorQuetzal> {
        let mut izquierda = self.parsear_igualdad()?;
        while self.consumir_si(&TipoToken::YLogico) {
            let derecha = self.parsear_igualdad()?;
            izquierda = combinar(izquierda, OperadorBinario::Y, derecha);
        }
        Ok(izquierda)
    }

    fn parsear_igualdad(&mut self) -> Result<Expresion, ErrorQuetzal> {
        let mut izquierda = self.parsear_comparacion()?;
        loop {
            let operador = match self.tipo_actual() {
                TipoToken::IgualIgual => OperadorBinario::Igual,
                TipoToken::Diferente => OperadorBinario::Diferente,
                _ => break,
            };
            self.avanzar();
            let derecha = self.parsear_comparacion()?;
            izquierda = combinar(izquierda, operador, derecha);
        }
        Ok(izquierda)
    }

    fn parsear_comparacion(&mut self) -> Result<Expresion, ErrorQuetzal> {
        let mut izquierda = self.parsear_aditiva()?;
        loop {
            let operador = match self.tipo_actual() {
                TipoToken::Mayor => OperadorBinario::Mayor,
                TipoToken::Menor => OperadorBinario::Menor,
                TipoToken::MayorOIgual => OperadorBinario::MayorOIgual,
                TipoToken::MenorOIgual => OperadorBinario::MenorOIgual,
                _ => break,
            };
            self.avanzar();
            let derecha = self.parsear_aditiva()?;
            izquierda = combinar(izquierda, operador, derecha);
        }
        Ok(izquierda)
    }

    fn parsear_aditiva(&mut self) -> Result<Expresion, ErrorQuetzal> {
        let mut izquierda = self.parsear_multiplicativa()?;
        loop {
            let operador = match self.tipo_actual() {
                TipoToken::Mas => OperadorBinario::Sumar,
                TipoToken::Menos => OperadorBinario::Restar,
                _ => break,
            };
            self.avanzar();
            let derecha = self.parsear_multiplicativa()?;
            izquierda = combinar(izquierda, operador, derecha);
        }
        Ok(izquierda)
    }

    fn parsear_multiplicativa(&mut self) -> Result<Expresion, ErrorQuetzal> {
        let mut izquierda = self.parsear_unaria()?;
        loop {
            let operador = match self.tipo_actual() {
                TipoToken::Por => OperadorBinario::Multiplicar,
                TipoToken::Entre => OperadorBinario::Dividir,
                TipoToken::Modulo => OperadorBinario::Modulo,
                _ => break,
            };
            self.avanzar();
            let derecha = self.parsear_unaria()?;
            izquierda = combinar(izquierda, operador, derecha);
        }
        Ok(izquierda)
    }

    fn parsear_unaria(&mut self) -> Result<Expresion, ErrorQuetzal> {
        match self.tipo_actual() {
            TipoToken::NoLogico => {
                let inicio = self.avanzar().ubicacion;
                let operando = self.parsear_unaria()?;
                let ubicacion = inicio.unir(operando.ubicacion);
                Ok(Expresion::nueva(
                    NodoExpresion::Unaria {
                        operador: OperadorUnario::NoLogico,
                        operando: Box::new(operando),
                    },
                    ubicacion,
                ))
            }
            TipoToken::Menos => {
                let inicio = self.avanzar().ubicacion;
                let operando = self.parsear_unaria()?;
                let ubicacion = inicio.unir(operando.ubicacion);
                Ok(Expresion::nueva(
                    NodoExpresion::Unaria {
                        operador: OperadorUnario::Negacion,
                        operando: Box::new(operando),
                    },
                    ubicacion,
                ))
            }
            TipoToken::Esperar => {
                let inicio = self.avanzar().ubicacion;
                let operando = self.parsear_unaria()?;
                let ubicacion = inicio.unir(operando.ubicacion);
                Ok(Expresion::nueva(
                    NodoExpresion::Esperar(Box::new(operando)),
                    ubicacion,
                ))
            }
            _ => self.parsear_postfija(),
        }
    }

    /// Llamadas, acceso a miembros e indexación: `obj.metodo(x)[0].campo`.
    fn parsear_postfija(&mut self) -> Result<Expresion, ErrorQuetzal> {
        let mut expresion = self.parsear_primaria()?;
        loop {
            match self.tipo_actual() {
                TipoToken::Punto => {
                    self.avanzar();
                    let (miembro, ubicacion_miembro) = self.parsear_nombre_de_miembro()?;
                    let ubicacion = expresion.ubicacion.unir(ubicacion_miembro);
                    expresion = Expresion::nueva(
                        NodoExpresion::AccesoMiembro {
                            objeto: Box::new(expresion),
                            miembro,
                        },
                        ubicacion,
                    );
                }
                TipoToken::ParentesisIzquierdo => {
                    let argumentos = self.parsear_argumentos()?;
                    let ubicacion = expresion.ubicacion.unir(self.ubicacion_anterior());
                    expresion = Expresion::nueva(
                        NodoExpresion::Llamada {
                            objetivo: Box::new(expresion),
                            argumentos,
                        },
                        ubicacion,
                    );
                }
                TipoToken::CorcheteIzquierdo => {
                    self.avanzar();
                    let indice = self.parsear_expresion()?;
                    self.esperar(&TipoToken::CorcheteDerecho, "']' para cerrar el índice")?;
                    let ubicacion = expresion.ubicacion.unir(self.ubicacion_anterior());
                    expresion = Expresion::nueva(
                        NodoExpresion::Indexacion {
                            objeto: Box::new(expresion),
                            indice: Box::new(indice),
                        },
                        ubicacion,
                    );
                }
                _ => break,
            }
        }
        Ok(expresion)
    }

    /// Nombre de miembro después de un punto. Acepta identificadores y
    /// palabras reservadas (`.texto()`, `.entero()`, `.lista()`, ...).
    fn parsear_nombre_de_miembro(&mut self) -> Result<(String, Ubicacion), ErrorQuetzal> {
        let token = self.actual().clone();
        let texto = self.texto_de(&token);
        let es_palabra = texto
            .chars()
            .next()
            .map(|caracter| caracter.is_alphabetic() || caracter == '_')
            .unwrap_or(false)
            && texto
                .chars()
                .all(|caracter| caracter.is_alphanumeric() || caracter == '_');
        if !es_palabra {
            return Err(self.error_token_inesperado("el nombre de un miembro"));
        }
        self.avanzar();
        Ok((texto, token.ubicacion))
    }

    fn parsear_argumentos(&mut self) -> Result<Vec<Expresion>, ErrorQuetzal> {
        self.esperar(&TipoToken::ParentesisIzquierdo, "'('")?;
        let mut argumentos = Vec::new();
        if !self.comprobar(&TipoToken::ParentesisDerecho) {
            loop {
                argumentos.push(self.parsear_expresion()?);
                if !self.consumir_si(&TipoToken::Coma) {
                    break;
                }
            }
        }
        self.esperar(
            &TipoToken::ParentesisDerecho,
            "')' para cerrar los argumentos",
        )?;
        Ok(argumentos)
    }

    fn parsear_primaria(&mut self) -> Result<Expresion, ErrorQuetzal> {
        let token = self.actual().clone();
        let ubicacion = token.ubicacion;
        let nodo = match token.tipo {
            TipoToken::LiteralEntero(valor) => {
                self.avanzar();
                NodoExpresion::LiteralEntero(valor)
            }
            TipoToken::LiteralNumero(texto) => {
                self.avanzar();
                NodoExpresion::LiteralNumero(texto)
            }
            TipoToken::LiteralTexto(texto) => {
                self.avanzar();
                NodoExpresion::LiteralTexto(texto)
            }
            TipoToken::Verdadero => {
                self.avanzar();
                NodoExpresion::LiteralLog(true)
            }
            TipoToken::Falso => {
                self.avanzar();
                NodoExpresion::LiteralLog(false)
            }
            TipoToken::Nulo => {
                self.avanzar();
                NodoExpresion::Nulo
            }
            TipoToken::Esto => {
                self.avanzar();
                NodoExpresion::Esto
            }
            TipoToken::Padre => {
                self.avanzar();
                NodoExpresion::Padre
            }
            TipoToken::Identificador(nombre) => {
                self.avanzar();
                NodoExpresion::Identificador(nombre)
            }
            TipoToken::TextoInterpolado(crudo) => {
                self.avanzar();
                let segmentos = self.parsear_segmentos_interpolados(&crudo, ubicacion)?;
                NodoExpresion::TextoInterpolado(segmentos)
            }
            TipoToken::Nuevo => {
                self.avanzar();
                let (clase, _) = self.esperar_identificador("el nombre del objeto a instanciar")?;
                let argumentos = self.parsear_argumentos()?;
                NodoExpresion::Nuevo { clase, argumentos }
            }
            TipoToken::ParentesisIzquierdo => {
                self.avanzar();
                let interior = self.parsear_expresion()?;
                self.esperar(
                    &TipoToken::ParentesisDerecho,
                    "')' para cerrar el paréntesis",
                )?;
                let ubicacion_total = ubicacion.unir(self.ubicacion_anterior());
                return Ok(Expresion::nueva(interior.nodo, ubicacion_total));
            }
            TipoToken::CorcheteIzquierdo => {
                self.avanzar();
                let mut elementos = Vec::new();
                if !self.comprobar(&TipoToken::CorcheteDerecho) {
                    loop {
                        elementos.push(self.parsear_expresion()?);
                        if !self.consumir_si(&TipoToken::Coma) {
                            break;
                        }
                    }
                }
                self.esperar(&TipoToken::CorcheteDerecho, "']' para cerrar la lista")?;
                NodoExpresion::ListaLiteral(elementos)
            }
            TipoToken::LlaveIzquierda => {
                return self.parsear_jsn_literal();
            }
            _ => return Err(self.error_se_esperaba_expresion()),
        };

        let ubicacion_total = ubicacion.unir(self.ubicacion_anterior());
        Ok(Expresion::nueva(nodo, ubicacion_total))
    }

    /// Literal `jsn`: `{ clave: valor, "otra clave": 1 }`.
    ///
    /// Las comas entre entradas son opcionales para tolerar JSON escrito en
    /// varias líneas (los ejemplos del lenguaje incluyen entradas separadas
    /// solo por salto de línea).
    fn parsear_jsn_literal(&mut self) -> Result<Expresion, ErrorQuetzal> {
        let inicio = self
            .esperar(&TipoToken::LlaveIzquierda, "'{' para abrir el jsn")?
            .ubicacion;
        let mut entradas = Vec::new();

        while !self.comprobar(&TipoToken::LlaveDerecha) && !self.comprobar(&TipoToken::Fin) {
            let clave = self.parsear_clave_jsn()?;
            self.esperar(&TipoToken::DosPuntos, "':' después de la clave")?;
            let valor = self.parsear_expresion()?;
            entradas.push((clave, valor));
            // Coma opcional entre entradas.
            self.consumir_si(&TipoToken::Coma);
        }

        self.esperar(&TipoToken::LlaveDerecha, "'}' para cerrar el jsn")?;
        let ubicacion = inicio.unir(self.ubicacion_anterior());
        Ok(Expresion::nueva(
            NodoExpresion::JsnLiteral(entradas),
            ubicacion,
        ))
    }

    fn parsear_clave_jsn(&mut self) -> Result<ClaveJsn, ErrorQuetzal> {
        let token = self.actual().clone();
        match token.tipo {
            TipoToken::Identificador(nombre) => {
                self.avanzar();
                Ok(ClaveJsn::Identificador(nombre))
            }
            TipoToken::LiteralTexto(texto) => {
                self.avanzar();
                Ok(ClaveJsn::Texto(texto))
            }
            _ => {
                // Palabras reservadas como clave (`tipo: "casa"`).
                let texto = self.texto_de(&token);
                let es_palabra = texto
                    .chars()
                    .next()
                    .map(|caracter| caracter.is_alphabetic() || caracter == '_')
                    .unwrap_or(false);
                if es_palabra {
                    self.avanzar();
                    Ok(ClaveJsn::Identificador(texto))
                } else {
                    Err(self.error_token_inesperado("una clave de jsn"))
                }
            }
        }
    }

    /// Separa el contenido crudo de un `t"..."` en segmentos de texto y
    /// expresiones `{...}`. Las expresiones se parsean con un sub-parser
    /// cuyo offset coincide con el archivo original para que los
    /// diagnósticos apunten al lugar correcto.
    fn parsear_segmentos_interpolados(
        &self,
        crudo: &str,
        ubicacion: Ubicacion,
    ) -> Result<Vec<SegmentoInterpolado>, ErrorQuetzal> {
        // El contenido crudo empieza después de `t"`.
        let offset_base = ubicacion.inicio + 2;
        let mut segmentos = Vec::new();
        let mut texto_actual = String::new();
        let bytes = crudo.as_bytes();
        let mut indice = 0;

        while indice < bytes.len() {
            let resto = &crudo[indice..];
            let caracter = match resto.chars().next() {
                Some(caracter) => caracter,
                None => break,
            };

            if caracter == '\\' {
                // Conserva el escape completo; se resuelve al cerrar el segmento.
                texto_actual.push('\\');
                indice += 1;
                if let Some(escapado) = crudo[indice..].chars().next() {
                    texto_actual.push(escapado);
                    indice += escapado.len_utf8();
                }
                continue;
            }

            if caracter == '{' {
                if !texto_actual.is_empty() {
                    segmentos.push(SegmentoInterpolado::Texto(resolver_escapes(&texto_actual)));
                    texto_actual.clear();
                }
                let inicio_expresion = indice + 1;
                let fin_expresion =
                    buscar_llave_de_cierre(crudo, inicio_expresion).ok_or_else(|| {
                        nucleo::ErrorQuetzal::nuevo(
                            "E0103",
                            nucleo::CategoriaError::Sintactico,
                            "falta '}' para cerrar la interpolación",
                        )
                        .con_ubicacion(Ubicacion::nueva(
                            offset_base + indice,
                            offset_base + indice + 1,
                        ))
                        .con_archivo(&self.fuente.nombre)
                        .con_etiqueta("esta llave nunca se cierra")
                    })?;

                let texto_expresion = &crudo[inicio_expresion..fin_expresion];
                let expresion = self.parsear_expresion_incrustada(
                    texto_expresion,
                    offset_base + inicio_expresion,
                )?;
                segmentos.push(SegmentoInterpolado::Expresion(expresion));
                indice = fin_expresion + 1;
                continue;
            }

            texto_actual.push(caracter);
            indice += caracter.len_utf8();
        }

        if !texto_actual.is_empty() {
            segmentos.push(SegmentoInterpolado::Texto(resolver_escapes(&texto_actual)));
        }

        Ok(segmentos)
    }

    /// Parsea una expresión incrustada en una interpolación. Se rellena el
    /// inicio con espacios para que las ubicaciones coincidan con el archivo
    /// original.
    fn parsear_expresion_incrustada(
        &self,
        texto: &str,
        offset_absoluto: usize,
    ) -> Result<Expresion, ErrorQuetzal> {
        let relleno = " ".repeat(offset_absoluto);
        let contenido = format!("{relleno}{texto}");
        let sub_fuente = Fuente::nueva(self.fuente.nombre.clone(), contenido);
        let mut sub_parser = Parser::nuevo(&sub_fuente)?;
        let expresion = sub_parser.parsear_expresion()?;
        if !sub_parser.comprobar(&TipoToken::Fin) {
            return Err(sub_parser.error_token_inesperado("el final de la interpolación"));
        }
        Ok(expresion)
    }

    /// Ubicación del token recién consumido.
    pub(crate) fn ubicacion_anterior(&self) -> Ubicacion {
        if self.posicion == 0 {
            return self.ubicacion_actual();
        }
        self.tokens[self.posicion - 1].ubicacion
    }
}

fn combinar(izquierda: Expresion, operador: OperadorBinario, derecha: Expresion) -> Expresion {
    let ubicacion = izquierda.ubicacion.unir(derecha.ubicacion);
    Expresion::nueva(
        NodoExpresion::Binaria {
            operador,
            izquierda: Box::new(izquierda),
            derecha: Box::new(derecha),
        },
        ubicacion,
    )
}

/// Busca la `}` que cierra una interpolación, respetando llaves anidadas y
/// textos entre comillas dentro de la expresión.
fn buscar_llave_de_cierre(texto: &str, desde: usize) -> Option<usize> {
    let mut profundidad = 1usize;
    let mut dentro_de_texto = false;
    let mut indice = desde;
    let bytes = texto.as_bytes();

    while indice < bytes.len() {
        let byte = bytes[indice];
        match byte {
            b'\\' => {
                indice += 1; // salta el carácter escapado
            }
            b'"' => dentro_de_texto = !dentro_de_texto,
            b'{' if !dentro_de_texto => profundidad += 1,
            b'}' if !dentro_de_texto => {
                profundidad -= 1;
                if profundidad == 0 {
                    return Some(indice);
                }
            }
            _ => {}
        }
        indice += 1;
    }
    None
}
