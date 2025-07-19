#[cfg(test)]
mod tests {
    use crate::interprete::interpretar;

    #[test]
    fn test_declaracion_objeto_basico() {
        let codigo = r#"
            objeto Persona {
                texto nombre = "Sin nombre"
                entero edad = 0
                
                vacio saludar() {
                    consola.imprimir("Hola, soy " + ambiente.nombre)
                }
            }
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error al declarar objeto básico: {:?}", resultado.err());
    }

    #[test]
    fn test_instanciacion_objeto_sin_parametros() {
        let codigo = r#"
            objeto Persona {
                texto nombre = "Juan"
                entero edad = 25
            }
            
            Persona persona1 = nuevo Persona()
            consola.imprimir("Persona creada: " + persona1.nombre)
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error al instanciar objeto sin parámetros: {:?}", resultado.err());
    }

    #[test]
    fn test_instanciacion_objeto_con_constructor() {
        let codigo = r#"
            objeto Persona {
                texto nombre
                entero edad
                
                Persona(texto n, entero e) {
                    ambiente.nombre = n
                    ambiente.edad = e
                }
            }
            
            Persona persona1 = nuevo Persona("María", 30)
            consola.imprimir("Nombre: " + persona1.nombre + ", Edad: " + persona1.edad.texto())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error al instanciar objeto con constructor: {:?}", resultado.err());
    }

    #[test]
    fn test_acceso_propiedades_publicas() {
        let codigo = r#"
            objeto Contador {
                publico:
                    entero var valor = 0
                    
                    entero obtener_valor() {
                        retornar ambiente.valor
                    }
            }
            
            Contador contador = nuevo Contador()
            consola.imprimir("Valor inicial: " + contador.obtener_valor().texto())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error al acceder a propiedades públicas: {:?}", resultado.err());
    }

    #[test]
    fn test_modificacion_propiedades_publicas() {
        let codigo = r#"
            objeto Contador {
                publico:
                    entero var valor = 0
                    
                    vacio establecer_valor(entero nuevo_valor) {
                        ambiente.valor = nuevo_valor
                    }
                    
                    entero obtener_valor() {
                        retornar ambiente.valor
                    }
            }
            
            Contador contador = nuevo Contador()
            contador.establecer_valor(5)
            consola.imprimir("Valor actualizado: " + contador.obtener_valor().texto())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error al modificar propiedades públicas: {:?}", resultado.err());
    }

    #[test]
    fn test_llamada_metodos_publicos() {
        let codigo = r#"
            objeto Calculadora {
                entero sumar(entero a, entero b) {
                    retornar a + b
                }
                
                entero multiplicar(entero a, entero b) {
                    retornar a * b
                }
            }
            
            Calculadora calc = nuevo Calculadora()
            entero suma = calc.sumar(5, 3)
            entero producto = calc.multiplicar(4, 6)
            
            consola.imprimir("Suma: " + suma.texto())
            consola.imprimir("Producto: " + producto.texto())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error al llamar métodos públicos: {:?}", resultado.err());
    }

