use crate::errores::{CodigoError, Error, Resultado};
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::path::Path;

pub const ESQUEMA_QUETZAL_JSON: &str = include_str!("../../ejemplos/esquema_quetzal.json");

fn error_esquema(ruta: &Path, mensaje: impl Into<String>) -> Error {
    Error::analisis(
        CodigoError::SintaxisGeneral,
        format!("quetzal.json inválido '{}': {}", ruta.display(), mensaje.into()),
        Some(ruta.display().to_string()),
        None,
        None,
    )
}

fn cargar_esquema() -> Value {
    serde_json::from_str(ESQUEMA_QUETZAL_JSON).expect("el esquema embebido de Quetzal debe ser JSON válido")
}

fn claves_objeto(schema: &Value, puntero: &str) -> HashSet<String> {
    schema
        .pointer(puntero)
        .and_then(Value::as_object)
        .map(|objeto| objeto.keys().cloned().collect())
        .unwrap_or_default()
}

fn enum_texto(schema: &Value, puntero: &str) -> HashSet<String> {
    schema
        .pointer(puntero)
        .and_then(Value::as_array)
        .map(|valores| {
            valores
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn obtener_objeto<'a>(ruta: &Path, valor: &'a Value) -> Resultado<&'a Map<String, Value>> {
    valor
        .as_object()
        .ok_or_else(|| error_esquema(ruta, "la raíz debe ser un objeto JSON"))
}

fn obtener_texto<'a>(ruta: &Path, clave: &str, valor: &'a Value) -> Resultado<&'a str> {
    valor
        .as_str()
        .ok_or_else(|| error_esquema(ruta, format!("'{}' debe ser texto", clave)))
}

fn validar_semver(ruta: &Path, clave: &str, valor: &Value) -> Resultado<()> {
    let texto = obtener_texto(ruta, clave, valor)?;
    let partes: Vec<&str> = texto.split('.').collect();
    let es_semver = partes.len() == 3
        && partes
            .iter()
            .all(|parte| !parte.is_empty() && parte.chars().all(|caracter| caracter.is_ascii_digit()));

    if es_semver {
        Ok(())
    } else {
        Err(error_esquema(
            ruta,
            format!("'{}' debe tener formato semántico 'x.y.z'", clave),
        ))
    }
}

fn es_nombre_proyecto_valido(nombre: &str) -> bool {
    !nombre.is_empty()
        && nombre.chars().all(|caracter| {
            caracter.is_ascii_alphanumeric()
                || matches!(
                    caracter,
                    '_' | '-' | 'ñ' | 'Ñ' | 'á' | 'é' | 'í' | 'ó' | 'ú' | 'Á' | 'É' | 'Í' | 'Ó' | 'Ú'
                )
        })
}

fn validar_nombre_proyecto(ruta: &Path, clave: &str, valor: &Value) -> Resultado<()> {
    let texto = obtener_texto(ruta, clave, valor)?;
    if es_nombre_proyecto_valido(texto) {
        Ok(())
    } else {
        Err(error_esquema(
            ruta,
            format!("'{}' contiene caracteres no permitidos", clave),
        ))
    }
}

fn validar_dependencias(ruta: &Path, valor: &Value) -> Resultado<()> {
    let dependencias = valor
        .as_object()
        .ok_or_else(|| error_esquema(ruta, "'dependencias' debe ser un objeto"))?;

    for (nombre, version_o_ruta) in dependencias {
        let texto = version_o_ruta.as_str().ok_or_else(|| {
            error_esquema(
                ruta,
                format!("la dependencia '{}' debe tener una versión o ruta en texto", nombre),
            )
        })?;

        if texto.trim().is_empty() {
            return Err(error_esquema(
                ruta,
                format!("la dependencia '{}' no puede ser vacía", nombre),
            ));
        }
    }

    Ok(())
}

fn validar_directorios(ruta: &Path, valor: &Value) -> Resultado<()> {
    let directorios = valor
        .as_array()
        .ok_or_else(|| error_esquema(ruta, "'directorios' debe ser una lista de rutas"))?;

    for directorio in directorios {
        if directorio.as_str().is_none() {
            return Err(error_esquema(ruta, "cada directorio permitido debe ser texto"));
        }
    }

    Ok(())
}

fn validar_alcance_permiso(ruta: &Path, valor: &Value) -> Resultado<()> {
    if valor.is_string() {
        return Ok(());
    }

    let operaciones = valor
        .as_array()
        .ok_or_else(|| error_esquema(ruta, "'alcance' debe ser texto o lista de operaciones"))?;
    let permitidas = ["lectura", "escritura", "ejecucion"];

    for operacion in operaciones {
        let operacion = operacion.as_str().ok_or_else(|| {
            error_esquema(ruta, "cada operación del alcance debe ser texto")
        })?;

        if !permitidas.contains(&operacion) {
            return Err(error_esquema(
                ruta,
                format!("la operación '{}' no es válida en 'alcance'", operacion),
            ));
        }
    }

    Ok(())
}

