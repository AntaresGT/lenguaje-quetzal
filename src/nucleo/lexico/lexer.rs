use crate::errores::{Error, CodigoError, Resultado};
use crate::nucleo::lexico::token::{Token, Posicion, TokenConPosicion};
use logos::Logos;

/// Token interno para logos (simplificado)
#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\r]+")] // Saltar espacios, tabs y retornos de carro
#[logos(skip r"//[^\n]*")] // Saltar comentarios de línea
#[logos(skip r"\n")] // Saltar saltos de línea (los procesamos explícitamente después)
pub enum TokenLogos {
    // Palabras reservadas - Tipos
    #[token("vacio")]
    #[token("vacío")]
    Vacio,
    
    #[token("entero")]
    Entero,
    
    #[token("número")]
    #[token("numero")]
    Numero,
    
    #[token("texto")]
    Texto,
    
    #[token("log")]
    #[token("lóg")]
    Log,
    
    #[token("lista")]
    Lista,
    
    #[token("jsn")]
    Jsn,
    
    // Valores literales
    #[token("verdadero")]
    Verdadero,
    
    #[token("falso")]
    Falso,
    
    #[token("nulo")]
    Nulo,
    
    // Modificadores
    #[token("var")]
    Var,

    #[token("opcional")]
    Opcional,
    
    #[token("publico")]
    #[token("público")]
    Publico,
    
    #[token("privado")]
    Privado,
    
    #[token("libre")]
    Libre,
    
    // Funciones y objetos
    #[token("retornar")]
    Retornar,
    
    #[token("objeto")]
    Objeto,

    #[token("prototipo")]
    Prototipo,

    #[token("implementa")]
    Implementa,
    
    #[token("nuevo")]
    Nuevo,
    
    #[token("ambiente")]
    Ambiente,
    
    #[token("padre")]
    Padre,
    
    // Programación asíncrona
    #[token("asincrono")]
    #[token("asíncrono")]
    #[token("asincróno")]
    Asincrono,
    
    #[token("esperar")]
    Esperar,
    
    // Control de flujo
    #[token("si")]
    Si,
    
    #[token("sino")]
    Sino,
    
    #[token("mientras")]
    Mientras,
    
    #[token("para")]
    Para,
    
    #[token("hacer")]
    Hacer,
    
    #[token("romper")]
    Romper,
    
    #[token("continuar")]
    Continuar,
    
    #[token("cada")]
    Cada,
    
    // Manejo de excepciones
    #[token("intentar")]
    Intentar,
    
    #[token("capturar")]
    Capturar,
    
    #[token("finalmente")]
    Finalmente,
    
    #[token("lanzar")]
    Lanzar,
    
    #[token("excepcion")]
    #[token("excepción")]
    Excepcion,
    
    // Módulos
    #[token("importar")]
    Importar,
    
    #[token("exportar")]
    Exportar,
    
    #[token("desde")]
    Desde,
    
    #[token("como")]
    Como,
    
    // Operadores lógicos (deben ir antes del regex de identificadores)
    #[token("y", priority = 3)]
    Y,
    
    #[token("o", priority = 3)]
    #[token("ó", priority = 3)]
    O,
    
    // Operadores
    #[token("++")]
    Incrementar,
    
    #[token("--")]
    Decrementar,
    
    #[token("**")]
    Potencia,
    
    #[token("+=")]
    MasAsignar,
    
    #[token("-=")]
    MenosAsignar,
    
    #[token("*=")]
    MultiplicarAsignar,
    
    #[token("/=")]
    DividirAsignar,
    
    #[token("%=")]
    ModuloAsignar,
    
    #[token("==")]
    Igual,
    
    #[token("!=")]
    Diferente,
    
    #[token(">=")]
    MayorIgual,
    
    #[token("<=")]
    MenorIgual,
    
    #[token("+")]
    Mas,
    
    #[token("-")]
    Menos,
    
    #[token("*")]
    Multiplicar,
    
    #[token("/")]
    Dividir,
    
    #[token("%")]
    Modulo,
    
    #[token(">")]
    Mayor,
    
    #[token("<")]
    Menor,
    
    #[token("=")]
    Asignar,
    
    #[token("!")]
    Negacion,
    
    #[token("?")]
    Ternario,
    
    #[token(":")]
    DosPuntos,
    
    // Delimitadores
    #[token("(")]
    ParentesisIzq,
    
