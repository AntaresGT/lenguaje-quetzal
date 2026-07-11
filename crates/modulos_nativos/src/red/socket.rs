//! Objetos `Socket` (cliente TCP), `ServidorSocket` y `ConexionSocket`:
//! conexiones TCP crudas con transferencia binaria directa vía `Bits`.
//!
//! El flujo TCP real (`std::net::TcpStream`) nunca vive dentro de un
//! `Valor` (la VM usa `Rc`, no es `Send`): vive en un registro compartido
//! entre hilos ([`RegistroFlujos`], `Arc<Mutex<...>>`), y las instancias
//! `Socket`/`ConexionSocket` solo guardan su identificador numérico. Así
//! una conexión aceptada en el hilo de aceptación de un `ServidorSocket`, o
//! conectada en un hilo de fondo por una variante asincrónica, puede
//! registrarse desde cualquier hilo sin tocar la VM; solo se lee/escribe
//! sobre el flujo cuando algún método nativo (síncrono, en el hilo de la
//! VM, o asincrónico, en el runtime de tokio del bucle de eventos) lo pide
//! por su identificador.
//!
//! - `Socket`: `nuevo Socket()` + `fijar_tiempo_espera(seg)` configuran una
//!   plantilla (igual que `ClienteHttp`); `conectar`/`conectar_asincrono`
//!   devuelven una instancia nueva ya conectada (con su identificador), sin
//!   mutar la plantilla original.
//! - `ServidorSocket`: `escuchar(puerto)` acepta conexiones en un hilo de
//!   aceptación (patrón reactor de `ServidorHttp`); cada conexión aceptada
//!   se despacha una sola vez al manejador registrado con `al_conectar`,
//!   que recibe un `ConexionSocket` con la misma API de envío/recepción.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use maquina_virtual::bucle_eventos::CargaNativa;
use maquina_virtual::{DatosInstanciaNativa, Fallo, ManijaBucle, Mensaje, RegistroNativos, Valor, Vm};
use runtime::GuardianPermisos;

use crate::util::{arg_entero, arg_texto, error, exigir_aridad};

use super::permiso_denegado;

const TIPO_SOCKET: &str = "Socket";
const TIPO_SERVIDOR: &str = "ServidorSocket";
const TIPO_CONEXION: &str = "ConexionSocket";

/// Tiempo de espera por defecto (segundos) de un `Socket` nuevo.
const TIEMPO_ESPERA_DEFECTO: i64 = 30;

/// Nombre del servicio del despachador de conexiones entrantes.
const SERVICIO: &str = "servidor_socket";

fn error_socket(mensaje: impl Into<String>) -> Fallo {
    error("E0705", mensaje.into())
}

/// Mensaje textual de un [`Fallo`], para cruzarlo a un hilo de fondo (donde
/// solo se puede transportar `String`, no el `Fallo` completo).
fn mensaje_de_fallo(fallo: Fallo) -> String {
    match fallo {
        Fallo::Excepcion(datos) => datos.mensaje,
        Fallo::Error(error) => error.mensaje,
    }
}

// =====================================================================
// Conexión TCP abierta: registro compartido entre hilos
// =====================================================================

/// Conexión TCP abierta. La lectura vive en un `BufReader` para que
/// `recibir_linea` pueda acumular datos entre llamadas; la escritura usa un
/// manejador clonado independiente (lectura y escritura no se bloquean
/// entre sí).
struct ConexionAbierta {
    lectura: BufReader<TcpStream>,
    escritura: TcpStream,
}

impl ConexionAbierta {
    fn desde(flujo: TcpStream) -> std::io::Result<Self> {
        let escritura = flujo.try_clone()?;
        Ok(Self {
            lectura: BufReader::new(flujo),
            escritura,
        })
    }

    fn fijar_tiempo_espera(&self, segundos: u64) -> std::io::Result<()> {
        let duracion = if segundos == 0 {
            None
        } else {
            Some(Duration::from_secs(segundos))
        };
        self.lectura.get_ref().set_read_timeout(duracion)?;
        self.escritura.set_write_timeout(duracion)?;
        Ok(())
    }
}

/// Registro de conexiones abiertas, por identificador. Se comparte (clon de
/// `Arc`) entre el hilo de la VM, los hilos de aceptación de `ServidorSocket`
/// y las tareas asincrónicas del runtime de tokio.
///
/// El `Mutex` externo solo protege el mapa (inserciones/eliminaciones,
/// siempre breves); cada conexión tiene su propio `Mutex` interno para que
/// una lectura/escritura larga en una conexión (por ejemplo, un cliente
/// lento) nunca bloquee las operaciones de las demás conexiones.
type RegistroFlujos = Arc<Mutex<HashMap<u64, Arc<Mutex<ConexionAbierta>>>>>;

