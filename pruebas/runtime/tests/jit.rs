#![cfg(feature = "jit")]

use std::rc::Rc;

use indexmap::IndexMap;
use maquina_virtual::{Valor, Vm};
use nucleo::Fuente;

fn compilar(codigo: &str) -> bytecode::ModuloCompilado {
    let fuente = Fuente::nueva("jit.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("debe parsear");
    semantica::analizar_modulo(&ast).expect("debe pasar semántica");
    bytecode::generar_modulo(&ast).expect("debe generar bytecode")
}

fn ejecutar(
    codigo: &str,
    jit: bool,
) -> Result<Rc<maquina_virtual::valores::EntornoModulo>, nucleo::ErrorQuetzal> {
    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro())).con_jit(jit);
    vm.cargar_modulo(Rc::new(compilar(codigo)), IndexMap::new())
        .map(|(entorno, _)| entorno)
}

fn global(entorno: &maquina_virtual::valores::EntornoModulo, nombre: &str) -> Valor {
    entorno
        .globales
        .borrow()
        .get(nombre)
        .expect("global presente")
        .valor
        .clone()
}

#[test]
fn jit_deberia_ejecutar_un_bucle_caliente_desde_quetzal() {
    let codigo = r#"
entero sumar_hasta(entero limite) {
    entero var suma = 0
    para (entero var i = 0; i < limite; i++) {
        suma += i
    }
    retornar suma
}
entero a = sumar_hasta(1)
entero b = sumar_hasta(2)
entero c = sumar_hasta(3)
entero d = sumar_hasta(4)
entero resultado = sumar_hasta(10000)
"#;
    let entorno = ejecutar(codigo, true).expect("programa con JIT");
    assert!(global(&entorno, "resultado").es_igual(&Valor::Entero(49_995_000)));
}

#[test]
fn jit_deberia_conservar_division_por_cero() {
    let codigo = r#"
entero dividir(entero a, entero b) {
    retornar a / b
}
entero a = dividir(10, 2)
entero b = dividir(10, 2)
entero c = dividir(10, 2)
entero d = dividir(10, 2)
entero resultado = dividir(1, 0)
"#;
    let error = ejecutar(codigo, true).expect_err("debe fallar");
    assert_eq!(error.codigo, "E0401");
    assert!(error.ubicacion.is_some());
}

#[test]
fn jit_deberia_conservar_overflow() {
    let codigo = r#"
entero sumar(entero a, entero b) {
    retornar a + b
}
entero a = sumar(1, 2)
entero b = sumar(2, 3)
entero c = sumar(3, 4)
entero d = sumar(4, 5)
entero resultado = sumar(9223372036854775807, 1)
"#;
    let error = ejecutar(codigo, true).expect_err("debe fallar");
    assert_eq!(error.codigo, "E0402");
    assert!(error.ubicacion.is_some());
}

#[test]
fn jit_deberia_mantener_fallback_recursivo() {
    let codigo = r#"
entero fib(entero n) {
    si (n < 2) { retornar n }
    retornar fib(n - 1) + fib(n - 2)
}
entero resultado = fib(20)
"#;
    let entorno = ejecutar(codigo, true).expect("recursión interpretada");
    assert!(global(&entorno, "resultado").es_igual(&Valor::Entero(6765)));
}

#[test]
fn jit_deberia_estar_desactivado_por_defecto() {
    let codigo = r#"
entero identidad(entero n) { retornar n }
entero a = identidad(1)
entero b = identidad(2)
entero c = identidad(3)
entero d = identidad(4)
entero resultado = identidad(5)
"#;
    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro()));
    let (entorno, _) = vm
        .cargar_modulo(Rc::new(compilar(codigo)), IndexMap::new())
        .expect("módulo interpretado");
    assert!(global(&entorno, "resultado").es_igual(&Valor::Entero(5)));
    assert_eq!(vm.estadisticas_jit().funciones_compiladas, 0);
}
