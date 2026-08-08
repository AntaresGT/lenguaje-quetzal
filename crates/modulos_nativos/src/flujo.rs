//! Objeto `Flujo` del módulo nativo `quetzal/sistema_archivos`.
//!
//! Un `Flujo` es un archivo abierto con **posición (cursor) persistente**:
//! recuerda en qué byte quedó, así que se puede leer o escribir por partes
//! sin recorrer el archivo completo. Se obtiene con
//! `SistemaArchivos.abrir_flujo(ruta, modo)` y se libera con `cerrar()`.
//!
//! El archivo abierto no cabe en un [`Valor`] (la VM solo guarda valores de
//! Quetzal), así que vive en un registro compartido y la instancia solo
//! lleva su identificador. El registro usa `Arc<Mutex<...>>` para que las
//! variantes `_asincrono` puedan trabajar en el runtime de fondo.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use indexmap::IndexMap;
use maquina_virtual::{CargaNativa, DatosInstanciaNativa, Fallo, RegistroNativos, Valor};
use runtime::GuardianPermisos;

use crate::bits;
use crate::sistema_archivos::{Salida, ejecutar, error_io, error_permiso, tarea};
use crate::util::{arg_entero, arg_texto, error, exigir_aridad};

/// Nombre del tipo visible para el usuario.
pub(crate) const TIPO: &str = "Flujo";

// ----- Modos de apertura -----

/// Cómo se abre el archivo y qué operaciones admite después.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Modo {
    Lectura,
    Escritura,
    LecturaEscritura,
    Anexar,
}

impl Modo {
    fn desde_texto(funcion: &str, texto: &str) -> Result<Self, Fallo> {
        match texto {
            "lectura" => Ok(Modo::Lectura),
            "escritura" => Ok(Modo::Escritura),
            "lectura_escritura" => Ok(Modo::LecturaEscritura),
            "anexar" => Ok(Modo::Anexar),
            otro => Err(error(
                "E0406",
                format!(
                    "'{funcion}' no reconoce el modo '{otro}'; usa 'lectura', 'escritura', \
                     'lectura_escritura' o 'anexar'"
                ),
            )),
        }
    }

    fn nombre(self) -> &'static str {
        match self {
            Modo::Lectura => "lectura",
            Modo::Escritura => "escritura",
            Modo::LecturaEscritura => "lectura_escritura",
            Modo::Anexar => "anexar",
        }
    }

    fn permite_lectura(self) -> bool {
        matches!(self, Modo::Lectura | Modo::LecturaEscritura)
    }

    fn permite_escritura(self) -> bool {
        matches!(
            self,
            Modo::Escritura | Modo::LecturaEscritura | Modo::Anexar
        )
    }

    fn abrir(self, ruta: &Path) -> std::io::Result<File> {
        let mut opciones = OpenOptions::new();
        match self {
            Modo::Lectura => opciones.read(true),
            Modo::Escritura => opciones.write(true).create(true).truncate(true),
            Modo::LecturaEscritura => opciones.read(true).write(true).create(true),
            Modo::Anexar => opciones.append(true).create(true),
        };
        opciones.open(ruta)
    }
}

// ----- Registro de flujos abiertos -----

/// Archivo abierto con su modo. `archivo` en `None` significa cerrado.
struct EstadoFlujo {
    archivo: Option<File>,
    ruta: PathBuf,
    modo: Modo,
}

impl EstadoFlujo {
    /// El archivo abierto, o el error de un flujo ya cerrado.
    fn archivo(&mut self, funcion: &str) -> Result<&mut File, String> {
        self.archivo.as_mut().ok_or_else(|| {
            format!(
                "'{funcion}' no puede operar sobre un flujo cerrado ('{}'); abre uno nuevo con \
                 'SistemaArchivos.abrir_flujo'",
                self.ruta.display()
            )
        })
    }
}

/// Manija enviable al runtime de fondo (por eso `Arc<Mutex<...>>`).
type Manija = Arc<Mutex<EstadoFlujo>>;

/// Flujos abiertos del programa, por identificador.
#[derive(Clone, Default)]
struct RegistroFlujos {
    abiertos: Arc<Mutex<HashMap<i64, Manija>>>,
    siguiente: Arc<AtomicI64>,
}

