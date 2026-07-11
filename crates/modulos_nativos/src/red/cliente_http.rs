//! Objeto `ClienteHttp`: cliente HTTP completo con cabeceras y tiempo de
//! espera configurables, métodos síncronos y variantes asincrónicas que no
//! bloquean el bucle de eventos (la E/S corre en el runtime de tokio del
//! bucle de eventos; ver `crate::bucle_eventos` en `maquina_virtual`).
//!
//! Los cuerpos de petición aceptan `texto`, `jsn` (se serializa como JSON),
//! `Bits` (binario) o `nulo` (sin cuerpo). Tanto la variante síncrona como la
//! asincrónica (tras `esperar`) devuelven una instancia
//! [`RespuestaHttp`](TIPO_RESPUESTA) con los mismos métodos
//! (`.estado()`, `.cabeceras()`, `.texto()`, `.jsn()`, `.bits()`): la
//! asincrónica viaja por el bucle de eventos como [`CargaNativa::Instancia`]
//! y se reconstruye igual que la síncrona en vez de convertirse en un `jsn`
//! genérico.

use std::collections::HashMap;
use std::io::Read;
use std::rc::Rc;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use indexmap::IndexMap;
use maquina_virtual::bucle_eventos::CargaNativa;
use maquina_virtual::{DatosInstanciaNativa, Fallo, Mensaje, RegistroNativos, Valor, Vm};
use runtime::GuardianPermisos;

use crate::sistema_archivos::{
    FlujoArchivo, escribir_desde_lector, metadatos_archivo, tipo_contenido,
};
use crate::util::{arg_entero, arg_ruta, arg_texto, error, exigir_aridad};
use base64::{Engine, engine::general_purpose};

use super::{bytes_y_tipo_contenido, metodo_http_desde_texto, permiso_denegado};

const TIPO_CLIENTE: &str = "ClienteHttp";
const TIPO_RESPUESTA: &str = "RespuestaHttp";

/// Tiempo de espera por defecto (segundos) de un `ClienteHttp` nuevo.
const TIEMPO_ESPERA_DEFECTO: i64 = 30;
const TIEMPO_CONEXION_DEFECTO: i64 = 10;
const TIEMPO_LECTURA_DEFECTO: i64 = 30;
const LIMITE_RESPUESTA_DEFECTO: i64 = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct ClaveCliente {
    tiempo_total: u64,
    tiempo_conexion: u64,
    tiempo_lectura: u64,
    redirecciones: usize,
}

struct ConfiguracionCliente {
    cabeceras: Vec<(String, String)>,
    clave: ClaveCliente,
    limite_respuesta: usize,
}

static CLIENTES_BLOQUEANTES: LazyLock<Mutex<HashMap<ClaveCliente, reqwest::blocking::Client>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static CLIENTES_ASINCRONOS: LazyLock<Mutex<HashMap<ClaveCliente, reqwest::Client>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn error_red(mensaje: impl Into<String>) -> Fallo {
    error("E0703", mensaje.into())
}

// ----- ClienteHttp: instancia -----

fn nuevo_cliente() -> Valor {
    let mut datos = IndexMap::new();
    datos.insert("cabeceras".to_string(), Valor::jsn(IndexMap::new()));
    datos.insert(
        "tiempo_espera".to_string(),
        Valor::Entero(TIEMPO_ESPERA_DEFECTO),
    );
    datos.insert(
        "tiempo_conexion".to_string(),
        Valor::Entero(TIEMPO_CONEXION_DEFECTO),
    );
    datos.insert(
        "tiempo_lectura".to_string(),
        Valor::Entero(TIEMPO_LECTURA_DEFECTO),
    );
    datos.insert(
        "limite_respuesta".to_string(),
        Valor::Entero(LIMITE_RESPUESTA_DEFECTO),
    );
    datos.insert("redirigir".to_string(), Valor::Entero(10));
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_CLIENTE),
        datos: std::cell::RefCell::new(datos),
    }))
}

fn receptor_cliente(
    funcion: &str,
    argumentos: &[Valor],
) -> Result<Rc<DatosInstanciaNativa>, Fallo> {
    match argumentos.first() {
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == TIPO_CLIENTE => {
            Ok(Rc::clone(instancia))
        }
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba un ClienteHttp, pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
        None => Err(error("E0210", format!("'{funcion}' necesita el receptor"))),
    }
}

/// Cabeceras y tiempo de espera configurados en la instancia (copia, para no
/// mantener el préstamo del `RefCell` mientras se hace la petición).
fn config_cliente(instancia: &DatosInstanciaNativa) -> ConfiguracionCliente {
    let datos = instancia.datos.borrow();
    let cabeceras = match datos.get("cabeceras") {
        Some(Valor::Jsn(mapa)) => mapa
            .borrow()
            .iter()
            .map(|(clave, valor)| (clave.clone(), maquina_virtual::texto_de_valor(valor)))
            .collect(),
        _ => Vec::new(),
    };
    let tiempo_espera = match datos.get("tiempo_espera") {
        Some(Valor::Entero(segundos)) => (*segundos).max(0) as u64,
        _ => TIEMPO_ESPERA_DEFECTO as u64,
    };
    let redirigir = match datos.get("redirigir") {
        Some(Valor::Entero(n)) => (*n).max(0) as usize,
        _ => 10,
    };
    let tiempo_conexion = match datos.get("tiempo_conexion") {
        Some(Valor::Entero(segundos)) => (*segundos).max(0) as u64,
        _ => TIEMPO_CONEXION_DEFECTO as u64,
    };
    let tiempo_lectura = match datos.get("tiempo_lectura") {
        Some(Valor::Entero(segundos)) => (*segundos).max(0) as u64,
        _ => TIEMPO_LECTURA_DEFECTO as u64,
    };
    let limite_respuesta = match datos.get("limite_respuesta") {
        Some(Valor::Entero(bytes)) => usize::try_from((*bytes).max(0)).unwrap_or(usize::MAX),
        _ => LIMITE_RESPUESTA_DEFECTO as usize,
    };
    ConfiguracionCliente {
        cabeceras,
        clave: ClaveCliente {
            tiempo_total: tiempo_espera,
            tiempo_conexion,
            tiempo_lectura,
            redirecciones: redirigir,
        },
        limite_respuesta,
    }
}

