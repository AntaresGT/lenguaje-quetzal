use crate::nucleo::lexico::token::Posicion;
use crate::nucleo::sintactico::ast::{ElementoExportacion, ElementoImportacion, NodoAst};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdSimbolo(pub u32);

#[derive(Debug, Clone, Default)]
pub struct InternadorSimbolos {
    simbolos: Vec<String>,
    indices: HashMap<String, IdSimbolo>,
}

impl InternadorSimbolos {
    pub fn internar(&mut self, simbolo: impl Into<String>) -> IdSimbolo {
        let simbolo = simbolo.into();
        if let Some(id) = self.indices.get(&simbolo) {
            return *id;
        }

        let id = IdSimbolo(self.simbolos.len() as u32);
        self.indices.insert(simbolo.clone(), id);
        self.simbolos.push(simbolo);
        id
    }

    pub fn resolver(&self, id: IdSimbolo) -> Option<&str> {
        self.simbolos.get(id.0 as usize).map(|s| s.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct TablaExportaciones {
    pub explicita: bool,
    pub simbolos: Vec<IdSimbolo>,
}

impl TablaExportaciones {
    pub fn vacia() -> Self {
        Self {
            explicita: false,
            simbolos: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ItemHir {
    DeclaracionVariable { nombre: IdSimbolo, nodo: NodoAst },
    DeclaracionFuncion { nombre: IdSimbolo, nodo: NodoAst },
    DeclaracionObjeto { nombre: IdSimbolo, nodo: NodoAst },
    DeclaracionPrototipo { nombre: IdSimbolo, nodo: NodoAst },
    Importacion { ruta: IdSimbolo, elementos: Vec<IdSimbolo>, nodo: NodoAst },
    Exportacion { elementos: Vec<IdSimbolo>, posicion: Posicion },
    Expresion { nodo: NodoAst },
}

impl ItemHir {
    pub fn como_nodo_ast(&self) -> Option<&NodoAst> {
        match self {
            ItemHir::DeclaracionVariable { nodo, .. }
            | ItemHir::DeclaracionFuncion { nodo, .. }
            | ItemHir::DeclaracionObjeto { nodo, .. }
            | ItemHir::DeclaracionPrototipo { nodo, .. }
            | ItemHir::Importacion { nodo, .. }
            | ItemHir::Expresion { nodo, .. } => Some(nodo),
            ItemHir::Exportacion { .. } => None,
        }
    }

    pub fn nombre_simbolo(&self) -> Option<IdSimbolo> {
        match self {
            ItemHir::DeclaracionVariable { nombre, .. }
            | ItemHir::DeclaracionFuncion { nombre, .. }
            | ItemHir::DeclaracionObjeto { nombre, .. }
            | ItemHir::DeclaracionPrototipo { nombre, .. } => Some(*nombre),
            _ => None,
        }
    }

    pub fn posicion(&self) -> Option<Posicion> {
        match self {
            ItemHir::Exportacion { posicion, .. } => Some(*posicion),
            _ => self.como_nodo_ast().map(|n| n.posicion()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProgramaHir {
    pub items: Vec<ItemHir>,
    pub exportaciones: TablaExportaciones,
    pub internador: InternadorSimbolos,
    pub ruta_modulo: Option<PathBuf>,
}

impl ProgramaHir {
    pub fn desde_ast(nodos: &[NodoAst]) -> Self {
        let mut internador = InternadorSimbolos::default();
        let mut items = Vec::new();
        let mut exportaciones = TablaExportaciones::vacia();

        for nodo in nodos {
            match nodo {
                NodoAst::DeclaracionVariable { nombre, .. } => {
                    let id = internador.internar(nombre.clone());
                    items.push(ItemHir::DeclaracionVariable {
                        nombre: id,
                        nodo: nodo.clone(),
                    });
                    if !exportaciones.explicita {
                        exportaciones.simbolos.push(id);
                    }
                }
                NodoAst::DeclaracionFuncion { nombre, .. } => {
                    let id = internador.internar(nombre.clone());
                    items.push(ItemHir::DeclaracionFuncion {
                        nombre: id,
                        nodo: nodo.clone(),
                    });
                    if !exportaciones.explicita {
                        exportaciones.simbolos.push(id);
                    }
                }
                NodoAst::DeclaracionObjeto { nombre, .. } => {
                    let id = internador.internar(nombre.clone());
                    items.push(ItemHir::DeclaracionObjeto {
                        nombre: id,
                        nodo: nodo.clone(),
                    });
                    if !exportaciones.explicita {
                        exportaciones.simbolos.push(id);
                    }
                }
                NodoAst::DeclaracionPrototipo { nombre, .. } => {
                    let id = internador.internar(nombre.clone());
                    items.push(ItemHir::DeclaracionPrototipo {
                        nombre: id,
                        nodo: nodo.clone(),
                    });
                    if !exportaciones.explicita {
                        exportaciones.simbolos.push(id);
                    }
                }
                NodoAst::Importacion { elementos, ruta, .. } => {
                    let ruta_id = internador.internar(ruta.clone());
                    let elementos_ids = elementos
                        .iter()
                        .flat_map(|elemento| [elemento.nombre.clone(), elemento.alias.clone().unwrap_or_default()])
                        .filter(|nombre| !nombre.is_empty())
                        .map(|nombre| internador.internar(nombre))
                        .collect();

                    items.push(ItemHir::Importacion {
                        ruta: ruta_id,
                        elementos: elementos_ids,
                        nodo: nodo.clone(),
                    });
                }
                NodoAst::Exportacion { elementos, posicion } => {
                    if !exportaciones.explicita {
                        exportaciones.simbolos.clear();
                    }

                    exportaciones.explicita = true;
                    let mut elementos_ids = Vec::new();
                    for ElementoExportacion { nombre } in elementos {
                        let id = internador.internar(nombre.clone());
                        exportaciones.simbolos.push(id);
                        elementos_ids.push(id);
                    }

                    items.push(ItemHir::Exportacion {
                        elementos: elementos_ids,
                        posicion: *posicion,
                    });
                }
                _ => items.push(ItemHir::Expresion { nodo: nodo.clone() }),
            }
        }

        Self {
            items,
            exportaciones,
            internador,
            ruta_modulo: None,
        }
    }

    pub fn con_ruta(mut self, ruta: PathBuf) -> Self {
        self.ruta_modulo = Some(ruta);
        self
    }

    pub fn resolver_nombre(&self, id: IdSimbolo) -> Option<&str> {
        self.internador.resolver(id)
    }

    pub fn exportaciones_ast(&self) -> HashMap<String, NodoAst> {
        let mut declarados = HashMap::new();
        for item in &self.items {
            if let (Some(nombre_id), Some(nodo)) = (item.nombre_simbolo(), item.como_nodo_ast()) {
                if let Some(nombre) = self.resolver_nombre(nombre_id) {
                    declarados.insert(nombre.to_string(), nodo.clone());
                }
            }
        }

        let mut exportaciones = HashMap::new();
        for id in &self.exportaciones.simbolos {
            if let Some(nombre) = self.resolver_nombre(*id) {
                if let Some(nodo) = declarados.get(nombre) {
                    exportaciones.insert(nombre.to_string(), nodo.clone());
                }
            }
        }

        exportaciones
    }
}

pub fn nombre_importado(elemento: &ElementoImportacion) -> &str {
    elemento.alias.as_deref().unwrap_or(&elemento.nombre)
}