impl RegistroFlujos {
    fn abrir(&self, funcion: &str, ruta: &Path, modo: Modo) -> Result<i64, String> {
        let archivo = modo
            .abrir(ruta)
            .map_err(|fallo| format!("'{funcion}' no pudo abrir '{}': {fallo}", ruta.display()))?;
        let id = self.siguiente.fetch_add(1, Ordering::Relaxed) + 1;
        let estado = EstadoFlujo {
            archivo: Some(archivo),
            ruta: ruta.to_path_buf(),
            modo,
        };
        self.guardar(id, Arc::new(Mutex::new(estado)));
        Ok(id)
    }

    fn guardar(&self, id: i64, manija: Manija) {
        if let Ok(mut abiertos) = self.abiertos.lock() {
            abiertos.insert(id, manija);
        }
    }

    fn buscar(&self, id: i64) -> Option<Manija> {
        self.abiertos
            .lock()
            .ok()
            .and_then(|abiertos| abiertos.get(&id).cloned())
    }

    fn quitar(&self, id: i64) -> Option<Manija> {
        self.abiertos
            .lock()
            .ok()
            .and_then(|mut abiertos| abiertos.remove(&id))
    }
}

// ----- Instancias -----

fn instancia(id: i64, ruta: &Path, modo: Modo) -> Valor {
    let mut datos = IndexMap::new();
    datos.insert("id".to_string(), Valor::Entero(id));
    datos.insert("ruta".to_string(), Valor::texto(ruta.to_string_lossy()));
    datos.insert("modo".to_string(), Valor::texto(modo.nombre()));
    datos.insert("texto".to_string(), Valor::texto(descripcion(ruta, modo)));
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO),
        datos: std::cell::RefCell::new(datos),
    }))
}

/// Misma instancia como carga del bucle de eventos (apertura asincrónica).
fn carga(id: i64, ruta: &Path, modo: Modo) -> CargaNativa {
    CargaNativa::Instancia {
        tipo: TIPO.to_string(),
        campos: vec![
            ("id".to_string(), CargaNativa::Entero(id)),
            (
                "ruta".to_string(),
                CargaNativa::Texto(ruta.to_string_lossy().to_string()),
            ),
            (
                "modo".to_string(),
                CargaNativa::Texto(modo.nombre().to_string()),
            ),
            (
                "texto".to_string(),
                CargaNativa::Texto(descripcion(ruta, modo)),
            ),
        ],
    }
}

fn descripcion(ruta: &Path, modo: Modo) -> String {
    format!("<Flujo {} ({})>", ruta.display(), modo.nombre())
}

/// Identificador guardado en el receptor del método.
fn id_del_receptor(funcion: &str, argumentos: &[Valor]) -> Result<i64, Fallo> {
    match argumentos.first() {
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == TIPO => {
            match instancia.datos.borrow().get("id") {
                Some(Valor::Entero(id)) => Ok(*id),
                _ => Err(error(
                    "E0406",
                    format!("'{funcion}' recibió un Flujo sin identificador válido"),
                )),
            }
        }
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba un Flujo, pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
        None => Err(error("E0210", format!("'{funcion}' necesita un receptor"))),
    }
}

/// Manija del receptor, exigiendo que el flujo siga abierto y que su modo
/// permita la operación.
fn manija_del_receptor(
    registro: &RegistroFlujos,
    funcion: &str,
    argumentos: &[Valor],
    necesita: Necesidad,
) -> Result<Manija, Fallo> {
    let id = id_del_receptor(funcion, argumentos)?;
    let manija = registro.buscar(id).ok_or_else(|| {
        error_io(format!(
            "'{funcion}' no puede operar sobre un flujo cerrado; abre uno nuevo con \
             'SistemaArchivos.abrir_flujo'"
        ))
    })?;

    let modo = {
        let estado = manija.lock().map_err(|_| {
            error_io(format!(
                "'{funcion}' encontró el flujo en un estado inválido"
            ))
        })?;
        estado.modo
    };
    match necesita {
        Necesidad::Lectura if !modo.permite_lectura() => Err(error_io(format!(
            "'{funcion}' necesita un flujo que lea, pero se abrió en modo '{}'",
            modo.nombre()
        ))),
        Necesidad::Escritura if !modo.permite_escritura() => Err(error_io(format!(
            "'{funcion}' necesita un flujo que escriba, pero se abrió en modo '{}'",
            modo.nombre()
        ))),
        _ => Ok(manija),
    }
}

/// Qué debe permitir el modo del flujo para ejecutar la operación.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Necesidad {
    Lectura,
    Escritura,
    Cursor,
}

// ----- Operaciones sobre el archivo abierto -----

