//! Pase de bytecode: plegado de enteros, saltos a saltos, inmediatos, `i++`
//! y superinstrucciones del camino caliente.

use std::collections::HashSet;

use nucleo::Ubicacion;

use crate::constantes::Constante;
use crate::instruccion::{CodigoOp, Instruccion, Trozo};
use crate::modulo::{FuncionCompilada, ModuloCompilado};

/// Optimiza todos los trozos de un módulo compilado.
pub fn optimizar_modulo(modulo: &mut ModuloCompilado) {
    let mut constantes = std::mem::take(&mut modulo.constantes);
    modulo.principal = optimizar_trozo(std::mem::take(&mut modulo.principal), &mut constantes);
    for funcion in &mut modulo.funciones {
        optimizar_funcion(funcion, &mut constantes);
    }
    for objeto in &mut modulo.objetos {
        for metodo in &mut objeto.metodos {
            optimizar_funcion(&mut metodo.funcion, &mut constantes);
        }
        for constructor in &mut objeto.constructores {
            optimizar_funcion(constructor, &mut constantes);
        }
        for atributo in &mut objeto.atributos {
            if let Some(inicial) = atributo.inicial.take() {
                atributo.inicial = Some(optimizar_trozo(inicial, &mut constantes));
            }
        }
    }
    modulo.constantes = constantes;
}

fn optimizar_funcion(funcion: &mut FuncionCompilada, constantes: &mut Vec<Constante>) {
    funcion.trozo = optimizar_trozo(std::mem::take(&mut funcion.trozo), constantes);
}

/// Aplica las reglas de peephole a un trozo.
pub fn optimizar_trozo(trozo: Trozo, constantes: &mut Vec<Constante>) -> Trozo {
    if trozo.instrucciones.is_empty() {
        return trozo;
    }
    let mut ops: Vec<Option<(Instruccion, Ubicacion)>> = trozo
        .instrucciones
        .into_iter()
        .zip(trozo.ubicaciones)
        .map(Some)
        .collect();

    convertir_enteros_inmediatos(&mut ops, constantes);
    while plegar_enteros(&mut ops, constantes) {}
    fusionar_ajustar_local(&mut ops);
    fusionar_superinstrucciones(&mut ops);
    let mut trozo = compactar(ops);
    redirigir_saltos_a_saltos(&mut trozo);
    trozo
}

fn convertir_enteros_inmediatos(
    ops: &mut [Option<(Instruccion, Ubicacion)>],
    constantes: &[Constante],
) {
    for op in ops.iter_mut().flatten() {
        if op.0.op == CodigoOp::CargarConstante
            && let Some(Constante::Entero(valor)) = constantes.get(op.0.indice_u32() as usize)
            && let Ok(pequeno) = i16::try_from(*valor)
        {
            op.0 = Instruccion::CargarEntero(pequeno);
        }
    }
}

fn plegar_enteros(
    ops: &mut [Option<(Instruccion, Ubicacion)>],
    constantes: &mut Vec<Constante>,
) -> bool {
    let destinos = destinos_salto(ops);
    let mut cambio = false;
    let mut i = 0;
    while i < ops.len() {
        let Some(izquierda_pc) = siguiente_ocupada(ops, i) else {
            break;
        };
        let Some(derecha_pc) = siguiente_ocupada(ops, izquierda_pc + 1) else {
            break;
        };
        let Some(op_pc) = siguiente_ocupada(ops, derecha_pc + 1) else {
            break;
        };
        let Some(a) = entero_cargado(ops[izquierda_pc].as_ref().map(|par| par.0), constantes)
        else {
            i = izquierda_pc + 1;
            continue;
        };
        let Some(b) = entero_cargado(ops[derecha_pc].as_ref().map(|par| par.0), constantes) else {
            i = izquierda_pc + 1;
            continue;
        };
        let Some((operacion, ubicacion)) = ops[op_pc] else {
            i = izquierda_pc + 1;
            continue;
        };
        if destinos.contains(&(derecha_pc as u32)) || destinos.contains(&(op_pc as u32)) {
            i = izquierda_pc + 1;
            continue;
        }
        let Some(resultado) = evaluar_entera(operacion, a, b) else {
            i = izquierda_pc + 1;
            continue;
        };
        ops[izquierda_pc] = Some((cargar_entero(resultado, constantes), ubicacion));
        ops[derecha_pc] = None;
        ops[op_pc] = None;
        cambio = true;
        i = izquierda_pc + 1;
    }
    cambio
}

