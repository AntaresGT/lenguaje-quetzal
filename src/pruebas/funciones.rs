use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_declaracion_funcion_sin_parametros() {
        let codigo = r#"
vacio saludar() {
    entero x = 5
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_declaracion_funcion_con_parametros() {
        let codigo = r#"
entero sumar(entero a, entero b) {
    retornar a + b
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_llamada_funcion_sin_parametros() {
        let codigo = r#"
vacio saludar() {
    entero x = 10
}
saludar()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_llamada_funcion_con_parametros() {
        let codigo = r#"
entero duplicar(entero valor) {
    retornar valor * 2
}
entero resultado = duplicar(5)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_funcion_retorno_entero() {
        let codigo = r#"
entero obtener_numero() {
    retornar 42
}
entero numero_valor = obtener_numero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_funcion_retorno_cadena() {
        let codigo = r#"
cadena obtener_saludo() {
    retornar "Hola mundo"
}
cadena saludo = obtener_saludo()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_funcion_retorno_booleano() {
        let codigo = r#"
bool es_verdadero() {
    retornar verdadero
}
bool resultado = es_verdadero()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_funcion_con_multiples_parametros() {
        let codigo = r#"
cadena concatenar(cadena a, cadena b, cadena c) {
    retornar a + b + c
}
cadena resultado = concatenar("Hola", " ", "mundo")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_funcion_con_operaciones_aritmeticas() {
        let codigo = r#"
entero calcular(entero a, entero b) {
    entero suma = a + b
    entero producto = suma * 2
    retornar producto
}
entero resultado = calcular(5, 3)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_funcion_con_condicionales() {
        let codigo = r#"
entero mayor(entero a, entero b) {
    si (a > b) {
        retornar a
    } sino {
        retornar b
    }
}
entero resultado = mayor(10, 5)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_funcion_recursiva_simple() {
        let codigo = r#"
entero factorial(entero n) {
    si (n <= 1) {
        retornar 1
    } sino {
        retornar n * factorial(n - 1)
    }
}
entero resultado = factorial(3)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_funcion_con_variables_locales() {
        let codigo = r#"
entero procesar_datos(entero entrada) {
    entero temporal = entrada * 2
    entero resultado_parcial = temporal + 5
    entero resultado_final = resultado_parcial / 2
    retornar resultado_final
}
entero resultado = procesar_datos(10)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_llamadas_funciones_anidadas() {
        let codigo = r#"
entero duplicar(entero x) {
    retornar x * 2
}
entero triplicar(entero x) {
    retornar x * 3
}
entero resultado = duplicar(triplicar(5))
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_funcion_con_tipos_mixtos() {
        let codigo = r#"
cadena formatear(entero numero, cadena texto) {
    retornar "Número: " + numero.cadena() + ", Texto: " + texto
}
cadena resultado = formatear(42, "prueba")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_error_funcion_sin_retorno() {
        let codigo = r#"
entero funcion_problematica() {
    entero x = 5
    // No hay retorno y debería fallar
}
        "#;
        
        // Esta prueba debe fallar porque la función no retorna
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_error_llamada_funcion_inexistente() {
        let codigo = r#"
entero resultado = funcion_que_no_existe(5)
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_error_parametros_incorrectos() {
        let codigo = r#"
entero sumar(entero a, entero b) {
    retornar a + b
}
entero resultado = sumar(5)  // Faltan parámetros
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_funcion_vacio_sin_retorno() {
        let codigo = r#"
vacio hacer_algo() {
    entero x = 5
    entero y = x * 2
}
hacer_algo()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_alcance_variables_funciones() {
        let codigo = r#"
entero variable_global = 100

entero usar_variable_global() {
    retornar variable_global + 10
}

entero resultado = usar_variable_global()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }
}