fn constructor(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    exigir_aridad("nuevo ClienteHttp", argumentos, 0)?;
    Ok(nuevo_cliente())
}

fn metodo_fijar_cabecera(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ClienteHttp.fijar_cabecera";
    exigir_aridad(F, &argumentos[1..], 2)?;
    let instancia = receptor_cliente(F, argumentos)?;
    let nombre = arg_texto(F, argumentos, 1)?.to_string();
    let valor = arg_texto(F, argumentos, 2)?.to_string();
    match instancia.datos.borrow().get("cabeceras") {
        Some(Valor::Jsn(mapa)) => {
            mapa.borrow_mut().insert(nombre, Valor::texto(valor));
        }
        _ => {
            return Err(error(
                "E0406",
                format!("'{F}' recibió un ClienteHttp sin cabeceras internas"),
            ));
        }
    }
    Ok(Valor::Nulo)
}

fn metodo_fijar_tiempo_espera(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ClienteHttp.fijar_tiempo_espera";
    fijar_entero_no_negativo(F, "tiempo_espera", argumentos)
}

fn metodo_fijar_tiempo_conexion(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ClienteHttp.fijar_tiempo_conexion";
    fijar_entero_no_negativo(F, "tiempo_conexion", argumentos)
}

fn metodo_fijar_tiempo_lectura(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ClienteHttp.fijar_tiempo_lectura";
    fijar_entero_no_negativo(F, "tiempo_lectura", argumentos)
}

fn metodo_limite_respuesta(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ClienteHttp.limite_respuesta";
    fijar_entero_no_negativo(F, "limite_respuesta", argumentos)
}

fn fijar_entero_no_negativo(
    funcion: &str,
    campo: &str,
    argumentos: &[Valor],
) -> Result<Valor, Fallo> {
    exigir_aridad(funcion, &argumentos[1..], 1)?;
    let instancia = receptor_cliente(funcion, argumentos)?;
    let valor = match argumentos.get(1) {
        Some(Valor::Entero(valor)) if *valor >= 0 => *valor,
        Some(otro) => {
            return Err(error(
                "E0406",
                format!(
                    "'{funcion}' espera un entero no negativo, pero recibió '{}'",
                    otro.nombre_tipo()
                ),
            ));
        }
        None => {
            return Err(error(
                "E0210",
                format!("'{funcion}' necesita al menos 1 argumento"),
            ));
        }
    };
    instancia
        .datos
        .borrow_mut()
        .insert(campo.to_string(), Valor::Entero(valor));
    Ok(Valor::Nulo)
}

/// `ClienteHttp.autenticacion_basica(usuario, clave)`: fija la cabecera
/// `Authorization: Basic ...` para autenticación HTTP Basic.
fn metodo_autenticacion_basica(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ClienteHttp.autenticacion_basica";
    exigir_aridad(F, &argumentos[1..], 2)?;
    let instancia = receptor_cliente(F, argumentos)?;
    let usuario = arg_texto(F, argumentos, 1)?;
    let clave = arg_texto(F, argumentos, 2)?;
    let credenciales = general_purpose::STANDARD.encode(format!("{usuario}:{clave}"));
    match instancia.datos.borrow().get("cabeceras") {
        Some(Valor::Jsn(mapa)) => {
            mapa.borrow_mut().insert(
                "Authorization".to_string(),
                Valor::texto(format!("Basic {credenciales}")),
            );
        }
        _ => {
            return Err(error(
                "E0406",
                format!("'{F}' recibió un ClienteHttp sin cabeceras internas"),
            ));
        }
    }
    Ok(Valor::Nulo)
}

/// `ClienteHttp.portador(token)`: fija la cabecera `Authorization: Bearer ...`
/// para autenticación con token (OAuth2, JWT, ...).
fn metodo_portador(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ClienteHttp.portador";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let instancia = receptor_cliente(F, argumentos)?;
    let token = arg_texto(F, argumentos, 1)?;
    match instancia.datos.borrow().get("cabeceras") {
        Some(Valor::Jsn(mapa)) => {
            mapa.borrow_mut().insert(
                "Authorization".to_string(),
                Valor::texto(format!("Bearer {token}")),
            );
        }
        _ => {
            return Err(error(
                "E0406",
                format!("'{F}' recibió un ClienteHttp sin cabeceras internas"),
            ));
        }
    }
    Ok(Valor::Nulo)
}

/// `ClienteHttp.redirigir(seguidas)`: fija el número máximo de redirecciones
/// a seguir (0 = no seguir ninguna).
fn metodo_redirigir(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ClienteHttp.redirigir";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let instancia = receptor_cliente(F, argumentos)?;
    let seguidas = arg_entero(F, argumentos, 1)?;
    if seguidas < 0 {
        return Err(error(
            "E0406",
            format!("'{F}' espera una cantidad no negativa de redirecciones"),
        ));
    }
    instancia
        .datos
        .borrow_mut()
        .insert("redirigir".to_string(), Valor::Entero(seguidas));
    Ok(Valor::Nulo)
}

/// Petición ya validada y lista para enviarse: solo datos `Send`, para que
/// las variantes asincrónicas puedan lanzarla en el runtime de tokio del
/// bucle de eventos.
struct PeticionPreparada {
    metodo: reqwest::Method,
    url: String,
    cabeceras: Vec<(String, String)>,
    cuerpo: CuerpoPreparado,
    clave_cliente: ClaveCliente,
    limite_respuesta: usize,
}

enum CuerpoPreparado {
    Vacio,
    Bytes(Vec<u8>),
    Archivo { ruta: String, tamano: u64 },
}

