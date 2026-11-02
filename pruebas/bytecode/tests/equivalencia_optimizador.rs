use std::rc::Rc;

use indexmap::IndexMap;
use maquina_virtual::{Valor, Vm};
use nucleo::{ErrorQuetzal, Fuente};

fn ejecutar(
    codigo: &str,
    optimizar: bool,
) -> Result<Rc<maquina_virtual::valores::EntornoModulo>, ErrorQuetzal> {
    let fuente = Fuente::nueva("equivalencia.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("debe parsear");
    semantica::analizar_modulo(&ast).expect("debe pasar semántica");
    let mut modulo = bytecode::generar_modulo_sin_procesar(&ast).expect("debe generar");
    if optimizar {
        bytecode::optimizar_modulo(&mut modulo);
    }
    bytecode::verificar_modulo(&mut modulo).expect("el bytecode debe verificar");
    Vm::nueva(Rc::new(modulos_nativos::crear_registro()))
        .sin_optimizar()
        .sin_despacho_rapido()
        .cargar_modulo(Rc::new(modulo), IndexMap::new())
        .map(|(entorno, _)| entorno)
}

fn global(entorno: &maquina_virtual::valores::EntornoModulo, nombre: &str) -> Valor {
    entorno
        .globales
        .borrow()
        .get(nombre)
        .unwrap_or_else(|| panic!("debe existir '{nombre}'"))
        .valor
        .clone()
}

fn mismos_globales(codigo: &str, nombres: &[&str]) {
    let original = ejecutar(codigo, false).expect("original ejecutable");
    let optimizado = ejecutar(codigo, true).expect("optimizado ejecutable");
    for nombre in nombres {
        let esperado = global(&original, nombre);
        let actual = global(&optimizado, nombre);
        assert!(actual.es_igual(&esperado), "divergencia en '{nombre}'");
    }
}

fn mismos_errores(codigo: &str) {
    let original = ejecutar(codigo, false).expect_err("original debe fallar");
    let optimizado = ejecutar(codigo, true).expect_err("optimizado debe fallar");
    assert_eq!(optimizado.codigo, original.codigo);
    assert_eq!(optimizado.categoria, original.categoria);
    assert_eq!(optimizado.mensaje, original.mensaje);
    assert_eq!(optimizado.ubicacion, original.ubicacion);
}

#[test]
fn plegado_deberia_conservar_enteros_y_fronteras() {
    mismos_globales(
        "entero pequeno = 40 + 2\nentero grande = 9223372036854775807 - 1\nentero negativo = -9223372036854775807 - 1\n",
        &["pequeno", "grande", "negativo"],
    );
}

#[test]
fn plegado_no_deberia_eliminar_errores_de_ejecucion() {
    mismos_errores("entero resultado = 10 / 0\n");
    mismos_errores("entero resultado = 9223372036854775807 + 1\n");
    mismos_errores("entero resultado = (-9223372036854775807 - 1) / -1\n");
}

#[test]
fn superinstrucciones_deberian_conservar_locales_y_saltos() {
    mismos_globales(
        r#"
entero calcular(entero limite) {
    entero var suma = 0
    para (entero var i = 0; i < limite; i++) {
        si (i % 2 == 0) { suma += i } sino { suma -= 1 }
    }
    retornar suma
}
entero resultado = calcular(101)
"#,
        &["resultado"],
    );
}

#[test]
fn optimizador_deberia_conservar_cortocircuitos_y_efectos() {
    mismos_globales(
        r#"
entero var llamadas = 0
log efecto(log valor) {
    llamadas++
    retornar valor
}
log primero = falso y efecto(verdadero)
log segundo = verdadero o efecto(falso)
log tercero = verdadero y efecto(verdadero)
"#,
        &["llamadas", "primero", "segundo", "tercero"],
    );
}

#[test]
fn optimizador_deberia_conservar_retorno_y_recursion() {
    mismos_globales(
        r#"
entero fib(entero n) {
    si (n < 2) { retornar n }
    retornar fib(n - 1) + fib(n - 2)
}
entero resultado = fib(12)
"#,
        &["resultado"],
    );
}

#[test]
fn optimizador_deberia_conservar_atributos_y_excepciones() {
    mismos_globales(
        r#"
objeto Caja { publico: entero var valor = 41 }
texto var mensaje = ""
vacio ejecutar() {
    Caja var caja = nuevo Caja()
    intentar {
        caja.valor++
        lanzar "esperado"
    } capturar (excepcion e) {
        mensaje = e.mensaje
    } finalmente {
        caja.valor++
    }
    entero resultado = caja.valor
    mensaje += ":" + resultado.texto()
}
ejecutar()
"#,
        &["mensaje"],
    );
}
