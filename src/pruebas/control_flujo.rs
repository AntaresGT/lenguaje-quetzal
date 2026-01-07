// Pruebas unitarias para control de flujo del lenguaje Quetzal

use super::auxiliares::*;

#[test]
fn prueba_condicional_si() {
    let codigo = r#"
        entero edad = 25
        texto var mensaje = ""
        
        si (edad > 18) {
            mensaje = "mayor de edad"
        } sino {
            mensaje = "menor de edad"
        }
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "mensaje", "mayor de edad"));
}

#[test]
fn prueba_condicional_si_sino_si() {
    let codigo = r#"
        entero edad = 41
        texto var mensaje = ""
        
        si (edad > 60) {
            mensaje = "tercera edad"
        } sino si (edad > 18) {
            mensaje = "mayor de edad"
        } sino {
            mensaje = "menor de edad"
        }
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "mensaje", "mayor de edad"));
}

#[test]
fn prueba_palabra_continuar() {
    let codigo = r#"
        entero var contador = 0
        
        para (entero var i = 0; i < 10; i++) {
            si (i % 2 == 0) {
                continuar
            }
            contador++
        }
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    // Debería contar solo los impares (1, 3, 5, 7, 9) = 5
    assert!(verificar_variable_entero(&entorno, "contador", 5));
}

#[test]
fn prueba_palabra_romper() {
    let codigo = r#"
        entero var contador = 0
        
        para (entero var i = 0; i < 10; i++) {
            si (i == 5) {
                romper
            }
            contador++
        }
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    // Debería contar hasta 5 (0, 1, 2, 3, 4) = 5
    assert!(verificar_variable_entero(&entorno, "contador", 5));
}
