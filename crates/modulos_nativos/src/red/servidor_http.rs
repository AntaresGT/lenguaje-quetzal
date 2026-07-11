//! Objeto `ServidorHttp`: servidor HTTP con rutas en español y manejadores
//! de Quetzal pasados por referencia (nunca copiados: los valores `Valor`
//! de tipo función son `Rc`, así que registrarlos como manejador es un
//! `Rc::clone` barato, no una copia de la función).
//!
//! Arquitectura reactor (igual que el resto de `quetzal/red`): cada
//! `ServidorHttp.escuchar(puerto)` abre un `TcpListener` y acepta conexiones
//! en hilos del sistema operativo (no en el runtime de tokio: la conexión se
//! atiende de forma síncrona, bloqueando solo ese hilo dedicado). Cada
//! petición se envía como [`Mensaje::Solicitud`] al hilo de la VM, que la
//! despacha a la ruta que corresponda, invoca el manejador de Quetzal
//! (`Vm::llamar_funcion`, con acceso completo al bucle de eventos: el
//! manejador puede usar `esperar` normalmente) y devuelve la respuesta por
//! el canal de vuelta para que el hilo de la conexión la escriba en el
//! socket. Así ninguna petición bloquea el hilo de la VM ni al resto del
//! programa.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use indexmap::IndexMap;
use maquina_virtual::bucle_eventos::CargaNativa;
use maquina_virtual::{DatosInstanciaNativa, Fallo, ManijaBucle, Mensaje, RegistroNativos, Valor, Vm};
use runtime::GuardianPermisos;

use crate::util::{arg_entero, arg_ruta, arg_texto, error, exigir_aridad};

use super::{bytes_y_tipo_contenido, metodo_http_desde_texto, permiso_denegado};

const TIPO_SERVIDOR: &str = "ServidorHttp";
const TIPO_PETICION: &str = "PeticionHttp";
const TIPO_RESPUESTA_SERVIDOR: &str = "RespuestaServidor";
const TIPO_PETICION_CRUDA: &str = "_peticion_cruda";
const TIPO_RESPUESTA_CRUDA: &str = "_respuesta_cruda";
/// Respuesta de [`Respuestas::archivo`] o de una ruta estática: en vez de un
/// cuerpo ya armado en memoria, lleva la ruta en disco y los metadatos de
/// caché; el hilo de la conexión transmite el archivo directamente al
/// socket por trozos (nunca lo carga completo en RAM), y resuelve `Range`
/// (`206`), condicionales de caché (`304`) y rangos inválidos (`416`).
const TIPO_RESPUESTA_ARCHIVO: &str = "_respuesta_archivo";
/// Respuesta de [`Respuestas::flujo`]: el cuerpo se produce en trozos (uno
/// por llamada a la función generadora de Quetzal) y se envía con
/// `Transfer-Encoding: chunked`, sin conocer el tamaño total de antemano
/// (útil para *server-sent events* o cualquier flujo en vivo).
const TIPO_RESPUESTA_FLUJO: &str = "_respuesta_flujo";
/// Tamaño de cada trozo al transmitir un archivo del disco al socket: ni el
/// archivo completo ni cada respuesta de `Respuestas.archivo` viven enteros
/// en memoria, sin importar cuán grande sea el archivo (video, etc.).
const TAMANO_TROZO_ARCHIVO: usize = 64 * 1024;

/// Nombre del servicio del despachador (`Vm::registrar_despachador`),
/// compartido por todas las instancias de `ServidorHttp`: cada una se
/// distingue por su `id_recurso`.
const SERVICIO: &str = "servidor_http";

/// Registro compartido: cada `ServidorHttp` en escucha se identifica por el
/// `id_recurso` que le asignó el bucle de eventos. El despachador único del
/// servicio lo usa para encontrar la instancia (y su tabla de rutas)
/// correspondiente a cada petición entrante.
type RegistroServidores = Rc<RefCell<HashMap<u64, Rc<DatosInstanciaNativa>>>>;

/// Bandera para detener limpiamente el hilo de aceptación de conexiones de
/// cada servidor (`ServidorHttp.detener()`).
type RegistroBanderas = Rc<RefCell<HashMap<u64, Arc<AtomicBool>>>>;

/// Generadores de `Respuestas.flujo(...)` pendientes, por `id_flujo`. Vive
/// exclusivamente en el hilo de la VM (la función de Quetzal es `Rc`, no es
/// `Send`): el hilo de la conexión solo conoce el `id_flujo` y pide "el
/// siguiente trozo" mandando un evento al despachador, igual que una
/// petición HTTP normal. Se limpia cuando el generador devuelve `nulo` o
/// cuando la conexión se cierra (evento `cerrar_flujo`), para no filtrar
/// memoria si el cliente se desconecta a mitad de un flujo.
type RegistroFlujos = Rc<RefCell<HashMap<u64, Valor>>>;

fn error_servidor(mensaje: impl Into<String>) -> Fallo {
    error("E0704", mensaje.into())
}

// =====================================================================
// ServidorHttp: instancia y registro de rutas
// =====================================================================

fn nuevo_servidor() -> Valor {
    let mut datos = IndexMap::new();
    datos.insert("rutas".to_string(), Valor::lista(Vec::new()));
    datos.insert("estaticos".to_string(), Valor::lista(Vec::new()));
    datos.insert("activo".to_string(), Valor::Log(false));
    datos.insert("puerto".to_string(), Valor::Nulo);
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_SERVIDOR),
        datos: RefCell::new(datos),
    }))
}

fn constructor(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    exigir_aridad("nuevo ServidorHttp", argumentos, 0)?;
    Ok(nuevo_servidor())
}

fn receptor_servidor(funcion: &str, argumentos: &[Valor]) -> Result<Rc<DatosInstanciaNativa>, Fallo> {
    match argumentos.first() {
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == TIPO_SERVIDOR => {
            Ok(Rc::clone(instancia))
        }
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba un ServidorHttp, pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
        None => Err(error("E0210", format!("'{funcion}' necesita el receptor"))),
    }
}

