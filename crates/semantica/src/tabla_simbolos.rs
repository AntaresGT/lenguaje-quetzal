//! Tabla de símbolos con ámbitos anidados.

use indexmap::IndexMap;
use nucleo::Ubicacion;

use crate::tipos::TipoSemantico;

/// Un símbolo declarado: variable, función, objeto o import.
#[derive(Debug, Clone)]
pub struct Simbolo {
    pub nombre: String,
    pub tipo: TipoSemantico,
    pub mutable: bool,
    pub ubicacion: Ubicacion,
}

/// Tabla de símbolos con pila de ámbitos.
#[derive(Debug, Default)]
pub struct TablaSimbolos {
    ambitos: Vec<IndexMap<String, Simbolo>>,
}

impl TablaSimbolos {
    pub fn nueva() -> Self {
        Self {
            ambitos: vec![IndexMap::new()],
        }
    }

    pub fn abrir_ambito(&mut self) {
        self.ambitos.push(IndexMap::new());
    }

    pub fn cerrar_ambito(&mut self) {
        if self.ambitos.len() > 1 {
            self.ambitos.pop();
        }
    }

    /// Declara un símbolo en el ámbito actual. Devuelve el símbolo previo si
    /// ya existía uno con el mismo nombre en este ámbito (duplicado).
    pub fn declarar(&mut self, simbolo: Simbolo) -> Option<Simbolo> {
        let ambito = self
            .ambitos
            .last_mut()
            .expect("siempre existe al menos un ámbito");
        ambito.insert(simbolo.nombre.clone(), simbolo)
    }

    /// Busca un símbolo desde el ámbito más interno hacia afuera.
    pub fn buscar(&self, nombre: &str) -> Option<&Simbolo> {
        self.ambitos
            .iter()
            .rev()
            .find_map(|ambito| ambito.get(nombre))
    }

    /// Si el símbolo existe en cualquier ámbito.
    pub fn existe(&self, nombre: &str) -> bool {
        self.buscar(nombre).is_some()
    }
}