    #[token(")")]
    ParentesisDer,
    
    #[token("{")]
    LlaveIzq,
    
    #[token("}")]
    LlaveDer,
    
    #[token("[")]
    CorcheteIzq,
    
    #[token("]")]
    CorcheteDer,
    
    #[token(".")]
    Punto,
    
    #[token(",")]
    Coma,
    
    #[token(";")]
    PuntoYComa,
    
    // Literales - guardamos el texto crudo y lo procesamos después
    // Usamos un patrón que maneja escapes: permite cualquier carácter o \ seguido de cualquier carácter
    #[regex(r#""([^"\\]|\\.)*""#)]
    CadenaRaw,
    
    #[regex(r#"t"([^"\\]|\\.)*""#)]
    InterpolacionTextoRaw,
    
    #[regex(r"\d+")]
    EnteroLiteralRaw,
    
    #[regex(r"\d+\.\d+")]
    NumeroLiteralRaw,
    
    #[regex(r"[a-zA-Z_áéíóúñüÁÉÍÓÚÑÜ][a-zA-Z0-9_áéíóúñüÁÉÍÓÚÑÜ]*", priority = 1)]
    IdentificadorRaw,
    
    // Nueva línea - ya no la necesitamos aquí porque la saltamos arriba
    // #[regex(r"\n")]
    // NuevaLinea,
    
    #[regex(r"/\*")]
    InicioComentarioBloque,
    
    Error,
}

fn procesar_cadena(input: &str) -> Option<String> {
    let mut resultado = String::new();
    let mut chars = input.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => resultado.push('\n'),
                Some('t') => resultado.push('\t'),
                Some('r') => resultado.push('\r'),
                Some('\\') => resultado.push('\\'),
                Some('"') => resultado.push('"'),
                Some(c) => {
                    resultado.push('\\');
                    resultado.push(c);
                }
                None => return None, // Escape incompleto
            }
        } else {
            resultado.push(ch);
        }
    }
    
    Some(resultado)
}

/// Analizador léxico para Quetzal
pub struct Lexer {
    fuente: String,
    tokens_logos: Vec<(TokenLogos, usize, usize)>, // token, inicio, fin
    posicion_actual: usize,
    linea_actual: usize,
    columna_actual: usize,
}

impl Lexer {
    /// Crea un nuevo lexer a partir del código fuente
    pub fn nuevo(fuente: &str) -> Resultado<Self> {
        // Procesar comentarios de bloque primero
        let fuente_procesada = Self::procesar_comentarios_bloque(fuente)?;
        let mut lexer_logos = TokenLogos::lexer(&fuente_procesada);
        let mut tokens = Vec::new();
        
        while let Some(token_result) = lexer_logos.next() {
            let span = lexer_logos.span();
            let token = match token_result {
                Ok(t) => t,
                Err(_) => {
                    let contexto = if span.start < fuente_procesada.len() {
                        let inicio = span.start.saturating_sub(10).max(0);
                        let fin = (span.start + 20).min(fuente_procesada.len());
                        format!("contexto: '{}'", &fuente_procesada[inicio..fin])
                    } else {
                        "sin contexto disponible".to_string()
                    };
                    return Err(Error::analisis(
                        CodigoError::CaracterInvalido,
                        format!("error al procesar token en posición {} ({})", span.start, contexto),
                        None,
                        None,
                        None,
                    ));
                }
            };
            
            match token {
                TokenLogos::InicioComentarioBloque => {
                    return Err(Error::analisis(
                        CodigoError::ComentarioSinCerrar,
                        "comentario de bloque sin cerrar",
                        None,
                        None,
                        None,
                    ));
                }
                TokenLogos::Error => {
                    return Err(Error::analisis(
                        CodigoError::CaracterInvalido,
                        format!("carácter inválido: '{}'", &fuente_procesada[span.start..span.end]),
                        None,
                        None,
                        None,
                    ));
                }
                _ => {
                    tokens.push((token, span.start, span.end));
                }
            }
        }
        
        Ok(Self {
            fuente: fuente_procesada,
            tokens_logos: tokens,
            posicion_actual: 0,
            linea_actual: 1,
            columna_actual: 1,
        })
    }
    
