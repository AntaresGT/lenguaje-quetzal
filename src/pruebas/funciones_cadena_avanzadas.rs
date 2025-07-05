use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_longitud_cadena() {
        let codigo = r#"
cadena texto = "Hola mundo"
entero longitud = texto.longitud()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_esta_vacia() {
        let codigo = r#"
cadena texto_vacio = ""
cadena texto_lleno = "contenido"
bool es_vacio = texto_vacio.esta_vacia()
bool es_lleno = texto_lleno.esta_vacia()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_buscar_patron() {
        let codigo = r#"
cadena texto = "Hola mundo cruel"
entero posicion = texto.buscar("mundo")
entero no_encontrado = texto.buscar("xyz")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_contiene_patron() {
        let codigo = r#"
cadena texto = "Hola mundo"
bool contiene_hola = texto.contiene("Hola")
bool contiene_xyz = texto.contiene("xyz")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_empieza_con() {
        let codigo = r#"
cadena texto = "Hola mundo"
bool empieza_hola = texto.empieza_con("Hola")
bool empieza_mundo = texto.empieza_con("mundo")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_termina_con() {
        let codigo = r#"
cadena texto = "archivo.txt"
bool es_txt = texto.termina_con(".txt")
bool es_pdf = texto.termina_con(".pdf")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_contar_ocurrencias() {
        let codigo = r#"
cadena texto = "ana, banana, manzana"
entero cuenta_ana = texto.contar_ocurrencias("ana")
entero cuenta_na = texto.contar_ocurrencias("na")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_transformaciones_caso() {
        let codigo = r#"
cadena texto = "Hola Mundo"
cadena mayusculas = texto.a_mayusculas()
cadena minusculas = texto.a_minusculas()
cadena capitalizado = texto.capitalizar()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_recortar() {
        let codigo = r#"
cadena texto = "  texto con espacios  "
cadena limpio = texto.recortar()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_repetir() {
        let codigo = r#"
cadena texto = "ab"
cadena repetido = texto.repetir(3)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_invertir() {
        let codigo = r#"
cadena texto = "Hola"
cadena invertido = texto.invertir()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_reemplazar() {
        let codigo = r#"
cadena texto = "Hola mundo, mundo cruel"
cadena reemplazado = texto.reemplazar("mundo", "universo")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_subcadena() {
        let codigo = r#"
cadena texto = "Hola mundo"
cadena sub1 = texto.subcadena(0, 4)
cadena sub2 = texto.subcadena(5)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_dividir() {
        let codigo = r#"
cadena texto = "uno,dos,tres,cuatro"
lista partes = texto.dividir(",")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_partir_lineas() {
        let codigo = r#"
cadena texto_multilinea = "linea1\nlinea2\nlinea3"
lista<cadena> lineas = texto_multilinea.partir_lineas()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_comparar() {
        let codigo = r#"
cadena texto1 = "abc"
cadena texto2 = "def"
cadena texto3 = "abc"
entero comp1 = texto1.comparar(texto2)
entero comp2 = texto1.comparar(texto3)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_igual_sin_caso() {
        let codigo = r#"
cadena texto1 = "HOLA"
cadena texto2 = "hola"
cadena texto3 = "mundo"
bool iguales = texto1.igual_sin_caso(texto2)
bool diferentes = texto1.igual_sin_caso(texto3)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_codificar_base64() {
        let codigo = r#"
cadena texto = "Hola mundo"
cadena codificado = texto.codificar_base64()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_decodificar_base64() {
        let codigo = r#"
cadena texto_base64 = "SG9sYSBtdW5kbw=="
cadena decodificado = texto_base64.decodificar_base64()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_codificar_uri() {
        let codigo = r#"
cadena texto = "Hola mundo!"
cadena codificado = texto.codificar_uri()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_decodificar_uri() {
        let codigo = r#"
cadena texto_uri = "Hola%20mundo%21"
cadena decodificado = texto_uri.decodificar_uri()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_acceso_por_indice() {
        let codigo = r#"
cadena texto = "Hola"
cadena primer_char = texto[0]
cadena segundo_char = texto[1]
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_lista_unir() {
        let codigo = r#"
lista<cadena> palabras = ["Hola", "mundo", "cruel"]
cadena unido = palabras.unir(" ")
cadena unido_comas = palabras.unir(", ")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_lista_unir_lineas() {
        let codigo = r#"
lista<cadena> lineas = ["Primera linea", "Segunda linea", "Tercera linea"]
cadena texto_multilinea = lineas.unir_lineas()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_error_repetir_negativo() {
        let codigo = r#"
cadena texto = "test"
cadena resultado = texto.repetir(-1)
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_error_indice_fuera_rango() {
        let codigo = r#"
cadena texto = "abc"
cadena char = texto[10]
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_error_buscar_sin_parametro() {
        let codigo = r#"
cadena texto = "test"
entero pos = texto.buscar()
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_error_dividir_delimitador_vacio() {
        let codigo = r#"
cadena texto = "abc"
lista<cadena> partes = texto.dividir("")
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_error_contar_patron_vacio() {
        let codigo = r#"
cadena texto = "abc"
entero count = texto.contar_ocurrencias("")
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_cadena_vacia_longitud() {
        let codigo = r#"
cadena texto_vacio = ""
entero longitud = texto_vacio.longitud()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_subcadena_indices_limite() {
        let codigo = r#"
cadena texto = "abc"
cadena todo = texto.subcadena(0)
cadena str_vacio = texto.subcadena(10)
cadena parte = texto.subcadena(1, 2)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_caracteres_especiales() {
        let codigo = r#"
cadena texto = "ñáéíóúü"
entero longitud = texto.longitud()
cadena mayusculas = texto.a_mayusculas()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_operaciones_encadenadas() {
        let codigo = r#"
cadena texto = "  HOLA MUNDO  "
cadena procesado = texto.recortar().a_minusculas().capitalizar()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_cadenas_en_expresiones() {
        let codigo = r#"
cadena texto1 = "Hola"
cadena texto2 = "mundo"
cadena concatenado = texto1 + " " + texto2
entero longitud_total = concatenado.longitud()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }
}
