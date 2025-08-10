// Pruebas para el método recortar_final de tipos texto
// Este método debe eliminar espacios en blanco al final del texto

#[cfg(test)]
pub mod tests {
    use crate::interprete;

    #[test]
    fn test_metodo_recortar_final_texto_con_espacios_final() {
        let codigo = r#"
            texto con_espacios = "Hola Mundo   "
            texto resultado = con_espacios.recortar_final()
            resultado
        "#;
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
        if let crate::tipos_datos::Valor::Texto(texto) = resultado.unwrap() {
            assert_eq!(texto, "Hola Mundo");
        } else {
            panic!("Se esperaba un valor de tipo texto");
        }
    }

    #[test]
    fn test_metodo_recortar_final_texto_con_espacios_ambos_lados() {
        let codigo = r#"
            texto con_espacios = "   Hola Mundo   "
            texto resultado = con_espacios.recortar_final()
            resultado
        "#;
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
        if let crate::tipos_datos::Valor::Texto(texto) = resultado.unwrap() {
            assert_eq!(texto, "   Hola Mundo");
        } else {
            panic!("Se esperaba un valor de tipo texto");
        }
    }

    #[test]
    fn test_metodo_recortar_final_texto_sin_espacios() {
        let codigo = r#"
            texto sin_espacios = "Hola Mundo"
            texto resultado = sin_espacios.recortar_final()
            resultado
        "#;
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
        if let crate::tipos_datos::Valor::Texto(texto) = resultado.unwrap() {
            assert_eq!(texto, "Hola Mundo");
        } else {
            panic!("Se esperaba un valor de tipo texto");
        }
    }

    #[test]
    fn test_metodo_recortar_final_solo_espacios() {
        let codigo = r#"
            texto solo_espacios = "   "
            texto resultado = solo_espacios.recortar_final()
            resultado
        "#;
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
        if let crate::tipos_datos::Valor::Texto(texto) = resultado.unwrap() {
            assert_eq!(texto, "");
        } else {
            panic!("Se esperaba un valor de tipo texto");
        }
    }

    #[test]
    fn test_metodo_recortar_final_con_tabs_y_saltos() {
        let codigo = r#"
            texto con_tabs = "Hola Mundo\t\n  "
            texto resultado = con_tabs.recortar_final()
            resultado
        "#;
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
        if let crate::tipos_datos::Valor::Texto(texto) = resultado.unwrap() {
            assert_eq!(texto, "Hola Mundo");
        } else {
            panic!("Se esperaba un valor de tipo texto");
        }
    }
}
