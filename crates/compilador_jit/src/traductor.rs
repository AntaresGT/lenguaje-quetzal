use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

use bytecode::{CodigoOp, Constante, FuncionCompilada, ModuloCompilado};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    AbiParam, Block, InstBuilder, MemFlagsData, StackSlotData, StackSlotKind, Value, types,
};
use cranelift_codegen::isa::CallConv;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

use crate::analisis::{Analisis, analizar};
use crate::compilados::FirmaNativa;
use crate::contexto::{ClaveFuncion, clave_funcion};
use crate::{ESTADO_DESBORDAMIENTO, ESTADO_DIVISION_CERO, ESTADO_EXITO, ErrorJit, FuncionNativa};

pub(crate) fn compilar(
    modulo: &ModuloCompilado,
    funcion: &FuncionCompilada,
    compilados: &HashMap<ClaveFuncion, Rc<FuncionNativa>>,
    id_modulo: usize,
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
        traducir_funcion(
            &mut generador,
            modulo,
            funcion,
            &analisis,
            compilados,
            id_modulo,
            puntero_tipo,
        )?;
        generador.finalize(configuracion_objetivo);
    }
    if std::env::var_os("QUETZAL_JIT_IR").is_some() {
        eprintln!("{}", contexto.func.display());
    }
    modulo_jit
        .define_function(id, &mut contexto)
        .map_err(|error| ErrorJit::Compilacion(error.to_string()))?;
    let bytes = contexto
        .compiled_code()
        .map(|codigo| codigo.code_buffer().len())
        .unwrap_or(0);
    if let Some(ruta) = std::env::var_os("QUETZAL_JIT_BIN")
        && let Some(codigo) = contexto.compiled_code()
    {
        std::fs::write(&ruta, codigo.code_buffer())
            .map_err(|error| ErrorJit::Compilacion(error.to_string()))?;
    }
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
    compilados: &HashMap<ClaveFuncion, Rc<FuncionNativa>>,
    id_modulo: usize,
    puntero_tipo: cranelift_codegen::ir::Type,
) -> Result<(), ErrorJit> {
    let n = funcion.trozo.instrucciones.len();
    let id_funcion = modulo
        .funciones
        .iter()
        .position(|candidata| candidata.nombre == funcion.nombre)
        .unwrap_or(0) as u32;
    let lideres = lideres_cfg(funcion);
    let bloques: Vec<Option<Block>> = (0..n)
        .map(|pc| {
            if lideres.contains(&pc) && analisis.profundidades[pc].is_some() {
                Some(generador.create_block())
            } else {
                None
            }
        })
        .collect();
    let entrada = bloques[0].expect("la entrada fue analizada");
    generador.append_block_params_for_function_params(entrada);
    let variables = VariablesSsa::nueva(generador, funcion, analisis);
    generador.switch_to_block(entrada);
    let parametros = generador.block_params(entrada).to_vec();
    let direccion_argumentos = parametros[0];
    let direccion_salida = parametros[2];
    for local in &variables.locales {
        let cero = generador.ins().iconst(types::I64, 0);
        generador.def_var(*local, cero);
    }
    for (indice, local) in variables
        .locales
        .iter()
        .take(funcion.parametros.len())
        .enumerate()
    {
        let argumento = generador.ins().load(
            types::I64,
            MemFlagsData::new(),
            direccion_argumentos,
            (indice * 8) as i32,
        );
        generador.def_var(*local, argumento);
    }

    for pc in 0..n {
        if analisis.profundidades[pc].is_none() {
            continue;
        }
        if pc != 0 && lideres.contains(&pc) {
            generador.switch_to_block(bloques[pc].expect("bloque básico"));
        }
        let instruccion = funcion.trozo.instrucciones[pc];
        let mut profundidad = analisis.profundidades[pc].expect("pc alcanzable") as usize;
        match instruccion.op {
            CodigoOp::CargarEntero => {
                let valor = generador
                    .ins()
                    .iconst(types::I64, instruccion.entero_inmediato() as i64);
                pila_guardar(generador, &variables, &mut profundidad, valor);
            }
            CodigoOp::CargarConstante => {
                let valor = match &modulo.constantes[instruccion.indice_u32() as usize] {
                    Constante::Entero(valor) => *valor,
                    Constante::Log(valor) => i64::from(*valor),
                    _ => return Err(fallo_traduccion(pc, "constante no entera")),
                };
                let valor = generador.ins().iconst(types::I64, valor);
                pila_guardar(generador, &variables, &mut profundidad, valor);
            }
            CodigoOp::CargarGlobal => {
                let nombre = &modulo.nombres_globales[instruccion.indice_u16() as usize];
                let direccion = compilados
                    .get(&clave_funcion(id_modulo, nombre))
                    .ok_or_else(|| fallo_traduccion(pc, "función directa no compilada"))?
                    .puntero as i64;
                let valor = generador.ins().iconst(puntero_tipo, direccion);
                pila_guardar(generador, &variables, &mut profundidad, valor);
            }
            CodigoOp::CargarLocal => {
                let valor = local_cargar(generador, &variables, instruccion.ranura());
                pila_guardar(generador, &variables, &mut profundidad, valor);
            }
            CodigoOp::GuardarLocal => {
                let valor = pila_sacar(generador, &variables, &mut profundidad);
                local_guardar(generador, &variables, instruccion.ranura(), valor);
            }
            CodigoOp::LimpiarLocales => {
                for local in variables.locales.iter().skip(instruccion.ranura() as usize) {
                    let cero = generador.ins().iconst(types::I64, 0);
                    generador.def_var(*local, cero);
                }
            }
            CodigoOp::AjustarLocal => {
                let actual = local_cargar(generador, &variables, instruccion.ranura());
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
                    id_funcion,
                );
                local_guardar(generador, &variables, instruccion.ranura(), valor);
            }
            CodigoOp::AjustarLocalConLocal => {
                let izquierda = local_cargar(generador, &variables, instruccion.ranura());
                let derecha = local_cargar(generador, &variables, instruccion.ranura_derecha());
                let valor = operacion_overflow(
                    generador,
                    instruccion.op,
                    izquierda,
                    derecha,
                    direccion_salida,
                    pc as u32,
                    id_funcion,
                );
                local_guardar(generador, &variables, instruccion.ranura(), valor);
            }
            CodigoOp::CargarLocalLocalSumarEntero => {
                let izquierda = local_cargar(generador, &variables, instruccion.ranura());
                let derecha = local_cargar(generador, &variables, instruccion.ranura_derecha());
                let valor = operacion_overflow(
                    generador,
                    instruccion.op,
                    izquierda,
                    derecha,
                    direccion_salida,
                    pc as u32,
                    id_funcion,
                );
                pila_guardar(generador, &variables, &mut profundidad, valor);
            }
            CodigoOp::Duplicar => {
                let valor = pila_cargar(generador, &variables, profundidad - 1);
                pila_guardar(generador, &variables, &mut profundidad, valor);
            }
            CodigoOp::Desechar => {}
            CodigoOp::NegarEntero => {
                let valor = pila_sacar(generador, &variables, &mut profundidad);
                let minimo = generador.ins().icmp_imm_s(IntCC::Equal, valor, i64::MIN);
                ramificar_error(
                    generador,
                    minimo,
                    direccion_salida,
                    ESTADO_DESBORDAMIENTO,
                    pc as u32,
                    detalle(CodigoOp::NegarEntero, valor, valor),
                    id_funcion,
                );
                let negado = generador.ins().ineg(valor);
                pila_guardar(generador, &variables, &mut profundidad, negado);
            }
            CodigoOp::NoLogico => {
                let valor = pila_sacar(generador, &variables, &mut profundidad);
                let comparacion = generador.ins().icmp_imm_s(IntCC::Equal, valor, 0);
                let negado = generador.ins().bmask(types::I64, comparacion);
                pila_guardar(generador, &variables, &mut profundidad, negado);
            }
            CodigoOp::SumarEntero
            | CodigoOp::RestarEntero
            | CodigoOp::MultiplicarEntero
            | CodigoOp::DividirEntero
            | CodigoOp::ModuloEntero => {
                let derecha = pila_sacar(generador, &variables, &mut profundidad);
                let izquierda = pila_sacar(generador, &variables, &mut profundidad);
                let valor = operacion_aritmetica(
                    generador,
                    instruccion.op,
                    izquierda,
                    derecha,
                    direccion_salida,
                    pc as u32,
                    id_funcion,
                );
                pila_guardar(generador, &variables, &mut profundidad, valor);
            }
            CodigoOp::IgualEntero
            | CodigoOp::DiferenteEntero
            | CodigoOp::MayorEntero
            | CodigoOp::MenorEntero
            | CodigoOp::MayorOIgualEntero
            | CodigoOp::MenorOIgualEntero => {
                let derecha = pila_sacar(generador, &variables, &mut profundidad);
                let izquierda = pila_sacar(generador, &variables, &mut profundidad);
                let comparacion =
                    generador
                        .ins()
                        .icmp(condicion(instruccion.op), izquierda, derecha);
                let valor = generador.ins().bmask(types::I64, comparacion);
                pila_guardar(generador, &variables, &mut profundidad, valor);
            }
            CodigoOp::Llamar => {
                llamar_directa(
                    generador,
                    &variables,
                    &mut profundidad,
                    instruccion.cantidad() as usize,
                    direccion_salida,
                    puntero_tipo,
                    CallConv::SystemV,
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
                let condicion = pila_sacar(generador, &variables, &mut profundidad);
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
                let derecha = pila_sacar(generador, &variables, &mut profundidad);
                let izquierda = pila_sacar(generador, &variables, &mut profundidad);
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
                let valor = pila_sacar(generador, &variables, &mut profundidad);
                emitir_salida(
                    generador,
                    direccion_salida,
                    valor,
                    ESTADO_EXITO,
                    0,
                    None,
                    id_funcion,
                );
                continue;
            }
            CodigoOp::RetornarLocal => {
                let valor = local_cargar(generador, &variables, instruccion.ranura());
                emitir_salida(
                    generador,
                    direccion_salida,
                    valor,
                    ESTADO_EXITO,
                    0,
                    None,
                    id_funcion,
                );
                continue;
            }
            _ => return Err(fallo_traduccion(pc, "instrucción no soportada")),
        }
        if let Some(siguiente) = bloques.get(pc + 1).and_then(|bloque| *bloque) {
            generador.ins().jump(siguiente, &[]);
        }
    }
    for bloque in bloques.iter().flatten().copied().collect::<BTreeSet<_>>() {
        generador.seal_block(bloque);
    }
    Ok(())
}

