// Pruebas unitarias para funciones del lenguaje Quetzal

use super::auxiliares::*;

#[test]
fn prueba_funcion_sin_retorno() {
    let codigo = r#"
        vacio saludar() {
            consola.mostrar("¡Hola, Quetzal!")
        }
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_funcion_que_devuelve_numero() {
    let codigo = r#"
        número sumar(número a, número b) {
            retornar a + b
        }
        
        número resultado = sumar(5.5, 3.2)
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(obtener_valor_variable(&entorno, "resultado").is_some());
}

#[test]
fn prueba_funcion_que_devuelve_entero() {
    let codigo = r#"
        entero multiplicar(entero a, entero b) {
            retornar a * b
        }
        
        entero resultado = multiplicar(5, 3)
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "resultado", 15));
}

#[test]
fn prueba_funcion_que_devuelve_texto() {
    let codigo = r#"
        texto concatenar(texto a, texto b) {
            retornar a + b
        }
        
        texto resultado = concatenar("Hola", " Mundo")
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "resultado", "Hola Mundo"));
}

#[test]
fn prueba_funcion_que_devuelve_logico() {
    let codigo = r#"
        log es_par(entero valor) {
            retornar valor % 2 == 0
        }
        
        log resultado = es_par(4)
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_logico(&entorno, "resultado", true));
}

#[test]
fn prueba_funcion_que_devuelve_json() {
    let codigo = r#"
        jsn obtener_datos() {
            retornar {
                nombre: "Quetzal",
                version: "0.0.2"
            }
        }
        
        jsn resultado = obtener_datos()
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_funcion_que_devuelve_lista() {
    let codigo = r#"
        lista<entero> obtener_numeros() {
            retornar [1, 2, 3, 4, 5]
        }
        
        lista<entero> resultado = obtener_numeros()
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_funcion_recursiva() {
    let codigo = r#"
        entero factorial(entero n) {
            si (n == 0) {
                retornar 1
            } sino {
                retornar n * factorial(n - 1)
            }
        }
        
        entero resultado = factorial(5)
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "resultado", 120));
}

#[test]
fn prueba_funcion_con_parametros_modificables() {
    let codigo = r#"
        texto funcion_con_parametros_modificables(texto var palabra) {
            palabra += " texto agregado."
            retornar palabra
        }
        
        texto resultado = funcion_con_parametros_modificables("Hola")
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "resultado", "Hola texto agregado."));
}

#[test]
fn prueba_funcion_con_tipo_objeto_retorno() {
    // Prueba de función que retorna un tipo de objeto definido por el usuario
    let codigo = r#"
        objeto DefinicionUsuario {
            publico:
                texto var nombre
                entero var edad
            
                DefinicionUsuario(texto nombre, entero edad) {
                    ambiente.nombre = nombre
                    ambiente.edad = edad
                }
                
                texto obtener_nombre() {
                    retornar ambiente.nombre
                }
        }
        
        DefinicionUsuario crear_usuario(texto nombre, entero edad) {
            retornar nuevo DefinicionUsuario(nombre, edad)
        }
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_funcion_anonima() {
    // Prueba de función anónima (lambda)
    let codigo = r#"
        entero doble(entero x) {
            retornar x * 2
        }
        
        entero resultado = doble(5)
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "resultado", 10));
}