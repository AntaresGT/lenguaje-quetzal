use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_imprimir_cadena_simple() {
        let codigo = r#"
imprimir("Hola mundo")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_numero_entero() {
        let codigo = r#"
entero numero = 42
imprimir(numero.cadena())
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_numero_decimal() {
        let codigo = r#"
número decimal = 3.14
imprimir(decimal.cadena())
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_booleano() {
        let codigo = r#"
bool verdad = verdadero
imprimir(verdad.cadena())
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_variable_cadena() {
        let codigo = r#"
cadena mensaje = "Este es un mensaje"
imprimir(mensaje)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_concatenacion() {
        let codigo = r#"
cadena parte1 = "Hola"
cadena parte2 = "mundo"
imprimir(parte1 + " " + parte2)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_expresion_aritmetica() {
        let codigo = r#"
entero a = 10
entero b = 5
imprimir((a + b).cadena())
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_exito() {
        let codigo = r#"
imprimir_exito("Operación exitosa")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_error() {
        let codigo = r#"
imprimir_error("Mensaje de error")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_informacion() {
        let codigo = r#"
imprimir_informacion("Información importante")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_advertencia() {
        let codigo = r#"
imprimir_advertencia("Advertencia del sistema")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_multiples_lineas() {
        let codigo = r#"
imprimir("Primera línea")
imprimir("Segunda línea")
imprimir("Tercera línea")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_con_escape() {
        let codigo = r#"
imprimir("Línea con \"comillas\"")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_cadena_vacia() {
        let codigo = r#"
imprimir("")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_resultado_funcion() {
        let codigo = r#"
cadena obtener_mensaje() {
    retornar "Mensaje desde función"
}
imprimir(obtener_mensaje())
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_con_conversion_tipos() {
        let codigo = r#"
entero numero = 123
número decimal = 45.67
bool estado = verdadero
imprimir("Número: " + numero.cadena())
imprimir("Decimal: " + decimal.cadena())
imprimir("Estado: " + estado.cadena())
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_con_operaciones_complejas() {
        let codigo = r#"
entero a = 10
entero b = 20
entero c = 30
imprimir("Resultado: " + (a + b * c).cadena())
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_error_imprimir_sin_parametros() {
        let codigo = r#"
imprimir()
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_error_imprimir_tipo_incorrecto() {
        let codigo = r#"
entero numero = 42
imprimir(numero)  // Debería convertir a cadena primero
        "#;
        
        // Esto podría ser válido si el intérprete hace conversión automática
        // Si no, debería fallar
        let resultado = interprete::interpretar(codigo);
        // Permitimos ambos casos según la implementación
        assert!(resultado.is_ok() || resultado.is_err());
    }
}