/// Un manejador debe pasarse por referencia (una función de Quetzal ya
/// definida): nunca se copia, solo se clona el `Rc` interno del `Valor`.
fn exigir_manejador(funcion: &str, valor: &Valor) -> Result<Valor, Fallo> {
    match valor {
        Valor::Funcion(..) => Ok(valor.clone()),
        otro => Err(error(
            "E0406",
            format!(
                "'{funcion}' espera una función definida como manejador, pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
    }
}

fn agregar_ruta(
    instancia: &DatosInstanciaNativa,
    metodo: &reqwest::Method,
    patron: &str,
    manejador: Valor,
) {
    let mut entrada = IndexMap::new();
    entrada.insert("metodo".to_string(), Valor::texto(metodo.as_str()));
    entrada.insert("patron".to_string(), Valor::texto(patron));
    entrada.insert("manejador".to_string(), manejador);
    if let Some(Valor::Lista(rutas)) = instancia.datos.borrow().get("rutas") {
        rutas.borrow_mut().push(Valor::jsn(entrada));
    }
}

fn metodo_ruta_fija(
    funcion: &'static str,
    metodo_http: reqwest::Method,
) -> maquina_virtual::FuncionNativa {
    Box::new(move |argumentos| {
        exigir_aridad(funcion, &argumentos[1..], 2)?;
        let instancia = receptor_servidor(funcion, argumentos)?;
        let patron = arg_texto(funcion, argumentos, 1)?.to_string();
        let manejador = exigir_manejador(funcion, &argumentos[2])?;
        agregar_ruta(&instancia, &metodo_http, &patron, manejador);
        Ok(Valor::Nulo)
    })
}

fn metodo_ruta_generica(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ServidorHttp.ruta";
    exigir_aridad(F, &argumentos[1..], 3)?;
    let instancia = receptor_servidor(F, argumentos)?;
    let metodo_texto = arg_texto(F, argumentos, 1)?;
    let metodo_http = metodo_http_desde_texto(F, metodo_texto)?;
    let patron = arg_texto(F, argumentos, 2)?.to_string();
    let manejador = exigir_manejador(F, &argumentos[3])?;
    agregar_ruta(&instancia, &metodo_http, &patron, manejador);
    Ok(Valor::Nulo)
}

/// Registra una carpeta estática: toda petición `GET` cuya ruta empiece con
/// `prefijo` que no coincida con ninguna ruta explícita se resuelve como el
/// archivo `directorio/resto` (protegido contra *path traversal* por el
/// propio [`GuardianPermisos`], que canonicaliza la ruta antes de servirla).
fn agregar_estatico(instancia: &DatosInstanciaNativa, prefijo: &str, directorio: &str) {
    let mut entrada = IndexMap::new();
    entrada.insert("prefijo".to_string(), Valor::texto(prefijo));
    entrada.insert("directorio".to_string(), Valor::texto(directorio));
    if let Some(Valor::Lista(estaticos)) = instancia.datos.borrow().get("estaticos") {
        estaticos.borrow_mut().push(Valor::jsn(entrada));
    }
}

fn metodo_estaticos(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ServidorHttp.estaticos";
    exigir_aridad(F, &argumentos[1..], 2)?;
    let instancia = receptor_servidor(F, argumentos)?;
    let prefijo = arg_texto(F, argumentos, 1)?.trim_matches('/').to_string();
    let directorio = arg_texto(F, argumentos, 2)?.trim_end_matches('/').to_string();
    agregar_estatico(&instancia, &prefijo, &directorio);
    Ok(Valor::Nulo)
}

fn metodo_puerto(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ServidorHttp.puerto";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let instancia = receptor_servidor(F, argumentos)?;
    Ok(instancia.datos.borrow().get("puerto").cloned().unwrap_or(Valor::Nulo))
}

fn metodo_esta_escuchando(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ServidorHttp.esta_escuchando";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let instancia = receptor_servidor(F, argumentos)?;
    let activo = matches!(instancia.datos.borrow().get("activo"), Some(Valor::Log(true)));
    Ok(Valor::Log(activo))
}

// =====================================================================
// ServidorHttp.escuchar(puerto) / .detener()
// =====================================================================

fn metodo_escuchar(
    guardian: &Rc<GuardianPermisos>,
    registro_servidores: &RegistroServidores,
    banderas: &RegistroBanderas,
    registro_flujos: &RegistroFlujos,
) -> maquina_virtual::FuncionNativaConVm {
    const F: &str = "ServidorHttp.escuchar";
    let guardian = Rc::clone(guardian);
    let registro_servidores = Rc::clone(registro_servidores);
    let banderas = Rc::clone(banderas);
    let registro_flujos = Rc::clone(registro_flujos);
    Box::new(move |vm, argumentos| {
        exigir_aridad(F, &argumentos[1..], 1)?;
        let instancia = receptor_servidor(F, argumentos)?;
        let puerto_solicitado = arg_entero(F, argumentos, 1)?;
        if !(0..=65535).contains(&puerto_solicitado) {
            return Err(error(
                "E0406",
                format!("'{F}' espera un puerto entre 0 y 65535, pero recibió {puerto_solicitado}"),
            ));
        }
        let puerto = puerto_solicitado as u16;
        guardian
            .verificar_red_servidor(puerto)
            .map_err(permiso_denegado)?;

        let ya_activo = matches!(instancia.datos.borrow().get("activo"), Some(Valor::Log(true)));
        if ya_activo {
            return Err(error_servidor(format!("'{F}': el servidor ya está escuchando")));
        }

        let escucha = TcpListener::bind(("0.0.0.0", puerto))
            .map_err(|causa| error_servidor(format!("no se pudo escuchar en el puerto {puerto}: {causa}")))?;
        escucha
            .set_nonblocking(true)
            .map_err(|causa| error_servidor(format!("no se pudo configurar el servidor: {causa}")))?;
        let puerto_real = escucha
            .local_addr()
            .map_err(|causa| error_servidor(format!("no se pudo leer el puerto del servidor: {causa}")))?
            .port();

        let id = vm.bucle().nuevo_id();
        registro_servidores
            .borrow_mut()
            .insert(id, Rc::clone(&instancia));
        let bandera = Arc::new(AtomicBool::new(true));
        banderas.borrow_mut().insert(id, Arc::clone(&bandera));

        {
            let mut datos = instancia.datos.borrow_mut();
            datos.insert("activo".to_string(), Valor::Log(true));
            datos.insert("puerto".to_string(), Valor::Entero(i64::from(puerto_real)));
            datos.insert("id_interno".to_string(), Valor::Entero(id as i64));
        }

        vm.registrar_despachador(SERVICIO, {
            let registro_servidores = Rc::clone(&registro_servidores);
            let registro_flujos = Rc::clone(&registro_flujos);
            let guardian = Rc::clone(&guardian);
            move |vm, id_recurso, datos| {
                despachar_peticion(vm, id_recurso, &registro_servidores, &registro_flujos, &guardian, datos)
            }
        });
        vm.bucle().registrar_trabajo_activo();

        let manija = vm.bucle().manija();
        thread::spawn(move || bucle_aceptacion(escucha, id, manija, bandera));

        Ok(Valor::Nulo)
    })
}

fn metodo_detener(banderas: &RegistroBanderas, registro_servidores: &RegistroServidores) -> maquina_virtual::FuncionNativaConVm {
    const F: &str = "ServidorHttp.detener";
    let banderas = Rc::clone(banderas);
    let registro_servidores = Rc::clone(registro_servidores);
    Box::new(move |vm, argumentos| {
        exigir_aridad(F, &argumentos[1..], 0)?;
        let instancia = receptor_servidor(F, argumentos)?;
        let id = match instancia.datos.borrow().get("id_interno") {
            Some(Valor::Entero(id)) => Some(*id as u64),
            _ => None,
        };
        let activo = matches!(instancia.datos.borrow().get("activo"), Some(Valor::Log(true)));
        if !activo {
            return Ok(Valor::Nulo);
        }
        instancia
            .datos
            .borrow_mut()
            .insert("activo".to_string(), Valor::Log(false));
        if let Some(id) = id {
            if let Some(bandera) = banderas.borrow_mut().remove(&id) {
                bandera.store(false, Ordering::Relaxed);
            }
            registro_servidores.borrow_mut().remove(&id);
        }
        vm.bucle().liberar_trabajo_activo();
        Ok(Valor::Nulo)
    })
}

/// Hilo de aceptación de conexiones: se detiene en cuanto `bandera` pasa a
/// `false` (`ServidorHttp.detener()`). Cada conexión se atiende en su propio
/// hilo para no bloquear la aceptación de las siguientes.
fn bucle_aceptacion(escucha: TcpListener, id_recurso: u64, manija: ManijaBucle, bandera: Arc<AtomicBool>) {
    while bandera.load(Ordering::Relaxed) {
        match escucha.accept() {
            Ok((flujo, _)) => {
                let manija = manija.clone();
                thread::spawn(move || manejar_conexion(flujo, id_recurso, manija));
            }
            Err(fallo) if fallo.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(_) => break,
        }
    }
}

// =====================================================================
// Conexión: lectura cruda de la petición y escritura de la respuesta
// =====================================================================

struct PeticionCruda {
    metodo: String,
    ruta: String,
    consulta: Vec<(String, String)>,
    cabeceras: Vec<(String, String)>,
    cuerpo: Vec<u8>,
}

fn leer_linea(lector: &mut impl BufRead) -> String {
    let mut linea = String::new();
    let _ = lector.read_line(&mut linea);
    linea.trim_end_matches(['\r', '\n']).to_string()
}

fn decodificar_percentual(texto: &str) -> String {
    percent_encoding::percent_decode_str(&texto.replace('+', " "))
        .decode_utf8_lossy()
        .into_owned()
}

fn analizar_consulta(consulta: &str) -> Vec<(String, String)> {
    consulta
        .split('&')
        .filter(|par| !par.is_empty())
        .map(|par| match par.split_once('=') {
            Some((clave, valor)) => (decodificar_percentual(clave), decodificar_percentual(valor)),
            None => (decodificar_percentual(par), String::new()),
        })
        .collect()
}

fn leer_peticion_cruda(flujo: &TcpStream) -> Option<PeticionCruda> {
    let mut lector = BufReader::new(flujo.try_clone().ok()?);
    let primera_linea = leer_linea(&mut lector);
    if primera_linea.is_empty() {
        return None;
    }
    let mut partes = primera_linea.split_whitespace();
    let metodo = partes.next()?.to_string();
    let objetivo = partes.next().unwrap_or("/");
    let (ruta, consulta) = match objetivo.split_once('?') {
        Some((ruta, consulta)) => (ruta.to_string(), analizar_consulta(consulta)),
        None => (objetivo.to_string(), Vec::new()),
    };

    let mut cabeceras = Vec::new();
    loop {
        let linea = leer_linea(&mut lector);
        if linea.is_empty() {
            break;
        }
        if let Some((nombre, valor)) = linea.split_once(':') {
            cabeceras.push((nombre.trim().to_string(), valor.trim().to_string()));
        }
    }

    let longitud: usize = cabeceras
        .iter()
        .find(|(nombre, _)| nombre.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, valor)| valor.parse().ok())
        .unwrap_or(0);
    let mut cuerpo = vec![0u8; longitud];
    if longitud > 0 && lector.read_exact(&mut cuerpo).is_err() {
        return None;
    }

    Some(PeticionCruda {
        metodo,
        ruta,
        consulta,
        cabeceras,
        cuerpo,
    })
}

fn razon_estado(estado: u16) -> &'static str {
    match estado {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        206 => "Partial Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        416 => "Range Not Satisfiable",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        _ if (200..300).contains(&estado) => "OK",
        _ if (400..500).contains(&estado) => "Bad Request",
        _ => "Internal Server Error",
    }
}

fn construir_bytes_http(estado: u16, cabeceras: &[(String, String)], cuerpo: &[u8]) -> Vec<u8> {
    let mut salida = format!("HTTP/1.1 {estado} {}\r\n", razon_estado(estado)).into_bytes();
    for (nombre, valor) in cabeceras {
        salida.extend_from_slice(format!("{nombre}: {valor}\r\n").as_bytes());
    }
    if !cabeceras
        .iter()
        .any(|(nombre, _)| nombre.eq_ignore_ascii_case("content-length"))
    {
        salida.extend_from_slice(format!("Content-Length: {}\r\n", cuerpo.len()).as_bytes());
    }
    salida.extend_from_slice(b"Connection: close\r\n\r\n");
    salida.extend_from_slice(cuerpo);
    salida
}

/// Busca una cabecera por nombre, sin distinguir mayúsculas/minúsculas
/// (`Range`, `range` y `RANGE` son la misma cabecera para HTTP).
fn buscar_cabecera<'a>(cabeceras: &'a [(String, String)], nombre: &str) -> Option<&'a str> {
    cabeceras
        .iter()
        .find(|(clave, _)| clave.eq_ignore_ascii_case(nombre))
        .map(|(_, valor)| valor.as_str())
}

