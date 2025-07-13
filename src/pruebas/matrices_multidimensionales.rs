#[cfg(test)]
mod tests {
    use crate::interprete::interpretar;

    #[test]
    fn test_matriz_4d_basica() {
        let codigo = r#"
            lista matriz_4d = [
                [
                    [[1, 2], [3, 4]],
                    [[5, 6], [7, 8]]
                ],
                [
                    [[9, 10], [11, 12]],
                    [[13, 14], [15, 16]]
                ]
            ]
            
            consola.imprimir("Matriz 4D creada: " + matriz_4d.cadena())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_acceso_matriz_4d() {
        let codigo = r#"
            lista matriz_4d = [
                [
                    [[1, 2], [3, 4]],
                    [[5, 6], [7, 8]]
                ],
                [
                    [[9, 10], [11, 12]],
                    [[13, 14], [15, 16]]
                ]
            ]
            
            entero valor = matriz_4d[0][1][0][1]
            consola.imprimir("Valor [0][1][0][1]: " + valor.cadena())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_asignacion_matriz_4d() {
        let codigo = r#"
            lista mut matriz_4d = [
                [
                    [[1, 2], [3, 4]],
                    [[5, 6], [7, 8]]
                ],
                [
                    [[9, 10], [11, 12]],
                    [[13, 14], [15, 16]]
                ]
            ]
            
            matriz_4d[1][0][1][0] = 999
            entero valor_modificado = matriz_4d[1][0][1][0]
            consola.imprimir("Matriz modificada - nuevo valor: " + valor_modificado.cadena())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_matriz_3d_con_diferentes_tipos() {
        let codigo = r#"
            lista matriz_3d = [
                [
                    [1, "texto", 3],
                    [verdadero, 5, "hola"]
                ],
                [
                    ["mundo", 7, falso],
                    [42, "quetzal", 8]
                ]
            ]
            
            consola.imprimir("Matriz 3D mixta creada")
            cadena elemento = matriz_3d[0][1][2].cadena()
            consola.imprimir("Elemento [0][1][2]: " + elemento)
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }

    #[test]
    fn test_modificacion_matriz_3d() {
        let codigo = r#"
            lista mut matriz_3d = [
                [[1, 2], [3, 4]],
                [[5, 6], [7, 8]]
            ]
            
            matriz_3d[0][0][1] = 99
            matriz_3d[1][1][0] = 100
            
            entero valor1 = matriz_3d[0][0][1]
            entero valor2 = matriz_3d[1][1][0]
            consola.imprimir("Matriz 3D modificada")
            consola.imprimir("Valor [0][0][1]: " + valor1.cadena())
            consola.imprimir("Valor [1][1][0]: " + valor2.cadena())
        "#;
        
        let resultado = interpretar(codigo);
        assert!(resultado.is_ok());
    }
}
