
/// Representa un token del lenguaje Quetzal
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Palabras reservadas - Tipos
    Vacio,
    VacioAcento, // vacío
    Entero,
    Numero,
    NumeroAcento, // número
    Texto,
    Log,
    LogAcento, // lóg
    Lista,
    Jsn,
    
    // Valores literales
    Verdadero,
    Falso,
    Nulo,
    
    // Modificadores
    Var,
    Publico,
    PublicoAcento, // público
    Privado,
    Libre,
    
    // Funciones y objetos
    Retornar,
    Objeto,
    Nuevo,
    Ambiente,
    Padre,
    
    // Programación asíncrona
    Asincrono,
    Esperar,
    
    // Control de flujo
    Si,
    Sino,
    Mientras,
    Para,
    Hacer,
    Romper,
    Continuar,
    Cada,
    
    // Manejo de excepciones
    Intentar,
    Capturar,
    Finalmente,
    Lanzar,
    Excepcion,
    ExcepcionSinAcento, // excepcion
    
    // Módulos
    Importar,
    Exportar,
    Desde,
    Como,
    
    // Operadores lógicos
    Y,
    O,
    OConAcento, // ó
    
    // Operadores
    Mas,              // +
    Menos,            // -
    Multiplicar,      // *
    Dividir,         // /
    Modulo,          // %
    Potencia,        // **
    Igual,           // ==
    Diferente,       // !=
    Mayor,           // >
    Menor,           // <
    MayorIgual,      // >=
    MenorIgual,      // <=
    Asignar,         // =
    MasAsignar,      // +=
    MenosAsignar,    // -=
    MultiplicarAsignar, // *=
    DividirAsignar,  // /=
    ModuloAsignar,   // %=
    Incrementar,     // ++
    Decrementar,     // --
    Negacion,        // !
    Ternario,        // ?
    DosPuntos,       // :
    
    // Delimitadores
    ParentesisIzq,   // (
    ParentesisDer,   // )
    LlaveIzq,        // {
    LlaveDer,        // }
    CorcheteIzq,     // [
    CorcheteDer,     // ]
    Punto,           // .
    Coma,            // ,
    PuntoYComa,      // ;
    
    // Literales
    LiteralEntero(i64),
    LiteralNumero(String), // Usamos String para mantener precisión
    LiteralTexto(String),
    LiteralLogico(bool),
    
    // Interpolación de texto
    InterpolacionTexto(String), // t"..."
    
    // Identificadores
    Identificador(String),
    
    // Especiales
    FinDeArchivo,
    NuevaLinea,
    ComentarioLinea(String),
    ComentarioBloque(String),
}

impl Token {
    /// Verifica si el token es una palabra reservada
    pub fn es_palabra_reservada(&self) -> bool {
        matches!(
            self,
            Token::Vacio
                | Token::VacioAcento
                | Token::Entero
                | Token::Numero
                | Token::NumeroAcento
                | Token::Texto
                | Token::Log
                | Token::LogAcento
                | Token::Lista
                | Token::Jsn
                | Token::Verdadero
                | Token::Falso
                | Token::Nulo
                | Token::Var
                | Token::Publico
                | Token::PublicoAcento
                | Token::Privado
                | Token::Libre
                | Token::Retornar
                | Token::Objeto
                | Token::Nuevo
                | Token::Ambiente
                | Token::Padre
                | Token::Asincrono
                | Token::Esperar
                | Token::Si
                | Token::Sino
                | Token::Mientras
                | Token::Para
                | Token::Hacer
                | Token::Romper
                | Token::Continuar
                | Token::Cada
                | Token::Intentar
                | Token::Capturar
                | Token::Finalmente
                | Token::Lanzar
                | Token::Excepcion
                | Token::ExcepcionSinAcento
                | Token::Importar
                | Token::Exportar
                | Token::Desde
                | Token::Como
                | Token::Y
                | Token::O
                | Token::OConAcento
        )
    }
    
