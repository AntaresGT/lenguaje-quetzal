use crate::nucleo::lexico::token::Posicion;

/// Nodo del Árbol de Sintaxis Abstracta (AST)
#[derive(Debug, Clone)]
pub enum NodoAst {
    // Declaraciones
    DeclaracionVariable {
        tipo: TipoAst,
        mutable: bool,
        nombre: String,
        valor: Box<NodoAst>,
        posicion: Posicion,
    },
    
    DeclaracionFuncion {
        asincrono: bool,
        tipo_retorno: TipoAst,
        nombre: String,
        parametros: Vec<ParametroAst>,
        cuerpo: Box<NodoAst>,
        posicion: Posicion,
    },
    
    DeclaracionObjeto {
        nombre: String,
        padres: Vec<String>,
        miembros: Vec<MiembroObjetoAst>,
        posicion: Posicion,
    },
    
    // Expresiones
    ExpresionLiteral {
        valor: LiteralAst,
        posicion: Posicion,
    },
    
    ExpresionIdentificador {
        nombre: String,
        posicion: Posicion,
    },
    
    ExpresionBinaria {
        operador: OperadorBinario,
        izquierda: Box<NodoAst>,
        derecha: Box<NodoAst>,
        posicion: Posicion,
    },
    
    ExpresionUnaria {
        operador: OperadorUnario,
        expresion: Box<NodoAst>,
        posicion: Posicion,
    },
    
    ExpresionLlamada {
        funcion: Box<NodoAst>,
        argumentos: Vec<NodoAst>,
        posicion: Posicion,
    },
    
    ExpresionAcceso {
        objeto: Box<NodoAst>,
        miembro: String,
        posicion: Posicion,
    },
    
    ExpresionIndice {
        objeto: Box<NodoAst>,
        indice: Box<NodoAst>,
        posicion: Posicion,
    },
    
    ExpresionTernario {
        condicion: Box<NodoAst>,
        verdadero: Box<NodoAst>,
        falso: Box<NodoAst>,
        posicion: Posicion,
    },
    
    ExpresionLista {
        elementos: Vec<NodoAst>,
        posicion: Posicion,
    },
    
    ExpresionJson {
        propiedades: Vec<PropiedadJsonAst>,
        posicion: Posicion,
    },
    
    ExpresionNuevo {
        tipo: String,
        argumentos: Vec<NodoAst>,
        posicion: Posicion,
    },
    
    ExpresionAsignar {
        objetivo: Box<NodoAst>,
        valor: Box<NodoAst>,
        posicion: Posicion,
    },
    
    ExpresionEsperar {
        expresion: Box<NodoAst>,
        posicion: Posicion,
    },
    
    // Estructuras de control
    Bloque {
        declaraciones: Vec<NodoAst>,
        posicion: Posicion,
    },
    
    Si {
        condicion: Box<NodoAst>,
        entonces: Box<NodoAst>,
        sino: Option<Box<NodoAst>>,
        posicion: Posicion,
    },
    
    Mientras {
        condicion: Box<NodoAst>,
        cuerpo: Box<NodoAst>,
        posicion: Posicion,
    },
    
    Para {
        inicializacion: Option<Box<NodoAst>>,
        condicion: Option<Box<NodoAst>>,
        incremento: Option<Box<NodoAst>>,
        cuerpo: Box<NodoAst>,
        posicion: Posicion,
    },
    
    ParaEn {
        variable: String,
        tipo: TipoAst,
        mutable: bool,
        coleccion: Box<NodoAst>,
        cuerpo: Box<NodoAst>,
        posicion: Posicion,
    },
    
    HacerMientras {
        cuerpo: Box<NodoAst>,
        condicion: Box<NodoAst>,
        posicion: Posicion,
    },
    
    Retornar {
        valor: Option<Box<NodoAst>>,
        posicion: Posicion,
    },
    
    Romper {
        posicion: Posicion,
    },
    
    Continuar {
        posicion: Posicion,
    },
    
    Intentar {
        bloque: Box<NodoAst>,
        capturar: Option<CapturarAst>,
        finalmente: Option<Box<NodoAst>>,
        posicion: Posicion,
    },
    
    Lanzar {
        expresion: Box<NodoAst>,
        posicion: Posicion,
    },
    
