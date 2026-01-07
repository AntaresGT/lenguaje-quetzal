// Pruebas unitarias para tipos básicos del lenguaje Quetzal

use super::auxiliares::*;

#[test]
fn prueba_tipos_primitivos() {
    let codigo = r#"
        entero valor_entero = 3
        número valor_numero = 3.1416
        texto valor_texto = "Hola, Quetzal"
        log valor_logico = verdadero
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    
    assert!(verificar_variable_entero(&entorno, "valor_entero", 3));
    assert!(verificar_variable_texto(&entorno, "valor_texto", "Hola, Quetzal"));
    assert!(verificar_variable_logico(&entorno, "valor_logico", true));
}

#[test]
fn prueba_listas_no_tipadas() {
    let codigo = r#"
        lista valor_lista = [1, 2, "Texto", verdadero]
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_listas_tipadas() {
    let codigo = r#"
        lista<entero> valor_lista_tipada = [1, 2, 3, 4, 5]
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_json_nativo() {
    let codigo = r#"
        jsn valor_json = {
            valor_entero: 42,
            valor_texto: "Texto en JSON",
            valor_logico: falso
        }
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_tipos_mutables() {
    let codigo = r#"
        entero var valor_entero_mutable = 10
        número var valor_numero_mutable = 2.71828
        texto var valor_texto_mutable = "Texto mutable"
        log var valor_logico_mutable = falso
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_valores_nulos() {
    let codigo = r#"
        entero valor_entero_nulo = nulo
        número valor_numero_nulo = nulo
        texto valor_texto_nulo = nulo
        log valor_logico_nulo = nulo
        lista valor_lista_nula = nulo
        jsn valor_json_nulo = nulo
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_distincion_tildes() {
    let codigo = r#"
        número valor_con_tilde = 5.5
        numero valor_sin_tilde = 10.10
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}
