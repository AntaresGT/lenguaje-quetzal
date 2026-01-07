// Pruebas unitarias para objetos del lenguaje Quetzal

use super::auxiliares::*;

#[test]
fn prueba_objeto_basico() {
    let codigo = r#"
        objeto Usuario {
            privado:
                texto var nombre
                entero var edad
            publico:
                Usuario(texto nombre, entero edad) {
                    ambiente.nombre = nombre
                    ambiente.edad = edad
                }
                
                texto obtener_nombre() {
                    retornar ambiente.nombre
                }
                
                entero obtener_edad() {
                    retornar ambiente.edad
                }
        }
        
        Usuario usuario1 = nuevo Usuario("Juan", 30)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_objeto_sin_etiquetas_acceso() {
    let codigo = r#"
        objeto DefinicionUsuario {
            texto var nombre
            entero var edad
            
            DefinicionUsuario(texto nombre, entero edad) {
                ambiente.nombre = nombre
                ambiente.edad = edad
            }
            
            texto obtener_nombre() {
                retornar ambiente.nombre
            }
        }
        
        DefinicionUsuario usuario2 = nuevo DefinicionUsuario("Juan", 30)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_objeto_con_funciones_libres() {
    let codigo = r#"
        objeto UsuarioLibre {
            libre texto nombre
            libre entero edad
            
            libre entero absoluto(entero valor) {
                retornar valor < 0 ? -valor : valor
            }
        }
        
        entero valor_absoluto = UsuarioLibre.absoluto(-10)
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "valor_absoluto", 10));
}

#[test]
fn prueba_objeto_con_atributos_libres_privados() {
    // Prueba simplificada de miembros libres (estáticos) con acceso desde el tipo
    let codigo = r#"
        objeto Calculadora {
            publico:
                libre entero sumar(entero a, entero b) {
                    retornar a + b
                }
                
                libre entero multiplicar(entero a, entero b) {
                    retornar a * b
                }
        }
        
        entero resultado_suma = Calculadora.sumar(5, 3)
        entero resultado_mult = Calculadora.multiplicar(4, 7)
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "resultado_suma", 8));
    assert!(verificar_variable_entero(&entorno, "resultado_mult", 28));
}

// =====================================================
// PRUEBAS DE HERENCIA
// =====================================================

#[test]
fn prueba_herencia_simple() {
    let codigo = r#"
        objeto Animal {
            publico:
                Animal(texto nombre) {
                    ambiente.nombre = nombre
                }
                texto obtener_nombre() {
                    retornar ambiente.nombre
                }
        }
        
        objeto Perro como Animal {
            publico:
                Perro(texto nombre) {
                    padre.Animal(nombre)
                }
                texto ladrar() {
                    retornar "Guau!"
                }
        }
        
        Perro perro = nuevo Perro("Firulais")
        texto nombre = perro.obtener_nombre()
        texto sonido = perro.ladrar()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "nombre", "Firulais"));
    assert!(verificar_variable_texto(&entorno, "sonido", "Guau!"));
}

#[test]
fn prueba_herencia_con_multiples_propiedades() {
    let codigo = r#"
        objeto Vehiculo {
            publico:
                Vehiculo(texto marca, entero anio) {
                    ambiente.marca = marca
                    ambiente.anio = anio
                }
                texto obtener_marca() {
                    retornar ambiente.marca
                }
                entero obtener_anio() {
                    retornar ambiente.anio
                }
        }
        
        objeto Auto como Vehiculo {
            publico:
                Auto(texto marca, entero anio, entero puertas) {
                    padre.Vehiculo(marca, anio)
                    ambiente.puertas = puertas
                }
                entero obtener_puertas() {
                    retornar ambiente.puertas
                }
        }
        
        Auto auto = nuevo Auto("Toyota", 2023, 4)
        texto marca = auto.obtener_marca()
        entero anio = auto.obtener_anio()
        entero puertas = auto.obtener_puertas()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "marca", "Toyota"));
    assert!(verificar_variable_entero(&entorno, "anio", 2023));
    assert!(verificar_variable_entero(&entorno, "puertas", 4));
}

#[test]
fn prueba_herencia_cascada() {
    let codigo = r#"
        objeto SerVivo {
            publico:
                SerVivo(texto nombre) {
                    ambiente.nombre = nombre
                }
                texto obtener_nombre() {
                    retornar ambiente.nombre
                }
        }
        
        objeto Animal como SerVivo {
            publico:
                Animal(texto nombre, texto especie) {
                    padre.SerVivo(nombre)
                    ambiente.especie = especie
                }
                texto obtener_especie() {
                    retornar ambiente.especie
                }
        }
        
        objeto Mamifero como Animal {
            publico:
                Mamifero(texto nombre, texto especie) {
                    padre.Animal(nombre, especie)
                }
                texto respirar() {
                    retornar "Respira aire"
                }
        }
        
        Mamifero leon = nuevo Mamifero("Simba", "Felino")
        texto nombre = leon.obtener_nombre()
        texto especie = leon.obtener_especie()
        texto respiracion = leon.respirar()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "nombre", "Simba"));
    assert!(verificar_variable_texto(&entorno, "especie", "Felino"));
    assert!(verificar_variable_texto(&entorno, "respiracion", "Respira aire"));
}

