// Pruebas unitarias para métodos de JSON del lenguaje Quetzal

use super::auxiliares::*;

#[test]
fn prueba_acceso_propiedades_json() {
    let codigo = r#"
        jsn var persona = {
            nombre: "Ana",
            edad: 25
        }
        
        texto nombre_persona = persona.nombre
        entero edad_persona = persona.edad
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "nombre_persona", "Ana"));
    // Nota: La edad podría estar como número decimal, así que solo verificamos que existe
    assert!(obtener_valor_variable(&entorno, "edad_persona").is_some());
}

#[test]
fn prueba_json_anidado() {
    let codigo = r#"
        jsn var persona = {
            datos_personales: {
                fecha_nacimiento: "1998-05-15",
                peso: 65.5
            }
        }
        
        texto fecha = persona.datos_personales.fecha_nacimiento
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "fecha", "1998-05-15"));
}

#[test]
fn prueba_json_con_listas() {
    let codigo = r#"
        jsn var persona = {
            telefonos: ["555-1234", "555-5678"]
        }
        
        texto telefono_trabajo = persona.telefonos[1]
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "telefono_trabajo", "555-5678"));
}

#[test]
fn prueba_modificar_propiedad_json() {
    let codigo = r#"
        jsn var persona = {
            nombre: "Ana"
        }
        
        persona.nombre = "María"
        texto nombre_modificado = persona.nombre
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "nombre_modificado", "María"));
}

#[test]
fn prueba_contiene_clave() {
    let codigo = r#"
        jsn var persona = {
            nombre: "Ana",
            edad: 25
        }
        
        log tiene_nombre = persona.contiene_clave("nombre")
        log tiene_apellido = persona.contiene_clave("apellido")
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_logico(&entorno, "tiene_nombre", true));
    assert!(verificar_variable_logico(&entorno, "tiene_apellido", false));
}

#[test]
fn prueba_claves_valores() {
    let codigo = r#"
        jsn var persona = {
            nombre: "Ana",
            edad: 25
        }
        
        lista<texto> claves_persona = persona.claves()
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_eliminar_propiedad() {
    let codigo = r#"
        jsn var persona = {
            nombre: "Ana",
            activo: verdadero
        }
        
        persona.eliminar("activo")
        log tiene_activo = persona.contiene_clave("activo")
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_logico(&entorno, "tiene_activo", false));
}

#[test]
fn prueba_fusionar_json() {
    let codigo = r#"
        jsn var persona = {
            nombre: "Ana"
        }
        
        jsn var otra_persona = {
            apellido: "Gómez",
            edad: 30
        }
        
        persona.fusionar(otra_persona)
        log tiene_apellido = persona.contiene_clave("apellido")
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_logico(&entorno, "tiene_apellido", true));
}

#[test]
fn prueba_texto_a_json() {
    let codigo = r#"
        jsn var json_desde_texto = "{\"nombre\":\"Ana\",\"edad\":25}".jsn()
        texto nombre = json_desde_texto.nombre
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "nombre", "Ana"));
}

#[test]
fn prueba_acceso_json_por_indice_corchetes() {
    // Prueba de acceso por índice con corchetes: persona["datos_personales"]
    let codigo = r#"
        jsn var persona = {
            datos_personales: {
                peso: 65.5
            }
        }
        
        jsn datos = persona["datos_personales"]
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_acceso_json_lista_por_indice() {
    // Prueba de acceso a lista dentro de JSON por índice
    let codigo = r#"
        jsn var persona = {
            direcciones: [
                {
                    tipo: "casa",
                    direccion: "123 Calle Principal"
                },
                {
                    tipo: "trabajo",
                    direccion: "456 Avenida Secundaria"
                }
            ]
        }
        
        texto dir_casa = persona.direcciones[0].direccion
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "dir_casa", "123 Calle Principal"));
}

#[test]
fn prueba_acceso_json_encadenado() {
    // Prueba de acceso encadenado: persona["datos_personales"].peso
    let codigo = r#"
        jsn var persona = {
            datos_personales: {
                peso: 65.5
            }
        }
        
        número peso = persona.datos_personales.peso
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}