//! Coordinación de lexer, parser y análisis semántico sin ejecutar código.

use nucleo::{ErrorQuetzal, Fuente};
use semantica::analizar_modulo_para_herramientas;
use sintaxis::parsear_modulo_para_editor;

/// Analiza un fragmento en memoria y acumula errores reales de Quetzal.
pub fn analizar_documento(nombre: &str, contenido: &str) -> Vec<ErrorQuetzal> {
    let fuente = Fuente::nueva(nombre, contenido);
    let resultado = parsear_modulo_para_editor(&fuente);
    let mut errores = resultado.errores;
    if let Some(modulo) = resultado.modulo {
        let analisis = analizar_modulo_para_herramientas(&modulo);
        errores.extend(analisis.diagnosticos);
    }
    errores
}
