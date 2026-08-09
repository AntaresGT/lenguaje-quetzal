//! Tipos del Lenguaje Quetzal tal como aparecen en el código fuente.

/// Tipo declarado de una variable, parámetro o retorno.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tipo {
    Entero,
    Numero,
    Texto,
    Log,
    Jsn,
    /// `vacio`: solo válido como tipo de retorno de funciones.
    Vacio,
    /// `lista` sin tipar o `lista<T>` tipada.
    Lista(Option<Box<Tipo>>),
    /// Tipo definido por el usuario: objetos y prototipos.
    Nombrado(String),
}

impl Tipo {
    /// Nombre legible para diagnósticos.
    pub fn nombre(&self) -> String {
        match self {
            Tipo::Entero => "entero".to_string(),
            Tipo::Numero => "número".to_string(),
            Tipo::Texto => "texto".to_string(),
            Tipo::Log => "log".to_string(),
            Tipo::Jsn => "jsn".to_string(),
            Tipo::Vacio => "vacio".to_string(),
            Tipo::Lista(None) => "lista".to_string(),
            Tipo::Lista(Some(interior)) => format!("lista<{}>", interior.nombre()),
            Tipo::Nombrado(nombre) => nombre.clone(),
        }
    }
}
