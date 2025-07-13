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

    // ========== TESTS PARA ÁMBITO DE VARIABLES EN BUCLES ==========
    
    #[test]
    fn test_ambito_variables_bucle_para_mismo_nombre() {
        // Este test verifica que se puede usar el mismo nombre de variable
        // en diferentes iteraciones del mismo bucle sin conflictos de ámbito
        let codigo = r#"
para (entero mut i = 0; i < 3; i = i + 1) {
    cadena palabra = "test_" + i.cadena()
    consola.imprimir(palabra)
}
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok(), "Error en ámbito de variables en bucle para: {:?}", resultado);
    }

    #[test]
    fn test_ambito_variables_bucles_anidados_mismo_nombre() {
        // Test para verificar que variables con el mismo nombre en bucles anidados
        // no causan conflictos de ámbito
        let codigo = r#"
para (entero mut i = 0; i < 2; i = i + 1) {
    cadena palabra = "exterior_" + i.cadena()
    consola.imprimir(palabra)
    
    para (entero mut j = 0; j < 2; j = j + 1) {
        cadena palabra = "interior_" + j.cadena()
        consola.imprimir(palabra)
    }
}
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok(), "Error en ámbito de variables en bucles anidados: {:?}", resultado);
    }

    #[test]
    fn test_ambito_variables_funciones_con_bucles() {
        // Test que verifica el problema original reportado: 
        // variables con el mismo nombre en funciones diferentes que contienen bucles
        let codigo = r#"
vacio funcion1() {
    para (entero mut i = 0; i < 2; i = i + 1) {
        cadena palabra = "funcion1_" + i.cadena()
        consola.imprimir(palabra)
    }
}

vacio funcion2() {
    para (entero mut j = 0; j < 2; j = j + 1) {
        cadena palabra = "funcion2_" + j.cadena()
        consola.imprimir(palabra)
    }
}

funcion1()
funcion2()
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok(), "Error en ámbito de variables entre funciones con bucles: {:?}", resultado);
    }

    #[test]
    fn test_ambito_variables_bucle_para_cada_mismo_nombre() {
        // Test para bucles para_cada (foreach) con variables del mismo nombre
        let codigo = r#"
lista numeros1 = [1, 2]
lista numeros2 = [3, 4]

para (elemento en numeros1) {
    cadena mensaje = "lista1: " + elemento.cadena()
    consola.imprimir(mensaje)
}

para (elemento en numeros2) {
    cadena mensaje = "lista2: " + elemento.cadena()
    consola.imprimir(mensaje)
}
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok(), "Error en ámbito de variables en bucles para_cada: {:?}", resultado);
    }

    #[test]
    fn test_ambito_variables_multiples_bucles_consecutivos() {
        // Test con múltiples bucles consecutivos usando variables con el mismo nombre
        let codigo = r#"
// Primer bucle
para (entero mut i = 0; i < 2; i = i + 1) {
    cadena resultado = "primer_" + i.cadena()
    consola.imprimir(resultado)
}

// Segundo bucle
para (entero mut k = 0; k < 2; k = k + 1) {
    cadena resultado = "segundo_" + k.cadena()
    consola.imprimir(resultado)
}

// Tercer bucle
para (entero mut m = 0; m < 2; m = m + 1) {
    cadena resultado = "tercer_" + m.cadena()
    consola.imprimir(resultado)
}
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok(), "Error en ámbito de variables en bucles consecutivos: {:?}", resultado);
    }

    #[test]
    fn test_ambito_variables_bucle_dentro_funcion_con_parametros() {
        // Test más complejo: función con parámetros que contiene bucle con variables locales
        let codigo = r#"
vacio procesar_lista(lista elementos) {
    para (entero mut i = 0; i < elementos.longitud(); i = i + 1) {
        cadena elemento_actual = elementos[i].cadena()
        cadena mensaje = "Procesando: " + elemento_actual
        consola.imprimir(mensaje)
    }
}

lista datos1 = ["a", "b"]
lista datos2 = ["x", "y"]

procesar_lista(datos1)
procesar_lista(datos2)
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok(), "Error en ámbito de variables en función con bucle y parámetros: {:?}", resultado);
    }

    #[test]
    fn test_ambito_variables_bucle_tres_niveles_anidacion() {
        // Test con tres niveles de anidación de bucles
        let codigo = r#"
para (entero mut i = 0; i < 2; i = i + 1) {
    cadena nivel1 = "nivel1_" + i.cadena()
    
    para (entero mut j = 0; j < 2; j = j + 1) {
        cadena nivel2 = "nivel2_" + j.cadena()
        
        para (entero mut k = 0; k < 2; k = k + 1) {
            cadena nivel3 = "nivel3_" + k.cadena()
            cadena combinado = nivel1 + "_" + nivel2 + "_" + nivel3
            consola.imprimir(combinado)
        }
    }
}
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok(), "Error en ámbito de variables en bucles con tres niveles de anidación: {:?}", resultado);
    }

    #[test]
    fn test_ambito_variables_bucle_con_shadowing() {
        // Test para verificar que el shadowing (ocultación de variables) funciona correctamente
        let codigo = r#"
cadena variable_global = "global"

para (entero mut i = 0; i < 2; i = i + 1) {
    cadena variable_global = "local_" + i.cadena()
    consola.imprimir("Dentro del bucle: " + variable_global)
}

consola.imprimir("Fuera del bucle: " + variable_global)
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok(), "Error en shadowing de variables en bucles: {:?}", resultado);
    }
}
