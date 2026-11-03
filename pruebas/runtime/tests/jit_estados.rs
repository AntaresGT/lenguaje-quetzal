#![cfg(feature = "jit")]

use std::rc::Rc;

use compilador_jit::ConfiguracionJit;
use indexmap::IndexMap;
use maquina_virtual::codigo::EstadoJit;
use maquina_virtual::{Valor, Vm};
use nucleo::Fuente;

fn modulo(codigo: &str) -> bytecode::ModuloCompilado {
    let fuente = Fuente::nueva("estados_jit.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("debe parsear");
    semantica::analizar_modulo(&ast).expect("debe pasar semántica");
    bytecode::generar_modulo(&ast).expect("debe generar")
}

fn entorno(vm: &mut Vm, codigo: &str) -> Rc<maquina_virtual::valores::EntornoModulo> {
    vm.cargar_modulo(Rc::new(modulo(codigo)), IndexMap::new())
        .expect("módulo ejecutable")
        .0
}

#[test]
fn jit_deberia_distinguir_presupuesto_excedido_de_no_elegible() {
    let codigo = r#"
entero identidad(entero n) { retornar n }
entero a = identidad(1)
entero b = identidad(2)
entero c = identidad(3)
entero d = identidad(4)
entero resultado = identidad(5)
"#;
    let configuracion = ConfiguracionJit {
        max_funciones: 0,
        metricas: true,
        ..ConfiguracionJit::default()
    };
    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro()))
        .con_configuracion_jit(configuracion, true);
    let entorno = entorno(&mut vm, codigo);
    let resultado = entorno.globales.borrow()["resultado"].valor.clone();
    assert!(resultado.es_igual(&Valor::Entero(5)));
    assert_eq!(vm.estadisticas_jit().funciones_compiladas, 0);
    assert_eq!(vm.estadisticas_jit().compilaciones_descartadas, 1);
    let runtimes = entorno.runtimes.borrow();
    assert!(matches!(
        &*runtimes["identidad"].jit.borrow(),
        EstadoJit::NoDisponible
    ));
}

#[test]
fn jit_deberia_contar_guardas_y_ejecuciones_solo_con_metricas() {
    let configuracion = ConfiguracionJit {
        metricas: true,
        ..ConfiguracionJit::default()
    };
    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro()))
        .con_configuracion_jit(configuracion, true);
    let entorno = entorno(&mut vm, "entero identidad(entero n) { retornar n }\n");
    let funcion = match &entorno.globales.borrow()["identidad"].valor {
        Valor::Funcion(datos) => Rc::clone(&datos.funcion),
        otro => panic!("se esperaba función: {otro:?}"),
    };
    for _ in 0..4 {
        vm.llamar_funcion(&funcion, &entorno, vec![Valor::Entero(1)], None, None)
            .expect("calentamiento nativo");
    }
    let resultado = vm
        .llamar_funcion(&funcion, &entorno, vec![Valor::texto("x")], None, None)
        .expect("guarda interpretada");
    assert!(resultado.es_igual(&Valor::texto("x")));
    let estadisticas = vm.estadisticas_jit();
    assert_eq!(estadisticas.funciones_elegibles, 1);
    assert_eq!(estadisticas.ejecuciones_nativas, 1);
    assert_eq!(estadisticas.guardas_fallidas, 1);
}
