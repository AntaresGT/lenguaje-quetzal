//! Objetos `ClienteRs` y `ServidorRs`: RedSocket (equivalente en español de
//! WebSocket) con transferencia binaria directa vía `Bits`, sobre el mismo
//! runtime tokio del bucle de eventos (a diferencia de
//! `socket.rs`/`socket_udp.rs`, `tokio-tungstenite` es async-nativo: no hay
//! una variante "bloqueante" de la librería, así que aquí *todos* los
//! métodos necesitan acceso a la VM para poder bombear el runtime con
//! `Runtime::block_on` desde el hilo de la VM, tal como las demás variantes
//! síncronas de este módulo bloquean ese hilo).
//!
//! El esquema de la URL sigue siendo `ws://`/`wss://` porque lo define el
//! protocolo estándar (RFC 6455), no nuestra API: solo el nombre de los
//! tipos de Quetzal cambia a español.
//!
//! - `ClienteRs`: `conectar(url)` (permiso de cliente), `enviar_texto`,
//!   `enviar_bits`, `recibir()` (texto, `Bits` o `nulo` si el otro lado
//!   cerró la conexión), `cerrar()`.
//! - `ServidorRs`: `escuchar(puerto)` (permiso de servidor), con dos
//!   manejadores por referencia: `al_conectar(manejador)` se invoca una vez
//!   por conexión nueva con `manejador(ConexionRs conexion)`; `al_mensaje`
//!   se invoca por cada mensaje recibido con
//!   `manejador(ConexionRs conexion, mensaje)` (`mensaje` es texto o
//!   `Bits`). `ConexionRs` comparte `enviar_texto`/`enviar_bits`/`cerrar`
//!   con `ClienteRs`. `detener()` para poder testear.
//!
//! Arquitectura: cada conexión (cliente o aceptada por un servidor) se
//! separa en mitad de escritura y mitad de lectura (`futures_util::split`).
//! La escritura vive en un registro compartido por identificador
//! (`Arc<tokio::sync::Mutex<..>>`, se puede mantener a través de un
//! `.await` con seguridad) para que los métodos `enviar_*`/`cerrar` la
//! encuentren desde cualquier llamada nativa. La lectura de `ClienteRs` se
//! guarda igual, para `recibir()` "a demanda"; la de una conexión de
//! `ServidorRs` la posee por completo una tarea de fondo que hace
//! `lectura.next().await` en bucle y despacha cada mensaje al hilo de la VM
//! como [`Mensaje::Solicitud`] (mismo patrón reactor de `servidor_http.rs`
//! y `socket.rs`, pero con muchos despachos por conexión en vez de uno).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use maquina_virtual::bucle_eventos::CargaNativa;
use maquina_virtual::{DatosInstanciaNativa, Fallo, ManijaBucle, Mensaje, RegistroNativos, Valor, Vm};
use runtime::GuardianPermisos;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex as MutexAsincronico;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::util::{arg_entero, arg_texto, error, exigir_aridad};

use super::permiso_denegado;

const TIPO_CLIENTE_RS: &str = "ClienteRs";
const TIPO_SERVIDOR_RS: &str = "ServidorRs";
const TIPO_CONEXION_RS: &str = "ConexionRs";

const SERVICIO: &str = "servidor_rs";

fn error_rs(mensaje: impl Into<String>) -> Fallo {
    error("E0706", mensaje.into())
}

// =====================================================================
// Conexión RedSocket: registro de escritura/lectura compartido
// =====================================================================

/// Ambos lados (cliente y servidor) se unifican en el mismo tipo de flujo:
/// el lado servidor envuelve su `TcpStream` normal en `MaybeTlsStream::Plain`
/// antes del *handshake* para que las dos mitades tengan el mismo tipo.
type FlujoRs = WebSocketStream<MaybeTlsStream<TcpStream>>;
type Escritor = SplitSink<FlujoRs, Message>;
type Lector = SplitStream<FlujoRs>;

type RegistroEscritores = Arc<Mutex<HashMap<u64, Arc<MutexAsincronico<Escritor>>>>>;
type RegistroLectores = Arc<Mutex<HashMap<u64, Arc<MutexAsincronico<Lector>>>>>;

