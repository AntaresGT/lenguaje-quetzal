//! Lexer del Lenguaje Quetzal basado en `logos`.

use logos::Logos;
use nucleo::{CategoriaError, ErrorQuetzal, Fuente, Ubicacion};

use crate::palabras_reservadas;
use crate::token::{TipoToken, Token};

/// Tokens crudos que reconoce logos; luego se convierten a [`TipoToken`]
/// (los identificadores se clasifican como palabras reservadas aparte).
#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
#[logos(skip r"//[^\n]*")]
#[logos(skip r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/")]
enum TokenCrudo {
    // La 't' pegada a la comilla distingue el texto interpolado; tiene
    // prioridad sobre el identificador `t` por ser una coincidencia más larga.
    #[regex(r#"t"([^"\\]|\\[\s\S])*""#)]
    TextoInterpolado,

    #[regex(r#""([^"\\]|\\[\s\S])*""#)]
    Texto,

    #[regex(r"[0-9]+\.[0-9]+")]
    Numero,

    #[regex(r"[0-9]+")]
    Entero,

    #[regex(r"[\p{L}_][\p{L}\p{N}_]*")]
    Identificador,

    #[token("+=")]
    MasIgual,
    #[token("-=")]
    MenosIgual,
    #[token("*=")]
    PorIgual,
    #[token("/=")]
    EntreIgual,
    #[token("%=")]
    ModuloIgual,
    #[token("++")]
    Incremento,
    #[token("--")]
    Decremento,
    #[token("==")]
    IgualIgual,
    #[token("!=")]
    Diferente,
    #[token(">=")]
    MayorOIgual,
    #[token("<=")]
    MenorOIgual,
    #[token("&&")]
    YLogico,
    #[token("||")]
    OLogico,
    #[token("+")]
    Mas,
    #[token("-")]
    Menos,
    #[token("*")]
    Por,
    #[token("/")]
    Entre,
    #[token("%")]
    Modulo,
    #[token("=")]
    Asignar,
    #[token(">")]
    Mayor,
    #[token("<")]
    Menor,
    #[token("!")]
    NoLogico,
    #[token("?")]
    Interrogacion,
    #[token("(")]
    ParentesisIzquierdo,
    #[token(")")]
    ParentesisDerecho,
    #[token("{")]
    LlaveIzquierda,
    #[token("}")]
    LlaveDerecha,
    #[token("[")]
    CorcheteIzquierdo,
    #[token("]")]
    CorcheteDerecho,
    #[token(",")]
    Coma,
    #[token(".")]
    Punto,
    #[token(":")]
    DosPuntos,
    #[token(";")]
    PuntoYComa,
}