fn fusionar_ajustar_local(ops: &mut [Option<(Instruccion, Ubicacion)>]) {
    let destinos = destinos_salto(ops);
    let mut i = 0;
    while i < ops.len() {
        let Some(carga_pc) = siguiente_ocupada(ops, i) else {
            break;
        };
        let Some(delta_pc) = siguiente_ocupada(ops, carga_pc + 1) else {
            break;
        };
        let Some(op_pc) = siguiente_ocupada(ops, delta_pc + 1) else {
            break;
        };
        let Some(guarda_pc) = siguiente_ocupada(ops, op_pc + 1) else {
            break;
        };
        let Some((carga, ubicacion)) = ops[carga_pc] else {
            i = carga_pc + 1;
            continue;
        };
        if carga.op != CodigoOp::CargarLocal {
            i = carga_pc + 1;
            continue;
        }
        let ranura = carga.ranura();
        let Some((delta_op, _)) = ops[delta_pc] else {
            i = carga_pc + 1;
            continue;
        };
        if delta_op.op != CodigoOp::CargarEntero {
            i = carga_pc + 1;
            continue;
        }
        let delta = delta_op.entero_inmediato();
        let Some((operacion, _)) = ops[op_pc] else {
            i = carga_pc + 1;
            continue;
        };
        let Some((guarda, _)) = ops[guarda_pc] else {
            i = carga_pc + 1;
            continue;
        };
        if guarda.op != CodigoOp::GuardarLocal || guarda.ranura() != ranura {
            i = carga_pc + 1;
            continue;
        }
        if destinos.contains(&(delta_pc as u32))
            || destinos.contains(&(op_pc as u32))
            || destinos.contains(&(guarda_pc as u32))
        {
            i = carga_pc + 1;
            continue;
        }
        let ajuste = match operacion.op {
            CodigoOp::SumarEntero => Some(delta),
            CodigoOp::RestarEntero => delta.checked_neg(),
            _ => None,
        };
        let Some(ajuste) = ajuste else {
            i = carga_pc + 1;
            continue;
        };
        ops[carga_pc] = Some((Instruccion::AjustarLocal(ranura, ajuste), ubicacion));
        ops[delta_pc] = None;
        ops[op_pc] = None;
        ops[guarda_pc] = None;
        i = carga_pc + 1;
    }
}

fn fusionar_superinstrucciones(ops: &mut [Option<(Instruccion, Ubicacion)>]) {
    let destinos = destinos_salto(ops);
    let mut i = 0;
    while i < ops.len() {
        let Some(a_pc) = siguiente_ocupada(ops, i) else {
            break;
        };
        let Some((a, ubicacion)) = ops[a_pc] else {
            break;
        };

        if let Some(b_pc) = siguiente_ocupada(ops, a_pc + 1)
            && !destinos.contains(&(b_pc as u32))
            && let Some((b, _)) = ops[b_pc]
        {
            if a.op == CodigoOp::MenorEntero && b.op == CodigoOp::SaltarSiFalso {
                ops[a_pc] = Some((
                    Instruccion::MenorEnteroSaltarSiFalso(b.destino()),
                    ubicacion,
                ));
                ops[b_pc] = None;
                i = a_pc + 1;
                continue;
            }
            if a.op == CodigoOp::CargarLocal && b.op == CodigoOp::Retornar {
                ops[a_pc] = Some((Instruccion::RetornarLocal(a.ranura()), ubicacion));
                ops[b_pc] = None;
                i = a_pc + 1;
                continue;
            }
            if a.op == CodigoOp::CargarEsto && b.op == CodigoOp::CargarAtributo {
                ops[a_pc] = Some((Instruccion::CargarEstoAtributo(b.slot()), ubicacion));
                ops[b_pc] = None;
                i = a_pc + 1;
                continue;
            }
            if let Some(c_pc) = siguiente_ocupada(ops, b_pc + 1)
                && !destinos.contains(&(c_pc as u32))
                && let Some((c, _)) = ops[c_pc]
                && a.op == CodigoOp::CargarLocal
                && b.op == CodigoOp::CargarLocal
                && c.op == CodigoOp::SumarEntero
            {
                ops[a_pc] = Some((
                    Instruccion::CargarLocalLocalSumarEntero(a.ranura(), b.ranura()),
                    ubicacion,
                ));
                ops[b_pc] = None;
                ops[c_pc] = None;
                i = a_pc + 1;
                continue;
            }
        }
        i = a_pc + 1;
    }
}