/// Ejecuta una operación con el archivo bloqueado.
fn con_archivo<T>(
    manija: &Manija,
    funcion: &str,
    operacion: impl FnOnce(&mut File) -> Result<T, String>,
) -> Result<T, String> {
    let mut estado = manija
        .lock()
        .map_err(|_| format!("'{funcion}' encontró el flujo en un estado inválido"))?;
    let ruta = estado.ruta.clone();
    let archivo = estado.archivo(funcion)?;
    operacion(archivo).map_err(|fallo| {
        if fallo.starts_with('\'') {
            fallo
        } else {
            format!("'{funcion}' falló sobre '{}': {fallo}", ruta.display())
        }
    })
}

fn op_posicion(manija: Manija) -> Salida {
    con_archivo(&manija, "posicion", |archivo| {
        archivo
            .stream_position()
            .map(|posicion| CargaNativa::Entero(posicion as i64))
            .map_err(|fallo| fallo.to_string())
    })
}

fn op_longitud(manija: Manija) -> Salida {
    con_archivo(&manija, "longitud", |archivo| {
        archivo
            .metadata()
            .map(|datos| CargaNativa::Entero(datos.len() as i64))
            .map_err(|fallo| fallo.to_string())
    })
}

fn op_ir_a(manija: Manija, destino: i64) -> Salida {
    if destino < 0 {
        return Err(format!(
            "'ir_a' recibió la posición {destino}; el cursor no puede ser negativo"
        ));
    }
    mover(manija, "ir_a", SeekFrom::Start(destino as u64))
}

fn op_ir_al_inicio(manija: Manija) -> Salida {
    mover(manija, "ir_al_inicio", SeekFrom::Start(0))
}

fn op_ir_al_final(manija: Manija) -> Salida {
    mover(manija, "ir_al_final", SeekFrom::End(0))
}

fn op_avanzar(manija: Manija, bytes: i64) -> Salida {
    if bytes < 0 {
        return Err(format!(
            "'avanzar' recibió {bytes}; usa 'retroceder' para ir hacia atrás"
        ));
    }
    mover(manija, "avanzar", SeekFrom::Current(bytes))
}

fn op_retroceder(manija: Manija, bytes: i64) -> Salida {
    if bytes < 0 {
        return Err(format!(
            "'retroceder' recibió {bytes}; usa 'avanzar' para ir hacia adelante"
        ));
    }
    let actual = match op_posicion(Arc::clone(&manija))? {
        CargaNativa::Entero(posicion) => posicion,
        _ => 0,
    };
    if bytes > actual {
        return Err(format!(
            "'retroceder' no puede retroceder {bytes} bytes desde la posición {actual}"
        ));
    }
    mover(
        manija,
        "retroceder",
        SeekFrom::Start((actual - bytes) as u64),
    )
}

fn mover(manija: Manija, funcion: &str, destino: SeekFrom) -> Salida {
    con_archivo(&manija, funcion, |archivo| {
        archivo
            .seek(destino)
            .map(|posicion| CargaNativa::Entero(posicion as i64))
            .map_err(|fallo| fallo.to_string())
    })
}

fn op_leer(manija: Manija, cantidad: i64) -> Salida {
    let bytes = leer_bytes(&manija, "leer", cantidad)?;
    String::from_utf8(bytes)
        .map(CargaNativa::Texto)
        .map_err(|_| {
            "'leer' obtuvo bytes que no son texto UTF-8 válido (¿se cortó un carácter?); usa \
             'leer_bits' para datos binarios"
                .to_string()
        })
}

fn op_leer_bits(manija: Manija, cantidad: i64) -> Salida {
    leer_bytes(&manija, "leer_bits", cantidad).map(|bytes| bits::carga(&bytes))
}

fn op_leer_todo(manija: Manija) -> Salida {
    let bytes = con_archivo(&manija, "leer_todo", |archivo| {
        let mut bytes = Vec::new();
        archivo
            .read_to_end(&mut bytes)
            .map(|_| bytes)
            .map_err(|fallo| fallo.to_string())
    })?;
    String::from_utf8(bytes)
        .map(CargaNativa::Texto)
        .map_err(|_| {
            "'leer_todo' obtuvo bytes que no son texto UTF-8 válido; usa 'leer_bits' para datos \
             binarios"
                .to_string()
        })
}

