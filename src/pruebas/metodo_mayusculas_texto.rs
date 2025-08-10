// Prueba unitaria para el método mayusculas() de tipos texto

use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metodo_mayusculas_texto_simple() {
        let codigo = r#"
            texto mi_texto = "hola mundo"
            texto resultado = mi_texto.mayusculas()
            consola.mostrar(resultado)
        "#;
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_metodo_mayusculas_texto_mixto() {
        let codigo = r#"
            texto mi_texto = "HoLa MuNdO"
            texto resultado = mi_texto.mayusculas()
            consola.mostrar(resultado)
        "#;
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_metodo_mayusculas_texto_con_numeros() {
        let codigo = r#"
            texto mi_texto = "quetzal123"
            texto resultado = mi_texto.mayusculas()
            consola.mostrar(resultado)
        "#;
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_metodo_mayusculas_texto_vacio() {
        let codigo = r#"
            texto mi_texto = ""
            texto resultado = mi_texto.mayusculas()
            consola.mostrar(resultado)
        "#;
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_metodo_mayusculas_texto_con_acentos() {
        let codigo = r#"
            texto mi_texto = "programación"
            texto resultado = mi_texto.mayusculas()
            consola.mostrar(resultado)
        "#;
        assert!(interprete::interpretar(codigo).is_ok());
    }
}
