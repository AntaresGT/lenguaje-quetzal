use std::rc::Rc;

use bytecode::ModuloCompilado;
use indexmap::IndexMap;
use maquina_virtual::valores::{EntornoModulo, Variable};
use maquina_virtual::{Valor, Vm};
use nucleo::Fuente;

fn compilar(nombre: &str, codigo: &str) -> ModuloCompilado {
    let fuente = Fuente::nueva(nombre, codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("debe parsear");
    semantica::analizar_modulo(&ast).expect("debe pasar semántica");
    bytecode::generar_modulo(&ast).expect("debe generar bytecode")
}

fn global(entorno: &Rc<EntornoModulo>, nombre: &str) -> Valor {
    entorno
        .globales
        .borrow()
        .get(nombre)
        .expect("global presente")
        .valor
        .clone()
}

#[test]
fn modulo_deberia_liberarse_al_soltar_sus_referencias() {
    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro()));
    let (entorno, _) = vm
        .cargar_modulo(
            Rc::new(compilar(
                "memoria",
                "entero identidad(entero n) { retornar n }\nobjeto Caja { publico: entero var valor = 1 }\nCaja caja = nuevo Caja()\n",
            )),
            IndexMap::new(),
        )
        .expect("módulo ejecutable");
    let debil = Rc::downgrade(&entorno);
    drop(entorno);
    drop(vm);
    assert!(debil.upgrade().is_none());
}

#[test]
fn funcion_importada_deberia_conservar_y_luego_soltar_su_entorno() {
    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro()));
    let (origen, _) = vm
        .cargar_modulo(
            Rc::new(compilar(
                "origen",
                "entero doble(entero n) { retornar n * 2 }\nexportar { doble }\n",
            )),
            IndexMap::new(),
        )
        .expect("origen ejecutable");
    let debil = Rc::downgrade(&origen);
    let importaciones = IndexMap::from([(
        "doble".to_string(),
        Variable {
            valor: global(&origen, "doble"),
            mutable: false,
        },
    )]);
    let (destino, _) = vm
        .cargar_modulo(
            Rc::new(compilar(
                "destino",
                "importar { doble } desde \"origen.qz\"\nentero resultado = doble(4)\n",
            )),
            importaciones,
        )
        .expect("destino ejecutable");
    vm.modulos.shift_remove("origen");
    drop(origen);
    assert!(debil.upgrade().is_some());

    let importada = global(&destino, "doble");
    drop(destino);
    drop(vm);
    assert!(debil.upgrade().is_some());
    drop(importada);
    assert!(debil.upgrade().is_none());
}

#[test]
fn repl_deberia_conservar_funciones_sin_acumular_fragmentos() {
    let mut sesion = motor::SesionInteractiva::nueva(motor::ConfiguracionMotor::sin_colores())
        .expect("sesión disponible");
    sesion
        .evaluar("entero doble(entero n) { retornar n * 2 }")
        .expect("declaración válida");
    sesion
        .evaluar("entero resultado = doble(4)")
        .expect("la función anterior debe seguir disponible");
    assert_eq!(sesion.modulos().len(), 1);
    assert_eq!(
        sesion.evaluar("resultado").expect("lectura válida"),
        Some("8".to_string())
    );

    sesion.limpiar_estado();
    assert!(sesion.estado().is_empty());
    assert!(sesion.modulos().is_empty());
}