fn siguiente_id(contador: &Arc<AtomicU64>) -> u64 {
    contador.fetch_add(1, Ordering::Relaxed)
}

fn insertar_conexion(flujos: &RegistroFlujos, id: u64, conexion: ConexionAbierta) -> Result<(), Fallo> {
    flujos
        .lock()
        .map_err(|_| error_socket("el registro de conexiones está dañado"))?
        .insert(id, Arc::new(Mutex::new(conexion)));
    Ok(())
}

fn obtener_conexion(flujos: &RegistroFlujos, id: u64) -> Result<Arc<Mutex<ConexionAbierta>>, Fallo> {
    flujos
        .lock()
        .map_err(|_| error_socket("el registro de conexiones está dañado"))?
        .get(&id)
        .cloned()
        .ok_or_else(|| error_socket("la conexión ya está cerrada"))
}

fn enviar_bytes(flujos: &RegistroFlujos, id: u64, bytes: &[u8]) -> Result<(), Fallo> {
    let conexion = obtener_conexion(flujos, id)?;
    let mut conexion = conexion.lock().map_err(|_| error_socket("la conexión está dañada"))?;
    conexion
        .escritura
        .write_all(bytes)
        .map_err(|causa| error_socket(format!("no se pudo enviar: {causa}")))
}

fn recibir_bytes(flujos: &RegistroFlujos, id: u64, maximo: usize) -> Result<Vec<u8>, Fallo> {
    let conexion = obtener_conexion(flujos, id)?;
    let mut conexion = conexion.lock().map_err(|_| error_socket("la conexión está dañada"))?;
    let mut buffer = vec![0u8; maximo.max(1)];
    let leidos = conexion
        .lectura
        .read(&mut buffer)
        .map_err(|causa| error_socket(format!("no se pudo recibir: {causa}")))?;
    buffer.truncate(leidos);
    Ok(buffer)
}

fn recibir_linea_texto(flujos: &RegistroFlujos, id: u64) -> Result<String, Fallo> {
    let conexion = obtener_conexion(flujos, id)?;
    let mut conexion = conexion.lock().map_err(|_| error_socket("la conexión está dañada"))?;
    let mut linea = String::new();
    conexion
        .lectura
        .read_line(&mut linea)
        .map_err(|causa| error_socket(format!("no se pudo recibir la línea: {causa}")))?;
    if linea.ends_with('\n') {
        linea.pop();
        if linea.ends_with('\r') {
            linea.pop();
        }
    }
    Ok(linea)
}

fn cerrar_conexion(flujos: &RegistroFlujos, id: u64) {
    let Ok(mut mapa) = flujos.lock() else { return };
    let Some(conexion) = mapa.remove(&id) else { return };
    drop(mapa);
    if let Ok(conexion) = conexion.lock() {
        let _ = conexion.escritura.shutdown(std::net::Shutdown::Both);
    }
}

fn fijar_tiempo_espera_id(flujos: &RegistroFlujos, id: u64, segundos: i64) -> Result<(), Fallo> {
    let segundos = segundos.max(0) as u64;
    let Ok(conexion) = obtener_conexion(flujos, id) else {
        // Sin conexión abierta todavía (p. ej. plantilla sin conectar): la
        // configuración se aplicará cuando se conecte.
        return Ok(());
    };
    let conexion = conexion.lock().map_err(|_| error_socket("la conexión está dañada"))?;
    conexion
        .fijar_tiempo_espera(segundos)
        .map_err(|causa| error_socket(format!("no se pudo fijar el tiempo de espera: {causa}")))
}

fn resolver_direccion(funcion: &str, anfitrion: &str, puerto: u16) -> Result<SocketAddr, Fallo> {
    (anfitrion, puerto)
        .to_socket_addrs()
        .map_err(|causa| error_socket(format!("'{funcion}' no pudo resolver '{anfitrion}:{puerto}': {causa}")))?
        .next()
        .ok_or_else(|| error_socket(format!("'{funcion}' no encontró ninguna dirección para '{anfitrion}:{puerto}'")))
}

fn conectar_flujo(direccion: SocketAddr, tiempo_espera: u64) -> std::io::Result<TcpStream> {
    if tiempo_espera == 0 {
        TcpStream::connect(direccion)
    } else {
        TcpStream::connect_timeout(&direccion, Duration::from_secs(tiempo_espera))
    }
}

// =====================================================================
// Instancias: helpers genéricos compartidos por Socket/ConexionSocket
// =====================================================================

