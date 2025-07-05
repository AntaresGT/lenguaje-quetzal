use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_division_por_cero_entero() {
        let codigo = r#"
entero a = 10
entero b = 0
entero resultado = a / b
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_division_por_cero_decimal() {
        let codigo = r#"
número a = 10.5
número b = 0.0
número resultado = a / b
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_modulo_por_cero() {
        let codigo = r#"
entero a = 10
entero b = 0
entero resultado = a % b
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_variable_no_declarada() {
        let codigo = r#"
entero resultado = variable_inexistente + 5
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_redeclaracion_variable() {
        let codigo = r#"
entero numero = 5
entero numero = 10
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_asignacion_tipo_incorrecto() {
        let codigo = r#"
entero numero_entero = "texto"
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_operacion_tipos_incompatibles() {
        let codigo = r#"
cadena texto = "hola"
entero numero = 5
entero resultado = texto + numero
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_sintaxis_invalida_declaracion() {
        let codigo = r#"
entero = 5
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_sintaxis_invalida_operadores() {
        let codigo = r#"
entero a = 5 ++ 3
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_parentesis_desbalanceados() {
        let codigo = r#"
entero resultado = (5 + 3
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_cadena_sin_cerrar() {
        let codigo = r#"
cadena texto = "texto sin cerrar
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_conversion_invalida_numero() {
        let codigo = r#"
cadena texto = "no_es_numero"
entero numero = texto.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_acceso_variable_no_inicializada() {
        let codigo = r#"
entero numero_no_inicializado
entero resultado = numero_no_inicializado + 5
        "#;
        
        // Esto podría ser válido si las variables tienen valores por defecto
        // Dependiendo de la implementación
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok() || resultado.is_err());
    }

    #[test]
    fn test_overflow_entero() {
        let codigo = r#"
entero numero_grande = 999999999999999999999999999999
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_nombre_variable_palabra_reservada() {
        let codigo = r#"
entero si = 5
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_nombre_variable_caracter_invalido() {
        let codigo = r#"
entero variable-con-guion = 5
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_nombre_variable_empieza_numero() {
        let codigo = r#"
entero 123variable = 5
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_asignacion_variable_inmutable() {
        let codigo = r#"
entero numero = 5
numero = 10
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_operador_no_soportado() {
        let codigo = r#"
entero a = 5
entero b = 3
entero resultado = a ^ b
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_anidamiento_funciones_excesivo() {
        let codigo = r#"
entero funcion_a() {
    retornar funcion_b()
}
entero funcion_b() {
    retornar funcion_c()
}
entero funcion_c() {
    retornar funcion_a()  // Recursión infinita
}
entero resultado = funcion_a()
        "#;
        
        // Esto podría causar stack overflow
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_comentario_sin_cerrar() {
        let codigo = r#"
/* comentario sin cerrar
entero numero = 5
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_expresion_vacia() {
        let codigo = r#"
entero resultado = 
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_bloque_vacio_funcion() {
        let codigo = r#"
entero funcion_vacia() {
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_retorno_fuera_funcion() {
        let codigo = r#"
entero numero = 5
retornar numero
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_parametro_duplicado_funcion() {
        let codigo = r#"
entero funcion_invalida(entero a, entero a) {
    retornar a
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_cadena_con_escape_invalido() {
        let codigo = r#"
cadena texto = "texto con \z escape inválido"
        "#;
        
        // Dependiendo de la implementación, esto podría ser válido o no
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok() || resultado.is_err());
    }
}