/// Escribe únicamente las cabeceras de la respuesta (línea de estado +
/// cabeceras + línea vacía) directamente en el socket, sin cuerpo. Lo usan
/// las respuestas de archivo/flujo, que transmiten el cuerpo por su cuenta
/// (streaming) en vez de construirlo entero en un `Vec<u8>`.
fn escribir_encabezados(flujo: &mut TcpStream, estado: u16, cabeceras: &[(String, String)]) -> std::io::Result<()> {
    let mut salida = format!("HTTP/1.1 {estado} {}\r\n", razon_estado(estado)).into_bytes();
    for (nombre, valor) in cabeceras {
        salida.extend_from_slice(format!("{nombre}: {valor}\r\n").as_bytes());
    }
    salida.extend_from_slice(b"Connection: close\r\n\r\n");
    flujo.write_all(&salida)
}

/// Copia `longitud` bytes de `archivo` (ya posicionado con `seek`) al
/// socket, en trozos de [`TAMANO_TROZO_ARCHIVO`]: nunca materializa el
/// archivo completo en memoria, sin importar su tamaño.
fn transmitir_archivo(flujo: &mut TcpStream, archivo: &mut File, longitud: u64) -> std::io::Result<()> {
    let mut restantes = longitud;
    let mut buffer = [0u8; TAMANO_TROZO_ARCHIVO];
    while restantes > 0 {
        let a_leer = restantes.min(TAMANO_TROZO_ARCHIVO as u64) as usize;
        let leidos = archivo.read(&mut buffer[..a_leer])?;
        if leidos == 0 {
            break;
        }
        flujo.write_all(&buffer[..leidos])?;
        restantes -= leidos as u64;
    }
    Ok(())
}

/// Tabla de tipos MIME por extensión (streaming de video/audio/imágenes
/// incluido). Sobreescribible con `RespuestaServidor.fijar_cabecera(
/// "Content-Type", ...)` antes de devolver `Respuestas.archivo(...)`.
fn mime_por_extension(ruta: &str) -> String {
    let extension = Path::new(ruta)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match extension.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "txt" => "text/plain; charset=utf-8",
        "csv" => "text/csv; charset=utf-8",
        "xml" => "application/xml",
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
    .to_string()
}

/// Marca de tiempo (milisegundos Unix) formateada como fecha HTTP
/// (`Sun, 06 Nov 1994 08:49:37 GMT`, el formato preferido de RFC 7231) para
/// `Last-Modified`. No depende de una crate nueva: usa `chrono`, que ya es
/// dependencia del módulo `tiempo`.
fn formatear_fecha_http(marca_ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(marca_ms.max(0) / 1000, 0)
        .map(|fecha| fecha.format("%a, %d %b %Y %H:%M:%S GMT").to_string())
        .unwrap_or_else(|| "Thu, 01 Jan 1970 00:00:00 GMT".to_string())
}

/// Interpreta una fecha HTTP (`If-Modified-Since`) como segundos Unix; si
/// el formato no se reconoce, devuelve `None` (la condición se ignora en
/// vez de fallar la petición).
fn analizar_fecha_http(valor: &str) -> Option<i64> {
    chrono::NaiveDateTime::parse_from_str(valor.trim(), "%a, %d %b %Y %H:%M:%S GMT")
        .ok()
        .map(|fecha| fecha.and_utc().timestamp())
}

/// ETag débil: no garantiza igualdad byte a byte, pero cambia si cambia el
/// tamaño o la fecha de modificación (suficiente para caché condicional).
fn calcular_etag(tamano: u64, mtime_ms: i64) -> String {
    format!("W/\"{tamano:x}-{mtime_ms:x}\"")
}

/// `true` si la petición ya tiene una copia válida en caché (`If-None-Match`
/// con el mismo `ETag`, o `If-Modified-Since` no anterior a `mtime_ms`) y
/// debe responderse `304 Not Modified` sin cuerpo.
fn coincide_no_modificado(cabeceras_peticion: &[(String, String)], etag: &str, mtime_ms: i64) -> bool {
    if let Some(valor) = buscar_cabecera(cabeceras_peticion, "If-None-Match") {
        return valor.trim() == "*" || valor.split(',').any(|parte| parte.trim() == etag);
    }
    if let Some(valor) = buscar_cabecera(cabeceras_peticion, "If-Modified-Since")
        && let Some(segundos) = analizar_fecha_http(valor)
    {
        return mtime_ms / 1000 <= segundos;
    }
    false
}

/// Resultado de interpretar la cabecera `Range` contra el tamaño real del
/// archivo.
enum Rango {
    /// Sin `Range`, o con varios rangos (`bytes=0-1,2-3`): no soportado, se
    /// sirve el archivo completo con `200`.
    Ninguno,
    /// Un solo rango válido `inicio..=fin` (ambos inclusive).
    Valido(u64, u64),
    /// `Range` presente pero fuera de los límites del archivo: `416`.
    Invalido,
}

/// Interpreta `Range: bytes=inicio-fin` (también `bytes=inicio-` y
/// `bytes=-sufijo`, las formas abiertas de RFC 7233).
fn analizar_rango(cabecera: Option<&str>, tamano: u64) -> Rango {
    let Some(valor) = cabecera else { return Rango::Ninguno };
    let Some(especificacion) = valor.trim().strip_prefix("bytes=") else {
        return Rango::Ninguno;
    };
    if especificacion.contains(',') || tamano == 0 {
        return Rango::Ninguno;
    }
    let Some((inicio_texto, fin_texto)) = especificacion.split_once('-') else {
        return Rango::Invalido;
    };
    let extremos = match (inicio_texto.trim(), fin_texto.trim()) {
        ("", "") => None,
        ("", sufijo) => sufijo
            .parse::<u64>()
            .ok()
            .map(|n| (tamano.saturating_sub(n.min(tamano)), tamano - 1)),
        (inicio, "") => inicio.parse::<u64>().ok().map(|inicio| (inicio, tamano - 1)),
        (inicio, fin) => match (inicio.parse::<u64>(), fin.parse::<u64>()) {
            (Ok(inicio), Ok(fin)) => Some((inicio, fin.min(tamano - 1))),
            _ => None,
        },
    };
    match extremos {
        Some((inicio, fin)) if inicio <= fin && inicio < tamano => Rango::Valido(inicio, fin),
        _ => Rango::Invalido,
    }
}

