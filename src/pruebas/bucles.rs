use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bucle_para_basico() {
        let codigo = r#"
para (entero i = 0; i < 5; i++) {
    imprimir("Iteración: " + i.cadena())
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_bucle_mientras() {
        let codigo = r#"
entero contador = 0
mientras (contador < 3) {
    imprimir("Contador: " + contador.cadena())
    contador += 1
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_bucle_hacer_mientras() {
        let codigo = r#"
entero numero = 1
hacer {
    imprimir("Número: " + numero.cadena())
    numero += 1
} mientras (numero <= 3)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_bucle_foreach_lista() {
        let codigo = r#"
lista numeros = [1, 2, 3, 4, 5]
para (numero en numeros) {
    imprimir("Elemento: " + numero.cadena())
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_bucle_foreach_con_tipo() {
        let codigo = r#"
lista<entero> enteros = [10, 20, 30]
para (entero valor en enteros) {
    imprimir("Valor entero: " + valor.cadena())
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_bucle_anidado() {
        let codigo = r#"
para (entero i = 0; i < 3; i++) {
    para (entero j = 0; j < 2; j++) {
        imprimir("i: " + i.cadena() + ", j: " + j.cadena())
    }
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_control_flujo_romper() {
        let codigo = r#"
para (entero i = 0; i < 10; i++) {
    si (i == 5) {
        romper
    }
    imprimir("Valor: " + i.cadena())
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_control_flujo_continuar() {
        let codigo = r#"
para (entero i = 0; i < 5; i++) {
    si (i == 2) {
        continuar
    }
    imprimir("Valor: " + i.cadena())
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_bucle_para_con_decimales() {
        let codigo = r#"
para (número i = 0.0; i < 2.5; i += 0.5) {
    imprimir("Decimal: " + i.cadena())
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_bucle_mientras_condicion_compleja() {
        let codigo = r#"
entero a = 1
entero b = 10
mientras (a < 5 && b > 5) {
    imprimir("a: " + a.cadena() + ", b: " + b.cadena())
    a += 1
    b -= 1
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_bucle_infinito_con_romper() {
        let codigo = r#"
entero contador = 0
mientras (verdadero) {
    si (contador >= 3) {
        romper
    }
    imprimir("Iteración: " + contador.cadena())
    contador += 1
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }
}
