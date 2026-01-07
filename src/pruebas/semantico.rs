// Pruebas unitarias para el verificador semantico del lenguaje Quetzal

use crate::nucleo::sintactico::Parser;
use crate::nucleo::semantico::Verificador;
use crate::errores::CodigoError;

/// Funcion auxiliar para verificar que un codigo pasa la verificacion semantica
fn verificar_semantica_exitosa(codigo: &str) -> bool {
    let ast = match Parser::parsear(codigo) {
        Ok(ast) => ast,
        Err(e) => {
            eprintln!("Error de parseo: {:?}", e);
            return false;
        }
    };
    
    let mut verificador = Verificador::nuevo();
    match verificador.verificar_programa(&ast) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("Error semantico: {:?}", e);
            false
        }
    }
}

/// Funcion auxiliar para verificar que un codigo falla con un codigo de error especifico
fn verificar_error_semantico(codigo: &str, codigo_esperado: CodigoError) -> bool {
    let ast = match Parser::parsear(codigo) {
        Ok(ast) => ast,
        Err(_) => return false,
    };
    
    let mut verificador = Verificador::nuevo();
    match verificador.verificar_programa(&ast) {
        Ok(_) => false,
        Err(e) => e.codigo() == codigo_esperado.codigo(),
    }
}

// ============================================
// Pruebas de declaracion de variables
// ============================================

#[test]
fn prueba_declaracion_variable_valida() {
    let codigo = r#"entero x = 10
texto z = "hola"
log b = verdadero"#;
    
    assert!(verificar_semantica_exitosa(codigo), 
        "Las declaraciones de variables validas deberian pasar");
}

#[test]
fn prueba_variable_redeclarada() {
    let codigo = r#"entero x = 10
entero x = 20"#;
    
    assert!(verificar_error_semantico(codigo, CodigoError::VariableRedeclarada),
        "Redeclarar una variable deberia generar error");
}

// ============================================
// Pruebas de operaciones binarias
// ============================================

#[test]
fn prueba_operacion_numerica_valida() {
    let codigo = r#"entero a = 5
entero b = 3
entero c = a + b"#;
    
    assert!(verificar_semantica_exitosa(codigo),
        "Operaciones numericas entre enteros deberian ser validas");
}

#[test]
fn prueba_concatenacion_texto() {
    let codigo = r#"texto a = "Hola"
texto b = " Mundo"
texto c = a + b"#;
    
    assert!(verificar_semantica_exitosa(codigo),
        "Concatenacion de texto deberia ser valida");
}

#[test]
fn prueba_operacion_logica_valida() {
    let codigo = r#"log a = verdadero
log b = falso
log c = a y b
log d = a o b"#;
    
    assert!(verificar_semantica_exitosa(codigo),
        "Operaciones logicas entre booleanos deberian ser validas");
}

// ============================================
// Pruebas de estructuras de control
// ============================================

#[test]
fn prueba_si_condicion_valida() {
    let codigo = r#"log cond = verdadero
si (cond) {
    entero x = 10
}"#;
    
    assert!(verificar_semantica_exitosa(codigo),
        "Si con condicion booleana deberia ser valido");
}

#[test]
fn prueba_mientras_condicion_valida() {
    let codigo = r#"log var seguir = verdadero
mientras (seguir) {
    seguir = falso
}"#;
    
    assert!(verificar_semantica_exitosa(codigo),
        "Mientras con condicion booleana deberia ser valido");
}

#[test]
fn prueba_romper_fuera_de_bucle() {
    let codigo = "romper";
    
    assert!(verificar_error_semantico(codigo, CodigoError::RomperFueraDeBucle),
        "Romper fuera de bucle deberia generar error");
}

#[test]
fn prueba_continuar_fuera_de_bucle() {
    assert!(verificar_error_semantico("continuar", CodigoError::ContinuarFueraDeBucle),
        "Continuar fuera de bucle deberia generar error");
}

#[test]
fn prueba_romper_dentro_de_bucle() {
    let codigo = r#"mientras (verdadero) {
    romper
}"#;
    
    assert!(verificar_semantica_exitosa(codigo),
        "Romper dentro de bucle deberia ser valido");
}

