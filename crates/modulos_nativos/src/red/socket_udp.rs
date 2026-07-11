//! Objeto `SocketUdp`: datagramas UDP con transferencia binaria directa vía
//! `Bits`. Mismo principio que [`super::socket`]: el `std::net::UdpSocket`
//! real nunca vive en un `Valor` (no es `Send`); vive en un registro
//! compartido entre hilos por identificador, con un candado por socket para
//! que una recepción lenta en un `SocketUdp` no bloquee a los demás.
//!
//! - `nuevo SocketUdp()` + `fijar_tiempo_espera(seg)`: plantilla, sin
//!   enlazar todavía.
//! - `enlazar(puerto)`: reserva un puerto local para recibir (necesita
//!   permiso de servidor); mutación en el propio receptor, sin threading de
//!   por medio, así que no hace falta el patrón "instancia nueva" de
//!   `Socket` (TCP).
//! - `enviar_a(anfitrion, puerto, datos)`: si el socket aún no se enlazó,
//!   se enlaza implícitamente a un puerto efímero (como el bind automático
//!   de un `connect` de TCP): enviar no expone ningún puerto elegido por el
//!   usuario, así que solo pide permiso de cliente sobre el destino.
//! - `recibir()` / `recibir_asincrono()`: devuelven un `jsn`
//!   `{origen, puerto, datos}` con `datos` como `Bits`.
//! - `esta_enlazado()` y `direccion_local()` permiten observar el ciclo de
//!   vida; `permitir_difusion(log)` habilita o deshabilita broadcast.

use std::cell::RefCell;
use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use indexmap::IndexMap;
use maquina_virtual::bucle_eventos::CargaNativa;
use maquina_virtual::{DatosInstanciaNativa, Fallo, Mensaje, RegistroNativos, Valor};
use runtime::GuardianPermisos;

use crate::util::{arg_entero, arg_texto, error, exigir_aridad};

use super::permiso_denegado;

const TIPO_SOCKET_UDP: &str = "SocketUdp";
const TIEMPO_ESPERA_DEFECTO: i64 = 30;
const TAMANO_DATAGRAMA_MAXIMO: usize = 65536;

fn error_socket(mensaje: impl Into<String>) -> Fallo {
    error("E0705", mensaje.into())
}

fn mensaje_de_fallo(fallo: Fallo) -> String {
    match fallo {
        Fallo::Excepcion(datos) => datos.mensaje,
        Fallo::Error(error) => error.mensaje,
    }
}

/// Registro de sockets UDP enlazados, por identificador. Igual que en
/// `socket.rs`: el `Mutex` externo solo protege el mapa; cada socket tiene
/// su propio `Mutex` interno para la E/S real.
type RegistroSockets = Arc<Mutex<HashMap<u64, Arc<Mutex<UdpSocket>>>>>;

fn siguiente_id(contador: &Arc<AtomicU64>) -> u64 {
    contador.fetch_add(1, Ordering::Relaxed)
}

fn insertar_socket(registro: &RegistroSockets, id: u64, socket: UdpSocket) -> Result<(), Fallo> {
    registro
        .lock()
        .map_err(|_| error_socket("el registro de sockets UDP está dañado"))?
        .insert(id, Arc::new(Mutex::new(socket)));
    Ok(())
}

fn obtener_socket(registro: &RegistroSockets, id: u64) -> Result<Arc<Mutex<UdpSocket>>, Fallo> {
    registro
        .lock()
        .map_err(|_| error_socket("el registro de sockets UDP está dañado"))?
        .get(&id)
        .cloned()
        .ok_or_else(|| error_socket("el SocketUdp ya está cerrado o nunca se enlazó"))
}

fn cerrar_socket(registro: &RegistroSockets, id: u64) {
    if let Ok(mut mapa) = registro.lock() {
        mapa.remove(&id);
    }
}

fn resolver_direccion(funcion: &str, anfitrion: &str, puerto: u16) -> Result<SocketAddr, Fallo> {
    (anfitrion, puerto)
        .to_socket_addrs()
        .map_err(|causa| error_socket(format!("'{funcion}' no pudo resolver '{anfitrion}:{puerto}': {causa}")))?
        .next()
        .ok_or_else(|| error_socket(format!("'{funcion}' no encontró ninguna dirección para '{anfitrion}:{puerto}'")))
}

fn fijar_tiempo_espera_socket(socket: &UdpSocket, segundos: u64) -> std::io::Result<()> {
    let duracion = if segundos == 0 { None } else { Some(Duration::from_secs(segundos)) };
    socket.set_read_timeout(duracion)
}

