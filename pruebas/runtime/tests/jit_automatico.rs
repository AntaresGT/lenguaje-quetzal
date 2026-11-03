#![cfg(feature = "jit")]

use std::rc::Rc;

use indexmap::IndexMap;
use maquina_virtual::codigo::EstadoJit;
use maquina_virtual::{Valor, Vm};
use nucleo::Fuente;

fn cargar(vm: &mut Vm, codigo: &str) -> Rc<maquina_virtual::valores::EntornoModulo> {
    let fuente = Fuente::nueva("automatico.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("debe parsear");
    semantica::analizar_modulo(&ast).expect("debe pasar semántica");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar");
    vm.cargar_modulo(Rc::new(modulo), IndexMap::new())
        .expect("módulo ejecutable")
        .0
}

#[test]
fn automatico_no_deberia_compilar_cinco_llamadas_pequenas() {
    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro())).con_jit_automatico();
    let entorno = cargar(
        &mut vm,
        r#"
entero identidad(entero n) { retornar n }
entero a = identidad(1)
entero b = identidad(2)
entero c = identidad(3)
entero d = identidad(4)
entero resultado = identidad(5)
"#,
    );
    assert_eq!(vm.estadisticas_jit().funciones_compiladas, 0);
    assert!(matches!(
        &*entorno.runtimes.borrow()["identidad"].jit.borrow(),
        EstadoJit::Frio
    ));
}

#[test]
fn automatico_deberia_compilar_tras_mil_retrocesos() {
    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro())).con_jit_automatico();
    let entorno = cargar(
        &mut vm,
        r#"
entero sumar(entero limite) {
    entero var suma = 0
    para (entero var i = 0; i < limite; i++) { suma += i }
    retornar suma
}
entero a = sumar(250)
entero b = sumar(250)
entero c = sumar(250)
entero d = sumar(250)
entero resultado = sumar(10000)
"#,
    );
    let resultado = entorno.globales.borrow()["resultado"].valor.clone();
    assert!(resultado.es_igual(&Valor::Entero(49_995_000)));
    assert_eq!(vm.estadisticas_jit().funciones_compiladas, 1);
}

#[test]
fn automatico_deberia_esperar_mil_veinticuatro_llamadas_sin_bucle() {
    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro())).con_jit_automatico();
    let entorno = cargar(&mut vm, "entero identidad(entero n) { retornar n }\n");
    let funcion = match &entorno.globales.borrow()["identidad"].valor {
        Valor::Funcion(datos) => Rc::clone(&datos.funcion),
        otro => panic!("se esperaba función: {otro:?}"),
    };
    for _ in 0..1023 {
        vm.llamar_funcion(&funcion, &entorno, vec![Valor::Entero(1)], None, None)
            .expect("llamada interpretada");
    }
    assert_eq!(vm.estadisticas_jit().funciones_compiladas, 0);
    vm.llamar_funcion(&funcion, &entorno, vec![Valor::Entero(1)], None, None)
        .expect("llamada nativa");
    assert_eq!(vm.estadisticas_jit().funciones_compiladas, 1);
}

#[test]
fn automatico_deberia_dejar_de_observar_una_funcion_no_elegible() {
    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro())).con_jit_automatico();
    let entorno = cargar(
        &mut vm,
        r#"
entero fib(entero n) {
    si (n < 2) { retornar n }
    retornar fib(n - 1) + fib(n - 2)
}
entero resultado = fib(20)
"#,
    );
    let resultado = entorno.globales.borrow()["resultado"].valor.clone();
    assert!(resultado.es_igual(&Valor::Entero(6765)));
    assert_eq!(vm.estadisticas_jit().fallbacks, 1);
    assert!(matches!(
        &*entorno.runtimes.borrow()["fib"].jit.borrow(),
        EstadoJit::NoElegible
    ));
}