fn preparar_peticion(
    funcion: &str,
    guardian: &GuardianPermisos,
    receptor: &DatosInstanciaNativa,
    metodo: reqwest::Method,
    url: &str,
    cuerpo: Option<&Valor>,
) -> Result<PeticionPreparada, Fallo> {
    let url_analizada = reqwest::Url::parse(url).map_err(|causa| {
        error(
            "E0406",
            format!("'{funcion}' recibió una URL inválida '{url}': {causa}"),
        )
    })?;
    let anfitrion = url_analizada.host_str().unwrap_or("").to_string();
    let puerto = url_analizada.port_or_known_default().unwrap_or(0);
    guardian
        .verificar_red_cliente(&anfitrion, puerto)
        .map_err(permiso_denegado)?;

    let configuracion = config_cliente(receptor);
    let mut cabeceras = configuracion.cabeceras;
    let (cuerpo_preparado, tipo_contenido) = match cuerpo {
        Some(valor) => {
            let (bytes, tipo) = bytes_y_tipo_contenido(funcion, valor)?;
            (CuerpoPreparado::Bytes(bytes), tipo)
        }
        None => (CuerpoPreparado::Vacio, None),
    };
    if let Some(tipo_contenido) = tipo_contenido
        && !cabeceras
            .iter()
            .any(|(nombre, _)| nombre.eq_ignore_ascii_case("content-type"))
    {
        cabeceras.push(("Content-Type".to_string(), tipo_contenido.to_string()));
    }

    Ok(PeticionPreparada {
        metodo,
        url: url.to_string(),
        cabeceras,
        cuerpo: cuerpo_preparado,
        clave_cliente: configuracion.clave,
        limite_respuesta: configuracion.limite_respuesta,
    })
}

fn preparar_peticion_archivo(
    funcion: &str,
    guardian: &GuardianPermisos,
    receptor: &DatosInstanciaNativa,
    url: &str,
    ruta: &str,
) -> Result<PeticionPreparada, Fallo> {
    guardian.verificar_lectura(ruta).map_err(permiso_denegado)?;
    let (tamano, _, es_archivo) = metadatos_archivo(ruta)
        .map_err(|causa| error_red(format!("no se pudo inspeccionar '{ruta}': {causa}")))?;
    if !es_archivo {
        return Err(error_red(format!("'{ruta}' no es un archivo regular")));
    }
    let mut peticion = preparar_peticion(
        funcion,
        guardian,
        receptor,
        reqwest::Method::POST,
        url,
        None,
    )?;
    if !peticion
        .cabeceras
        .iter()
        .any(|(nombre, _)| nombre.eq_ignore_ascii_case("content-type"))
    {
        peticion
            .cabeceras
            .push(("Content-Type".to_string(), tipo_contenido(ruta)));
    }
    peticion.cuerpo = CuerpoPreparado::Archivo {
        ruta: ruta.to_string(),
        tamano,
    };
    Ok(peticion)
}

fn cliente_bloqueante(clave: ClaveCliente) -> Result<reqwest::blocking::Client, Fallo> {
    if let Some(cliente) = CLIENTES_BLOQUEANTES
        .lock()
        .map_err(|_| error_red("el registro de clientes HTTP quedó bloqueado"))?
        .get(&clave)
        .cloned()
    {
        return Ok(cliente);
    }
    let mut constructor = reqwest::blocking::Client::builder();
    if clave.tiempo_total > 0 {
        constructor = constructor.timeout(Duration::from_secs(clave.tiempo_total));
    }
    if clave.tiempo_conexion > 0 {
        constructor = constructor.connect_timeout(Duration::from_secs(clave.tiempo_conexion));
    }
    constructor = if clave.redirecciones == 0 {
        constructor.redirect(reqwest::redirect::Policy::none())
    } else {
        constructor.redirect(reqwest::redirect::Policy::limited(clave.redirecciones))
    };
    let nuevo = constructor
        .build()
        .map_err(|causa| error_red(format!("no se pudo crear el cliente HTTP: {causa}")))?;
    let mut clientes = CLIENTES_BLOQUEANTES
        .lock()
        .map_err(|_| error_red("el registro de clientes HTTP quedó bloqueado"))?;
    Ok(clientes.entry(clave).or_insert(nuevo).clone())
}

fn cliente_asincrono(clave: ClaveCliente) -> Result<reqwest::Client, String> {
    if let Some(cliente) = CLIENTES_ASINCRONOS
        .lock()
        .map_err(|_| "el registro de clientes HTTP quedó bloqueado".to_string())?
        .get(&clave)
        .cloned()
    {
        return Ok(cliente);
    }
    let mut constructor = reqwest::Client::builder();
    if clave.tiempo_total > 0 {
        constructor = constructor.timeout(Duration::from_secs(clave.tiempo_total));
    }
    if clave.tiempo_conexion > 0 {
        constructor = constructor.connect_timeout(Duration::from_secs(clave.tiempo_conexion));
    }
    if clave.tiempo_lectura > 0 {
        constructor = constructor.read_timeout(Duration::from_secs(clave.tiempo_lectura));
    }
    constructor = if clave.redirecciones == 0 {
        constructor.redirect(reqwest::redirect::Policy::none())
    } else {
        constructor.redirect(reqwest::redirect::Policy::limited(clave.redirecciones))
    };
    let nuevo = constructor
        .build()
        .map_err(|causa| format!("no se pudo crear el cliente HTTP: {causa}"))?;
    let mut clientes = CLIENTES_ASINCRONOS
        .lock()
        .map_err(|_| "el registro de clientes HTTP quedó bloqueado".to_string())?;
    Ok(clientes.entry(clave).or_insert(nuevo).clone())
}

fn verificar_longitud_respuesta(
    longitud: Option<u64>,
    limite: usize,
    contexto: &str,
) -> Result<(), Fallo> {
    verificar_longitud_respuesta_texto(longitud, limite, contexto).map_err(error_red)
}

fn verificar_longitud_respuesta_texto(
    longitud: Option<u64>,
    limite: usize,
    contexto: &str,
) -> Result<(), String> {
    if longitud.is_some_and(|longitud| longitud > limite as u64) {
        return Err(format!("{contexto} excede el límite de {limite} bytes"));
    }
    Ok(())
}

fn texto_version_http(version: reqwest::Version) -> &'static str {
    match version {
        reqwest::Version::HTTP_09 => "HTTP/0.9",
        reqwest::Version::HTTP_10 => "HTTP/1.0",
        reqwest::Version::HTTP_11 => "HTTP/1.1",
        reqwest::Version::HTTP_2 => "HTTP/2",
        reqwest::Version::HTTP_3 => "HTTP/3",
        _ => "HTTP/desconocido",
    }
}

// ----- Ejecución síncrona -----