// =====================================================================
// Instancia, configuración y enlace
// =====================================================================

fn nuevo_socket_udp() -> Valor {
    let mut datos = IndexMap::new();
    datos.insert("id".to_string(), Valor::Nulo);
    datos.insert("tiempo_espera".to_string(), Valor::Entero(TIEMPO_ESPERA_DEFECTO));
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_SOCKET_UDP),
        datos: RefCell::new(datos),
    }))
}

fn constructor(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    exigir_aridad("nuevo SocketUdp", argumentos, 0)?;
    Ok(nuevo_socket_udp())
}

fn receptor(funcion: &str, argumentos: &[Valor]) -> Result<Rc<DatosInstanciaNativa>, Fallo> {
    match argumentos.first() {
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == TIPO_SOCKET_UDP => Ok(Rc::clone(instancia)),
        Some(otro) => Err(error(
            "E0406",
            format!("'{funcion}' esperaba un SocketUdp, pero recibió '{}'", otro.nombre_tipo()),
        )),
        None => Err(error("E0210", format!("'{funcion}' necesita el receptor"))),
    }
}

fn tiempo_espera_de(instancia: &DatosInstanciaNativa) -> i64 {
    match instancia.datos.borrow().get("tiempo_espera") {
        Some(Valor::Entero(segundos)) => (*segundos).max(0),
        _ => TIEMPO_ESPERA_DEFECTO,
    }
}

fn id_de_instancia(instancia: &DatosInstanciaNativa) -> Option<u64> {
    match instancia.datos.borrow().get("id") {
        Some(Valor::Entero(id)) if *id >= 0 => Some(*id as u64),
        _ => None,
    }
}

fn metodo_fijar_tiempo_espera(registro: &RegistroSockets) -> impl Fn(&[Valor]) -> Result<Valor, Fallo> + 'static {
    const F: &str = "SocketUdp.fijar_tiempo_espera";
    let registro = Arc::clone(registro);
    move |argumentos| {
        exigir_aridad(F, &argumentos[1..], 1)?;
        let instancia = receptor(F, argumentos)?;
        let segundos = match argumentos.get(1) {
            Some(Valor::Entero(segundos)) if *segundos >= 0 => *segundos,
            Some(otro) => {
                return Err(error(
                    "E0406",
                    format!("'{F}' espera un entero positivo de segundos, pero recibió '{}'", otro.nombre_tipo()),
                ));
            }
            None => {
                return Err(error("E0210", format!("'{F}' necesita al menos 1 argumento")));
            }
        };
        instancia
            .datos
            .borrow_mut()
            .insert("tiempo_espera".to_string(), Valor::Entero(segundos));
        if let Some(id) = id_de_instancia(&instancia) {
            let socket = obtener_socket(&registro, id)?;
            let socket = socket.lock().map_err(|_| error_socket("el SocketUdp está dañado"))?;
            fijar_tiempo_espera_socket(&socket, segundos as u64)
                .map_err(|causa| error_socket(format!("no se pudo fijar el tiempo de espera: {causa}")))?;
        }
        Ok(Valor::Nulo)
    }
}

/// Enlaza el socket subyacente de `instancia` si todavía no lo tiene
/// (`enlazar` explícito, o implícito la primera vez que se envía algo).
fn asegurar_enlazado(
    instancia: &DatosInstanciaNativa,
    registro: &RegistroSockets,
    contador: &Arc<AtomicU64>,
    direccion_local: SocketAddr,
) -> Result<u64, Fallo> {
    if let Some(id) = id_de_instancia(instancia) {
        return Ok(id);
    }
    let socket = UdpSocket::bind(direccion_local).map_err(|causa| error_socket(format!("no se pudo enlazar el SocketUdp: {causa}")))?;
    let tiempo_espera = tiempo_espera_de(instancia);
    fijar_tiempo_espera_socket(&socket, tiempo_espera as u64)
        .map_err(|causa| error_socket(format!("no se pudo fijar el tiempo de espera: {causa}")))?;
    let id = siguiente_id(contador);
    insertar_socket(registro, id, socket)?;
    instancia.datos.borrow_mut().insert("id".to_string(), Valor::Entero(id as i64));
    Ok(id)
}

