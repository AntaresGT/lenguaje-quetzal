use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversion_entero_a_cadena() {
        let codigo = r#"
entero numero = 42
texto mi_texto = numero.texto()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_decimal_a_cadena() {
        let codigo = r#"
número decimal = 3.14159
texto mi_texto = decimal.texto()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_booleano_a_cadena() {
        let codigo = r#"
log verdad = verdadero
log mentira = falso
texto texto_verdad = verdad.texto()
texto texto_mentira = mentira.texto()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_cadena_a_numero() {
        let codigo = r#"
texto texto_numero = "123"
entero numero = texto_numero.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_cadena_a_decimal() {
        let codigo = r#"
texto texto_decimal = "123.456"
número decimal = texto_decimal.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_entero_a_decimal() {
        let codigo = r#"
entero entero_val = 42
número decimal_val = entero_val.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_decimal_a_entero() {
        let codigo = r#"
número decimal_val = 42.7
entero entero_val = decimal_val.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_numero_negativo() {
        let codigo = r#"
texto texto_negativo = "-123"
entero numero_negativo = texto_negativo.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_decimal_negativo() {
        let codigo = r#"
texto texto_decimal_negativo = "-123.456"
número decimal_negativo = texto_decimal_negativo.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_con_espacios() {
        let codigo = r#"
texto texto_con_espacios = "  123  "
entero numero = texto_con_espacios.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_cero() {
        let codigo = r#"
texto texto_cero = "0"
entero cero = texto_cero.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_decimal_cero() {
        let codigo = r#"
texto texto_decimal_cero = "0.0"
número decimal_cero = texto_decimal_cero.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_en_operaciones() {
        let codigo = r#"
entero numero = 42
texto resultado = "El número es: " + numero.texto()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_en_expresiones() {
        let codigo = r#"
entero a = 10
entero b = 20
texto resultado = (a + b).texto()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_booleano_verdadero() {
        let codigo = r#"
log estado = verdadero
texto texto_estado = estado.texto()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_booleano_falso() {
        let codigo = r#"
log estado = falso
texto texto_estado = estado.texto()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_encadenada() {
        let codigo = r#"
número decimal = 123.456
texto mi_texto = decimal.texto()
entero numero_entero = mi_texto.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_en_funcion() {
        let codigo = r#"
texto formatear_numero(entero num) {
    retornar "Número: " + num.texto()
}
texto resultado = formatear_numero(42)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_error_conversion_texto_invalido() {
        let codigo = r#"
texto texto_invalido = "no_es_numero"
entero numero = texto_invalido.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_error_conversion_cadena_vacia() {
        let codigo = r#"
texto cadena_vacia = ""
entero numero = cadena_vacia.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_error_conversion_caracteres_especiales() {
        let codigo = r#"
texto texto_especial = "123abc"
entero numero = texto_especial.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_error_conversion_multiples_puntos() {
        let codigo = r#"
texto texto_puntos = "12.34.56"
número decimal = texto_puntos.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_conversion_numero_muy_grande() {
        let codigo = r#"
texto numero_grande = "999999999999999999999999999"
entero entero_grande = numero_grande.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_conversion_decimal_precision() {
        let codigo = r#"
texto decimal_precision = "3.141592653589793238462643383279"
número decimal = decimal_precision.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }
}
