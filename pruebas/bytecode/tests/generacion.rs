//! El generador debe producir bytecode para todos los ejemplos del lenguaje.

use nucleo::Fuente;

fn generar(codigo: &str) -> bytecode::ModuloCompilado {
    let fuente = Fuente::nueva("prueba.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("el código de prueba debe parsear");
    bytecode::generar_modulo(&ast).expect("el código de prueba debe generar bytecode")
}

#[test]
fn generar_deberia_emitir_constantes_y_llamadas() {
    let modulo = generar("consola.mostrar(1 + 2)\n");
    assert!(!modulo.principal.instrucciones.is_empty());
    assert!(
        modulo
            .principal
            .instrucciones
            .contains(&bytecode::Instruccion::CargarEntero(3)),
        "1 + 2 debe plegarse a la constante 3"
    );
}

#[test]
fn generar_deberia_usar_ranuras_locales_y_globales() {
    let modulo =
        generar("entero var suma = 0\npara (entero var i = 1; i <= 3; i++) {\n    suma += i\n}\n");
    assert!(
        modulo
            .nombres_globales
            .iter()
            .any(|nombre| nombre == "suma")
    );
    assert!(modulo.ranuras_locales_principal >= 1);
    assert!(
        modulo
            .principal
            .instrucciones
            .iter()
            .any(|instruccion| instruccion.op == bytecode::CodigoOp::CargarLocal)
    );
    assert!(
        modulo
            .principal
            .instrucciones
            .iter()
            .any(|instruccion| instruccion.op == bytecode::CodigoOp::CargarGlobal)
    );
}

#[test]
fn generar_deberia_registrar_funciones() {
    let modulo = generar("entero doble(entero n) {\n    retornar n * 2\n}\n");
    assert_eq!(modulo.funciones.len(), 1);
    assert_eq!(modulo.funciones[0].nombre, "doble");
    assert_eq!(modulo.funciones[0].parametros.len(), 1);
}

#[test]
fn generar_deberia_registrar_exportaciones() {
    let modulo = generar(
        "entero sumar(entero a, entero b) {\n    retornar a + b\n}\n\nexportar { sumar }\n",
    );
    assert_eq!(modulo.exportaciones, vec!["sumar".to_string()]);
}

#[test]
fn generar_deberia_procesar_todos_los_ejemplos() {
    let directorio = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ejemplos");
    let mut revisados = 0;

    for entrada in std::fs::read_dir(directorio).expect("debe existir ejemplos/") {
        let ruta = entrada.expect("entrada legible").path();
        if ruta.extension().and_then(|extension| extension.to_str()) != Some("qz") {
            continue;
        }
        let contenido = std::fs::read_to_string(&ruta).expect("ejemplo legible");
        let nombre = ruta.display().to_string();
        let fuente = Fuente::nueva(&nombre, contenido);
        let ast = sintaxis::parsear_modulo(&fuente)
            .unwrap_or_else(|error| panic!("{nombre} debe parsear: {error}"));
        bytecode::generar_modulo(&ast)
            .unwrap_or_else(|error| panic!("{nombre} debe generar bytecode: {error}"));
        revisados += 1;
    }

    assert!(revisados >= 18, "se esperaban al menos 18 ejemplos");
}

#[test]
fn generar_deberia_emitir_cargar_esto_atributo_para_esto() {
    let modulo = generar(
        "objeto Caja {\n    entero var n = 1\n    entero ver() { retornar esto.n }\n}\n",
    );
    let metodo = &modulo.objetos[0].metodos[0].funcion.trozo.instrucciones;
    assert!(
        metodo
            .iter()
            .any(|instruccion| instruccion.op == bytecode::CodigoOp::CargarEstoAtributo
                || instruccion.op == bytecode::CodigoOp::CargarAtributo),
        "esto.n debe usar slot, no CargarMiembro: {metodo:?}"
    );
}
