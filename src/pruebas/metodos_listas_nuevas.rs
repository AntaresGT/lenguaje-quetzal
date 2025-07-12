use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_longitud_listas() {
        let codigo = r#"
lista numeros = [1, 2, 3, 4, 5]
entero longitud = numeros.longitud()
imprimir(longitud)
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_longitud_lista_vacia() {
        let codigo = r#"
lista vacia = []
entero longitud = vacia.longitud()
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_primer_elemento() {
        let codigo = r#"
lista numeros = [10, 20, 30]
entero primero = numeros.primero()
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_ultimo_elemento() {
        let codigo = r#"
lista numeros = [10, 20, 30]
entero ultimo = numeros.ultimo()
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_primer_elemento_lista_vacia() {
        let codigo = r#"
lista vacia = []
entero primero = vacia.primero()
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_err()); // Debe fallar
    }

    #[test]
    fn test_ultimo_elemento_lista_vacia() {
        let codigo = r#"
lista vacia = []
entero ultimo = vacia.ultimo()
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_err()); // Debe fallar
    }

    #[test]
    fn test_ordenar_numeros() {
        let codigo = r#"
lista mut numeros = [5, 1, 3, 2, 4]
numeros.ordenar()
imprimir(numeros)
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_ordenar_cadenas() {
        let codigo = r#"
lista mut nombres = ["Carlos", "Ana", "Beatriz"]
nombres.ordenar()
imprimir(nombres)
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_esta_vacia_lista() {
        let codigo = r#"
lista vacia = []
lista llena = [1, 2, 3]
bool vacia_result = vacia.esta_vacia()
bool llena_result = llena.esta_vacia()
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_asignacion_por_indice() {
        let codigo = r#"
lista mut numeros = [10, 20, 30, 40]
numeros[0] = 100
numeros[3] = 400
imprimir(numeros)
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_asignacion_indice_cadenas() {
        let codigo = r#"
lista mut frutas = ["manzana", "banana", "naranja"]
frutas[1] = "kiwi"
imprimir(frutas)
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_asignacion_indice_fuera_de_rango() {
        let codigo = r#"
lista numeros = [1, 2, 3]
numeros[10] = 100
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_err()); // Debe fallar
    }

    #[test]
    fn test_metodo_agregar() {
        let codigo = r#"
lista mut numeros = [1, 2, 3]
numeros.agregar(4)
numeros.agregar(5)
imprimir(numeros)
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_agregar_diferentes_tipos() {
        let codigo = r#"
lista mut mixta = ["uno", "dos"]
mixta.agregar(3.cadena())
mixta.agregar(verdadero.cadena())
imprimir(mixta)
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_compatibilidad_longitud_cadena_lista() {
        let codigo = r#"
cadena texto = "Hola Quetzal"
lista numeros = [10, 20, 30]
entero longitud_texto = texto.longitud()
entero longitud_lista = numeros.longitud()
imprimir(longitud_texto)
imprimir(longitud_lista)
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_esta_vacia_cadena_y_lista() {
        let codigo = r#"
cadena texto_vacio = ""
cadena texto_lleno = "contenido"
lista lista_vacia = []
lista lista_llena = [1, 2, 3]

bool texto_vacio_result = texto_vacio.esta_vacia()
bool texto_lleno_result = texto_lleno.esta_vacia()
bool lista_vacia_result = lista_vacia.esta_vacia()
bool lista_llena_result = lista_llena.esta_vacia()
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_metodos_encadenados_con_listas() {
        let codigo = r#"
lista mut numeros = [5, 1, 3, 2, 4]
numeros.ordenar()
entero longitud = numeros.longitud()
entero primero = numeros.primero()
entero ultimo = numeros.ultimo()
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_modificaciones_multiples() {
        let codigo = r#"
lista mut datos = [1, 2, 3]
datos.agregar(4)
datos[0] = 10
datos.agregar(5)
datos[1] = 20
imprimir(datos)
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_lista_mixta_con_metodos() {
        let codigo = r#"
lista mut mixta = [1, "texto", verdadero, 3.14]
entero longitud = mixta.longitud()
mixta.agregar("nuevo")
entero nueva_longitud = mixta.longitud()
        "#;
        
        let resultado = interprete::interpretar(codigo);
        assert!(resultado.is_ok());
    }
}
