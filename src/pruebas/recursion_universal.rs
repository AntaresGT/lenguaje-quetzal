//! Pruebas para la gestión universal de recursión en Quetzal
//!
//! Estas pruebas verifican que CUALQUIER función puede ser recursiva infinita
//! sin restricciones de nombres o palabras reservadas.

use crate::nucleo::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recursion_universal_cualquier_nombre() {
        let codigo = r#"
entero var resultado = 0

vacio cualquier_nombre() {
    resultado = resultado + 1
    si (resultado < 10) {
        cualquier_nombre()
    }
}

cualquier_nombre()
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_recursion_profunda_universal() {
        let codigo = r#"
entero var profundidad = 0

vacio ir_profundo() {
    profundidad = profundidad + 1
    si (profundidad < 100) {
        ir_profundo()
    }
}

ir_profundo()
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_multiples_funciones_recursivas_simultaneas() {
        let codigo = r#"
entero var contador_a = 0
entero var contador_b = 0

vacio funcion_a() {
    contador_a = contador_a + 1
    si (contador_a < 25) {
        funcion_b()
    }
}

vacio funcion_b() {
    contador_b = contador_b + 1
    si (contador_b < 25) {
        funcion_a()
    }
}

funcion_a()
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_expansion_automatica_limites() {
        let codigo = r#"
entero var niveles = 0

vacio expandir_limites() {
    niveles = niveles + 1
    si (niveles < 150) {
        expandir_limites()
    }
}

expandir_limites()
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_funciones_sin_restricciones_nombres() {
        let codigo = r#"
entero var resultado = 0

vacio cualquier_nombre_que_quiera() {
    resultado = resultado + 1
    si (resultado < 3) {
        cualquier_nombre_que_quiera()
    }
}

vacio otro_nombre_completamente_diferente() {
    resultado = resultado + 10
    si (resultado < 30) {
        otro_nombre_completamente_diferente()
    }
}

cualquier_nombre_que_quiera()
otro_nombre_completamente_diferente()
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }
}