    fn procesar_comentarios_bloque(fuente: &str) -> Resultado<String> {
        let mut resultado = String::new();
        let mut chars = fuente.chars().peekable();
        let mut en_comentario = false;

        
        while let Some(ch) = chars.next() {
            if en_comentario {
                if ch == '*' {
                    if let Some('/') = chars.peek() {
                        chars.next();
                        en_comentario = false;
                        continue;
                    }
                }
                // Reemplazar contenido del comentario con espacios para mantener posiciones
                resultado.push(' ');
            } else {
                if ch == '/' {
                    if let Some('*') = chars.peek() {
                        chars.next();
                        en_comentario = true;
                        resultado.push(' ');
                        continue;
                    }
                }
                resultado.push(ch);
            }

        }
        
        if en_comentario {
            return Err(Error::analisis(
                CodigoError::ComentarioSinCerrar,
                "comentario de bloque sin cerrar",
                None,
                None,
                None,
            ));
        }
        
        Ok(resultado)
    }
    
    /// Obtiene el siguiente token
    pub fn siguiente(&mut self) -> Resultado<Option<TokenConPosicion>> {
        if self.posicion_actual >= self.tokens_logos.len() {
            return Ok(None);
        }
        
        let (token_logos, inicio, fin) = &self.tokens_logos[self.posicion_actual];
        let token = self.convertir_token(token_logos, *inicio, *fin)?;
        
        // Calcular posición
        let linea = self.calcular_linea(*inicio);
        let columna = self.calcular_columna(*inicio, linea);
        
        let posicion = Posicion::nueva(linea, columna, *inicio);
        let token_con_posicion = TokenConPosicion::nueva(token, posicion);
        
        self.posicion_actual += 1;
        self.linea_actual = linea;
        self.columna_actual = columna;
        
        Ok(Some(token_con_posicion))
    }
    