/// Convierte el código fuente en una lista de tokens terminada en [`TipoToken::Fin`].
pub fn tokenizar(fuente: &Fuente) -> Result<Vec<Token>, ErrorQuetzal> {
    let mut lexer = TokenCrudo::lexer(&fuente.contenido);
    let mut tokens = Vec::new();

    while let Some(resultado) = lexer.next() {
        let rango = lexer.span();
        let ubicacion = Ubicacion::nueva(rango.start, rango.end);
        let texto = lexer.slice();

        let crudo = match resultado {
            Ok(crudo) => crudo,
            Err(()) => return Err(error_lexico(fuente, ubicacion, texto)),
        };

        let tipo = match crudo {
            TokenCrudo::Identificador => palabras_reservadas::clasificar(texto)
                .unwrap_or_else(|| TipoToken::Identificador(texto.to_string())),
            TokenCrudo::Entero => {
                let valor = texto.parse::<i64>().map_err(|_| {
                    ErrorQuetzal::nuevo(
                        diagnosticos_codigo_numero(),
                        CategoriaError::Lexico,
                        format!("el entero '{texto}' es demasiado grande"),
                    )
                    .con_ubicacion(ubicacion)
                    .con_archivo(&fuente.nombre)
                    .con_etiqueta("este número no cabe en un entero de 64 bits")
                })?;
                TipoToken::LiteralEntero(valor)
            }
            TokenCrudo::Numero => TipoToken::LiteralNumero(texto.to_string()),
            TokenCrudo::Texto => {
                let interior = &texto[1..texto.len() - 1];
                TipoToken::LiteralTexto(resolver_escapes(interior))
            }
            TokenCrudo::TextoInterpolado => {
                // Se guarda crudo (sin `t"` ni la comilla final); el parser
                // separa los segmentos `{expresion}` y resuelve escapes.
                let interior = &texto[2..texto.len() - 1];
                TipoToken::TextoInterpolado(interior.to_string())
            }
            TokenCrudo::MasIgual => TipoToken::MasIgual,
            TokenCrudo::MenosIgual => TipoToken::MenosIgual,
            TokenCrudo::PorIgual => TipoToken::PorIgual,
            TokenCrudo::EntreIgual => TipoToken::EntreIgual,
            TokenCrudo::ModuloIgual => TipoToken::ModuloIgual,
            TokenCrudo::Incremento => TipoToken::Incremento,
            TokenCrudo::Decremento => TipoToken::Decremento,
            TokenCrudo::IgualIgual => TipoToken::IgualIgual,
            TokenCrudo::Diferente => TipoToken::Diferente,
            TokenCrudo::MayorOIgual => TipoToken::MayorOIgual,
            TokenCrudo::MenorOIgual => TipoToken::MenorOIgual,
            TokenCrudo::YLogico => TipoToken::YLogico,
            TokenCrudo::OLogico => TipoToken::OLogico,
            TokenCrudo::Mas => TipoToken::Mas,
            TokenCrudo::Menos => TipoToken::Menos,
            TokenCrudo::Por => TipoToken::Por,
            TokenCrudo::Entre => TipoToken::Entre,
            TokenCrudo::Modulo => TipoToken::Modulo,
            TokenCrudo::Asignar => TipoToken::Asignar,
            TokenCrudo::Mayor => TipoToken::Mayor,
            TokenCrudo::Menor => TipoToken::Menor,
            TokenCrudo::NoLogico => TipoToken::NoLogico,
            TokenCrudo::Interrogacion => TipoToken::Interrogacion,
            TokenCrudo::ParentesisIzquierdo => TipoToken::ParentesisIzquierdo,
            TokenCrudo::ParentesisDerecho => TipoToken::ParentesisDerecho,
            TokenCrudo::LlaveIzquierda => TipoToken::LlaveIzquierda,
            TokenCrudo::LlaveDerecha => TipoToken::LlaveDerecha,
            TokenCrudo::CorcheteIzquierdo => TipoToken::CorcheteIzquierdo,
            TokenCrudo::CorcheteDerecho => TipoToken::CorcheteDerecho,
            TokenCrudo::Coma => TipoToken::Coma,
            TokenCrudo::Punto => TipoToken::Punto,
            TokenCrudo::DosPuntos => TipoToken::DosPuntos,
            TokenCrudo::PuntoYComa => TipoToken::PuntoYComa,
        };

        tokens.push(Token { tipo, ubicacion });
    }

    let fin = fuente.contenido.len();
    tokens.push(Token {
        tipo: TipoToken::Fin,
        ubicacion: Ubicacion::nueva(fin, fin),
    });

    Ok(tokens)
}

/// Resuelve las secuencias de escape estándar de un texto.
pub fn resolver_escapes(texto: &str) -> String {
    let mut resultado = String::with_capacity(texto.len());
    let mut caracteres = texto.chars();
    while let Some(caracter) = caracteres.next() {
        if caracter != '\\' {
            resultado.push(caracter);
            continue;
        }
        match caracteres.next() {
            Some('n') => resultado.push('\n'),
            Some('t') => resultado.push('\t'),
            Some('r') => resultado.push('\r'),
            Some('"') => resultado.push('"'),
            Some('\\') => resultado.push('\\'),
            Some('{') => resultado.push('{'),
            Some('}') => resultado.push('}'),
            Some(otro) => {
                // Escape desconocido: se conserva tal cual.
                resultado.push('\\');
                resultado.push(otro);
            }
            None => resultado.push('\\'),
        }
    }
    resultado
}

fn error_lexico(fuente: &Fuente, ubicacion: Ubicacion, texto: &str) -> ErrorQuetzal {
    let resto = &fuente.contenido[ubicacion.inicio..];
    let (codigo, mensaje, etiqueta) = if resto.starts_with("t\"") || resto.starts_with('"') {
        (
            "E0002",
            "texto sin cerrar".to_string(),
            "falta la comilla de cierre \"",
        )
    } else if resto.starts_with("/*") {
        (
            "E0004",
            "comentario de bloque sin cerrar".to_string(),
            "falta el cierre */",
        )
    } else {
        (
            "E0001",
            format!("carácter inesperado '{texto}'"),
            "este carácter no es válido en Quetzal",
        )
    };

    ErrorQuetzal::nuevo(codigo, CategoriaError::Lexico, mensaje)
        .con_ubicacion(ubicacion)
        .con_archivo(&fuente.nombre)
        .con_etiqueta(etiqueta)
}

fn diagnosticos_codigo_numero() -> &'static str {
    "E0003"
}