    // Importaciones
    Importacion {
        elementos: Vec<ElementoImportacion>,
        ruta: String,
        posicion: Posicion,
    },
}

/// Tipo en el AST
#[derive(Debug, Clone, PartialEq)]
pub enum TipoAst {
    Vacio,
    Entero,
    Numero,
    Texto,
    Logico,
    Lista(Box<TipoAst>),
    Json,
    Objeto(String), // Nombre del tipo de objeto
}

/// Literal en el AST
#[derive(Debug, Clone)]
pub enum LiteralAst {
    Entero(i64),
    Numero(String),
    Texto(String),
    InterpolacionTexto(String),
    Logico(bool),
    Nulo,
}

/// Operador binario
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperadorBinario {
    // Aritméticos
    Suma,
    Resta,
    Multiplicacion,
    Division,
    Modulo,
    Potencia,
    
    // Comparación
    Igual,
    Diferente,
    Mayor,
    Menor,
    MayorIgual,
    MenorIgual,
    
    // Lógicos
    Y,
    O,
    
    // Asignación
    Asignar,
    SumaAsignar,
    RestaAsignar,
    MultiplicacionAsignar,
    DivisionAsignar,
    ModuloAsignar,
}

/// Operador unario
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperadorUnario {
    Negacion,
    Negativo,
    Positivo,
    Incrementar,
    Decrementar,
}

/// Parámetro de función
#[derive(Debug, Clone)]
pub struct ParametroAst {
    pub tipo: TipoAst,
    pub mutable: bool,
    pub nombre: String,
    pub posicion: Posicion,
}

/// Miembro de objeto
#[derive(Debug, Clone)]
pub struct MiembroObjetoAst {
    pub modificador_acceso: ModificadorAcceso,
    pub libre: bool,
    pub declaracion: Box<NodoAst>,
    pub posicion: Posicion,
}

/// Modificador de acceso
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModificadorAcceso {
    Publico,
    Privado,
}

/// Propiedad JSON
#[derive(Debug, Clone)]
pub struct PropiedadJsonAst {
    pub clave: String,
    pub valor: Box<NodoAst>,
    pub posicion: Posicion,
}

/// Bloque capturar
#[derive(Debug, Clone)]
pub struct CapturarAst {
    pub variable: String,
    pub bloque: Box<NodoAst>,
    pub posicion: Posicion,
}

/// Elemento importado de un módulo
#[derive(Debug, Clone)]
pub struct ElementoImportacion {
    pub nombre: String,      // Nombre original en el módulo
    pub alias: Option<String>, // Nombre alternativo (alias), si existe
}

impl NodoAst {
    pub fn posicion(&self) -> Posicion {
        match self {
            NodoAst::DeclaracionVariable { posicion, .. }
            | NodoAst::DeclaracionFuncion { posicion, .. }
            | NodoAst::DeclaracionObjeto { posicion, .. }
            | NodoAst::ExpresionLiteral { posicion, .. }
            | NodoAst::ExpresionIdentificador { posicion, .. }
            | NodoAst::ExpresionBinaria { posicion, .. }
            | NodoAst::ExpresionUnaria { posicion, .. }
            | NodoAst::ExpresionLlamada { posicion, .. }
            | NodoAst::ExpresionAcceso { posicion, .. }
            | NodoAst::ExpresionIndice { posicion, .. }
            | NodoAst::ExpresionTernario { posicion, .. }
            | NodoAst::ExpresionLista { posicion, .. }
            | NodoAst::ExpresionJson { posicion, .. }
            | NodoAst::ExpresionNuevo { posicion, .. }
            | NodoAst::ExpresionAsignar { posicion, .. }
            | NodoAst::ExpresionEsperar { posicion, .. }
            | NodoAst::Bloque { posicion, .. }
            | NodoAst::Si { posicion, .. }
            | NodoAst::Mientras { posicion, .. }
            | NodoAst::Para { posicion, .. }
            | NodoAst::ParaEn { posicion, .. }
            | NodoAst::HacerMientras { posicion, .. }
            | NodoAst::Retornar { posicion, .. }
            | NodoAst::Romper { posicion, .. }
            | NodoAst::Continuar { posicion, .. }
            | NodoAst::Intentar { posicion, .. }
            | NodoAst::Lanzar { posicion, .. }
            | NodoAst::Importacion { posicion, .. } => *posicion,
        }
    }
}
