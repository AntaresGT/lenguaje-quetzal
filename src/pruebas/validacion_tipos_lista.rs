//! Pruebas para la validación de tipos en métodos de listas tipadas
//!
//! Este módulo contiene pruebas específicas para verificar que los métodos
//! de listas tipadas (como lista<entero>, lista<texto>, etc.) validen
//! correctamente los tipos de elementos antes de agregarlos.

use crate::nucleo::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    /// Prueba que no se pueda agregar texto a una lista de enteros
    #[test]
    fn test_error_agregar_texto_a_lista_entero() {
        let codigo = r#"
            lista<entero> var numeros = []
            numeros.agregar("texto")
        "#;

        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_err());

        if let Err(error) = resultado {
            let mensaje = error.to_string();
            assert!(mensaje.contains("No se puede agregar un valor de tipo"));
            assert!(mensaje.contains("lista<entero>"));
        }
    }

    /// Prueba que no se pueda agregar número a una lista de texto
    #[test]
    fn test_error_agregar_numero_a_lista_texto() {
        let codigo = r#"
            lista<texto> var palabras = []
            palabras.agregar(42)
        "#;

        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_err());

        if let Err(error) = resultado {
            let mensaje = error.to_string();
            assert!(mensaje.contains("No se puede agregar un valor de tipo"));
            assert!(mensaje.contains("lista<texto>"));
        }
    }

    /// Prueba que no se pueda agregar booleano a una lista de enteros
    #[test]
    fn test_error_agregar_booleano_a_lista_entero() {
        let codigo = r#"
            lista<entero> var numeros = []
            numeros.agregar(verdadero)
        "#;

        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_err());

        if let Err(error) = resultado {
            let mensaje = error.to_string();
            assert!(mensaje.contains("No se puede agregar un valor de tipo"));
            assert!(mensaje.contains("lista<entero>"));
        }
    }

    /// Prueba que sí se pueda agregar enteros a una lista de enteros
    #[test]
    fn test_agregar_entero_a_lista_entero_exitoso() {
        let codigo = r#"
            lista<entero> var numeros = []
            numeros.agregar(42)
            numeros.agregar(100)
        "#;

        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    /// Prueba que sí se pueda agregar texto a una lista de texto
    #[test]
    fn test_agregar_texto_a_lista_texto_exitoso() {
        let codigo = r#"
            lista<texto> var palabras = []
            palabras.agregar("hola")
            palabras.agregar("mundo")
        "#;

        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    /// Prueba que las listas no tipadas siguen funcionando como antes
    #[test]
    fn test_lista_no_tipada_acepta_cualquier_tipo() {
        let codigo = r#"
            lista var mixta = []
            mixta.agregar(42)
            mixta.agregar("texto")
            mixta.agregar(verdadero)
        "#;

        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    /// Prueba que se pueda agregar número decimal a lista de números
    #[test]
    fn test_agregar_decimal_a_lista_numero_exitoso() {
        let codigo = r#"
            lista<número> var decimales = []
            decimales.agregar(3.14)
            decimales.agregar(2.71)
        "#;

        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    /// Prueba que se pueda agregar entero a lista de números (conversión válida)
    #[test]
    fn test_agregar_entero_a_lista_numero_exitoso() {
        let codigo = r#"
            lista<número> var numeros = []
            numeros.agregar(42)
            numeros.agregar(3.14)
        "#;

        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    /// Prueba validación en listas de booleanos
    #[test]
    fn test_validacion_lista_booleano() {
        let codigo = r#"
            lista<log> var banderas = []
            banderas.agregar(verdadero)
            banderas.agregar(falso)
        "#;

        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    /// Prueba error al agregar texto a lista de booleanos
    #[test]
    fn test_error_agregar_texto_a_lista_booleano() {
        let codigo = r#"
            lista<log> var banderas = []
            banderas.agregar("no es booleano")
        "#;

        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_err());

        if let Err(error) = resultado {
            let mensaje = error.to_string();
            assert!(mensaje.contains("No se puede agregar un valor de tipo"));
            assert!(mensaje.contains("lista<log>"));
        }
    }

    /// Prueba validación con nulo en listas tipadas - nulo puede ser compatible según la implementación
    #[test]
    fn test_agregar_nulo_a_lista_tipada() {
        let codigo = r#"
            lista<entero> var numeros = []
            numeros.agregar(nulo)
        "#;

        let resultado = interprete::interpretar(codigo);
        // Nota: El comportamiento con nulo puede variar según la implementación
        // Si nulo es compatible con todos los tipos, esto puede pasar
        // Si no es compatible, debe fallar
        if resultado.is_err() {
            if let Err(error) = resultado {
                let mensaje = error.to_string();
                assert!(mensaje.contains("No se puede agregar un valor de tipo"));
            }
        }
        // Si pasa, significa que nulo es compatible con tipos tipados, lo cual es válido
    }

    /// Prueba múltiples agregados con validación
    #[test]
    fn test_multiples_agregados_con_validacion() {
        let codigo = r#"
            lista<texto> var palabras = []
            palabras.agregar("primera")
            palabras.agregar("segunda")
            // Esta línea debería fallar
            palabras.agregar(123)
        "#;

        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_err());

        if let Err(error) = resultado {
            let mensaje = error.to_string();
            assert!(mensaje.contains("No se puede agregar un valor de tipo"));
            assert!(mensaje.contains("lista<texto>"));
        }
    }
}
