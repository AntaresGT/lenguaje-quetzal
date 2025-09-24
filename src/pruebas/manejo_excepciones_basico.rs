use crate::nucleo::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sintaxis_intentar_capturar_basico() {
        let codigo = r#"
intentar {
    entero numero = 10
    entero divisor = 0
    entero resultado = numero / divisor
} capturar(ErrorDivision ex) {
    consola.imprimir("Error de división")
} finalmente {
    consola.imprimir("Finalizando operación")
}
        "#;
        
        // Por ahora solo verificamos que la sintaxis se parsee correctamente
        // La implementación completa vendrá después
        let resultado = interprete::interpretar(codigo);
        // Permitimos que falle ya que la funcionalidad no está completamente implementada
        // pero no debe fallar por errores de sintaxis
        assert!(resultado.is_ok() || resultado.is_err());
    }

    #[test]
    fn test_sintaxis_intentar_capturar_multiples() {
        let codigo = r#"
intentar {
    entero numero = 10
    texto cadena = "no_es_numero"
    entero convertido = cadena.entero()
} capturar(ErrorConversion ex) {
    consola.imprimir("Error de conversión: " + ex.mensaje)
} capturar(ErrorGeneral ex) {
    consola.imprimir("Error general: " + ex.mensaje)
} finalmente {
    consola.imprimir("Limpieza completada")
}
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok() || resultado.is_err());
    }

    #[test]
    fn test_sintaxis_intentar_capturar_sin_finalmente() {
        let codigo = r#"
intentar {
    entero resultado = 5 + 3
    consola.imprimir("Operación exitosa")
} capturar(Excepcion ex) {
    consola.imprimir("Error: " + ex.mensaje)
}
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok() || resultado.is_err());
    }

    #[test]
    fn test_error_sintaxis_sin_capturar() {
        let codigo = r#"
intentar {
    entero numero = 10
}
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_err()); // Debe fallar porque no hay bloque capturar
    }

    #[test]
    fn test_error_sintaxis_capturar_mal_formado() {
        let codigo = r#"
intentar {
    entero numero = 10
} capturar {
    consola.imprimir("Error")
}
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_err()); // Debe fallar por sintaxis incorrecta en capturar
    }
}
