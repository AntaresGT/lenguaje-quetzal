//! Pruebas diferenciales: el intérprete puro y el modo optimizado
//! (especialización + caches) deben producir el mismo estado.

use std::rc::Rc;

use indexmap::IndexMap;
use maquina_virtual::{Valor, Vm};
use nucleo::Fuente;

fn intentar_entorno(
    codigo: &str,
    optimizar: bool,
) -> Result<Rc<maquina_virtual::valores::EntornoModulo>, nucleo::ErrorQuetzal> {
    let fuente = Fuente::nueva("diferencial.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("debe parsear");
    semantica::analizar_modulo(&ast).expect("debe pasar semántica");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar bytecode");
    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro()));
    if !optimizar {
        vm = vm.sin_optimizar();
    }
    vm.cargar_modulo(Rc::new(modulo), IndexMap::new())
        .map(|(entorno, _)| entorno)
}

fn entorno(codigo: &str, optimizar: bool) -> Rc<maquina_virtual::valores::EntornoModulo> {
    intentar_entorno(codigo, optimizar).expect("debe ejecutar")
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
    let con = entorno(codigo, true);
    let sin = entorno(codigo, false);
    for nombre in nombres {
        let a = global(&con, nombre);
        let b = global(&sin, nombre);
        assert!(
            a.es_igual(&b),
            "global '{nombre}' difiere: optimizar={a:?} puro={b:?}"
        );
    }
}

fn mismos_errores(codigo: &str) {
    let con = match intentar_entorno(codigo, true) {
        Ok(_) => panic!("el optimizado debe fallar"),
        Err(error) => error,
    };
    let sin = match intentar_entorno(codigo, false) {
        Ok(_) => panic!("el intérprete puro debe fallar"),
        Err(error) => error,
    };
    assert_eq!(con.codigo, sin.codigo);
    assert_eq!(con.categoria, sin.categoria);
    assert_eq!(con.mensaje, sin.mensaje);
}

#[test]
fn diferencial_deberia_coincidir_en_aritmetica_y_bucles() {
    mismos_globales(
        "entero var suma = 0\npara (entero var i = 0; i < 200; i++) {\n    suma += i * 2\n}\n",
        &["suma"],
    );
}

#[test]
fn diferencial_deberia_coincidir_en_fibonacci() {
    mismos_globales(
        "entero fib(entero n) {\n    si (n < 2) { retornar n }\n    retornar fib(n - 1) + fib(n - 2)\n}\nentero resultado = fib(12)\n",
        &["resultado"],
    );
}

#[test]
fn diferencial_deberia_coincidir_en_objetos() {
    mismos_globales(
        "objeto Caja {\n    entero var n = 1\n    entero ver() { retornar esto.n }\n}\nCaja var c = nuevo Caja()\nc.n = 9\nentero resultado = c.ver()\n",
        &["resultado"],
    );
}

#[test]
fn diferencial_deberia_coincidir_en_listas_y_textos() {
    mismos_globales(
        "lista<entero> nums = [1, 2, 3]\nentero largo = nums.longitud()\ntexto t = \"Hola\".mayusculas()\n",
        &["largo", "t"],
    );
}

#[test]
fn diferencial_atributo_deberia_validar_la_clase_tras_especializar() {
    mismos_globales(
        r#"
objeto A {
    publico:
        entero var valor = 11
}
objeto B {
    publico:
        entero var otro = 22
        entero var valor = 33
}
jsn var actual = { objeto: nuevo A() }
entero var resultado = 0
para (entero var i = 0; i < 1002; i++) {
    si (i == 1001) { actual.objeto = nuevo B() }
    resultado = actual.objeto.valor
}
"#,
        &["resultado"],
    );
}

#[test]
fn diferencial_binaria_deberia_volver_al_camino_generico() {
    mismos_globales(
        r#"
jsn var valores = { a: 1, b: 2 }
número var resultado = 0
para (entero var i = 0; i < 1002; i++) {
    si (i == 1001) { valores.a = 0.5 }
    resultado = valores.a + valores.b
}
"#,
        &["resultado"],
    );
}

#[test]
fn diferencial_nativa_deberia_validar_el_receptor_tras_especializar() {
    mismos_globales(
        r#"
jsn var actual = { valor: "a" }
entero var resultado = 0
para (entero var i = 0; i < 1002; i++) {
    si (i == 1001) { actual.valor = [1, 2, 3] }
    resultado = actual.valor.longitud()
}
"#,
        &["resultado"],
    );
}

#[test]
fn diferencial_atributo_deberia_soportar_alternancia_polimorfica() {
    mismos_globales(
        r#"
objeto A {
    publico:
        entero var valor = 11
}
objeto B {
    publico:
        entero var otro = 22
        entero var valor = 33
}
jsn var actual = { objeto: nuevo A() }
entero var resultado = 0
para (entero var i = 0; i < 1001; i++) {
    si (i % 2 == 0) {
        actual.objeto = nuevo A()
    } sino {
        actual.objeto = nuevo B()
    }
    resultado = actual.objeto.valor
}
"#,
        &["resultado"],
    );
}

#[test]
fn diferencial_especializacion_deberia_respetar_los_umbrales() {
    for cantidad in [1, 10, 999, 1000, 1001] {
        let codigo = format!(
            r#"
jsn var valores = {{ a: 1, b: 2 }}
número var resultado = 0
para (entero var i = 0; i < {cantidad}; i++) {{
    si (i == {cantidad} - 1) {{ valores.a = 0.5 }}
    resultado = valores.a + valores.b
}}
"#
        );
        mismos_globales(&codigo, &["resultado"]);
    }
}

#[test]
fn diferencial_error_deberia_conservar_la_operacion_generica() {
    mismos_errores(
        r#"
jsn var valores = { a: 1, b: 2 }
entero var resultado = 0
para (entero var i = 0; i < 1001; i++) {
    si (i == 1000) { valores.a = verdadero }
    resultado = valores.a - valores.b
}
"#,
    );
}

#[test]
fn diferencial_atributo_constante_no_deberia_mutarse_por_camino_dinamico() {
    mismos_errores(
        r#"
objeto Caja {
    publico:
        entero valor = 1
}
jsn var actual = { objeto: nuevo Caja() }
actual.objeto.valor = 2
"#,
    );
}

#[test]
fn diferencial_atributo_libre_constante_no_deberia_mutarse() {
    mismos_errores(
        r#"
objeto Configuracion {
    publico:
        libre entero valor = 1
}
jsn var actual = { objeto: Configuracion }
actual.objeto.valor = 2
"#,
    );
}