fn receptor_con_tipo(funcion: &str, argumentos: &[Valor], tipo: &str) -> Result<Rc<DatosInstanciaNativa>, Fallo> {
    match argumentos.first() {
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == tipo => Ok(Rc::clone(instancia)),
        Some(otro) => Err(error(
            "E0406",
            format!("'{funcion}' esperaba un {tipo}, pero recibió '{}'", otro.nombre_tipo()),
        )),
        None => Err(error("E0210", format!("'{funcion}' necesita el receptor"))),
    }
}

fn id_de_instancia(funcion: &str, instancia: &DatosInstanciaNativa) -> Result<u64, Fallo> {
    match instancia.datos.borrow().get("id") {
        Some(Valor::Entero(id)) if *id >= 0 => Ok(*id as u64),
        _ => Err(error_socket(format!(
            "'{funcion}' recibió una conexión sin identificador válido (¿ya se cerró o nunca se conectó?)"
        ))),
    }
}

fn instancia_con_id(tipo: &str, id: u64, tiempo_espera: i64) -> Valor {
    let mut datos = indexmap::IndexMap::new();
    datos.insert("id".to_string(), Valor::Entero(id as i64));
    datos.insert("tiempo_espera".to_string(), Valor::Entero(tiempo_espera));
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(tipo),
        datos: RefCell::new(datos),
    }))
}

// =====================================================================
// Socket: instancia y configuración (plantilla, sin conectar)
// =====================================================================

fn nuevo_socket() -> Valor {
    let mut datos = indexmap::IndexMap::new();
    datos.insert("id".to_string(), Valor::Nulo);
    datos.insert("tiempo_espera".to_string(), Valor::Entero(TIEMPO_ESPERA_DEFECTO));
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_SOCKET),
        datos: RefCell::new(datos),
    }))
}

fn constructor(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    exigir_aridad("nuevo Socket", argumentos, 0)?;
    Ok(nuevo_socket())
}

fn tiempo_espera_de(instancia: &DatosInstanciaNativa) -> i64 {
    match instancia.datos.borrow().get("tiempo_espera") {
        Some(Valor::Entero(segundos)) => (*segundos).max(0),
        _ => TIEMPO_ESPERA_DEFECTO,
    }
}

fn metodo_fijar_tiempo_espera(
    tipo: &'static str,
    flujos: &RegistroFlujos,
) -> impl Fn(&[Valor]) -> Result<Valor, Fallo> + 'static {
    let flujos = Arc::clone(flujos);
    move |argumentos| {
        let funcion = &format!("{tipo}.fijar_tiempo_espera");
        exigir_aridad(funcion, &argumentos[1..], 1)?;
        let instancia = receptor_con_tipo(funcion, argumentos, tipo)?;
        let segundos = match argumentos.get(1) {
            Some(Valor::Entero(segundos)) if *segundos >= 0 => *segundos,
            Some(otro) => {
                return Err(error(
                    "E0406",
                    format!(
                        "'{funcion}' espera un entero positivo de segundos, pero recibió '{}'",
                        otro.nombre_tipo()
                    ),
                ));
            }
            None => return Err(error("E0210", format!("'{funcion}' necesita al menos 1 argumento"))),
        };
        instancia
            .datos
            .borrow_mut()
            .insert("tiempo_espera".to_string(), Valor::Entero(segundos));
        if let Ok(id) = id_de_instancia(funcion, &instancia) {
            fijar_tiempo_espera_id(&flujos, id, segundos)?;
        }
        Ok(Valor::Nulo)
    }
}

// =====================================================================
// Socket: conectar (síncrono y asincrónico)
// =====================================================================

fn conectar_sincrono(
    guardian: &Rc<GuardianPermisos>,
    flujos: &RegistroFlujos,
    contador: &Arc<AtomicU64>,
) -> impl Fn(&[Valor]) -> Result<Valor, Fallo> + 'static {
    const F: &str = "Socket.conectar";
    let guardian = Rc::clone(guardian);
    let flujos = Arc::clone(flujos);
    let contador = Arc::clone(contador);
    move |argumentos| {
        exigir_aridad(F, &argumentos[1..], 2)?;
        let instancia = receptor_con_tipo(F, argumentos, TIPO_SOCKET)?;
        let anfitrion = arg_texto(F, argumentos, 1)?.to_string();
        let puerto = arg_entero(F, argumentos, 2)?;
        if !(0..=65535).contains(&puerto) {
            return Err(error(
                "E0406",
                format!("'{F}' espera un puerto entre 0 y 65535, pero recibió {puerto}"),
            ));
        }
        let puerto = puerto as u16;
        guardian
            .verificar_red_cliente(&anfitrion, puerto)
            .map_err(permiso_denegado)?;

        let tiempo_espera = tiempo_espera_de(&instancia);
        let direccion = resolver_direccion(F, &anfitrion, puerto)?;
        let flujo = conectar_flujo(direccion, tiempo_espera as u64)
            .map_err(|causa| error_socket(format!("no se pudo conectar a '{anfitrion}:{puerto}': {causa}")))?;
        let conexion = ConexionAbierta::desde(flujo)
            .map_err(|causa| error_socket(format!("no se pudo preparar la conexión: {causa}")))?;
        conexion
            .fijar_tiempo_espera(tiempo_espera as u64)
            .map_err(|causa| error_socket(format!("no se pudo fijar el tiempo de espera: {causa}")))?;

        let id = siguiente_id(&contador);
        insertar_conexion(&flujos, id, conexion)?;

        Ok(instancia_con_id(TIPO_SOCKET, id, tiempo_espera))
    }
}

