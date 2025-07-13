#[cfg(test)]
mod tests {
    use crate::interprete::interpretar;

    #[test]
    fn test_analisis_texto_avanzado() {
        let codigo = r#"
            cadena texto = "El lenguaje Quetzal es poderoso y versátil"
            
            lista palabras = texto.dividir(" ")
            entero total_palabras = palabras.longitud()
            entero total_caracteres = texto.longitud()
            
            consola.imprimir("Análisis de texto:")
            consola.imprimir("Palabras: " + total_palabras.cadena())
            consola.imprimir("Caracteres: " + total_caracteres.cadena())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_procesamiento_cadenas_complejas() {
        let codigo = r#"
            cadena texto = "  Hola Mundo Quetzal  "
            
            cadena limpio = texto.recortar()
            cadena minusculas = limpio.a_minusculas()
            lista palabras = minusculas.dividir(" ")
            cadena unido = palabras.unir("-")
            
            consola.imprimir("Texto procesado: " + unido)
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_busqueda_y_reemplazo() {
        let codigo = r#"
            cadena texto = "Quetzal es un lenguaje de programación. Quetzal es fácil de usar."
            
            cadena nuevo_texto = texto.reemplazar("Quetzal", "Q-Lang")
            
            consola.imprimir("Texto modificado: " + nuevo_texto)
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_validacion_cadenas() {
        let codigo = r#"
            cadena email = "usuario@dominio.com"
            
            bool tiene_arroba = email.contiene("@")
            bool tiene_punto = email.contiene(".")
            bool empieza_letra = email.empieza_con("u")
            bool termina_com = email.termina_con(".com")
            
            consola.imprimir("Validación email:")
            consola.imprimir("Tiene @: " + tiene_arroba.cadena())
            consola.imprimir("Tiene .: " + tiene_punto.cadena())
            consola.imprimir("Empieza con 'u': " + empieza_letra.cadena())
            consola.imprimir("Termina con '.com': " + termina_com.cadena())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_manipulacion_avanzada_texto() {
        let codigo = r#"
            cadena frase = "programación con quetzal"
            lista palabras = frase.dividir(" ")
            
            cadena item = palabras[0].cadena()
            cadena item_caps = item.a_mayusculas()
            
            consola.imprimir("Texto: " + frase)
            consola.imprimir("Primera palabra en mayúsculas: " + item_caps)
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_estadisticas_texto() {
        let codigo = r#"
            cadena texto = "Quetzal es un lenguaje moderno y potente para desarrollo"
            
            entero total_chars = texto.longitud()
            lista palabras = texto.dividir(" ")
            entero total_palabras = palabras.longitud()
            
            cadena primera_palabra = palabras[0].cadena()
            cadena segunda_palabra = palabras[1].cadena()
            
            consola.imprimir("Estadísticas del texto:")
            consola.imprimir("Caracteres totales: " + total_chars.cadena())
            consola.imprimir("Palabras totales: " + total_palabras.cadena())
            consola.imprimir("Primera palabra: " + primera_palabra)
            consola.imprimir("Segunda palabra: " + segunda_palabra)
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }
}
