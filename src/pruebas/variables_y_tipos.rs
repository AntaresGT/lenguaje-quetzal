use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_declaracion_variables_basicas() {
        let codigo = r#"
entero numero_entero = 42
número numero_decimal = 3.14
cadena texto = "Hola mundo"
bool verdad = verdadero
bool mentira = falso
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_declaracion_variables_por_defecto() {
        let codigo = r#"
entero numero_vacio
número decimal_vacio
cadena texto_vacio
bool booleano_vacio
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_declaracion_variables_mutables() {
        let codigo = r#"
entero mut numero_mutable = 10
número mut decimal_mutable = 5.5
cadena mut texto_mutable = "Variable"
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_tipos_de_datos_validos() {
        let codigo = r#"
vacio variable_vacia
entero entero_positivo = 100
entero entero_negativo = -50
número numero_positivo = 123.456
número numero_negativo = -789.012
cadena cadena_simple = "Texto simple"
cadena cadena_vacia = ""
bool verdadero_explicito = verdadero
bool falso_explicito = falso
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_error_palabra_reservada() {
        let codigo = r#"
entero entero = 5
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_error_nombre_variable_invalido() {
        let codigo = r#"
entero 123variable = 5
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_nombres_variables_validos() {
        let codigo = r#"
entero variable_normal = 1
entero _variable_con_guion = 2
entero variableCamelCase = 3
entero variable123 = 4
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }
}
