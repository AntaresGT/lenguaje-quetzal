use bytecode::{CodigoOp, Constante, FuncionCompilada, ModuloCompilado};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    AbiParam, Block, InstBuilder, MemFlagsData, StackSlot, StackSlotData, StackSlotKind, Value,
    types,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

use crate::analisis::{Analisis, analizar};
use crate::compilados::FirmaNativa;
use crate::{ESTADO_DESBORDAMIENTO, ESTADO_DIVISION_CERO, ESTADO_EXITO, ErrorJit, FuncionNativa};

pub(crate) fn compilar(
    modulo: &ModuloCompilado,
    funcion: &FuncionCompilada,
) -> Result<(FuncionNativa, usize), ErrorJit> {
    let analisis = analizar(modulo, funcion)?;
    let constructor = JITBuilder::new(default_libcall_names())
        .map_err(|error| ErrorJit::Compilacion(error.to_string()))?;
    let mut modulo_jit = JITModule::new(constructor);
    let mut contexto = modulo_jit.make_context();
    let configuracion_objetivo = modulo_jit.target_config();
    let puntero_tipo = configuracion_objetivo.pointer_type();
    contexto
        .func
        .signature
        .params
        .push(AbiParam::new(puntero_tipo));
    contexto
        .func
        .signature
        .params
        .push(AbiParam::new(types::I64));
    contexto
        .func
        .signature
        .params
        .push(AbiParam::new(puntero_tipo));
    let id = modulo_jit
        .declare_function("quetzal_entera", Linkage::Export, &contexto.func.signature)
        .map_err(|error| ErrorJit::Compilacion(error.to_string()))?;
    let mut constructor_funcion = FunctionBuilderContext::new();
    {
        let mut generador = FunctionBuilder::new(&mut contexto.func, &mut constructor_funcion);
        traducir_funcion(&mut generador, modulo, funcion, &analisis, puntero_tipo)?;
        generador.finalize(configuracion_objetivo);
    }
    modulo_jit
        .define_function(id, &mut contexto)
        .map_err(|error| ErrorJit::Compilacion(error.to_string()))?;
    let bytes = contexto
        .compiled_code()
        .map(|codigo| codigo.code_buffer().len())
        .unwrap_or(0);
    modulo_jit.clear_context(&mut contexto);
    modulo_jit.finalize_definitions().map_err(|error| {
        ErrorJit::Compilacion(format!("no se pudo finalizar el módulo: {error:?}"))
    })?;
    let puntero = modulo_jit.get_finalized_function(id);
    Ok((
        FuncionNativa {
            _modulo: modulo_jit,
            puntero,
            aridad: funcion.parametros.len(),
            firma: FirmaNativa::Entera,
        },
        bytes,
    ))
}

