// Prueba unitaria para el método esta_vacia() de tipos texto

use crate::nucleo::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metodo_esta_vacia_texto_no_vacia() {
        let codigo = r#"
            texto mi_texto = "Hola"
            log resultado = mi_texto.esta_vacia()
            consola.mostrar("Resultado: " + resultado.texto())
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_metodo_esta_vacia_texto_vacia() {
        let codigo = r#"
            texto texto_vacio = ""
            log resultado = texto_vacio.esta_vacia()
            consola.mostrar("Texto vacío está vacío: " + resultado.texto())
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_metodo_esta_vacia_texto_con_espacios() {
        let codigo = r#"
            texto con_espacios = "   "
            log resultado = con_espacios.esta_vacia()
            consola.mostrar("Texto con espacios está vacío: " + resultado.texto())
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_metodo_esta_vacia_texto_unicode() {
        let codigo = r#"
            texto unicode = "🚀"
            log resultado = unicode.esta_vacia()
            consola.mostrar("Texto Unicode está vacío: " + resultado.texto())
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_metodo_esta_vacia_comparacion_con_longitud() {
        let codigo = r#"
            texto prueba1 = ""
            texto prueba2 = "abc"
            
            log vacia1 = prueba1.esta_vacia()
            log vacia2 = prueba2.esta_vacia()
            entero long1 = prueba1.longitud()
            entero long2 = prueba2.longitud()
            
            consola.mostrar("Cadena vacía - está_vacia: " + vacia1.texto() + ", longitud: " + long1.texto())
            consola.mostrar("Cadena 'abc' - está_vacia: " + vacia2.texto() + ", longitud: " + long2.texto())
        "#;

        assert!(interprete::interpretar(codigo).is_ok());
    }
}
