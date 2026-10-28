//! Pruebas diferenciales: el intérprete puro y el modo optimizado
//! (especialización + cierres) deben producir el mismo estado.

use std::rc::Rc;

use indexmap::IndexMap;
use maquina_virtual::{Valor, Vm};
use nucleo::Fuente;

fn entorno(codigo: &str, optimizar: bool) -> Rc<maquina_virtual::valores::EntornoModulo> {
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
        .expect("debe ejecutar")
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
