//! Módulo nativo `quetzal/entorno`.
//!
//! Permite consultar variables del sistema y cargar un archivo `.env` del
//! directorio actual. Los valores cargados se mantienen dentro del runtime de
//! Quetzal: no modifican el entorno global del proceso.
//!
//! Toda la operación pasa antes por el [`GuardianPermisos`]: el programa debe
//! declarar el permiso `entorno` en su `quetzal.json`, porque estas variables
//! suelen contener secretos.

use std::collections::HashMap;
use std::fs;
use std::rc::Rc;
use std::sync::{Mutex, OnceLock};

use maquina_virtual::{Fallo, RegistroNativos, Valor};
use runtime::GuardianPermisos;

use crate::sistema_archivos::error_permiso;
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

/// Verifica el permiso `entorno` para una operación del módulo.
fn exigir_permiso(guardian: &GuardianPermisos, operacion: &str) -> Result<(), Fallo> {
    guardian.verificar_entorno(operacion).map_err(error_permiso)
}

fn obtener(guardian: Rc<GuardianPermisos>, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    exigir_aridad("entorno.obtener", argumentos, 1)?;
    let nombre = arg_texto("entorno.obtener", argumentos, 0)?;
    exigir_permiso(&guardian, &format!("entorno.obtener('{nombre}')"))?;
    Ok(leer_variable(nombre).map_or(Valor::Nulo, Valor::texto))
}

fn obtener_o(guardian: Rc<GuardianPermisos>, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    exigir_aridad("entorno.obtener_o", argumentos, 2)?;
    let nombre = arg_texto("entorno.obtener_o", argumentos, 0)?;
    exigir_permiso(&guardian, &format!("entorno.obtener_o('{nombre}')"))?;
    if let Some(valor) = leer_variable(nombre) {
        return Ok(Valor::texto(valor));
    }
    Ok(Valor::texto(arg_texto("entorno.obtener_o", argumentos, 1)?))
}

fn contiene(guardian: Rc<GuardianPermisos>, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    exigir_aridad("entorno.contiene", argumentos, 1)?;
    let nombre = arg_texto("entorno.contiene", argumentos, 0)?;
    exigir_permiso(&guardian, &format!("entorno.contiene('{nombre}')"))?;
    Ok(Valor::Log(leer_variable(nombre).is_some()))
}

fn cargar(guardian: Rc<GuardianPermisos>, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    exigir_aridad("entorno.cargar", argumentos, 0)?;
    exigir_permiso(&guardian, "entorno.cargar()")?;
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

fn registrar_lecturas(registro: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    let guardian_obtener = Rc::clone(guardian);
    registro.registrar_funcion(
        "entorno.obtener",
        Box::new(move |argumentos| obtener(Rc::clone(&guardian_obtener), argumentos)),
    );
    let guardian_obtener_o = Rc::clone(guardian);
    registro.registrar_funcion(
        "entorno.obtener_o",
        Box::new(move |argumentos| obtener_o(Rc::clone(&guardian_obtener_o), argumentos)),
    );
    let guardian_contiene = Rc::clone(guardian);
    registro.registrar_funcion(
        "entorno.contiene",
        Box::new(move |argumentos| contiene(Rc::clone(&guardian_contiene), argumentos)),
    );
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
pub fn registrar(registro: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    registro.registrar_modulo("entorno");
    registrar_lecturas(registro, guardian);
    let guardian_cargar = Rc::clone(guardian);
    registro.registrar_funcion(
        "entorno.cargar",
        Box::new(move |argumentos| cargar(Rc::clone(&guardian_cargar), argumentos)),
    );
}

#[cfg(test)]
mod pruebas {
    use super::quitar_comillas;
    use paquetes::Permisos;
    use runtime::GuardianPermisos;

    #[test]
    fn quita_comillas_simples_y_dobles() {
        assert_eq!(quitar_comillas("'valor'"), "valor");
        assert_eq!(quitar_comillas("\"valor\""), "valor");
        assert_eq!(quitar_comillas("valor"), "valor");
    }

    #[test]
    fn deniega_sin_permiso_declarado() {
        let guardian = GuardianPermisos::denegado();
        let mensaje = guardian
            .verificar_entorno("entorno.obtener('SECRETO')")
            .err()
            .unwrap();
        assert!(mensaje.contains("permiso de entorno"));
        assert!(mensaje.contains("quetzal.json"));
    }

    #[test]
    fn permite_con_permiso_declarado() {
        let valor: serde_json::Value =
            serde_json::from_str(r#"{"entorno": {"habilitado": true}}"#).unwrap();
        let permisos = Permisos::desde_json(&valor).expect("permisos válidos");
        let guardian = GuardianPermisos::denegado();
        guardian.configurar(permisos, std::path::Path::new("."));
        assert!(
            guardian
                .verificar_entorno("entorno.obtener('MODO')")
                .is_ok()
        );
    }
}