fn conectar_asincrono(
    guardian: &Rc<GuardianPermisos>,
    flujos: &RegistroFlujos,
    contador: &Arc<AtomicU64>,
) -> maquina_virtual::FuncionNativaConVm {
    const F: &str = "Socket.conectar_asincrono";
    let guardian = Rc::clone(guardian);
    let flujos = Arc::clone(flujos);
    let contador = Arc::clone(contador);
    Box::new(move |vm, argumentos| {
        exigir_aridad(F, &argumentos[1..], 2)?;
        let instancia = receptor_con_tipo(F, argumentos, TIPO_SOCKET)?;
        let anfitrion = arg_texto(F, argumentos, 1)?.to_string();
        let puerto = arg_entero(F, argumentos, 2)?;
        if !(0..=65535).contains(&puerto) {
            return Err(error(
                "E0406",
                format!("'{F}' espera un puerto entre 0 y 65535, pero recibió {puerto}"),
            ));
        }
        let puerto = puerto as u16;
        guardian
            .verificar_red_cliente(&anfitrion, puerto)
            .map_err(permiso_denegado)?;
        let tiempo_espera = tiempo_espera_de(&instancia);

        let id_tarea = vm.bucle().nuevo_id();
        let manija = vm.bucle().manija();
        let manija_tarea = manija.clone();
        let flujos_tarea = Arc::clone(&flujos);
        let contador_tarea = Arc::clone(&contador);

        manija.runtime().spawn(async move {
            let resultado = tokio::task::spawn_blocking(move || -> Result<u64, String> {
                let direccion = resolver_direccion(F, &anfitrion, puerto).map_err(|_| {
                    format!("no se pudo resolver '{anfitrion}:{puerto}'")
                })?;
                let flujo = conectar_flujo(direccion, tiempo_espera as u64)
                    .map_err(|causa| format!("no se pudo conectar a '{anfitrion}:{puerto}': {causa}"))?;
                let conexion = ConexionAbierta::desde(flujo)
                    .map_err(|causa| format!("no se pudo preparar la conexión: {causa}"))?;
                conexion
                    .fijar_tiempo_espera(tiempo_espera as u64)
                    .map_err(|causa| format!("no se pudo fijar el tiempo de espera: {causa}"))?;
                let id = siguiente_id(&contador_tarea);
                insertar_conexion(&flujos_tarea, id, conexion).map_err(mensaje_de_fallo)?;
                Ok(id)
            })
            .await
            .unwrap_or_else(|causa| Err(format!("la tarea de conexión no pudo completarse: {causa}")));

            let carga = resultado.map(|id| CargaNativa::Instancia {
                tipo: TIPO_SOCKET.to_string(),
                campos: vec![
                    ("id".to_string(), CargaNativa::Entero(id as i64)),
                    ("tiempo_espera".to_string(), CargaNativa::Entero(tiempo_espera)),
                ],
            });
            manija_tarea.enviar(Mensaje::TareaLista { id: id_tarea, resultado: carga });
        });

        Ok(Valor::TareaNativa(Rc::new(maquina_virtual::EstadoTareaNativa { id: id_tarea })))
    })
}

// =====================================================================
// Socket / ConexionSocket: envío y recepción (comparten identificador)
// =====================================================================