// ============================================
// Pruebas de funciones
// ============================================

#[test]
fn prueba_declaracion_funcion_valida() {
    let codigo = r#"entero suma(entero a, entero b) {
    retornar a + b
}"#;
    
    assert!(verificar_semantica_exitosa(codigo),
        "Declaracion de funcion valida deberia pasar");
}

#[test]
fn prueba_funcion_redeclarada() {
    let codigo = r#"entero duplicada() {
    retornar 1
}
entero duplicada() {
    retornar 2
}"#;
    
    assert!(verificar_error_semantico(codigo, CodigoError::FuncionRedeclarada),
        "Redeclarar una funcion deberia generar error");
}

// ============================================
// Pruebas de listas
// ============================================

#[test]
fn prueba_lista_valida() {
    let codigo = "lista<entero> nums = [1, 2, 3, 4, 5]";
    
    assert!(verificar_semantica_exitosa(codigo),
        "Lista tipada valida deberia pasar");
}

#[test]
fn prueba_lista_vacia() {
    let codigo = "lista vacia = []";
    
    assert!(verificar_semantica_exitosa(codigo),
        "Lista vacia deberia pasar");
}

// ============================================
// Pruebas de JSON
// ============================================

#[test]
fn prueba_json_valido() {
    let codigo = r#"jsn datos = {
    nombre: "Test",
    valor: 42,
    activo: verdadero
}"#;
    
    assert!(verificar_semantica_exitosa(codigo),
        "JSON valido deberia pasar");
}

// ============================================
// Pruebas de bloques y ambitos
// ============================================

#[test]
fn prueba_variable_en_ambito_interno() {
    // Simplificado: acceso a variable externa en bloque
    let codigo = "entero x = 10\nsi (verdadero) { entero valor = x + 1 }";
    
    assert!(verificar_semantica_exitosa(codigo),
        "Acceso a variable del ambito externo deberia ser valido");
}

#[test]
fn prueba_variable_mismo_nombre_ambitos_distintos() {
    let codigo = r#"entero x = 10
si (verdadero) {
    entero x = 20
}"#;
    
    assert!(verificar_semantica_exitosa(codigo),
        "Declarar variable con mismo nombre en ambito interno deberia ser valido");
}

// ============================================
// Pruebas de manejo de excepciones
// ============================================

#[test]
fn prueba_intentar_capturar_valido() {
    // Simplificado: bloque intentar-capturar en una linea
    let codigo = "intentar { entero x = 10 } capturar (e) { entero val = 1 }";
    
    assert!(verificar_semantica_exitosa(codigo),
        "Bloque intentar-capturar valido deberia pasar");
}

#[test]
fn prueba_lanzar_valido() {
    let codigo = r#"lanzar "Error de prueba""#;
    
    assert!(verificar_semantica_exitosa(codigo),
        "Lanzar excepcion deberia ser valido");
}

// ============================================
// Pruebas de operador ternario
// ============================================

#[test]
fn prueba_ternario_valido() {
    let codigo = r#"log cond = verdadero
entero resultado = cond ? 1 : 0"#;
    
    assert!(verificar_semantica_exitosa(codigo),
        "Operador ternario con condicion booleana deberia ser valido");
}

// ============================================
// Pruebas de bucle para-en
// ============================================

#[test]
fn prueba_para_en_lista() {
    let codigo = r#"lista<entero> nums = [1, 2, 3]
para (entero var n en nums) {
    entero x = n + 1
}"#;
    
    assert!(verificar_semantica_exitosa(codigo),
        "Bucle para-en sobre lista deberia ser valido");
}

// ============================================
// Pruebas de asignacion a inmutables
// ============================================

#[test]
fn prueba_asignacion_a_inmutable() {
    let codigo = r#"entero x = 10
x = 20"#;
    
    assert!(verificar_error_semantico(codigo, CodigoError::AsignacionAInmutable),
        "Asignar a variable inmutable deberia generar error");
}

#[test]
fn prueba_asignacion_a_mutable() {
    let codigo = r#"entero var x = 10
x = 20"#;
    
    assert!(verificar_semantica_exitosa(codigo),
        "Asignar a variable mutable deberia ser valido");
}
