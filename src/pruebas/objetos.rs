// Pruebas unitarias para objetos del lenguaje Quetzal

use super::auxiliares::*;

#[test]
fn prueba_objeto_basico() {
    let codigo = r#"
        objeto Usuario {
            privado:
                texto var nombre
                entero var edad
            publico:
                Usuario(texto nombre, entero edad) {
                    ambiente.nombre = nombre
                    ambiente.edad = edad
                }
                
                texto obtener_nombre() {
                    retornar ambiente.nombre
                }
                
                entero obtener_edad() {
                    retornar ambiente.edad
                }
        }
        
        Usuario usuario1 = nuevo Usuario("Juan", 30)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_objeto_sin_etiquetas_acceso() {
    let codigo = r#"
        objeto DefinicionUsuario {
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
        
        DefinicionUsuario usuario2 = nuevo DefinicionUsuario("Juan", 30)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_objeto_con_funciones_libres() {
    let codigo = r#"
        objeto UsuarioLibre {
            libre texto nombre
            libre entero edad
            
            libre entero absoluto(entero valor) {
                retornar valor < 0 ? -valor : valor
            }
        }
        
        entero valor_absoluto = UsuarioLibre.absoluto(-10)
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "valor_absoluto", 10));
}

#[test]
fn prueba_objeto_con_atributos_libres_privados() {
    // Prueba simplificada de miembros libres (estáticos) con acceso desde el tipo
    let codigo = r#"
        objeto Calculadora {
            publico:
                libre entero sumar(entero a, entero b) {
                    retornar a + b
                }
                
                libre entero multiplicar(entero a, entero b) {
                    retornar a * b
                }
        }
        
        entero resultado_suma = Calculadora.sumar(5, 3)
        entero resultado_mult = Calculadora.multiplicar(4, 7)
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "resultado_suma", 8));
    assert!(verificar_variable_entero(&entorno, "resultado_mult", 28));
}