fn siguiente_id(contador: &Arc<AtomicU64>) -> u64 {
    contador.fetch_add(1, Ordering::Relaxed)
}

fn insertar_escritor(registro: &RegistroEscritores, id: u64, escritor: Escritor) -> Result<(), Fallo> {
    registro
        .lock()
        .map_err(|_| error_rs("el registro de conexiones RedSocket está dañado"))?
        .insert(id, Arc::new(MutexAsincronico::new(escritor)));
    Ok(())
}

fn insertar_lector(registro: &RegistroLectores, id: u64, lector: Lector) -> Result<(), Fallo> {
    registro
        .lock()
        .map_err(|_| error_rs("el registro de conexiones RedSocket está dañado"))?
        .insert(id, Arc::new(MutexAsincronico::new(lector)));
    Ok(())
}

fn obtener_escritor(registro: &RegistroEscritores, id: u64) -> Result<Arc<MutexAsincronico<Escritor>>, Fallo> {
    registro
        .lock()
        .map_err(|_| error_rs("el registro de conexiones RedSocket está dañado"))?
        .get(&id)
        .cloned()
        .ok_or_else(|| error_rs("la conexión RedSocket ya está cerrada"))
}

fn obtener_lector(registro: &RegistroLectores, id: u64) -> Result<Arc<MutexAsincronico<Lector>>, Fallo> {
    registro
        .lock()
        .map_err(|_| error_rs("el registro de conexiones RedSocket está dañado"))?
        .get(&id)
        .cloned()
        .ok_or_else(|| error_rs("la conexión RedSocket ya está cerrada"))
}

fn quitar_escritor(registro: &RegistroEscritores, id: u64) -> Option<Arc<MutexAsincronico<Escritor>>> {
    registro.lock().ok()?.remove(&id)
}

fn quitar_lector(registro: &RegistroLectores, id: u64) {
    if let Ok(mut mapa) = registro.lock() {
        mapa.remove(&id);
    }
}

/// Resuelve el esquema (`ws`/`wss`), anfitrión y puerto de una URL de
/// RedSocket, para el chequeo de permisos de cliente. El esquema sigue
/// siendo `ws://`/`wss://`: lo define el protocolo (RFC 6455), no esta API.
fn analizar_url_rs(funcion: &str, url_texto: &str) -> Result<(url::Url, String, u16), Fallo> {
    let url = url::Url::parse(url_texto)
        .map_err(|causa| error("E0406", format!("'{funcion}': URL inválida '{url_texto}': {causa}")))?;
    match url.scheme() {
        "ws" | "wss" => {}
        otro => {
            return Err(error(
                "E0406",
                format!("'{funcion}' espera una URL 'ws://' o 'wss://', pero recibió el esquema '{otro}'"),
            ));
        }
    }
    let anfitrion = url
        .host_str()
        .ok_or_else(|| error("E0406", format!("'{funcion}': la URL '{url_texto}' no tiene anfitrión")))?
        .to_string();
    let puerto = url
        .port_or_known_default()
        .ok_or_else(|| error("E0406", format!("'{funcion}': no se pudo determinar el puerto de '{url_texto}'")))?;
    Ok((url, anfitrion, puerto))
}

// =====================================================================
// Instancias: helpers genéricos compartidos por ClienteRs/ConexionRs
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
        _ => Err(error_rs(format!(
            "'{funcion}' recibió una conexión sin identificador válido (¿ya se cerró?)"
        ))),
    }
}

fn instancia_con_id(tipo: &str, id: u64) -> Valor {
    let mut datos = indexmap::IndexMap::new();
    datos.insert("id".to_string(), Valor::Entero(id as i64));
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(tipo),
        datos: RefCell::new(datos),
    }))
}

// =====================================================================
// ClienteRs: conectar
// =====================================================================

fn nuevo_cliente_rs() -> Valor {
    let mut datos = indexmap::IndexMap::new();
    datos.insert("id".to_string(), Valor::Nulo);
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_CLIENTE_RS),
        datos: RefCell::new(datos),
    }))
}