fn ejecutar_peticion_bloqueante(peticion: PeticionPreparada) -> Result<Valor, Fallo> {
    let cliente = cliente_bloqueante(peticion.clave_cliente)?;

    let mut peticion_construida = cliente.request(peticion.metodo, &peticion.url);
    for (nombre, valor) in &peticion.cabeceras {
        peticion_construida = peticion_construida.header(nombre, valor);
    }
    match peticion.cuerpo {
        CuerpoPreparado::Vacio => {}
        CuerpoPreparado::Bytes(bytes) if bytes.is_empty() => {}
        CuerpoPreparado::Bytes(bytes) => {
            peticion_construida = peticion_construida.body(bytes);
        }
        CuerpoPreparado::Archivo { ruta, tamano } => {
            let flujo = FlujoArchivo::abrir(&ruta)
                .map_err(|causa| error_red(format!("no se pudo abrir '{ruta}': {causa}")))?;
            peticion_construida =
                peticion_construida.body(reqwest::blocking::Body::sized(flujo, tamano));
        }
    }

    let respuesta = peticion_construida
        .send()
        .map_err(|causa| error_red(format!("falló la petición a '{}': {causa}", peticion.url)))?;
    verificar_longitud_respuesta(
        respuesta.content_length(),
        peticion.limite_respuesta,
        "respuesta HTTP",
    )?;
    let estado = respuesta.status().as_u16();
    let url_final = respuesta.url().to_string();
    let protocolo = texto_version_http(respuesta.version()).to_string();
    let cabeceras_respuesta: Vec<(String, String)> = respuesta
        .headers()
        .iter()
        .map(|(nombre, valor)| {
            (
                nombre.to_string(),
                String::from_utf8_lossy(valor.as_bytes()).into_owned(),
            )
        })
        .collect();
    let mut cuerpo = Vec::new();
    respuesta
        .take((peticion.limite_respuesta as u64).saturating_add(1))
        .read_to_end(&mut cuerpo)
        .map_err(|causa| error_red(format!("no se pudo leer la respuesta: {causa}")))?;
    if cuerpo.len() > peticion.limite_respuesta {
        return Err(error_red(format!(
            "la respuesta HTTP excede el límite de {} bytes",
            peticion.limite_respuesta
        )));
    }

    let peso = cuerpo.len();
    Ok(valor_respuesta_http(
        estado,
        url_final,
        protocolo,
        cabeceras_respuesta,
        cuerpo,
        peso,
    ))
}

fn ejecutar_descarga_bloqueante(peticion: PeticionPreparada, ruta: &str) -> Result<Valor, Fallo> {
    let cliente = cliente_bloqueante(peticion.clave_cliente)?;
    let mut peticion_construida = cliente.request(peticion.metodo, &peticion.url);
    for (nombre, valor) in &peticion.cabeceras {
        peticion_construida = peticion_construida.header(nombre, valor);
    }
    if !matches!(peticion.cuerpo, CuerpoPreparado::Vacio) {
        return Err(error_red("una descarga HTTP no debe enviar cuerpo"));
    }
    let mut respuesta = peticion_construida
        .send()
        .map_err(|causa| error_red(format!("falló la petición a '{}': {causa}", peticion.url)))?;
    verificar_longitud_respuesta(
        respuesta.content_length(),
        peticion.limite_respuesta,
        "respuesta HTTP",
    )?;
    let estado = respuesta.status().as_u16();
    let url_final = respuesta.url().to_string();
    let protocolo = texto_version_http(respuesta.version()).to_string();
    let cabeceras: Vec<(String, String)> = respuesta
        .headers()
        .iter()
        .map(|(nombre, valor)| {
            (
                nombre.to_string(),
                String::from_utf8_lossy(valor.as_bytes()).into_owned(),
            )
        })
        .collect();
    let peso = escribir_desde_lector(ruta, &mut respuesta, peticion.limite_respuesta)
        .map_err(|causa| error_red(format!("no se pudo descargar a '{ruta}': {causa}")))?;
    Ok(valor_respuesta_http(
        estado,
        url_final,
        protocolo,
        cabeceras,
        Vec::new(),
        peso.min(usize::MAX as u64) as usize,
    ))
}

fn valor_respuesta_http(
    estado: u16,
    url: String,
    protocolo: String,
    cabeceras: Vec<(String, String)>,
    cuerpo: Vec<u8>,
    peso: usize,
) -> Valor {
    let mut mapa_cabeceras = IndexMap::new();
    for (nombre, valor) in cabeceras {
        insertar_cabecera_respuesta(&mut mapa_cabeceras, nombre, valor);
    }
    let mut datos = IndexMap::new();
    datos.insert("estado".to_string(), Valor::Entero(i64::from(estado)));
    datos.insert("url".to_string(), Valor::texto(url));
    datos.insert("protocolo".to_string(), Valor::texto(protocolo));
    datos.insert(
        "peso".to_string(),
        Valor::Entero(peso.min(i64::MAX as usize) as i64),
    );
    datos.insert("cabeceras".to_string(), Valor::jsn(mapa_cabeceras));
    datos.insert(
        "cuerpo".to_string(),
        maquina_virtual::instancia_bits_desde(&cuerpo),
    );
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_RESPUESTA),
        datos: std::cell::RefCell::new(datos),
    }))
}

fn insertar_cabecera_respuesta(mapa: &mut IndexMap<String, Valor>, nombre: String, valor: String) {
    match mapa.get_mut(&nombre) {
        Some(Valor::Lista(valores)) => valores.borrow_mut().push(Valor::texto(valor)),
        Some(existente) => {
            let anterior = existente.clone();
            *existente = Valor::lista(vec![anterior, Valor::texto(valor)]);
        }
        None => {
            mapa.insert(nombre, Valor::texto(valor));
        }
    }
}

fn cabeceras_respuesta_a_carga(
    cabeceras: &reqwest::header::HeaderMap,
) -> Vec<(String, CargaNativa)> {
    let mut agrupadas: IndexMap<String, Vec<String>> = IndexMap::new();
    for (nombre, valor) in cabeceras {
        agrupadas
            .entry(nombre.to_string())
            .or_default()
            .push(String::from_utf8_lossy(valor.as_bytes()).into_owned());
    }
    agrupadas
        .into_iter()
        .map(|(nombre, valores)| {
            let valor = if valores.len() == 1 {
                CargaNativa::Texto(valores.into_iter().next().unwrap_or_default())
            } else {
                CargaNativa::Lista(valores.into_iter().map(CargaNativa::Texto).collect())
            };
            (nombre, valor)
        })
        .collect()
}

