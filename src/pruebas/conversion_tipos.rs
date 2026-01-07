// Pruebas unitarias para conversión de tipos del lenguaje Quetzal

use super::auxiliares::*;

#[test]
fn prueba_texto_a_entero() {
    let codigo = r#"
        entero valor_entero = "1234".entero()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "valor_entero", 1234));
}

#[test]
fn prueba_texto_a_numero() {
    let codigo = r#"
        número valor_numero = "1234.56".número()
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_texto_a_logico() {
    let codigo = r#"
        log valor_logico = "verdadero".log()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_logico(&entorno, "valor_logico", true));
}

#[test]
fn prueba_texto_a_json() {
    let codigo = r#"
        jsn valor_json = "{\"clave\": \"valor\"}".jsn()
        texto clave = valor_json.clave
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "clave", "valor"));
}

#[test]
fn prueba_entero_a_texto() {
    let codigo = r#"
        texto desde_entero = 1234.texto()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "desde_entero", "1234"));
}

#[test]
fn prueba_numero_a_texto() {
    let codigo = r#"
        texto desde_numero = 1234.56.texto()
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_logico_a_texto() {
    let codigo = r#"
        texto desde_logico = falso.texto()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "desde_logico", "falso"));
}

#[test]
fn prueba_lista_a_texto() {
    let codigo = r#"
        lista<entero> lista_enteros = [1, 2, 3]
        texto desde_lista = lista_enteros.texto()
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_json_a_texto() {
    let codigo = r#"
        jsn valor_json2 = {
            clave: "valor"
        }
        texto desde_json = valor_json2.texto()
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}
