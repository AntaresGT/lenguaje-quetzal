use crate::nucleo::sintactico::ast::TipoAst;

/// Tipo en el sistema semántico
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Tipo {
    Vacio,
    Entero,
    Numero,
    Texto,
    Logico,
    Lista(Box<Tipo>),
    Json,
    Objeto(String),
    Funcion {
        parametros: Vec<Tipo>,
        retorno: Box<Tipo>,
    },
}

impl Tipo {
    pub fn puede_asignarse_a(&self, destino: &Tipo) -> bool {
        match (self, destino) {
            (a, b) if a == b => true,
            (Tipo::Entero, Tipo::Numero) => true,
            (Tipo::Numero, Tipo::Entero) => false,
            (Tipo::Vacio, _) | (_, Tipo::Vacio) => true,
            (Tipo::Lista(t1), Tipo::Lista(t2)) => {
                if **t1 == Tipo::Vacio || **t2 == Tipo::Vacio {
                    true
                } else {
                    t1.puede_asignarse_a(t2)
                }
            },
            _ => false,
        }
    }
    
    /// Convierte un TipoAst a Tipo
    pub fn desde_ast(ast: &TipoAst) -> Self {
        match ast {
            TipoAst::Vacio => Tipo::Vacio,
            TipoAst::Entero => Tipo::Entero,
            TipoAst::Numero => Tipo::Numero,
            TipoAst::Texto => Tipo::Texto,
            TipoAst::Logico => Tipo::Logico,
            TipoAst::Lista(tipo_interno) => Tipo::Lista(Box::new(Tipo::desde_ast(tipo_interno))),
            TipoAst::Json => Tipo::Json,
            TipoAst::Objeto(nombre) => Tipo::Objeto(nombre.clone()),
        }
    }
    
    /// Verifica si dos tipos son compatibles
    pub fn es_compatible_con(&self, otro: &Tipo) -> bool {
        match (self, otro) {
            (Tipo::Vacio, Tipo::Vacio) => true,
            (Tipo::Entero, Tipo::Entero) => true,
            (Tipo::Numero, Tipo::Numero) => true,
            (Tipo::Texto, Tipo::Texto) => true,
            (Tipo::Logico, Tipo::Logico) => true,
            (Tipo::Lista(t1), Tipo::Lista(t2)) => {
                if **t1 == Tipo::Vacio || **t2 == Tipo::Vacio {
                    true
                } else {
                    t1.es_compatible_con(t2)
                }
            },
            (Tipo::Json, Tipo::Json) => true,
            (Tipo::Objeto(n1), Tipo::Objeto(n2)) => n1 == n2,
            // Conversiones implícitas
            (Tipo::Entero, Tipo::Numero) => true,
            (Tipo::Numero, Tipo::Entero) => false, // No permitimos pérdida de precisión
            _ => false,
        }
    }
    
    /// Obtiene el nombre del tipo como string
    pub fn nombre(&self) -> String {
        match self {
            Tipo::Vacio => "vacio".to_string(),
            Tipo::Entero => "entero".to_string(),
            Tipo::Numero => "número".to_string(),
            Tipo::Texto => "texto".to_string(),
            Tipo::Logico => "logico".to_string(),
            Tipo::Lista(tipo_interno) => format!("lista<{}>", tipo_interno.nombre()),
            Tipo::Json => "jsn".to_string(),
            Tipo::Objeto(nombre) => nombre.clone(),
            Tipo::Funcion { parametros, retorno } => {
                let params: Vec<String> = parametros.iter().map(|t| t.nombre()).collect();
                format!("funcion({}) -> {}", params.join(", "), retorno.nombre())
            }
        }
    }
}