/// Cabeceras extra (`Respuestas.crear`/`fijar_cabecera`) desde una
/// [`CargaNativa::Mapa`], como pares de texto listos para escribir.
fn cabeceras_desde_mapa(carga: Option<&CargaNativa>) -> Vec<(String, String)> {
    match carga {
        Some(CargaNativa::Mapa(pares)) => pares
            .iter()
            .filter_map(|(nombre, valor)| match valor {
                CargaNativa::Texto(texto) => Some((nombre.clone(), texto.clone())),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Responde una petición con `Respuestas.archivo(...)` (o una ruta
/// estática): resuelve caché condicional (`304`), `Range` (`206`/`416`) y
/// transmite el cuerpo del disco al socket sin cargarlo completo en
/// memoria. Corre en el hilo de la conexión.
fn responder_archivo(flujo: &mut TcpStream, cabeceras_peticion: &[(String, String)], campos: Vec<(String, CargaNativa)>) {
    let mapa: HashMap<String, CargaNativa> = campos.into_iter().collect();
    let texto_de = |clave: &str| match mapa.get(clave) {
        Some(CargaNativa::Texto(texto)) => texto.clone(),
        _ => String::new(),
    };
    let entero_de = |clave: &str| match mapa.get(clave) {
        Some(CargaNativa::Entero(numero)) => *numero,
        _ => 0,
    };

    let ruta = texto_de("ruta");
    let tamano = entero_de("tamano").max(0) as u64;
    let mtime_ms = entero_de("mtime_ms");
    let etag = texto_de("etag");
    let mime = texto_de("mime");
    let mut cabeceras_extra = cabeceras_desde_mapa(mapa.get("cabeceras"));
    let fecha_modificacion = formatear_fecha_http(mtime_ms);

    if coincide_no_modificado(cabeceras_peticion, &etag, mtime_ms) {
        let mut cabeceras = vec![
            ("ETag".to_string(), etag),
            ("Last-Modified".to_string(), fecha_modificacion),
            ("Accept-Ranges".to_string(), "bytes".to_string()),
        ];
        cabeceras.append(&mut cabeceras_extra);
        let _ = escribir_encabezados(flujo, 304, &cabeceras);
        return;
    }

    let mut archivo = match File::open(&ruta) {
        Ok(archivo) => archivo,
        Err(causa) => {
            let bytes = construir_bytes_http(
                500,
                &[("Content-Type".to_string(), "text/plain; charset=utf-8".to_string())],
                format!("no se pudo transmitir el archivo: {causa}").as_bytes(),
            );
            let _ = flujo.write_all(&bytes);
            return;
        }
    };

    let tipo_contenido = cabeceras_extra
        .iter()
        .find(|(nombre, _)| nombre.eq_ignore_ascii_case("content-type"))
        .map(|(_, valor)| valor.clone())
        .unwrap_or(mime);

    match analizar_rango(buscar_cabecera(cabeceras_peticion, "Range"), tamano) {
        Rango::Valido(inicio, fin) => {
            let longitud = fin - inicio + 1;
            if archivo.seek(SeekFrom::Start(inicio)).is_err() {
                let _ = escribir_encabezados(flujo, 500, &[]);
                return;
            }
            let mut cabeceras = vec![
                ("Content-Type".to_string(), tipo_contenido),
                ("Content-Length".to_string(), longitud.to_string()),
                ("Content-Range".to_string(), format!("bytes {inicio}-{fin}/{tamano}")),
                ("Accept-Ranges".to_string(), "bytes".to_string()),
                ("ETag".to_string(), etag),
                ("Last-Modified".to_string(), fecha_modificacion),
            ];
            cabeceras.append(&mut cabeceras_extra);
            if escribir_encabezados(flujo, 206, &cabeceras).is_ok() {
                let _ = transmitir_archivo(flujo, &mut archivo, longitud);
            }
        }
        Rango::Invalido => {
            let cabeceras = vec![
                ("Content-Range".to_string(), format!("bytes */{tamano}")),
                ("Accept-Ranges".to_string(), "bytes".to_string()),
            ];
            let _ = escribir_encabezados(flujo, 416, &cabeceras);
        }
        Rango::Ninguno => {
            let mut cabeceras = vec![
                ("Content-Type".to_string(), tipo_contenido),
                ("Content-Length".to_string(), tamano.to_string()),
                ("Accept-Ranges".to_string(), "bytes".to_string()),
                ("ETag".to_string(), etag),
                ("Last-Modified".to_string(), fecha_modificacion),
            ];
            cabeceras.append(&mut cabeceras_extra);
            if escribir_encabezados(flujo, 200, &cabeceras).is_ok() {
                let _ = transmitir_archivo(flujo, &mut archivo, tamano);
            }
        }
    }
}

/// Responde una petición con `Respuestas.flujo(...)`: escribe las cabeceras
/// con `Transfer-Encoding: chunked` y, por cada trozo, pide al hilo de la VM
/// (mismo despachador `servidor_http`, evento `"trozo"`) el siguiente valor
/// que produzca la función generadora de Quetzal, hasta que devuelva `nulo`.
/// Corre en el hilo de la conexión.
fn responder_flujo(flujo: &mut TcpStream, id_recurso: u64, manija: &ManijaBucle, campos: Vec<(String, CargaNativa)>) {
    let mapa: HashMap<String, CargaNativa> = campos.into_iter().collect();
    let estado = match mapa.get("estado") {
        Some(CargaNativa::Entero(numero)) => (*numero).clamp(100, 599) as u16,
        _ => 200,
    };
    let Some(CargaNativa::Entero(id_flujo)) = mapa.get("id_flujo") else {
        return;
    };
    let id_flujo = *id_flujo;

    let mut cabeceras = cabeceras_desde_mapa(mapa.get("cabeceras"));
    if !cabeceras.iter().any(|(nombre, _)| nombre.eq_ignore_ascii_case("content-type")) {
        cabeceras.push(("Content-Type".to_string(), "application/octet-stream".to_string()));
    }
    cabeceras.push(("Transfer-Encoding".to_string(), "chunked".to_string()));

    if escribir_encabezados(flujo, estado, &cabeceras).is_err() {
        return;
    }

    loop {
        let (respuesta_tx, respuesta_rx) = mpsc::channel();
        manija.enviar(Mensaje::Solicitud {
            servicio: SERVICIO.to_string(),
            id_recurso,
            datos: CargaNativa::Mapa(vec![
                ("evento".to_string(), CargaNativa::Texto("trozo".to_string())),
                ("id_flujo".to_string(), CargaNativa::Entero(id_flujo)),
            ]),
            respuesta: respuesta_tx,
        });
        let Ok(CargaNativa::Bytes(trozo)) = respuesta_rx.recv_timeout(Duration::from_secs(30)) else {
            break;
        };
        if !trozo.is_empty() {
            let cabecera_trozo = format!("{:x}\r\n", trozo.len());
            let escrito = flujo.write_all(cabecera_trozo.as_bytes()).is_ok()
                && flujo.write_all(&trozo).is_ok()
                && flujo.write_all(b"\r\n").is_ok();
            if !escrito {
                break;
            }
        }
    }
    let _ = flujo.write_all(b"0\r\n\r\n");

    // Limpieza defensiva: si el bucle terminó por desconexión, error de
    // escritura o tiempo de espera agotado (no porque el generador devolvió
    // `nulo`), el generador puede seguir registrado en el hilo de la VM; se
    // avisa para liberarlo y no filtrar memoria.
    let (respuesta_tx, _) = mpsc::channel();
    manija.enviar(Mensaje::Solicitud {
        servicio: SERVICIO.to_string(),
        id_recurso,
        datos: CargaNativa::Mapa(vec![
            ("evento".to_string(), CargaNativa::Texto("cerrar_flujo".to_string())),
            ("id_flujo".to_string(), CargaNativa::Entero(id_flujo)),
        ]),
        respuesta: respuesta_tx,
    });
}

/// Reconstruye los bytes HTTP a partir de la [`CargaNativa`] que produjo el
/// despachador (en el hilo de la VM). Corre en el hilo de la conexión: solo
/// usa tipos `Send`, sin volver a tocar la VM.
fn serializar_respuesta_cruda(carga: CargaNativa) -> Vec<u8> {
    let CargaNativa::Instancia { campos, .. } = carga else {
        return construir_bytes_http(
            500,
            &[("Content-Type".to_string(), "text/plain; charset=utf-8".to_string())],
            b"error interno del servidor",
        );
    };
    let mut estado = 200u16;
    let mut cabeceras = Vec::new();
    let mut cuerpo = Vec::new();
    for (clave, valor) in campos {
        match (clave.as_str(), valor) {
            ("estado", CargaNativa::Entero(numero)) => estado = numero.clamp(100, 599) as u16,
            ("cabeceras", CargaNativa::Mapa(pares)) => {
                for (nombre, valor) in pares {
                    if let CargaNativa::Texto(texto) = valor {
                        cabeceras.push((nombre, texto));
                    }
                }
            }
            ("cuerpo", CargaNativa::Bytes(bytes)) => cuerpo = bytes,
            _ => {}
        }
    }
    construir_bytes_http(estado, &cabeceras, &cuerpo)
}

fn manejar_conexion(flujo: TcpStream, id_recurso: u64, manija: ManijaBucle) {
    let _ = flujo.set_nonblocking(false);
    let Some(peticion) = leer_peticion_cruda(&flujo) else {
        return;
    };
    // Se necesitan más adelante (Range, If-None-Match...) para resolver una
    // respuesta de archivo, pero `peticion` se consume al construir `datos`.
    let cabeceras_peticion = peticion.cabeceras.clone();

    let datos = CargaNativa::Instancia {
        tipo: TIPO_PETICION_CRUDA.to_string(),
        campos: vec![
            ("metodo".to_string(), CargaNativa::Texto(peticion.metodo)),
            ("ruta".to_string(), CargaNativa::Texto(peticion.ruta)),
            (
                "consulta".to_string(),
                CargaNativa::Mapa(
                    peticion
                        .consulta
                        .into_iter()
                        .map(|(clave, valor)| (clave, CargaNativa::Texto(valor)))
                        .collect(),
                ),
            ),
            (
                "cabeceras".to_string(),
                CargaNativa::Mapa(
                    peticion
                        .cabeceras
                        .into_iter()
                        .map(|(clave, valor)| (clave, CargaNativa::Texto(valor)))
                        .collect(),
                ),
            ),
            ("cuerpo".to_string(), CargaNativa::Bytes(peticion.cuerpo)),
        ],
    };

    let (respuesta_tx, respuesta_rx) = mpsc::channel();
    manija.enviar(Mensaje::Solicitud {
        servicio: SERVICIO.to_string(),
        id_recurso,
        datos,
        respuesta: respuesta_tx,
    });
    let respuesta = respuesta_rx.recv_timeout(Duration::from_secs(30)).unwrap_or(CargaNativa::Nula);
    let mut flujo = flujo;
    match respuesta {
        CargaNativa::Instancia { tipo, campos } if tipo == TIPO_RESPUESTA_ARCHIVO => {
            responder_archivo(&mut flujo, &cabeceras_peticion, campos);
        }
        CargaNativa::Instancia { tipo, campos } if tipo == TIPO_RESPUESTA_FLUJO => {
            responder_flujo(&mut flujo, id_recurso, &manija, campos);
        }
        otra => {
            let bytes = serializar_respuesta_cruda(otra);
            let _ = flujo.write_all(&bytes);
        }
    }
    let _ = flujo.flush();
}

// =====================================================================
// Despachador (hilo de la VM): enrutamiento e invocación del manejador
// =====================================================================

/// Segmentos no vacíos de una ruta (`/a/b/` -> `["a", "b"]`).
fn segmentos(ruta: &str) -> Vec<&str> {
    ruta.trim_matches('/').split('/').filter(|s| !s.is_empty()).collect()
}

/// Si `patron` coincide con `ruta` (soporta `:nombre` como comodín),
/// devuelve los parámetros capturados.
fn coincide_ruta(patron: &str, ruta: &str) -> Option<IndexMap<String, Valor>> {
    let patron_segmentos = segmentos(patron);
    let ruta_segmentos = segmentos(ruta);
    if patron_segmentos.len() != ruta_segmentos.len() {
        return None;
    }
    let mut parametros = IndexMap::new();
    for (segmento_patron, segmento_ruta) in patron_segmentos.iter().zip(ruta_segmentos.iter()) {
        if let Some(nombre) = segmento_patron.strip_prefix(':') {
            parametros.insert(nombre.to_string(), Valor::texto(*segmento_ruta));
        } else if segmento_patron != segmento_ruta {
            return None;
        }
    }
    Some(parametros)
}

/// Busca la primera ruta registrada que coincida con `metodo` y `ruta`.
fn buscar_ruta(instancia: &DatosInstanciaNativa, metodo: &str, ruta: &str) -> Option<(Valor, IndexMap<String, Valor>)> {
    let datos = instancia.datos.borrow();
    let Some(Valor::Lista(rutas)) = datos.get("rutas") else {
        return None;
    };
    for entrada in rutas.borrow().iter() {
        let Valor::Jsn(mapa) = entrada else { continue };
        let mapa = mapa.borrow();
        let coincide_metodo = matches!(mapa.get("metodo"), Some(Valor::Texto(m)) if m.eq_ignore_ascii_case(metodo));
        if !coincide_metodo {
            continue;
        }
        let Some(Valor::Texto(patron)) = mapa.get("patron") else {
            continue;
        };
        if let Some(parametros) = coincide_ruta(patron, ruta) {
            let manejador = mapa.get("manejador").cloned().unwrap_or(Valor::Nulo);
            return Some((manejador, parametros));
        }
    }
    None
}

fn invocar_manejador(vm: &mut Vm, manejador: &Valor, peticion: Valor) -> Result<Valor, Fallo> {
    match manejador {
        Valor::Funcion(funcion, entorno) => {
            vm.llamar_funcion(funcion, entorno, vec![peticion], None, None)
        }
        otro => Err(error(
            "E0406",
            format!(
                "el manejador de la ruta no es una función válida (tipo '{}')",
                otro.nombre_tipo()
            ),
        )),
    }
}

fn construir_carga_respuesta(
    estado: i64,
    tipo_contenido: Option<&str>,
    mut cabeceras: Vec<(String, String)>,
    cuerpo: Vec<u8>,
) -> CargaNativa {
    if let Some(tipo) = tipo_contenido
        && !cabeceras.iter().any(|(nombre, _)| nombre.eq_ignore_ascii_case("content-type"))
    {
        cabeceras.push(("Content-Type".to_string(), tipo.to_string()));
    }
    CargaNativa::Instancia {
        tipo: TIPO_RESPUESTA_CRUDA.to_string(),
        campos: vec![
            ("estado".to_string(), CargaNativa::Entero(estado)),
            (
                "cabeceras".to_string(),
                CargaNativa::Mapa(
                    cabeceras
                        .into_iter()
                        .map(|(clave, valor)| (clave, CargaNativa::Texto(valor)))
                        .collect(),
                ),
            ),
            ("cuerpo".to_string(), CargaNativa::Bytes(cuerpo)),
        ],
    }
}

/// Cabeceras extra puestas con `RespuestaServidor.fijar_cabecera(...)`,
/// convertidas a la representación que cruza al hilo de la conexión.
fn cabeceras_extra_a_carga(datos: &IndexMap<String, Valor>) -> CargaNativa {
    let cabeceras = match datos.get("cabeceras") {
        Some(Valor::Jsn(mapa)) => mapa
            .borrow()
            .iter()
            .map(|(clave, valor)| (clave.clone(), CargaNativa::Texto(maquina_virtual::texto_de_valor(valor))))
            .collect(),
        _ => Vec::new(),
    };
    CargaNativa::Mapa(cabeceras)
}

/// Convierte una `RespuestaServidor` de `Respuestas.archivo(...)` (o de una
/// ruta estática) en la carga que cruza al hilo de la conexión: solo la
/// ruta y los metadatos viajan, el archivo se transmite del disco al socket
/// allá (ver [`responder_archivo`]).
fn respuesta_archivo_a_carga(datos: &IndexMap<String, Valor>) -> CargaNativa {
    let texto_de = |clave: &str| match datos.get(clave) {
        Some(Valor::Texto(texto)) => texto.to_string(),
        _ => String::new(),
    };
    let entero_de = |clave: &str| match datos.get(clave) {
        Some(Valor::Entero(numero)) => *numero,
        _ => 0,
    };
    CargaNativa::Instancia {
        tipo: TIPO_RESPUESTA_ARCHIVO.to_string(),
        campos: vec![
            ("ruta".to_string(), CargaNativa::Texto(texto_de("archivo_ruta"))),
            ("tamano".to_string(), CargaNativa::Entero(entero_de("archivo_tamano"))),
            ("mtime_ms".to_string(), CargaNativa::Entero(entero_de("archivo_mtime_ms"))),
            ("etag".to_string(), CargaNativa::Texto(texto_de("archivo_etag"))),
            ("mime".to_string(), CargaNativa::Texto(texto_de("archivo_mime"))),
            ("cabeceras".to_string(), cabeceras_extra_a_carga(datos)),
        ],
    }
}

/// Convierte una `RespuestaServidor` de `Respuestas.flujo(...)` en la carga
/// que cruza al hilo de la conexión: la función generadora (`Rc`, no
/// `Send`) se queda registrada en `registro_flujos` bajo un `id_flujo`
/// nuevo, y solo ese identificador viaja.
fn respuesta_flujo_a_carga(vm: &mut Vm, registro_flujos: &RegistroFlujos, datos: &IndexMap<String, Valor>) -> CargaNativa {
    let estado = match datos.get("estado") {
        Some(Valor::Entero(numero)) => *numero,
        _ => 200,
    };
    let generador = datos.get("generador").cloned().unwrap_or(Valor::Nulo);
    let cabeceras = cabeceras_extra_a_carga(datos);
    let id_flujo = vm.bucle().nuevo_id();
    registro_flujos.borrow_mut().insert(id_flujo, generador);
    CargaNativa::Instancia {
        tipo: TIPO_RESPUESTA_FLUJO.to_string(),
        campos: vec![
            ("estado".to_string(), CargaNativa::Entero(estado)),
            ("id_flujo".to_string(), CargaNativa::Entero(id_flujo as i64)),
            ("cabeceras".to_string(), cabeceras),
        ],
    }
}

/// Convierte lo que devolvió el manejador de Quetzal en una respuesta cruda:
/// una instancia `RespuestaServidor` (control total, incluyendo
/// `Respuestas.archivo`/`Respuestas.flujo`) o directamente texto, jsn,
/// `Bits` o nulo (200 implícito, tipo de contenido inferido).
fn respuesta_desde_valor(vm: &mut Vm, registro_flujos: &RegistroFlujos, valor: &Valor) -> CargaNativa {
    if let Valor::InstanciaNativa(instancia) = valor
        && &*instancia.tipo == TIPO_RESPUESTA_SERVIDOR
    {
        let datos = instancia.datos.borrow();
        if matches!(datos.get("es_archivo"), Some(Valor::Log(true))) {
            return respuesta_archivo_a_carga(&datos);
        }
        if matches!(datos.get("es_flujo"), Some(Valor::Log(true))) {
            return respuesta_flujo_a_carga(vm, registro_flujos, &datos);
        }
        let estado = match datos.get("estado") {
            Some(Valor::Entero(numero)) => *numero,
            _ => 200,
        };
        let cabeceras = match datos.get("cabeceras") {
            Some(Valor::Jsn(mapa)) => mapa
                .borrow()
                .iter()
                .map(|(clave, valor)| (clave.clone(), maquina_virtual::texto_de_valor(valor)))
                .collect(),
            _ => Vec::new(),
        };
        let cuerpo_valor = datos.get("cuerpo").cloned().unwrap_or(Valor::Nulo);
        let (bytes, tipo_contenido) = bytes_y_tipo_contenido("RespuestaServidor", &cuerpo_valor)
            .unwrap_or_else(|_| (maquina_virtual::texto_de_valor(&cuerpo_valor).into_bytes(), Some("text/plain; charset=utf-8")));
        return construir_carga_respuesta(estado, tipo_contenido, cabeceras, bytes);
    }

    let (bytes, tipo_contenido) = match bytes_y_tipo_contenido("el manejador", valor) {
        Ok(par) => par,
        Err(_) => (
            maquina_virtual::texto_de_valor(valor).into_bytes(),
            Some("text/plain; charset=utf-8"),
        ),
    };
    construir_carga_respuesta(200, tipo_contenido, Vec::new(), bytes)
}

fn respuesta_desde_error(fallo: Fallo) -> CargaNativa {
    let mensaje = match fallo {
        Fallo::Excepcion(datos) => datos.mensaje,
        Fallo::Error(error) => error.mensaje,
    };
    construir_carga_respuesta(
        500,
        Some("text/plain; charset=utf-8"),
        Vec::new(),
        format!("error interno: {mensaje}").into_bytes(),
    )
}

fn respuesta_no_encontrada(metodo: &str, ruta: &str) -> CargaNativa {
    construir_carga_respuesta(
        404,
        Some("text/plain; charset=utf-8"),
        Vec::new(),
        format!("ruta no encontrada: {metodo} {ruta}").into_bytes(),
    )
}

fn construir_peticion_http(
    metodo: String,
    ruta: String,
    parametros: IndexMap<String, Valor>,
    consulta: Valor,
    cabeceras: Valor,
    cuerpo: Valor,
) -> Valor {
    let mut datos = IndexMap::new();
    datos.insert("metodo".to_string(), Valor::texto(metodo));
    datos.insert("ruta".to_string(), Valor::texto(ruta));
    datos.insert("parametros".to_string(), Valor::jsn(parametros));
    datos.insert("consulta".to_string(), consulta);
    datos.insert("cabeceras".to_string(), cabeceras);
    datos.insert("cuerpo".to_string(), cuerpo);
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_PETICION),
        datos: RefCell::new(datos),
    }))
}

/// Pide al generador de `Respuestas.flujo(...)` registrado bajo `id_flujo`
/// su siguiente trozo (invocando la función de Quetzal, sin argumentos).
/// `nulo` termina el flujo y libera el registro; cualquier otro valor se
/// convierte a bytes (texto o `Bits`) como un trozo más.
fn siguiente_trozo(vm: &mut Vm, registro_flujos: &RegistroFlujos, id_flujo: u64) -> CargaNativa {
    let Some(generador) = registro_flujos.borrow().get(&id_flujo).cloned() else {
        return CargaNativa::Nula;
    };
    let Valor::Funcion(funcion, entorno) = &generador else {
        registro_flujos.borrow_mut().remove(&id_flujo);
        return CargaNativa::Nula;
    };
    match vm.llamar_funcion(funcion, entorno, Vec::new(), None, None) {
        Ok(Valor::Nulo) => {
            registro_flujos.borrow_mut().remove(&id_flujo);
            CargaNativa::Nula
        }
        Ok(valor) => match bytes_y_tipo_contenido("Respuestas.flujo", &valor) {
            Ok((bytes, _)) => CargaNativa::Bytes(bytes),
            Err(_) => CargaNativa::Bytes(maquina_virtual::texto_de_valor(&valor).into_bytes()),
        },
        Err(_fallo) => {
            registro_flujos.borrow_mut().remove(&id_flujo);
            CargaNativa::Nula
        }
    }
}

/// Atiende los eventos internos del hilo de conexión de una respuesta por
/// trozos (`"trozo"`: siguiente valor del generador; `"cerrar_flujo"`:
/// limpieza al terminar o desconectarse), distintos de una petición HTTP
/// normal (que llega como [`CargaNativa::Instancia`], no como
/// [`CargaNativa::Mapa`]).
fn despachar_evento_flujo(vm: &mut Vm, registro_flujos: &RegistroFlujos, campos: Vec<(String, CargaNativa)>) -> CargaNativa {
    let mapa: HashMap<String, CargaNativa> = campos.into_iter().collect();
    let Some(CargaNativa::Entero(id_flujo)) = mapa.get("id_flujo") else {
        return CargaNativa::Nula;
    };
    let id_flujo = *id_flujo as u64;
    match mapa.get("evento") {
        Some(CargaNativa::Texto(evento)) if evento == "trozo" => siguiente_trozo(vm, registro_flujos, id_flujo),
        Some(CargaNativa::Texto(evento)) if evento == "cerrar_flujo" => {
            registro_flujos.borrow_mut().remove(&id_flujo);
            CargaNativa::Nula
        }
        _ => CargaNativa::Nula,
    }
}

/// Busca `ruta` entre las carpetas registradas con `ServidorHttp.estaticos`
/// (solo para `GET`, y solo si ninguna ruta explícita coincidió antes: esas
/// tienen prioridad). La protección contra *path traversal* la hace
/// [`GuardianPermisos::verificar_lectura`] al canonicalizar la ruta final:
/// si queda fuera de los directorios declarados en `quetzal.json`, se
/// deniega igual que cualquier otra lectura de archivo.
fn intentar_estatico(guardian: &GuardianPermisos, instancia: &DatosInstanciaNativa, ruta: &str) -> Option<CargaNativa> {
    let datos = instancia.datos.borrow();
    let Some(Valor::Lista(lista)) = datos.get("estaticos") else {
        return None;
    };
    let segmentos_ruta = segmentos(ruta);
    for entrada in lista.borrow().iter() {
        let Valor::Jsn(mapa) = entrada else { continue };
        let mapa = mapa.borrow();
        let (Some(Valor::Texto(prefijo)), Some(Valor::Texto(directorio))) = (mapa.get("prefijo"), mapa.get("directorio")) else {
            continue;
        };
        let segmentos_prefijo = segmentos(prefijo);
        if segmentos_ruta.len() < segmentos_prefijo.len() || segmentos_ruta[..segmentos_prefijo.len()] != segmentos_prefijo[..] {
            continue;
        }
        let resto = segmentos_ruta[segmentos_prefijo.len()..].join("/");
        let ruta_completa = if resto.is_empty() {
            directorio.to_string()
        } else {
            format!("{directorio}/{resto}")
        };
        if guardian.verificar_lectura(&ruta_completa).is_err() {
            continue;
        }
        let Ok(metadatos) = std::fs::metadata(&ruta_completa) else {
            continue;
        };
        if !metadatos.is_file() {
            continue;
        }
        let tamano = metadatos.len();
        let mtime_ms = metadatos
            .modified()
            .ok()
            .and_then(|tiempo| tiempo.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duracion| duracion.as_millis() as i64)
            .unwrap_or(0);
        return Some(CargaNativa::Instancia {
            tipo: TIPO_RESPUESTA_ARCHIVO.to_string(),
            campos: vec![
                ("ruta".to_string(), CargaNativa::Texto(ruta_completa.clone())),
                ("tamano".to_string(), CargaNativa::Entero(tamano as i64)),
                ("mtime_ms".to_string(), CargaNativa::Entero(mtime_ms)),
                ("etag".to_string(), CargaNativa::Texto(calcular_etag(tamano, mtime_ms))),
                ("mime".to_string(), CargaNativa::Texto(mime_por_extension(&ruta_completa))),
                ("cabeceras".to_string(), CargaNativa::Mapa(Vec::new())),
            ],
        });
    }
    None
}

/// Despachador único del servicio `"servidor_http"`: corre en el hilo de la
/// VM (`Vm::despachar_solicitud`) para cada petición entrante de cualquier
/// `ServidorHttp` en escucha, identificado por `id_recurso`, y también para
/// los eventos internos de una respuesta por trozos (`Respuestas.flujo`).
fn despachar_peticion(
    vm: &mut Vm,
    id_recurso: u64,
    registro_servidores: &RegistroServidores,
    registro_flujos: &RegistroFlujos,
    guardian: &Rc<GuardianPermisos>,
    datos: CargaNativa,
) -> CargaNativa {
    // Los eventos de un flujo por trozos (`"trozo"`/`"cerrar_flujo"`) viajan
    // como `Mapa`; una petición HTTP normal siempre llega como `Instancia`
    // (`_peticion_cruda`), así que no hay ambigüedad posible.
    let CargaNativa::Instancia { .. } = &datos else {
        let CargaNativa::Mapa(campos) = datos else {
            return CargaNativa::Nula;
        };
        return despachar_evento_flujo(vm, registro_flujos, campos);
    };

    let Some(instancia) = registro_servidores.borrow().get(&id_recurso).cloned() else {
        return construir_carga_respuesta(
            503,
            Some("text/plain; charset=utf-8"),
            Vec::new(),
            b"el servidor ya no esta disponible".to_vec(),
        );
    };

    let peticion_cruda = maquina_virtual::carga_a_valor(datos);
    let Valor::InstanciaNativa(cruda) = &peticion_cruda else {
        return respuesta_desde_error(error_servidor("no se pudo interpretar la petición entrante"));
    };
    let (metodo, ruta, consulta, cabeceras, cuerpo) = {
        let datos_cruda = cruda.datos.borrow();
        let metodo = match datos_cruda.get("metodo") {
            Some(Valor::Texto(texto)) => texto.to_string(),
            _ => String::new(),
        };
        let ruta = match datos_cruda.get("ruta") {
            Some(Valor::Texto(texto)) => texto.to_string(),
            _ => "/".to_string(),
        };
        let consulta = datos_cruda.get("consulta").cloned().unwrap_or_else(|| Valor::jsn(IndexMap::new()));
        let cabeceras = datos_cruda.get("cabeceras").cloned().unwrap_or_else(|| Valor::jsn(IndexMap::new()));
        let cuerpo = datos_cruda.get("cuerpo").cloned().unwrap_or(Valor::Nulo);
        (metodo, ruta, consulta, cabeceras, cuerpo)
    };

    let Some((manejador, parametros)) = buscar_ruta(&instancia, &metodo, &ruta) else {
        if metodo.eq_ignore_ascii_case("GET")
            && let Some(carga) = intentar_estatico(guardian, &instancia, &ruta)
        {
            return carga;
        }
        return respuesta_no_encontrada(&metodo, &ruta);
    };

    let peticion_http = construir_peticion_http(metodo, ruta, parametros, consulta, cabeceras, cuerpo);
    match invocar_manejador(vm, &manejador, peticion_http) {
        Ok(resultado) => respuesta_desde_valor(vm, registro_flujos, &resultado),
        Err(fallo) => respuesta_desde_error(fallo),
    }
}

// =====================================================================
// PeticionHttp: métodos de solo lectura
// =====================================================================

fn receptor_peticion(funcion: &str, argumentos: &[Valor]) -> Result<Rc<DatosInstanciaNativa>, Fallo> {
    match argumentos.first() {
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == TIPO_PETICION => {
            Ok(Rc::clone(instancia))
        }
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba un PeticionHttp, pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
        None => Err(error("E0210", format!("'{funcion}' necesita el receptor"))),
    }
}

fn campo_peticion(funcion: &str, argumentos: &[Valor], campo: &str) -> Result<Valor, Fallo> {
    let instancia = receptor_peticion(funcion, argumentos)?;
    Ok(instancia.datos.borrow().get(campo).cloned().unwrap_or(Valor::Nulo))
}

fn metodo_peticion_metodo(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.metodo";
    exigir_aridad(F, &argumentos[1..], 0)?;
    campo_peticion(F, argumentos, "metodo")
}

fn metodo_peticion_ruta(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.ruta";
    exigir_aridad(F, &argumentos[1..], 0)?;
    campo_peticion(F, argumentos, "ruta")
}

fn metodo_peticion_parametros(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.parametros";
    exigir_aridad(F, &argumentos[1..], 0)?;
    campo_peticion(F, argumentos, "parametros")
}

fn metodo_peticion_consulta(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.consulta";
    exigir_aridad(F, &argumentos[1..], 0)?;
    campo_peticion(F, argumentos, "consulta")
}

fn metodo_peticion_cabeceras(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.cabeceras";
    exigir_aridad(F, &argumentos[1..], 0)?;
    campo_peticion(F, argumentos, "cabeceras")
}

/// `PeticionHttp.cabecera(nombre)`: busca una cabecera sin distinguir
/// mayúsculas/minúsculas (`Content-Type`, `content-type`... son la misma
/// cabecera) y devuelve su valor como `texto`, o `nulo` si no está.
fn metodo_peticion_cabecera(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.cabecera";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let nombre = arg_texto(F, argumentos, 1)?.to_string();
    let cabeceras = campo_peticion(F, argumentos, "cabeceras")?;
    if let Valor::Jsn(mapa) = cabeceras {
        for (clave, valor) in mapa.borrow().iter() {
            if clave.eq_ignore_ascii_case(&nombre) {
                return Ok(valor.clone());
            }
        }
    }
    Ok(Valor::Nulo)
}

fn metodo_peticion_bits(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.bits";
    exigir_aridad(F, &argumentos[1..], 0)?;
    campo_peticion(F, argumentos, "cuerpo")
}

fn bytes_cuerpo_peticion(funcion: &str, argumentos: &[Valor]) -> Result<Vec<u8>, Fallo> {
    let cuerpo = campo_peticion(funcion, argumentos, "cuerpo")?;
    maquina_virtual::bytes_de_bits(&cuerpo)
        .ok_or_else(|| error("E0406", format!("'{funcion}' recibió una PeticionHttp sin cuerpo binario válido")))
}

fn metodo_peticion_texto(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.texto";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let bytes = bytes_cuerpo_peticion(F, argumentos)?;
    String::from_utf8(bytes)
        .map(Valor::texto)
        .map_err(|_| error("E0406", format!("'{F}' no pudo decodificar el cuerpo como texto UTF-8 válido")))
}

fn metodo_peticion_jsn(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.jsn";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let bytes = bytes_cuerpo_peticion(F, argumentos)?;
    let texto = String::from_utf8(bytes)
        .map_err(|_| error("E0406", format!("'{F}' no pudo decodificar el cuerpo como texto UTF-8 válido")))?;
    let json: serde_json::Value = serde_json::from_str(&texto)
        .map_err(|causa| error("E0406", format!("'{F}': el cuerpo no es JSON válido: {causa}")))?;
    Ok(maquina_virtual::valores::json_a_valor(&json))
}

// =====================================================================
// RespuestaServidor: `Respuestas.crear(estado, cuerpo)`
// =====================================================================

fn respuestas_crear(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Respuestas.crear";
    exigir_aridad(F, argumentos, 2)?;
    let estado = arg_entero(F, argumentos, 0)?;
    if !(100..=599).contains(&estado) {
        return Err(error(
            "E0406",
            format!("'{F}' espera un código de estado HTTP entre 100 y 599, pero recibió {estado}"),
        ));
    }
    let mut datos = IndexMap::new();
    datos.insert("estado".to_string(), Valor::Entero(estado));
    datos.insert("cabeceras".to_string(), Valor::jsn(IndexMap::new()));
    datos.insert("cuerpo".to_string(), argumentos[1].clone());
    Ok(Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_RESPUESTA_SERVIDOR),
        datos: RefCell::new(datos),
    })))
}

