use crate::nucleo::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operadores_aritmeticos_basicos() {
        let codigo = r#"
entero a = 10
entero b = 5
entero suma = a + b
entero resta = a - b
entero multiplicacion = a * b
entero division = a / b
entero modulo = a % b
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_operadores_con_numeros_decimales() {
        let codigo = r#"
número a = 10.5
número b = 2.5
número suma = a + b
número resta = a - b
número multiplicacion = a * b
número division = a / b
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_operadores_mixtos_entero_numero() {
        let codigo = r#"
entero entero_val = 10
número decimal_val = 3.5
número resultado = entero_val + decimal_val
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_operadores_comparacion() {
        let codigo = r#"
entero a = 10
entero b = 5
log mayor = a > b
log menor = a < b
log mayor_igual = a >= b
log menor_igual = a <= b
log igual = a == b
log diferente = a != b
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_operadores_logicos() {
        let codigo = r#"
log verdad = verdadero
log mentira = falso
log y_logico = verdad && mentira
log o_logico = verdad || mentira
log y_espanol = verdad y mentira
log o_espanol = verdad o mentira
log negacion = !verdad
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_asignacion_compuesta() {
        let codigo = r#"
entero numero = 10
numero += 5
numero -= 3
numero *= 2
numero /= 4
numero %= 3
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_asignacion_compuesta_decimales() {
        let codigo = r#"
número decimal = 10.5
decimal += 2.5
decimal -= 1.0
decimal *= 3.0
decimal /= 2.0
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_asignacion_compuesta_cadenas() {
        let codigo = r#"
texto var mi_texto = "Hola"
mi_texto += " mundo"
mi_texto += "!"
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_operador_ternario() {
        let codigo = r#"
entero a = 10
entero b = 5
texto resultado = a > b ? "a es mayor" : "b es mayor"
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_precedencia_operadores() {
        let codigo = r#"
entero resultado1 = 2 + 3 * 4
entero resultado2 = (2 + 3) * 4
número resultado3 = 10.0 / 2.0 + 3.0
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_error_division_por_cero() {
        let codigo = r#"
entero numero = 10
numero /= 0
        "#;

        assert!(interprete::interpretar(codigo).is_err());
    }
}
