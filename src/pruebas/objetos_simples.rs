use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    fn interpretar(codigo: &str) -> Result<(), crate::errores::ErrorQuetzal> {
        match interprete::interpretar(codigo) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    #[test]
    fn test_objeto_contador_simple() {
        let codigo = r#"
            objeto Contador {
                publico:
                    entero var valor = 0
                    
                    vacio incrementar() {
                        ambiente.valor = ambiente.valor + 1
                    }
                    
                    entero obtener() {
                        retornar ambiente.valor
                    }
            }
            
            Contador contador = nuevo Contador()
            contador.incrementar()
            contador.incrementar()
            consola.imprimir("Contador: " + contador.obtener().texto())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error con contador simple: {:?}", resultado.err());
    }

    #[test]
    fn test_objeto_con_constructor_basico() {
        let codigo = r#"
            objeto Persona {
                publico:
                    texto var nombre = ""
                    entero var edad = 0
                    
                    vacio establecer(texto n, entero e) {
                        ambiente.nombre = n
                        ambiente.edad = e
                    }
                    
                    texto saludar() {
                        retornar "Hola, soy " + ambiente.nombre
                    }
            }
            
            Persona persona = nuevo Persona()
            persona.establecer("Juan", 25)
            consola.imprimir(persona.saludar())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error con constructor básico: {:?}", resultado.err());
    }

    #[test]
    fn test_objeto_con_propiedades_privadas() {
        let codigo = r#"
            objeto CuentaBanco {
                privado:
                    número var saldo = 100.0
                    
                publico:
                    vacio depositar(número cantidad) {
                        ambiente.saldo = ambiente.saldo + cantidad
                    }
                    
                    número consultar_saldo() {
                        retornar ambiente.saldo
                    }
            }
            
            CuentaBanco cuenta = nuevo CuentaBanco()
            cuenta.depositar(50.0)
            consola.imprimir("Saldo: " + cuenta.consultar_saldo().texto())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error con propiedades privadas: {:?}", resultado.err());
    }

    #[test]
    fn test_objeto_calculadora() {
        let codigo = r#"
            objeto Calculadora {
                publico:
                    número sumar(número a, número b) {
                        retornar a + b
                    }
                    
                    número multiplicar(número a, número b) {
                        retornar a * b
                    }
                    
                    número potencia(número base, entero exponente) {
                        número var resultado = 1.0
                        para (entero var i = 0; i < exponente; i = i + 1) {
                            resultado = resultado * base
                        }
                        retornar resultado
                    }
            }
            
            Calculadora calc = nuevo Calculadora()
            número suma = calc.sumar(5.5, 3.2)
            número producto = calc.multiplicar(4.0, 6.0)
            número cuadrado = calc.potencia(3.0, 2)
            
            consola.imprimir("Suma: " + suma.texto())
            consola.imprimir("Producto: " + producto.texto())
            consola.imprimir("3^2: " + cuadrado.texto())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error con calculadora: {:?}", resultado.err());
    }

    #[test]
    fn test_multiples_objetos() {
        let codigo = r#"
            objeto Punto {
                publico:
                    número var coordX = 0.0
                    número var coordY = 0.0
                    
                    vacio establecer(número px, número py) {
                        ambiente.coordX = px
                        ambiente.coordY = py
                    }
                    
                    número distancia() {
                        número cuadX = ambiente.coordX * ambiente.coordX
                        número cuadY = ambiente.coordY * ambiente.coordY
                        retornar cuadX + cuadY
                    }
            }
            
            Punto p1 = nuevo Punto()
            p1.establecer(3.0, 4.0)
            
            Punto p2 = nuevo Punto()
            p2.establecer(1.0, 2.0)
            
            consola.imprimir("Punto 1 - X: " + p1.coordX.texto() + ", Y: " + p1.coordY.texto())
            consola.imprimir("Distancia P1: " + p1.distancia().texto())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error con múltiples objetos: {:?}", resultado.err());
    }

    #[test]
    fn test_objeto_estado_juego() {
        let codigo = r#"
            objeto JuegoEstado {
                publico:
                    entero var puntos = 0
                    entero var vidas = 3
                    log var terminado = falso
                    
                    vacio sumar_puntos(entero cantidad) {
                        ambiente.puntos = ambiente.puntos + cantidad
                        consola.imprimir("Puntos sumados: " + cantidad.texto())
                    }
                    
                    vacio perder_vida() {
                        si (ambiente.vidas > 0) {
                            ambiente.vidas = ambiente.vidas - 1
                            consola.imprimir("Vida perdida. Vidas restantes: " + ambiente.vidas.texto())
                        }
                        
                        si (ambiente.vidas == 0) {
                            ambiente.terminado = verdadero
                            consola.imprimir("Game Over!")
                        }
                    }
                    
                    texto obtener_estado() {
                        retornar "Puntos: " + ambiente.puntos.texto() + 
                                ", Vidas: " + ambiente.vidas.texto() + 
                                ", Terminado: " + ambiente.terminado.texto()
                    }
            }
            
            JuegoEstado juego = nuevo JuegoEstado()
            juego.sumar_puntos(100)
            juego.sumar_puntos(50)
            juego.perder_vida()
            consola.imprimir(juego.obtener_estado())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error con estado de juego: {:?}", resultado.err());
    }

    #[test]
    fn test_objeto_con_inicializacion() {
        let codigo = r#"
            objeto Producto {
                publico:
                    texto var nombre = "Sin nombre"
                    número var precio = 0.0
                    
                    vacio establecer(texto n, número p) {
                        ambiente.nombre = n
                        ambiente.precio = p
                    }
                    
                    texto informacion() {
                        retornar ambiente.nombre + " - $" + ambiente.precio.texto()
                    }
            }
            
            Producto producto = nuevo Producto()
            producto.establecer("Laptop", 999.99)
            consola.imprimir(producto.informacion())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error con inicialización: {:?}", resultado.err());
    }

    #[test]
    fn test_objeto_con_metodos_matematicos() {
        let codigo = r#"
            objeto Matematicas {
                publico:
                    entero factorial(entero n) {
                        si (n <= 1) {
                            retornar 1
                        } sino {
                            retornar n * ambiente.factorial(n - 1)
                        }
                    }
                    
                    entero fibonacci(entero n) {
                        si (n <= 1) {
                            retornar n
                        } sino {
                            retornar ambiente.fibonacci(n - 1) + ambiente.fibonacci(n - 2)
                        }
                    }
                    
                    número promedio(número a, número b, número c) {
                        retornar (a + b + c) / 3.0
                    }
            }
            
            Matematicas mat = nuevo Matematicas()
            entero fact5 = mat.factorial(5)
            entero fib6 = mat.fibonacci(6)
            número prom = mat.promedio(8.5, 9.0, 7.5)
            
            consola.imprimir("Factorial de 5: " + fact5.texto())
            consola.imprimir("Fibonacci de 6: " + fib6.texto())
            consola.imprimir("Promedio: " + prom.texto())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error con métodos matemáticos: {:?}", resultado.err());
    }
}
