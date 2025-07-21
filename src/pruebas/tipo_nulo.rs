use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_nulo_basico() {
        let codigo = r#"
// Declarar una variable con literal nulo
auto valor_nulo = nulo
consola.imprimir(valor_nulo)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_asignacion_nulo_a_diferentes_tipos() {
        let codigo = r#"
// Asignar nulo a variables de diferentes tipos
texto texto_nulo = nulo
entero entero_nulo = nulo
número numero_nulo = nulo
log bool_nulo = nulo

consola.imprimir("Texto nulo: " + texto_nulo)
consola.imprimir("Entero nulo: " + entero_nulo)
consola.imprimir("Número nulo: " + numero_nulo)
consola.imprimir("Bool nulo: " + bool_nulo)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_acceso_propiedades_json_inexistentes() {
        let codigo = r#"
// Crear objeto JSON y acceder a propiedades inexistentes
jsn persona = {
    "nombre": "Juan",
    "edad": 25
}

// Acceder a propiedades que no existen (deben devolver nulo)
texto apellido = persona["apellido"]
texto telefono = persona["telefono"]
entero salario = persona["salario"]

consola.imprimir("Apellido: " + apellido)
consola.imprimir("Telefono: " + telefono)
consola.imprimir("Salario: " + salario)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_comparacion_igualdad_nulo() {
        let codigo = r#"
// Probar comparaciones de igualdad con nulo
auto valor1 = nulo
auto valor2 = nulo
texto valor3 = "texto"

// Comparaciones que deben ser verdaderas
si (valor1 == nulo) {
    consola.imprimir("valor1 es nulo: CORRECTO")
}

si (valor2 == valor1) {
    consola.imprimir("valor2 igual a valor1: CORRECTO")
}

// Comparaciones que deben ser falsas
si (valor3 == nulo) {
    consola.imprimir("valor3 es nulo: ERROR")
} sino {
    consola.imprimir("valor3 NO es nulo: CORRECTO")
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_comparacion_desigualdad_nulo() {
        let codigo = r#"
// Probar comparaciones de desigualdad con nulo
auto valor_nulo = nulo
texto valor_texto = "no nulo"

// Comparaciones que deben ser verdaderas
si (valor_texto != nulo) {
    consola.imprimir("valor_texto NO es nulo: CORRECTO")
}

// Comparaciones que deben ser falsas
si (valor_nulo != nulo) {
    consola.imprimir("valor_nulo NO es nulo: ERROR")
} sino {
    consola.imprimir("valor_nulo ES nulo: CORRECTO")
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_nulo_con_propiedades_json_directas() {
        let codigo = r#"
// Probar comparaciones directas con propiedades JSON inexistentes
jsn datos = {
    "nombre": "Ana",
    "ciudad": "Madrid"
}

// Comparación directa con propiedad inexistente
si (datos["apellido"] == nulo) {
    consola.imprimir("datos['apellido'] es nulo: CORRECTO")
}

si (datos["telefono"] != nulo) {
    consola.imprimir("datos['telefono'] NO es nulo: ERROR")
} sino {
    consola.imprimir("datos['telefono'] ES nulo: CORRECTO")
}

// Comparación con propiedad existente
si (datos["nombre"] == nulo) {
    consola.imprimir("datos['nombre'] es nulo: ERROR")
} sino {
    consola.imprimir("datos['nombre'] NO es nulo: CORRECTO")
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_conversion_nulo_a_texto() {
        let codigo = r#"
// Probar conversión de nulo a texto
auto valor_nulo = nulo
texto nulo_como_texto = valor_nulo

// Verificar que se convierte correctamente
consola.imprimir("Nulo como texto: '" + nulo_como_texto + "'")

// Probar concatenación con nulo
texto resultado = "Valor: " + valor_nulo
consola.imprimir(resultado)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_nulo_en_condicionales() {
        let codigo = r#"
// Usar nulo en estructuras condicionales
jsn config = {
    "modo": "desarrollo",
    "puerto": 3000
}

// Verificar configuración opcional
texto var host = config["host"]
si (host == nulo) {
    consola.imprimir("Host no configurado, usando localhost")
    host = "localhost"
} sino {
    consola.imprimir("Host configurado: " + host)
}

// Verificar otra configuración opcional
entero timeout = config["timeout"]
si (timeout != nulo) {
    consola.imprimir("Timeout configurado: " + timeout)
} sino {
    consola.imprimir("Timeout no configurado, usando valor por defecto")
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_nulo_en_listas() {
        let codigo = r#"
// Probar nulo en listas
lista valores = [1, nulo, "texto", nulo, verdadero]

// Iterar y verificar valores nulos
para (entero var i = 0; i < valores.longitud(); i++) {
    // Usar tipo auto para elementos de lista mixta
    auto elemento = valores[i]
    si (elemento == nulo) {
        consola.imprimir("Elemento " + i + " es nulo")
    } sino {
        consola.imprimir("Elemento " + i + ": " + elemento)
    }
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_nulo_con_objetos_json_anidados() {
        let codigo = r#"
// Probar nulo con objetos JSON anidados (simplificado)
jsn usuario = {
    "perfil": {
        "nombre": "Carlos",
        "edad": 30
    },
    "configuracion": {
        "tema": "oscuro"
    }
}

// Acceder a propiedades de primer nivel que no existen
texto apellido = usuario["apellido"]
entero puntos = usuario["puntos"]

// Verificar que devuelven nulo
si (apellido == nulo) {
    consola.imprimir("Apellido no configurado")
}

si (puntos == nulo) {
    consola.imprimir("Puntos no disponibles")
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_nulo_validacion_tipo() {
        let codigo = r#"
// Validar que nulo es compatible con el sistema de tipos
auto valor = nulo

// Asignar nulo a diferentes tipos y verificar compatibilidad
texto var texto_var = "inicial"
texto_var = nulo
consola.imprimir("Texto después de asignar nulo: " + texto_var)

entero var entero_var = 42
entero_var = nulo
consola.imprimir("Entero después de asignar nulo: " + entero_var)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_nulo_en_operaciones_logicas() {
        let codigo = r#"
// Probar nulo en operaciones lógicas
variable valor_nulo = nulo
texto valor_texto = "no nulo"

// Operaciones AND con nulo
si (valor_nulo == nulo y valor_texto != nulo) {
    consola.imprimir("Operación AND correcta")
}

// Operaciones OR con nulo
si (valor_nulo == nulo o valor_texto == nulo) {
    consola.imprimir("Operación OR correcta")
}

// Verificar múltiples condiciones con nulo
jsn datos = {"activo": verdadero}
si (datos["nombre"] == nulo y datos["activo"] == verdadero) {
    consola.imprimir("Nombre no configurado pero está activo")
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_nulo_casos_limite() {
        let codigo = r#"
// Casos límite y edge cases con nulo (simplificado)
jsn objeto_vacio = {}

// Acceso a objeto vacío
auto valor1 = objeto_vacio["cualquier_clave"]
si (valor1 == nulo) {
    consola.imprimir("Objeto vacío devuelve nulo correctamente")
}

// Comparación simple con nulo
auto resultado_texto = (valor1 == nulo) ? "es nulo" : "no es nulo"
consola.imprimir("Resultado: " + resultado_texto)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }
}
