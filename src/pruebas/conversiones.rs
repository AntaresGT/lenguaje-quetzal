use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversion_entero_a_cadena() {
        let codigo = r#"
entero numero = 42
cadena texto = numero.cadena()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_decimal_a_cadena() {
        let codigo = r#"
número decimal = 3.14159
cadena texto = decimal.cadena()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_booleano_a_cadena() {
        let codigo = r#"
bool verdad = verdadero
bool mentira = falso
cadena texto_verdad = verdad.cadena()
cadena texto_mentira = mentira.cadena()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_cadena_a_numero() {
        let codigo = r#"
cadena texto_numero = "123"
entero numero = texto_numero.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_cadena_a_decimal() {
        let codigo = r#"
cadena texto_decimal = "123.456"
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
cadena texto_negativo = "-123"
entero numero_negativo = texto_negativo.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_decimal_negativo() {
        let codigo = r#"
cadena texto_decimal_negativo = "-123.456"
número decimal_negativo = texto_decimal_negativo.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_con_espacios() {
        let codigo = r#"
cadena texto_con_espacios = "  123  "
entero numero = texto_con_espacios.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_cero() {
        let codigo = r#"
cadena texto_cero = "0"
entero cero = texto_cero.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_decimal_cero() {
        let codigo = r#"
cadena texto_decimal_cero = "0.0"
número decimal_cero = texto_decimal_cero.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_en_operaciones() {
        let codigo = r#"
entero numero = 42
cadena resultado = "El número es: " + numero.cadena()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_en_expresiones() {
        let codigo = r#"
entero a = 10
entero b = 20
cadena resultado = (a + b).cadena()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_booleano_verdadero() {
        let codigo = r#"
bool estado = verdadero
cadena texto_estado = estado.cadena()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_booleano_falso() {
        let codigo = r#"
bool estado = falso
cadena texto_estado = estado.cadena()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_encadenada() {
        let codigo = r#"
número decimal = 123.456
cadena texto = decimal.cadena()
entero numero = texto.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_en_funcion() {
        let codigo = r#"
cadena formatear_numero(entero num) {
    retornar "Número: " + num.cadena()
}
cadena resultado = formatear_numero(42)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_error_conversion_texto_invalido() {
        let codigo = r#"
cadena texto_invalido = "no_es_numero"
entero numero = texto_invalido.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_error_conversion_cadena_vacia() {
        let codigo = r#"
cadena cadena_vacia = ""
entero numero = cadena_vacia.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_error_conversion_caracteres_especiales() {
        let codigo = r#"
cadena texto_especial = "123abc"
entero numero = texto_especial.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_error_conversion_multiples_puntos() {
        let codigo = r#"
cadena texto_puntos = "12.34.56"
número decimal = texto_puntos.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_conversion_numero_muy_grande() {
        let codigo = r#"
cadena numero_grande = "999999999999999999999999999"
entero numero = numero_grande.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_conversion_decimal_precision() {
        let codigo = r#"
cadena decimal_precision = "3.141592653589793238462643383279"
número decimal = decimal_precision.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }
}