    fn convertir_token(&self, token_logos: &TokenLogos, inicio: usize, fin: usize) -> Resultado<Token> {
        Ok(match token_logos {
            TokenLogos::Vacio => Token::Vacio,
            TokenLogos::Entero => Token::Entero,
            TokenLogos::Numero => Token::Numero,
            TokenLogos::Texto => Token::Texto,
            TokenLogos::Log => Token::Log,
            TokenLogos::Lista => Token::Lista,
            TokenLogos::Jsn => Token::Jsn,
            TokenLogos::Verdadero => Token::Verdadero,
            TokenLogos::Falso => Token::Falso,
            TokenLogos::Nulo => Token::Nulo,
            TokenLogos::Var => Token::Var,
            TokenLogos::Opcional => Token::Opcional,
            TokenLogos::Publico => Token::Publico,
            TokenLogos::Privado => Token::Privado,
            TokenLogos::Libre => Token::Libre,
            TokenLogos::Retornar => Token::Retornar,
            TokenLogos::Objeto => Token::Objeto,
            TokenLogos::Prototipo => Token::Prototipo,
            TokenLogos::Implementa => Token::Implementa,
            TokenLogos::Nuevo => Token::Nuevo,
            TokenLogos::Ambiente => Token::Ambiente,
            TokenLogos::Padre => Token::Padre,
            TokenLogos::Asincrono => Token::Asincrono,
            TokenLogos::Esperar => Token::Esperar,
            TokenLogos::Si => Token::Si,
            TokenLogos::Sino => Token::Sino,
            TokenLogos::Mientras => Token::Mientras,
            TokenLogos::Para => Token::Para,
            TokenLogos::Hacer => Token::Hacer,
            TokenLogos::Romper => Token::Romper,
            TokenLogos::Continuar => Token::Continuar,
            TokenLogos::Cada => Token::Cada,
            TokenLogos::Intentar => Token::Intentar,
            TokenLogos::Capturar => Token::Capturar,
            TokenLogos::Finalmente => Token::Finalmente,
            TokenLogos::Lanzar => Token::Lanzar,
            TokenLogos::Excepcion => Token::Excepcion,
            TokenLogos::Importar => Token::Importar,
            TokenLogos::Exportar => Token::Exportar,
            TokenLogos::Desde => Token::Desde,
            TokenLogos::Como => Token::Como,
            TokenLogos::Y => Token::Y,
            TokenLogos::O => Token::O,
            TokenLogos::Mas => Token::Mas,
            TokenLogos::Menos => Token::Menos,
            TokenLogos::Multiplicar => Token::Multiplicar,
            TokenLogos::Dividir => Token::Dividir,
            TokenLogos::Modulo => Token::Modulo,
            TokenLogos::Potencia => Token::Potencia,
            TokenLogos::Igual => Token::Igual,
            TokenLogos::Diferente => Token::Diferente,
            TokenLogos::Mayor => Token::Mayor,
            TokenLogos::Menor => Token::Menor,
            TokenLogos::MayorIgual => Token::MayorIgual,
            TokenLogos::MenorIgual => Token::MenorIgual,
            TokenLogos::Asignar => Token::Asignar,
            TokenLogos::MasAsignar => Token::MasAsignar,
            TokenLogos::MenosAsignar => Token::MenosAsignar,
            TokenLogos::MultiplicarAsignar => Token::MultiplicarAsignar,
            TokenLogos::DividirAsignar => Token::DividirAsignar,
            TokenLogos::ModuloAsignar => Token::ModuloAsignar,
            TokenLogos::Incrementar => Token::Incrementar,
            TokenLogos::Decrementar => Token::Decrementar,
            TokenLogos::Negacion => Token::Negacion,
            TokenLogos::Ternario => Token::Ternario,
            TokenLogos::DosPuntos => Token::DosPuntos,
            TokenLogos::ParentesisIzq => Token::ParentesisIzq,
            TokenLogos::ParentesisDer => Token::ParentesisDer,
            TokenLogos::LlaveIzq => Token::LlaveIzq,
            TokenLogos::LlaveDer => Token::LlaveDer,
            TokenLogos::CorcheteIzq => Token::CorcheteIzq,
            TokenLogos::CorcheteDer => Token::CorcheteDer,
            TokenLogos::Punto => Token::Punto,
            TokenLogos::Coma => Token::Coma,
            TokenLogos::PuntoYComa => Token::PuntoYComa,
            TokenLogos::EnteroLiteralRaw => {
                let texto = &self.fuente[inicio..fin];
                Token::LiteralEntero(texto.parse().unwrap_or(0))
            }
            TokenLogos::NumeroLiteralRaw => {
                let texto = &self.fuente[inicio..fin];
                Token::LiteralNumero(texto.to_string())
            }
            TokenLogos::CadenaRaw => {
                let texto = &self.fuente[inicio..fin];
                // Remover comillas y procesar escapes
                let contenido = procesar_cadena(&texto[1..texto.len()-1])
                    .unwrap_or_else(|| texto[1..texto.len()-1].to_string());
                Token::LiteralTexto(contenido)
            }
            TokenLogos::InterpolacionTextoRaw => {
                let texto = &self.fuente[inicio..fin];
                // Remover t" y " y procesar escapes
                let contenido = procesar_cadena(&texto[2..texto.len()-1])
                    .unwrap_or_else(|| texto[2..texto.len()-1].to_string());
                Token::InterpolacionTexto(contenido)
            }
            TokenLogos::IdentificadorRaw => {
                let texto = &self.fuente[inicio..fin];
                Token::Identificador(texto.to_string())
            }
            // NuevaLinea ya no se usa porque la saltamos en el skip
            // TokenLogos::NuevaLinea => Token::NuevaLinea,
            TokenLogos::InicioComentarioBloque => {
                return Err(Error::analisis(
                    CodigoError::ComentarioSinCerrar,
                    "comentario de bloque sin cerrar",
                    None,
                    None,
                    None,
                ));
            }
            TokenLogos::Error => {
                return Err(Error::analisis(
                    CodigoError::CaracterInvalido,
                    "carácter inválido",
                    None,
                    None,
                    None,
                ));
            }
        })
    }
    
    fn calcular_linea(&self, offset: usize) -> usize {
        self.fuente[..offset].matches('\n').count() + 1
    }
    
    fn calcular_columna(&self, offset: usize, linea: usize) -> usize {
        if linea == 1 {
            offset + 1
        } else {
            let ultima_nueva_linea = self.fuente[..offset]
                .rfind('\n')
                .map(|pos| pos + 1)
                .unwrap_or(0);
            offset - ultima_nueva_linea + 1
        }
    }
    
    /// Obtiene todos los tokens del código fuente
    pub fn tokenizar(&mut self) -> Resultado<Vec<TokenConPosicion>> {
        let mut tokens = Vec::new();
        
        while let Some(token) = self.siguiente()? {
            tokens.push(token);
        }
        
        Ok(tokens)
    }
}

