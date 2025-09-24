// Prueba unitaria para el método minusculas() de tipos texto

use crate::nucleo::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metodo_minusculas_texto_simple() {
        let codigo = r#"
            texto mi_texto = "HOLA MUNDO"
            texto resultado = mi_texto.minusculas()
            consola.mostrar(resultado)
        "#;
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_metodo_minusculas_texto_mixto() {
        let codigo = r#"
            texto mi_texto = "HoLa MuNdO"
            texto resultado = mi_texto.minusculas()
            consola.mostrar(resultado)
        "#;
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_metodo_minusculas_texto_con_numeros() {
        let codigo = r#"
            texto mi_texto = "QUETZAL123"
            texto resultado = mi_texto.minusculas()
            consola.mostrar(resultado)
        "#;
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_metodo_minusculas_texto_vacio() {
        let codigo = r#"
            texto mi_texto = ""
            texto resultado = mi_texto.minusculas()
            consola.mostrar(resultado)
        "#;
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_metodo_minusculas_texto_con_acentos() {
        let codigo = r#"
            texto mi_texto = "PROGRAMACIÓN"
            texto resultado = mi_texto.minusculas()
            consola.mostrar(resultado)
        "#;
        assert!(interprete::interpretar(codigo).is_ok());
    }
}
