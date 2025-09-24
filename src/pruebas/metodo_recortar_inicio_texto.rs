// Pruebas para el método recortar_inicio de tipos texto
// Este método debe eliminar espacios en blanco al inicio del texto

#[cfg(test)]
pub mod tests {
    use crate::nucleo::interprete;

    #[test]
    fn test_metodo_recortar_inicio_texto_con_espacios_inicio() {
        let codigo = r#"
            texto con_espacios = "   Hola Mundo"
            texto resultado = con_espacios.recortar_inicio()
            resultado
        "#;
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
        if let crate::datos::tipos_datos::Valor::Texto(texto) = resultado.unwrap() {
            assert_eq!(texto, "Hola Mundo");
        } else {
            panic!("Se esperaba un valor de tipo texto");
        }
    }

    #[test]
    fn test_metodo_recortar_inicio_texto_con_espacios_ambos_lados() {
        let codigo = r#"
            texto con_espacios = "   Hola Mundo   "
            texto resultado = con_espacios.recortar_inicio()
            resultado
        "#;
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
        if let crate::datos::tipos_datos::Valor::Texto(texto) = resultado.unwrap() {
            assert_eq!(texto, "Hola Mundo   ");
        } else {
            panic!("Se esperaba un valor de tipo texto");
        }
    }

    #[test]
    fn test_metodo_recortar_inicio_texto_sin_espacios() {
        let codigo = r#"
            texto sin_espacios = "Hola Mundo"
            texto resultado = sin_espacios.recortar_inicio()
            resultado
        "#;
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
        if let crate::datos::tipos_datos::Valor::Texto(texto) = resultado.unwrap() {
            assert_eq!(texto, "Hola Mundo");
        } else {
            panic!("Se esperaba un valor de tipo texto");
        }
    }

    #[test]
    fn test_metodo_recortar_inicio_solo_espacios() {
        let codigo = r#"
            texto solo_espacios = "   "
            texto resultado = solo_espacios.recortar_inicio()
            resultado
        "#;
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
        if let crate::datos::tipos_datos::Valor::Texto(texto) = resultado.unwrap() {
            assert_eq!(texto, "");
        } else {
            panic!("Se esperaba un valor de tipo texto");
        }
    }

    #[test]
    fn test_metodo_recortar_inicio_con_tabs_y_saltos() {
        let codigo = r#"
            texto con_tabs = "\t\n  Hola Mundo"
            texto resultado = con_tabs.recortar_inicio()
            resultado
        "#;
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
        if let crate::datos::tipos_datos::Valor::Texto(texto) = resultado.unwrap() {
            assert_eq!(texto, "Hola Mundo");
        } else {
            panic!("Se esperaba un valor de tipo texto");
        }
    }
}