/// `Respuestas.archivo(ruta)`: sirve un archivo del disco con streaming de
/// 64 KiB por trozo (nunca lo carga completo en memoria), soporte de
/// `Range` (`206`), caché condicional (`ETag`/`Last-Modified`, `304`) y
/// tipo MIME automático por extensión. Requiere permiso de lectura sobre
/// `ruta` en `quetzal.json` (`sistema_archivos`), verificado aquí mismo
/// (igual que `SistemaArchivos.leer_bits`).
fn respuestas_archivo(guardian: &Rc<GuardianPermisos>) -> maquina_virtual::FuncionNativa {
    const F: &str = "Respuestas.archivo";
    let guardian = Rc::clone(guardian);
    Box::new(move |argumentos| {
        exigir_aridad(F, argumentos, 1)?;
        let ruta = arg_ruta(F, argumentos, 0)?;
        guardian.verificar_lectura(&ruta).map_err(permiso_denegado)?;
        let metadatos = std::fs::metadata(&ruta)
            .map_err(|causa| error_servidor(format!("'{F}': no se pudo leer '{ruta}': {causa}")))?;
        if !metadatos.is_file() {
            return Err(error_servidor(format!("'{F}': '{ruta}' no es un archivo")));
        }
        let tamano = metadatos.len();
        let mtime_ms = metadatos
            .modified()
            .ok()
            .and_then(|tiempo| tiempo.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duracion| duracion.as_millis() as i64)
            .unwrap_or(0);

        let mime = mime_por_extension(&ruta);
        let mut datos = IndexMap::new();
        datos.insert("estado".to_string(), Valor::Entero(200));
        datos.insert("cabeceras".to_string(), Valor::jsn(IndexMap::new()));
        datos.insert("cuerpo".to_string(), Valor::Nulo);
        datos.insert("es_archivo".to_string(), Valor::Log(true));
        datos.insert("archivo_ruta".to_string(), Valor::texto(ruta));
        datos.insert("archivo_tamano".to_string(), Valor::Entero(tamano as i64));
        datos.insert("archivo_mtime_ms".to_string(), Valor::Entero(mtime_ms));
        datos.insert("archivo_etag".to_string(), Valor::texto(calcular_etag(tamano, mtime_ms)));
        datos.insert("archivo_mime".to_string(), Valor::texto(mime));
        Ok(Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
            tipo: Rc::from(TIPO_RESPUESTA_SERVIDOR),
            datos: RefCell::new(datos),
        })))
    })
}