fn metodo_enlazar(
    guardian: &Rc<GuardianPermisos>,
    registro: &RegistroSockets,
    contador: &Arc<AtomicU64>,
) -> impl Fn(&[Valor]) -> Result<Valor, Fallo> + 'static {
    const F: &str = "SocketUdp.enlazar";
    let guardian = Rc::clone(guardian);
    let registro = Arc::clone(registro);
    let contador = Arc::clone(contador);
    move |argumentos| {
        exigir_aridad(F, &argumentos[1..], 1)?;
        let instancia = receptor(F, argumentos)?;
        let puerto = arg_entero(F, argumentos, 1)?;
        if !(0..=65535).contains(&puerto) {
            return Err(error(
                "E0406",
                format!("'{F}' espera un puerto entre 0 y 65535, pero recibió {puerto}"),
            ));
        }
        let puerto = puerto as u16;
        guardian.verificar_red_servidor(puerto).map_err(permiso_denegado)?;
        if id_de_instancia(&instancia).is_some() {
            return Err(error_socket(format!("'{F}': el SocketUdp ya está enlazado")));
        }
        asegurar_enlazado(&instancia, &registro, &contador, SocketAddr::from(([0, 0, 0, 0], puerto)))?;
        Ok(Valor::Nulo)
    }
}

fn metodo_puerto(registro: &RegistroSockets) -> impl Fn(&[Valor]) -> Result<Valor, Fallo> + 'static {
    const F: &str = "SocketUdp.puerto";
    let registro = Arc::clone(registro);
    move |argumentos| {
        exigir_aridad(F, &argumentos[1..], 0)?;
        let instancia = receptor(F, argumentos)?;
        let Some(id) = id_de_instancia(&instancia) else {
            return Ok(Valor::Nulo);
        };
        let socket = obtener_socket(&registro, id)?;
        let socket = socket.lock().map_err(|_| error_socket("el SocketUdp está dañado"))?;
        let puerto = socket
            .local_addr()
            .map_err(|causa| error_socket(format!("no se pudo leer el puerto local: {causa}")))?
            .port();
        Ok(Valor::Entero(i64::from(puerto)))
    }
}

fn metodo_esta_enlazado(registro: &RegistroSockets) -> impl Fn(&[Valor]) -> Result<Valor, Fallo> + 'static {
    const F: &str = "SocketUdp.esta_enlazado";
    let registro = Arc::clone(registro);
    move |argumentos| {
        exigir_aridad(F, &argumentos[1..], 0)?;
        let instancia = receptor(F, argumentos)?;
        let Some(id) = id_de_instancia(&instancia) else {
            return Ok(Valor::Log(false));
        };
        let enlazado = registro
            .lock()
            .map_err(|_| error_socket("el registro de sockets UDP está dañado"))?
            .contains_key(&id);
        Ok(Valor::Log(enlazado))
    }
}

fn metodo_direccion_local(registro: &RegistroSockets) -> impl Fn(&[Valor]) -> Result<Valor, Fallo> + 'static {
    const F: &str = "SocketUdp.direccion_local";
    let registro = Arc::clone(registro);
    move |argumentos| {
        exigir_aridad(F, &argumentos[1..], 0)?;
        let instancia = receptor(F, argumentos)?;
        let id = id_de_instancia(&instancia).ok_or_else(|| error_socket(format!("'{F}': el SocketUdp no está enlazado")))?;
        let socket = obtener_socket(&registro, id)?;
        let direccion = socket
            .lock()
            .map_err(|_| error_socket("el SocketUdp está dañado"))?
            .local_addr()
            .map_err(|causa| error_socket(format!("no se pudo leer la dirección local: {causa}")))?;
        let mut datos = IndexMap::new();
        datos.insert("anfitrion".to_string(), Valor::texto(direccion.ip().to_string()));
        datos.insert("puerto".to_string(), Valor::Entero(i64::from(direccion.port())));
        Ok(Valor::jsn(datos))
    }
}

fn metodo_permitir_difusion(registro: &RegistroSockets) -> impl Fn(&[Valor]) -> Result<Valor, Fallo> + 'static {
    const F: &str = "SocketUdp.permitir_difusion";
    let registro = Arc::clone(registro);
    move |argumentos| {
        exigir_aridad(F, &argumentos[1..], 1)?;
        let instancia = receptor(F, argumentos)?;
        let habilitado = match argumentos.get(1) {
            Some(Valor::Log(valor)) => *valor,
            Some(otro) => {
                return Err(error(
                    "E0406",
                    format!("'{F}' espera un lógico, pero recibió '{}'", otro.nombre_tipo()),
                ));
            }
            None => return Err(error("E0210", format!("'{F}' necesita 1 argumento"))),
        };
        let id = id_de_instancia(&instancia).ok_or_else(|| error_socket(format!("'{F}': el SocketUdp no está enlazado")))?;
        let socket = obtener_socket(&registro, id)?;
        socket
            .lock()
            .map_err(|_| error_socket("el SocketUdp está dañado"))?
            .set_broadcast(habilitado)
            .map_err(|causa| error_socket(format!("no se pudo configurar la difusión: {causa}")))?;
        Ok(Valor::Nulo)
    }
}