fn constructor_cliente(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    exigir_aridad("nuevo ClienteRs", argumentos, 0)?;
    Ok(nuevo_cliente_rs())
}

fn metodo_conectar(
    guardian: &Rc<GuardianPermisos>,
    escritores: &RegistroEscritores,
    lectores: &RegistroLectores,
    contador: &Arc<AtomicU64>,
) -> maquina_virtual::FuncionNativaConVm {
    const F: &str = "ClienteRs.conectar";
    let guardian = Rc::clone(guardian);
    let escritores = Arc::clone(escritores);
    let lectores = Arc::clone(lectores);
    let contador = Arc::clone(contador);
    Box::new(move |vm, argumentos| {
        exigir_aridad(F, &argumentos[1..], 1)?;
        receptor_con_tipo(F, argumentos, TIPO_CLIENTE_RS)?;
        let url_texto = arg_texto(F, argumentos, 1)?.to_string();
        let (url, anfitrion, puerto) = analizar_url_rs(F, &url_texto)?;
        guardian.verificar_red_cliente(&anfitrion, puerto).map_err(permiso_denegado)?;

        let manija = vm.bucle().manija();
        let flujo = manija
            .runtime()
            .block_on(tokio_tungstenite::connect_async(url.as_str()))
            .map_err(|causa| error_rs(format!("no se pudo conectar a '{url_texto}': {causa}")))?
            .0;
        let (escritura, lectura) = flujo.split();
        let id = siguiente_id(&contador);
        insertar_escritor(&escritores, id, escritura)?;
        insertar_lector(&lectores, id, lectura)?;
        Ok(instancia_con_id(TIPO_CLIENTE_RS, id))
    })
}

// =====================================================================
// Envío / recepción / cierre (comparten identificador y registro)
// =====================================================================

fn metodo_enviar_texto(tipo: &'static str, escritores: &RegistroEscritores) -> maquina_virtual::FuncionNativaConVm {
    let escritores = Arc::clone(escritores);
    Box::new(move |vm, argumentos| {
        let funcion = &format!("{tipo}.enviar_texto");
        exigir_aridad(funcion, &argumentos[1..], 1)?;
        let instancia = receptor_con_tipo(funcion, argumentos, tipo)?;
        let texto = arg_texto(funcion, argumentos, 1)?.to_string();
        let id = id_de_instancia(funcion, &instancia)?;
        let escritor = obtener_escritor(&escritores, id)?;
        vm.bucle()
            .manija()
            .runtime()
            .block_on(async { escritor.lock().await.send(Message::Text(texto.into())).await })
            .map_err(|causa| error_rs(format!("no se pudo enviar: {causa}")))?;
        Ok(Valor::Nulo)
    })
}

fn metodo_enviar_bits(tipo: &'static str, escritores: &RegistroEscritores) -> maquina_virtual::FuncionNativaConVm {
    let escritores = Arc::clone(escritores);
    Box::new(move |vm, argumentos| {
        let funcion = &format!("{tipo}.enviar_bits");
        exigir_aridad(funcion, &argumentos[1..], 1)?;
        let instancia = receptor_con_tipo(funcion, argumentos, tipo)?;
        let bytes = argumentos
            .get(1)
            .and_then(maquina_virtual::bytes_de_bits)
            .ok_or_else(|| error("E0406", format!("'{funcion}' espera un Bits en el argumento 1")))?;
        let id = id_de_instancia(funcion, &instancia)?;
        let escritor = obtener_escritor(&escritores, id)?;
        vm.bucle()
            .manija()
            .runtime()
            .block_on(async { escritor.lock().await.send(Message::Binary(bytes.into())).await })
            .map_err(|causa| error_rs(format!("no se pudo enviar: {causa}")))?;
        Ok(Valor::Nulo)
    })
}

