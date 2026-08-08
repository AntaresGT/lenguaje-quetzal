//! Módulo nativo `quetzal/sistema_archivos`: el objeto `SistemaArchivos`.
//!
//! Expone funciones `libre` (no se instancia) para leer y escribir archivos,
//! crear y listar directorios, comprobar existencia y copiar, mover,
//! renombrar y borrar. Cada función tiene su par `_asincrono`, que hace el
//! trabajo de disco en el runtime de fondo y se resuelve con `esperar`.
//!
//! Toda operación pasa antes por el [`GuardianPermisos`]: la ruta debe estar
//! dentro de los `directorios` declarados en `quetzal.json` y con el nivel
//! suficiente (`lectura`, `escritura` o `todo`). Un rechazo es una excepción
//! de Quetzal, capturable con `intentar` / `capturar`.

use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use maquina_virtual::{
    CargaNativa, EstadoTareaNativa, Fallo, Mensaje, RegistroNativos, Valor, Vm, carga_a_valor,
};
use runtime::GuardianPermisos;

use crate::archivo;
use crate::bits;
use crate::flujo;
use crate::observador;
use crate::util::{arg_texto, error, exigir_aridad};

/// Operación de disco: se ejecuta igual en la forma síncrona (en el hilo de
/// la VM) y en la asíncrona (en el runtime de fondo), por eso devuelve una
/// [`CargaNativa`] y un mensaje de error como texto.
pub(crate) type Salida = Result<CargaNativa, String>;

type OpRuta = fn(PathBuf) -> Salida;
type OpRutaTexto = fn(PathBuf, String) -> Salida;
type OpRutaBytes = fn(PathBuf, Vec<u8>) -> Salida;
type OpDosRutas = fn(PathBuf, PathBuf) -> Salida;

// ----- Permisos y errores -----

/// Excepción por un permiso denegado (capturable con `intentar`).
pub(crate) fn error_permiso(mensaje: String) -> Fallo {
    error("E0501", mensaje)
}

/// Excepción por un fallo del sistema de archivos.
pub(crate) fn error_io(mensaje: String) -> Fallo {
    error("E0407", mensaje)
}

/// Ruta del argumento `indice` verificada para lectura.
pub(crate) fn ruta_lectura(
    guardian: &GuardianPermisos,
    funcion: &str,
    argumentos: &[Valor],
    indice: usize,
) -> Result<PathBuf, Fallo> {
    let ruta = arg_texto(funcion, argumentos, indice)?;
    guardian.verificar_lectura(ruta).map_err(error_permiso)
}

/// Ruta del argumento `indice` verificada para escritura.
pub(crate) fn ruta_escritura(
    guardian: &GuardianPermisos,
    funcion: &str,
    argumentos: &[Valor],
    indice: usize,
) -> Result<PathBuf, Fallo> {
    let ruta = arg_texto(funcion, argumentos, indice)?;
    guardian.verificar_escritura(ruta).map_err(error_permiso)
}

/// Ejecuta la operación en el hilo de la VM y convierte su salida.
pub(crate) fn ejecutar(salida: Salida) -> Result<Valor, Fallo> {
    salida.map(carga_a_valor).map_err(error_io)
}

/// Lanza la operación en el runtime de fondo y devuelve la tarea pendiente
/// que `esperar` resolverá. La verificación de permisos ya ocurrió en el
/// hilo de la VM, así que el hilo de fondo solo toca el disco.
pub(crate) fn tarea<F>(vm: &mut Vm, trabajo: F) -> Result<Valor, Fallo>
where
    F: FnOnce() -> Salida + Send + 'static,
{
    let id = vm.bucle().nuevo_id();
    let manija = vm.bucle().manija();
    manija.clone().runtime().spawn_blocking(move || {
        let resultado = trabajo();
        manija.enviar(Mensaje::TareaLista { id, resultado });
    });
    Ok(Valor::TareaNativa(Rc::new(EstadoTareaNativa { id })))
}

// ----- Operaciones de disco -----

pub(crate) fn op_leer(ruta: PathBuf) -> Salida {
    fs::read(&ruta)
        .map_err(|fallo| formato_error("leer", &ruta, &fallo))
        .and_then(|bytes| {
            String::from_utf8(bytes).map_err(|_| {
                format!(
                    "el archivo '{}' no contiene texto UTF-8 válido; usa 'leer_bits' para datos binarios",
                    ruta.display()
                )
            })
        })
        .map(CargaNativa::Texto)
}

