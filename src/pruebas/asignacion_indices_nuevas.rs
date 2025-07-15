use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asignacion_indice_simple() {
        let codigo = r#"
lista var numeros = [1, 2, 3]
numeros[0] = 10
numeros[1] = 20
numeros[2] = 30
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_asignacion_indice_tipos_diferentes() {
        let codigo = r#"
lista var valores = [1, 2, 3]
valores[0] = "texto"
valores[1] = verdadero
valores[2] = 3.14
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_asignacion_indice_negativo() {
        let codigo = r#"
lista var numeros = [1, 2, 3]
numeros[-1] = 100
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_err()); // Debe fallar con índice negativo
    }

    #[test]
    fn test_asignacion_indice_muy_grande() {
        let codigo = r#"
lista var numeros = [1, 2, 3]
numeros[1000] = 100
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_err()); // Debe fallar fuera de rango
    }

    #[test]
    fn test_asignacion_indice_con_expresiones() {
        let codigo = r#"
lista var numeros = [10, 20, 30, 40, 50]
entero indice = 2
numeros[indice] = 999
numeros[indice + 1] = 888
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_asignacion_indice_en_bucle() {
        let codigo = r#"
lista var datos = [1, 2, 3, 4, 5]
entero i = 0
mientras (i < datos.longitud()) {
    datos[i] = datos[i] * 2
    i = i + 1
}
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_lectura_despues_asignacion() {
        let codigo = r#"
lista var numeros = [1, 2, 3]
numeros[1] = 100
entero valor = numeros[1]
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_asignacion_lista_vacia() {
        let codigo = r#"
lista vacia = []
vacia[0] = 10
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_err()); // Debe fallar en lista vacía
    }

    #[test]
    fn test_asignacion_con_metodos() {
        let codigo = r#"
lista var numeros = [1, 2, 3]
numeros[0] = numeros.longitud()
numeros[1] = numeros.primero()
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_asignacion_multiple_mismo_indice() {
        let codigo = r#"
lista var datos = [1, 2, 3]
datos[1] = 10
datos[1] = 20
datos[1] = 30
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }
}
