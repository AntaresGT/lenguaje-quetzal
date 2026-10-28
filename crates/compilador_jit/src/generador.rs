use cranelift_codegen::ir::{AbiParam, InstBuilder, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

use crate::{ErrorJit, FuncionNativa};

pub(crate) fn compilar_trivial(valor: i64) -> Result<(FuncionNativa, usize), ErrorJit> {
    let constructor = JITBuilder::new(default_libcall_names())
        .map_err(|error| ErrorJit::Compilacion(error.to_string()))?;
    let mut modulo = JITModule::new(constructor);
    let mut contexto = modulo.make_context();
    let configuracion_objetivo = modulo.target_config();
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
        .returns
        .push(AbiParam::new(types::I64));
    let id = modulo
        .declare_function("trivial", Linkage::Export, &contexto.func.signature)
        .map_err(|error| ErrorJit::Compilacion(error.to_string()))?;
    let mut constructor_funcion = FunctionBuilderContext::new();
    {
        let mut generador = FunctionBuilder::new(&mut contexto.func, &mut constructor_funcion);
        let bloque = generador.create_block();
        generador.append_block_params_for_function_params(bloque);
        generador.switch_to_block(bloque);
        generador.seal_block(bloque);
        let salida = generador.ins().iconst(types::I64, valor);
        generador.ins().return_(&[salida]);
        generador.finalize(configuracion_objetivo);
    }
    modulo
        .define_function(id, &mut contexto)
        .map_err(|error| ErrorJit::Compilacion(error.to_string()))?;
    let bytes = contexto
        .compiled_code()
        .map(|codigo| codigo.code_buffer().len())
        .unwrap_or(0);
    modulo.clear_context(&mut contexto);
    modulo.finalize_definitions().map_err(|error| {
        ErrorJit::Compilacion(format!("no se pudo finalizar el módulo: {error:?}"))
    })?;
    let puntero = modulo.get_finalized_function(id);
    Ok((
        FuncionNativa {
            _modulo: modulo,
            puntero,
            aridad: 0,
        },
        bytes,
    ))
}
