//! Resolución y presentación de símbolos para consultas del editor.

use nucleo::Ubicacion;
use semantica::indice::{ClaseSimbolo, ResultadoAnalisis, SimboloAnalizado};

pub fn simbolo_en_offset<'a>(
    analisis: &'a ResultadoAnalisis,
    contenido: &str,
    offset: usize,
) -> Option<(&'a SimboloAnalizado, Ubicacion)> {
    let (ubicacion, nombre) = identificador_en_offset(contenido, offset)?;
    let referencia = analisis
        .referencias
        .iter()
        .filter(|referencia| contiene_offset(referencia.ubicacion, offset))
        .filter(|referencia| {
            analisis
                .simbolo(referencia.simbolo)
                .is_some_and(|simbolo| simbolo.nombre == nombre)
        })
        .min_by_key(|referencia| referencia.ubicacion.longitud())
        .and_then(|referencia| analisis.simbolo(referencia.simbolo));
    if let Some(simbolo) = referencia {
        return Some((simbolo, ubicacion));
    }

    analisis
        .simbolos
        .iter()
        .filter(|simbolo| simbolo.nombre == nombre)
        .filter(|simbolo| {
            rango_nombre(simbolo, contenido).is_some_and(|rango| contiene_offset(rango, offset))
        })
        .min_by_key(|simbolo| simbolo.ubicacion.longitud())
        .map(|simbolo| (simbolo, ubicacion))
}

pub fn rango_nombre(simbolo: &SimboloAnalizado, contenido: &str) -> Option<Ubicacion> {
    if simbolo.clase == ClaseSimbolo::Nativo {
        return None;
    }
    let inicio = simbolo.ubicacion.inicio;
    let fin = if matches!(
        simbolo.clase,
        ClaseSimbolo::Funcion | ClaseSimbolo::Metodo | ClaseSimbolo::Constructor
    ) || simbolo.ubicacion.longitud() == 0
    {
        contenido
            .get(inicio..)?
            .find('\n')
            .map_or(contenido.len(), |fin| inicio + fin)
    } else {
        simbolo.ubicacion.fin.min(contenido.len())
    };
    let fragmento = contenido.get(inicio..fin)?;
    fragmento
        .match_indices(&simbolo.nombre)
        .find(|(relativo, _)| {
            let anterior = fragmento[..*relativo].chars().next_back();
            let siguiente = fragmento[*relativo + simbolo.nombre.len()..].chars().next();
            !anterior.is_some_and(es_identificador) && !siguiente.is_some_and(es_identificador)
        })
        .map(|(relativo, _)| Ubicacion {
            inicio: inicio + relativo,
            fin: inicio + relativo + simbolo.nombre.len(),
        })
}

pub fn describir_simbolo(simbolo: &SimboloAnalizado) -> String {
    if let Some(firma) = &simbolo.firma {
        let parametros = firma
            .parametros
            .iter()
            .map(|parametro| {
                let mutable = if parametro.mutable { "var " } else { "" };
                format!("{} {mutable}{}", parametro.tipo.nombre(), parametro.nombre)
            })
            .collect::<Vec<_>>()
            .join(", ");
        let asincrona = if firma.asincrona { "asincrono " } else { "" };
        if simbolo.clase == ClaseSimbolo::Constructor {
            return format!("{}({parametros})", simbolo.nombre);
        }
        return format!(
            "{asincrona}{} {}({parametros})",
            firma.retorno.nombre(),
            simbolo.nombre
        );
    }

    let mutable = if simbolo.mutable { "var " } else { "" };
    match simbolo.clase {
        ClaseSimbolo::Variable | ClaseSimbolo::Parametro => {
            format!("{} {mutable}{}", simbolo.tipo.nombre(), simbolo.nombre)
        }
        ClaseSimbolo::Atributo => {
            let visibilidad = match simbolo.visibilidad {
                Some(ast::Visibilidad::Privado) => "privado ",
                _ => "",
            };
            let libre = if simbolo.libre { "libre " } else { "" };
            format!(
                "{visibilidad}{libre}{} {mutable}{}",
                simbolo.tipo.nombre(),
                simbolo.nombre
            )
        }
        ClaseSimbolo::Objeto => format!("objeto {}", simbolo.nombre),
        ClaseSimbolo::Prototipo => format!("prototipo {}", simbolo.nombre),
        ClaseSimbolo::Importacion => format!("importar {}", simbolo.nombre),
        ClaseSimbolo::Nativo => format!("nativo {}", simbolo.nombre),
        ClaseSimbolo::Funcion | ClaseSimbolo::Metodo | ClaseSimbolo::Constructor => {
            format!("{} {}", simbolo.tipo.nombre(), simbolo.nombre)
        }
    }
}

fn identificador_en_offset(contenido: &str, offset: usize) -> Option<(Ubicacion, &str)> {
    let mut inicio = None;
    for (indice, caracter) in contenido.char_indices() {
        if es_identificador(caracter) {
            inicio.get_or_insert(indice);
        } else if let Some(comienzo) = inicio.take()
            && comienzo <= offset
            && offset <= indice
        {
            return Some((
                Ubicacion::nueva(comienzo, indice),
                &contenido[comienzo..indice],
            ));
        }
    }
    let comienzo = inicio?;
    (comienzo <= offset && offset <= contenido.len()).then_some((
        Ubicacion::nueva(comienzo, contenido.len()),
        &contenido[comienzo..],
    ))
}

fn contiene_offset(ubicacion: Ubicacion, offset: usize) -> bool {
    ubicacion.inicio <= offset && offset <= ubicacion.fin
}

fn es_identificador(caracter: char) -> bool {
    caracter == '_' || caracter.is_alphanumeric()
}
