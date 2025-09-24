#[cfg(test)]
mod tests {
    use crate::nucleo::interprete::interpretar;

    #[test]
    fn test_factorial_recursivo() {
        let codigo = r#"
            entero factorial(entero n) {
                si (n <= 1) {
                    retornar 1
                } sino {
                    retornar n * factorial(n - 1)
                }
            }
            
            entero resultado = factorial(5)
            consola.imprimir("Factorial de 5: " + resultado.texto())
        "#;

        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_fibonacci_recursivo() {
        let codigo = r#"
            entero fibonacci(entero n) {
                si (n <= 1) {
                    retornar n
                } sino {
                    retornar fibonacci(n - 1) + fibonacci(n - 2)
                }
            }
            
            entero resultado = fibonacci(6)
            consola.imprimir("Fibonacci de 6: " + resultado.texto())
        "#;

        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_suma_lista_recursiva() {
        let codigo = r#"
            entero suma_lista(lista numeros, entero indice) {
                si (indice >= numeros.longitud()) {
                    retornar 0
                } sino {
                    entero elemento = numeros[indice]
                    retornar elemento + suma_lista(numeros, indice + 1)
                }
            }
            
            lista numeros = [1, 2, 3, 4, 5]
            entero total = suma_lista(numeros, 0)
            consola.imprimir("Suma total: " + total.texto())
        "#;

        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_potencia_recursiva() {
        let codigo = r#"
            entero potencia(entero base, entero exponente) {
                si (exponente == 0) {
                    retornar 1
                } sino {
                    si (exponente == 1) {
                        retornar base
                    } sino {
                        retornar base * potencia(base, exponente - 1)
                    }
                }
            }
            
            entero resultado = potencia(2, 4)
            consola.imprimir("2 elevado a la 4: " + resultado.texto())
        "#;

        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_es_primo_iterativo() {
        let codigo = r#"
            log es_primo(entero num) {
                si (num < 2) {
                    retornar falso
                }
                si (num == 2) {
                    retornar verdadero
                }
                si (num % 2 == 0) {
                    retornar falso
                }
                
                entero var i = 3
                mientras (i * i <= num) {
                    si (num % i == 0) {
                        retornar falso
                    }
                    i = i + 2
                }
                retornar verdadero
            }
            
            log resultado = es_primo(17)
            consola.imprimir("17 es primo: " + resultado.texto())
        "#;

        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }
}
