use crate::interprete;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_longitud_cadena() {
        let codigo = r#"
texto mi_texto = "Hola mundo"
entero longitud = mi_texto.longitud()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_esta_vacia() {
        let codigo = r#"
texto texto_vacio = ""
texto texto_lleno = "contenido"
log es_vacio = texto_vacio.esta_vacia()
log es_lleno = texto_lleno.esta_vacia()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_buscar_patron() {
        let codigo = r#"
texto mi_texto = "Hola mundo cruel"
entero posicion = mi_texto.buscar("mundo")
entero no_encontrado = mi_texto.buscar("xyz")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_contiene_patron() {
        let codigo = r#"
texto mi_texto = "Hola mundo"
log contiene_hola = mi_texto.contiene("Hola")
log contiene_xyz = mi_texto.contiene("xyz")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_empieza_con() {
        let codigo = r#"
texto mi_texto = "Hola mundo"
log empieza_hola = mi_texto.empieza_con("Hola")
log empieza_mundo = mi_texto.empieza_con("mundo")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_termina_con() {
        let codigo = r#"
texto mi_texto = "archivo.txt"
log es_txt = mi_texto.termina_con(".txt")
log es_pdf = mi_texto.termina_con(".pdf")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_contar_ocurrencias() {
        let codigo = r#"
texto mi_texto = "ana, banana, manzana"
entero cuenta_ana = mi_texto.contar_ocurrencias("ana")
entero cuenta_na = mi_texto.contar_ocurrencias("na")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_transformaciones_caso() {
        let codigo = r#"
texto mi_texto = "Hola Mundo"
texto mayusculas = mi_texto.a_mayusculas()
texto minusculas = mi_texto.a_minusculas()
texto capitalizado = mi_texto.capitalizar()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_recortar() {
        let codigo = r#"
texto mi_texto = "  texto con espacios  "
texto limpio = mi_texto.recortar()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_repetir() {
        let codigo = r#"
texto mi_texto = "ab"
texto repetido = mi_texto.repetir(3)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_invertir() {
        let codigo = r#"
texto mi_texto = "Hola"
texto invertido = mi_texto.invertir()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_reemplazar() {
        let codigo = r#"
texto mi_texto = "Hola mundo, mundo cruel"
texto reemplazado = mi_texto.reemplazar("mundo", "universo")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_subcadena() {
        let codigo = r#"
texto mi_texto = "Hola mundo"
texto sub1 = mi_texto.subcadena(0, 4)
texto sub2 = mi_texto.subcadena(5)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_dividir() {
        let codigo = r#"
texto mi_texto = "uno,dos,tres,cuatro"
lista partes = mi_texto.dividir(",")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_partir_lineas() {
        let codigo = r#"
texto texto_multilinea = "linea1\nlinea2\nlinea3"
lista<texto> lineas = texto_multilinea.partir_lineas()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_comparar() {
        let codigo = r#"
texto texto1 = "abc"
texto texto2 = "def"
texto texto3 = "abc"
entero comp1 = texto1.comparar(texto2)
entero comp2 = texto1.comparar(texto3)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_igual_sin_caso() {
        let codigo = r#"
texto texto1 = "HOLA"
texto texto2 = "hola"
texto texto3 = "mundo"
log iguales = texto1.igual_sin_caso(texto2)
log diferentes = texto1.igual_sin_caso(texto3)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_codificar_base64() {
        let codigo = r#"
texto mi_texto = "Hola mundo"
texto codificado = mi_texto.codificar_base64()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_decodificar_base64() {
        let codigo = r#"
texto texto_base64 = "SG9sYSBtdW5kbw=="
texto decodificado = texto_base64.decodificar_base64()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_codificar_uri() {
        let codigo = r#"
texto mi_texto = "Hola mundo!"
texto codificado = mi_texto.codificar_uri()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_decodificar_uri() {
        let codigo = r#"
texto texto_uri = "Hola%20mundo%21"
texto decodificado = texto_uri.decodificar_uri()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_acceso_por_indice() {
        let codigo = r#"
texto mi_texto = "Hola"
texto primer_char = mi_texto[0]
texto segundo_char = mi_texto[1]
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_lista_unir() {
        let codigo = r#"
lista<texto> palabras = ["Hola", "mundo", "cruel"]
texto unido = palabras.unir(" ")
texto unido_comas = palabras.unir(", ")
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_lista_unir_lineas() {
        let codigo = r#"
lista<texto> lineas = ["Primera linea", "Segunda linea", "Tercera linea"]
texto texto_multilinea = lineas.unir_lineas()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_error_repetir_negativo() {
        let codigo = r#"
texto mi_texto = "test"
texto resultado = mi_texto.repetir(-1)
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_error_indice_fuera_rango() {
        let codigo = r#"
texto mi_texto = "abc"
texto char = texto[10]
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_error_buscar_sin_parametro() {
        let codigo = r#"
texto mi_texto = "test"
entero pos = mi_texto.buscar()
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_error_dividir_delimitador_vacio() {
        let codigo = r#"
texto mi_texto = "abc"
lista<texto> partes = mi_texto.dividir("")
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_error_contar_patron_vacio() {
        let codigo = r#"
texto mi_texto = "abc"
entero count = mi_texto.contar_ocurrencias("")
        "#;
        
        assert!(interprete::interpretar(codigo).is_err());
    }

    #[test]
    fn test_cadena_vacia_longitud() {
        let codigo = r#"
texto texto_vacio = ""
entero longitud = texto_vacio.longitud()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_subcadena_indices_limite() {
        let codigo = r#"
texto mi_texto = "abc"
texto todo = mi_texto.subcadena(0)
texto str_vacio = mi_texto.subcadena(10)
texto parte = mi_texto.subcadena(1, 2)
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_caracteres_especiales() {
        let codigo = r#"
texto mi_texto = "ñáéíóúü"
entero longitud = mi_texto.longitud()
texto mayusculas = mi_texto.a_mayusculas()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_operaciones_encadenadas() {
        let codigo = r#"
texto mi_texto = "  HOLA MUNDO  "
texto procesado = mi_texto.recortar().a_minusculas().capitalizar()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }

    #[test]
    fn test_cadenas_en_expresiones() {
        let codigo = r#"
texto texto1 = "Hola"
texto texto2 = "mundo"
texto concatenado = texto1 + " " + texto2
entero longitud_total = concatenado.longitud()
        "#;
        
        assert!(interprete::interpretar(codigo).is_ok());
    }
}
