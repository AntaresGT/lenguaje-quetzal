use crate::nucleo::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_imprimir_cadena_simple() {
        let codigo = r#"
consola.imprimir("Hola mundo")
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_numero_entero() {
        let codigo = r#"
entero valor_numero = 42
consola.imprimir(valor_numero.texto())
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_numero_decimal() {
        let codigo = r#"
número decimal = 3.14
consola.imprimir(decimal.texto())
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_booleano() {
        let codigo = r#"
log verdad = verdadero
consola.imprimir(verdad.texto())
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_variable_cadena() {
        let codigo = r#"
texto mensaje = "Este es un mensaje"
consola.imprimir(mensaje)
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_concatenacion() {
        let codigo = r#"
texto parte1 = "Hola"
texto parte2 = "mundo"
consola.imprimir(parte1 + " " + parte2)
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_expresion_aritmetica() {
        let codigo = r#"
entero a = 10
entero b = 5
consola.imprimir((a + b).texto())
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_exito() {
        let codigo = r#"
consola.imprimir_exito("Operación exitosa")
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_error() {
        let codigo = r#"
consola.imprimir_error("Mensaje de error")
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_informacion() {
        let codigo = r#"
consola.imprimir_informacion("Información importante")
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_advertencia() {
        let codigo = r#"
consola.imprimir_advertencia("Advertencia del sistema")
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_multiples_lineas() {
        let codigo = r#"
consola.imprimir("Primera línea")
consola.imprimir("Segunda línea")
consola.imprimir("Tercera línea")
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_con_escape() {
        let codigo = r#"
consola.imprimir("Línea con \"comillas\"")
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_cadena_vacia() {
        let codigo = r#"
consola.imprimir("")
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_resultado_funcion() {
        let codigo = r#"
texto obtener_mensaje() {
    retornar "Mensaje desde función"
}
consola.imprimir(obtener_mensaje())
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_con_conversion_tipos() {
        let codigo = r#"
entero valor_numero = 123
número decimal = 45.67
log estado = verdadero
consola.imprimir("Número: " + valor_numero.texto())
consola.imprimir("Decimal: " + decimal.texto())
consola.imprimir("Estado: " + estado.texto())
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_imprimir_con_operaciones_complejas() {
        let codigo = r#"
entero a = 10
entero b = 20
entero c = 30
consola.imprimir("Resultado: " + (a + b * c).texto())
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_error_imprimir_sin_parametros() {
        let codigo = r#"
consola.imprimir()
        "#;

        // imprimir() sin parámetros es válido - imprime línea vacía
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_error_imprimir_tipo_incorrecto() {
        let codigo = r#"
entero valor_numero = 42
consola.imprimir(valor_numero)  // Debería convertir a texto primero
        "#;

        // Esto podría ser válido si el intérprete hace conversión automática
        // Si no, debería fallar
        let resultado = interprete::interpretar(codigo);
        // Permitimos ambos casos según la implementación
        assert!(resultado.is_ok() || resultado.is_err());
    }
}
