//! Pruebas del modo de parseo tolerante para editores.

use ast::{Elemento, NodoSentencia};
use nucleo::Fuente;
use sintaxis::{ResultadoParseo, parsear_modulo, parsear_modulo_para_editor};

fn parsear(codigo: &str) -> ResultadoParseo {
    parsear_modulo_para_editor(&Fuente::nueva("editor.qz", codigo))
}

fn nombres_variables(resultado: &ResultadoParseo) -> Vec<&str> {
    resultado
        .modulo
        .as_ref()
        .expect("debe existir AST parcial")
        .elementos
        .iter()
        .filter_map(|elemento| match elemento {
            Elemento::Sentencia(sentencia) => match &sentencia.nodo {
                NodoSentencia::DeclaracionVariable { nombre, .. } => Some(nombre.as_str()),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

#[test]
fn editor_deberia_aceptar_modulo_completo() {
    let resultado = parsear("entero valor = 1");
    assert!(resultado.completo);
    assert!(resultado.errores.is_empty());
}

#[test]
fn modo_estricto_deberia_seguir_rechazando_codigo_incompleto() {
    let fuente = Fuente::nueva("estricto.qz", "entero valor =");
    assert!(parsear_modulo(&fuente).is_err());
}

#[test]
fn editor_deberia_conservar_declaraciones_alrededor_de_error() {
    let resultado = parsear("entero antes = 1\n+\ntexto despues = \"ok\"");
    assert_eq!(nombres_variables(&resultado), ["antes", "despues"]);
}

#[test]
fn editor_deberia_conservar_funcion_con_parametros_incompletos() {
    let resultado = parsear("entero sumar(entero a,");
    let modulo = resultado.modulo.expect("debe existir AST parcial");
    let Elemento::Funcion(funcion) = &modulo.elementos[0] else {
        panic!("debe conservar la función");
    };
    assert_eq!(funcion.nombre, "sumar");
    assert_eq!(funcion.parametros.len(), 1);
}

#[test]
fn editor_deberia_conservar_funcion_con_llave_abierta() {
    let resultado = parsear("entero sumar() {\n    retornar 1");
    let modulo = resultado.modulo.expect("debe existir AST parcial");
    assert!(matches!(modulo.elementos[0], Elemento::Funcion(_)));
    assert!(!resultado.completo);
}

#[test]
fn editor_deberia_conservar_variable_sin_expresion() {
    let resultado = parsear("entero valor =");
    assert_eq!(nombres_variables(&resultado), ["valor"]);
}

#[test]
fn editor_deberia_conservar_simbolos_antes_de_acceso_incompleto() {
    let resultado = parsear("texto mensaje = \"Hola\"\nmensaje.");
    assert_eq!(nombres_variables(&resultado), ["mensaje"]);
    assert!(!resultado.tokens.is_empty());
}

#[test]
fn editor_deberia_sobrevivir_importacion_y_cadena_incompletas() {
    let resultado = parsear("importar { ClienteHttp } desde \"");
    assert!(resultado.modulo.is_some());
    assert!(!resultado.errores.is_empty());
}

#[test]
fn editor_deberia_conservar_llamada_con_argumentos_incompletos() {
    let resultado = parsear("entero sumar(entero a, entero b) { retornar a + b }\nsumar(10,");
    let modulo = resultado.modulo.expect("debe existir AST parcial");
    assert_eq!(modulo.elementos.len(), 2);
}

#[test]
fn editor_deberia_recuperar_sentencias_posteriores_dentro_de_bloque() {
    let resultado = parsear("vacio ejecutar() {\n entero antes = 1\n +\n entero despues = 2\n}");
    let modulo = resultado.modulo.expect("debe existir AST parcial");
    let Elemento::Funcion(funcion) = &modulo.elementos[0] else {
        panic!("debe conservar la función");
    };
    assert_eq!(funcion.cuerpo.sentencias.len(), 2);
}

#[test]
fn editor_deberia_continuar_despues_de_error_lexico() {
    let resultado = parsear("entero antes = 1\n@\nentero despues = 2");
    assert_eq!(nombres_variables(&resultado), ["antes", "despues"]);
    assert_eq!(resultado.errores[0].codigo, "E0001");
}
