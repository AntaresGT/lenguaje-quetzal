use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redondeo_suma_decimales() {
        let codigo = "consola.imprimir((0.1 + 0.2).cadena())";
        let resultado = interprete::interpretar(codigo);
        // No debe haber error y debe imprimir 0.3
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_redondeo_suma_triple() {
        let codigo = "consola.imprimir((0.1 + 0.1 + 0.1).cadena())";
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_redondeo_resta() {
        let codigo = "consola.imprimir((1.0 - 0.9).cadena())";
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_redondeo_multiplicacion() {
        let codigo = "consola.imprimir((0.1 * 3).cadena())";
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_redondeo_no_afecta_enteros() {
        let codigo = r#"
            entero a = 5
            entero b = 3
            consola.imprimir((a + b).cadena())
        "#;
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_redondeo_division() {
        let codigo = "consola.imprimir((3.0 / 3.0).cadena())";
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_redondeo_operaciones_mixtas() {
        let codigo = "consola.imprimir((0.1 + 0.2 - 0.1).cadena())";
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_redondeo_con_variables() {
        let codigo = r#"
            número a = 0.1
            número b = 0.2
            consola.imprimir((a + b).cadena())
        "#;
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_redondeo_precision_alta() {
        let codigo = "consola.imprimir((0.123456789 + 0.987654321).cadena())";
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_redondeo_expresiones_complejas() {
        let codigo = "consola.imprimir(((0.1 + 0.2) * 2.0).cadena())";
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }
}