#[test]
fn prueba_herencia_sobreescritura_metodo() {
    let codigo = r#"
        objeto Figura {
            publico:
                Figura() {
                }
                texto describir() {
                    retornar "Una figura"
                }
        }
        
        objeto Circulo como Figura {
            publico:
                Circulo(numero radio) {
                    padre.Figura()
                    ambiente.radio = radio
                }
                texto describir() {
                    retornar "Un circulo"
                }
        }
        
        Circulo c = nuevo Circulo(5.0)
        texto descripcion = c.describir()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "descripcion", "Un circulo"));
}

#[test]
fn prueba_herencia_multiple() {
    let codigo = r#"
        objeto Volador {
            publico:
                Volador() {
                    ambiente.puede_volar = verdadero
                }
                texto volar() {
                    retornar "Volando!"
                }
        }
        
        objeto Nadador {
            publico:
                Nadador() {
                    ambiente.puede_nadar = verdadero
                }
                texto nadar() {
                    retornar "Nadando!"
                }
        }
        
        objeto Pato como Volador, Nadador {
            publico:
                Pato(texto nombre) {
                    padre.Volador()
                    padre.Nadador()
                    ambiente.nombre = nombre
                }
                texto obtener_nombre() {
                    retornar ambiente.nombre
                }
        }
        
        Pato pato = nuevo Pato("Donald")
        texto nombre = pato.obtener_nombre()
        texto volando = pato.volar()
        texto nadando = pato.nadar()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "nombre", "Donald"));
    assert!(verificar_variable_texto(&entorno, "volando", "Volando!"));
    assert!(verificar_variable_texto(&entorno, "nadando", "Nadando!"));
}

#[test]
fn prueba_herencia_multiple_con_propiedades() {
    let codigo = r#"
        objeto Motor {
            publico:
                Motor(entero potencia) {
                    ambiente.potencia = potencia
                }
                entero obtener_potencia() {
                    retornar ambiente.potencia
                }
        }
        
        objeto Carroceria {
            publico:
                Carroceria(texto color) {
                    ambiente.color = color
                }
                texto obtener_color() {
                    retornar ambiente.color
                }
        }
        
        objeto Carro como Motor, Carroceria {
            publico:
                Carro(entero potencia, texto color, texto modelo) {
                    padre.Motor(potencia)
                    padre.Carroceria(color)
                    ambiente.modelo = modelo
                }
                texto obtener_modelo() {
                    retornar ambiente.modelo
                }
        }
        
        Carro carro = nuevo Carro(200, "Rojo", "Deportivo")
        entero potencia = carro.obtener_potencia()
        texto color = carro.obtener_color()
        texto modelo = carro.obtener_modelo()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "potencia", 200));
    assert!(verificar_variable_texto(&entorno, "color", "Rojo"));
    assert!(verificar_variable_texto(&entorno, "modelo", "Deportivo"));
}

#[test]
fn prueba_llamada_metodo_padre_especifico() {
    let codigo = r#"
        objeto ClaseA {
            publico:
                ClaseA() {
                }
                texto metodo() {
                    retornar "Metodo de A"
                }
        }
        
        objeto ClaseB {
            publico:
                ClaseB() {
                }
                texto metodo() {
                    retornar "Metodo de B"
                }
        }
        
        objeto ClaseC como ClaseA, ClaseB {
            publico:
                ClaseC() {
                    padre.ClaseA()
                    padre.ClaseB()
                }
                texto metodo_a() {
                    retornar padre.ClaseA.metodo()
                }
                texto metodo_b() {
                    retornar padre.ClaseB.metodo()
                }
        }
        
        ClaseC obj = nuevo ClaseC()
        texto resultado_a = obj.metodo_a()
        texto resultado_b = obj.metodo_b()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "resultado_a", "Metodo de A"));
    assert!(verificar_variable_texto(&entorno, "resultado_b", "Metodo de B"));
}

#[test]
fn prueba_herencia_acceso_directo_propiedades() {
    let codigo = r#"
        objeto Base {
            publico:
                Base(texto valor) {
                    ambiente.dato = valor
                }
        }
        
        objeto Derivado como Base {
            publico:
                Derivado(texto valor) {
                    padre.Base(valor)
                }
        }
        
        Derivado d = nuevo Derivado("test")
        texto resultado = d.dato
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "resultado", "test"));
}

#[test]
fn prueba_herencia_con_atributo_declarado() {
    let codigo = r#"
        objeto Persona {
            publico:
                texto var nombre = "Sin nombre"
                
                Persona(texto n) {
                    ambiente.nombre = n
                }
                
                texto saludar() {
                    retornar "Hola, soy " + ambiente.nombre
                }
        }
        
        objeto Empleado como Persona {
            publico:
                texto var puesto = "Sin puesto"
                
                Empleado(texto n, texto p) {
                    padre.Persona(n)
                    ambiente.puesto = p
                }
                
                texto presentarse() {
                    retornar ambiente.nombre + " - " + ambiente.puesto
                }
        }
        
        Empleado emp = nuevo Empleado("Juan", "Desarrollador")
        texto saludo = emp.saludar()
        texto presentacion = emp.presentarse()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "saludo", "Hola, soy Juan"));
    assert!(verificar_variable_texto(&entorno, "presentacion", "Juan - Desarrollador"));
}