fn traducir_funcion(
    generador: &mut FunctionBuilder,
    modulo: &ModuloCompilado,
    funcion: &FuncionCompilada,
    analisis: &Analisis,
    puntero_tipo: cranelift_codegen::ir::Type,
) -> Result<(), ErrorJit> {
    let n = funcion.trozo.instrucciones.len();
    let bloques: Vec<Option<Block>> = analisis
        .profundidades
        .iter()
        .map(|profundidad| profundidad.map(|_| generador.create_block()))
        .collect();
    let entrada = bloques[0].expect("la entrada fue analizada");
    generador.append_block_params_for_function_params(entrada);
    let cantidad_slots = funcion.ranuras_locales as usize + analisis.max_pila as usize;
    let slots: Vec<StackSlot> = (0..cantidad_slots)
        .map(|_| {
            generador.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3))
        })
        .collect();
    generador.switch_to_block(entrada);
    let parametros = generador.block_params(entrada).to_vec();
    let direccion_argumentos = parametros[0];
    let direccion_salida = parametros[2];
    for slot in &slots {
        let cero = generador.ins().iconst(types::I64, 0);
        generador.ins().stack_store(puntero_tipo, cero, *slot, 0);
    }
    for (indice, slot) in slots.iter().take(funcion.parametros.len()).enumerate() {
        let argumento = generador.ins().load(
            types::I64,
            MemFlagsData::new(),
            direccion_argumentos,
            (indice * 8) as i32,
        );
        generador
            .ins()
            .stack_store(puntero_tipo, argumento, *slot, 0);
    }

    for pc in 0..n {
        let Some(bloque) = bloques[pc] else {
            continue;
        };
        if pc != 0 {
            generador.switch_to_block(bloque);
        }
        let instruccion = funcion.trozo.instrucciones[pc];
        let mut profundidad = analisis.profundidades[pc].expect("pc alcanzable") as usize;
        match instruccion.op {
            CodigoOp::CargarEntero => {
                let valor = generador
                    .ins()
                    .iconst(types::I64, instruccion.entero_inmediato() as i64);
                pila_guardar(
                    generador,
                    &slots,
                    funcion,
                    &mut profundidad,
                    valor,
                    puntero_tipo,
                );
            }
            CodigoOp::CargarConstante => {
                let valor = match &modulo.constantes[instruccion.indice_u32() as usize] {
                    Constante::Entero(valor) => *valor,
                    Constante::Log(valor) => i64::from(*valor),
                    _ => return Err(fallo_traduccion(pc, "constante no entera")),
                };
                let valor = generador.ins().iconst(types::I64, valor);
                pila_guardar(
                    generador,
                    &slots,
                    funcion,
                    &mut profundidad,
                    valor,
                    puntero_tipo,
                );
            }
            CodigoOp::CargarLocal => {
                let valor = local_cargar(generador, &slots, instruccion.ranura(), puntero_tipo);
                pila_guardar(
                    generador,
                    &slots,
                    funcion,
                    &mut profundidad,
                    valor,
                    puntero_tipo,
                );
            }
            CodigoOp::GuardarLocal => {
                let valor = pila_sacar(generador, &slots, funcion, &mut profundidad, puntero_tipo);
                local_guardar(generador, &slots, instruccion.ranura(), valor, puntero_tipo);
            }
            CodigoOp::LimpiarLocales => {
                for slot in slots
                    .iter()
                    .skip(instruccion.ranura() as usize)
                    .take(funcion.ranuras_locales as usize - instruccion.ranura() as usize)
                {
                    let cero = generador.ins().iconst(types::I64, 0);
                    generador.ins().stack_store(puntero_tipo, cero, *slot, 0);
                }
            }
            CodigoOp::AjustarLocal => {
                let actual = local_cargar(generador, &slots, instruccion.ranura(), puntero_tipo);
                let delta = generador
                    .ins()
                    .iconst(types::I64, instruccion.delta_ajuste() as i64);
                let valor = operacion_overflow(
                    generador,
                    instruccion.op,
                    actual,
                    delta,
                    direccion_salida,
                    pc as u32,
                );
                local_guardar(generador, &slots, instruccion.ranura(), valor, puntero_tipo);
            }
            CodigoOp::AjustarLocalConLocal => {
                let izquierda = local_cargar(generador, &slots, instruccion.ranura(), puntero_tipo);
                let derecha = local_cargar(
                    generador,
                    &slots,
                    instruccion.ranura_derecha(),
                    puntero_tipo,
                );
                let valor = operacion_overflow(
                    generador,
                    instruccion.op,
                    izquierda,
                    derecha,
                    direccion_salida,
                    pc as u32,
                );
                local_guardar(generador, &slots, instruccion.ranura(), valor, puntero_tipo);
            }
            CodigoOp::CargarLocalLocalSumarEntero => {
                let izquierda = local_cargar(generador, &slots, instruccion.ranura(), puntero_tipo);
                let derecha = local_cargar(
                    generador,
                    &slots,
                    instruccion.ranura_derecha(),
                    puntero_tipo,
                );
                let valor = operacion_overflow(
                    generador,
                    instruccion.op,
                    izquierda,
                    derecha,
                    direccion_salida,
                    pc as u32,
                );
                pila_guardar(
                    generador,
                    &slots,
                    funcion,
                    &mut profundidad,
                    valor,
                    puntero_tipo,
                );
            }
            CodigoOp::Duplicar => {
                let valor = pila_cargar(generador, &slots, funcion, profundidad - 1, puntero_tipo);
                pila_guardar(
                    generador,
                    &slots,
                    funcion,
                    &mut profundidad,
                    valor,
                    puntero_tipo,
                );
            }
            CodigoOp::Desechar => {}
            CodigoOp::NegarEntero => {
                let valor = pila_sacar(generador, &slots, funcion, &mut profundidad, puntero_tipo);
                let minimo = generador.ins().icmp_imm_s(IntCC::Equal, valor, i64::MIN);
                ramificar_error(
                    generador,
                    minimo,
                    direccion_salida,
                    ESTADO_DESBORDAMIENTO,
                    pc as u32,
                    detalle(CodigoOp::NegarEntero, valor, valor),
                );
                let negado = generador.ins().ineg(valor);
                pila_guardar(
                    generador,
                    &slots,
                    funcion,
                    &mut profundidad,
                    negado,
                    puntero_tipo,
                );
            }
            CodigoOp::NoLogico => {
                let valor = pila_sacar(generador, &slots, funcion, &mut profundidad, puntero_tipo);
                let comparacion = generador.ins().icmp_imm_s(IntCC::Equal, valor, 0);
                let negado = generador.ins().bmask(types::I64, comparacion);
                pila_guardar(
                    generador,
                    &slots,
                    funcion,
                    &mut profundidad,
                    negado,
                    puntero_tipo,
                );
            }
            CodigoOp::SumarEntero
            | CodigoOp::RestarEntero
            | CodigoOp::MultiplicarEntero
            | CodigoOp::DividirEntero
            | CodigoOp::ModuloEntero => {
                let derecha =
                    pila_sacar(generador, &slots, funcion, &mut profundidad, puntero_tipo);
                let izquierda =
                    pila_sacar(generador, &slots, funcion, &mut profundidad, puntero_tipo);
                let valor = operacion_aritmetica(
                    generador,
                    instruccion.op,
                    izquierda,
                    derecha,
                    direccion_salida,
                    pc as u32,
                );
                pila_guardar(
                    generador,
                    &slots,
                    funcion,
                    &mut profundidad,
                    valor,
                    puntero_tipo,
                );
            }
            CodigoOp::IgualEntero
            | CodigoOp::DiferenteEntero
            | CodigoOp::MayorEntero
            | CodigoOp::MenorEntero
            | CodigoOp::MayorOIgualEntero
            | CodigoOp::MenorOIgualEntero => {
                let derecha =
                    pila_sacar(generador, &slots, funcion, &mut profundidad, puntero_tipo);
                let izquierda =
                    pila_sacar(generador, &slots, funcion, &mut profundidad, puntero_tipo);
                let comparacion =
                    generador
                        .ins()
                        .icmp(condicion(instruccion.op), izquierda, derecha);
                let valor = generador.ins().bmask(types::I64, comparacion);
                pila_guardar(
                    generador,
                    &slots,
                    funcion,
                    &mut profundidad,
                    valor,
                    puntero_tipo,
                );
            }
            CodigoOp::Saltar => {
                generador.ins().jump(
                    bloques[instruccion.destino() as usize].expect("destino analizado"),
                    &[],
                );
                continue;
            }
            CodigoOp::SaltarSiFalso | CodigoOp::SaltarSiVerdadero => {
                let condicion =
                    pila_sacar(generador, &slots, funcion, &mut profundidad, puntero_tipo);
                let destino = bloques[instruccion.destino() as usize].expect("destino analizado");
                let siguiente = bloques[pc + 1].expect("siguiente analizado");
                if instruccion.op == CodigoOp::SaltarSiFalso {
                    generador
                        .ins()
                        .brif(condicion, siguiente, &[], destino, &[]);
                } else {
                    generador
                        .ins()
                        .brif(condicion, destino, &[], siguiente, &[]);
                }
                continue;
            }
            CodigoOp::MenorEnteroSaltarSiFalso => {
                let derecha =
                    pila_sacar(generador, &slots, funcion, &mut profundidad, puntero_tipo);
                let izquierda =
                    pila_sacar(generador, &slots, funcion, &mut profundidad, puntero_tipo);
                let comparacion = generador
                    .ins()
                    .icmp(IntCC::SignedLessThan, izquierda, derecha);
                let destino = bloques[instruccion.destino() as usize].expect("destino analizado");
                let siguiente = bloques[pc + 1].expect("siguiente analizado");
                generador
                    .ins()
                    .brif(comparacion, siguiente, &[], destino, &[]);
                continue;
            }
            CodigoOp::Retornar => {
                let valor = pila_sacar(generador, &slots, funcion, &mut profundidad, puntero_tipo);
                emitir_salida(generador, direccion_salida, valor, ESTADO_EXITO, 0, None);
                continue;
            }
            CodigoOp::RetornarLocal => {
                let valor = local_cargar(generador, &slots, instruccion.ranura(), puntero_tipo);
                emitir_salida(generador, direccion_salida, valor, ESTADO_EXITO, 0, None);
                continue;
            }
            _ => return Err(fallo_traduccion(pc, "instrucción no soportada")),
        }
        let siguiente = bloques
            .get(pc + 1)
            .and_then(|bloque| *bloque)
            .ok_or_else(|| fallo_traduccion(pc, "camino sin sucesor nativo"))?;
        generador.ins().jump(siguiente, &[]);
    }
    for bloque in bloques.into_iter().flatten() {
        generador.seal_block(bloque);
    }
    Ok(())
}

