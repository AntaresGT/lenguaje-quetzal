use crate::nucleo::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agregar_basico() {
        let codigo = r#"
lista var numeros = [1, 2, 3]
numeros.agregar(4)
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_agregar_multiple() {
        let codigo = r#"
lista var datos = []
datos.agregar(1)
datos.agregar(2)
datos.agregar(3)
entero longitud = datos.longitud()
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_agregar_tipos_diferentes() {
        let codigo = r#"
lista var mixta = []
mixta.agregar(1)
mixta.agregar("texto")
mixta.agregar(verdadero)
mixta.agregar(3.14)
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_agregar_con_conversiones() {
        let codigo = r#"
lista var cadenas = ["uno", "dos"]
cadenas.agregar(3.texto())
cadenas.agregar(verdadero.texto())
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_agregar_expresiones_complejas() {
        let codigo = r#"
lista var numeros = [1, 2]
numeros.agregar(1 + 2)
numeros.agregar(numeros.longitud())
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_agregar_sin_argumentos() {
        let codigo = r#"
lista var datos = [1, 2, 3]
datos.agregar()
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_err()); // Debe fallar sin argumentos
    }

    #[test]
    fn test_agregar_multiples_argumentos() {
        let codigo = r#"
lista var datos = [1, 2, 3]
datos.agregar(4, 5)
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_err()); // Debe fallar con múltiples argumentos
    }

    #[test]
    fn test_verificar_persistencia_cambios() {
        let codigo = r#"
lista var numeros = [1, 2, 3]
entero longitud_antes = numeros.longitud()
numeros.agregar(4)
entero longitud_despues = numeros.longitud()
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_agregar_en_bucle() {
        let codigo = r#"
lista var datos = []
entero i = 0
mientras (i < 5) {
    datos.agregar(i)
    i = i + 1
}
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_agregar_con_acceso_posterior() {
        let codigo = r#"
lista var numeros = [1, 2, 3]
numeros.agregar(4)
entero ultimo = numeros.ultimo()
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_agregar_con_asignacion_indice() {
        let codigo = r#"
lista var datos = [1, 2]
datos.agregar(3)
datos[0] = 10
datos.agregar(4)
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_agregar_listas_anidadas() {
        let codigo = r#"
lista var principal = [[1, 2], [3, 4]]
lista nueva = [5, 6]
principal.agregar(nueva)
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }
}
