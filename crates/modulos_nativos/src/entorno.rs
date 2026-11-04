//! Módulo nativo `quetzal/entorno`.
//!
//! Permite consultar variables del sistema y cargar un archivo `.env` del
//! directorio actual. Los valores cargados se mantienen dentro del runtime de
//! Quetzal: no modifican el entorno global del proceso.

use std::collections::HashMap;
use std::fs;
use std::sync::{Mutex, OnceLock};

use maquina_virtual::{Fallo, RegistroNativos, Valor};

use crate::util::{arg_texto, error, exigir_aridad};

static VARIABLES_CARGADAS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn variables_cargadas() -> &'static Mutex<HashMap<String, String>> {
    VARIABLES_CARGADAS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn leer_variable(nombre: &str) -> Option<String> {
    variables_cargadas()
        .lock()
        .ok()
        .and_then(|variables| variables.get(nombre).cloned())
        .or_else(|| std::env::var(nombre).ok())
}

fn obtener(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    exigir_aridad("entorno.obtener", argumentos, 1)?;
    let nombre = arg_texto("entorno.obtener", argumentos, 0)?;
    Ok(leer_variable(nombre).map_or(Valor::Nulo, Valor::texto))
}

fn obtener_o(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    exigir_aridad("entorno.obtener_o", argumentos, 2)?;
    let nombre = arg_texto("entorno.obtener_o", argumentos, 0)?;
    if let Some(valor) = leer_variable(nombre) {
        return Ok(Valor::texto(valor));
    }
    Ok(Valor::texto(arg_texto("entorno.obtener_o", argumentos, 1)?))
}

fn contiene(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    exigir_aridad("entorno.contiene", argumentos, 1)?;
    let nombre = arg_texto("entorno.contiene", argumentos, 0)?;
    Ok(Valor::Log(leer_variable(nombre).is_some()))
}

fn cargar(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    exigir_aridad("entorno.cargar", argumentos, 0)?;
    let contenido = match fs::read_to_string(".env") {
        Ok(contenido) => contenido,
        Err(error_fs) if error_fs.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Valor::Entero(0));
        }
        Err(error_fs) => {
            return Err(error(
                "E0701",
                format!("no se pudo leer el archivo .env: {error_fs}"),
            ));
        }
    };

    let mut nuevas = HashMap::new();
    for (indice, linea) in contenido.lines().enumerate() {
        let linea = linea.trim();
        if linea.is_empty() || linea.starts_with('#') {
            continue;
        }
        let linea = linea.strip_prefix("export ").unwrap_or(linea);
        let Some((nombre, valor)) = linea.split_once('=') else {
            return Err(error(
                "E0703",
                format!(
                    "línea {} inválida en .env; se esperaba NOMBRE=valor",
                    indice + 1
                ),
            ));
        };
        let nombre = nombre.trim();
        if nombre.is_empty()
            || !nombre
                .chars()
                .all(|caracter| caracter == '_' || caracter.is_ascii_alphanumeric())
        {
            return Err(error(
                "E0703",
                format!("nombre de variable inválido en .env, línea {}", indice + 1),
            ));
        }
        nuevas.insert(nombre.to_string(), quitar_comillas(valor.trim()));
    }

    let cantidad = nuevas.len() as i64;
    variables_cargadas()
        .lock()
        .map_err(|_| error("E0704", "no se pudo actualizar el entorno cargado"))?
        .extend(nuevas);
    Ok(Valor::Entero(cantidad))
}

fn quitar_comillas(valor: &str) -> String {
    if valor.len() >= 2
        && ((valor.starts_with('"') && valor.ends_with('"'))
            || (valor.starts_with('\'') && valor.ends_with('\'')))
    {
        valor[1..valor.len() - 1].to_string()
    } else {
        valor.to_string()
    }
}

/// Registra el módulo `quetzal/entorno`.
pub fn registrar(registro: &mut RegistroNativos) {
    registro.registrar_modulo("entorno");
    registro.registrar_funcion("entorno.obtener", Box::new(obtener));
    registro.registrar_funcion("entorno.obtener_o", Box::new(obtener_o));
    registro.registrar_funcion("entorno.contiene", Box::new(contiene));
    registro.registrar_funcion("entorno.cargar", Box::new(cargar));
}

#[cfg(test)]
mod pruebas {
    use super::quitar_comillas;

    #[test]
    fn quita_comillas_simples_y_dobles() {
        assert_eq!(quitar_comillas("'valor'"), "valor");
        assert_eq!(quitar_comillas("\"valor\""), "valor");
        assert_eq!(quitar_comillas("valor"), "valor");
    }
}