fn local_cargar(
    generador: &mut FunctionBuilder,
    slots: &[StackSlot],
    ranura: u16,
    puntero_tipo: cranelift_codegen::ir::Type,
) -> Value {
    generador
        .ins()
        .stack_load(puntero_tipo, types::I64, slots[ranura as usize], 0)
}

fn local_guardar(
    generador: &mut FunctionBuilder,
    slots: &[StackSlot],
    ranura: u16,
    valor: Value,
    puntero_tipo: cranelift_codegen::ir::Type,
) {
    generador
        .ins()
        .stack_store(puntero_tipo, valor, slots[ranura as usize], 0);
}

fn pila_cargar(
    generador: &mut FunctionBuilder,
    slots: &[StackSlot],
    funcion: &FuncionCompilada,
    indice: usize,
    puntero_tipo: cranelift_codegen::ir::Type,
) -> Value {
    generador.ins().stack_load(
        puntero_tipo,
        types::I64,
        slots[funcion.ranuras_locales as usize + indice],
        0,
    )
}

fn pila_sacar(
    generador: &mut FunctionBuilder,
    slots: &[StackSlot],
    funcion: &FuncionCompilada,
    profundidad: &mut usize,
    puntero_tipo: cranelift_codegen::ir::Type,
) -> Value {
    *profundidad -= 1;
    pila_cargar(generador, slots, funcion, *profundidad, puntero_tipo)
}