// ----- Ejecución asincrónica -----

fn ejecutar_peticion_asincrona(vm: &mut Vm, peticion: PeticionPreparada) -> Valor {
    let id = vm.bucle().nuevo_id();
    let manija = vm.bucle().manija();
    let manija_tarea = manija.clone();

    manija.runtime().spawn(async move {
        let resultado = enviar_peticion_async(peticion).await;
        manija_tarea.enviar(Mensaje::TareaLista { id, resultado });
    });

    Valor::TareaNativa(Rc::new(maquina_virtual::EstadoTareaNativa { id }))
}

fn ejecutar_peticion_bloqueante_asincrona(vm: &mut Vm, peticion: PeticionPreparada) -> Valor {
    let id = vm.bucle().nuevo_id();
    let manija = vm.bucle().manija();
    let manija_tarea = manija.clone();
    manija.runtime().spawn_blocking(move || {
        let resultado = ejecutar_peticion_bloqueante(peticion)
            .map(|valor| maquina_virtual::valor_a_carga(&valor))
            .map_err(mensaje_de_fallo);
        manija_tarea.enviar(Mensaje::TareaLista { id, resultado });
    });
    Valor::TareaNativa(Rc::new(maquina_virtual::EstadoTareaNativa { id }))
}

fn ejecutar_descarga_bloqueante_asincrona(
    vm: &mut Vm,
    peticion: PeticionPreparada,
    ruta: String,
) -> Valor {
    let id = vm.bucle().nuevo_id();
    let manija = vm.bucle().manija();
    let manija_tarea = manija.clone();
    manija.runtime().spawn_blocking(move || {
        let resultado = ejecutar_descarga_bloqueante(peticion, &ruta)
            .map(|valor| maquina_virtual::valor_a_carga(&valor))
            .map_err(mensaje_de_fallo);
        manija_tarea.enviar(Mensaje::TareaLista { id, resultado });
    });
    Valor::TareaNativa(Rc::new(maquina_virtual::EstadoTareaNativa { id }))
}

fn mensaje_de_fallo(fallo: Fallo) -> String {
    match fallo {
        Fallo::Excepcion(datos) => datos.mensaje,
        Fallo::Error(error) => error.mensaje,
    }
}

async fn enviar_peticion_async(peticion: PeticionPreparada) -> Result<CargaNativa, String> {
    let cliente = cliente_asincrono(peticion.clave_cliente)?;

    let mut peticion_construida = cliente.request(peticion.metodo, &peticion.url);
    for (nombre, valor) in &peticion.cabeceras {
        peticion_construida = peticion_construida.header(nombre, valor);
    }
    match peticion.cuerpo {
        CuerpoPreparado::Vacio => {}
        CuerpoPreparado::Bytes(bytes) if bytes.is_empty() => {}
        CuerpoPreparado::Bytes(bytes) => {
            peticion_construida = peticion_construida.body(bytes);
        }
        CuerpoPreparado::Archivo { .. } => {
            return Err(
                "el envío de archivo debe ejecutarse en el pool bloqueante especializado"
                    .to_string(),
            );
        }
    }

    let mut respuesta = peticion_construida
        .send()
        .await
        .map_err(|causa| format!("falló la petición a '{}': {causa}", peticion.url))?;
    verificar_longitud_respuesta_texto(
        respuesta.content_length(),
        peticion.limite_respuesta,
        "respuesta HTTP",
    )?;
    let estado = i64::from(respuesta.status().as_u16());
    let url_final = respuesta.url().to_string();
    let protocolo = texto_version_http(respuesta.version()).to_string();
    let cabeceras = cabeceras_respuesta_a_carga(respuesta.headers());
    let mut cuerpo = Vec::new();
    while let Some(trozo) = respuesta
        .chunk()
        .await
        .map_err(|causa| format!("no se pudo leer la respuesta: {causa}"))?
    {
        let nuevo_tamano = cuerpo
            .len()
            .checked_add(trozo.len())
            .ok_or_else(|| "la respuesta HTTP es demasiado grande".to_string())?;
        if nuevo_tamano > peticion.limite_respuesta {
            return Err(format!(
                "la respuesta HTTP excede el límite de {} bytes",
                peticion.limite_respuesta
            ));
        }
        cuerpo.extend_from_slice(&trozo);
    }
    let peso = cuerpo.len().min(i64::MAX as usize) as i64;

    // Se reconstruye como una instancia real `RespuestaHttp` (no un `jsn`
    // genérico) para que `esperar` devuelva el mismo tipo que la variante
    // síncrona, con los mismos métodos (`.estado()`, `.texto()`, `.bits()`, ...).
    Ok(CargaNativa::Instancia {
        tipo: TIPO_RESPUESTA.to_string(),
        campos: vec![
            ("estado".to_string(), CargaNativa::Entero(estado)),
            ("url".to_string(), CargaNativa::Texto(url_final)),
            ("protocolo".to_string(), CargaNativa::Texto(protocolo)),
            ("peso".to_string(), CargaNativa::Entero(peso)),
            ("cabeceras".to_string(), CargaNativa::Mapa(cabeceras)),
            ("cuerpo".to_string(), CargaNativa::Bytes(cuerpo)),
        ],
    })
}

// ----- Métodos de instancia: peticiones -----

/// Extrae el cuerpo opcional de los argumentos (después de la URL), si la
/// aridad esperada lo incluye.
fn cuerpo_opcional(argumentos: &[Valor], indice: usize) -> Option<&Valor> {
    argumentos.get(indice)
}

fn metodo_sin_cuerpo(
    funcion: &'static str,
    guardian: &Rc<GuardianPermisos>,
    metodo_http: reqwest::Method,
) -> maquina_virtual::FuncionNativa {
    let guardian = Rc::clone(guardian);
    Box::new(move |argumentos| {
        exigir_aridad(funcion, &argumentos[1..], 1)?;
        let instancia = receptor_cliente(funcion, argumentos)?;
        let url = arg_texto(funcion, argumentos, 1)?;
        let peticion = preparar_peticion(
            funcion,
            &guardian,
            &instancia,
            metodo_http.clone(),
            url,
            None,
        )?;
        ejecutar_peticion_bloqueante(peticion)
    })
}

