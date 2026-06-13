//! Utilidades compartidas por los módulos nativos: extracción de argumentos
//! con errores claros.

use std::cell::RefCell;
use std::rc::Rc;

use indexmap::IndexMap;
use maquina_virtual::{Fallo, Valor};
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};

/// Excepción de runtime con código.
pub fn error(codigo: &str, mensaje: impl Into<String>) -> Fallo {
    Fallo::excepcion(codigo, mensaje, Vec::new(), None)
}

/// Error estándar de cantidad de argumentos.
pub fn error_aridad(funcion: &str, esperados: usize, recibidos: usize) -> Fallo {
    error(
        "E0210",
        format!("'{funcion}' espera {esperados} argumentos, pero recibió {recibidos}"),
    )
}

pub fn exigir_aridad(funcion: &str, argumentos: &[Valor], esperados: usize) -> Result<(), Fallo> {
    if argumentos.len() == esperados {
        Ok(())
    } else {
        Err(error_aridad(funcion, esperados, argumentos.len()))
    }
}

pub fn arg_texto<'a>(
    funcion: &str,
    argumentos: &'a [Valor],
    indice: usize,
) -> Result<&'a str, Fallo> {
    match argumentos.get(indice) {
        Some(Valor::Texto(texto)) => Ok(texto),
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba un texto en el argumento {}, pero recibió '{}'",
                indice + 1,
                otro.nombre_tipo()
            ),
        )),
        None => Err(error(
            "E0210",
            format!("'{funcion}' necesita al menos {} argumentos", indice + 1),
        )),
    }
}

pub fn arg_entero(funcion: &str, argumentos: &[Valor], indice: usize) -> Result<i64, Fallo> {
    match argumentos.get(indice) {
        Some(Valor::Entero(entero)) => Ok(*entero),
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba un entero en el argumento {}, pero recibió '{}'",
                indice + 1,
                otro.nombre_tipo()
            ),
        )),
        None => Err(error(
            "E0210",
            format!("'{funcion}' necesita al menos {} argumentos", indice + 1),
        )),
    }
}

/// Acepta `entero` o `número` y lo devuelve como decimal.
pub fn arg_decimal(funcion: &str, argumentos: &[Valor], indice: usize) -> Result<Decimal, Fallo> {
    match argumentos.get(indice) {
        Some(Valor::Entero(entero)) => Ok(Decimal::from(*entero)),
        Some(Valor::Numero(decimal)) => Ok(*decimal),
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba un número en el argumento {}, pero recibió '{}'",
                indice + 1,
                otro.nombre_tipo()
            ),
        )),
        None => Err(error(
            "E0210",
            format!("'{funcion}' necesita al menos {} argumentos", indice + 1),
        )),
    }
}

pub fn arg_lista(
    funcion: &str,
    argumentos: &[Valor],
    indice: usize,
) -> Result<Rc<RefCell<Vec<Valor>>>, Fallo> {
    match argumentos.get(indice) {
        Some(Valor::Lista(lista)) => Ok(Rc::clone(lista)),
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba una lista en el argumento {}, pero recibió '{}'",
                indice + 1,
                otro.nombre_tipo()
            ),
        )),
        None => Err(error(
            "E0210",
            format!("'{funcion}' necesita al menos {} argumentos", indice + 1),
        )),
    }
}

pub fn arg_jsn(
    funcion: &str,
    argumentos: &[Valor],
    indice: usize,
) -> Result<Rc<RefCell<IndexMap<String, Valor>>>, Fallo> {
    match argumentos.get(indice) {
        Some(Valor::Jsn(mapa)) => Ok(Rc::clone(mapa)),
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba un jsn en el argumento {}, pero recibió '{}'",
                indice + 1,
                otro.nombre_tipo()
            ),
        )),
        None => Err(error(
            "E0210",
            format!("'{funcion}' necesita al menos {} argumentos", indice + 1),
        )),
    }
}

