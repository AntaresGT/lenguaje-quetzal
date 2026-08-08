//! Utilidades compartidas por los objetos de `quetzal/red`: construcción de
//! instancias nativas, acceso a sus campos y extracción de argumentos.

use std::cell::RefCell;
use std::rc::Rc;

use indexmap::IndexMap;
use maquina_virtual::{CargaNativa, DatosInstanciaNativa, Fallo, Valor};

use crate::util::error;

/// Excepción por un fallo de red (conexión, protocolo o estado rechazado).
pub(crate) fn error_red(mensaje: impl Into<String>) -> Fallo {
    error("E0408", mensaje.into())
}

/// Excepción por un permiso de red denegado.
pub(crate) fn error_permiso(mensaje: impl Into<String>) -> Fallo {
    error("E0501", mensaje.into())
}

/// Excepción por un argumento con el tipo equivocado.
pub(crate) fn error_tipo(mensaje: impl Into<String>) -> Fallo {
    error("E0406", mensaje.into())
}

/// Crea una instancia nativa con esos campos.
pub(crate) fn instancia(tipo: &str, campos: Vec<(&str, Valor)>) -> Valor {
    let mut datos = IndexMap::new();
    for (nombre, valor) in campos {
        datos.insert(nombre.to_string(), valor);
    }
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(tipo),
        datos: RefCell::new(datos),
    }))
}

/// Valor de un campo de una instancia nativa (`Nulo` si no existe).
pub(crate) fn campo(valor: &Valor, nombre: &str) -> Valor {
    match valor {
        Valor::InstanciaNativa(instancia) => instancia
            .datos
            .borrow()
            .get(nombre)
            .cloned()
            .unwrap_or(Valor::Nulo),
        _ => Valor::Nulo,
    }
}

/// Asigna un campo de una instancia nativa.
pub(crate) fn poner_campo(valor: &Valor, nombre: &str, nuevo: Valor) {
    if let Valor::InstanciaNativa(instancia) = valor {
        instancia.datos.borrow_mut().insert(nombre.to_string(), nuevo);
    }
}

/// Campo entero de una instancia (0 si falta).
pub(crate) fn campo_entero(valor: &Valor, nombre: &str) -> i64 {
    match campo(valor, nombre) {
        Valor::Entero(entero) => entero,
        _ => 0,
    }
}

/// Campo de texto de una instancia (vacío si falta).
pub(crate) fn campo_texto(valor: &Valor, nombre: &str) -> String {
    match campo(valor, nombre) {
        Valor::Texto(texto) => texto.to_string(),
        _ => String::new(),
    }
}

/// Campo lógico de una instancia (falso si falta).
pub(crate) fn campo_log(valor: &Valor, nombre: &str) -> bool {
    matches!(campo(valor, nombre), Valor::Log(true))
}

/// Verifica que el receptor sea del tipo esperado y devuelve su campo `id`.
pub(crate) fn id_receptor(funcion: &str, argumentos: &[Valor], tipo: &str) -> Result<i64, Fallo> {
    let receptor = receptor(funcion, argumentos, tipo)?;
    match campo(&receptor, "id") {
        Valor::Entero(id) => Ok(id),
        _ => Err(error_tipo(format!(
            "'{funcion}' recibió un {tipo} sin identificador válido"
        ))),
    }
}

/// El receptor de un método, verificando su tipo.
pub(crate) fn receptor(funcion: &str, argumentos: &[Valor], tipo: &str) -> Result<Valor, Fallo> {
    match argumentos.first() {
        Some(valor @ Valor::InstanciaNativa(instancia)) if &*instancia.tipo == tipo => {
            Ok(valor.clone())
        }
        Some(otro) => Err(error_tipo(format!(
            "'{funcion}' esperaba un {tipo} como receptor, pero recibió '{}'",
            otro.nombre_tipo()
        ))),
        None => Err(error(
            "E0210",
            format!("'{funcion}' necesita un receptor de tipo {tipo}"),
        )),
    }
}

/// Convierte un mapa de pares en un `jsn` de Quetzal.
pub(crate) fn jsn_de_pares(pares: Vec<(String, String)>) -> Valor {
    let mut mapa = IndexMap::new();
    for (clave, valor) in pares {
        mapa.insert(clave, Valor::texto(valor));
    }
    Valor::jsn(mapa)
}

/// Lee un `jsn` como pares de texto (los valores no textuales se convierten).
pub(crate) fn pares_de_jsn(valor: &Valor) -> Vec<(String, String)> {
    match valor {
        Valor::Jsn(mapa) => mapa
            .borrow()
            .iter()
            .map(|(clave, valor)| {
                (
                    clave.clone(),
                    maquina_virtual::texto_de_valor(valor),
                )
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Busca una clave sin distinguir mayúsculas dentro de un `jsn`.
pub(crate) fn buscar_sin_caso(valor: &Valor, clave: &str) -> Option<Valor> {
    match valor {
        Valor::Jsn(mapa) => mapa
            .borrow()
            .iter()
            .find(|(nombre, _)| nombre.eq_ignore_ascii_case(clave))
            .map(|(_, valor)| valor.clone()),
        _ => None,
    }
}

/// Convierte una carga del bucle de eventos en pares de texto.
pub(crate) fn pares_de_carga(carga: &CargaNativa) -> Vec<(String, String)> {
    match carga {
        CargaNativa::Mapa(pares) => pares
            .iter()
            .map(|(clave, valor)| (clave.clone(), texto_de_carga(valor)))
            .collect(),
        _ => Vec::new(),
    }
}

/// Representación textual de una carga simple.
pub(crate) fn texto_de_carga(carga: &CargaNativa) -> String {
    match carga {
        CargaNativa::Texto(texto) => texto.clone(),
        CargaNativa::Entero(entero) => entero.to_string(),
        CargaNativa::Log(true) => "verdadero".to_string(),
        CargaNativa::Log(false) => "falso".to_string(),
        CargaNativa::Nula => String::new(),
        _ => String::new(),
    }
}

/// Bytes guardados en una carga como lista de enteros.
pub(crate) fn bytes_de_carga(carga: &CargaNativa) -> Vec<u8> {
    match carga {
        CargaNativa::Lista(elementos) => elementos
            .iter()
            .filter_map(|elemento| match elemento {
                CargaNativa::Entero(entero) => u8::try_from(*entero).ok(),
                _ => None,
            })
            .collect(),
        CargaNativa::Texto(texto) => texto.as_bytes().to_vec(),
        _ => Vec::new(),
    }
}

/// Empaqueta bytes como lista de enteros para cruzar al hilo de fondo.
pub(crate) fn carga_de_bytes(bytes: &[u8]) -> CargaNativa {
    CargaNativa::Lista(
        bytes
            .iter()
            .map(|byte| CargaNativa::Entero(*byte as i64))
            .collect(),
    )
}

/// Busca un valor en una carga de tipo mapa.
pub(crate) fn campo_de_carga<'a>(carga: &'a CargaNativa, nombre: &str) -> Option<&'a CargaNativa> {
    match carga {
        CargaNativa::Mapa(pares) => pares
            .iter()
            .find(|(clave, _)| clave == nombre)
            .map(|(_, valor)| valor),
        _ => None,
    }
}
