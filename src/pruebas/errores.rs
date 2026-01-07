// Pruebas unitarias para el sistema de reportes de errores del lenguaje Quetzal

use crate::errores::{Error, CodigoError};

// ============================================
// Pruebas de creacion de errores
// ============================================

#[test]
fn prueba_error_analisis_basico() {
    let error = Error::analisis(
        CodigoError::SintaxisGeneral,
        "error de sintaxis",
        None,
        Some(1),
        Some(5),
    );
    
    assert_eq!(error.codigo(), "E0001");
}

#[test]
fn prueba_error_semantico() {
    let error = Error::semantico(
        CodigoError::VariableNoDeclarada,
        "variable 'x' no esta declarada",
        Some("declara la variable antes de usarla".to_string()),
        Some(10),
        Some(15),
    );
    
    assert_eq!(error.codigo(), "E0100");
}

#[test]
fn prueba_error_ejecucion() {
    let error = Error::ejecucion(
        CodigoError::DivisionPorCero,
        "no se puede dividir por cero",
        None,
        Some(5),
        Some(10),
    );
    
    assert_eq!(error.codigo(), "E0206");
}

#[test]
fn prueba_error_modulo() {
    let error = Error::modulo(
        CodigoError::ModuloNoEncontrado,
        "no se encontro el modulo 'test'",
        Some("test.qz".to_string()),
    );
    
    assert_eq!(error.codigo(), "E0400");
}

#[test]
fn prueba_error_sistema() {
    let error = Error::sistema(
        CodigoError::ErrorLecturaArchivo,
        "no se pudo leer el archivo",
        Some("archivo no encontrado".to_string()),
    );
    
    assert_eq!(error.codigo(), "E0900");
}

// ============================================
// Pruebas de formato de errores
// ============================================

#[test]
fn prueba_formato_error_con_ubicacion() {
    let error = Error::analisis(
        CodigoError::CadenaSinCerrar,
        "cadena sin cerrar",
        Some("test.qz".to_string()),
        Some(5),
        Some(10),
    );
    
    let mensaje = error.to_string();
    assert!(mensaje.contains("cadena sin cerrar"), "Mensaje deberia contener descripcion");
}

#[test]
fn prueba_formato_error_con_ayuda() {
    let error = Error::semantico(
        CodigoError::VariableNoDeclarada,
        "variable 'x' no esta declarada",
        Some("declara la variable antes de usarla".to_string()),
        Some(1),
        Some(1),
    );
    
    // Verificar que el error tiene ayuda
    if let Error::Semantico { ayuda, .. } = &error {
        assert!(ayuda.is_some(), "Error deberia tener ayuda");
    }
}

// ============================================
// Pruebas de codigos de error
// ============================================

#[test]
fn prueba_codigos_error_sintaxis() {
    assert_eq!(CodigoError::SintaxisGeneral.codigo(), "E0001");
    assert_eq!(CodigoError::ComentarioSinCerrar.codigo(), "E0002");
    assert_eq!(CodigoError::CadenaSinCerrar.codigo(), "E0003");
}

#[test]
fn prueba_codigos_error_declaracion() {
    assert_eq!(CodigoError::VariableNoDeclarada.codigo(), "E0100");
    assert_eq!(CodigoError::VariableRedeclarada.codigo(), "E0101");
    assert_eq!(CodigoError::FuncionRedeclarada.codigo(), "E0103");
}

#[test]
fn prueba_codigos_error_tipos() {
    assert_eq!(CodigoError::TiposIncompatibles.codigo(), "E0200");
    assert_eq!(CodigoError::DivisionPorCero.codigo(), "E0206");
}

#[test]
fn prueba_codigos_error_control_flujo() {
    assert_eq!(CodigoError::RomperFueraDeBucle.codigo(), "E0303");
    assert_eq!(CodigoError::ContinuarFueraDeBucle.codigo(), "E0304");
}

#[test]
fn prueba_codigos_error_modulos() {
    assert_eq!(CodigoError::ModuloNoEncontrado.codigo(), "E0400");
    assert_eq!(CodigoError::DependenciaCircular.codigo(), "E0402");
}

// ============================================
// Pruebas de reporte de errores
// ============================================

#[test]
fn prueba_reportar_error_no_panic() {
    use crate::errores::reporte::reportar_error;
    
    let error = Error::analisis(
        CodigoError::SintaxisGeneral,
        "prueba de error",
        None,
        Some(1),
        Some(1),
    );
    
    let codigo_fuente = "entero x = 10";
    
    // Esto no deberia causar panic
    reportar_error(&error, Some(codigo_fuente));
}

#[test]
fn prueba_reportar_error_simple_no_panic() {
    use crate::errores::reporte::reportar_error_simple;
    
    let error = Error::sistema(
        CodigoError::ErrorInternoInterprete,
        "error interno",
        None,
    );
    
    // Esto no deberia causar panic
    reportar_error_simple(&error);
}

#[test]
fn prueba_reportar_error_modulo() {
    use crate::errores::reporte::reportar_error;
    
    let error = Error::modulo(
        CodigoError::ModuloNoEncontrado,
        "modulo no encontrado",
        Some("test.qz".to_string()),
    );
    
    // Esto no deberia causar panic
    reportar_error(&error, None);
}
