// Pruebas unitarias para operadores del lenguaje Quetzal

use super::auxiliares::*;

#[test]
fn prueba_operaciones_con_tipos_diferentes() {
    let codigo = r#"
        entero resultado_suma = 5 + 10.5
        número resultado_resta = 10.5 - 5
        texto resultado_concatenacion = "Hola" + " Mundo"
        log resultado_logico = verdadero y falso
        log resultado_logico2 = verdadero o falso
        log resultado_logico3 = !verdadero
        log resultado_logico4 = !falso
    "#;
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_operadores_aritmeticos() {
    let codigo = r#"
        entero suma(entero a, entero b) {
            retornar a + b
        }
        
        entero resta(entero a, entero b) {
            retornar a - b
        }
        
        entero resultado_suma = suma(5, 10)
        entero resultado_resta = resta(10, 5)
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "resultado_suma", 15));
    assert!(verificar_variable_entero(&entorno, "resultado_resta", 5));
}

#[test]
fn prueba_operadores_logicos() {
    let codigo = r#"
        log es_verdadero(log valor) {
            retornar valor
        }
        
        log es_falso(log valor) {
            retornar !valor
        }
        
        log resultado_es_verdadero = es_verdadero(verdadero)
        log resultado_es_falso = es_falso(falso)
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_logico(&entorno, "resultado_es_verdadero", true));
    assert!(verificar_variable_logico(&entorno, "resultado_es_falso", true));
}

#[test]
fn prueba_operadores_comparacion() {
    let codigo = r#"
        log es_igual(entero a, entero b) {
            retornar a == b
        }
        
        log es_diferente(entero a, entero b) {
            retornar a != b
        }
        
        log es_mayor(entero a, entero b) {
            retornar a > b
        }
        
        log es_menor(entero a, entero b) {
            retornar a < b
        }
        
        log resultado_es_igual = es_igual(5, 5)
        log resultado_es_diferente = es_diferente(5, 10)
        log resultado_es_mayor = es_mayor(10, 5)
        log resultado_es_menor = es_menor(5, 10)
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_logico(&entorno, "resultado_es_igual", true));
    assert!(verificar_variable_logico(&entorno, "resultado_es_diferente", true));
    assert!(verificar_variable_logico(&entorno, "resultado_es_mayor", true));
    assert!(verificar_variable_logico(&entorno, "resultado_es_menor", true));
}

#[test]
fn prueba_operadores_incremento_decremento() {
    let codigo = r#"
        entero incremento(entero var valor) {
            valor++
            retornar valor
        }
        
        entero decremento(entero var valor) {
            valor--
            retornar valor
        }
        
        entero resultado_incremento = incremento(5)
        entero resultado_decremento = decremento(5)
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "resultado_incremento", 6));
    assert!(verificar_variable_entero(&entorno, "resultado_decremento", 4));
}

#[test]
fn prueba_operadores_compuestos() {
    let codigo = r#"
        entero suma_compuesta(entero var valor, entero incremento) {
            valor += incremento
            retornar valor
        }
        
        entero resta_compuesta(entero var valor, entero decremento) {
            valor -= decremento
            retornar valor
        }
        
        entero multiplicacion_compuesta(entero var valor, entero factor) {
            valor *= factor
            retornar valor
        }
        
        entero division_compuesta(entero var valor, entero divisor) {
            valor /= divisor
            retornar valor
        }
        
        entero resultado_suma_compuesta = suma_compuesta(5, 10)
        entero resultado_resta_compuesta = resta_compuesta(10, 5)
        entero resultado_multiplicacion_compuesta = multiplicacion_compuesta(5, 10)
        entero resultado_division_compuesta = division_compuesta(10, 5)
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "resultado_suma_compuesta", 15));
    assert!(verificar_variable_entero(&entorno, "resultado_resta_compuesta", 5));
    assert!(verificar_variable_entero(&entorno, "resultado_multiplicacion_compuesta", 50));
    assert!(verificar_variable_entero(&entorno, "resultado_division_compuesta", 2));
}

#[test]
fn prueba_operadores_concatenacion() {
    let codigo = r#"
        texto concatenacion(texto var palabra) {
            palabra += " concatenado"
            retornar palabra
        }
        
        texto resultado_concatenacion = concatenacion("Hola")
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "resultado_concatenacion", "Hola concatenado"));
}