fn metodo_recibir(lectores: &RegistroLectores) -> maquina_virtual::FuncionNativaConVm {
    const F: &str = "ClienteRs.recibir";
    let lectores = Arc::clone(lectores);
    Box::new(move |vm, argumentos| {
        exigir_aridad(F, &argumentos[1..], 0)?;
        let instancia = receptor_con_tipo(F, argumentos, TIPO_CLIENTE_RS)?;
        let id = id_de_instancia(F, &instancia)?;
        let lector = obtener_lector(&lectores, id)?;

        let resultado = vm.bucle().manija().runtime().block_on(async {
            let mut guardia = lector.lock().await;
            loop {
                match guardia.next().await {
                    Some(Ok(Message::Text(texto))) => return Ok(Some(Valor::texto(&texto))),
                    Some(Ok(Message::Binary(bytes))) => {
                        return Ok(Some(maquina_virtual::instancia_bits_desde(&bytes)));
                    }
                    Some(Ok(Message::Close(_))) | None => return Ok(None),
                    Some(Ok(_)) => continue,
                    Some(Err(causa)) => return Err(causa),
                }
            }
        });
        match resultado {
            Ok(Some(valor)) => Ok(valor),
            Ok(None) => Ok(Valor::Nulo),
            Err(causa) => Err(error_rs(format!("no se pudo recibir: {causa}"))),
        }
    })
}

fn cerrar_escritor_async(vm: &mut Vm, escritor: &Arc<MutexAsincronico<Escritor>>) {
    let _ = vm
        .bucle()
        .manija()
        .runtime()
        .block_on(async { escritor.lock().await.close().await });
}

fn metodo_cerrar_cliente(escritores: &RegistroEscritores, lectores: &RegistroLectores) -> maquina_virtual::FuncionNativaConVm {
    const F: &str = "ClienteRs.cerrar";
    let escritores = Arc::clone(escritores);
    let lectores = Arc::clone(lectores);
    Box::new(move |vm, argumentos| {
        exigir_aridad(F, &argumentos[1..], 0)?;
        let instancia = receptor_con_tipo(F, argumentos, TIPO_CLIENTE_RS)?;
        if let Ok(id) = id_de_instancia(F, &instancia) {
            if let Some(escritor) = quitar_escritor(&escritores, id) {
                cerrar_escritor_async(vm, &escritor);
            }
            quitar_lector(&lectores, id);
        }
        instancia.datos.borrow_mut().insert("id".to_string(), Valor::Nulo);
        Ok(Valor::Nulo)
    })
}

fn metodo_cerrar_conexion(escritores: &RegistroEscritores) -> maquina_virtual::FuncionNativaConVm {
    const F: &str = "ConexionRs.cerrar";
    let escritores = Arc::clone(escritores);
    Box::new(move |vm, argumentos| {
        exigir_aridad(F, &argumentos[1..], 0)?;
        let instancia = receptor_con_tipo(F, argumentos, TIPO_CONEXION_RS)?;
        if let Ok(id) = id_de_instancia(F, &instancia)
            && let Some(escritor) = quitar_escritor(&escritores, id)
        {
            cerrar_escritor_async(vm, &escritor);
        }
        instancia.datos.borrow_mut().insert("id".to_string(), Valor::Nulo);
        Ok(Valor::Nulo)
    })
}

// =====================================================================
// ServidorRs: instancia, manejadores y escucha
// =====================================================================

type RegistroServidores = Rc<RefCell<HashMap<u64, Rc<DatosInstanciaNativa>>>>;
type RegistroBanderas = Rc<RefCell<HashMap<u64, Arc<AtomicBool>>>>;

fn nuevo_servidor_rs() -> Valor {
    let mut datos = indexmap::IndexMap::new();
    datos.insert("activo".to_string(), Valor::Log(false));
    datos.insert("puerto".to_string(), Valor::Nulo);
    datos.insert("manejador_conectar".to_string(), Valor::Nulo);
    datos.insert("manejador_mensaje".to_string(), Valor::Nulo);
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_SERVIDOR_RS),
        datos: RefCell::new(datos),
    }))
}

fn constructor_servidor(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    exigir_aridad("nuevo ServidorRs", argumentos, 0)?;
    Ok(nuevo_servidor_rs())
}

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