struct VariablesSsa {
    locales: Vec<Variable>,
    pila: Vec<Variable>,
}

impl VariablesSsa {
    fn nueva(
        generador: &mut FunctionBuilder,
        funcion: &FuncionCompilada,
        analisis: &Analisis,
    ) -> Self {
        let locales = (0..funcion.ranuras_locales)
            .map(|_| generador.declare_var(types::I64))
            .collect();
        let pila = (0..analisis.max_pila)
            .map(|_| generador.declare_var(types::I64))
            .collect();
        Self { locales, pila }
    }
}

fn lideres_cfg(funcion: &FuncionCompilada) -> BTreeSet<usize> {
    let n = funcion.trozo.instrucciones.len();
    let mut lideres = BTreeSet::from([0]);
    for (pc, instruccion) in funcion.trozo.instrucciones.iter().enumerate() {
        if let Some(destino) = instruccion.destino_salto()
            && (destino as usize) < n
        {
            lideres.insert(destino as usize);
        }
        if matches!(
            instruccion.op,
            CodigoOp::SaltarSiFalso
                | CodigoOp::SaltarSiVerdadero
                | CodigoOp::MenorEnteroSaltarSiFalso
        ) && pc + 1 < n
        {
            lideres.insert(pc + 1);
        }
    }
    lideres
}

