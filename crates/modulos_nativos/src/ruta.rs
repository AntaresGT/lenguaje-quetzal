//! Módulo nativo `quetzal/sistema_archivos`: el objeto `Ruta`.
//!
//! Una `Ruta` es inmutable: cada operación (`unir`, `padre`, `con_extension`)
//! devuelve una instancia nueva. Las operaciones de texto puro no piden
//! permisos; las que tocan el disco (`existe`, `es_archivo`, `es_directorio`,
//! `absoluta`) consultan el guardián de permisos (lectura).

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use indexmap::IndexMap;
use maquina_virtual::{DatosInstanciaNativa, Fallo, RegistroNativos, Valor};
use runtime::GuardianPermisos;

use crate::util::{arg_texto, error, exigir_aridad};

const TIPO: &str = "Ruta";

/// Función nativa de este módulo, registrable bajo varios nombres.
type Nativa = fn(&[Valor]) -> Result<Valor, Fallo>;

/// Función nativa que necesita consultar el guardián de permisos.
type NativaConPermisos = fn(&GuardianPermisos, &[Valor]) -> Result<Valor, Fallo>;

// ----- Instancias -----

/// Crea una instancia de `Ruta` a partir de su texto.
pub(crate) fn valor_ruta(texto: impl Into<String>) -> Valor {
    let mut datos = IndexMap::new();
    datos.insert("texto".to_string(), Valor::texto(texto.into()));
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO),
        datos: RefCell::new(datos),
    }))
}

