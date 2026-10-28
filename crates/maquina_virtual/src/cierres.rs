//! Compilación a cierres de funciones calientes (nivel 1, Rust seguro).
//!
//! El subconjunto entero/log/locales/saltos/atributos se ejecuta en un
//! bucle especializado. Cualquier otra instrucción devuelve el `pc` para
//! que el intérprete reanude.

use std::rc::Rc;

use bytecode::CodigoOp;

use crate::codigo::{CierreCompilado, CodigoRuntime, ResultadoCierre};
use crate::errores::Fallo;
use crate::valores::Valor;
use crate::vm::Vm;

fn es_subset_entero(op: CodigoOp) -> bool {
    matches!(
        op,
        CodigoOp::CargarEntero
            | CodigoOp::CargarLocal
            | CodigoOp::GuardarLocal
            | CodigoOp::AjustarLocal
            | CodigoOp::LimpiarLocales
            | CodigoOp::CargarLocalLocalSumarEntero
            | CodigoOp::SumarEntero
            | CodigoOp::RestarEntero
            | CodigoOp::MultiplicarEntero
            | CodigoOp::DividirEntero
            | CodigoOp::ModuloEntero
            | CodigoOp::NegarEntero
            | CodigoOp::IgualEntero
            | CodigoOp::DiferenteEntero
            | CodigoOp::MayorEntero
            | CodigoOp::MenorEntero
            | CodigoOp::MayorOIgualEntero
            | CodigoOp::MenorOIgualEntero
            | CodigoOp::MenorEnteroSaltarSiFalso
            | CodigoOp::Saltar
            | CodigoOp::SaltarSiFalso
            | CodigoOp::SaltarSiVerdadero
            | CodigoOp::Retornar
            | CodigoOp::RetornarLocal
            | CodigoOp::CargarEsto
            | CodigoOp::CargarEstoAtributo
            | CodigoOp::CargarAtributo
            | CodigoOp::GuardarAtributo
            | CodigoOp::Duplicar
            | CodigoOp::Desechar
    )
}

/// Compila un código a cierre si todas sus instrucciones (u la mayoría
/// caliente) entran en el subconjunto entero. Si no, `None`.
pub fn compilar(codigo: Rc<CodigoRuntime>) -> Option<CierreCompilado> {
    let cubiertas = codigo
        .instrucciones
        .iter()
        .filter(|instruccion| es_subset_entero(instruccion.op))
        .count();
    if codigo.instrucciones.is_empty() || cubiertas * 2 < codigo.instrucciones.len() {
        return None;
    }
    Some(CierreCompilado {
        correr: Box::new(move |vm: &mut Vm, pc_inicio| correr_cierre(vm, &codigo, pc_inicio)),
    })
}

fn correr_cierre(
    vm: &mut Vm,
    codigo: &CodigoRuntime,
    pc_inicio: u32,
) -> Result<ResultadoCierre, Fallo> {
    let mut pc = pc_inicio as usize;
    let n = codigo.instrucciones.len();
    while pc < n {
        let instruccion = codigo.instruccion(pc, vm.optimizar);
        if !es_subset_entero(instruccion.op) {
            return Ok(ResultadoCierre::Interpretar { pc: pc as u32 });
        }
        match vm.ejecutar_op_cierre(instruccion, pc as u32)? {
            ControlCierre::Continuar => pc += 1,
            ControlCierre::Saltar(destino) => pc = destino as usize,
            ControlCierre::Retornar(valor) => return Ok(ResultadoCierre::Valor(valor)),
            ControlCierre::Interpretar => {
                return Ok(ResultadoCierre::Interpretar { pc: pc as u32 });
            }
        }
    }
    Ok(ResultadoCierre::Valor(vm.pila.pop().unwrap_or(Valor::Nulo)))
}

pub enum ControlCierre {
    Continuar,
    Saltar(u32),
    Retornar(Valor),
    Interpretar,
}