fn metodo_al_conectar(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ServidorRs.al_conectar";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let instancia = receptor_con_tipo(F, argumentos, TIPO_SERVIDOR_RS)?;
    let manejador = exigir_manejador(F, argumentos.get(1).ok_or_else(|| error("E0210", format!("'{F}' necesita al menos 1 argumento")))?)?;
    instancia.datos.borrow_mut().insert("manejador_conectar".to_string(), manejador);
    Ok(Valor::Nulo)
}

fn metodo_al_mensaje(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ServidorRs.al_mensaje";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let instancia = receptor_con_tipo(F, argumentos, TIPO_SERVIDOR_RS)?;
    let manejador = exigir_manejador(F, argumentos.get(1).ok_or_else(|| error("E0210", format!("'{F}' necesita al menos 1 argumento")))?)?;
    instancia.datos.borrow_mut().insert("manejador_mensaje".to_string(), manejador);
    Ok(Valor::Nulo)
}

fn metodo_puerto_servidor(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ServidorRs.puerto";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let instancia = receptor_con_tipo(F, argumentos, TIPO_SERVIDOR_RS)?;
    Ok(instancia.datos.borrow().get("puerto").cloned().unwrap_or(Valor::Nulo))
}

fn metodo_esta_escuchando_servidor(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ServidorRs.esta_escuchando";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let instancia = receptor_con_tipo(F, argumentos, TIPO_SERVIDOR_RS)?;
    let activo = matches!(instancia.datos.borrow().get("activo"), Some(Valor::Log(true)));
    Ok(Valor::Log(activo))
}

/// Despachador único del servicio `"servidor_rs"`: corre en el hilo de la
/// VM para cada evento (conexión nueva, mensaje recibido, cierre) de
/// cualquier `ServidorRs` en escucha, identificado por `id_recurso`.
fn despachar_evento(
    vm: &mut Vm,
    id_recurso: u64,
    registro_servidores: &RegistroServidores,
    escritores: &RegistroEscritores,
    datos: CargaNativa,
) -> CargaNativa {
    let CargaNativa::Mapa(campos) = datos else {
        return CargaNativa::Nula;
    };
    let mapa: HashMap<String, CargaNativa> = campos.into_iter().collect();
    let Some(CargaNativa::Texto(evento)) = mapa.get("evento") else {
        return CargaNativa::Nula;
    };
    let Some(CargaNativa::Entero(id_conexion)) = mapa.get("id_conexion") else {
        return CargaNativa::Nula;
    };
    let id_conexion = *id_conexion as u64;

    if evento == "cerrar" {
        if let Some(escritor) = quitar_escritor(escritores, id_conexion) {
            cerrar_escritor_async(vm, &escritor);
        }
        return CargaNativa::Nula;
    }

    let Some(instancia_servidor) = registro_servidores.borrow().get(&id_recurso).cloned() else {
        return CargaNativa::Nula;
    };
    let conexion_valor = instancia_con_id(TIPO_CONEXION_RS, id_conexion);

    match evento.as_str() {
        "conectar" => {
            let manejador = instancia_servidor.datos.borrow().get("manejador_conectar").cloned().unwrap_or(Valor::Nulo);
            if let Valor::Funcion(funcion, entorno) = manejador {
                let _ = vm.llamar_funcion(&funcion, &entorno, vec![conexion_valor], None, None);
            }
        }
        "mensaje" => {
            let manejador = instancia_servidor.datos.borrow().get("manejador_mensaje").cloned().unwrap_or(Valor::Nulo);
            if let Valor::Funcion(funcion, entorno) = manejador {
                let mensaje_valor = match mapa.get("datos") {
                    Some(CargaNativa::Texto(texto)) => Valor::texto(texto.clone()),
                    Some(CargaNativa::Bytes(bytes)) => maquina_virtual::instancia_bits_desde(bytes),
                    _ => Valor::Nulo,
                };
                let _ = vm.llamar_funcion(&funcion, &entorno, vec![conexion_valor, mensaje_valor], None, None);
            }
        }
        _ => {}
    }
    CargaNativa::Nula
}

