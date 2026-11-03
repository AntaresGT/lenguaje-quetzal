use bytecode::{Instruccion, ModuloCompilado, Trozo, verificar_modulo};
use nucleo::Ubicacion;

fn modulo_con(instrucciones: Vec<Instruccion>) -> ModuloCompilado {
    ModuloCompilado {
        ranuras_locales_principal: 1,
        principal: Trozo {
            ubicaciones: vec![Ubicacion::nueva(0, 1); instrucciones.len()],
            instrucciones,
            max_pila: 0,
        },
        ..ModuloCompilado::default()
    }
}

fn rechazado(modulo: &mut ModuloCompilado) {
    let error = verificar_modulo(modulo).expect_err("el módulo corrupto debe rechazarse");
    assert_eq!(error.codigo, "E0410");
}

#[test]
fn verificador_deberia_rechazar_limpieza_de_excepcion_fuera_de_rango() {
    let mut modulo = modulo_con(vec![Instruccion::EntrarIntentar(1, 2)]);
    rechazado(&mut modulo);
}

#[test]
fn verificador_deberia_rechazar_salida_de_excepcion_sin_entrada() {
    let mut modulo = modulo_con(vec![Instruccion::SalirIntentar()]);
    rechazado(&mut modulo);
}

#[test]
fn verificador_deberia_calcular_pila_en_bucles() {
    let mut modulo = modulo_con(vec![
        Instruccion::CargarEntero(1),
        Instruccion::CargarEntero(2),
        Instruccion::CargarEntero(3),
        Instruccion::Desechar(),
        Instruccion::CargarEntero(1),
        Instruccion::SaltarSiFalso(9),
        Instruccion::CargarEntero(4),
        Instruccion::Desechar(),
        Instruccion::Saltar(4),
        Instruccion::Retornar(),
    ]);
    verificar_modulo(&mut modulo).expect("flujo válido");
    assert_eq!(modulo.principal.max_pila, 3);
}

#[test]
fn verificador_deberia_ignorar_regiones_inaccesibles_en_max_pila() {
    let mut modulo = modulo_con(vec![
        Instruccion::CargarEntero(1),
        Instruccion::Retornar(),
        Instruccion::CargarEntero(2),
        Instruccion::CargarEntero(3),
        Instruccion::CargarEntero(4),
    ]);
    verificar_modulo(&mut modulo).expect("código inaccesible permitido");
    assert_eq!(modulo.principal.max_pila, 1);
}

#[test]
fn verificador_deberia_rechazar_todos_los_indices_fuera_de_rango() {
    let mut casos = vec![
        modulo_con(vec![Instruccion::CargarConstante(0)]),
        modulo_con(vec![Instruccion::CargarLocal(1)]),
        modulo_con(vec![Instruccion::CargarGlobal(0)]),
        modulo_con(vec![Instruccion::CargarExterno(0)]),
        modulo_con(vec![Instruccion::Saltar(9)]),
    ];
    for modulo in &mut casos {
        rechazado(modulo);
    }
}

#[test]
fn verificador_no_deberia_entrar_en_panico_con_mutaciones_reproducibles() {
    for semilla in 0..512u32 {
        let mut modulo = modulo_con(vec![
            Instruccion::CargarEntero(1),
            Instruccion::SaltarSiFalso(semilla % 9),
            Instruccion::EntrarIntentar(semilla % 9, semilla as u16),
            Instruccion::CrearJsn(semilla),
            Instruccion::LlamarNativa(semilla as u16, semilla as u16),
            Instruccion::SalirIntentar(),
            Instruccion::Retornar(),
        ]);
        let resultado = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            verificar_modulo(&mut modulo)
        }));
        assert!(resultado.is_ok(), "pánico con semilla {semilla}");
    }
}