    /// Obtiene el nombre del token para mensajes de error
    pub fn nombre(&self) -> &'static str {
        match self {
            Token::Vacio | Token::VacioAcento => "vacio",
            Token::Entero => "entero",
            Token::Numero | Token::NumeroAcento => "número",
            Token::Texto => "texto",
            Token::Log | Token::LogAcento => "log",
            Token::Lista => "lista",
            Token::Jsn => "jsn",
            Token::Verdadero => "verdadero",
            Token::Falso => "falso",
            Token::Nulo => "nulo",
            Token::Var => "var",
            Token::Publico | Token::PublicoAcento => "publico",
            Token::Privado => "privado",
            Token::Libre => "libre",
            Token::Retornar => "retornar",
            Token::Objeto => "objeto",
            Token::Nuevo => "nuevo",
            Token::Ambiente => "ambiente",
            Token::Padre => "padre",
            Token::Asincrono => "asincrono",
            Token::Esperar => "esperar",
            Token::Si => "si",
            Token::Sino => "sino",
            Token::Mientras => "mientras",
            Token::Para => "para",
            Token::Hacer => "hacer",
            Token::Romper => "romper",
            Token::Continuar => "continuar",
            Token::Cada => "cada",
            Token::Intentar => "intentar",
            Token::Capturar => "capturar",
            Token::Finalmente => "finalmente",
            Token::Lanzar => "lanzar",
            Token::Excepcion | Token::ExcepcionSinAcento => "excepcion",
            Token::Importar => "importar",
            Token::Exportar => "exportar",
            Token::Desde => "desde",
            Token::Como => "como",
            Token::Y => "y",
            Token::O | Token::OConAcento => "o",
            Token::Mas => "+",
            Token::Menos => "-",
            Token::Multiplicar => "*",
            Token::Dividir => "/",
            Token::Modulo => "%",
            Token::Potencia => "**",
            Token::Igual => "==",
            Token::Diferente => "!=",
            Token::Mayor => ">",
            Token::Menor => "<",
            Token::MayorIgual => ">=",
            Token::MenorIgual => "<=",
            Token::Asignar => "=",
            Token::MasAsignar => "+=",
            Token::MenosAsignar => "-=",
            Token::MultiplicarAsignar => "*=",
            Token::DividirAsignar => "/=",
            Token::ModuloAsignar => "%=",
            Token::Incrementar => "++",
            Token::Decrementar => "--",
            Token::Negacion => "!",
            Token::Ternario => "?",
            Token::DosPuntos => ":",
            Token::ParentesisIzq => "(",
            Token::ParentesisDer => ")",
            Token::LlaveIzq => "{",
            Token::LlaveDer => "}",
            Token::CorcheteIzq => "[",
            Token::CorcheteDer => "]",
            Token::Punto => ".",
            Token::Coma => ",",
            Token::PuntoYComa => ";",
            Token::LiteralEntero(_) => "número entero",
            Token::LiteralNumero(_) => "número decimal",
            Token::LiteralTexto(_) => "texto",
            Token::LiteralLogico(_) => "booleano",
            Token::InterpolacionTexto(_) => "interpolación de texto",
            Token::Identificador(_) => "identificador",
            Token::FinDeArchivo => "fin de archivo",
            Token::NuevaLinea => "nueva línea",
            Token::ComentarioLinea(_) => "comentario",
            Token::ComentarioBloque(_) => "comentario",
        }
    }
}

/// Información de posición de un token
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Posicion {
    pub linea: usize,
    pub columna: usize,
    pub offset: usize,
}

impl Posicion {
    pub fn nueva(linea: usize, columna: usize, offset: usize) -> Self {
        Self {
            linea,
            columna,
            offset,
        }
    }
    
    pub fn inicial() -> Self {
        Self {
            linea: 1,
            columna: 1,
            offset: 0,
        }
    }
}

/// Token con información de posición
#[derive(Debug, Clone)]
pub struct TokenConPosicion {
    pub token: Token,
    pub posicion: Posicion,
}

impl TokenConPosicion {
    pub fn nueva(token: Token, posicion: Posicion) -> Self {
        Self { token, posicion }
    }
}
