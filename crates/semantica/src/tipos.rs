//! Sistema de tipos del análisis semántico.

use ast::Tipo;

/// Tipo inferido o declarado durante el análisis semántico.
#[derive(Debug, Clone, PartialEq)]
pub enum TipoSemantico {
    Entero,
    Numero,
    Texto,
    Log,
    Jsn,
    Vacio,
    Lista(Option<Box<TipoSemantico>>),
    /// Instancia de un objeto definido por el usuario.
    Objeto(String),
    /// Función declarada con su firma.
    Funcion {
        parametros: Vec<TipoSemantico>,
        retorno: Box<TipoSemantico>,
    },
    /// El literal `nulo`: asignable a cualquier tipo.
    Nulo,
    /// Tipo que no puede determinarse estáticamente (módulos nativos,
    /// miembros de jsn, símbolos importados). Compatible con todo; las
    /// verificaciones finas se hacen en runtime.
    Desconocido,
}

impl TipoSemantico {
    /// Convierte un tipo del AST en tipo semántico.
    pub fn desde_ast(tipo: &Tipo) -> Self {
        match tipo {
            Tipo::Entero => TipoSemantico::Entero,
            Tipo::Numero => TipoSemantico::Numero,
            Tipo::Texto => TipoSemantico::Texto,
            Tipo::Log => TipoSemantico::Log,
            Tipo::Jsn => TipoSemantico::Jsn,
            Tipo::Vacio => TipoSemantico::Vacio,
            Tipo::Lista(None) => TipoSemantico::Lista(None),
            Tipo::Lista(Some(interior)) => {
                TipoSemantico::Lista(Some(Box::new(TipoSemantico::desde_ast(interior))))
            }
            Tipo::Nombrado(nombre) => TipoSemantico::Objeto(nombre.clone()),
        }
    }

    /// Si un valor de este tipo puede asignarse a una variable del tipo destino.
    pub fn es_asignable_a(&self, destino: &TipoSemantico) -> bool {
        match (self, destino) {
            (TipoSemantico::Desconocido, _) | (_, TipoSemantico::Desconocido) => true,
            // `nulo` puede asignarse a cualquier tipo.
            (TipoSemantico::Nulo, _) => true,
            // Un entero se promociona a número sin pérdida.
            (TipoSemantico::Entero, TipoSemantico::Numero) => true,
            // Lista sin tipar acepta cualquier lista y viceversa.
            (TipoSemantico::Lista(_), TipoSemantico::Lista(None)) => true,
            (TipoSemantico::Lista(None), TipoSemantico::Lista(_)) => true,
            (TipoSemantico::Lista(Some(a)), TipoSemantico::Lista(Some(b))) => a.es_asignable_a(b),
            (a, b) => a == b,
        }
    }

    pub fn es_numerico(&self) -> bool {
        matches!(
            self,
            TipoSemantico::Entero | TipoSemantico::Numero | TipoSemantico::Desconocido
        )
    }

    /// Nombre legible para diagnósticos.
    pub fn nombre(&self) -> String {
        match self {
            TipoSemantico::Entero => "entero".to_string(),
            TipoSemantico::Numero => "número".to_string(),
            TipoSemantico::Texto => "texto".to_string(),
            TipoSemantico::Log => "log".to_string(),
            TipoSemantico::Jsn => "jsn".to_string(),
            TipoSemantico::Vacio => "vacio".to_string(),
            TipoSemantico::Lista(None) => "lista".to_string(),
            TipoSemantico::Lista(Some(interior)) => format!("lista<{}>", interior.nombre()),
            TipoSemantico::Objeto(nombre) => nombre.clone(),
            TipoSemantico::Funcion { retorno, .. } => format!("función -> {}", retorno.nombre()),
            TipoSemantico::Nulo => "nulo".to_string(),
            TipoSemantico::Desconocido => "desconocido".to_string(),
        }
    }
}
