use crate::nucleo::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_declaracion_variables_basicas() {
        let codigo = r#"
entero numero_entero = 42
número numero_decimal = 3.14
texto mi_texto = "Hola mundo"
log verdad = verdadero
log mentira = falso
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_declaracion_variables_por_defecto() {
        let codigo = r#"
entero numero_vacio
número decimal_vacio
texto texto_vacio
log booleano_vacio
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_declaracion_variables_mutables() {
        let codigo = r#"
entero var numero_mutable = 10
número var decimal_mutable = 5.5
texto var texto_mutable = "Variable"
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_tipos_de_datos_validos() {
        let codigo = r#"
entero entero_positivo = 100
entero entero_negativo = -50
número numero_positivo = 123.456
número numero_negativo = -789.012
texto cadena_simple = "Texto simple"
texto cadena_vacia = ""
log verdadero_explicito = verdadero
log falso_explicito = falso
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