pub(crate) fn op_leer_bits(ruta: PathBuf) -> Salida {
    fs::read(&ruta)
        .map_err(|fallo| formato_error("leer_bits", &ruta, &fallo))
        .map(|bytes| bits::carga(&bytes))
}

pub(crate) fn op_escribir(ruta: PathBuf, contenido: String) -> Salida {
    fs::write(&ruta, contenido)
        .map_err(|fallo| formato_error("escribir", &ruta, &fallo))
        .map(|_| CargaNativa::Nula)
}

pub(crate) fn op_escribir_bits(ruta: PathBuf, bytes: Vec<u8>) -> Salida {
    fs::write(&ruta, bytes)
        .map_err(|fallo| formato_error("escribir_bits", &ruta, &fallo))
        .map(|_| CargaNativa::Nula)
}

pub(crate) fn op_anexar(ruta: PathBuf, contenido: String) -> Salida {
    anexar_bytes("anexar", &ruta, contenido.as_bytes())
}

pub(crate) fn op_anexar_bits(ruta: PathBuf, bytes: Vec<u8>) -> Salida {
    anexar_bytes("anexar_bits", &ruta, &bytes)
}

/// Agrega bytes al final del archivo, creándolo si no existe.
fn anexar_bytes(funcion: &str, ruta: &Path, bytes: &[u8]) -> Salida {
    use std::io::Write;

    let mut archivo = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(ruta)
        .map_err(|fallo| formato_error(funcion, ruta, &fallo))?;
    archivo
        .write_all(bytes)
        .map_err(|fallo| formato_error(funcion, ruta, &fallo))
        .map(|_| CargaNativa::Nula)
}

fn op_crear_directorio(ruta: PathBuf) -> Salida {
    fs::create_dir(&ruta)
        .map_err(|fallo| formato_error("crear_directorio", &ruta, &fallo))
        .map(|_| archivo::carga(&ruta))
}

fn op_crear_directorios(ruta: PathBuf) -> Salida {
    fs::create_dir_all(&ruta)
        .map_err(|fallo| formato_error("crear_directorios", &ruta, &fallo))
        .map(|_| archivo::carga(&ruta))
}

pub(crate) fn op_listar_archivos(ruta: PathBuf) -> Salida {
    listar(&ruta, "listar_archivos", false)
}

pub(crate) fn op_listar_directorios(ruta: PathBuf) -> Salida {
    listar(&ruta, "listar_directorios", true)
}

/// Nombres (sin ruta) de las entradas del directorio, ordenados.
fn listar(ruta: &Path, funcion: &str, directorios: bool) -> Salida {
    let entradas = fs::read_dir(ruta).map_err(|fallo| formato_error(funcion, ruta, &fallo))?;
    let mut nombres = Vec::new();
    for entrada in entradas {
        let entrada = entrada.map_err(|fallo| formato_error(funcion, ruta, &fallo))?;
        let es_directorio = entrada.path().is_dir();
        if es_directorio == directorios {
            nombres.push(entrada.file_name().to_string_lossy().to_string());
        }
    }
    nombres.sort();
    Ok(CargaNativa::Lista(
        nombres.into_iter().map(CargaNativa::Texto).collect(),
    ))
}

pub(crate) fn op_existe(ruta: PathBuf) -> Salida {
    Ok(CargaNativa::Log(ruta.exists()))
}

pub(crate) fn op_es_archivo(ruta: PathBuf) -> Salida {
    Ok(CargaNativa::Log(ruta.is_file()))
}

pub(crate) fn op_es_directorio(ruta: PathBuf) -> Salida {
    Ok(CargaNativa::Log(ruta.is_dir()))
}

pub(crate) fn op_abrir(ruta: PathBuf) -> Salida {
    Ok(archivo::carga(&ruta))
}

pub(crate) fn op_borrar(ruta: PathBuf) -> Salida {
    let resultado = if ruta.is_dir() {
        // `borrar` solo quita directorios vacíos: el mensaje señala la
        // alternativa en vez de dejar al usuario adivinando.
        fs::remove_dir(&ruta).map_err(|fallo| {
            format!(
                "no se pudo borrar el directorio '{}': {fallo}; si no está vacío usa \
                 'borrar_recursivo'",
                ruta.display()
            )
        })
    } else {
        fs::remove_file(&ruta).map_err(|fallo| formato_error("borrar", &ruta, &fallo))
    };
    resultado.map(|_| CargaNativa::Nula)
}

