use std::rc::Rc;

use bytecode::{Instruccion, ModuloCompilado, Trozo};
use indexmap::IndexMap;
use maquina_virtual::Vm;
use nucleo::Ubicacion;

#[test]
fn vm_deberia_verificar_un_modulo_externo_antes_de_ejecutarlo() {
    let modulo = ModuloCompilado {
        nombre: "corrupto.qz".to_string(),
        ranuras_locales_principal: 0,
        principal: Trozo {
            instrucciones: vec![Instruccion::CargarLocal(0), Instruccion::Retornar()],
            ubicaciones: vec![Ubicacion::default(); 2],
            max_pila: 0,
        },
        ..ModuloCompilado::default()
    };
    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro()));
    let error = vm
        .cargar_modulo(Rc::new(modulo), IndexMap::new())
        .expect_err("el bytecode inválido no debe ejecutarse");
    assert_eq!(error.codigo, "E0410");
}