/// Receptor (`esto`) de un método: el texto de la ruta.
fn receptor(funcion: &str, argumentos: &[Valor]) -> Result<String, Fallo> {
    match argumentos.first() {
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == TIPO => {
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
                "'{funcion}' esperaba una instancia de Ruta, pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
        None => Err(error(
            "E0210",
            format!("'{funcion}' necesita el receptor"),
        )),
    }
}

/// Une dos partes con la semántica de rutas del sistema operativo.
fn unir_partes(base: &str, parte: &str) -> String {
    PathBuf::from(base).join(parte).to_string_lossy().into_owned()
}

// ----- Constructor: `nuevo Ruta(...)` -----

fn constructor(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "nuevo Ruta";
    match argumentos.len() {
        // `nuevo Ruta("./datos/archivo.txt")`.
        1 => Ok(valor_ruta(arg_texto(F, argumentos, 0)?)),
        // `nuevo Ruta("./datos", "archivo.txt")`.
        2 => {
            let base = arg_texto(F, argumentos, 0)?;
            let parte = arg_texto(F, argumentos, 1)?;
            Ok(valor_ruta(unir_partes(base, parte)))
        }
        otros => Err(error(
            "E0210",
            format!("'{F}' acepta 1 o 2 argumentos, pero recibió {otros}"),
        )),
    }
}

// ----- Funciones libres: `Ruta.x()` -----

fn libre_actual(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Ruta.actual";
    exigir_aridad(F, argumentos, 0)?;
    let actual = std::env::current_dir()
        .map_err(|fallo| error("E0702", format!("'{F}' no pudo obtener el directorio actual: {fallo}")))?;
    Ok(valor_ruta(actual.to_string_lossy().into_owned()))
}

fn libre_temporal(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    exigir_aridad("Ruta.temporal", argumentos, 0)?;
    Ok(valor_ruta(std::env::temp_dir().to_string_lossy().into_owned()))
}

fn libre_separador(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    exigir_aridad("Ruta.separador", argumentos, 0)?;
    Ok(Valor::texto(std::path::MAIN_SEPARATOR.to_string()))
}

// ----- Métodos: texto puro -----

fn metodo_texto(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Ruta.texto";
    exigir_aridad(F, &argumentos[1..], 0)?;
    Ok(Valor::texto(receptor(F, argumentos)?))
}

fn metodo_nombre(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Ruta.nombre";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let ruta = receptor(F, argumentos)?;
    let nombre = Path::new(&ruta)
        .file_name()
        .map(|nombre| nombre.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(Valor::texto(nombre))
}

fn metodo_nombre_sin_extension(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Ruta.nombre_sin_extension";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let ruta = receptor(F, argumentos)?;
    let nombre = Path::new(&ruta)
        .file_stem()
        .map(|nombre| nombre.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(Valor::texto(nombre))
}

fn metodo_extension(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Ruta.extension";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let ruta = receptor(F, argumentos)?;
    let extension = Path::new(&ruta)
        .extension()
        .map(|ext| ext.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(Valor::texto(extension))
}

fn metodo_padre(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Ruta.padre";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let ruta = receptor(F, argumentos)?;
    let padre = Path::new(&ruta)
        .parent()
        .map(|padre| padre.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(valor_ruta(padre))
}

fn metodo_unir(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Ruta.unir";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let ruta = receptor(F, argumentos)?;
    let parte = arg_texto(F, argumentos, 1)?;
    Ok(valor_ruta(unir_partes(&ruta, parte)))
}

fn metodo_con_extension(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Ruta.con_extension";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let ruta = receptor(F, argumentos)?;
    let extension = arg_texto(F, argumentos, 1)?;
    let mut nueva = PathBuf::from(&ruta);
    nueva.set_extension(extension.trim_start_matches('.'));
    Ok(valor_ruta(nueva.to_string_lossy().into_owned()))
}

fn metodo_componentes(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Ruta.componentes";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let ruta = receptor(F, argumentos)?;
    let componentes: Vec<Valor> = Path::new(&ruta)
        .components()
        .map(|componente| Valor::texto(componente.as_os_str().to_string_lossy()))
        .collect();
    Ok(Valor::lista(componentes))
}

fn metodo_es_absoluta(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Ruta.es_absoluta";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let ruta = receptor(F, argumentos)?;
    Ok(Valor::Log(Path::new(&ruta).is_absolute()))
}

// ----- Métodos: tocan disco (permiso de lectura) -----

fn permiso_denegado(mensaje: String) -> Fallo {
    error("E0701", mensaje)
}

fn metodo_absoluta(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Ruta.absoluta";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let ruta = receptor(F, argumentos)?;
    permisos.verificar_lectura(&ruta).map_err(permiso_denegado)?;
    let absoluta = Path::new(&ruta).canonicalize().map_err(|fallo| {
        error("E0702", format!("no se pudo resolver la ruta '{ruta}': {fallo}"))
    })?;
    Ok(valor_ruta(absoluta.to_string_lossy().into_owned()))
}

fn metodo_existe(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Ruta.existe";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let ruta = receptor(F, argumentos)?;
    permisos.verificar_lectura(&ruta).map_err(permiso_denegado)?;
    Ok(Valor::Log(Path::new(&ruta).exists()))
}

fn metodo_es_archivo(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Ruta.es_archivo";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let ruta = receptor(F, argumentos)?;
    permisos.verificar_lectura(&ruta).map_err(permiso_denegado)?;
    Ok(Valor::Log(Path::new(&ruta).is_file()))
}

fn metodo_es_directorio(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Ruta.es_directorio";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let ruta = receptor(F, argumentos)?;
    permisos.verificar_lectura(&ruta).map_err(permiso_denegado)?;
    Ok(Valor::Log(Path::new(&ruta).is_dir()))
}

// ----- Registro -----

pub fn registrar(registro: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    registro.registrar_modulo("ruta");

    // Constructor de `nuevo Ruta(...)`.
    registro.registrar_funcion("ruta.constructor", Box::new(constructor));

    // Funciones libres: `Ruta.actual()`, ...
    let libres: [(&str, Nativa); 3] = [
        ("actual", libre_actual),
        ("temporal", libre_temporal),
        ("separador", libre_separador),
    ];
    for (nombre, funcion) in libres {
        registro.registrar_funcion(&format!("ruta.{nombre}"), Box::new(funcion));
    }

    // Métodos de instancia de texto puro.
    let metodos: [(&str, Nativa); 9] = [
        ("texto", metodo_texto),
        ("nombre", metodo_nombre),
        ("nombre_sin_extension", metodo_nombre_sin_extension),
        ("extension", metodo_extension),
        ("padre", metodo_padre),
        ("unir", metodo_unir),
        ("con_extension", metodo_con_extension),
        ("componentes", metodo_componentes),
        ("es_absoluta", metodo_es_absoluta),
    ];
    for (nombre, funcion) in metodos {
        registro.registrar_funcion(&format!("{TIPO}.{nombre}"), Box::new(funcion));
    }

    // Métodos que tocan el disco: consultan el guardián de permisos.
    let con_permisos: [(&str, NativaConPermisos); 4] = [
        ("absoluta", metodo_absoluta),
        ("existe", metodo_existe),
        ("es_archivo", metodo_es_archivo),
        ("es_directorio", metodo_es_directorio),
    ];
    for (nombre, funcion) in con_permisos {
        let permisos = Rc::clone(guardian);
        registro.registrar_funcion(
            &format!("{TIPO}.{nombre}"),
            Box::new(move |argumentos| funcion(&permisos, argumentos)),
        );
    }
}