pub(crate) fn op_borrar_recursivo(ruta: PathBuf) -> Salida {
    let resultado = if ruta.is_dir() {
        fs::remove_dir_all(&ruta)
    } else {
        fs::remove_file(&ruta)
    };
    resultado
        .map_err(|fallo| formato_error("borrar_recursivo", &ruta, &fallo))
        .map(|_| CargaNativa::Nula)
}

pub(crate) fn op_copiar(origen: PathBuf, destino: PathBuf) -> Salida {
    if origen.is_dir() {
        copiar_directorio(&origen, &destino)?;
    } else {
        fs::copy(&origen, &destino).map_err(|fallo| {
            format!(
                "no se pudo copiar '{}' a '{}': {fallo}",
                origen.display(),
                destino.display()
            )
        })?;
    }
    Ok(CargaNativa::Nula)
}

fn copiar_directorio(origen: &Path, destino: &Path) -> Result<(), String> {
    fs::create_dir_all(destino).map_err(|fallo| formato_error("copiar", destino, &fallo))?;
    let entradas = fs::read_dir(origen).map_err(|fallo| formato_error("copiar", origen, &fallo))?;
    for entrada in entradas {
        let entrada = entrada.map_err(|fallo| formato_error("copiar", origen, &fallo))?;
        let ruta = entrada.path();
        let hijo = destino.join(entrada.file_name());
        if ruta.is_dir() {
            copiar_directorio(&ruta, &hijo)?;
        } else {
            fs::copy(&ruta, &hijo).map_err(|fallo| {
                format!(
                    "no se pudo copiar '{}' a '{}': {fallo}",
                    ruta.display(),
                    hijo.display()
                )
            })?;
        }
    }
    Ok(())
}

pub(crate) fn op_mover(origen: PathBuf, destino: PathBuf) -> Salida {
    if fs::rename(&origen, &destino).is_ok() {
        return Ok(CargaNativa::Nula);
    }
    // Distinto volumen (o Windows con destino en otra unidad): copiar y borrar.
    op_copiar(origen.clone(), destino)?;
    op_borrar_recursivo(origen)
}

/// Renombra dentro del mismo directorio: el segundo argumento es un nombre,
/// no una ruta.
pub(crate) fn destino_de_renombrar(ruta: &Path, nombre: &str) -> Result<PathBuf, Fallo> {
    if nombre.contains('/') || nombre.contains('\\') {
        return Err(error_io(format!(
            "'renombrar' espera un nombre, no una ruta ('{nombre}'); usa 'mover' para cambiar de \
             directorio"
        )));
    }
    let padre = ruta.parent().unwrap_or(Path::new("."));
    Ok(padre.join(nombre))
}

fn formato_error(funcion: &str, ruta: &Path, fallo: &std::io::Error) -> String {
    format!("'{funcion}' falló sobre '{}': {fallo}", ruta.display())
}

/// Marca Unix en milisegundos de un instante del sistema de archivos.
pub(crate) fn marca_de(tiempo: SystemTime) -> i64 {
    tiempo
        .duration_since(UNIX_EPOCH)
        .map(|duracion| duracion.as_millis() as i64)
        .unwrap_or(0)
}

// ----- Registro -----

/// Registra la pareja síncrona/asíncrona de una operación sobre una ruta.
fn registrar_ruta(
    registro: &mut RegistroNativos,
    guardian: &Rc<GuardianPermisos>,
    nombre: &'static str,
    escritura: bool,
    operacion: OpRuta,
) {
    let funcion = format!("SistemaArchivos.{nombre}");

    let guardian_sincrono = Rc::clone(guardian);
    let etiqueta = funcion.clone();
    registro.registrar_funcion(
        &format!("sistema_archivos.{nombre}"),
        Box::new(move |argumentos| {
            exigir_aridad(&etiqueta, argumentos, 1)?;
            let ruta = verificar(&guardian_sincrono, &etiqueta, argumentos, 0, escritura)?;
            ejecutar(operacion(ruta))
        }),
    );

    let guardian_asincrono = Rc::clone(guardian);
    registro.registrar_funcion_con_vm(
        &format!("sistema_archivos.{nombre}_asincrono"),
        Box::new(move |vm, argumentos| {
            exigir_aridad(&funcion, argumentos, 1)?;
            let ruta = verificar(&guardian_asincrono, &funcion, argumentos, 0, escritura)?;
            tarea(vm, move || operacion(ruta))
        }),
    );
}