fn validar_permisos(ruta: &Path, schema: &Value, valor: &Value) -> Resultado<()> {
    let permisos = valor
        .as_array()
        .ok_or_else(|| error_esquema(ruta, "'permisos' debe ser una lista"))?;
    let claves_permitidas = claves_objeto(schema, "/properties/permisos/items/properties");
    let tipos_permitidos = enum_texto(schema, "/properties/permisos/items/properties/tipo/enum");

    for permiso in permisos {
        let permiso = permiso
            .as_object()
            .ok_or_else(|| error_esquema(ruta, "cada permiso debe ser un objeto"))?;

        for clave in permiso.keys() {
            if !claves_permitidas.contains(clave) {
                return Err(error_esquema(
                    ruta,
                    format!("la propiedad '{}' no está permitida dentro de 'permisos'", clave),
                ));
            }
        }

        let tipo = permiso
            .get("tipo")
            .and_then(Value::as_str)
            .ok_or_else(|| error_esquema(ruta, "cada permiso debe incluir 'tipo' como texto"))?;
        if !tipos_permitidos.contains(tipo) {
            return Err(error_esquema(
                ruta,
                format!("el permiso '{}' no está permitido por el esquema", tipo),
            ));
        }

        if !permiso.get("habilitado").map(Value::is_boolean).unwrap_or(false) {
            return Err(error_esquema(
                ruta,
                "cada permiso debe incluir 'habilitado' como booleano",
            ));
        }

        if let Some(alcance) = permiso.get("alcance") {
            validar_alcance_permiso(ruta, alcance)?;
        } else if tipo == "sistema-archivos" {
            return Err(error_esquema(
                ruta,
                "el permiso 'sistema-archivos' requiere el campo 'alcance'",
            ));
        }

        if let Some(directorios) = permiso.get("directorios") {
            validar_directorios(ruta, directorios)?;
        }
    }

    Ok(())
}

pub fn validar_configuracion_quetzal(ruta: &Path, contenido: &str) -> Resultado<()> {
    let schema = cargar_esquema();
    let valor: Value = serde_json::from_str(contenido).map_err(|e| {
        error_esquema(ruta, format!("no es JSON válido: {}", e))
    })?;
    let objeto = obtener_objeto(ruta, &valor)?;
    let claves_permitidas = claves_objeto(&schema, "/properties");
    let tipos_permitidos = enum_texto(&schema, "/properties/tipo/enum");

    for clave in objeto.keys() {
        if !claves_permitidas.contains(clave) {
            return Err(error_esquema(
                ruta,
                format!("la propiedad '{}' no está permitida por el esquema", clave),
            ));
        }
    }

    let tiene_campos_legado = objeto.contains_key("versión") && objeto.contains_key("aplicación");
    let tiene_campos_actuales = objeto.contains_key("version") && objeto.contains_key("nombre");
    if !tiene_campos_legado && !tiene_campos_actuales {
        return Err(error_esquema(
            ruta,
            "debe incluir 'versión' y 'aplicación', o bien 'version' y 'nombre'",
        ));
    }

    if let Some(version) = objeto.get("versión") {
        validar_semver(ruta, "versión", version)?;
    }
    if let Some(version) = objeto.get("version") {
        validar_semver(ruta, "version", version)?;
    }
    if let Some(nombre) = objeto.get("aplicación") {
        validar_nombre_proyecto(ruta, "aplicación", nombre)?;
    }
    if let Some(nombre) = objeto.get("nombre") {
        validar_nombre_proyecto(ruta, "nombre", nombre)?;
    }

    if let Some(tipo) = objeto.get("tipo") {
        let tipo = obtener_texto(ruta, "tipo", tipo)?;
        if !tipos_permitidos.contains(tipo) {
            return Err(error_esquema(
                ruta,
                format!("'tipo' solo puede ser uno de: {}", {
                    let mut valores: Vec<String> = tipos_permitidos.iter().cloned().collect();
                    valores.sort();
                    valores.join(", ")
                }),
            ));
        }
    }

    for clave in ["entrada", "biblioteca"] {
        if let Some(valor) = objeto.get(clave) {
            let texto = obtener_texto(ruta, clave, valor)?;
            if texto.trim().is_empty() {
                return Err(error_esquema(
                    ruta,
                    format!("'{}' no puede estar vacío", clave),
                ));
            }
        }
    }

    if let Some(dependencias) = objeto.get("dependencias") {
        validar_dependencias(ruta, dependencias)?;
    }

    if let Some(permisos) = objeto.get("permisos") {
        validar_permisos(ruta, &schema, permisos)?;
    }

    if let Some(nativos) = objeto.get("nativos") {
        let nativos = nativos
            .as_array()
            .ok_or_else(|| error_esquema(ruta, "'nativos' debe ser una lista"))?;
        for nativo in nativos {
            if nativo.as_str().is_none() {
                return Err(error_esquema(ruta, "cada entrada de 'nativos' debe ser texto"));
            }
        }
    }

    Ok(())
}
