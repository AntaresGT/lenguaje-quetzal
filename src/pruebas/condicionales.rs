use crate::nucleo::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condicional_simple() {
        let codigo = r#"
entero numero = 10
si (numero > 5) {
    consola.imprimir("Número es mayor que 5")
}
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_condicional_si_sino() {
        let codigo = r#"
entero numero = 3
si (numero > 5) {
    consola.imprimir("Mayor que 5")
} sino {
    consola.imprimir("Menor o igual que 5")
}
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_condicional_si_sino_si() {
        let codigo = r#"
entero numero = 10
si (numero < 5) {
    consola.imprimir("Menor que 5")
} sino si (numero == 10) {
    consola.imprimir("Igual a 10")
} sino {
    consola.imprimir("Otro caso")
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
        consola.imprimir("a es mayor que 8 y mayor que b")
    } sino {
        consola.imprimir("a es mayor que b pero menor o igual que 8")
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
    consola.imprimir("Condición compleja verdadera")
}
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_condiciones_con_operadores_espanol() {
        let codigo = r#"
log verdad = verdadero
log mentira = falso
si (verdad y !mentira) {
    consola.imprimir("Condición en español verdadera")
}
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_comparacion_cadenas() {
        let codigo = r#"
texto texto1 = "Hola"
texto texto2 = "Hola"
si (texto1 == texto2) {
    consola.imprimir("Las cadenas son iguales")
}
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_comparacion_booleanos() {
        let codigo = r#"
log valor1 = verdadero
log valor2 = falso
si (valor1 != valor2) {
    consola.imprimir("Los valores booleanos son diferentes")
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
    consola.imprimir("Valores numéricos iguales")
}
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }
}
