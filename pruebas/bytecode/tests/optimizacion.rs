//! El pase de peephole corre al final de `generar_modulo`.

use nucleo::Fuente;

fn generar(codigo: &str) -> bytecode::ModuloCompilado {
    let fuente = Fuente::nueva("prueba.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("el código de prueba debe parsear");
    bytecode::generar_modulo(&ast).expect("el código de prueba debe generar bytecode")
}

fn instrucciones_funcion(
    modulo: &bytecode::ModuloCompilado,
    nombre: &str,
) -> Vec<bytecode::Instruccion> {
    modulo
        .funciones
        .iter()
        .find(|funcion| funcion.nombre == nombre)
        .unwrap_or_else(|| panic!("debe existir la función '{nombre}'"))
        .trozo
        .instrucciones
        .clone()
}

#[test]
fn generar_deberia_emitir_cargar_entero_para_literales_pequenos() {
    let modulo = generar("entero n = 7\n");
    assert!(
        modulo
            .principal
            .instrucciones
            .contains(&bytecode::Instruccion::CargarEntero(7))
    );
}

#[test]
fn generar_deberia_plegar_aritmetica_entera_constante() {
    let modulo = generar("entero n = 10 * 4 + 2\n");
    assert!(
        modulo
            .principal
            .instrucciones
            .contains(&bytecode::Instruccion::CargarEntero(42))
    );
    assert!(
        !modulo
            .principal
            .instrucciones
            .iter()
            .any(|instruccion| matches!(
                instruccion.op,
                bytecode::CodigoOp::SumarEntero | bytecode::CodigoOp::MultiplicarEntero
            ))
    );
}

#[test]
fn generar_deberia_fusionar_incremento_local_en_ajustar() {
    let modulo = generar("entero contar() {\n    entero var i = 0\n    i++\n    retornar i\n}\n");
    let instrucciones = instrucciones_funcion(&modulo, "contar");
    assert!(
        instrucciones.iter().any(|instruccion| {
            instruccion.op == bytecode::CodigoOp::AjustarLocal && instruccion.delta_ajuste() == 1
        }),
        "i++ sobre un local debe fusionarse a AjustarLocal: {instrucciones:?}"
    );
}

#[test]
fn generar_no_deberia_plegar_division_por_cero() {
    let modulo = generar("entero n = 10 / 0\n");
    assert!(
        modulo
            .principal
            .instrucciones
            .contains(&bytecode::Instruccion::DividirEntero())
    );
}

#[test]
fn generar_sin_procesar_deberia_permitir_medir_cada_fase_una_vez() {
    let fuente = Fuente::nueva(
        "prueba.qz",
        "entero contar() {\n    entero var i = 0\n    i++\n    retornar i\n}\n",
    );
    let ast = sintaxis::parsear_modulo(&fuente).expect("el código debe parsear");
    let crudo = bytecode::generar_modulo_sin_procesar(&ast).expect("debe generar bytecode crudo");
    let mut procesado = crudo.clone();
    bytecode::optimizar_modulo(&mut procesado);
    bytecode::verificar_modulo(&mut procesado).expect("el bytecode debe ser válido");
    assert_ne!(crudo, procesado);
    assert_eq!(
        bytecode::generar_modulo(&ast).expect("debe ejecutar el flujo completo"),
        procesado
    );
}