fn metodo_enviar_texto(tipo: &'static str, flujos: &RegistroFlujos) -> impl Fn(&[Valor]) -> Result<Valor, Fallo> + 'static {
    let flujos = Arc::clone(flujos);
    move |argumentos| {
        let funcion = &format!("{tipo}.enviar_texto");
        exigir_aridad(funcion, &argumentos[1..], 1)?;
        let instancia = receptor_con_tipo(funcion, argumentos, tipo)?;
        let texto = arg_texto(funcion, argumentos, 1)?;
        let id = id_de_instancia(funcion, &instancia)?;
        enviar_bytes(&flujos, id, texto.as_bytes())?;
        Ok(Valor::Nulo)
    }
}

fn metodo_enviar_bits(tipo: &'static str, flujos: &RegistroFlujos) -> impl Fn(&[Valor]) -> Result<Valor, Fallo> + 'static {
    let flujos = Arc::clone(flujos);
    move |argumentos| {
        let funcion = &format!("{tipo}.enviar_bits");
        exigir_aridad(funcion, &argumentos[1..], 1)?;
        let instancia = receptor_con_tipo(funcion, argumentos, tipo)?;
        let bytes = argumentos
            .get(1)
            .and_then(maquina_virtual::bytes_de_bits)
            .ok_or_else(|| error("E0406", format!("'{funcion}' espera un Bits en el argumento 1")))?;
        let id = id_de_instancia(funcion, &instancia)?;
        enviar_bytes(&flujos, id, &bytes)?;
        Ok(Valor::Nulo)
    }
}

/// Tamaño máximo de lectura para `recibir_texto` (sin límite explícito del
/// usuario): una sola lectura del sistema, igual que `recibir_bits`.
const TAMANO_LECTURA_TEXTO: usize = 65536;

fn metodo_recibir_bits(tipo: &'static str, flujos: &RegistroFlujos) -> impl Fn(&[Valor]) -> Result<Valor, Fallo> + 'static {
    let flujos = Arc::clone(flujos);
    move |argumentos| {
        let funcion = &format!("{tipo}.recibir_bits");
        exigir_aridad(funcion, &argumentos[1..], 1)?;
        let instancia = receptor_con_tipo(funcion, argumentos, tipo)?;
        let maximo = arg_entero(funcion, argumentos, 1)?;
        if maximo <= 0 {
            return Err(error("E0406", format!("'{funcion}' espera un máximo de bytes positivo")));
        }
        let id = id_de_instancia(funcion, &instancia)?;
        let bytes = recibir_bytes(&flujos, id, maximo as usize)?;
        Ok(maquina_virtual::instancia_bits_desde(&bytes))
    }
}

fn metodo_recibir_texto(tipo: &'static str, flujos: &RegistroFlujos) -> impl Fn(&[Valor]) -> Result<Valor, Fallo> + 'static {
    let flujos = Arc::clone(flujos);
    move |argumentos| {
        let funcion = &format!("{tipo}.recibir_texto");
        exigir_aridad(funcion, &argumentos[1..], 0)?;
        let instancia = receptor_con_tipo(funcion, argumentos, tipo)?;
        let id = id_de_instancia(funcion, &instancia)?;
        let bytes = recibir_bytes(&flujos, id, TAMANO_LECTURA_TEXTO)?;
        String::from_utf8(bytes)
            .map(Valor::texto)
            .map_err(|_| error("E0406", format!("'{funcion}' no pudo decodificar los datos como texto UTF-8 válido")))
    }
}

fn metodo_recibir_linea(tipo: &'static str, flujos: &RegistroFlujos) -> impl Fn(&[Valor]) -> Result<Valor, Fallo> + 'static {
    let flujos = Arc::clone(flujos);
    move |argumentos| {
        let funcion = &format!("{tipo}.recibir_linea");
        exigir_aridad(funcion, &argumentos[1..], 0)?;
        let instancia = receptor_con_tipo(funcion, argumentos, tipo)?;
        let id = id_de_instancia(funcion, &instancia)?;
        recibir_linea_texto(&flujos, id).map(Valor::texto)
    }
}

fn metodo_cerrar(tipo: &'static str, flujos: &RegistroFlujos) -> impl Fn(&[Valor]) -> Result<Valor, Fallo> + 'static {
    let flujos = Arc::clone(flujos);
    move |argumentos| {
        let funcion = &format!("{tipo}.cerrar");
        exigir_aridad(funcion, &argumentos[1..], 0)?;
        let instancia = receptor_con_tipo(funcion, argumentos, tipo)?;
        if let Ok(id) = id_de_instancia(funcion, &instancia) {
            cerrar_conexion(&flujos, id);
        }
        instancia.datos.borrow_mut().insert("id".to_string(), Valor::Nulo);
        Ok(Valor::Nulo)
    }
}

fn metodo_recibir_bits_asincrono(flujos: &RegistroFlujos) -> maquina_virtual::FuncionNativaConVm {
    const F: &str = "Socket.recibir_bits_asincrono";
    let flujos = Arc::clone(flujos);
    Box::new(move |vm, argumentos| {
        exigir_aridad(F, &argumentos[1..], 1)?;
        let instancia = receptor_con_tipo(F, argumentos, TIPO_SOCKET)?;
        let maximo = arg_entero(F, argumentos, 1)?;
        if maximo <= 0 {
            return Err(error("E0406", format!("'{F}' espera un máximo de bytes positivo")));
        }
        let id = id_de_instancia(F, &instancia)?;

        let id_tarea = vm.bucle().nuevo_id();
        let manija = vm.bucle().manija();
        let manija_tarea = manija.clone();
        let flujos_tarea = Arc::clone(&flujos);
        manija.runtime().spawn(async move {
            let resultado = tokio::task::spawn_blocking(move || {
                recibir_bytes(&flujos_tarea, id, maximo as usize).map_err(mensaje_de_fallo)
            })
            .await
            .unwrap_or_else(|causa| Err(format!("la tarea de recepción no pudo completarse: {causa}")));
            manija_tarea.enviar(Mensaje::TareaLista {
                id: id_tarea,
                resultado: resultado.map(CargaNativa::Bytes),
            });
        });

        Ok(Valor::TareaNativa(Rc::new(maquina_virtual::EstadoTareaNativa { id: id_tarea })))
    })
}

// =====================================================================
// ServidorSocket: instancia, rutas de configuración y escucha
// =====================================================================

type RegistroServidores = Rc<RefCell<HashMap<u64, Rc<DatosInstanciaNativa>>>>;
type RegistroBanderas = Rc<RefCell<HashMap<u64, Arc<AtomicBool>>>>;

fn nuevo_servidor_socket() -> Valor {
    let mut datos = indexmap::IndexMap::new();
    datos.insert("activo".to_string(), Valor::Log(false));
    datos.insert("puerto".to_string(), Valor::Nulo);
    datos.insert("manejador".to_string(), Valor::Nulo);
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_SERVIDOR),
        datos: RefCell::new(datos),
    }))
}

fn constructor_servidor(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    exigir_aridad("nuevo ServidorSocket", argumentos, 0)?;
    Ok(nuevo_servidor_socket())
}

fn metodo_al_conectar(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ServidorSocket.al_conectar";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let instancia = receptor_con_tipo(F, argumentos, TIPO_SERVIDOR)?;
    let manejador = match argumentos.get(1) {
        Some(valor @ Valor::Funcion(..)) => valor.clone(),
        Some(otro) => {
            return Err(error(
                "E0406",
                format!(
                    "'{F}' espera una función definida como manejador, pero recibió '{}'",
                    otro.nombre_tipo()
                ),
            ));
        }
        None => return Err(error("E0210", format!("'{F}' necesita al menos 1 argumento"))),
    };
    instancia.datos.borrow_mut().insert("manejador".to_string(), manejador);
    Ok(Valor::Nulo)
}

fn metodo_servidor_puerto(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ServidorSocket.puerto";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let instancia = receptor_con_tipo(F, argumentos, TIPO_SERVIDOR)?;
    Ok(instancia.datos.borrow().get("puerto").cloned().unwrap_or(Valor::Nulo))
}

fn metodo_servidor_esta_escuchando(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ServidorSocket.esta_escuchando";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let instancia = receptor_con_tipo(F, argumentos, TIPO_SERVIDOR)?;
    let activo = matches!(instancia.datos.borrow().get("activo"), Some(Valor::Log(true)));
    Ok(Valor::Log(activo))
}

fn despachar_conexion(
    vm: &mut Vm,
    id_recurso: u64,
    registro_servidores: &RegistroServidores,
    flujos: &RegistroFlujos,
    datos: CargaNativa,
) -> CargaNativa {
    let CargaNativa::Entero(id_conexion) = datos else {
        return CargaNativa::Nula;
    };
    let id_conexion = id_conexion as u64;

    let Some(instancia_servidor) = registro_servidores.borrow().get(&id_recurso).cloned() else {
        cerrar_conexion(flujos, id_conexion);
        return CargaNativa::Nula;
    };
    let manejador = instancia_servidor
        .datos
        .borrow()
        .get("manejador")
        .cloned()
        .unwrap_or(Valor::Nulo);
    let Valor::Funcion(funcion, entorno) = manejador else {
        cerrar_conexion(flujos, id_conexion);
        return CargaNativa::Nula;
    };

    let conexion_valor = instancia_con_id(TIPO_CONEXION, id_conexion, TIEMPO_ESPERA_DEFECTO);
    let _ = vm.llamar_funcion(&funcion, &entorno, vec![conexion_valor], None, None);
    // Limpieza automática: el modelo es "un manejador por conexión", sin más
    // eventos después de que retorna. Si el manejador ya cerró la conexión
    // explícitamente, esto es un no-op (la clave ya no está en el mapa).
    cerrar_conexion(flujos, id_conexion);
    CargaNativa::Nula
}

fn metodo_escuchar_servidor(
    guardian: &Rc<GuardianPermisos>,
    registro_servidores: &RegistroServidores,
    banderas: &RegistroBanderas,
    flujos: &RegistroFlujos,
    contador: &Arc<AtomicU64>,
) -> maquina_virtual::FuncionNativaConVm {
    const F: &str = "ServidorSocket.escuchar";
    let guardian = Rc::clone(guardian);
    let registro_servidores = Rc::clone(registro_servidores);
    let banderas = Rc::clone(banderas);
    let flujos = Arc::clone(flujos);
    let contador = Arc::clone(contador);
    Box::new(move |vm, argumentos| {
        exigir_aridad(F, &argumentos[1..], 1)?;
        let instancia = receptor_con_tipo(F, argumentos, TIPO_SERVIDOR)?;
        let puerto_solicitado = arg_entero(F, argumentos, 1)?;
        if !(0..=65535).contains(&puerto_solicitado) {
            return Err(error(
                "E0406",
                format!("'{F}' espera un puerto entre 0 y 65535, pero recibió {puerto_solicitado}"),
            ));
        }
        let puerto = puerto_solicitado as u16;
        guardian.verificar_red_servidor(puerto).map_err(permiso_denegado)?;

        let ya_activo = matches!(instancia.datos.borrow().get("activo"), Some(Valor::Log(true)));
        if ya_activo {
            return Err(error_socket(format!("'{F}': el servidor ya está escuchando")));
        }

        let escucha = TcpListener::bind(("0.0.0.0", puerto))
            .map_err(|causa| error_socket(format!("no se pudo escuchar en el puerto {puerto}: {causa}")))?;
        escucha
            .set_nonblocking(true)
            .map_err(|causa| error_socket(format!("no se pudo configurar el servidor: {causa}")))?;
        let puerto_real = escucha
            .local_addr()
            .map_err(|causa| error_socket(format!("no se pudo leer el puerto del servidor: {causa}")))?
            .port();

        let id_servidor = vm.bucle().nuevo_id();
        registro_servidores.borrow_mut().insert(id_servidor, Rc::clone(&instancia));
        let bandera = Arc::new(AtomicBool::new(true));
        banderas.borrow_mut().insert(id_servidor, Arc::clone(&bandera));

        {
            let mut datos = instancia.datos.borrow_mut();
            datos.insert("activo".to_string(), Valor::Log(true));
            datos.insert("puerto".to_string(), Valor::Entero(i64::from(puerto_real)));
            datos.insert("id_interno".to_string(), Valor::Entero(id_servidor as i64));
        }

        vm.registrar_despachador(SERVICIO, {
            let registro_servidores = Rc::clone(&registro_servidores);
            let flujos = Arc::clone(&flujos);
            move |vm, id_recurso, datos| despachar_conexion(vm, id_recurso, &registro_servidores, &flujos, datos)
        });
        vm.bucle().registrar_trabajo_activo();

        let manija = vm.bucle().manija();
        let flujos_hilo = Arc::clone(&flujos);
        let contador_hilo = Arc::clone(&contador);
        thread::spawn(move || {
            bucle_aceptacion_socket(escucha, id_servidor, manija, bandera, flujos_hilo, contador_hilo)
        });

        Ok(Valor::Nulo)
    })
}

fn bucle_aceptacion_socket(
    escucha: TcpListener,
    id_servidor: u64,
    manija: ManijaBucle,
    bandera: Arc<AtomicBool>,
    flujos: RegistroFlujos,
    contador: Arc<AtomicU64>,
) {
    while bandera.load(Ordering::Relaxed) {
        match escucha.accept() {
            Ok((flujo, _)) => {
                // El socket aceptado puede heredar el modo no bloqueante del
                // `TcpListener` en algunas plataformas: se fuerza a
                // bloqueante para que `recibir_linea`/`recibir_bits` esperen
                // datos en vez de fallar con `WouldBlock` de inmediato.
                let _ = flujo.set_nonblocking(false);
                let Ok(conexion) = ConexionAbierta::desde(flujo) else {
                    continue;
                };
                let id_conexion = siguiente_id(&contador);
                if insertar_conexion(&flujos, id_conexion, conexion).is_err() {
                    continue;
                }
                let (respuesta_tx, _respuesta_rx) = std::sync::mpsc::channel();
                manija.enviar(Mensaje::Solicitud {
                    servicio: SERVICIO.to_string(),
                    id_recurso: id_servidor,
                    datos: CargaNativa::Entero(id_conexion as i64),
                    respuesta: respuesta_tx,
                });
            }
            Err(fallo) if fallo.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(_) => break,
        }
    }
}

fn metodo_detener_servidor(banderas: &RegistroBanderas, registro_servidores: &RegistroServidores) -> maquina_virtual::FuncionNativaConVm {
    const F: &str = "ServidorSocket.detener";
    let banderas = Rc::clone(banderas);
    let registro_servidores = Rc::clone(registro_servidores);
    Box::new(move |vm, argumentos| {
        exigir_aridad(F, &argumentos[1..], 0)?;
        let instancia = receptor_con_tipo(F, argumentos, TIPO_SERVIDOR)?;
        let id = match instancia.datos.borrow().get("id_interno") {
            Some(Valor::Entero(id)) => Some(*id as u64),
            _ => None,
        };
        let activo = matches!(instancia.datos.borrow().get("activo"), Some(Valor::Log(true)));
        if !activo {
            return Ok(Valor::Nulo);
        }
        instancia.datos.borrow_mut().insert("activo".to_string(), Valor::Log(false));
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

// =====================================================================
// Registro
// =====================================================================

pub fn registrar(registro: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    let flujos: RegistroFlujos = Arc::new(Mutex::new(HashMap::new()));
    let contador: Arc<AtomicU64> = Arc::new(AtomicU64::new(1));
    let registro_servidores: RegistroServidores = Rc::new(RefCell::new(HashMap::new()));
    let banderas: RegistroBanderas = Rc::new(RefCell::new(HashMap::new()));

    // ----- Socket (cliente) -----
    registro.registrar_modulo("socket");
    registro.registrar_funcion("socket.constructor", Box::new(constructor));
    registro.registrar_funcion(
        &format!("{TIPO_SOCKET}.fijar_tiempo_espera"),
        Box::new(metodo_fijar_tiempo_espera(TIPO_SOCKET, &flujos)),
    );
    registro.registrar_funcion(
        &format!("{TIPO_SOCKET}.conectar"),
        Box::new(conectar_sincrono(guardian, &flujos, &contador)),
    );
    registro.registrar_funcion_con_vm(
        &format!("{TIPO_SOCKET}.conectar_asincrono"),
        conectar_asincrono(guardian, &flujos, &contador),
    );
    registro.registrar_funcion_con_vm(
        &format!("{TIPO_SOCKET}.recibir_bits_asincrono"),
        metodo_recibir_bits_asincrono(&flujos),
    );

    // ----- Métodos comunes a Socket y ConexionSocket -----
    for tipo in [TIPO_SOCKET, TIPO_CONEXION] {
        registro.registrar_funcion(&format!("{tipo}.enviar_texto"), Box::new(metodo_enviar_texto(tipo, &flujos)));
        registro.registrar_funcion(&format!("{tipo}.enviar_bits"), Box::new(metodo_enviar_bits(tipo, &flujos)));
        registro.registrar_funcion(&format!("{tipo}.recibir_bits"), Box::new(metodo_recibir_bits(tipo, &flujos)));
        registro.registrar_funcion(&format!("{tipo}.recibir_texto"), Box::new(metodo_recibir_texto(tipo, &flujos)));
        registro.registrar_funcion(&format!("{tipo}.recibir_linea"), Box::new(metodo_recibir_linea(tipo, &flujos)));
        registro.registrar_funcion(&format!("{tipo}.cerrar"), Box::new(metodo_cerrar(tipo, &flujos)));
    }
    registro.registrar_funcion(
        &format!("{TIPO_CONEXION}.fijar_tiempo_espera"),
        Box::new(metodo_fijar_tiempo_espera(TIPO_CONEXION, &flujos)),
    );

    // ----- ServidorSocket -----
    registro.registrar_modulo("servidorsocket");
    registro.registrar_funcion("servidorsocket.constructor", Box::new(constructor_servidor));
    registro.registrar_funcion(&format!("{TIPO_SERVIDOR}.al_conectar"), Box::new(metodo_al_conectar));
    registro.registrar_funcion(&format!("{TIPO_SERVIDOR}.puerto"), Box::new(metodo_servidor_puerto));
    registro.registrar_funcion(
        &format!("{TIPO_SERVIDOR}.esta_escuchando"),
        Box::new(metodo_servidor_esta_escuchando),
    );
    registro.registrar_funcion_con_vm(
        &format!("{TIPO_SERVIDOR}.escuchar"),
        metodo_escuchar_servidor(guardian, &registro_servidores, &banderas, &flujos, &contador),
    );
    registro.registrar_funcion_con_vm(
        &format!("{TIPO_SERVIDOR}.detener"),
        metodo_detener_servidor(&banderas, &registro_servidores),
    );
}
