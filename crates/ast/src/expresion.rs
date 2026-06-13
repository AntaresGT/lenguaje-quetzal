//! Expresiones del Lenguaje Quetzal.

use nucleo::Ubicacion;

/// Una expresión con su ubicación en el archivo fuente.
#[derive(Debug, Clone, PartialEq)]
pub struct Expresion {
    pub nodo: NodoExpresion,
    pub ubicacion: Ubicacion,
}

impl Expresion {
    pub fn nueva(nodo: NodoExpresion, ubicacion: Ubicacion) -> Self {
        Self { nodo, ubicacion }
    }
}

/// Todas las formas de expresión del lenguaje.
#[derive(Debug, Clone, PartialEq)]
pub enum NodoExpresion {
    // ----- Literales -----
    LiteralEntero(i64),
    /// Literal decimal; conserva el texto exacto para convertirlo a decimal
    /// sin pasar por f64.
    LiteralNumero(String),
    LiteralTexto(String),
    LiteralLog(bool),
    Nulo,
    /// `t"Hola {nombre}"` separado en segmentos.
    TextoInterpolado(Vec<SegmentoInterpolado>),
    /// `[1, 2, 3]`
    ListaLiteral(Vec<Expresion>),
    /// `{ clave: valor, "otra clave": 1 }`
    JsnLiteral(Vec<(ClaveJsn, Expresion)>),

    // ----- Referencias -----
    Identificador(String),
    /// `esto`: la instancia actual dentro de un objeto.
    Esto,
    /// `padre`: el objeto padre dentro de un objeto con herencia.
    Padre,

    // ----- Operaciones -----
    Binaria {
        operador: OperadorBinario,
        izquierda: Box<Expresion>,
        derecha: Box<Expresion>,
    },
    Unaria {
        operador: OperadorUnario,
        operando: Box<Expresion>,
    },
    /// `condicion ? si_verdadero : si_falso`
    Ternaria {
        condicion: Box<Expresion>,
        si_verdadero: Box<Expresion>,
        si_falso: Box<Expresion>,
    },

    // ----- Acceso y llamadas -----
    /// `objetivo(argumentos)`
    Llamada {
        objetivo: Box<Expresion>,
        argumentos: Vec<Expresion>,
    },
    /// `objeto.miembro`
    AccesoMiembro {
        objeto: Box<Expresion>,
        miembro: String,
    },
    /// `objeto[indice]` (acepta índices negativos en listas y textos)
    Indexacion {
        objeto: Box<Expresion>,
        indice: Box<Expresion>,
    },
    /// `nuevo Usuario("Ana", 25)`
    Nuevo {
        clase: String,
        argumentos: Vec<Expresion>,
    },
    /// `esperar expresion`
    Esperar(Box<Expresion>),
}

/// Clave de una entrada en un literal `jsn`.
#[derive(Debug, Clone, PartialEq)]
pub enum ClaveJsn {
    /// `nombre: ...`
    Identificador(String),
    /// `"clave con espacio": ...`
    Texto(String),
}

impl ClaveJsn {
    pub fn texto(&self) -> &str {
        match self {
            ClaveJsn::Identificador(texto) | ClaveJsn::Texto(texto) => texto,
        }
    }
}

/// Segmento de un texto interpolado `t"..."`.
#[derive(Debug, Clone, PartialEq)]
pub enum SegmentoInterpolado {
    Texto(String),
    Expresion(Expresion),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperadorBinario {
    Sumar,
    Restar,
    Multiplicar,
    Dividir,
    Modulo,
    Igual,
    Diferente,
    Mayor,
    Menor,
    MayorOIgual,
    MenorOIgual,
    Y,
    O,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperadorUnario {
    /// `-x`
    Negacion,
    /// `!x`
    NoLogico,
}