/// Lee hasta `cantidad` bytes desde el cursor (menos si llega al final).
fn leer_bytes(manija: &Manija, funcion: &str, cantidad: i64) -> Result<Vec<u8>, String> {
    if cantidad < 0 {
        return Err(format!(
            "'{funcion}' recibió {cantidad}; la cantidad de bytes no puede ser negativa"
        ));
    }
    con_archivo(manija, funcion, |archivo| {
        let mut buffer = vec![0u8; cantidad as usize];
        let mut leidos = 0usize;
        while leidos < buffer.len() {
            match archivo.read(&mut buffer[leidos..]) {
                Ok(0) => break,
                Ok(cantidad) => leidos += cantidad,
                Err(fallo) => return Err(fallo.to_string()),
            }
        }
        buffer.truncate(leidos);
        Ok(buffer)
    })
}

fn op_escribir(manija: Manija, contenido: String) -> Salida {
    escribir_bytes(&manija, "escribir", contenido.into_bytes())
}

fn op_escribir_bits(manija: Manija, bytes: Vec<u8>) -> Salida {
    escribir_bytes(&manija, "escribir_bits", bytes)
}

fn escribir_bytes(manija: &Manija, funcion: &str, bytes: Vec<u8>) -> Salida {
    con_archivo(manija, funcion, |archivo| {
        archivo
            .write_all(&bytes)
            .map(|_| CargaNativa::Entero(bytes.len() as i64))
            .map_err(|fallo| fallo.to_string())
    })
}

// ----- Registro de las funciones nativas -----

/// `SistemaArchivos.abrir_flujo(ruta, modo)` en sus formas síncrona y
/// asíncrona. Lo llama el módulo `sistema_archivos` al registrarse.
pub fn registrar_apertura(registro: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    registro.registrar_modulo("flujo");

    // Un solo registro compartido por todas las funciones del tipo: las
    // instancias solo guardan el identificador que lo consulta.
    let flujos = RegistroFlujos::default();

    let guardian_sincrono = Rc::clone(guardian);
    let flujos_sincronos = flujos.clone();
    registro.registrar_funcion(
        "sistema_archivos.abrir_flujo",
        Box::new(move |argumentos| {
            let (ruta, modo) = apertura(&guardian_sincrono, argumentos)?;
            let id = flujos_sincronos
                .abrir("SistemaArchivos.abrir_flujo", &ruta, modo)
                .map_err(error_io)?;
            Ok(instancia(id, &ruta, modo))
        }),
    );

    let guardian_asincrono = Rc::clone(guardian);
    let flujos_asincronos = flujos.clone();
    registro.registrar_funcion_con_vm(
        "sistema_archivos.abrir_flujo_asincrono",
        Box::new(move |vm, argumentos| {
            let (ruta, modo) = apertura(&guardian_asincrono, argumentos)?;
            let flujos = flujos_asincronos.clone();
            tarea(vm, move || {
                let id = flujos.abrir("SistemaArchivos.abrir_flujo_asincrono", &ruta, modo)?;
                Ok(carga(id, &ruta, modo))
            })
        }),
    );

    registrar_metodos(registro, &flujos);
}

/// Ruta verificada y modo validado de `abrir_flujo`.
fn apertura(guardian: &GuardianPermisos, argumentos: &[Valor]) -> Result<(PathBuf, Modo), Fallo> {
    const F: &str = "SistemaArchivos.abrir_flujo";
    exigir_aridad(F, argumentos, 2)?;
    let ruta = arg_texto(F, argumentos, 0)?;
    let modo = Modo::desde_texto(F, arg_texto(F, argumentos, 1)?)?;

    // Cada modo exige su nivel: `lectura_escritura` necesita ambos.
    let mut resuelta = None;
    if modo.permite_lectura() {
        resuelta = Some(guardian.verificar_lectura(ruta).map_err(error_permiso)?);
    }
    if modo.permite_escritura() {
        resuelta = Some(guardian.verificar_escritura(ruta).map_err(error_permiso)?);
    }
    Ok((resuelta.unwrap_or_else(|| PathBuf::from(ruta)), modo))
}