fn pila_guardar(
    generador: &mut FunctionBuilder,
    slots: &[StackSlot],
    funcion: &FuncionCompilada,
    profundidad: &mut usize,
    valor: Value,
    puntero_tipo: cranelift_codegen::ir::Type,
) {
    generador.ins().stack_store(
        puntero_tipo,
        valor,
        slots[funcion.ranuras_locales as usize + *profundidad],
        0,
    );
    *profundidad += 1;
}

#[derive(Clone, Copy)]
struct DetalleError {
    operacion: u32,
    izquierda: Value,
    derecha: Value,
}

fn detalle(op: CodigoOp, izquierda: Value, derecha: Value) -> DetalleError {
    let operacion = match op {
        CodigoOp::SumarEntero
        | CodigoOp::CargarLocalLocalSumarEntero
        | CodigoOp::AjustarLocalConLocal => crate::OPERACION_SUMA,
        CodigoOp::RestarEntero => crate::OPERACION_RESTA,
        CodigoOp::MultiplicarEntero => crate::OPERACION_MULTIPLICACION,
        CodigoOp::DividirEntero => crate::OPERACION_DIVISION,
        CodigoOp::ModuloEntero => crate::OPERACION_MODULO,
        CodigoOp::AjustarLocal => crate::OPERACION_AJUSTE,
        CodigoOp::NegarEntero => crate::OPERACION_NEGACION,
        _ => crate::OPERACION_NINGUNA,
    };
    DetalleError {
        operacion,
        izquierda,
        derecha,
    }
}

fn operacion_aritmetica(
    generador: &mut FunctionBuilder,
    op: CodigoOp,
    izquierda: Value,
    derecha: Value,
    salida: Value,
    pc: u32,
) -> Value {
    match op {
        CodigoOp::SumarEntero | CodigoOp::RestarEntero | CodigoOp::MultiplicarEntero => {
            operacion_overflow(generador, op, izquierda, derecha, salida, pc)
        }
        CodigoOp::DividirEntero | CodigoOp::ModuloEntero => {
            let divisor_cero = generador.ins().icmp_imm_s(IntCC::Equal, derecha, 0);
            ramificar_error(
                generador,
                divisor_cero,
                salida,
                ESTADO_DIVISION_CERO,
                pc,
                detalle(op, izquierda, derecha),
            );
            let minimo = generador
                .ins()
                .icmp_imm_s(IntCC::Equal, izquierda, i64::MIN);
            let menos_uno = generador.ins().icmp_imm_s(IntCC::Equal, derecha, -1);
            let ambos = generador.ins().band(minimo, menos_uno);
            ramificar_error(
                generador,
                ambos,
                salida,
                ESTADO_DESBORDAMIENTO,
                pc,
                detalle(op, izquierda, derecha),
            );
            if op == CodigoOp::DividirEntero {
                generador.ins().sdiv(izquierda, derecha)
            } else {
                generador.ins().srem(izquierda, derecha)
            }
        }
        _ => unreachable!("operación aritmética verificada"),
    }
}

