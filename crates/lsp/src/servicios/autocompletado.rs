//! Autocompletado de palabras clave y símbolos conocidos.

use std::collections::BTreeMap;

use lsp_types::{CompletionItem, CompletionItemKind};
use semantica::indice::{ClaseSimbolo, ResultadoAnalisis, SimboloAnalizado};

use super::simbolos::describir_simbolo;

const PALABRAS_CLAVE: &[&str] = &[
    "asincrono",
    "cada",
    "capturar",
    "como",
    "continuar",
    "desde",
    "en",
    "entero",
    "esperar",
    "esto",
    "excepcion",
    "exportar",
    "falso",
    "finalmente",
    "funcion",
    "hacer",
    "hereda",
    "implementa",
    "importar",
    "intentar",
    "jsn",
    "lanzar",
    "libre",
    "lista",
    "log",
    "mientras",
    "nuevo",
    "nulo",
    "número",
    "objeto",
    "opcional",
    "padre",
    "para",
    "privado",
    "prototipo",
    "publico",
    "retornar",
    "romper",
    "si",
    "sino",
    "texto",
    "vacio",
    "var",
    "verdadero",
];

pub fn completar(
    analisis: &ResultadoAnalisis,
    contenido: &str,
    offset: usize,
) -> Vec<CompletionItem> {
    let prefijo = prefijo_en_offset(contenido, offset);
    let mut elementos = BTreeMap::new();
    for palabra in PALABRAS_CLAVE {
        if palabra.starts_with(prefijo) {
            elementos.insert(
                (*palabra).to_string(),
                CompletionItem {
                    label: (*palabra).to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    ..CompletionItem::default()
                },
            );
        }
    }
    for simbolo in analisis.simbolos.iter().filter(|simbolo| {
        !matches!(
            simbolo.clase,
            ClaseSimbolo::Atributo | ClaseSimbolo::Metodo | ClaseSimbolo::Constructor
        )
    }) {
        if simbolo.nombre.starts_with(prefijo) {
            elementos
                .entry(simbolo.nombre.clone())
                .or_insert_with(|| completar_simbolo(simbolo));
        }
    }
    elementos.into_values().collect()
}

fn completar_simbolo(simbolo: &SimboloAnalizado) -> CompletionItem {
    CompletionItem {
        label: simbolo.nombre.clone(),
        kind: Some(match simbolo.clase {
            ClaseSimbolo::Variable | ClaseSimbolo::Parametro => CompletionItemKind::VARIABLE,
            ClaseSimbolo::Funcion => CompletionItemKind::FUNCTION,
            ClaseSimbolo::Objeto => CompletionItemKind::CLASS,
            ClaseSimbolo::Prototipo => CompletionItemKind::INTERFACE,
            ClaseSimbolo::Importacion => CompletionItemKind::MODULE,
            ClaseSimbolo::Nativo => CompletionItemKind::VARIABLE,
            ClaseSimbolo::Atributo => CompletionItemKind::FIELD,
            ClaseSimbolo::Metodo | ClaseSimbolo::Constructor => CompletionItemKind::METHOD,
        }),
        detail: Some(describir_simbolo(simbolo)),
        ..CompletionItem::default()
    }
}

fn prefijo_en_offset(contenido: &str, offset: usize) -> &str {
    let mut offset = offset.min(contenido.len());
    while !contenido.is_char_boundary(offset) {
        offset -= 1;
    }
    let inicio = contenido[..offset]
        .char_indices()
        .rev()
        .find_map(|(indice, caracter)| {
            (!(caracter == '_' || caracter.is_alphanumeric()))
                .then_some(indice + caracter.len_utf8())
        })
        .unwrap_or(0);
    &contenido[inicio..offset]
}

#[cfg(test)]
mod pruebas {
    use super::prefijo_en_offset;

    #[test]
    fn prefijo_deberia_aceptar_identificadores_unicode() {
        assert_eq!(prefijo_en_offset("consola.imprimir(añ", 20), "añ");
    }

    #[test]
    fn prefijo_deberia_detenerse_en_un_delimitador() {
        assert_eq!(prefijo_en_offset("entero total", 12), "total");
    }

    #[test]
    fn prefijo_deberia_ajustar_un_offset_dentro_de_un_caracter() {
        assert_eq!(prefijo_en_offset("añ", 2), "a");
    }
}
