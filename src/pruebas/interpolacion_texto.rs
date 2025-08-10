// Pruebas unitarias para la interpolación de texto t"..."

use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolacion_basica() {
        let codigo = r#"
            entero valor = 42
            texto resultado = t"El valor es {valor}"
            consola.mostrar(resultado)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_interpolacion_con_expresiones() {
        let codigo = r#"
            entero a = 10
            entero b = 5
            texto resultado = t"La suma de {a} + {b} = {a + b}"
            consola.mostrar(resultado)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_interpolacion_multiples_variables() {
        let codigo = r#"
            texto nombre = "Quetzal"
            entero version = 2
            número decimal = 0.2
            texto resultado = t"Lenguaje {nombre} v{version}.{decimal}"
            consola.mostrar(resultado)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_interpolacion_con_booleanos() {
        let codigo = r#"
            log verdadero_valor = verdadero
            log falso_valor = falso
            texto resultado = t"Verdadero: {verdadero_valor}, Falso: {falso_valor}"
            consola.mostrar(resultado)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_interpolacion_con_listas() {
        let codigo = r#"
            lista numeros = [1, 2, 3]
            texto resultado = t"Números: {numeros}"
            consola.mostrar(resultado)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_interpolacion_con_caracteres_especiales() {
        let codigo = r#"
            texto emoji = "🚀"
            texto acento = "ñáéíóú"
            texto resultado = t"Especiales: {emoji} y {acento}"
            consola.mostrar(resultado)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_interpolacion_multilinea() {
        let codigo = r#"
            texto nombre = "Usuario"
            entero edad = 25
            texto resultado = t"Hola {nombre}\nTienes {edad} años\n¡Bienvenido!"
            consola.mostrar(resultado)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_interpolacion_con_operaciones_complejas() {
        let codigo = r#"
            entero base = 5
            entero exponente = 3
            texto resultado = t"Potencia: {base}^{exponente} = {base * base * base}"
            consola.mostrar(resultado)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_interpolacion_solo_texto() {
        let codigo = r#"
            texto resultado = t"Solo texto sin interpolación"
            consola.mostrar(resultado)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_interpolacion_solo_expresiones() {
        let codigo = r#"
            entero a = 7
            entero b = 3
            texto resultado = t"{a}{b}{a + b}"
            consola.mostrar(resultado)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }
}
