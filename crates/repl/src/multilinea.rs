//! Detección de bloques sin cerrar para la entrada multilínea.

use reedline::{ValidationResult, Validator};

/// Marca la entrada como incompleta mientras haya `{`, `(`, `[`, un texto o
/// un comentario de bloque sin cerrar.
pub struct ValidadorBloques;

impl Validator for ValidadorBloques {
    fn validate(&self, linea: &str) -> ValidationResult {
        if entrada_incompleta(linea) {
            ValidationResult::Incomplete
        } else {
            ValidationResult::Complete
        }
    }
}

/// Recorre el texto ignorando textos y comentarios, contando delimitadores.
pub fn entrada_incompleta(texto: &str) -> bool {
    let mut profundidad: i64 = 0;
    let mut caracteres = texto.chars().peekable();
    let mut en_texto = false;
    let mut en_comentario_bloque = false;
    let mut en_comentario_linea = false;

    while let Some(caracter) = caracteres.next() {
        if en_comentario_linea {
            if caracter == '\n' {
                en_comentario_linea = false;
            }
            continue;
        }
        if en_comentario_bloque {
            if caracter == '*' && caracteres.peek() == Some(&'/') {
                caracteres.next();
                en_comentario_bloque = false;
            }
            continue;
        }
        if en_texto {
            match caracter {
                '\\' => {
                    caracteres.next();
                }
                '"' => en_texto = false,
                _ => {}
            }
            continue;
        }
        match caracter {
            '"' => en_texto = true,
            '/' if caracteres.peek() == Some(&'/') => en_comentario_linea = true,
            '/' if caracteres.peek() == Some(&'*') => {
                caracteres.next();
                en_comentario_bloque = true;
            }
            '{' | '(' | '[' => profundidad += 1,
            '}' | ')' | ']' => profundidad -= 1,
            _ => {}
        }
    }

    en_texto || en_comentario_bloque || profundidad > 0
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn entrada_incompleta_deberia_detectar_llaves_abiertas() {
        assert!(entrada_incompleta("si (x > 1) {"));
        assert!(!entrada_incompleta("si (x > 1) {\n    x = 2\n}"));
    }

    #[test]
    fn entrada_incompleta_deberia_ignorar_delimitadores_en_textos() {
        assert!(!entrada_incompleta("texto t = \"abre { y no pasa nada\""));
        assert!(entrada_incompleta("texto t = \"sin cerrar"));
    }

    #[test]
    fn entrada_incompleta_deberia_ignorar_comentarios() {
        assert!(!entrada_incompleta("// abre { sin problema"));
        assert!(entrada_incompleta("/* comentario sin cerrar"));
    }
}
