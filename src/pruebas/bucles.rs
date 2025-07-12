use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bucle_para_basico() {
        let codigo = r#"
para (entero mut i = 0; i < 5; i = i + 1) {
    consola.imprimir("Iteración: " + i.cadena())
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_bucle_mientras() {
        let codigo = r#"
entero mut contador = 0
mientras (contador < 3) {
    consola.imprimir("Contador: " + contador.cadena())
    contador = contador + 1
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_bucle_hacer_mientras() {
        let codigo = r#"
entero mut valor_numero = 1
hacer {
    consola.imprimir("Número: " + valor_numero.cadena())
    valor_numero = valor_numero + 1
} mientras (valor_numero <= 3)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_bucle_foreach_lista() {
        let codigo = r#"
lista numeros = [1, 2, 3, 4, 5]
para (valor_numero en numeros) {
    consola.imprimir("Elemento: " + valor_numero.cadena())
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_bucle_foreach_con_tipo() {
        let codigo = r#"
lista<entero> enteros = [10, 20, 30]
para (entero valor en enteros) {
    consola.imprimir("Valor entero: " + valor.cadena())
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_bucle_anidado() {
        let codigo = r#"
para (entero mut i = 0; i < 3; i = i + 1) {
    para (entero mut j = 0; j < 2; j = j + 1) {
        consola.imprimir("i: " + i.cadena() + ", j: " + j.cadena())
    }
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_control_flujo_romper() {
        let codigo = r#"
para (entero mut i = 0; i < 10; i = i + 1) {
    si (i == 5) {
        romper
    }
    consola.imprimir("Valor: " + i.cadena())
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_control_flujo_continuar() {
        let codigo = r#"
para (entero mut i = 0; i < 5; i = i + 1) {
    si (i == 2) {
        continuar
    }
    consola.imprimir("Valor: " + i.cadena())
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_bucle_para_con_decimales() {
        let codigo = r#"
para (número mut i = 0.0; i < 2.5; i = i + 0.5) {
    consola.imprimir("Decimal: " + i.cadena())
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_bucle_mientras_condicion_compleja() {
        let codigo = r#"
entero mut a = 1
entero mut b = 10
mientras ((a < 5) && (b > 5)) {
    consola.imprimir("a: " + a.cadena() + ", b: " + b.cadena())
    a = a + 1
    b = b - 1
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_bucle_infinito_con_romper() {
        let codigo = r#"
entero mut contador = 0
mientras (verdadero) {
    si (contador >= 3) {
        romper
    }
    contador = contador + 1
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }
}
