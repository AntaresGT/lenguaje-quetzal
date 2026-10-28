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

#[test]
#[ignore = "benchmark: cargo test -p pruebas_runtime --release -- --ignored --nocapture bench_"]
fn bench_fibonacci_recursivo() {
    let codigo = r#"
entero fib(entero n) {
    si (n < 2) {
        retornar n
    }
    retornar fib(n - 1) + fib(n - 2)
}

entero resultado = fib(30)
"#;
    let inicio = Instant::now();
    let resultado = ejecutar(codigo);
    let duracion = inicio.elapsed();
    let valor = entero(&resultado);
    println!("bench_fibonacci_recursivo resultado={valor} tiempo={duracion:?}");
    assert_eq!(valor, 832_040);
}

#[test]
#[ignore = "benchmark: cargo test -p pruebas_runtime --release -- --ignored --nocapture bench_"]
fn bench_nativos_texto_lista() {
    let codigo = r#"
entero recorrer(entero lotes) {
    entero var suma = 0
    para (entero var i = 0; i < lotes; i++) {
        texto t = "Hola Mundo"
        suma += t.longitud()
        lista<entero> nums = [1, 2, 3]
        suma += nums.longitud()
    }
    retornar suma
}

entero resultado = recorrer(50000)
"#;
    let inicio = Instant::now();
    let resultado = ejecutar(codigo);
    let duracion = inicio.elapsed();
    let valor = entero(&resultado);
    println!("bench_nativos_texto_lista resultado={valor} tiempo={duracion:?}");
    assert_eq!(valor, 50_000 * (10 + 3));
}

#[test]
#[ignore = "benchmark: cargo test -p pruebas_runtime --release -- --ignored --nocapture bench_"]
fn bench_jsn_lectura_escritura() {
    let codigo = r#"
entero recorrer(entero lotes) {
    jsn var persona = { nombre: "Ana", edad: 0 }
    para (entero var i = 0; i < lotes; i++) {
        persona.edad = i
        entero n = persona["edad"]
    }
    retornar persona.edad
}

entero resultado = recorrer(80000)
"#;
    let inicio = Instant::now();
    let resultado = ejecutar(codigo);
    let duracion = inicio.elapsed();
    let valor = entero(&resultado);
    println!("bench_jsn_lectura_escritura resultado={valor} tiempo={duracion:?}");
    assert_eq!(valor, 79_999);
}

#[test]
#[ignore = "benchmark: cargo test -p pruebas_runtime --release -- --ignored --nocapture bench_"]
fn bench_concatenacion_e_interpolacion() {
    let codigo = r#"
texto recorrer(entero lotes) {
    texto var acc = ""
    para (entero var i = 0; i < lotes; i++) {
        acc = t"{acc}x"
    }
    retornar acc
}

texto resultado = recorrer(4000)
"#;
    let inicio = Instant::now();
    let resultado = ejecutar(codigo);
    let duracion = inicio.elapsed();
    match resultado {
        Valor::Texto(texto) => {
            println!(
                "bench_concatenacion_e_interpolacion longitud={} tiempo={duracion:?}",
                texto.len()
            );
            assert_eq!(texto.len(), 4000);
        }
        otro => panic!("se esperaba texto, se obtuvo {otro:?}"),
    }
}

#[test]
#[ignore = "benchmark: cargo test -p pruebas_runtime --release -- --ignored --nocapture bench_"]
fn bench_instanciar_objetos() {
    let codigo = r#"
objeto Punto {
    entero var x = 0

    entero poner(entero n) {
        esto.x = n
        retornar esto.x
    }
}

entero recorrer(entero lotes) {
    entero var suma = 0
    para (entero var i = 0; i < lotes; i++) {
        Punto var p = nuevo Punto()
        suma += p.poner(i)
    }
    retornar suma
}

entero resultado = recorrer(100000)
"#;
    let inicio = Instant::now();
    let resultado = ejecutar(codigo);
    let duracion = inicio.elapsed();
    let valor = entero(&resultado);
    println!("bench_instanciar_objetos resultado={valor} tiempo={duracion:?}");
    assert_eq!(valor, 4_999_950_000i64);
}

#[test]
#[ignore = "benchmark: cargo test -p pruebas_runtime --release -- --ignored --nocapture bench_"]
fn bench_arranque_hola_mundo() {
    let codigo = "entero resultado = 1\n";
    let inicio = Instant::now();
    let resultado = ejecutar(codigo);
    let duracion = inicio.elapsed();
    println!("bench_arranque_hola_mundo tiempo={duracion:?}");
    assert_eq!(entero(&resultado), 1);
}