fn compactar(ops: Vec<Option<(Instruccion, Ubicacion)>>) -> Trozo {
    let n = ops.len();
    let mut mapa = vec![0u32; n + 1];
    let mut nuevo_pc = 0u32;
    for (viejo, op) in ops.iter().enumerate() {
        mapa[viejo] = nuevo_pc;
        if op.is_some() {
            nuevo_pc += 1;
        }
    }
    mapa[n] = nuevo_pc;

    let mut instrucciones = Vec::with_capacity(nuevo_pc as usize);
    let mut ubicaciones = Vec::with_capacity(nuevo_pc as usize);
    for op in ops.into_iter().flatten() {
        instrucciones.push(remapear_salto(op.0, &mapa));
        ubicaciones.push(op.1);
    }
    Trozo {
        instrucciones,
        ubicaciones,
        max_pila: 0,
    }
}

fn remapear_salto(instruccion: Instruccion, mapa: &[u32]) -> Instruccion {
    let Some(destino) = instruccion.destino_salto() else {
        return instruccion;
    };
    let destino = (destino as usize).min(mapa.len().saturating_sub(1));
    instruccion.con_destino_salto(mapa[destino])
}

fn redirigir_saltos_a_saltos(trozo: &mut Trozo) {
    let n = trozo.instrucciones.len();
    for i in 0..n {
        let Some(inicio) = trozo.instrucciones[i].destino_salto() else {
            continue;
        };
        let destino = seguir_saltos(&trozo.instrucciones, inicio);
        if destino != inicio {
            trozo.instrucciones[i] = trozo.instrucciones[i].con_destino_salto(destino);
        }
    }
}

fn seguir_saltos(instrucciones: &[Instruccion], mut destino: u32) -> u32 {
    let mut vistos = HashSet::new();
    loop {
        if !vistos.insert(destino) {
            return destino;
        }
        match instrucciones.get(destino as usize) {
            Some(instruccion) if instruccion.op == CodigoOp::Saltar => {
                destino = instruccion.destino();
            }
            _ => return destino,
        }
    }
}

fn destinos_salto(ops: &[Option<(Instruccion, Ubicacion)>]) -> HashSet<u32> {
    let mut destinos = HashSet::new();
    for op in ops.iter().flatten() {
        if let Some(destino) = op.0.destino_salto() {
            destinos.insert(destino);
        }
    }
    destinos
}

fn siguiente_ocupada(ops: &[Option<(Instruccion, Ubicacion)>], desde: usize) -> Option<usize> {
    (desde..ops.len()).find(|&indice| ops[indice].is_some())
}

fn entero_cargado(instruccion: Option<Instruccion>, constantes: &[Constante]) -> Option<i64> {
    let instruccion = instruccion?;
    match instruccion.op {
        CodigoOp::CargarEntero => Some(instruccion.entero_inmediato() as i64),
        CodigoOp::CargarConstante => match constantes.get(instruccion.indice_u32() as usize)? {
            Constante::Entero(valor) => Some(*valor),
            _ => None,
        },
        _ => None,
    }
}