fn registrar_metodos(registro: &mut RegistroNativos, flujos: &RegistroFlujos) {
    // Cursor (operaciones baratas: solo forma síncrona).
    registrar_cursor(registro, flujos, "posicion", op_posicion);
    registrar_cursor(registro, flujos, "longitud", op_longitud);
    registrar_cursor(registro, flujos, "ir_al_inicio", op_ir_al_inicio);
    registrar_cursor(registro, flujos, "ir_al_final", op_ir_al_final);
    registrar_cursor_con_bytes(registro, flujos, "ir_a", op_ir_a);
    registrar_cursor_con_bytes(registro, flujos, "avanzar", op_avanzar);
    registrar_cursor_con_bytes(registro, flujos, "retroceder", op_retroceder);

    // Entrada/salida (síncrona y asíncrona).
    registrar_lectura_con_cantidad(registro, flujos, "leer", op_leer);
    registrar_lectura_con_cantidad(registro, flujos, "leer_bits", op_leer_bits);
    registrar_lectura_total(registro, flujos);
    registrar_escritura_texto(registro, flujos);
    registrar_escritura_bits(registro, flujos);

    // Ciclo de vida y metadatos de la instancia.
    registrar_cierre(registro, flujos);
    registrar_dato(registro, "ruta");
    registrar_dato(registro, "modo");
}

fn registrar_cursor(
    registro: &mut RegistroNativos,
    flujos: &RegistroFlujos,
    nombre: &'static str,
    operacion: fn(Manija) -> Salida,
) {
    let flujos = flujos.clone();
    let funcion = format!("{TIPO}.{nombre}");
    registro.registrar_funcion(
        &format!("{TIPO}.{nombre}"),
        Box::new(move |argumentos| {
            exigir_aridad(&funcion, &argumentos[1..], 0)?;
            let manija = manija_del_receptor(&flujos, &funcion, argumentos, Necesidad::Cursor)?;
            ejecutar(operacion(manija))
        }),
    );
}

fn registrar_cursor_con_bytes(
    registro: &mut RegistroNativos,
    flujos: &RegistroFlujos,
    nombre: &'static str,
    operacion: fn(Manija, i64) -> Salida,
) {
    let flujos = flujos.clone();
    let funcion = format!("{TIPO}.{nombre}");
    registro.registrar_funcion(
        &format!("{TIPO}.{nombre}"),
        Box::new(move |argumentos| {
            exigir_aridad(&funcion, &argumentos[1..], 1)?;
            let manija = manija_del_receptor(&flujos, &funcion, argumentos, Necesidad::Cursor)?;
            let bytes = arg_entero(&funcion, argumentos, 1)?;
            ejecutar(operacion(manija, bytes))
        }),
    );
}

fn registrar_lectura_con_cantidad(
    registro: &mut RegistroNativos,
    flujos: &RegistroFlujos,
    nombre: &'static str,
    operacion: fn(Manija, i64) -> Salida,
) {
    let funcion = format!("{TIPO}.{nombre}");

    let flujos_sincronos = flujos.clone();
    let etiqueta = funcion.clone();
    registro.registrar_funcion(
        &format!("{TIPO}.{nombre}"),
        Box::new(move |argumentos| {
            exigir_aridad(&etiqueta, &argumentos[1..], 1)?;
            let manija =
                manija_del_receptor(&flujos_sincronos, &etiqueta, argumentos, Necesidad::Lectura)?;
            let cantidad = arg_entero(&etiqueta, argumentos, 1)?;
            ejecutar(operacion(manija, cantidad))
        }),
    );

    let flujos_asincronos = flujos.clone();
    registro.registrar_funcion_con_vm(
        &format!("{TIPO}.{nombre}_asincrono"),
        Box::new(move |vm, argumentos| {
            exigir_aridad(&funcion, &argumentos[1..], 1)?;
            let manija =
                manija_del_receptor(&flujos_asincronos, &funcion, argumentos, Necesidad::Lectura)?;
            let cantidad = arg_entero(&funcion, argumentos, 1)?;
            tarea(vm, move || operacion(manija, cantidad))
        }),
    );
}

fn registrar_lectura_total(registro: &mut RegistroNativos, flujos: &RegistroFlujos) {
    const F: &str = "Flujo.leer_todo";

    let flujos_sincronos = flujos.clone();
    registro.registrar_funcion(
        "Flujo.leer_todo",
        Box::new(move |argumentos| {
            exigir_aridad(F, &argumentos[1..], 0)?;
            let manija = manija_del_receptor(&flujos_sincronos, F, argumentos, Necesidad::Lectura)?;
            ejecutar(op_leer_todo(manija))
        }),
    );

    let flujos_asincronos = flujos.clone();
    registro.registrar_funcion_con_vm(
        "Flujo.leer_todo_asincrono",
        Box::new(move |vm, argumentos| {
            exigir_aridad(F, &argumentos[1..], 0)?;
            let manija =
                manija_del_receptor(&flujos_asincronos, F, argumentos, Necesidad::Lectura)?;
            tarea(vm, move || op_leer_todo(manija))
        }),
    );
}

