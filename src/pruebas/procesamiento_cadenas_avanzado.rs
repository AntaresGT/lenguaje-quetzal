#[cfg(test)]
mod tests {
    use crate::interprete::interpretar;

    #[test]
    fn test_analisis_texto_avanzado() {
        let codigo = r#"
            texto mi_texto = "El lenguaje Quetzal es poderoso y versátil"
            
            lista palabras = mi_texto.dividir(" ")
            entero total_palabras = palabras.longitud()
            entero total_caracteres = mi_texto.longitud()
            
            consola.imprimir("Análisis de texto:")
            consola.imprimir("Palabras: " + total_palabras.texto())
            consola.imprimir("Caracteres: " + total_caracteres.texto())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_procesamiento_cadenas_complejas() {
        let codigo = r#"
            texto mi_texto = "  Hola Mundo Quetzal  "
            
            texto limpio = mi_texto.recortar()
            texto minusculas = limpio.a_minusculas()
            lista palabras = minusculas.dividir(" ")
            texto unido = palabras.unir("-")
            
            consola.imprimir("Texto procesado: " + unido)
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_busqueda_y_reemplazo() {
        let codigo = r#"
            texto mi_texto = "Quetzal es un lenguaje de programación. Quetzal es fácil de usar."
            
            texto nuevo_texto = mi_texto.reemplazar("Quetzal", "Q-Lang")
            
            consola.imprimir("Texto modificado: " + nuevo_texto)
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_validacion_cadenas() {
        let codigo = r#"
            texto email = "usuario@dominio.com"
            
            log tiene_arroba = email.contiene("@")
            log tiene_punto = email.contiene(".")
            log empieza_letra = email.empieza_con("u")
            log termina_com = email.termina_con(".com")
            
            consola.imprimir("Validación email:")
            consola.imprimir("Tiene @: " + tiene_arroba.texto())
            consola.imprimir("Tiene .: " + tiene_punto.texto())
            consola.imprimir("Empieza con 'u': " + empieza_letra.texto())
            consola.imprimir("Termina con '.com': " + termina_com.texto())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_manipulacion_avanzada_texto() {
        let codigo = r#"
            texto frase = "programación con quetzal"
            lista palabras = frase.dividir(" ")
            
            texto item = palabras[0].texto()
            texto item_caps = item.a_mayusculas()
            
            consola.imprimir("Texto: " + frase)
            consola.imprimir("Primera palabra en mayúsculas: " + item_caps)
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_estadisticas_texto() {
        let codigo = r#"
            texto mi_texto = "Quetzal es un lenguaje moderno y potente para desarrollo"
            
            entero total_chars = mi_texto.longitud()
            lista palabras = mi_texto.dividir(" ")
            entero total_palabras = palabras.longitud()
            
            texto primera_palabra = palabras[0].texto()
            texto segunda_palabra = palabras[1].texto()
            
            consola.imprimir("Estadísticas del texto:")
            consola.imprimir("Caracteres totales: " + total_chars.texto())
            consola.imprimir("Palabras totales: " + total_palabras.texto())
            consola.imprimir("Primera palabra: " + primera_palabra)
            consola.imprimir("Segunda palabra: " + segunda_palabra)
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }
}