fn metodo_con_cuerpo(
    funcion: &'static str,
    guardian: &Rc<GuardianPermisos>,
    metodo_http: reqwest::Method,
) -> maquina_virtual::FuncionNativa {
    let guardian = Rc::clone(guardian);
    Box::new(move |argumentos| {
        exigir_aridad(funcion, &argumentos[1..], 2)?;
        let instancia = receptor_cliente(funcion, argumentos)?;
        let url = arg_texto(funcion, argumentos, 1)?;
        let cuerpo = cuerpo_opcional(argumentos, 2);
        let peticion = preparar_peticion(
            funcion,
            &guardian,
            &instancia,
            metodo_http.clone(),
            url,
            cuerpo,
        )?;
        ejecutar_peticion_bloqueante(peticion)
    })
}

fn metodo_pedir(guardian: &Rc<GuardianPermisos>) -> maquina_virtual::FuncionNativa {
    const F: &str = "ClienteHttp.pedir";
    let guardian = Rc::clone(guardian);
    Box::new(move |argumentos| {
        if argumentos.len() < 3 || argumentos.len() > 4 {
            return Err(error(
                "E0210",
                format!(
                    "'{F}' espera 2 o 3 argumentos (metodo, url, cuerpo?), pero recibió {}",
                    argumentos.len() - 1
                ),
            ));
        }
        let instancia = receptor_cliente(F, argumentos)?;
        let metodo_texto = arg_texto(F, argumentos, 1)?;
        let metodo_http = metodo_http_desde_texto(F, metodo_texto)?;
        let url = arg_texto(F, argumentos, 2)?;
        let cuerpo = cuerpo_opcional(argumentos, 3);
        let peticion = preparar_peticion(F, &guardian, &instancia, metodo_http, url, cuerpo)?;
        ejecutar_peticion_bloqueante(peticion)
    })
}

// ----- Envío y descarga de archivos -----

/// `ClienteHttp.enviar_archivo(url, ruta)`: transmite el archivo de `ruta`
/// como cuerpo de una petición POST, sin cargarlo completo en memoria.
fn metodo_enviar_archivo(guardian: &Rc<GuardianPermisos>) -> maquina_virtual::FuncionNativa {
    const F: &str = "ClienteHttp.enviar_archivo";
    let guardian = Rc::clone(guardian);
    Box::new(move |argumentos| {
        exigir_aridad(F, &argumentos[1..], 2)?;
        let instancia = receptor_cliente(F, argumentos)?;
        let url = arg_texto(F, argumentos, 1)?;
        let ruta = arg_ruta(F, argumentos, 2)?;
        let peticion = preparar_peticion_archivo(F, &guardian, &instancia, url, &ruta)?;
        ejecutar_peticion_bloqueante(peticion)
    })
}

/// `ClienteHttp.descargar(url, ruta)`: transmite la respuesta a un archivo
/// temporal y lo confirma al finalizar. El cuerpo no se duplica en memoria.
fn metodo_descargar(guardian: &Rc<GuardianPermisos>) -> maquina_virtual::FuncionNativa {
    const F: &str = "ClienteHttp.descargar";
    let guardian = Rc::clone(guardian);
    Box::new(move |argumentos| {
        exigir_aridad(F, &argumentos[1..], 2)?;
        let instancia = receptor_cliente(F, argumentos)?;
        let url = arg_texto(F, argumentos, 1)?;
        let ruta = arg_ruta(F, argumentos, 2)?;
        guardian
            .verificar_escritura(&ruta)
            .map_err(permiso_denegado)?;
        let peticion =
            preparar_peticion(F, &guardian, &instancia, reqwest::Method::GET, url, None)?;
        ejecutar_descarga_bloqueante(peticion, &ruta)
    })
}

// ----- Métodos de instancia: variantes asincrónicas -----

fn metodo_sin_cuerpo_asincrono(
    funcion: &'static str,
    guardian: &Rc<GuardianPermisos>,
    metodo_http: reqwest::Method,
) -> maquina_virtual::FuncionNativaConVm {
    let guardian = Rc::clone(guardian);
    Box::new(move |vm, argumentos| {
        exigir_aridad(funcion, &argumentos[1..], 1)?;
        let instancia = receptor_cliente(funcion, argumentos)?;
        let url = arg_texto(funcion, argumentos, 1)?;
        let peticion = preparar_peticion(
            funcion,
            &guardian,
            &instancia,
            metodo_http.clone(),
            url,
            None,
        )?;
        Ok(ejecutar_peticion_asincrona(vm, peticion))
    })
}

fn metodo_con_cuerpo_asincrono(
    funcion: &'static str,
    guardian: &Rc<GuardianPermisos>,
    metodo_http: reqwest::Method,
) -> maquina_virtual::FuncionNativaConVm {
    let guardian = Rc::clone(guardian);
    Box::new(move |vm, argumentos| {
        exigir_aridad(funcion, &argumentos[1..], 2)?;
        let instancia = receptor_cliente(funcion, argumentos)?;
        let url = arg_texto(funcion, argumentos, 1)?;
        let cuerpo = cuerpo_opcional(argumentos, 2).cloned();
        let peticion = preparar_peticion(
            funcion,
            &guardian,
            &instancia,
            metodo_http.clone(),
            url,
            cuerpo.as_ref(),
        )?;
        Ok(ejecutar_peticion_asincrona(vm, peticion))
    })
}

fn metodo_pedir_asincrono(guardian: &Rc<GuardianPermisos>) -> maquina_virtual::FuncionNativaConVm {
    const F: &str = "ClienteHttp.pedir_asincrono";
    let guardian = Rc::clone(guardian);
    Box::new(move |vm, argumentos| {
        if argumentos.len() < 3 || argumentos.len() > 4 {
            return Err(error(
                "E0210",
                format!(
                    "'{F}' espera 2 o 3 argumentos (metodo, url, cuerpo?), pero recibió {}",
                    argumentos.len() - 1
                ),
            ));
        }
        let instancia = receptor_cliente(F, argumentos)?;
        let metodo_texto = arg_texto(F, argumentos, 1)?;
        let metodo_http = metodo_http_desde_texto(F, metodo_texto)?;
        let url = arg_texto(F, argumentos, 2)?;
        let cuerpo = cuerpo_opcional(argumentos, 3).cloned();
        let peticion =
            preparar_peticion(F, &guardian, &instancia, metodo_http, url, cuerpo.as_ref())?;
        Ok(ejecutar_peticion_asincrona(vm, peticion))
    })
}