fn evaluar_entera(operacion: Instruccion, izquierda: i64, derecha: i64) -> Option<i64> {
    match operacion.op {
        CodigoOp::SumarEntero => izquierda.checked_add(derecha),
        CodigoOp::RestarEntero => izquierda.checked_sub(derecha),
        CodigoOp::MultiplicarEntero => izquierda.checked_mul(derecha),
        CodigoOp::DividirEntero if derecha != 0 => izquierda.checked_div(derecha),
        CodigoOp::ModuloEntero if derecha != 0 => izquierda.checked_rem(derecha),
        _ => None,
    }
}

fn cargar_entero(valor: i64, constantes: &mut Vec<Constante>) -> Instruccion {
    if let Ok(pequeno) = i16::try_from(valor) {
        return Instruccion::CargarEntero(pequeno);
    }
    if let Some(indice) = constantes
        .iter()
        .position(|constante| constante == &Constante::Entero(valor))
    {
        return Instruccion::CargarConstante(indice as u32);
    }
    constantes.push(Constante::Entero(valor));
    Instruccion::CargarConstante((constantes.len() - 1) as u32)
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use nucleo::Ubicacion;

    fn trozo(ops: Vec<Instruccion>) -> Trozo {
        let ubicacion = Ubicacion::nueva(0, 1);
        Trozo {
            ubicaciones: vec![ubicacion; ops.len()],
            instrucciones: ops,
            max_pila: 0,
        }
    }

    #[test]
    fn plegar_deberia_sumar_constantes_enteras() {
        let mut constantes = vec![Constante::Entero(40), Constante::Entero(2)];
        let resultado = optimizar_trozo(
            trozo(vec![
                Instruccion::CargarConstante(0),
                Instruccion::CargarConstante(1),
                Instruccion::SumarEntero(),
            ]),
            &mut constantes,
        );
        assert_eq!(resultado.instrucciones, vec![Instruccion::CargarEntero(42)]);
    }

    #[test]
    fn plegar_no_deberia_dividir_por_cero() {
        let mut constantes = Vec::new();
        let resultado = optimizar_trozo(
            trozo(vec![
                Instruccion::CargarEntero(10),
                Instruccion::CargarEntero(0),
                Instruccion::DividirEntero(),
            ]),
            &mut constantes,
        );
        assert_eq!(
            resultado.instrucciones,
            vec![
                Instruccion::CargarEntero(10),
                Instruccion::CargarEntero(0),
                Instruccion::DividirEntero(),
            ]
        );
    }

    #[test]
    fn saltos_deberian_seguir_saltos_incondicionales() {
        let mut constantes = Vec::new();
        let resultado = optimizar_trozo(
            trozo(vec![
                Instruccion::SaltarSiFalso(1),
                Instruccion::Saltar(3),
                Instruccion::CargarEntero(1),
                Instruccion::Retornar(),
            ]),
            &mut constantes,
        );
        assert_eq!(resultado.instrucciones[0], Instruccion::SaltarSiFalso(3));
    }

    #[test]
    fn fusion_deberia_emitir_ajustar_local() {
        let mut constantes = Vec::new();
        let resultado = optimizar_trozo(
            trozo(vec![
                Instruccion::CargarLocal(0),
                Instruccion::CargarEntero(1),
                Instruccion::SumarEntero(),
                Instruccion::GuardarLocal(0),
            ]),
            &mut constantes,
        );
        assert_eq!(
            resultado.instrucciones,
            vec![Instruccion::AjustarLocal(0, 1)]
        );
    }

    #[test]
    fn fusion_deberia_emitir_menor_entero_saltar() {
        let mut constantes = Vec::new();
        let resultado = optimizar_trozo(
            trozo(vec![
                Instruccion::MenorEntero(),
                Instruccion::SaltarSiFalso(4),
                Instruccion::CargarEntero(1),
                Instruccion::CargarEntero(2),
                Instruccion::Retornar(),
            ]),
            &mut constantes,
        );
        assert_eq!(
            resultado.instrucciones[0],
            Instruccion::MenorEnteroSaltarSiFalso(3)
        );
    }
}
