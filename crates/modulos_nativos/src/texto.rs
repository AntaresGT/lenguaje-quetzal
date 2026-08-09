//! Métodos nativos del tipo `texto` según `ejemplos/metodos_texto.qz`.
//!
//! El receptor llega como primer argumento; los índices cuentan caracteres
//! Unicode, no bytes.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use maquina_virtual::{Fallo, RegistroNativos, Valor};
use percent_encoding::{NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};

use crate::util::{arg_entero, arg_texto, error, exigir_aridad, indice_normalizado};

pub fn registrar(registro: &mut RegistroNativos) {
    metodo(registro, "texto.longitud", 0, |texto, _| {
        Ok(Valor::Entero(texto.chars().count() as i64))
    });
    metodo(registro, "texto.mayusculas", 0, |texto, _| {
        Ok(Valor::texto(texto.to_uppercase()))
    });
    metodo(registro, "texto.minusculas", 0, |texto, _| {
        Ok(Valor::texto(texto.to_lowercase()))
    });
    metodo(registro, "texto.capitalizar", 0, |texto, _| {
        Ok(Valor::texto(capitalizar(texto)))
    });
    metodo(registro, "texto.titulo", 0, |texto, _| {
        let titulo = texto
            .split(' ')
            .map(capitalizar)
            .collect::<Vec<_>>()
            .join(" ");
        Ok(Valor::texto(titulo))
    });
    metodo(registro, "texto.recortar", 0, |texto, _| {
        Ok(Valor::texto(texto.trim()))
    });
    metodo(registro, "texto.recortar_inicio", 0, |texto, _| {
        Ok(Valor::texto(texto.trim_start()))
    });
    metodo(registro, "texto.recortar_final", 0, |texto, _| {
        Ok(Valor::texto(texto.trim_end()))
    });
    metodo(registro, "texto.contiene", 1, |texto, argumentos| {
        let buscado = arg_texto("texto.contiene", argumentos, 1)?;
        Ok(Valor::Log(texto.contains(buscado)))
    });
    metodo(registro, "texto.empieza_con", 1, |texto, argumentos| {
        let buscado = arg_texto("texto.empieza_con", argumentos, 1)?;
        Ok(Valor::Log(texto.starts_with(buscado)))
    });
    metodo(registro, "texto.termina_con", 1, |texto, argumentos| {
        let buscado = arg_texto("texto.termina_con", argumentos, 1)?;
        Ok(Valor::Log(texto.ends_with(buscado)))
    });
    metodo(registro, "texto.encontrar", 1, |texto, argumentos| {
        let buscado = arg_texto("texto.encontrar", argumentos, 1)?;
        Ok(Valor::Entero(posicion_en_caracteres(
            texto,
            texto.find(buscado),
        )))
    });
    metodo(registro, "texto.buscar_ultimo", 1, |texto, argumentos| {
        let buscado = arg_texto("texto.buscar_ultimo", argumentos, 1)?;
        Ok(Valor::Entero(posicion_en_caracteres(
            texto,
            texto.rfind(buscado),
        )))
    });
    metodo(registro, "texto.reemplazar", 2, |texto, argumentos| {
        let buscado = arg_texto("texto.reemplazar", argumentos, 1)?;
        let reemplazo = arg_texto("texto.reemplazar", argumentos, 2)?;
        Ok(Valor::texto(texto.replace(buscado, reemplazo)))
    });
    metodo(
        registro,
        "texto.reemplazar_primero",
        2,
        |texto, argumentos| {
            let buscado = arg_texto("texto.reemplazar_primero", argumentos, 1)?;
            let reemplazo = arg_texto("texto.reemplazar_primero", argumentos, 2)?;
            Ok(Valor::texto(texto.replacen(buscado, reemplazo, 1)))
        },
    );
    metodo(registro, "texto.dividir", 1, |texto, argumentos| {
        let separador = arg_texto("texto.dividir", argumentos, 1)?;
        let partes = texto.split(separador).map(Valor::texto).collect();
        Ok(Valor::lista(partes))
    });
    metodo(registro, "texto.partir_lineas", 0, |texto, _| {
        Ok(Valor::lista(texto.lines().map(Valor::texto).collect()))
    });
    metodo(registro, "texto.repetir", 1, |texto, argumentos| {
        let veces = arg_entero("texto.repetir", argumentos, 1)?;
        let veces = usize::try_from(veces)
            .map_err(|_| error("E0406", "'repetir' necesita una cantidad >= 0"))?;
        Ok(Valor::texto(texto.repeat(veces)))
    });
    metodo(registro, "texto.subtexto", 2, |texto, argumentos| {
        let inicio = arg_entero("texto.subtexto", argumentos, 1)?;
        let fin = arg_entero("texto.subtexto", argumentos, 2)?;
        let caracteres: Vec<char> = texto.chars().collect();
        let inicio = usize::try_from(inicio.max(0))
            .unwrap_or(0)
            .min(caracteres.len());
        let fin = usize::try_from(fin.max(0))
            .unwrap_or(0)
            .min(caracteres.len());
        let recorte: String = caracteres[inicio..fin.max(inicio)].iter().collect();
        Ok(Valor::texto(recorte))
    });
    metodo(registro, "texto.izquierda", 1, |texto, argumentos| {
        let cantidad = arg_entero("texto.izquierda", argumentos, 1)?.max(0) as usize;
        Ok(Valor::texto(
            texto.chars().take(cantidad).collect::<String>(),
        ))
    });
    metodo(registro, "texto.derecha", 1, |texto, argumentos| {
        let cantidad = arg_entero("texto.derecha", argumentos, 1)?.max(0) as usize;
        let total = texto.chars().count();
        let saltar = total.saturating_sub(cantidad);
        Ok(Valor::texto(texto.chars().skip(saltar).collect::<String>()))
    });
    metodo(registro, "texto.es_numero", 0, |texto, _| {
        Ok(Valor::Log(texto.trim().parse::<f64>().is_ok()))
    });
    metodo(registro, "texto.es_entero", 0, |texto, _| {
        Ok(Valor::Log(texto.trim().parse::<i64>().is_ok()))
    });
    metodo(registro, "texto.es_alfanumerico", 0, |texto, _| {
        Ok(Valor::Log(
            !texto.is_empty() && texto.chars().all(char::is_alphanumeric),
        ))
    });
    metodo(registro, "texto.a_base64", 0, |texto, _| {
        Ok(Valor::texto(BASE64.encode(texto.as_bytes())))
    });
    metodo(registro, "texto.decodificar_base64", 0, |texto, _| {
        let bytes = BASE64
            .decode(texto.trim().replace(' ', ""))
            .map_err(|_| error("E0406", "el texto no es base64 válido"))?;
        String::from_utf8(bytes)
            .map(Valor::texto)
            .map_err(|_| error("E0406", "el contenido base64 no es texto UTF-8 válido"))
    });
    metodo(registro, "texto.a_enlace", 0, |texto, _| {
        Ok(Valor::texto(
            utf8_percent_encode(texto, NON_ALPHANUMERIC).to_string(),
        ))
    });
    metodo(registro, "texto.decodificar_enlace", 0, |texto, _| {
        percent_decode_str(texto)
            .decode_utf8()
            .map(|decodificado| Valor::texto(decodificado.as_ref()))
            .map_err(|_| error("E0406", "el enlace no contiene texto UTF-8 válido"))
    });
    metodo(registro, "texto.igual_sin_caso", 1, |texto, argumentos| {
        let otro = arg_texto("texto.igual_sin_caso", argumentos, 1)?;
        Ok(Valor::Log(texto.to_lowercase() == otro.to_lowercase()))
    });
    metodo(registro, "texto.contar", 1, |texto, argumentos| {
        let buscado = arg_texto("texto.contar", argumentos, 1)?;
        if buscado.is_empty() {
            return Ok(Valor::Entero(0));
        }
        Ok(Valor::Entero(texto.matches(buscado).count() as i64))
    });
    metodo(registro, "texto.invertir", 0, |texto, _| {
        Ok(Valor::texto(texto.chars().rev().collect::<String>()))
    });
    metodo(registro, "texto.caracter_en", 1, |texto, argumentos| {
        let indice = arg_entero("texto.caracter_en", argumentos, 1)?;
        let caracteres: Vec<char> = texto.chars().collect();
        let real = indice_normalizado(indice, caracteres.len())
            .ok_or_else(|| error("E0403", format!("el índice {indice} está fuera de rango")))?;
        Ok(Valor::texto(caracteres[real].to_string()))
    });
}

/// Registra un método de texto verificando receptor y aridad.
fn metodo(
    registro: &mut RegistroNativos,
    nombre: &'static str,
    argumentos_extra: usize,
    implementacion: impl Fn(&str, &[Valor]) -> Result<Valor, Fallo> + 'static,
) {
    registro.registrar_funcion(
        nombre,
        Box::new(move |argumentos| {
            exigir_aridad(nombre, argumentos, argumentos_extra + 1)?;
            let receptor = arg_texto(nombre, argumentos, 0)?;
            implementacion(receptor, argumentos)
        }),
    );
}

fn capitalizar(texto: &str) -> String {
    let mut caracteres = texto.chars();
    match caracteres.next() {
        Some(primero) => {
            primero.to_uppercase().collect::<String>() + &caracteres.as_str().to_lowercase()
        }
        None => String::new(),
    }
}

/// Convierte una posición en bytes a posición en caracteres (-1 si no existe).
fn posicion_en_caracteres(texto: &str, posicion_bytes: Option<usize>) -> i64 {
    match posicion_bytes {
        Some(bytes) => texto[..bytes].chars().count() as i64,
        None => -1,
    }
}