/// Tarea de fondo por conexión aceptada: hace el *handshake* RedSocket,
/// avisa al hilo de la VM de la conexión nueva y luego reenvía cada mensaje
/// entrante como un evento más, hasta que la conexión se cierra.
async fn manejar_conexion_rs(tcp: TcpStream, id_servidor: u64, escritores: RegistroEscritores, contador: Arc<AtomicU64>, manija: ManijaBucle) {
    let flujo = MaybeTlsStream::Plain(tcp);
    let Ok(rs) = tokio_tungstenite::accept_async(flujo).await else {
        return;
    };
    let (escritura, mut lectura) = rs.split();
    let id_conexion = siguiente_id(&contador);
    if insertar_escritor(&escritores, id_conexion, escritura).is_err() {
        return;
    }

    enviar_evento(&manija, id_servidor, CargaNativa::Mapa(vec![
        ("evento".to_string(), CargaNativa::Texto("conectar".to_string())),
        ("id_conexion".to_string(), CargaNativa::Entero(id_conexion as i64)),
    ]));

    while let Some(resultado) = lectura.next().await {
        let Ok(mensaje) = resultado else { break };
        let datos = match mensaje {
            Message::Text(texto) => CargaNativa::Texto(texto.to_string()),
            Message::Binary(bytes) => CargaNativa::Bytes(bytes.to_vec()),
            Message::Close(_) => break,
            _ => continue,
        };
        enviar_evento(&manija, id_servidor, CargaNativa::Mapa(vec![
            ("evento".to_string(), CargaNativa::Texto("mensaje".to_string())),
            ("id_conexion".to_string(), CargaNativa::Entero(id_conexion as i64)),
            ("datos".to_string(), datos),
        ]));
    }

    enviar_evento(&manija, id_servidor, CargaNativa::Mapa(vec![
        ("evento".to_string(), CargaNativa::Texto("cerrar".to_string())),
        ("id_conexion".to_string(), CargaNativa::Entero(id_conexion as i64)),
    ]));
}

fn enviar_evento(manija: &ManijaBucle, id_servidor: u64, datos: CargaNativa) {
    let (respuesta, _receptor) = std::sync::mpsc::channel();
    manija.enviar(Mensaje::Solicitud {
        servicio: SERVICIO.to_string(),
        id_recurso: id_servidor,
        datos,
        respuesta,
    });
}

async fn bucle_aceptacion_rs(
    escucha: TcpListener,
    id_servidor: u64,
    bandera: Arc<AtomicBool>,
    escritores: RegistroEscritores,
    contador: Arc<AtomicU64>,
    manija: ManijaBucle,
) {
    while bandera.load(Ordering::Relaxed) {
        match tokio::time::timeout(Duration::from_millis(200), escucha.accept()).await {
            Ok(Ok((tcp, _))) => {
                tokio::spawn(manejar_conexion_rs(
                    tcp,
                    id_servidor,
                    Arc::clone(&escritores),
                    Arc::clone(&contador),
                    manija.clone(),
                ));
            }
            Ok(Err(_)) => break,
            Err(_agotado) => continue,
        }
    }
}

fn metodo_escuchar_servidor(
    guardian: &Rc<GuardianPermisos>,
    registro_servidores: &RegistroServidores,
    banderas: &RegistroBanderas,
    escritores: &RegistroEscritores,
    contador: &Arc<AtomicU64>,
) -> maquina_virtual::FuncionNativaConVm {
    const F: &str = "ServidorRs.escuchar";
    let guardian = Rc::clone(guardian);
    let registro_servidores = Rc::clone(registro_servidores);
    let banderas = Rc::clone(banderas);
    let escritores = Arc::clone(escritores);
    let contador = Arc::clone(contador);
    Box::new(move |vm, argumentos| {
        exigir_aridad(F, &argumentos[1..], 1)?;
        let instancia = receptor_con_tipo(F, argumentos, TIPO_SERVIDOR_RS)?;
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
            return Err(error_rs(format!("'{F}': el servidor ya está escuchando")));
        }

        let manija = vm.bucle().manija();
        let escucha = manija
            .runtime()
            .block_on(TcpListener::bind(("0.0.0.0", puerto)))
            .map_err(|causa| error_rs(format!("no se pudo escuchar en el puerto {puerto}: {causa}")))?;
        let puerto_real = escucha
            .local_addr()
            .map_err(|causa| error_rs(format!("no se pudo leer el puerto del servidor: {causa}")))?
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
            let escritores = Arc::clone(&escritores);
            move |vm, id_recurso, datos| despachar_evento(vm, id_recurso, &registro_servidores, &escritores, datos)
        });
        vm.bucle().registrar_trabajo_activo();

        let manija_tarea = manija.clone();
        manija.runtime().spawn(bucle_aceptacion_rs(
            escucha,
            id_servidor,
            bandera,
            Arc::clone(&escritores),
            Arc::clone(&contador),
            manija_tarea,
        ));

        Ok(Valor::Nulo)
    })
}

