//! Benchmarks de la VM. Se ignoran en CI: el tiempo varía con el CPU.
//!
//! Correr en release:
//! `cargo test -p pruebas_runtime --release -- --ignored --nocapture bench_`

use std::rc::Rc;
use std::time::Instant;

use indexmap::IndexMap;
use maquina_virtual::{Valor, Vm};
use nucleo::Fuente;

fn ejecutar(codigo: &str) -> Valor {
    let fuente = Fuente::nueva("bench.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("el código de bench debe parsear");
    semantica::analizar_modulo(&ast).expect("el código de bench debe pasar la semántica");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar bytecode");
    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro()));
    let (entorno, _valor) = vm
        .cargar_modulo(Rc::new(modulo), IndexMap::new())
        .expect("el bench debe ejecutar sin errores");
    entorno
        .globales
        .borrow()
        .get("resultado")
        .unwrap_or_else(|| panic!("debe existir la global 'resultado'"))
        .valor
        .clone()
}

fn entero(valor: &Valor) -> i64 {
    match valor {
        Valor::Entero(entero) => *entero,
        otro => panic!("se esperaba un entero, se obtuvo {otro:?}"),
    }
}

#[test]
#[ignore = "benchmark: cargo test -p pruebas_runtime --release -- --ignored --nocapture bench_"]
fn bench_bucle_enteros() {
    let codigo = r#"
entero sumar_hasta(entero limite) {
    entero var suma = 0
    para (entero var i = 0; i < limite; i++) {
        suma += i
    }
    retornar suma
}

entero resultado = sumar_hasta(2000000)
"#;
    let inicio = Instant::now();
    let resultado = ejecutar(codigo);
    let duracion = inicio.elapsed();
    let suma = entero(&resultado);
    println!("bench_bucle_enteros resultado={suma} tiempo={duracion:?}");
    assert_eq!(suma, 1_999_999_000_000i64);
}

#[test]
#[ignore = "benchmark: cargo test -p pruebas_runtime --release -- --ignored --nocapture bench_"]
fn bench_metodos_y_atributos() {
    let codigo = r#"
objeto Zoo {
    entero var aardvark = 1
    entero var baboon = 1
    entero var cat = 1
    entero var donkey = 1
    entero var elephant = 1
    entero var fox = 1

    entero ant() { retornar esto.aardvark }
    entero banana() { retornar esto.baboon }
    entero tuna() { retornar esto.cat }
    entero hay() { retornar esto.donkey }
    entero grass() { retornar esto.elephant }
    entero mouse() { retornar esto.fox }
}

entero recorrer(entero lotes) {
    Zoo var zoo = nuevo Zoo()
    entero var suma = 0
    entero var n = 0
    mientras (n < lotes) {
        suma = suma + zoo.ant() + zoo.banana() + zoo.tuna() + zoo.hay() + zoo.grass() + zoo.mouse()
        n++
    }
    retornar suma
}

entero resultado = recorrer(200000)
"#;
    let inicio = Instant::now();
    let resultado = ejecutar(codigo);
    let duracion = inicio.elapsed();
    let suma = entero(&resultado);
    println!("bench_metodos_y_atributos resultado={suma} tiempo={duracion:?}");
    assert_eq!(suma, 200_000 * 6);
}