// =====================================================================
// enviar_a / recibir (síncronos)
// =====================================================================

fn bytes_de_datos(funcion: &str, valor: &Valor) -> Result<Vec<u8>, Fallo> {
    match valor {
        Valor::Texto(texto) => Ok(texto.as_bytes().to_vec()),
        otro => maquina_virtual::bytes_de_bits(otro).ok_or_else(|| {
            error(
                "E0406",
                format!("'{funcion}' espera texto o Bits en 'datos', pero recibió '{}'", otro.nombre_tipo()),
            )
        }),
    }
}

fn metodo_enviar_a(
    guardian: &Rc<GuardianPermisos>,
    registro: &RegistroSockets,
    contador: &Arc<AtomicU64>,
) -> impl Fn(&[Valor]) -> Result<Valor, Fallo> + 'static {
    const F: &str = "SocketUdp.enviar_a";
    let guardian = Rc::clone(guardian);
    let registro = Arc::clone(registro);
    let contador = Arc::clone(contador);
    move |argumentos| {
        exigir_aridad(F, &argumentos[1..], 3)?;
        let instancia = receptor(F, argumentos)?;
        let anfitrion = arg_texto(F, argumentos, 1)?.to_string();
        let puerto = arg_entero(F, argumentos, 2)?;
        if !(0..=65535).contains(&puerto) {
            return Err(error(
                "E0406",
                format!("'{F}' espera un puerto entre 0 y 65535, pero recibió {puerto}"),
            ));
        }
        let puerto = puerto as u16;
        guardian.verificar_red_cliente(&anfitrion, puerto).map_err(permiso_denegado)?;
        let bytes = bytes_de_datos(F, &argumentos[3])?;

        let id = asegurar_enlazado(&instancia, &registro, &contador, SocketAddr::from(([0, 0, 0, 0], 0)))?;
        let direccion = resolver_direccion(F, &anfitrion, puerto)?;
        let socket = obtener_socket(&registro, id)?;
        let socket = socket.lock().map_err(|_| error_socket("el SocketUdp está dañado"))?;
        socket
            .send_to(&bytes, direccion)
            .map_err(|causa| error_socket(format!("no se pudo enviar a '{anfitrion}:{puerto}': {causa}")))?;
        Ok(Valor::Nulo)
    }
}

fn datagrama_a_jsn(origen: SocketAddr, bytes: Vec<u8>) -> Valor {
    let mut datos = IndexMap::new();
    datos.insert("origen".to_string(), Valor::texto(origen.ip().to_string()));
    datos.insert("puerto".to_string(), Valor::Entero(i64::from(origen.port())));
    datos.insert("datos".to_string(), maquina_virtual::instancia_bits_desde(&bytes));
    Valor::jsn(datos)
}

fn recibir_datagrama(socket: &UdpSocket) -> Result<(SocketAddr, Vec<u8>), Fallo> {
    let mut buffer = vec![0u8; TAMANO_DATAGRAMA_MAXIMO];
    let (leidos, origen) = socket
        .recv_from(&mut buffer)
        .map_err(|causa| error_socket(format!("no se pudo recibir: {causa}")))?;
    buffer.truncate(leidos);
    Ok((origen, buffer))
}

fn metodo_recibir(registro: &RegistroSockets) -> impl Fn(&[Valor]) -> Result<Valor, Fallo> + 'static {
    const F: &str = "SocketUdp.recibir";
    let registro = Arc::clone(registro);
    move |argumentos| {
        exigir_aridad(F, &argumentos[1..], 0)?;
        let instancia = receptor(F, argumentos)?;
        let id = id_de_instancia(&instancia)
            .ok_or_else(|| error_socket(format!("'{F}': el SocketUdp no está enlazado (llama a 'enlazar' primero)")))?;
        let socket = obtener_socket(&registro, id)?;
        let socket = socket.lock().map_err(|_| error_socket("el SocketUdp está dañado"))?;
        let (origen, bytes) = recibir_datagrama(&socket)?;
        Ok(datagrama_a_jsn(origen, bytes))
    }
}