/// `Respuestas.flujo(generador)` (o `Respuestas.flujo(estado, generador)`):
/// respuesta cuyo cuerpo se produce en trozos, uno por cada llamada a
/// `generador` (una función de Quetzal sin argumentos, pasada por
/// referencia: nunca se copia, solo se clona el `Rc` del `Valor`). Cada
/// llamada debe devolver el siguiente trozo (`texto` o `Bits`) o `nulo`
/// para terminar el flujo. Se envía con `Transfer-Encoding: chunked`, así
/// que sirve tanto para archivos generados sobre la marcha como para
/// *server-sent events* (`Content-Type: text/event-stream` + líneas
/// `data: ...\n\n`).
fn respuestas_flujo(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Respuestas.flujo";
    let (estado, generador) = match argumentos.len() {
        1 => (200i64, argumentos[0].clone()),
        2 => {
            let estado = arg_entero(F, argumentos, 0)?;
            if !(100..=599).contains(&estado) {
                return Err(error(
                    "E0406",
                    format!("'{F}' espera un código de estado HTTP entre 100 y 599, pero recibió {estado}"),
                ));
            }
            (estado, argumentos[1].clone())
        }
        recibidos => {
            return Err(error(
                "E0210",
                format!("'{F}' espera 1 argumento (generador) o 2 (estado, generador), pero recibió {recibidos}"),
            ));
        }
    };
    if !matches!(generador, Valor::Funcion(..)) {
        return Err(error(
            "E0406",
            format!(
                "'{F}' espera una función generadora sin argumentos como manejador, pero recibió '{}'",
                generador.nombre_tipo()
            ),
        ));
    }
    let mut datos = IndexMap::new();
    datos.insert("estado".to_string(), Valor::Entero(estado));
    datos.insert("cabeceras".to_string(), Valor::jsn(IndexMap::new()));
    datos.insert("cuerpo".to_string(), Valor::Nulo);
    datos.insert("es_flujo".to_string(), Valor::Log(true));
    datos.insert("generador".to_string(), generador);
    Ok(Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_RESPUESTA_SERVIDOR),
        datos: RefCell::new(datos),
    })))
}