/// Convierte un valor de Quetzal a byte (entero 0–255).
pub fn byte_de_valor(funcion: &str, valor: &Valor) -> Result<u8, Fallo> {
    match valor {
        Valor::Entero(entero) if (0..=255).contains(entero) => Ok(*entero as u8),
        Valor::Entero(entero) => Err(error(
            "E0406",
            format!("'{funcion}' esperaba un byte entre 0 y 255, pero recibió {entero}"),
        )),
        otro => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba un byte (entero 0-255), pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
    }
}

/// Extrae los bytes de un argumento binario: una instancia de `Bits` o una
/// `lista<entero>` con valores 0–255.
pub fn arg_bits(funcion: &str, argumentos: &[Valor], indice: usize) -> Result<Vec<u8>, Fallo> {
    match argumentos.get(indice) {
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == "Bits" => {
            match instancia.datos.borrow().get("datos") {
                Some(Valor::Lista(lista)) => lista
                    .borrow()
                    .iter()
                    .map(|valor| byte_de_valor(funcion, valor))
                    .collect(),
                _ => Err(error(
                    "E0406",
                    format!("'{funcion}' recibió un Bits sin datos internos"),
                )),
            }
        }
        Some(Valor::Lista(lista)) => lista
            .borrow()
            .iter()
            .map(|valor| byte_de_valor(funcion, valor))
            .collect(),
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba un Bits o una lista de bytes en el argumento {}, pero recibió '{}'",
                indice + 1,
                otro.nombre_tipo()
            ),
        )),
        None => Err(error(
            "E0210",
            format!("'{funcion}' necesita al menos {} argumentos", indice + 1),
        )),
    }
}

/// Extrae una ruta de un argumento: un `texto` o una instancia de `Ruta`.
pub fn arg_ruta(funcion: &str, argumentos: &[Valor], indice: usize) -> Result<String, Fallo> {
    match argumentos.get(indice) {
        Some(Valor::Texto(texto)) => Ok(texto.to_string()),
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == "Ruta" => {
            match instancia.datos.borrow().get("texto") {
                Some(Valor::Texto(texto)) => Ok(texto.to_string()),
                _ => Err(error(
                    "E0406",
                    format!("'{funcion}' recibió una Ruta sin texto interno"),
                )),
            }
        }
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba una ruta (texto o Ruta) en el argumento {}, pero recibió '{}'",
                indice + 1,
                otro.nombre_tipo()
            ),
        )),
        None => Err(error(
            "E0210",
            format!("'{funcion}' necesita al menos {} argumentos", indice + 1),
        )),
    }
}

/// Convierte un decimal a `f64` para funciones trigonométricas/logarítmicas.
pub fn decimal_a_f64(funcion: &str, decimal: Decimal) -> Result<f64, Fallo> {
    decimal.to_f64().ok_or_else(|| {
        error(
            "E0406",
            format!("'{funcion}' no pudo convertir {decimal} a número de punto flotante"),
        )
    })
}

/// Convierte el resultado `f64` de vuelta a decimal con error claro.
pub fn f64_a_decimal(funcion: &str, valor: f64) -> Result<Valor, Fallo> {
    if !valor.is_finite() {
        return Err(error(
            "E0406",
            format!("'{funcion}' produjo un resultado no representable ({valor})"),
        ));
    }
    Decimal::from_f64(valor).map(Valor::Numero).ok_or_else(|| {
        error(
            "E0406",
            format!("'{funcion}' produjo un resultado fuera del rango decimal ({valor})"),
        )
    })
}

/// Normaliza un índice aceptando negativos desde el final.
pub fn indice_normalizado(indice: i64, longitud: usize) -> Option<usize> {
    let longitud = longitud as i64;
    let real = if indice < 0 {
        longitud + indice
    } else {
        indice
    };
    if real >= 0 && real < longitud {
        Some(real as usize)
    } else {
        None
    }
}
