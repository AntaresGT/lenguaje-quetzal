use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_funciones_encadenadas_basicas() {
        let codigo = r#"
número numero_test = 42
cadena resultado = numero_test.cadena().numero().cadena()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_funciones_encadenadas_con_decimales() {
        let codigo = r#"
número numero_decimal = 3.1416
cadena resultado = numero_decimal.cadena().numero().cadena()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_funciones_encadenadas_en_impresion() {
        let codigo = r#"
número numero_test = 123
imprimir("Resultado: " + numero_test.cadena().numero().cadena())
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_funciones_encadenadas_multiples() {
        let codigo = r#"
número numero1 = 10
número numero2 = 20
imprimir(numero1.cadena() + " y " + numero2.cadena())
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_cadena_a_numero() {
        let codigo = r#"
cadena texto = "123"
número resultado = texto.numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_numero_a_entero() {
        let codigo = r#"
número decimal = 42.7
entero resultado = decimal.entero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_funciones_encadenadas_con_negativos() {
        let codigo = r#"
número negativo = -15.5
cadena resultado = negativo.cadena().numero().cadena()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_funciones_encadenadas_tipo_entero() {
        let codigo = r#"
entero numero_entero = 999
cadena resultado = numero_entero.cadena().entero().numero().cadena()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_error_metodo_inexistente() {
        let codigo = r#"
número numero_test = 42
cadena resultado = numero_test.metodo_inexistente()
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_funciones_encadenadas_largas() {
        let codigo = r#"
número numero_test = 3.14159
cadena resultado = numero_test.cadena().numero().entero().numero().cadena()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_funciones_encadenadas_con_saltos_de_linea() {
        let codigo = r#"
número numero_test = 3.14
imprimir("Resultado: " + numero_test.cadena()
                                    .numero()
                                    .cadena()
        )
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }
}