fn metodo_detener_servidor(banderas: &RegistroBanderas, registro_servidores: &RegistroServidores) -> maquina_virtual::FuncionNativaConVm {
    const F: &str = "ServidorRs.detener";
    let banderas = Rc::clone(banderas);
    let registro_servidores = Rc::clone(registro_servidores);
    Box::new(move |vm, argumentos| {
        exigir_aridad(F, &argumentos[1..], 0)?;
        let instancia = receptor_con_tipo(F, argumentos, TIPO_SERVIDOR_RS)?;
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
    let escritores: RegistroEscritores = Arc::new(Mutex::new(HashMap::new()));
    let lectores: RegistroLectores = Arc::new(Mutex::new(HashMap::new()));
    let contador: Arc<AtomicU64> = Arc::new(AtomicU64::new(1));
    let registro_servidores: RegistroServidores = Rc::new(RefCell::new(HashMap::new()));
    let banderas: RegistroBanderas = Rc::new(RefCell::new(HashMap::new()));

    registro.registrar_modulo("clienters");
    registro.registrar_funcion("clienters.constructor", Box::new(constructor_cliente));
    registro.registrar_funcion_con_vm(
        &format!("{TIPO_CLIENTE_RS}.conectar"),
        metodo_conectar(guardian, &escritores, &lectores, &contador),
    );
    registro.registrar_funcion_con_vm(&format!("{TIPO_CLIENTE_RS}.recibir"), metodo_recibir(&lectores));
    registro.registrar_funcion_con_vm(
        &format!("{TIPO_CLIENTE_RS}.cerrar"),
        metodo_cerrar_cliente(&escritores, &lectores),
    );

    for tipo in [TIPO_CLIENTE_RS, TIPO_CONEXION_RS] {
        registro.registrar_funcion_con_vm(&format!("{tipo}.enviar_texto"), metodo_enviar_texto(tipo, &escritores));
        registro.registrar_funcion_con_vm(&format!("{tipo}.enviar_bits"), metodo_enviar_bits(tipo, &escritores));
    }
    registro.registrar_funcion_con_vm(&format!("{TIPO_CONEXION_RS}.cerrar"), metodo_cerrar_conexion(&escritores));

    registro.registrar_modulo("servidorrs");
    registro.registrar_funcion("servidorrs.constructor", Box::new(constructor_servidor));
    registro.registrar_funcion(&format!("{TIPO_SERVIDOR_RS}.al_conectar"), Box::new(metodo_al_conectar));
    registro.registrar_funcion(&format!("{TIPO_SERVIDOR_RS}.al_mensaje"), Box::new(metodo_al_mensaje));
    registro.registrar_funcion(&format!("{TIPO_SERVIDOR_RS}.puerto"), Box::new(metodo_puerto_servidor));
    registro.registrar_funcion(
        &format!("{TIPO_SERVIDOR_RS}.esta_escuchando"),
        Box::new(metodo_esta_escuchando_servidor),
    );
    registro.registrar_funcion_con_vm(
        &format!("{TIPO_SERVIDOR_RS}.escuchar"),
        metodo_escuchar_servidor(guardian, &registro_servidores, &banderas, &escritores, &contador),
    );
    registro.registrar_funcion_con_vm(
        &format!("{TIPO_SERVIDOR_RS}.detener"),
        metodo_detener_servidor(&banderas, &registro_servidores),
    );
}