/// Registra la pareja de una operación con ruta y un contenido de texto.
fn registrar_ruta_texto(
    registro: &mut RegistroNativos,
    guardian: &Rc<GuardianPermisos>,
    nombre: &'static str,
    operacion: OpRutaTexto,
) {
    let funcion = format!("SistemaArchivos.{nombre}");

    let guardian_sincrono = Rc::clone(guardian);
    let etiqueta = funcion.clone();
    registro.registrar_funcion(
        &format!("sistema_archivos.{nombre}"),
        Box::new(move |argumentos| {
            exigir_aridad(&etiqueta, argumentos, 2)?;
            let ruta = ruta_escritura(&guardian_sincrono, &etiqueta, argumentos, 0)?;
            let contenido = arg_texto(&etiqueta, argumentos, 1)?.to_string();
            ejecutar(operacion(ruta, contenido))
        }),
    );

    let guardian_asincrono = Rc::clone(guardian);
    registro.registrar_funcion_con_vm(
        &format!("sistema_archivos.{nombre}_asincrono"),
        Box::new(move |vm, argumentos| {
            exigir_aridad(&funcion, argumentos, 2)?;
            let ruta = ruta_escritura(&guardian_asincrono, &funcion, argumentos, 0)?;
            let contenido = arg_texto(&funcion, argumentos, 1)?.to_string();
            tarea(vm, move || operacion(ruta, contenido))
        }),
    );
}

/// Registra la pareja de una operación con origen y destino.
fn registrar_dos_rutas(
    registro: &mut RegistroNativos,
    guardian: &Rc<GuardianPermisos>,
    nombre: &'static str,
    escritura_en_origen: bool,
    operacion: OpDosRutas,
) {
    let funcion = format!("SistemaArchivos.{nombre}");

    let guardian_sincrono = Rc::clone(guardian);
    let etiqueta = funcion.clone();
    registro.registrar_funcion(
        &format!("sistema_archivos.{nombre}"),
        Box::new(move |argumentos| {
            exigir_aridad(&etiqueta, argumentos, 2)?;
            let (origen, destino) = rutas_par(
                &guardian_sincrono,
                &etiqueta,
                argumentos,
                escritura_en_origen,
            )?;
            ejecutar(operacion(origen, destino))
        }),
    );

    let guardian_asincrono = Rc::clone(guardian);
    registro.registrar_funcion_con_vm(
        &format!("sistema_archivos.{nombre}_asincrono"),
        Box::new(move |vm, argumentos| {
            exigir_aridad(&funcion, argumentos, 2)?;
            let (origen, destino) = rutas_par(
                &guardian_asincrono,
                &funcion,
                argumentos,
                escritura_en_origen,
            )?;
            tarea(vm, move || operacion(origen, destino))
        }),
    );
}

fn rutas_par(
    guardian: &GuardianPermisos,
    funcion: &str,
    argumentos: &[Valor],
    escritura_en_origen: bool,
) -> Result<(PathBuf, PathBuf), Fallo> {
    let origen = ruta_lectura(guardian, funcion, argumentos, 0)?;
    if escritura_en_origen {
        ruta_escritura(guardian, funcion, argumentos, 0)?;
    }
    let destino = ruta_escritura(guardian, funcion, argumentos, 1)?;
    Ok((origen, destino))
}

fn verificar(
    guardian: &GuardianPermisos,
    funcion: &str,
    argumentos: &[Valor],
    indice: usize,
    escritura: bool,
) -> Result<PathBuf, Fallo> {
    if escritura {
        ruta_escritura(guardian, funcion, argumentos, indice)
    } else {
        ruta_lectura(guardian, funcion, argumentos, indice)
    }
}

