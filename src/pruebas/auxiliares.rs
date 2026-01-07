// Funciones auxiliares para las pruebas unitarias

use crate::errores::Resultado;
use crate::interprete::entorno::Entorno;
use crate::interprete::declaraciones::evaluar_declaracion;
use crate::interprete::valores::Valor;
use crate::nucleo::sintactico::Parser;
use crate::nativos::registro::registrar_modulos_nativos;

/// Ejecuta código Quetzal y retorna el entorno con las variables definidas
pub fn ejecutar_codigo(codigo: &str) -> Resultado<Entorno> {
    let ast = Parser::parsear(codigo)?;
    let mut entorno = Entorno::nuevo();
    registrar_modulos_nativos(&mut entorno)?;
    
    for nodo in &ast {
        evaluar_declaracion(nodo, &mut entorno)?;
    }
    
    Ok(entorno)
}

/// Obtiene el valor de una variable del entorno
pub fn obtener_valor_variable(entorno: &Entorno, nombre: &str) -> Option<Valor> {
    entorno.obtener_variable(nombre).cloned()
}

/// Verifica que una variable existe y tiene un valor específico
pub fn verificar_variable_entero(entorno: &Entorno, nombre: &str, valor_esperado: i64) -> bool {
    if let Some(Valor::Entero(valor)) = obtener_valor_variable(entorno, nombre) {
        valor == valor_esperado
    } else {
        false
    }
}

/// Verifica que una variable existe y tiene un valor de texto específico
pub fn verificar_variable_texto(entorno: &Entorno, nombre: &str, valor_esperado: &str) -> bool {
    if let Some(Valor::Texto(valor)) = obtener_valor_variable(entorno, nombre) {
        valor == valor_esperado
    } else {
        false
    }
}

/// Verifica que una variable existe y tiene un valor lógico específico
pub fn verificar_variable_logico(entorno: &Entorno, nombre: &str, valor_esperado: bool) -> bool {
    if let Some(Valor::Logico(valor)) = obtener_valor_variable(entorno, nombre) {
        valor == valor_esperado
    } else {
        false
    }
}

/// Verifica que el código se ejecuta sin errores
pub fn verificar_ejecucion_exitosa(codigo: &str) -> bool {
    ejecutar_codigo(codigo).is_ok()
}

/// Verifica que el código produce un error
pub fn verificar_ejecucion_con_error(codigo: &str) -> bool {
    ejecutar_codigo(codigo).is_err()
}
