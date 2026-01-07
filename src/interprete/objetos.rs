// Módulo de objetos - implementación básica
// Se expandirá para manejar constructores, métodos, modificadores de acceso, etc.

use crate::interprete::valores::Valor;
use std::collections::HashMap;

/// Crea una instancia de objeto
pub fn crear_instancia_objeto(
    tipo: String,
    propiedades: HashMap<String, Valor>,
) -> Valor {
    Valor::Objeto {
        tipo,
        propiedades,
    }
}
