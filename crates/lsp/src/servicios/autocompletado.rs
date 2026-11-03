//! Autocompletado de palabras clave, símbolos y miembros conocidos.

use std::collections::{BTreeMap, BTreeSet};

use lsp_types::{CompletionItem, CompletionItemKind};
use semantica::TipoSemantico;
use semantica::indice::{ClaseSimbolo, ResultadoAnalisis, SimboloAnalizado};

use super::nativos::{self, CatalogoNativo};
use super::simbolos::{describir_simbolo, simbolo_en_offset};

const PALABRAS_CLAVE: &[&str] = &[
    "asincrono",
    "asincróno",
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
    "excepción",
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
    "numero",
    "número",
    "objeto",
    "o",
    "opcional",
    "padre",
    "para",
    "privado",
    "prototipo",
    "publico",
    "público",
    "retornar",
    "romper",
    "si",
    "sino",
    "texto",
    "vacio",
    "vacío",
    "var",
    "verdadero",
    "y",
];

pub fn completar(
    analisis: &ResultadoAnalisis,
    contenido: &str,
    offset: usize,
) -> Vec<CompletionItem> {
    let prefijo = prefijo_en_offset(contenido, offset);
    if let Some(receptor) = receptor_en_offset(contenido, offset) {
        return completar_miembros(analisis, contenido, receptor, prefijo);
    }

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

struct Receptor<'a> {
    nombre: &'a str,
    inicio: usize,
}

fn completar_miembros(
    analisis: &ResultadoAnalisis,
    contenido: &str,
    receptor: Receptor<'_>,
    prefijo: &str,
) -> Vec<CompletionItem> {
    let simbolo = simbolo_en_offset(analisis, contenido, receptor.inicio)
        .map(|(simbolo, _)| simbolo)
        .or_else(|| buscar_simbolo(analisis, receptor.nombre, receptor.inicio));

    if let Some(catalogo) = simbolo
        .and_then(|simbolo| nativos::por_tipo(&simbolo.tipo))
        .or_else(|| nativos::por_nombre(receptor.nombre))
    {
        return completar_nativos(catalogo, prefijo);
    }

    let Some(simbolo) = simbolo else {
        return Vec::new();
    };
    let (objeto, solo_libres) = if simbolo.clase == ClaseSimbolo::Objeto {
        (&simbolo.nombre, true)
    } else if let TipoSemantico::Objeto(nombre) = &simbolo.tipo {
        (nombre, false)
    } else {
        return Vec::new();
    };

    let mut elementos = BTreeMap::new();
    let mut visitados = BTreeSet::new();
    agregar_miembros_objeto(
        analisis,
        objeto,
        solo_libres,
        prefijo,
        &mut visitados,
        &mut elementos,
    );
    elementos.into_values().collect()
}

fn buscar_simbolo<'a>(
    analisis: &'a ResultadoAnalisis,
    nombre: &str,
    offset: usize,
) -> Option<&'a SimboloAnalizado> {
    analisis
        .simbolos
        .iter()
        .filter(|simbolo| simbolo.nombre == nombre)
        .min_by_key(|simbolo| simbolo.ubicacion.inicio.abs_diff(offset))
}

fn agregar_miembros_objeto(
    analisis: &ResultadoAnalisis,
    nombre: &str,
    solo_libres: bool,
    prefijo: &str,
    visitados: &mut BTreeSet<String>,
    elementos: &mut BTreeMap<String, CompletionItem>,
) {
    if !visitados.insert(nombre.to_string()) {
        return;
    }
    let Some(objeto) = analisis
        .objetos
        .iter()
        .find(|objeto| objeto.nombre == nombre)
    else {
        return;
    };
    for miembro in objeto
        .miembros
        .iter()
        .filter_map(|id| analisis.simbolo(*id))
        .filter(|simbolo| {
            simbolo.clase != ClaseSimbolo::Constructor
                && simbolo.visibilidad != Some(ast::Visibilidad::Privado)
                && (!solo_libres || simbolo.libre)
                && (solo_libres || !simbolo.libre)
                && simbolo.nombre.starts_with(prefijo)
        })
    {
        elementos
            .entry(miembro.nombre.clone())
            .or_insert_with(|| completar_simbolo(miembro));
    }
    for padre in objeto.padres.iter().chain(&objeto.extiende_como) {
        agregar_miembros_objeto(analisis, padre, solo_libres, prefijo, visitados, elementos);
    }
}

fn completar_nativos(catalogo: CatalogoNativo, prefijo: &str) -> Vec<CompletionItem> {
    let mut elementos = BTreeMap::new();
    for metodo in catalogo
        .metodos
        .iter()
        .filter(|nombre| nombre.starts_with(prefijo))
    {
        elementos.insert(
            (*metodo).to_string(),
            CompletionItem {
                label: (*metodo).to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some(format!("Método nativo de {}", catalogo.nombre)),
                ..CompletionItem::default()
            },
        );
    }
    for propiedad in catalogo
        .propiedades
        .iter()
        .filter(|nombre| nombre.starts_with(prefijo))
    {
        elementos.insert(
            (*propiedad).to_string(),
            CompletionItem {
                label: (*propiedad).to_string(),
                kind: Some(CompletionItemKind::CONSTANT),
                detail: Some(format!("Constante nativa de {}", catalogo.nombre)),
                ..CompletionItem::default()
            },
        );
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

fn receptor_en_offset(contenido: &str, offset: usize) -> Option<Receptor<'_>> {
    let offset = ajustar_offset(contenido, offset);
    let inicio_prefijo = inicio_prefijo(contenido, offset);
    let antes = contenido[..inicio_prefijo].trim_end();
    let antes = antes.strip_suffix('.')?.trim_end();
    let inicio = antes
        .char_indices()
        .rev()
        .find_map(|(indice, caracter)| {
            (!(caracter == '_' || caracter.is_alphanumeric()))
                .then_some(indice + caracter.len_utf8())
        })
        .unwrap_or(0);
    (inicio < antes.len()).then_some(Receptor {
        nombre: &antes[inicio..],
        inicio,
    })
}

fn prefijo_en_offset(contenido: &str, offset: usize) -> &str {
    let offset = ajustar_offset(contenido, offset);
    let inicio = inicio_prefijo(contenido, offset);
    &contenido[inicio..offset]
}

fn inicio_prefijo(contenido: &str, offset: usize) -> usize {
    contenido[..offset]
        .char_indices()
        .rev()
        .find_map(|(indice, caracter)| {
            (!(caracter == '_' || caracter.is_alphanumeric()))
                .then_some(indice + caracter.len_utf8())
        })
        .unwrap_or(0)
}

fn ajustar_offset(contenido: &str, offset: usize) -> usize {
    let mut offset = offset.min(contenido.len());
    while !contenido.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
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
