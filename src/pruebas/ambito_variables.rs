use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    // ========== TESTS DE ÁMBITO DE VARIABLES (SCOPING) ==========

    #[test]
    fn test_ambito_funcion_basico() {
        // Test básico de ámbito entre funciones
        let codigo = r#"
entero funcion1() {
    entero resultado = 10
    retornar resultado
}

entero funcion2() {
    entero resultado = 20
    retornar resultado
}

entero suma = funcion1() + funcion2()
consola.imprimir("Suma: " + suma.cadena())
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok(), "Error en ámbito básico entre funciones: {:?}", resultado);
    }

    #[test]
    fn test_ambito_funciones_con_bucles_misma_variable() {
        // Test del problema original: mismo nombre de variable en bucles de diferentes funciones
        let codigo = r#"
entero contar_elementos() {
    entero mut total = 0
    para (entero mut i = 0; i < 3; i = i + 1) {
        cadena palabra = "elemento_" + i.cadena()
        total = total + 1
    }
    retornar total
}

cadena procesar_texto() {
    cadena mut resultado = ""
    para (entero mut j = 1; j <= 2; j = j + 1) {
        cadena palabra = "texto_" + j.cadena()
        resultado = resultado + palabra + " "
    }
    retornar resultado.recortar()
}

entero conteo = contar_elementos()
cadena texto = procesar_texto()
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok(), "Error en ámbito de variables entre funciones con bucles: {:?}", resultado);
    }

    #[test]
    fn test_ambito_bucle_variables_iteracion() {
        // Test que verifica que cada iteración tiene su propio ámbito
        let codigo = r#"
para (entero mut iteracion = 0; iteracion < 4; iteracion = iteracion + 1) {
    cadena mensaje_iteracion = "Iteración número: " + iteracion.cadena()
    entero valor_local = iteracion * 2
    cadena resultado_local = mensaje_iteracion + ", valor: " + valor_local.cadena()
    consola.imprimir(resultado_local)
}
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok(), "Error en ámbito de variables por iteración: {:?}", resultado);
    }

    #[test]
    fn test_ambito_shadowing_variables() {
        // Test de shadowing (ocultación) de variables
        let codigo = r#"
cadena nombre = "global"

vacio funcion_con_shadowing() {
    cadena nombre = "local_funcion"
    consola.imprimir("En función: " + nombre)
    
    para (entero mut i = 0; i < 2; i = i + 1) {
        cadena nombre = "local_bucle_" + i.cadena()
        consola.imprimir("En bucle: " + nombre)
    }
    
    consola.imprimir("Después del bucle en función: " + nombre)
}

funcion_con_shadowing()
consola.imprimir("En global: " + nombre)
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok(), "Error en shadowing de variables: {:?}", resultado);
    }

    #[test]
    fn test_ambito_bucles_anidados_profundos() {
        // Test con múltiples niveles de anidación
        let codigo = r#"
vacio test_anidacion() {
    para (entero mut nivel1 = 0; nivel1 < 2; nivel1 = nivel1 + 1) {
        cadena var1 = "n1_" + nivel1.cadena()
        
        para (entero mut nivel2 = 0; nivel2 < 2; nivel2 = nivel2 + 1) {
            cadena var2 = "n2_" + nivel2.cadena()
            
            para (entero mut nivel3 = 0; nivel3 < 2; nivel3 = nivel3 + 1) {
                cadena var3 = "n3_" + nivel3.cadena()
                cadena completo = var1 + "_" + var2 + "_" + var3
                consola.imprimir(completo)
            }
        }
    }
}

test_anidacion()
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok(), "Error en ámbito de bucles anidados profundos: {:?}", resultado);
    }

    #[test]
    fn test_ambito_parametros_funcion() {
        // Test de ámbito de parámetros de función
        let codigo = r#"
cadena procesar_con_bucle(cadena entrada, entero repeticiones) {
    cadena mut resultado = ""
    
    para (entero mut i = 0; i < repeticiones; i = i + 1) {
        cadena iteracion_actual = entrada + "_" + i.cadena()
        resultado = resultado + iteracion_actual + " "
    }
    
    retornar resultado.recortar()
}

cadena resultado1 = procesar_con_bucle("test", 3)
cadena resultado2 = procesar_con_bucle("otro", 2)

consola.imprimir("Resultado 1: " + resultado1)
consola.imprimir("Resultado 2: " + resultado2)
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok(), "Error en ámbito de parámetros de función: {:?}", resultado);
    }

    #[test]
    fn test_ambito_variables_locales_complejas() {
        // Test con estructuras de datos más complejas en ámbitos locales
        let codigo = r#"
vacio procesar_listas() {
    lista mut datos_locales = []
    
    para (entero mut indice = 0; indice < 3; indice = indice + 1) {
        lista elementos_iteracion = [indice, indice * 2, indice * 3]
        cadena descripcion = "Lista en iteración " + indice.cadena()
        
        para (entero mut j = 0; j < elementos_iteracion.longitud(); j = j + 1) {
            cadena elemento_str = elementos_iteracion[j].cadena()
            cadena mensaje_elemento = descripcion + ": " + elemento_str
            consola.imprimir(mensaje_elemento)
        }
    }
}

procesar_listas()
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok(), "Error en ámbito de variables locales complejas: {:?}", resultado);
    }

    #[test]
    fn test_ambito_variables_entre_bloques_condicionales() {
        // Test de ámbito entre bloques condicionales y bucles
        let codigo = r#"
vacio test_bloques_mixtos(bool condicion) {
    si (condicion) {
        cadena mensaje_condicional = "En bloque verdadero"
        consola.imprimir(mensaje_condicional)
        
        para (entero mut i = 0; i < 2; i = i + 1) {
            cadena mensaje_bucle = "Bucle en verdadero: " + i.cadena()
            consola.imprimir(mensaje_bucle)
        }
    } sino {
        cadena mensaje_condicional = "En bloque falso"
        consola.imprimir(mensaje_condicional)
        
        para (entero mut i = 0; i < 2; i = i + 1) {
            cadena mensaje_bucle = "Bucle en falso: " + i.cadena()
            consola.imprimir(mensaje_bucle)
        }
    }
}

test_bloques_mixtos(verdadero)
test_bloques_mixtos(falso)
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok(), "Error en ámbito entre bloques condicionales y bucles: {:?}", resultado);
    }

    #[test]
    fn test_ambito_variables_reutilizacion_nombres() {
        // Test específico para reutilización de nombres en diferentes contextos
        let codigo = r#"
vacio funcion_reutilizacion() {
    // Primer contexto
    para (entero mut contador = 0; contador < 2; contador = contador + 1) {
        cadena dato = "primer_contexto_" + contador.cadena()
        consola.imprimir(dato)
    }
    
    // Segundo contexto (mismo nombre de variable)
    para (entero mut contador = 10; contador < 12; contador = contador + 1) {
        cadena dato = "segundo_contexto_" + contador.cadena()
        consola.imprimir(dato)
    }
    
    // Tercer contexto en un bloque condicional
    si (verdadero) {
        cadena dato = "contexto_condicional"
        consola.imprimir(dato)
        
        para (entero mut contador = 20; contador < 22; contador = contador + 1) {
            cadena dato = "contexto_bucle_condicional_" + contador.cadena()
            consola.imprimir(dato)
        }
    }
}

funcion_reutilizacion()
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok(), "Error en reutilización de nombres de variables: {:?}", resultado);
    }

    #[test]
    fn test_ambito_error_variable_no_definida() {
        // Test que verifica que las variables locales no son accesibles fuera de su ámbito
        let codigo = r#"
vacio funcion_con_variable_local() {
    cadena variable_local = "solo_local"
}

funcion_con_variable_local()
consola.imprimir(variable_local)  // Esto debería fallar
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_err(), "Debería fallar al acceder a variable fuera de su ámbito");
    }

    #[test]
    fn test_ambito_variable_bucle_no_accesible_fuera() {
        // Test que verifica que las variables del bucle no son accesibles fuera
        let codigo = r#"
para (entero mut i = 0; i < 3; i = i + 1) {
    cadena variable_bucle = "en_bucle_" + i.cadena()
}

consola.imprimir(variable_bucle)  // Esto debería fallar
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_err(), "Debería fallar al acceder a variable de bucle fuera de su ámbito");
    }
}