fn local_cargar(generador: &mut FunctionBuilder, variables: &VariablesSsa, ranura: u16) -> Value {
    generador.use_var(variables.locales[ranura as usize])
}

fn local_guardar(
    generador: &mut FunctionBuilder,
    variables: &VariablesSsa,
    ranura: u16,
    valor: Value,
) {
    generador.def_var(variables.locales[ranura as usize], valor);
}

fn pila_cargar(generador: &mut FunctionBuilder, variables: &VariablesSsa, indice: usize) -> Value {
    generador.use_var(variables.pila[indice])
}

fn pila_sacar(
    generador: &mut FunctionBuilder,
    variables: &VariablesSsa,
    profundidad: &mut usize,
) -> Value {
    *profundidad -= 1;
    pila_cargar(generador, variables, *profundidad)
}

fn pila_guardar(
    generador: &mut FunctionBuilder,
    variables: &VariablesSsa,
    profundidad: &mut usize,
    valor: Value,
) {
    generador.def_var(variables.pila[*profundidad], valor);
    *profundidad += 1;
}

/// Emite una llamada a una función compilada por este mismo contexto. La
/// dirección nunca procede del programa y el registro conserva su módulo vivo.
fn llamar_directa(
    generador: &mut FunctionBuilder,
    variables: &VariablesSsa,
    profundidad: &mut usize,
    cantidad: usize,
    direccion_salida: Value,
    puntero_tipo: cranelift_codegen::ir::Type,
    convencion: CallConv,
) {
    let mut argumentos = Vec::with_capacity(cantidad);
    for _ in 0..cantidad {
        argumentos.push(pila_sacar(generador, variables, profundidad));
    }
    argumentos.reverse();
    let objetivo = pila_sacar(generador, variables, profundidad);
    let slot = generador.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        (cantidad.max(1) * 8) as u32,
        3,
    ));
    for (indice, argumento) in argumentos.into_iter().enumerate() {
        generador
            .ins()
            .stack_store(puntero_tipo, argumento, slot, (indice * 8) as i32);
    }
    let direccion_argumentos = generador.ins().stack_addr(puntero_tipo, slot, 0);
    let cantidad_nativa = generador.ins().iconst(types::I64, cantidad as i64);
    let mut firma = cranelift_codegen::ir::Signature::new(convencion);
    firma.params.push(AbiParam::new(puntero_tipo));
    firma.params.push(AbiParam::new(types::I64));
    firma.params.push(AbiParam::new(puntero_tipo));
    let firma = generador.import_signature(firma);
    generador.ins().call_indirect(
        firma,
        objetivo,
        &[direccion_argumentos, cantidad_nativa, direccion_salida],
    );
    let estado = generador
        .ins()
        .load(types::I32, MemFlagsData::new(), direccion_salida, 24);
    let exito = generador.ins().icmp_imm_s(IntCC::Equal, estado, 0);
    ramificar_retorno_error(generador, exito);
    let valor = generador
        .ins()
        .load(types::I64, MemFlagsData::new(), direccion_salida, 0);
    pila_guardar(generador, variables, profundidad, valor);
}

