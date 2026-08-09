//! Tokens del Lenguaje Quetzal.

use nucleo::Ubicacion;

/// Un token con su tipo y su ubicación en el archivo fuente.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub tipo: TipoToken,
    pub ubicacion: Ubicacion,
}

/// Todos los tipos de token del lenguaje.
#[derive(Debug, Clone, PartialEq)]
pub enum TipoToken {
    // ----- Tipos -----
    TipoEntero,
    TipoNumero,
    TipoTexto,
    TipoLog,
    TipoLista,
    TipoJsn,
    TipoVacio,

    // ----- Palabras reservadas -----
    Var,
    Si,
    Sino,
    Mientras,
    Para,
    Hacer,
    En,
    Cada,
    Romper,
    Continuar,
    Retornar,
    Intentar,
    Capturar,
    Finalmente,
    Lanzar,
    Excepcion,
    Objeto,
    Prototipo,
    Hereda,
    Implementa,
    Como,
    Nuevo,
    Esto,
    Padre,
    Publico,
    Privado,
    Libre,
    Opcional,
    Asincrono,
    Esperar,
    Importar,
    Exportar,
    Desde,

    // ----- Literales -----
    Verdadero,
    Falso,
    Nulo,
    /// Literal entero, por ejemplo `42`.
    LiteralEntero(i64),
    /// Literal decimal, se guarda el texto exacto para convertirlo a decimal
    /// sin pasar por f64 (evita errores de acarreo).
    LiteralNumero(String),
    /// Literal de texto ya sin comillas y con escapes resueltos.
    LiteralTexto(String),
    /// Texto interpolado `t"..."`; se guarda el contenido crudo (sin la `t`
    /// ni las comillas); el parser separa los segmentos `{expresion}`.
    TextoInterpolado(String),

    /// Identificador (soporta Unicode: `año`, `función`).
    Identificador(String),

    // ----- Operadores -----
    Mas,           // +
    Menos,         // -
    Por,           // *
    Entre,         // /
    Modulo,        // %
    MasIgual,      // +=
    MenosIgual,    // -=
    PorIgual,      // *=
    EntreIgual,    // /=
    ModuloIgual,   // %=
    Incremento,    // ++
    Decremento,    // --
    Asignar,       // =
    IgualIgual,    // ==
    Diferente,     // !=
    Mayor,         // >
    Menor,         // <
    MayorOIgual,   // >=
    MenorOIgual,   // <=
    YLogico,       // &&
    OLogico,       // ||
    NoLogico,      // !
    Interrogacion, // ?

    // ----- Delimitadores -----
    ParentesisIzquierdo, // (
    ParentesisDerecho,   // )
    LlaveIzquierda,      // {
    LlaveDerecha,        // }
    CorcheteIzquierdo,   // [
    CorcheteDerecho,     // ]
    Coma,                // ,
    Punto,               // .
    DosPuntos,           // :
    PuntoYComa,          // ;

    /// Fin del archivo.
    Fin,
}

impl TipoToken {
    /// Descripción legible para mensajes de error.
    pub fn descripcion(&self) -> String {
        match self {
            TipoToken::Identificador(nombre) => format!("identificador '{nombre}'"),
            TipoToken::LiteralEntero(valor) => format!("entero '{valor}'"),
            TipoToken::LiteralNumero(valor) => format!("número '{valor}'"),
            TipoToken::LiteralTexto(_) => "texto literal".to_string(),
            TipoToken::TextoInterpolado(_) => "texto interpolado".to_string(),
            TipoToken::Fin => "fin del archivo".to_string(),
            otro => format!("'{}'", otro.simbolo()),
        }
    }

    /// Símbolo o palabra tal como aparece en el código.
    fn simbolo(&self) -> &'static str {
        match self {
            TipoToken::TipoEntero => "entero",
            TipoToken::TipoNumero => "número",
            TipoToken::TipoTexto => "texto",
            TipoToken::TipoLog => "log",
            TipoToken::TipoLista => "lista",
            TipoToken::TipoJsn => "jsn",
            TipoToken::TipoVacio => "vacio",
            TipoToken::Var => "var",
            TipoToken::Si => "si",
            TipoToken::Sino => "sino",
            TipoToken::Mientras => "mientras",
            TipoToken::Para => "para",
            TipoToken::Hacer => "hacer",
            TipoToken::En => "en",
            TipoToken::Cada => "cada",
            TipoToken::Romper => "romper",
            TipoToken::Continuar => "continuar",
            TipoToken::Retornar => "retornar",
            TipoToken::Intentar => "intentar",
            TipoToken::Capturar => "capturar",
            TipoToken::Finalmente => "finalmente",
            TipoToken::Lanzar => "lanzar",
            TipoToken::Excepcion => "excepcion",
            TipoToken::Objeto => "objeto",
            TipoToken::Prototipo => "prototipo",
            TipoToken::Hereda => "hereda",
            TipoToken::Implementa => "implementa",
            TipoToken::Como => "como",
            TipoToken::Nuevo => "nuevo",
            TipoToken::Esto => "esto",
            TipoToken::Padre => "padre",
            TipoToken::Publico => "publico",
            TipoToken::Privado => "privado",
            TipoToken::Libre => "libre",
            TipoToken::Opcional => "opcional",
            TipoToken::Asincrono => "asincrono",
            TipoToken::Esperar => "esperar",
            TipoToken::Importar => "importar",
            TipoToken::Exportar => "exportar",
            TipoToken::Desde => "desde",
            TipoToken::Verdadero => "verdadero",
            TipoToken::Falso => "falso",
            TipoToken::Nulo => "nulo",
            TipoToken::Mas => "+",
            TipoToken::Menos => "-",
            TipoToken::Por => "*",
            TipoToken::Entre => "/",
            TipoToken::Modulo => "%",
            TipoToken::MasIgual => "+=",
            TipoToken::MenosIgual => "-=",
            TipoToken::PorIgual => "*=",
            TipoToken::EntreIgual => "/=",
            TipoToken::ModuloIgual => "%=",
            TipoToken::Incremento => "++",
            TipoToken::Decremento => "--",
            TipoToken::Asignar => "=",
            TipoToken::IgualIgual => "==",
            TipoToken::Diferente => "!=",
            TipoToken::Mayor => ">",
            TipoToken::Menor => "<",
            TipoToken::MayorOIgual => ">=",
            TipoToken::MenorOIgual => "<=",
            TipoToken::YLogico => "&&",
            TipoToken::OLogico => "||",
            TipoToken::NoLogico => "!",
            TipoToken::Interrogacion => "?",
            TipoToken::ParentesisIzquierdo => "(",
            TipoToken::ParentesisDerecho => ")",
            TipoToken::LlaveIzquierda => "{",
            TipoToken::LlaveDerecha => "}",
            TipoToken::CorcheteIzquierdo => "[",
            TipoToken::CorcheteDerecho => "]",
            TipoToken::Coma => ",",
            TipoToken::Punto => ".",
            TipoToken::DosPuntos => ":",
            TipoToken::PuntoYComa => ";",
            _ => "?",
        }
    }
}
