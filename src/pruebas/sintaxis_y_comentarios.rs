use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comentario_linea_simple() {
        let codigo = r#"
// Este es un comentario de línea
entero numero = 42
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_comentario_al_final_linea() {
        let codigo = r#"
entero numero = 42  // Comentario al final
cadena texto = "hola"  // Otro comentario
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_comentario_bloque() {
        let codigo = r#"
/* Este es un comentario
   de múltiples líneas */
entero numero = 42
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_comentario_bloque_inline() {
        let codigo = r#"
entero numero = /* comentario en medio */ 42
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_comentarios_multiples_lineas() {
        let codigo = r#"
// Comentario 1
entero a = 10
// Comentario 2
entero b = 20
// Comentario 3
entero suma = a + b
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_comentario_dentro_funcion() {
        let codigo = r#"
entero calcular(entero x) {
    // Multiplicar por 2
    entero resultado = x * 2
    /* Retornar el resultado */
    retornar resultado
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_lineas_vacias() {
        let codigo = r#"
entero numero = 42


cadena texto = "hola"

        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_espacios_y_tabs() {
        let codigo = r#"
    entero numero = 42
	cadena texto = "hola"
        bool estado = verdadero
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_comentario_con_caracteres_especiales() {
        let codigo = r#"
// Comentario con ñ, á, é, í, ó, ú, ü
entero numero = 42
/* Comentario con símbolos: @#$%^&*()+=[]{}|;:'"<>?/ */
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_comentario_solo_barras() {
        let codigo = r#"
// 
entero numero = 42
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_comentario_bloque_vacio() {
        let codigo = r#"
/**/
entero numero = 42
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_comentario_anidado_falso() {
        let codigo = r#"
/* Comentario principal /* esto no debería anidar */ fin */
entero numero = 42
        "#;
        
        // Los comentarios anidados generalmente no son soportados
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_codigo_sin_comentarios() {
        let codigo = r#"
entero numero = 42
cadena texto = "hola"
bool estado = verdadero
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_solo_comentarios() {
        let codigo = r#"
// Solo comentarios en este archivo
/* No hay código ejecutable */
// Esto debería ser válido
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_comentario_con_codigo_comentado() {
        let codigo = r#"
entero numero = 42
// entero otro_numero = 100
cadena texto = "hola"
/* 
entero variable_comentada = 50
cadena otra_variable = "comentada"
*/
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_comentario_con_doble_barra() {
        let codigo = r#"
// Comentario con // doble barra dentro
entero numero = 42
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_estructura_con_indentacion() {
        let codigo = r#"
entero calcular() {
    entero a = 10
    entero b = 20
    si (a < b) {
        retornar b
    } sino {
        retornar a
    }
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_expresion_multilinea() {
        let codigo = r#"
entero resultado = 10 +
                   20 +
                   30
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_funcion_multilinea() {
        let codigo = r#"
entero funcion_larga(entero parametro1,
                     entero parametro2,
                     entero parametro3) {
    retornar parametro1 + parametro2 + parametro3
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_comentario_unicode() {
        let codigo = r#"
// Prueba con emoji 🚀 y caracteres unicode ñáéíóúü
entero numero = 42
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_error_comentario_bloque_sin_cerrar() {
        let codigo = r#"
/* Comentario sin cerrar
entero numero = 42
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_archivo_vacio() {
        let codigo = "";
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_solo_espacios_y_saltos() {
        let codigo = "   \n\n  \t  \n   ";
        
        assert!(interprete::interpretar(codigo).is_ok());
    }
}
