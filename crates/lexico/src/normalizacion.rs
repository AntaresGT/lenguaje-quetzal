//! Normalización de identificadores para aceptar variantes con y sin tilde.
//!
//! La sintaxis de Quetzal soporta Unicode (`año`, `función`), pero las
//! palabras reservadas aceptan ambas variantes: `numero`/`número`,
//! `asincrono`/`asincróno`. Solo se normalizan las vocales acentuadas;
//! la `ñ` se conserva porque distingue identificadores (`año` ≠ `ano`).

/// Quita tildes de las vocales para comparar palabras reservadas.
pub fn quitar_tildes(texto: &str) -> String {
    texto
        .chars()
        .map(|caracter| match caracter {
            'á' | 'à' => 'a',
            'é' | 'è' => 'e',
            'í' | 'ì' => 'i',
            'ó' | 'ò' => 'o',
            'ú' | 'ù' => 'u',
            'Á' | 'À' => 'A',
            'É' | 'È' => 'E',
            'Í' | 'Ì' => 'I',
            'Ó' | 'Ò' => 'O',
            'Ú' | 'Ù' => 'U',
            otro => otro,
        })
        .collect()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn quitar_tildes_deberia_normalizar_vocales() {
        assert_eq!(quitar_tildes("número"), "numero");
        assert_eq!(quitar_tildes("asincróno"), "asincrono");
    }

    #[test]
    fn quitar_tildes_deberia_conservar_la_enie() {
        assert_eq!(quitar_tildes("año"), "año");
    }
}
