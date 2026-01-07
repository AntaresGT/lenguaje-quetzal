// Pruebas unitarias para manejo de excepciones del lenguaje Quetzal

use super::auxiliares::*;

#[test]
fn prueba_funcion_que_lanza_excepcion() {
    let codigo = r#"
        número dividir(número numerador, número denominador) {
            si (denominador == 0) {
                lanzar "Error: División por cero no permitida"
            }
            retornar numerador / denominador
        }
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_manejo_excepcion_con_capturar() {
    let codigo = r#"
        número dividir(número numerador, número denominador) {
            si (denominador == 0) {
                lanzar "Error: División por cero no permitida"
            }
            retornar numerador / denominador
        }
        
        texto var mensaje_error = ""
        
        intentar {
            número resultado = dividir(10, 0)
        } capturar (excepcion e) {
            mensaje_error = e.mensaje
        }
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "mensaje_error", "Error: División por cero no permitida"));
}

#[test]
fn prueba_division_exitosa_sin_excepcion() {
    let codigo = r#"
        número dividir(número numerador, número denominador) {
            si (denominador == 0) {
                lanzar "Error: División por cero no permitida"
            }
            retornar numerador / denominador
        }
        
        número var resultado = 0
        
        intentar {
            resultado = dividir(10, 2)
        } capturar (excepcion e) {
            resultado = -1
        }
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    // El resultado debería ser 5, no -1
    if let Some(valor) = obtener_valor_variable(&entorno, "resultado") {
        match valor {
            crate::interprete::valores::Valor::Numero(n) => {
                assert_eq!(n.to_string(), "5");
            }
            crate::interprete::valores::Valor::Entero(n) => {
                assert_eq!(n, 5);
            }
            _ => panic!("Se esperaba un número, pero se obtuvo {:?}", valor),
        }
    } else {
        panic!("No se encontró la variable resultado");
    }
}

#[test]
fn prueba_bloque_finalmente() {
    let codigo = r#"
        número dividir(número numerador, número denominador) {
            si (denominador == 0) {
                lanzar "Error: División por cero no permitida"
            }
            retornar numerador / denominador
        }
        
        texto var mensaje_final = ""
        
        intentar {
            número resultado = dividir(10, 0)
        } capturar (excepcion e) {
            mensaje_final = "Error capturado"
        } finalmente {
            mensaje_final = mensaje_final + " - Finalizado"
        }
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "mensaje_final", "Error capturado - Finalizado"));
}

#[test]
fn prueba_excepcion_propiedad_llamadas() {
    // Prueba que la excepción tiene la propiedad 'llamadas'
    let codigo = r#"
        número dividir(número numerador, número denominador) {
            si (denominador == 0) {
                lanzar "Error: División por cero"
            }
            retornar numerador / denominador
        }
        
        lista<texto> var pila = []
        
        intentar {
            número resultado = dividir(10, 0)
        } capturar (excepcion e) {
            pila = e.llamadas
        }
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_capturar_sin_tipo_explicito() {
    // Prueba de capturar con solo el identificador (sin el tipo 'excepcion')
    let codigo = r#"
        texto var mensaje_error = ""
        
        intentar {
            lanzar "Error de prueba"
        } capturar (e) {
            mensaje_error = e.mensaje
        }
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "mensaje_error", "Error de prueba"));
}