/// `renombrar(ruta, nombre)`: el nuevo nombre vive en el mismo directorio.
fn registrar_renombrar(registro: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    const F: &str = "SistemaArchivos.renombrar";

    let guardian_sincrono = Rc::clone(guardian);
    registro.registrar_funcion(
        "sistema_archivos.renombrar",
        Box::new(move |argumentos| {
            let (origen, destino) = rutas_de_renombrar(&guardian_sincrono, argumentos)?;
            ejecutar(op_mover(origen, destino))
        }),
    );

    let guardian_asincrono = Rc::clone(guardian);
    registro.registrar_funcion_con_vm(
        "sistema_archivos.renombrar_asincrono",
        Box::new(move |vm, argumentos| {
            let (origen, destino) = rutas_de_renombrar(&guardian_asincrono, argumentos)?;
            tarea(vm, move || op_mover(origen, destino))
        }),
    );

    fn rutas_de_renombrar(
        guardian: &GuardianPermisos,
        argumentos: &[Valor],
    ) -> Result<(PathBuf, PathBuf), Fallo> {
        exigir_aridad(F, argumentos, 2)?;
        let origen = ruta_escritura(guardian, F, argumentos, 0)?;
        let nombre = arg_texto(F, argumentos, 1)?;
        let destino = destino_de_renombrar(&origen, nombre)?;
        guardian
            .verificar_escritura(&destino.to_string_lossy())
            .map_err(error_permiso)?;
        Ok((origen, destino))
    }
}

/// Registra la pareja de una operación con ruta y contenido binario
/// (`escribir_bits`, `anexar_bits`): el contenido llega como objeto `Bits`.
fn registrar_ruta_bits(
    registro: &mut RegistroNativos,
    guardian: &Rc<GuardianPermisos>,
    nombre: &'static str,
    operacion: OpRutaBytes,
) {
    let funcion = format!("SistemaArchivos.{nombre}");

    let guardian_sincrono = Rc::clone(guardian);
    let etiqueta = funcion.clone();
    registro.registrar_funcion(
        &format!("sistema_archivos.{nombre}"),
        Box::new(move |argumentos| {
            exigir_aridad(&etiqueta, argumentos, 2)?;
            let ruta = ruta_escritura(&guardian_sincrono, &etiqueta, argumentos, 0)?;
            let bytes = bits::arg_bits(&etiqueta, argumentos, 1)?;
            ejecutar(operacion(ruta, bytes))
        }),
    );

    let guardian_asincrono = Rc::clone(guardian);
    registro.registrar_funcion_con_vm(
        &format!("sistema_archivos.{nombre}_asincrono"),
        Box::new(move |vm, argumentos| {
            exigir_aridad(&funcion, argumentos, 2)?;
            let ruta = ruta_escritura(&guardian_asincrono, &funcion, argumentos, 0)?;
            let bytes = bits::arg_bits(&funcion, argumentos, 1)?;
            tarea(vm, move || operacion(ruta, bytes))
        }),
    );
}

/// Registra el módulo `sistema_archivos` con el guardián de permisos.
pub fn registrar(registro: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    registro.registrar_modulo("sistema_archivos");

    // Operaciones de lectura sobre una ruta.
    let lecturas: [(&'static str, OpRuta); 7] = [
        ("leer", op_leer),
        ("leer_bits", op_leer_bits),
        ("listar_archivos", op_listar_archivos),
        ("listar_directorios", op_listar_directorios),
        ("existe", op_existe),
        ("es_archivo", op_es_archivo),
        ("es_directorio", op_es_directorio),
    ];
    for (nombre, operacion) in lecturas {
        registrar_ruta(registro, guardian, nombre, false, operacion);
    }
    registrar_ruta(registro, guardian, "abrir", false, op_abrir);

    // Operaciones de escritura sobre una ruta.
    let escrituras: [(&'static str, OpRuta); 4] = [
        ("crear_directorio", op_crear_directorio),
        ("crear_directorios", op_crear_directorios),
        ("borrar", op_borrar),
        ("borrar_recursivo", op_borrar_recursivo),
    ];
    for (nombre, operacion) in escrituras {
        registrar_ruta(registro, guardian, nombre, true, operacion);
    }

    registrar_ruta_texto(registro, guardian, "escribir", op_escribir);
    registrar_ruta_texto(registro, guardian, "anexar", op_anexar);
    registrar_ruta_bits(registro, guardian, "escribir_bits", op_escribir_bits);
    registrar_ruta_bits(registro, guardian, "anexar_bits", op_anexar_bits);
    registrar_dos_rutas(registro, guardian, "copiar", false, op_copiar);
    registrar_dos_rutas(registro, guardian, "mover", true, op_mover);
    registrar_renombrar(registro, guardian);

    // `abrir_flujo` y `observar` fabrican los objetos con estado propio.
    flujo::registrar_apertura(registro, guardian);
    observador::registrar_apertura(registro, guardian);
}
