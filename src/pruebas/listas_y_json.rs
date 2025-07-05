use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_listas_basicas() {
        let codigo = r#"
lista numeros = [1, 2, 3, 4, 5]
lista<entero> enteros = [10, 20, 30]
lista<cadena> textos = ["hola", "mundo", "quetzal"]
lista mixta = [1, "texto", verdadero, 3.14]
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_listas_vacias() {
        let codigo = r#"
lista vacia = []
lista<entero> enteros_vacios = []
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_listas_anidadas() {
        let codigo = r#"
lista matriz = [[1, 2], [3, 4], [5, 6]]
lista compleja = [1, [2, 3], "texto", [verdadero, falso]]
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_acceso_elementos_lista() {
        let codigo = r#"
lista numeros = [10, 20, 30, 40, 50]
// Nota: El acceso por índice requeriría implementación adicional
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_json_basico() {
        let codigo = r#"
jsn persona = {
    nombre: "Juan",
    edad: 30,
    activo: verdadero
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_json_anidado() {
        let codigo = r#"
jsn configuracion = {
    servidor: {
        host: "localhost",
        puerto: 8080
    },
    base_datos: {
        nombre: "mi_db",
        usuario: "admin"
    }
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_json_con_arrays() {
        let codigo = r#"
jsn datos = {
    titulo: "Proyecto",
    numeros: [1, 2, 3, 4, 5],
    configuraciones: {
        opciones: ["opcion1", "opcion2"],
        valores: [verdadero, falso, verdadero]
    }
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_json_con_diferentes_tipos() {
        let codigo = r#"
jsn completo = {
    cadena: "texto",
    entero: 42,
    decimal: 3.14159,
    booleano: verdadero,
    lista: [1, 2, 3],
    objeto: {
        subcampo: "valor"
    }
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_json_vacio() {
        let codigo = r#"
jsn vacio = {}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_json_con_variables() {
        let codigo = r#"
cadena nombre = "Ana"
entero edad = 25
jsn usuario = {
    nombre: nombre,
    edad: edad,
    registrado: verdadero
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_lista_con_variables() {
        let codigo = r#"
entero a = 10
entero b = 20
entero c = 30
lista numeros = [a, b, c]
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_tipos_complejos_mutables() {
        let codigo = r#"
lista mut numeros_mutables = [1, 2, 3]
jsn mut configuracion_mutable = {
    debug: verdadero,
    version: "1.0"
}
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }
}