// ----- Variantes asincrónicas de enviar_archivo y descargar -----

fn metodo_enviar_archivo_asincrono(
    guardian: &Rc<GuardianPermisos>,
) -> maquina_virtual::FuncionNativaConVm {
    const F: &str = "ClienteHttp.enviar_archivo_asincrono";
    let guardian = Rc::clone(guardian);
    Box::new(move |vm, argumentos| {
        exigir_aridad(F, &argumentos[1..], 2)?;
        let instancia = receptor_cliente(F, argumentos)?;
        let url = arg_texto(F, argumentos, 1)?;
        let ruta = arg_ruta(F, argumentos, 2)?;
        let peticion = preparar_peticion_archivo(F, &guardian, &instancia, url, &ruta)?;
        Ok(ejecutar_peticion_bloqueante_asincrona(vm, peticion))
    })
}

fn metodo_descargar_asincrono(
    guardian: &Rc<GuardianPermisos>,
) -> maquina_virtual::FuncionNativaConVm {
    const F: &str = "ClienteHttp.descargar_asincrono";
    let guardian = Rc::clone(guardian);
    Box::new(move |vm, argumentos| {
        exigir_aridad(F, &argumentos[1..], 2)?;
        let instancia = receptor_cliente(F, argumentos)?;
        let url = arg_texto(F, argumentos, 1)?;
        let ruta = arg_ruta(F, argumentos, 2)?;
        guardian
            .verificar_escritura(&ruta)
            .map_err(permiso_denegado)?;

        let peticion =
            preparar_peticion(F, &guardian, &instancia, reqwest::Method::GET, url, None)?;
        Ok(ejecutar_descarga_bloqueante_asincrona(vm, peticion, ruta))
    })
}

// ----- RespuestaHttp: métodos -----

fn receptor_respuesta(
    funcion: &str,
    argumentos: &[Valor],
) -> Result<Rc<DatosInstanciaNativa>, Fallo> {
    match argumentos.first() {
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == TIPO_RESPUESTA => {
            Ok(Rc::clone(instancia))
        }
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba un RespuestaHttp, pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
        None => Err(error("E0210", format!("'{funcion}' necesita el receptor"))),
    }
}

fn campo_respuesta(funcion: &str, argumentos: &[Valor], campo: &str) -> Result<Valor, Fallo> {
    let instancia = receptor_respuesta(funcion, argumentos)?;
    Ok(instancia
        .datos
        .borrow()
        .get(campo)
        .cloned()
        .unwrap_or(Valor::Nulo))
}

fn metodo_estado(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "RespuestaHttp.estado";
    exigir_aridad(F, &argumentos[1..], 0)?;
    campo_respuesta(F, argumentos, "estado")
}

fn metodo_url(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "RespuestaHttp.url";
    exigir_aridad(F, &argumentos[1..], 0)?;
    campo_respuesta(F, argumentos, "url")
}

fn metodo_protocolo(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "RespuestaHttp.protocolo";
    exigir_aridad(F, &argumentos[1..], 0)?;
    campo_respuesta(F, argumentos, "protocolo")
}

fn metodo_cabeceras(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "RespuestaHttp.cabeceras";
    exigir_aridad(F, &argumentos[1..], 0)?;
    campo_respuesta(F, argumentos, "cabeceras")
}

fn metodo_bits(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "RespuestaHttp.bits";
    exigir_aridad(F, &argumentos[1..], 0)?;
    campo_respuesta(F, argumentos, "cuerpo")
}

fn bytes_cuerpo(funcion: &str, argumentos: &[Valor]) -> Result<Vec<u8>, Fallo> {
    let cuerpo = campo_respuesta(funcion, argumentos, "cuerpo")?;
    maquina_virtual::bytes_de_bits(&cuerpo).ok_or_else(|| {
        error(
            "E0406",
            format!("'{funcion}' recibió un RespuestaHttp sin cuerpo binario válido"),
        )
    })
}

fn metodo_texto(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "RespuestaHttp.texto";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let bytes = bytes_cuerpo(F, argumentos)?;
    String::from_utf8(bytes).map(Valor::texto).map_err(|_| {
        error(
            "E0406",
            format!("'{F}' no pudo decodificar el cuerpo como texto UTF-8 válido"),
        )
    })
}

fn metodo_jsn(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "RespuestaHttp.jsn";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let bytes = bytes_cuerpo(F, argumentos)?;
    let texto = String::from_utf8(bytes).map_err(|_| {
        error(
            "E0406",
            format!("'{F}' no pudo decodificar el cuerpo como texto UTF-8 válido"),
        )
    })?;
    let json: serde_json::Value = serde_json::from_str(&texto).map_err(|causa| {
        error(
            "E0406",
            format!("'{F}': el cuerpo no es JSON válido: {causa}"),
        )
    })?;
    Ok(maquina_virtual::valores::json_a_valor(&json))
}

/// `RespuestaHttp.peso() -> entero`: tamaño del cuerpo de la respuesta en
/// bytes. Útil para verificar descargas, informar progreso, etc.
fn metodo_peso(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "RespuestaHttp.peso";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let peso_explicito = campo_respuesta(F, argumentos, "peso")?;
    if matches!(peso_explicito, Valor::Entero(_)) {
        return Ok(peso_explicito);
    }
    let cuerpo = campo_respuesta(F, argumentos, "cuerpo")?;
    let peso = match &cuerpo {
        Valor::Nulo => 0i64,
        Valor::InstanciaNativa(instancia) if &*instancia.tipo == "Bits" => {
            match instancia.datos.borrow().get("datos") {
                Some(Valor::Lista(lista)) => lista.borrow().len() as i64,
                _ => 0,
            }
        }
        Valor::Texto(t) => t.len() as i64,
        _ => 0,
    };
    Ok(Valor::Entero(peso))
}

