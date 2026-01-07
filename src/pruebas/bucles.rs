// Pruebas unitarias para bucles del lenguaje Quetzal

use super::auxiliares::*;

#[test]
fn prueba_bucle_mientras() {
    let codigo = r#"
        entero var iterador_mientras = 0
        
        mientras (iterador_mientras < 5) {
            iterador_mientras++
        }
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "iterador_mientras", 5));
}

#[test]
fn prueba_bucle_para() {
    let codigo = r#"
        entero var suma = 0
        
        para (entero var i = 0; i < 5; i++) {
            suma += i
        }
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    // Suma de 0 + 1 + 2 + 3 + 4 = 10
    assert!(verificar_variable_entero(&entorno, "suma", 10));
}

#[test]
fn prueba_bucle_hacer_mientras() {
    let codigo = r#"
        entero var iterador_hacer = 0
        
        hacer {
            iterador_hacer++
        } mientras (iterador_hacer < 3)
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "iterador_hacer", 3));
}

#[test]
fn prueba_bucle_en() {
    let codigo = r#"
        lista<entero> lista_numeros = [10, 20, 30]
        entero var suma = 0
        
        para (entero var valor_numero en lista_numeros) {
            suma += valor_numero
        }
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    // Suma de 10 + 20 + 30 = 60
    assert!(verificar_variable_entero(&entorno, "suma", 60));
}

#[test]
fn prueba_bucle_cada() {
    let codigo = r#"
        lista<entero> lista_numeros = [10, 20, 30]
        entero var suma = 0
        
        para (entero var valor_numero cada lista_numeros) {
            suma += valor_numero
        }
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    // Suma de 10 + 20 + 30 = 60
    assert!(verificar_variable_entero(&entorno, "suma", 60));
}

#[test]
fn prueba_romper_en_bucle_mientras() {
    let codigo = r#"
        entero var contador = 0
        
        mientras (contador < 10) {
            si (contador == 5) {
                romper
            }
            contador++
        }
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "contador", 5));
}
