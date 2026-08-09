//! Clasificación de palabras reservadas del Lenguaje Quetzal.

use crate::normalizacion::quitar_tildes;
use crate::token::TipoToken;

/// Si el texto es una palabra reservada, devuelve su token; si no, `None`.
///
/// Acepta variantes con tilde (`número`, `asincróno`, `vacío`) porque los
/// ejemplos del lenguaje las usan indistintamente.
pub fn clasificar(texto: &str) -> Option<TipoToken> {
    let normalizado = quitar_tildes(texto);
    let token = match normalizado.as_str() {
        // Tipos
        "entero" => TipoToken::TipoEntero,
        "numero" => TipoToken::TipoNumero,
        "texto" => TipoToken::TipoTexto,
        "log" => TipoToken::TipoLog,
        "lista" => TipoToken::TipoLista,
        "jsn" => TipoToken::TipoJsn,
        "vacio" => TipoToken::TipoVacio,

        // Control de flujo
        "si" => TipoToken::Si,
        "sino" => TipoToken::Sino,
        "mientras" => TipoToken::Mientras,
        "para" => TipoToken::Para,
        "hacer" => TipoToken::Hacer,
        "en" => TipoToken::En,
        "cada" => TipoToken::Cada,
        "romper" => TipoToken::Romper,
        "continuar" => TipoToken::Continuar,
        "retornar" => TipoToken::Retornar,

        // Excepciones
        "intentar" => TipoToken::Intentar,
        "capturar" => TipoToken::Capturar,
        "finalmente" => TipoToken::Finalmente,
        "lanzar" => TipoToken::Lanzar,
        "excepcion" => TipoToken::Excepcion,

        // Objetos y prototipos
        "objeto" => TipoToken::Objeto,
        "prototipo" => TipoToken::Prototipo,
        "hereda" => TipoToken::Hereda,
        "implementa" => TipoToken::Implementa,
        "como" => TipoToken::Como,
        "nuevo" => TipoToken::Nuevo,
        "esto" => TipoToken::Esto,
        "padre" => TipoToken::Padre,
        "publico" => TipoToken::Publico,
        "privado" => TipoToken::Privado,
        "libre" => TipoToken::Libre,
        "opcional" => TipoToken::Opcional,

        // Asincronía
        "asincrono" => TipoToken::Asincrono,
        "esperar" => TipoToken::Esperar,

        // Módulos
        "importar" => TipoToken::Importar,
        "exportar" => TipoToken::Exportar,
        "desde" => TipoToken::Desde,

        // Operadores lógicos
        "y" => TipoToken::YLogico,
        "o" => TipoToken::OLogico,

        // Otros
        "var" => TipoToken::Var,
        "verdadero" => TipoToken::Verdadero,
        "falso" => TipoToken::Falso,
        "nulo" => TipoToken::Nulo,

        _ => return None,
    };
    Some(token)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn clasificar_deberia_aceptar_variantes_con_tilde() {
        assert_eq!(clasificar("número"), Some(TipoToken::TipoNumero));
        assert_eq!(clasificar("asincróno"), Some(TipoToken::Asincrono));
    }

    #[test]
    fn clasificar_deberia_reconocer_operadores_logicos() {
        assert_eq!(clasificar("y"), Some(TipoToken::YLogico));
        assert_eq!(clasificar("o"), Some(TipoToken::OLogico));
    }

    #[test]
    fn clasificar_deberia_devolver_none_para_identificadores() {
        assert_eq!(clasificar("año"), None);
        assert_eq!(clasificar("mi_variable"), None);
    }
}