fn registrar_escritura_texto(registro: &mut RegistroNativos, flujos: &RegistroFlujos) {
    const F: &str = "Flujo.escribir";

    let flujos_sincronos = flujos.clone();
    registro.registrar_funcion(
        "Flujo.escribir",
        Box::new(move |argumentos| {
            exigir_aridad(F, &argumentos[1..], 1)?;
            let manija =
                manija_del_receptor(&flujos_sincronos, F, argumentos, Necesidad::Escritura)?;
            let contenido = arg_texto(F, argumentos, 1)?.to_string();
            ejecutar(op_escribir(manija, contenido))
        }),
    );

    let flujos_asincronos = flujos.clone();
    registro.registrar_funcion_con_vm(
        "Flujo.escribir_asincrono",
        Box::new(move |vm, argumentos| {
            exigir_aridad(F, &argumentos[1..], 1)?;
            let manija =
                manija_del_receptor(&flujos_asincronos, F, argumentos, Necesidad::Escritura)?;
            let contenido = arg_texto(F, argumentos, 1)?.to_string();
            tarea(vm, move || op_escribir(manija, contenido))
        }),
    );
}

fn registrar_escritura_bits(registro: &mut RegistroNativos, flujos: &RegistroFlujos) {
    const F: &str = "Flujo.escribir_bits";

    let flujos_sincronos = flujos.clone();
    registro.registrar_funcion(
        "Flujo.escribir_bits",
        Box::new(move |argumentos| {
            exigir_aridad(F, &argumentos[1..], 1)?;
            let manija =
                manija_del_receptor(&flujos_sincronos, F, argumentos, Necesidad::Escritura)?;
            let bytes = bits::arg_bits(F, argumentos, 1)?;
            ejecutar(op_escribir_bits(manija, bytes))
        }),
    );

    let flujos_asincronos = flujos.clone();
    registro.registrar_funcion_con_vm(
        "Flujo.escribir_bits_asincrono",
        Box::new(move |vm, argumentos| {
            exigir_aridad(F, &argumentos[1..], 1)?;
            let manija =
                manija_del_receptor(&flujos_asincronos, F, argumentos, Necesidad::Escritura)?;
            let bytes = bits::arg_bits(F, argumentos, 1)?;
            tarea(vm, move || op_escribir_bits(manija, bytes))
        }),
    );
}

fn registrar_cierre(registro: &mut RegistroNativos, flujos: &RegistroFlujos) {
    let flujos_cerrar = flujos.clone();
    registro.registrar_funcion(
        "Flujo.cerrar",
        Box::new(move |argumentos| {
            const F: &str = "Flujo.cerrar";
            exigir_aridad(F, &argumentos[1..], 0)?;
            // Cerrar dos veces no es un error: el recurso ya está libre.
            if let Some(manija) = flujos_cerrar.quitar(id_del_receptor(F, argumentos)?)
                && let Ok(mut estado) = manija.lock()
            {
                estado.archivo = None;
            }
            Ok(Valor::Nulo)
        }),
    );

    let flujos_estado = flujos.clone();
    registro.registrar_funcion(
        "Flujo.esta_cerrado",
        Box::new(move |argumentos| {
            const F: &str = "Flujo.esta_cerrado";
            exigir_aridad(F, &argumentos[1..], 0)?;
            let abierto = flujos_estado
                .buscar(id_del_receptor(F, argumentos)?)
                .is_some();
            Ok(Valor::Log(!abierto))
        }),
    );
}

/// Dato guardado en la instancia (`ruta()`, `modo()`): no toca el disco.
fn registrar_dato(registro: &mut RegistroNativos, nombre: &'static str) {
    let funcion = format!("{TIPO}.{nombre}");
    registro.registrar_funcion(
        &format!("{TIPO}.{nombre}"),
        Box::new(move |argumentos| {
            exigir_aridad(&funcion, &argumentos[1..], 0)?;
            match argumentos.first() {
                Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == TIPO => {
                    Ok(instancia
                        .datos
                        .borrow()
                        .get(nombre)
                        .cloned()
                        .unwrap_or(Valor::Nulo))
                }
                _ => Err(error(
                    "E0406",
                    format!("'{funcion}' esperaba un Flujo como receptor"),
                )),
            }
        }),
    );
}