fn metodo_recibir_asincrono(registro: &RegistroSockets) -> maquina_virtual::FuncionNativaConVm {
    const F: &str = "SocketUdp.recibir_asincrono";
    let registro = Arc::clone(registro);
    Box::new(move |vm, argumentos| {
        exigir_aridad(F, &argumentos[1..], 0)?;
        let instancia = receptor(F, argumentos)?;
        let id = id_de_instancia(&instancia)
            .ok_or_else(|| error_socket(format!("'{F}': el SocketUdp no está enlazado (llama a 'enlazar' primero)")))?;

        let id_tarea = vm.bucle().nuevo_id();
        let manija = vm.bucle().manija();
        let manija_tarea = manija.clone();
        let registro_tarea = Arc::clone(&registro);
        manija.runtime().spawn(async move {
            let resultado = tokio::task::spawn_blocking(move || -> Result<(SocketAddr, Vec<u8>), String> {
                let socket = obtener_socket(&registro_tarea, id).map_err(mensaje_de_fallo)?;
                let socket = socket.lock().map_err(|_| "el SocketUdp está dañado".to_string())?;
                recibir_datagrama(&socket).map_err(mensaje_de_fallo)
            })
            .await
            .unwrap_or_else(|causa| Err(format!("la tarea de recepción no pudo completarse: {causa}")));

            let carga = resultado.map(|(origen, bytes)| {
                CargaNativa::Mapa(vec![
                    ("origen".to_string(), CargaNativa::Texto(origen.ip().to_string())),
                    ("puerto".to_string(), CargaNativa::Entero(i64::from(origen.port()))),
                    ("datos".to_string(), CargaNativa::Bytes(bytes)),
                ])
            });
            manija_tarea.enviar(Mensaje::TareaLista {
                id: id_tarea,
                resultado: carga,
            });
        });

        Ok(Valor::TareaNativa(Rc::new(maquina_virtual::EstadoTareaNativa { id: id_tarea })))
    })
}

fn metodo_cerrar(registro: &RegistroSockets) -> impl Fn(&[Valor]) -> Result<Valor, Fallo> + 'static {
    const F: &str = "SocketUdp.cerrar";
    let registro = Arc::clone(registro);
    move |argumentos| {
        exigir_aridad(F, &argumentos[1..], 0)?;
        let instancia = receptor(F, argumentos)?;
        if let Some(id) = id_de_instancia(&instancia) {
            cerrar_socket(&registro, id);
        }
        instancia.datos.borrow_mut().insert("id".to_string(), Valor::Nulo);
        Ok(Valor::Nulo)
    }
}

// =====================================================================
// Registro
// =====================================================================

pub fn registrar(registro_nativos: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    let registro: RegistroSockets = Arc::new(Mutex::new(HashMap::new()));
    let contador: Arc<AtomicU64> = Arc::new(AtomicU64::new(1));

    registro_nativos.registrar_modulo("socketudp");
    registro_nativos.registrar_funcion("socketudp.constructor", Box::new(constructor));
    registro_nativos.registrar_funcion(
        &format!("{TIPO_SOCKET_UDP}.fijar_tiempo_espera"),
        Box::new(metodo_fijar_tiempo_espera(&registro)),
    );
    registro_nativos.registrar_funcion(
        &format!("{TIPO_SOCKET_UDP}.enlazar"),
        Box::new(metodo_enlazar(guardian, &registro, &contador)),
    );
    registro_nativos.registrar_funcion(&format!("{TIPO_SOCKET_UDP}.puerto"), Box::new(metodo_puerto(&registro)));
    registro_nativos.registrar_funcion(
        &format!("{TIPO_SOCKET_UDP}.esta_enlazado"),
        Box::new(metodo_esta_enlazado(&registro)),
    );
    registro_nativos.registrar_funcion(
        &format!("{TIPO_SOCKET_UDP}.direccion_local"),
        Box::new(metodo_direccion_local(&registro)),
    );
    registro_nativos.registrar_funcion(
        &format!("{TIPO_SOCKET_UDP}.permitir_difusion"),
        Box::new(metodo_permitir_difusion(&registro)),
    );
    registro_nativos.registrar_funcion(
        &format!("{TIPO_SOCKET_UDP}.enviar_a"),
        Box::new(metodo_enviar_a(guardian, &registro, &contador)),
    );
    registro_nativos.registrar_funcion(&format!("{TIPO_SOCKET_UDP}.recibir"), Box::new(metodo_recibir(&registro)));
    registro_nativos.registrar_funcion_con_vm(&format!("{TIPO_SOCKET_UDP}.recibir_asincrono"), metodo_recibir_asincrono(&registro));
    registro_nativos.registrar_funcion(&format!("{TIPO_SOCKET_UDP}.cerrar"), Box::new(metodo_cerrar(&registro)));
}