fn ramificar_retorno_error(generador: &mut FunctionBuilder, exito: Value) {
    let continuar = generador.create_block();
    let error = generador.create_block();
    generador.ins().brif(exito, continuar, &[], error, &[]);
    generador.switch_to_block(error);
    generador.ins().return_(&[]);
    generador.switch_to_block(continuar);
    generador.seal_block(error);
    generador.seal_block(continuar);
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
    id_funcion: u32,
) -> Value {
    match op {
        CodigoOp::SumarEntero | CodigoOp::RestarEntero | CodigoOp::MultiplicarEntero => {
            operacion_overflow(generador, op, izquierda, derecha, salida, pc, id_funcion)
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
                id_funcion,
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
                id_funcion,
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
    id_funcion: u32,
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
        id_funcion,
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
    id_funcion: u32,
) {
    let error = generador.create_block();
    let continuar = generador.create_block();
    generador.ins().brif(condicion, error, &[], continuar, &[]);
    generador.switch_to_block(error);
    let cero = generador.ins().iconst(types::I64, 0);
    emitir_salida(
        generador,
        direccion_salida,
        cero,
        estado,
        pc,
        Some(detalle),
        id_funcion,
    );
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
    id_funcion: u32,
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
    let id_funcion = generador.ins().iconst(types::I32, id_funcion as i64);
    generador
        .ins()
        .store(MemFlagsData::new(), id_funcion, direccion_salida, 36);
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
