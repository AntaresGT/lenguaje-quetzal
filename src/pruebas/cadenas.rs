use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concatenacion_basica() {
        let codigo = r#"
cadena saludo = "Hola"
cadena nombre = "Mundo"
cadena mensaje = saludo + " " + nombre
imprimir(mensaje)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_concatenacion_con_numeros() {
        let codigo = r#"
entero valor_numero = 42
cadena texto = "El número es: " + valor_numero.cadena()
imprimir(texto)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_concatenacion_multiple() {
        let codigo = r#"
cadena parte1 = "Primera"
cadena parte2 = "Segunda"
cadena parte3 = "Tercera"
cadena resultado = parte1 + " - " + parte2 + " - " + parte3
imprimir(resultado)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_concatenacion_con_expresiones() {
        let codigo = r#"
entero a = 10
entero b = 5
cadena resultado = "La suma de " + a.cadena() + " y " + b.cadena() + " es " + (a + b).cadena()
imprimir(resultado)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_interpolacion_simple() {
        let codigo = r#"
cadena nombre = "Juan"
entero edad = 25
cadena mensaje = "Hola, soy " + nombre + " y tengo " + edad.cadena() + " años"
imprimir(mensaje)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_cadenas_con_diferentes_tipos() {
        let codigo = r#"
entero entero_val = 100
número decimal_val = 3.14159
bool booleano_val = verdadero
cadena resultado = "Entero: " + entero_val.cadena() + 
                   ", Decimal: " + decimal_val.cadena() + 
                   ", Booleano: " + booleano_val.cadena()
imprimir(resultado)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_cadenas_vacias() {
        let codigo = r#"
cadena vacia = ""
cadena resultado = "Inicio" + vacia + "Final"
imprimir(resultado)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_concatenacion_en_impresion() {
        let codigo = r#"
cadena nombre = "Quetzal"
número version = 0.1
imprimir("Lenguaje: " + nombre + ", Versión: " + version.cadena())
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_cadenas_con_espacios() {
        let codigo = r#"
cadena palabra1 = "Hola"
cadena palabra2 = "mundo"
cadena con_espacios = palabra1 + " " + palabra2 + " desde Quetzal"
imprimir(con_espacios)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_cadenas_largas() {
        let codigo = r#"
cadena parte1 = "Esta es una cadena muy larga que se compone"
cadena parte2 = "de múltiples partes concatenadas para formar"
cadena parte3 = "un mensaje completo y coherente"
cadena mensaje_completo = parte1 + " " + parte2 + " " + parte3
imprimir(mensaje_completo)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_concatenacion_con_operaciones() {
        let codigo = r#"
entero base = 10
entero exponente = 2
cadena resultado = base.cadena() + " elevado a " + exponente.cadena() + 
                   " es igual a " + (base * base).cadena()
imprimir(resultado)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }
}
