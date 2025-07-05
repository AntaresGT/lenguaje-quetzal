use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condicional_simple() {
        let codigo = r#"
entero numero = 10
si (numero > 5) {
    imprimir("Número es mayor que 5")
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_condicional_si_sino() {
        let codigo = r#"
entero numero = 3
si (numero > 5) {
    imprimir("Mayor que 5")
} sino {
    imprimir("Menor o igual que 5")
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_condicional_si_sino_si() {
        let codigo = r#"
entero numero = 10
si (numero < 5) {
    imprimir("Menor que 5")
} sino si (numero == 10) {
    imprimir("Igual a 10")
} sino {
    imprimir("Otro caso")
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_condicionales_anidados() {
        let codigo = r#"
entero a = 10
entero b = 5
si (a > b) {
    si (a > 8) {
        imprimir("a es mayor que 8 y mayor que b")
    } sino {
        imprimir("a es mayor que b pero menor o igual que 8")
    }
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_condiciones_complejas() {
        let codigo = r#"
entero a = 10
entero b = 5
entero c = 15
si ((a > b) && (c > a)) {
    imprimir("Condición compleja verdadera")
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_condiciones_con_operadores_espanol() {
        let codigo = r#"
bool verdad = verdadero
bool mentira = falso
si (verdad y !mentira) {
    imprimir("Condición en español verdadera")
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_comparacion_cadenas() {
        let codigo = r#"
cadena texto1 = "Hola"
cadena texto2 = "Hola"
si (texto1 == texto2) {
    imprimir("Las cadenas son iguales")
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_comparacion_booleanos() {
        let codigo = r#"
bool valor1 = verdadero
bool valor2 = falso
si (valor1 != valor2) {
    imprimir("Los valores booleanos son diferentes")
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_comparacion_mixta_tipos_numericos() {
        let codigo = r#"
entero entero_val = 10
número decimal_val = 10.0
si (entero_val == decimal_val) {
    imprimir("Valores numéricos iguales")
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }
}
