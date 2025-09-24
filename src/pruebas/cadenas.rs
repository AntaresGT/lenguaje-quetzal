use crate::nucleo::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concatenacion_basica() {
        let codigo = r#"
texto saludo = "Hola"
texto nombre = "Mundo"
texto mensaje = saludo + " " + nombre
imprimir(mensaje)
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_concatenacion_con_numeros() {
        let codigo = r#"
entero valor_numero = 42
texto mi_texto = "El número es: " + valor_numero.texto()
consola.imprimir(mi_texto)
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_concatenacion_multiple() {
        let codigo = r#"
texto parte1 = "Primera"
texto parte2 = "Segunda"
texto parte3 = "Tercera"
texto resultado = parte1 + " - " + parte2 + " - " + parte3
imprimir(resultado)
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_concatenacion_con_expresiones() {
        let codigo = r#"
entero a = 10
entero b = 5
texto resultado = "La suma de " + a.texto() + " y " + b.texto() + " es " + (a + b).texto()
imprimir(resultado)
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_interpolacion_simple() {
        let codigo = r#"
texto nombre = "Juan"
entero edad = 25
texto mensaje = "Hola, soy " + nombre + " y tengo " + edad.texto() + " años"
imprimir(mensaje)
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_cadenas_con_diferentes_tipos() {
        let codigo = r#"
entero entero_val = 100
número decimal_val = 3.14159
log booleano_val = verdadero
texto resultado = "Entero: " + entero_val.texto() + 
                   ", Decimal: " + decimal_val.texto() + 
                   ", Booleano: " + booleano_val.texto()
imprimir(resultado)
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_cadenas_vacias() {
        let codigo = r#"
texto vacia = ""
texto resultado = "Inicio" + vacia + "Final"
imprimir(resultado)
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_concatenacion_en_impresion() {
        let codigo = r#"
texto nombre = "Quetzal"
número version = 0.1
imprimir("Lenguaje: " + nombre + ", Versión: " + version.texto())
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_cadenas_con_espacios() {
        let codigo = r#"
texto palabra1 = "Hola"
texto palabra2 = "mundo"
texto con_espacios = palabra1 + " " + palabra2 + " desde Quetzal"
imprimir(con_espacios)
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_cadenas_largas() {
        let codigo = r#"
texto parte1 = "Esta es una texto muy larga que se compone"
texto parte2 = "de múltiples partes concatenadas para formar"
texto parte3 = "un mensaje completo y coherente"
texto mensaje_completo = parte1 + " " + parte2 + " " + parte3
imprimir(mensaje_completo)
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_concatenacion_con_operaciones() {
        let codigo = r#"
entero base = 10
entero exponente = 2
texto resultado = base.texto() + " elevado a " + exponente.texto() + 
                   " es igual a " + (base * base).texto()
imprimir(resultado)
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }
}
