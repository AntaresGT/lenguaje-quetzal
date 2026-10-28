//! Código ejecutable: caches en línea, especialización y contadores de calor.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use bytecode::{CodigoOp, FuncionCompilada, Instruccion, Trozo};
use nucleo::Ubicacion;

use crate::nativos::IdNativo;

/// Umbral de llamadas/saltos atrás para compilar a cierres.
pub const UMBRAL_CALOR: u32 = 1_000;
/// Tope de funciones compiladas a cierres (evita crecer sin límite).
pub const TOPE_CIERRES: usize = 256;

/// Sitio de caché en línea para `CargarMiembro` / `LlamarMetodo`.
#[derive(Debug, Clone, Copy)]
pub enum CacheSitio {
    Vacia,
    Atributo { clase: usize, slot: u16 },
    Metodo { clase: usize },
    Nativa { tipo: u8, id: IdNativo },
    Polimorfica,
}

/// Bytecode de una función o trozo, con tablas laterales de runtime.
#[derive(Debug)]
pub struct CodigoRuntime {
    pub instrucciones: Rc<[Instruccion]>,
    pub ubicaciones: Rc<[Ubicacion]>,
    pub caches: Vec<Cell<CacheSitio>>,
    pub especializadas: Vec<Cell<Instruccion>>,
    pub calor: Cell<u32>,
    pub max_pila: u16,
    pub ranuras: u16,
    pub nombre: Option<Rc<str>>,
    pub funcion: Option<Rc<FuncionCompilada>>,
    pub cierre: RefCell<Option<Rc<CierreCompilado>>>,
}

/// Función compilada a un cierre (nivel 1). El `u32` extra es el `pc` de
/// arranque; si el cierre no puede seguir, devuelve ese pc para el intérprete.
pub struct CierreCompilado {
    pub correr: Box<dyn Fn(&mut crate::vm::Vm, u32) -> Result<ResultadoCierre, crate::errores::Fallo>>,
}

impl std::fmt::Debug for CierreCompilado {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CierreCompilado").finish_non_exhaustive()
    }
}

pub enum ResultadoCierre {
    Valor(crate::valores::Valor),
    Interpretar { pc: u32 },
}

impl CodigoRuntime {
    pub fn de_trozo(trozo: &Trozo, ranuras: u16, nombre: Option<Rc<str>>) -> Rc<Self> {
        let n = trozo.instrucciones.len();
        Rc::new(Self {
            instrucciones: trozo.instrucciones.clone().into(),
            ubicaciones: trozo.ubicaciones.clone().into(),
            caches: (0..n).map(|_| Cell::new(CacheSitio::Vacia)).collect(),
            especializadas: trozo.instrucciones.iter().copied().map(Cell::new).collect(),
            calor: Cell::new(0),
            max_pila: trozo.max_pila,
            ranuras,
            nombre,
            funcion: None,
            cierre: RefCell::new(None),
        })
    }

    pub fn de_funcion(funcion: &Rc<FuncionCompilada>) -> Rc<Self> {
        let mut codigo = Self::de_trozo(
            &funcion.trozo,
            funcion.ranuras_locales,
            Some(Rc::from(funcion.nombre.as_str())),
        );
        let inner = Rc::get_mut(&mut codigo).expect("código recién creado");
        inner.funcion = Some(Rc::clone(funcion));
        codigo
    }

    pub fn instruccion(&self, pc: usize, optimizar: bool) -> Instruccion {
        if optimizar {
            self.especializadas[pc].get()
        } else {
            self.instrucciones[pc]
        }
    }

    pub fn especializar(&self, pc: usize, instruccion: Instruccion) {
        self.especializadas[pc].set(instruccion);
    }

    pub fn desespecializar(&self, pc: usize) {
        self.especializadas[pc].set(self.instrucciones[pc]);
    }

    pub fn calentar(&self) -> bool {
        let actual = self.calor.get().saturating_add(1);
        self.calor.set(actual);
        actual == UMBRAL_CALOR
    }

    pub fn es_op_generica(op: CodigoOp) -> bool {
        matches!(
            op,
            CodigoOp::Sumar
                | CodigoOp::Restar
                | CodigoOp::Multiplicar
                | CodigoOp::Dividir
                | CodigoOp::Modulo
                | CodigoOp::Igual
                | CodigoOp::Diferente
                | CodigoOp::Mayor
                | CodigoOp::Menor
                | CodigoOp::MayorOIgual
                | CodigoOp::MenorOIgual
                | CodigoOp::CargarMiembro
                | CodigoOp::LlamarMetodo
        )
    }
}

pub fn clave_clase(clase: &Rc<crate::valores::ClaseObjeto>) -> usize {
    Rc::as_ptr(clase) as usize
}

pub fn tipo_cache(nombre: &str) -> u8 {
    match nombre {
        "texto" => 1,
        "lista" => 2,
        "jsn" => 3,
        "entero" => 4,
        "numero" => 5,
        "log" => 6,
        "instancia" => 7,
        "instancia nativa" => 8,
        "objeto" => 9,
        "modulo" => 10,
        _ => 0,
    }
}