    #[test]
    fn test_acceso_propiedades_privadas_debe_fallar() {
        let codigo = r#"
            objeto Secreto {
                publico:
                    texto publico = "Información pública"
                    
                privado:
                    texto secreto = "Información secreta"
            }
            
            Secreto obj = nuevo Secreto()
            texto publico = obj.publico
            texto secreto = obj.secreto // Esto debe fallar
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_err(), "Debería fallar al acceder a propiedades privadas");
    }

    #[test]
    fn test_metodos_con_ambiente() {
        let codigo = r#"
            objeto Usuario {
                publico:
                    texto nombre = "Usuario"
                    entero edad = 18
                    
                    texto obtener_info() {
                        retornar "Nombre: " + ambiente.nombre + ", Edad: " + ambiente.edad.texto()
                    }
                    
                    vacio cumplir_años() {
                        ambiente.edad = ambiente.edad + 1
                    }
            }
            
            Usuario usuario = nuevo Usuario()
            consola.imprimir(usuario.obtener_info())
            
            usuario.cumplir_años()
            consola.imprimir(usuario.obtener_info())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error con métodos que usan ambiente: {:?}", resultado.err());
    }

    #[test]
    fn test_multiples_instancias_independientes() {
        let codigo = r#"
            objeto Banco {
                publico:
                    texto titular = ""
                    número var saldo = 0.0
                    
                    constructor(texto t) {
                        ambiente.titular = t
                    }
                    
                    vacio depositar(número cantidad) {
                        ambiente.saldo = ambiente.saldo + cantidad
                    }
                    
                    número obtener_saldo() {
                        retornar ambiente.saldo
                    }
            }
            
            Banco cuenta1 = nuevo Banco("Juan")
            Banco cuenta2 = nuevo Banco("María")
            
            cuenta1.depositar(100.0)
            cuenta2.depositar(250.0)
            
            consola.imprimir("Saldo Juan: " + cuenta1.obtener_saldo().texto())
            consola.imprimir("Saldo María: " + cuenta2.obtener_saldo().texto())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error con múltiples instancias independientes: {:?}", resultado.err());
    }

    #[test]
    fn test_objeto_con_listas() {
        let codigo = r#"
            objeto Inventario {
                publico:
                    lista var productos = []
                    
                    vacio agregar_producto(texto producto) {
                        lista var temp = ambiente.productos
                        temp.agregar(producto)
                        ambiente.productos = temp
                    }
                    
                    entero contar_productos() {
                        retornar ambiente.productos.longitud()
                    }
                    
                    texto obtener_producto(entero indice) {
                        retornar ambiente.productos[indice]
                    }
            }
            
            Inventario inv = nuevo Inventario()
            inv.agregar_producto("Manzanas")
            inv.agregar_producto("Naranjas")
            inv.agregar_producto("Bananas")
            
            consola.imprimir("Total productos: " + inv.contar_productos().texto())
            consola.imprimir("Primer producto: " + inv.obtener_producto(0))
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error con objetos que contienen listas: {:?}", resultado.err());
    }

    #[test]
    fn test_objeto_con_objetos_json() {
        let codigo = r#"
            objeto Configuracion {
                publico:
                    jsn var configuraciones = {}
                    
                    vacio establecer(texto clave, texto valor) {
                        jsn temp = ambiente.configuraciones
                        temp[clave] = valor
                        ambiente.configuraciones = temp
                    }
                    
                    texto obtener(texto clave) {
                        retornar ambiente.configuraciones[clave]
                    }
            }
            
            Configuracion config = nuevo Configuracion()
            config.establecer("idioma", "español")
            config.establecer("tema", "oscuro")
            
            consola.imprimir("Idioma: " + config.obtener("idioma"))
            consola.imprimir("Tema: " + config.obtener("tema"))
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error con objetos que contienen JSON: {:?}", resultado.err());
    }

    #[test]
    fn test_herencia_o_composicion_basica() {
        let codigo = r#"
            objeto Motor {
                publico:
                    texto tipo = "V6"
                    número potencia = 250.0
                    
                    vacio encender() {
                        consola.imprimir("Motor " + ambiente.tipo + " encendido")
                    }
            }
            
            objeto Vehiculo {
                publico:
                    texto marca = ""
                    Motor motor = nuevo Motor()
                    
                    constructor(texto m) {
                        ambiente.marca = m
                        ambiente.motor = nuevo Motor()
                    }
                    
                    vacio arrancar() {
                        consola.imprimir("Arrancando " + ambiente.marca)
                        ambiente.motor.encender()
                    }
            }
            
            Vehiculo auto = nuevo Vehiculo("Toyota")
            auto.arrancar()
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error con composición de objetos: {:?}", resultado.err());
    }

    #[test]
    fn test_metodos_con_multiples_parametros() {
        let codigo = r#"
            objeto Matematicas {
                publico:
                    número calcular_area_rectangulo(número ancho, número alto) {
                        retornar ancho * alto
                    }
                    
                    número calcular_promedio(número a, número b, número c) {
                        retornar (a + b + c) / 3.0
                    }
                    
                    texto formar_mensaje(texto saludo, texto nombre, entero edad) {
                        retornar saludo + " " + nombre + ", tienes " + edad.texto() + " años"
                    }
            }
            
            Matematicas mat = nuevo Matematicas()
            número area = mat.calcular_area_rectangulo(5.0, 3.0)
            número promedio = mat.calcular_promedio(8.5, 9.0, 7.5)
            texto mensaje = mat.formar_mensaje("Hola", "Ana", 25)
            
            consola.imprimir("Área: " + area.texto())
            consola.imprimir("Promedio: " + promedio.texto())
            consola.imprimir(mensaje)
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error con métodos de múltiples parámetros: {:?}", resultado.err());
    }

    #[test]
    fn test_objeto_sin_propiedades_solo_metodos() {
        let codigo = r#"
            objeto Utilidades {
                publico:
                    texto saludar(texto nombre) {
                        retornar "¡Hola, " + nombre + "!"
                    }
                    
                    entero factorial(entero n) {
                        si (n <= 1) {
                            retornar 1
                        } sino {
                            retornar n * ambiente.factorial(n - 1)
                        }
                    }
            }
            
            Utilidades util = nuevo Utilidades()
            consola.imprimir(util.saludar("Mundo"))
            consola.imprimir("Factorial de 5: " + util.factorial(5).texto())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error con objeto sin propiedades: {:?}", resultado.err());
    }

    #[test]
    fn test_constructor_con_validaciones() {
        let codigo = r#"
            objeto Empleado {
                publico:
                    texto nombre = ""
                    entero edad = 0
                    número salario = 0.0
                    
                    constructor(texto n, entero e, número s) {
                        si (e < 18) {
                            consola.imprimir_error("La edad debe ser mayor a 18")
                            retornar
                        }
                        si (s < 0.0) {
                            consola.imprimir_error("El salario no puede ser negativo")
                            retornar
                        }
                        
                        ambiente.nombre = n
                        ambiente.edad = e
                        ambiente.salario = s
                        consola.imprimir("Empleado creado exitosamente")
                    }
                    
                    texto obtener_informacion() {
                        retornar ambiente.nombre + " (" + ambiente.edad.texto() + " años) - $" + ambiente.salario.texto()
                    }
            }
            
            Empleado emp1 = nuevo Empleado("Carlos", 30, 50000.0)
            consola.imprimir(emp1.obtener_informacion())
            
            Empleado emp2 = nuevo Empleado("Ana", 16, 30000.0) // Debería mostrar error
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error con constructor con validaciones: {:?}", resultado.err());
    }

    #[test]
    fn test_modificacion_propiedades_a_traves_de_metodos() {
        let codigo = r#"
            objeto EstadoJuego {
                publico:
                    entero var puntos = 0
                    entero var vidas = 3
                    log var activo = verdadero
                    
                    vacio sumar_puntos(entero cantidad) {
                        ambiente.puntos = ambiente.puntos + cantidad
                    }
                    
                    vacio perder_vida() {
                        si (ambiente.vidas > 0) {
                            ambiente.vidas = ambiente.vidas - 1
                        }
                        si (ambiente.vidas == 0) {
                            ambiente.activo = falso
                            consola.imprimir("¡Game Over!")
                        }
                    }
                    
                    texto obtener_estado() {
                        retornar "Puntos: " + ambiente.puntos.texto() + ", Vidas: " + ambiente.vidas.texto()
                    }
            }
            
            EstadoJuego juego = nuevo EstadoJuego()
            juego.sumar_puntos(100)
            juego.sumar_puntos(50)
            consola.imprimir(juego.obtener_estado())
            
            juego.perder_vida()
            juego.perder_vida()
            consola.imprimir(juego.obtener_estado())
            
            juego.perder_vida() // Debería mostrar Game Over
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok(), "Error al modificar propiedades a través de métodos: {:?}", resultado.err());
    }
}
