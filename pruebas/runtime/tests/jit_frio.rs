#![cfg(feature = "jit")]

use std::rc::Rc;

use indexmap::IndexMap;
use maquina_virtual::Vm;
use nucleo::Fuente;

#[test]
fn jit_no_deberia_compilar_funciones_con_bucles_al_cargar() {
    let fuente = Fuente::nueva(
        "frio.qz",
        "entero sumar(entero limite) {\n    entero var suma = 0\n    para (entero var i = 0; i < limite; i++) { suma += i }\n    retornar suma\n}\n",
    );
    let ast = sintaxis::parsear_modulo(&fuente).expect("debe parsear");
    semantica::analizar_modulo(&ast).expect("debe pasar semántica");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar");
    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro())).con_jit(true);
    let (entorno, _) = vm
        .cargar_modulo(Rc::new(modulo), IndexMap::new())
        .expect("debe cargar sin compilar");

    assert_eq!(vm.estadisticas_jit().funciones_compiladas, 0);
    let runtimes = entorno.runtimes.borrow();
    let codigo = runtimes.get("sumar").expect("runtime de sumar");
    assert!(matches!(
        &*codigo.jit.borrow(),
        maquina_virtual::codigo::EstadoJit::Frio
    ));
}
