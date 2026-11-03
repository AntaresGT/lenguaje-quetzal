//! Interrupción cooperativa de la ejecución (Ctrl+C): un bucle infinito
//! debe detenerse con el error E0410 cuando la bandera está activa.

use std::rc::Rc;
use std::sync::{Mutex, MutexGuard, OnceLock};

use indexmap::IndexMap;
use maquina_virtual::{Valor, Vm, interrupcion};
use nucleo::Fuente;

/// La bandera de interrupción es global al proceso; los tests deben correr
/// uno a la vez para no contaminarse entre sí.
fn ejecucion_exclusiva() -> MutexGuard<'static, ()> {
    static MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    let mutex = MUTEX.get_or_init(|| Mutex::new(()));
    let guard = mutex.lock().unwrap_or_else(|error| error.into_inner());
    interrupcion::reiniciar();
    guard
}

fn preparar(codigo: &str) -> (Vm, bytecode::ModuloCompilado) {
    let fuente = Fuente::nueva("interrupcion.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("el código debe parsear");
    semantica::analizar_modulo(&ast).expect("el código debe pasar la semántica");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar bytecode");
    (
        Vm::nueva(Rc::new(modulos_nativos::crear_registro())),
        modulo,
    )
}

/// Corre un programa con la bandera de interrupción ya activa, como si
/// Ctrl+C hubiera llegado justo al comenzar.
fn ejecutar_interrumpido(codigo: &str) -> String {
    let (mut vm, modulo) = preparar(codigo);
    interrupcion::interrumpir();
    let resultado = vm.cargar_modulo(Rc::new(modulo), IndexMap::new());
    interrupcion::reiniciar();
    resultado
        .expect_err("la ejecución interrumpida debe fallar")
        .to_string()
}

#[test]
fn bucle_infinito_se_detiene_con_e0410() {
    let _exclusivo = ejecucion_exclusiva();
    let error = ejecutar_interrumpido("mientras (verdadero) { }");
    assert!(
        error.contains("E0410"),
        "el error debe reportar el código E0410: {error}"
    );
    assert!(
        error.contains("interrumpida"),
        "el mensaje debe explicar la interrupción: {error}"
    );
}

#[test]
fn un_programa_simple_corre_sin_la_bandera() {
    let _exclusivo = ejecucion_exclusiva();
    let (mut vm, modulo) = preparar("entero resultado = 7 * 6");
    let (entorno, valor) = vm
        .cargar_modulo(Rc::new(modulo), IndexMap::new())
        .expect("el programa simple debe correr sin interrupción");
    assert!(matches!(valor, Valor::Nulo));
    let resultado = entorno
        .globales
        .borrow()
        .get("resultado")
        .expect("debe existir la global 'resultado'")
        .valor
        .clone();
    match resultado {
        Valor::Entero(entero) => assert_eq!(entero, 42),
        otro => panic!("se esperaba el entero 42, se obtuvo {otro:?}"),
    }
    interrupcion::reiniciar();
}
