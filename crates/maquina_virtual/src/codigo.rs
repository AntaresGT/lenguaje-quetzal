//! Código ejecutable: caches en línea, especialización y niveles de ejecución.

use std::cell::Cell;
#[cfg(feature = "jit")]
use std::cell::RefCell;
use std::rc::Rc;

use bytecode::{CodigoOp, FuncionCompilada, Instruccion, Trozo};
#[cfg(feature = "jit")]
use compilador_jit::FuncionNativa;
use nucleo::Ubicacion;

use crate::nativos::IdNativo;

#[cfg(feature = "jit")]
pub const UMBRAL_LLAMADAS_JIT: u32 = 4;
#[cfg(feature = "jit")]
pub const UMBRAL_ITERACIONES_JIT: u32 = 1_000;

#[cfg(feature = "jit")]
#[derive(Debug)]
pub enum EstadoJit {
    Frio,
    Compilado(Rc<FuncionNativa>),
    NoElegible,
}

/// Sitio de caché en línea para `CargarMiembro` / `LlamarMetodo`.
#[derive(Debug, Clone, Copy)]
pub enum CacheSitio {
    Vacia,
    Atributo { clase: u64, slot: u16 },
    Metodo { clase: u64 },
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
    pub max_pila: u16,
    pub ranuras: u16,
    pub nombre: Option<Rc<str>>,
    pub funcion: Option<Rc<FuncionCompilada>>,
    #[cfg(feature = "jit")]
    pub llamadas: Cell<u32>,
    #[cfg(feature = "jit")]
    pub iteraciones: Cell<u32>,
    #[cfg(feature = "jit")]
    pub jit: RefCell<EstadoJit>,
}

impl CodigoRuntime {
    pub fn de_trozo(trozo: &Trozo, ranuras: u16, nombre: Option<Rc<str>>) -> Rc<Self> {
        let n = trozo.instrucciones.len();
        Rc::new(Self {
            instrucciones: trozo.instrucciones.clone().into(),
            ubicaciones: trozo.ubicaciones.clone().into(),
            caches: (0..n).map(|_| Cell::new(CacheSitio::Vacia)).collect(),
            especializadas: trozo.instrucciones.iter().copied().map(Cell::new).collect(),
            max_pila: trozo.max_pila,
            ranuras,
            nombre,
            funcion: None,
            #[cfg(feature = "jit")]
            llamadas: Cell::new(0),
            #[cfg(feature = "jit")]
            iteraciones: Cell::new(0),
            #[cfg(feature = "jit")]
            jit: RefCell::new(EstadoJit::Frio),
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

    #[cfg(feature = "jit")]
    pub fn registrar_llamada(&self) {
        self.llamadas.set(self.llamadas.get().saturating_add(1));
    }

    #[cfg(feature = "jit")]
    pub fn registrar_retroceso(&self) {
        self.iteraciones
            .set(self.iteraciones.get().saturating_add(1));
    }

    #[cfg(feature = "jit")]
    pub fn suficientemente_caliente(&self) -> bool {
        self.llamadas.get() >= UMBRAL_LLAMADAS_JIT
            || self.iteraciones.get() >= UMBRAL_ITERACIONES_JIT
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

pub fn clave_clase(clase: &Rc<crate::valores::ClaseObjeto>) -> u64 {
    clase.id
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
