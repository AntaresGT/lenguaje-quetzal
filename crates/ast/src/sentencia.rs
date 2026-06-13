//! Sentencias del Lenguaje Quetzal.

use nucleo::Ubicacion;

use crate::expresion::Expresion;
use crate::tipos::Tipo;

/// Una sentencia con su ubicación en el archivo fuente.
#[derive(Debug, Clone, PartialEq)]
pub struct Sentencia {
    pub nodo: NodoSentencia,
    pub ubicacion: Ubicacion,
}

impl Sentencia {
    pub fn nueva(nodo: NodoSentencia, ubicacion: Ubicacion) -> Self {
        Self { nodo, ubicacion }
    }
}

/// Bloque de sentencias entre llaves.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Bloque {
    pub sentencias: Vec<Sentencia>,
}

/// Todas las formas de sentencia del lenguaje.
#[derive(Debug, Clone, PartialEq)]
pub enum NodoSentencia {
    /// `entero var contador = 0` / `Usuario usuario = nuevo Usuario(...)`
    DeclaracionVariable {
        tipo: Tipo,
        mutable: bool,
        nombre: String,
        valor: Option<Expresion>,
    },
    /// `x = 1`, `x += 2`, `persona.nombre = "Ana"`, `lista[0] = 5`
    Asignacion {
        objetivo: Expresion,
        operador: OperadorAsignacion,
        valor: Expresion,
    },
    /// `contador++` / `contador--`
    IncrementoDecremento {
        objetivo: Expresion,
        incremento: bool,
    },
    /// Una expresión usada como sentencia, por ejemplo una llamada.
    Expresion(Expresion),

    /// `si (...) { } sino si (...) { } sino { }`
    Si {
        condicion: Expresion,
        entonces: Bloque,
        /// `sino si` se representa como un `Si` anidado dentro del bloque.
        sino: Option<Bloque>,
    },
    /// `mientras (...) { }`
    Mientras {
        condicion: Expresion,
        cuerpo: Bloque,
    },
    /// `hacer { } mientras (...)`
    HacerMientras {
        cuerpo: Bloque,
        condicion: Expresion,
    },
    /// `para (entero var i = 0; i < 5; i++) { }`
    ParaClasico {
        inicializacion: Box<Sentencia>,
        condicion: Expresion,
        paso: Box<Sentencia>,
        cuerpo: Bloque,
    },
    /// `para (entero var valor en lista) { }` / `... cada lista`
    ParaEn {
        tipo_elemento: Tipo,
        mutable: bool,
        nombre: String,
        iterable: Expresion,
        cuerpo: Bloque,
    },
    Romper,
    Continuar,
    Retornar(Option<Expresion>),

    /// `intentar { } capturar (excepcion e) { } finalmente { }`
    Intentar {
        bloque: Bloque,
        captura: Option<Captura>,
        finalmente: Option<Bloque>,
    },
    /// `lanzar "mensaje"`
    Lanzar(Expresion),
}

/// Cláusula `capturar (excepcion e) { ... }`.
#[derive(Debug, Clone, PartialEq)]
pub struct Captura {
    pub nombre: String,
    pub bloque: Bloque,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperadorAsignacion {
    /// `=`
    Asignar,
    /// `+=`
    Sumar,
    /// `-=`
    Restar,
    /// `*=`
    Multiplicar,
    /// `/=`
    Dividir,
    /// `%=`
    Modulo,
}