fn operacion_overflow(
    generador: &mut FunctionBuilder,
    op: CodigoOp,
    izquierda: Value,
    derecha: Value,
    salida: Value,
    pc: u32,
) -> Value {
    let (valor, overflow) = match op {
        CodigoOp::SumarEntero
        | CodigoOp::CargarLocalLocalSumarEntero
        | CodigoOp::AjustarLocal
        | CodigoOp::AjustarLocalConLocal => generador.ins().sadd_overflow(izquierda, derecha),
        CodigoOp::RestarEntero => generador.ins().ssub_overflow(izquierda, derecha),
        CodigoOp::MultiplicarEntero => generador.ins().smul_overflow(izquierda, derecha),
        _ => unreachable!("operación con overflow verificada"),
    };
    ramificar_error(
        generador,
        overflow,
        salida,
        ESTADO_DESBORDAMIENTO,
        pc,
        detalle(op, izquierda, derecha),
    );
    valor
}

fn ramificar_error(
    generador: &mut FunctionBuilder,
    condicion: Value,
    direccion_salida: Value,
    estado: u32,
    pc: u32,
    detalle: DetalleError,
) {
    let error = generador.create_block();
    let continuar = generador.create_block();
    generador.ins().brif(condicion, error, &[], continuar, &[]);
    generador.switch_to_block(error);
    let cero = generador.ins().iconst(types::I64, 0);
    emitir_salida(generador, direccion_salida, cero, estado, pc, Some(detalle));
    generador.switch_to_block(continuar);
    generador.seal_block(error);
    generador.seal_block(continuar);
}

fn emitir_salida(
    generador: &mut FunctionBuilder,
    direccion_salida: Value,
    valor: Value,
    estado: u32,
    pc: u32,
    detalle: Option<DetalleError>,
) {
    generador
        .ins()
        .store(MemFlagsData::new(), valor, direccion_salida, 0);
    let (izquierda, derecha, operacion) = match detalle {
        Some(detalle) => (detalle.izquierda, detalle.derecha, detalle.operacion),
        None => (valor, valor, crate::OPERACION_NINGUNA),
    };
    generador
        .ins()
        .store(MemFlagsData::new(), izquierda, direccion_salida, 8);
    generador
        .ins()
        .store(MemFlagsData::new(), derecha, direccion_salida, 16);
    let estado = generador.ins().iconst(types::I32, estado as i64);
    generador
        .ins()
        .store(MemFlagsData::new(), estado, direccion_salida, 24);
    let pc = generador.ins().iconst(types::I32, pc as i64);
    generador
        .ins()
        .store(MemFlagsData::new(), pc, direccion_salida, 28);
    let operacion = generador.ins().iconst(types::I32, operacion as i64);
    generador
        .ins()
        .store(MemFlagsData::new(), operacion, direccion_salida, 32);
    generador.ins().return_(&[]);
}

fn condicion(op: CodigoOp) -> IntCC {
    match op {
        CodigoOp::IgualEntero => IntCC::Equal,
        CodigoOp::DiferenteEntero => IntCC::NotEqual,
        CodigoOp::MayorEntero => IntCC::SignedGreaterThan,
        CodigoOp::MenorEntero => IntCC::SignedLessThan,
        CodigoOp::MayorOIgualEntero => IntCC::SignedGreaterThanOrEqual,
        CodigoOp::MenorOIgualEntero => IntCC::SignedLessThanOrEqual,
        _ => unreachable!("comparación entera verificada"),
    }
}

fn fallo_traduccion(pc: usize, mensaje: &str) -> ErrorJit {
    ErrorJit::Compilacion(format!("pc {pc}: {mensaje}"))
}