fn receptor_respuesta_servidor(funcion: &str, argumentos: &[Valor]) -> Result<Rc<DatosInstanciaNativa>, Fallo> {
    match argumentos.first() {
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == TIPO_RESPUESTA_SERVIDOR => {
            Ok(Rc::clone(instancia))
        }
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba un RespuestaServidor, pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
        None => Err(error("E0210", format!("'{funcion}' necesita el receptor"))),
    }
}

fn metodo_respuesta_fijar_cabecera(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "RespuestaServidor.fijar_cabecera";
    exigir_aridad(F, &argumentos[1..], 2)?;
    let instancia = receptor_respuesta_servidor(F, argumentos)?;
    let nombre = arg_texto(F, argumentos, 1)?.to_string();
    let valor = arg_texto(F, argumentos, 2)?.to_string();
    match instancia.datos.borrow().get("cabeceras") {
        Some(Valor::Jsn(mapa)) => {
            mapa.borrow_mut().insert(nombre, Valor::texto(valor));
        }
        _ => return Err(error("E0406", format!("'{F}' recibió un RespuestaServidor sin cabeceras internas"))),
    }
    Ok(Valor::Nulo)
}

// =====================================================================
// Registro
// =====================================================================

pub fn registrar(registro: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    registro.registrar_modulo("servidorhttp");
    registro.registrar_modulo("respuestas");
    registro.registrar_funcion("servidorhttp.constructor", Box::new(constructor));
    registro.registrar_funcion("respuestas.crear", Box::new(respuestas_crear));
    registro.registrar_funcion("respuestas.archivo", respuestas_archivo(guardian));
    registro.registrar_funcion("respuestas.flujo", Box::new(respuestas_flujo));

    let registro_servidores: RegistroServidores = Rc::new(RefCell::new(HashMap::new()));
    let banderas: RegistroBanderas = Rc::new(RefCell::new(HashMap::new()));
    let registro_flujos: RegistroFlujos = Rc::new(RefCell::new(HashMap::new()));

    let rutas_fijas: [(&str, &'static str, reqwest::Method); 5] = [
        ("obtener", "ServidorHttp.obtener", reqwest::Method::GET),
        ("publicar", "ServidorHttp.publicar", reqwest::Method::POST),
        ("poner", "ServidorHttp.poner", reqwest::Method::PUT),
        ("parchar", "ServidorHttp.parchar", reqwest::Method::PATCH),
        ("eliminar", "ServidorHttp.eliminar", reqwest::Method::DELETE),
    ];
    for (nombre, etiqueta, metodo_http) in rutas_fijas {
        registro.registrar_funcion(
            &format!("{TIPO_SERVIDOR}.{nombre}"),
            metodo_ruta_fija(etiqueta, metodo_http),
        );
    }
    registro.registrar_funcion(&format!("{TIPO_SERVIDOR}.ruta"), Box::new(metodo_ruta_generica));
    registro.registrar_funcion(&format!("{TIPO_SERVIDOR}.estaticos"), Box::new(metodo_estaticos));
    registro.registrar_funcion(&format!("{TIPO_SERVIDOR}.puerto"), Box::new(metodo_puerto));
    registro.registrar_funcion(
        &format!("{TIPO_SERVIDOR}.esta_escuchando"),
        Box::new(metodo_esta_escuchando),
    );
    registro.registrar_funcion_con_vm(
        &format!("{TIPO_SERVIDOR}.escuchar"),
        metodo_escuchar(guardian, &registro_servidores, &banderas, &registro_flujos),
    );
    registro.registrar_funcion_con_vm(
        &format!("{TIPO_SERVIDOR}.detener"),
        metodo_detener(&banderas, &registro_servidores),
    );

    type MetodoPeticion = fn(&[Valor]) -> Result<Valor, Fallo>;
    let metodos_peticion: [(&str, MetodoPeticion); 9] = [
        ("metodo", metodo_peticion_metodo),
        ("ruta", metodo_peticion_ruta),
        ("parametros", metodo_peticion_parametros),
        ("consulta", metodo_peticion_consulta),
        ("cabeceras", metodo_peticion_cabeceras),
        ("cabecera", metodo_peticion_cabecera),
        ("bits", metodo_peticion_bits),
        ("texto", metodo_peticion_texto),
        ("jsn", metodo_peticion_jsn),
    ];
    for (nombre, funcion) in metodos_peticion {
        registro.registrar_funcion(&format!("{TIPO_PETICION}.{nombre}"), Box::new(funcion));
    }

    registro.registrar_funcion(
        &format!("{TIPO_RESPUESTA_SERVIDOR}.fijar_cabecera"),
        Box::new(metodo_respuesta_fijar_cabecera),
    );
}
