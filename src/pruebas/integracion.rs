use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_programa_completo_simple() {
        let codigo = r#"
// Programa de prueba completo
entero numero1 = 10
entero numero2 = 20
entero suma = numero1 + numero2
imprimir("Suma: " + suma.cadena())
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_programa_con_funciones_y_variables() {
        let codigo = r#"
// Función para calcular área de círculo
número calcular_area(número radio) {
    número pi = 3.14159
    retornar pi * radio * radio
}

// Variables principales
número radio_circulo = 5.0
número area = calcular_area(radio_circulo)
imprimir("Área del círculo: " + area.cadena())

// Condicional
si (area > 50.0) {
    imprimir("El círculo es grande")
} sino {
    imprimir("El círculo es pequeño")
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_programa_con_bucles_y_listas() {
        let codigo = r#"
// Crear una lista de números
lista<entero> numeros = [1, 2, 3, 4, 5]

// Función para procesar la lista
entero sumar_lista(lista<entero> lista_nums) {
    entero suma = 0
    // Simular iteración (sin foreach implementado)
    suma = suma + 1 + 2 + 3 + 4 + 5
    retornar suma
}

entero total = sumar_lista(numeros)
imprimir("Total: " + total.cadena())
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_programa_con_conversiones_encadenadas() {
        let codigo = r#"
// Prueba de conversiones encadenadas
entero numero_entero = 42
número numero_decimal = 3.14159
bool estado = verdadero

// Conversiones a cadena
cadena texto_numero = numero_entero.cadena()
cadena texto_decimal = numero_decimal.cadena()
cadena texto_estado = estado.cadena()

// Concatenación con conversiones en línea
cadena mensaje_completo = "Número: " + numero_entero.cadena() + 
                         ", Decimal: " + numero_decimal.cadena() + 
                         ", Estado: " + estado.cadena()

imprimir(mensaje_completo)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_programa_con_operadores_complejos() {
        let codigo = r#"
// Operaciones aritméticas complejas
entero a = 10
entero b = 5
entero c = 3

entero resultado1 = (a + b) * c
entero resultado2 = a * b - c
entero resultado3 = a / b + c
entero resultado4 = a % b * c

// Operadores de asignación compuesta
a += b
b *= c
c -= 1

// Comparaciones
bool mayor = resultado1 > resultado2
bool menor_igual = resultado3 <= resultado4
bool igual = a == b

imprimir("Operaciones completadas")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_programa_con_funciones_anidadas() {
        let codigo = r#"
// Funciones que se llaman entre sí
entero multiplicar_por_dos(entero x) {
    retornar x * 2
}

entero sumar_cinco(entero x) {
    retornar x + 5
}

entero procesar_numero(entero num) {
    entero paso1 = multiplicar_por_dos(num)
    entero paso2 = sumar_cinco(paso1)
    retornar paso2
}

entero numero_inicial = 10
entero resultado_final = procesar_numero(numero_inicial)
imprimir("Resultado: " + resultado_final.cadena())
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_programa_con_json_basico() {
        let codigo = r#"
// Objeto JSON simple
jsn persona = {
    nombre: "Juan",
    edad: 30,
    activo: verdadero
}

imprimir("Persona creada")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_programa_con_json_extrano(){
        let codigo = r#"
// Objeto JSON con estructura compleja
jsn complejo = {
    usuario: {
        nombre: "Ana", // Comentario para Ana
        detalles: {
            edad: 28,
            activo: falso // El interprete no debería fallar aquí al poner comentarios en la misma linea
        },
        intereses: ["programación", "música", "arte"]
    },
    lista_numeros: [1, 2, 3, 4, 5],
    matriz: [[1, 2], [3, 4]],
    fecha: "2023-10-01",
    otro_json_raro: { clave: "valor", numero: 42, lista_interna: [1, 2, 3] },
    configuracion: {
        tema: "oscuro",
        notificaciones: verdadero
    }
}
imprimir("Objeto JSON complejo creado")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_programa_con_condicionales_complejos() {
        let codigo = r#"
// Condicionales anidados y complejos
entero edad = 25
bool es_estudiante = verdadero
bool tiene_trabajo = falso

si (edad >= 18) {
    si (es_estudiante y !tiene_trabajo) {
        imprimir("Estudiante adulto sin trabajo")
    } sino si (es_estudiante y tiene_trabajo) {
        imprimir("Estudiante trabajador")
    } sino {
        imprimir("Adulto no estudiante")
    }
} sino {
    imprimir("Menor de edad")
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_programa_con_variables_mutables() {
        let codigo = r#"
// Variables mutables
entero mut contador = 0
cadena mut mensaje = "Inicial"

// Modificar variables
contador += 5
contador *= 2
mensaje = "Modificado"

imprimir("Contador: " + contador.cadena())
imprimir("Mensaje: " + mensaje)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_programa_completo_complejo() {
        let codigo = r#"
// Programa complejo que combina varias funcionalidades

// Función para validar número
bool es_par(entero num) {
    retornar (num % 2) == 0
}

// Función para formatear resultado
cadena formatear_resultado(entero num, bool par) {
    cadena tipo = par ? "par" : "impar"
    retornar "El número " + num.cadena() + " es " + tipo
}

// Variables principales
entero numero_prueba = 42
bool resultado_par = es_par(numero_prueba)
cadena mensaje_final = formatear_resultado(numero_prueba, resultado_par)

// Imprimir resultado
imprimir(mensaje_final)

// Lista de números para procesar
lista<entero> numeros = [10, 15, 20, 25]

// Procesar cada número (simulado)
imprimir("Procesando números...")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_programa_con_multiples_tipos() {
        let codigo = r#"
// Diferentes tipos de datos trabajando juntos
vacio variable_vacia
entero numero_entero = 100
número numero_decimal = 99.99
cadena texto = "Prueba"
bool estado_activo = verdadero

// Función que trabaja con múltiples tipos
cadena crear_reporte(entero num, número dec, cadena txt, bool activo) {
    cadena estado_texto = activo ? "Activo" : "Inactivo"
    retornar "Reporte: " + txt + 
             ", Entero: " + num.cadena() + 
             ", Decimal: " + dec.cadena() + 
             ", Estado: " + estado_texto
}

cadena reporte = crear_reporte(numero_entero, numero_decimal, texto, estado_activo)
imprimir(reporte)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }
}