/// `RespuestaHttp.cabecera(nombre) -> texto|nulo`: busca una cabecera de
/// la respuesta por nombre (sin distinguir mayúsculas/minúsculas).
fn metodo_cabecera_individual(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "RespuestaHttp.cabecera";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let nombre = arg_texto(F, argumentos, 1)?;
    let cabeceras = campo_respuesta(F, argumentos, "cabeceras")?;
    if let Valor::Jsn(mapa) = cabeceras {
        for (clave, valor) in mapa.borrow().iter() {
            if clave.eq_ignore_ascii_case(nombre) {
                return Ok(match valor {
                    Valor::Lista(valores) => {
                        valores.borrow().last().cloned().unwrap_or(Valor::Nulo)
                    }
                    valor => valor.clone(),
                });
            }
        }
    }
    Ok(Valor::Nulo)
}

fn metodo_cabeceras_todas(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "RespuestaHttp.cabeceras_todas";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let nombre = arg_texto(F, argumentos, 1)?;
    let cabeceras = campo_respuesta(F, argumentos, "cabeceras")?;
    if let Valor::Jsn(mapa) = cabeceras {
        for (clave, valor) in mapa.borrow().iter() {
            if clave.eq_ignore_ascii_case(nombre) {
                return Ok(match valor {
                    Valor::Lista(valores) => Valor::lista(valores.borrow().clone()),
                    valor => Valor::lista(vec![valor.clone()]),
                });
            }
        }
    }
    Ok(Valor::lista(Vec::new()))
}

// ----- Registro -----

pub fn registrar(registro: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    registro.registrar_modulo("clientehttp");
    registro.registrar_funcion("clientehttp.constructor", Box::new(constructor));

    registro.registrar_funcion(
        &format!("{TIPO_CLIENTE}.fijar_cabecera"),
        Box::new(metodo_fijar_cabecera),
    );
    registro.registrar_funcion(
        &format!("{TIPO_CLIENTE}.fijar_tiempo_espera"),
        Box::new(metodo_fijar_tiempo_espera),
    );
    registro.registrar_funcion(
        &format!("{TIPO_CLIENTE}.fijar_tiempo_conexion"),
        Box::new(metodo_fijar_tiempo_conexion),
    );
    registro.registrar_funcion(
        &format!("{TIPO_CLIENTE}.fijar_tiempo_lectura"),
        Box::new(metodo_fijar_tiempo_lectura),
    );
    registro.registrar_funcion(
        &format!("{TIPO_CLIENTE}.limite_respuesta"),
        Box::new(metodo_limite_respuesta),
    );
    registro.registrar_funcion(
        &format!("{TIPO_CLIENTE}.autenticacion_basica"),
        Box::new(metodo_autenticacion_basica),
    );
    registro.registrar_funcion(
        &format!("{TIPO_CLIENTE}.portador"),
        Box::new(metodo_portador),
    );
    registro.registrar_funcion(
        &format!("{TIPO_CLIENTE}.redirigir"),
        Box::new(metodo_redirigir),
    );

    let metodos_sin_cuerpo: [(&str, &'static str, reqwest::Method); 2] = [
        ("obtener", "ClienteHttp.obtener", reqwest::Method::GET),
        ("eliminar", "ClienteHttp.eliminar", reqwest::Method::DELETE),
    ];
    for (nombre, etiqueta, metodo_http) in metodos_sin_cuerpo {
        registro.registrar_funcion(
            &format!("{TIPO_CLIENTE}.{nombre}"),
            metodo_sin_cuerpo(etiqueta, guardian, metodo_http.clone()),
        );
        registro.registrar_funcion_con_vm(
            &format!("{TIPO_CLIENTE}.{nombre}_asincrono"),
            metodo_sin_cuerpo_asincrono(etiqueta, guardian, metodo_http),
        );
    }

    let metodos_con_cuerpo: [(&str, &'static str, reqwest::Method); 3] = [
        ("publicar", "ClienteHttp.publicar", reqwest::Method::POST),
        ("poner", "ClienteHttp.poner", reqwest::Method::PUT),
        ("parchar", "ClienteHttp.parchar", reqwest::Method::PATCH),
    ];
    for (nombre, etiqueta, metodo_http) in metodos_con_cuerpo {
        registro.registrar_funcion(
            &format!("{TIPO_CLIENTE}.{nombre}"),
            metodo_con_cuerpo(etiqueta, guardian, metodo_http.clone()),
        );
        registro.registrar_funcion_con_vm(
            &format!("{TIPO_CLIENTE}.{nombre}_asincrono"),
            metodo_con_cuerpo_asincrono(etiqueta, guardian, metodo_http),
        );
    }

    registro.registrar_funcion(&format!("{TIPO_CLIENTE}.pedir"), metodo_pedir(guardian));
    registro.registrar_funcion_con_vm(
        &format!("{TIPO_CLIENTE}.pedir_asincrono"),
        metodo_pedir_asincrono(guardian),
    );
    registro.registrar_funcion(
        &format!("{TIPO_CLIENTE}.enviar_archivo"),
        metodo_enviar_archivo(guardian),
    );
    registro.registrar_funcion_con_vm(
        &format!("{TIPO_CLIENTE}.enviar_archivo_asincrono"),
        metodo_enviar_archivo_asincrono(guardian),
    );
    registro.registrar_funcion(
        &format!("{TIPO_CLIENTE}.descargar"),
        metodo_descargar(guardian),
    );
    registro.registrar_funcion_con_vm(
        &format!("{TIPO_CLIENTE}.descargar_asincrono"),
        metodo_descargar_asincrono(guardian),
    );

    type MetodoRespuesta = fn(&[Valor]) -> Result<Valor, Fallo>;
    let metodos_respuesta: [(&str, MetodoRespuesta); 10] = [
        ("estado", metodo_estado),
        ("url", metodo_url),
        ("protocolo", metodo_protocolo),
        ("cabeceras", metodo_cabeceras),
        ("cabecera", metodo_cabecera_individual),
        ("cabeceras_todas", metodo_cabeceras_todas),
        ("bits", metodo_bits),
        ("texto", metodo_texto),
        ("jsn", metodo_jsn),
        ("peso", metodo_peso),
    ];
    for (nombre, funcion) in metodos_respuesta {
        registro.registrar_funcion(&format!("{TIPO_RESPUESTA}.{nombre}"), Box::new(funcion));
    }
}
