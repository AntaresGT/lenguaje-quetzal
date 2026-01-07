// Pruebas unitarias para métodos de texto del lenguaje Quetzal

use super::auxiliares::*;

#[test]
fn prueba_longitud_texto() {
    let codigo = r#"
        texto mi_texto = "Hola Mundo"
        entero tamaño = mi_texto.longitud()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "tamaño", 10));
}

#[test]
fn prueba_conversion_texto_a_entero() {
    let codigo = r#"
        texto texto_num = "123"
        entero valor_numero = texto_num.entero()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "valor_numero", 123));
}

#[test]
fn prueba_mayusculas_minusculas() {
    let codigo = r#"
        texto mi_texto = "Hola Mundo"
        texto mayusculas = mi_texto.mayusculas()
        texto minusculas = mi_texto.minusculas()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "mayusculas", "HOLA MUNDO"));
    assert!(verificar_variable_texto(&entorno, "minusculas", "hola mundo"));
}

#[test]
fn prueba_capitalizar() {
    let codigo = r#"
        texto mi_texto = "hola mundo"
        texto capitalizado = mi_texto.capitalizar()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "capitalizado", "Hola mundo"));
}

#[test]
fn prueba_recortar() {
    let codigo = r#"
        texto con_espacios = " Hola Mundo "
        texto recortado = con_espacios.recortar()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "recortado", "Hola Mundo"));
}

#[test]
fn prueba_contiene() {
    let codigo = r#"
        texto mi_texto = "Hola Mundo"
        log contiene = mi_texto.contiene("Mundo")
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_logico(&entorno, "contiene", true));
}

#[test]
fn prueba_empieza_con_termina_con() {
    let codigo = r#"
        texto mi_texto = "Hola Mundo"
        log empieza_con = mi_texto.empieza_con("Hola")
        log termina_con = mi_texto.termina_con("Mundo")
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_logico(&entorno, "empieza_con", true));
    assert!(verificar_variable_logico(&entorno, "termina_con", true));
}

#[test]
fn prueba_reemplazar() {
    let codigo = r#"
        texto mi_texto = "Hola Mundo"
        texto reemplazado = mi_texto.reemplazar("Mundo", "Quetzal")
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "reemplazado", "Hola Quetzal"));
}

#[test]
fn prueba_dividir() {
    let codigo = r#"
        texto mi_texto = "Hola Mundo"
        lista<texto> partes = mi_texto.dividir(" ")
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_repetir() {
    let codigo = r#"
        texto mi_texto = "Hola"
        texto repetido = mi_texto.repetir(3)
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "repetido", "HolaHolaHola"));
}

#[test]
fn prueba_subtexto() {
    let codigo = r#"
        texto mi_texto = "Hola Mundo"
        texto subtexto = mi_texto.subtexto(0, 4)
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "subtexto", "Hola"));
}

#[test]
fn prueba_izquierda_derecha() {
    let codigo = r#"
        texto mi_texto = "Hola Mundo"
        texto izquierda = mi_texto.izquierda(4)
        texto derecha = mi_texto.derecha(5)
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "izquierda", "Hola"));
    assert!(verificar_variable_texto(&entorno, "derecha", "Mundo"));
}

#[test]
fn prueba_es_numero() {
    let codigo = r#"
        log es_numero = "12345".es_numero()
        log no_es_numero = "abc123".es_numero()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_logico(&entorno, "es_numero", true));
    assert!(verificar_variable_logico(&entorno, "no_es_numero", false));
}

#[test]
fn prueba_invertir() {
    let codigo = r#"
        texto mi_texto = "Hola"
        texto invertido = mi_texto.invertir()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "invertido", "aloH"));
}

#[test]
fn prueba_interpolacion_basica() {
    // Prueba de interpolación de texto con t"..."
    let codigo = r#"
        texto nombre = "María"
        texto saludo = t"¡Hola {nombre}!"
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "saludo", "¡Hola María!"));
}

#[test]
fn prueba_interpolacion_con_expresiones() {
    // Prueba de interpolación con expresiones aritméticas
    let codigo = r#"
        entero a = 10
        entero b = 5
        texto calculo = t"La suma de {a} + {b} es {a + b}"
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "calculo", "La suma de 10 + 5 es 15"));
}

#[test]
fn prueba_base64_codificar() {
    // Prueba de codificación a base64
    let codigo = r#"
        texto mi_texto = "Hola Mundo"
        texto base64 = mi_texto.a_base64()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "base64", "SG9sYSBNdW5kbw=="));
}

#[test]
fn prueba_base64_decodificar() {
    // Prueba de decodificación de base64
    let codigo = r#"
        texto decodificado = "SG9sYSBNdW5kbw==".decodificar_base64()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "decodificado", "Hola Mundo"));
}

#[test]
fn prueba_url_codificar() {
    // Prueba de codificación a URL
    let codigo = r#"
        texto url_codificada = "Hola Mundo!".a_url()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "url_codificada", "Hola%20Mundo%21"));
}

#[test]
fn prueba_acceso_texto_por_indice() {
    // Prueba de acceso a caracter por índice
    let codigo = r#"
        texto mi_texto = "Hola"
        texto caracter = mi_texto[1]
        texto ultimo = mi_texto[-1]
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "caracter", "o"));
    assert!(verificar_variable_texto(&entorno, "ultimo", "a"));
}

#[test]
fn prueba_contar_ocurrencias() {
    // Prueba de contar ocurrencias de un texto
    let codigo = r#"
        texto mi_texto = "abracadabra"
        entero conteo = mi_texto.contar("a")
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "conteo", 5));
}