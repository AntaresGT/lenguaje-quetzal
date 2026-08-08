//! Cliente HTTP de `quetzal/red`: `ClienteHttp`, `RespuestaHttp` y
//! `ProgresoPeticion`.
//!
//! Sigue el modelo mental de Axios con nombres en español: una instancia con
//! configuración por omisión, un método por verbo HTTP (incluido `consultar`,
//! el método QUERY del RFC 10008), interceptores de petición y de respuesta,
//! y avisos de progreso de subida y descarga con porcentaje.
//!
//! La transferencia ocurre siempre en un hilo aparte. La forma síncrona
//! espera ahí mismo; la forma `_asincrono` usa el bucle de eventos, así que
//! el resto del programa (por ejemplo, un `ServidorHttp` en escucha) sigue
//! atendiendo trabajo mientras la petición viaja.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::Read;
use std::rc::Rc;
use std::sync::mpsc::{Sender, channel};
use std::time::Duration;

use indexmap::IndexMap;
use maquina_virtual::{CargaNativa, Fallo, Mensaje, RegistroNativos, Valor, Vm, texto_de_valor};
use nucleo::Ubicacion;
use runtime::GuardianPermisos;

use crate::bits;
use crate::red::codigos;
use crate::red::formulario;
use crate::red::http;
use crate::red::metodos::{self, METODOS};
use crate::red::objetos::{
    campo, campo_de_carga, campo_texto, carga_de_bytes, error_permiso, error_red, error_tipo,
    instancia, jsn_de_pares, pares_de_jsn, receptor, texto_de_carga,
};
use crate::util::{arg_texto, error, exigir_aridad, f64_a_decimal};

pub(crate) const TIPO_CLIENTE: &str = "ClienteHttp";
pub(crate) const TIPO_RESPUESTA: &str = "RespuestaHttp";
pub(crate) const TIPO_PROGRESO: &str = "ProgresoPeticion";

/// Servicio con el que el bucle de eventos entrega los avisos de progreso.
const SERVICIO_PROGRESO: &str = "cliente_http_progreso";

/// Tamaño de los trozos con los que se leen y escriben los cuerpos.
const TROZO: usize = 16 * 1024;

// ----- Estado compartido -----

/// Interceptores registrados en una instancia de cliente.
#[derive(Default, Clone)]
struct Interceptores {
    peticion: Vec<Valor>,
    respuesta: Vec<Valor>,
}

/// Callbacks de progreso de una petición: (subida, descarga).
type ManejadoresProgreso = (Option<Valor>, Option<Valor>);

#[derive(Clone)]
pub(crate) struct RegistroCliente {
    configuraciones: Rc<RefCell<HashMap<i64, Valor>>>,
    interceptores: Rc<RefCell<HashMap<i64, Interceptores>>>,
    /// Callbacks de progreso de las peticiones asincrónicas en curso.
    progresos: Rc<RefCell<HashMap<u64, ManejadoresProgreso>>>,
    siguiente: Rc<Cell<i64>>,
    guardian: Rc<GuardianPermisos>,
}

impl RegistroCliente {
    fn nuevo(guardian: &Rc<GuardianPermisos>) -> Self {
        Self {
            configuraciones: Rc::new(RefCell::new(HashMap::new())),
            interceptores: Rc::new(RefCell::new(HashMap::new())),
            progresos: Rc::new(RefCell::new(HashMap::new())),
            siguiente: Rc::new(Cell::new(1)),
            guardian: Rc::clone(guardian),
        }
    }

    fn crear(&self, configuracion: Valor) -> i64 {
        let id = self.siguiente.get();
        self.siguiente.set(id + 1);
        self.configuraciones.borrow_mut().insert(id, configuracion);
        self.interceptores
            .borrow_mut()
            .insert(id, Interceptores::default());
        id
    }

    fn configuracion(&self, id: i64) -> Valor {
        self.configuraciones
            .borrow()
            .get(&id)
            .cloned()
            .unwrap_or_else(|| Valor::jsn(IndexMap::new()))
    }
}

// ----- Configuración de una petición -----

/// Petición ya resuelta y lista para viajar al hilo de transferencia.
struct Solicitud {
    metodo: String,
    url: String,
    cabeceras: Vec<(String, String)>,
    cuerpo: Option<Vec<u8>>,
    tiempo_limite: Option<Duration>,
    maximo_redirecciones: usize,
}

/// Resultado de una transferencia.
struct Transferencia {
    estado: i64,
    url_final: String,
    cabeceras: Vec<(String, String)>,
    cuerpo: Vec<u8>,
}

/// Aviso de progreso enviado desde el hilo de transferencia.
struct Progreso {
    direccion: &'static str,
    cargado: u64,
    total: Option<u64>,
    bytes: u64,
}

impl Progreso {
    fn carga(&self) -> CargaNativa {
        CargaNativa::Mapa(vec![
            (
                "direccion".to_string(),
                CargaNativa::Texto(self.direccion.to_string()),
            ),
            ("cargado".to_string(), CargaNativa::Entero(self.cargado as i64)),
            (
                "total".to_string(),
                match self.total {
                    Some(total) => CargaNativa::Entero(total as i64),
                    None => CargaNativa::Nula,
                },
            ),
            ("bytes".to_string(), CargaNativa::Entero(self.bytes as i64)),
        ])
    }
}

/// Mensajes del hilo de transferencia hacia el hilo de la VM (forma síncrona).
enum Aviso {
    Progreso(Progreso),
    Fin(Result<Transferencia, String>),
}

// ----- Registro de las funciones nativas -----

pub(crate) fn registrar(registro: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    let red = RegistroCliente::nuevo(guardian);

    for tipo in [TIPO_CLIENTE, TIPO_RESPUESTA, TIPO_PROGRESO] {
        registro.registrar_modulo(tipo);
    }

    let crear = red.clone();
    registro.registrar_funcion(
        &format!("{TIPO_CLIENTE}.constructor"),
        Box::new(move |argumentos| {
            const F: &str = "ClienteHttp";
            let configuracion = match argumentos.len() {
                0 => Valor::jsn(IndexMap::new()),
                1 => match &argumentos[0] {
                    Valor::Jsn(_) => argumentos[0].clone(),
                    otro => {
                        return Err(error_tipo(format!(
                            "'{F}' esperaba un jsn de configuración, pero recibió '{}'",
                            otro.nombre_tipo()
                        )));
                    }
                },
                _ => {
                    return Err(error(
                        "E0210",
                        format!("'{F}' espera () o (configuracion)"),
                    ));
                }
            };
            Ok(instancia_cliente(crear.crear(configuracion)))
        }),
    );

    registrar_metodos(registro, &red);
    registrar_ajustes(registro, &red);
    registrar_respuesta(registro);
    registrar_progreso(registro);
}

fn instancia_cliente(id: i64) -> Valor {
    instancia(
        TIPO_CLIENTE,
        vec![
            ("id", Valor::Entero(id)),
            ("texto", Valor::texto(format!("<ClienteHttp {id}>"))),
        ],
    )
}

/// Métodos por verbo (`obtener`, `publicar`, ...) y `solicitar`, cada uno con
/// su variante `_asincrono`.
fn registrar_metodos(registro: &mut RegistroNativos, red: &RegistroCliente) {
    for (espanol, verbo) in METODOS {
        for asincrono in [false, true] {
            let red = red.clone();
            let verbo = (*verbo).to_string();
            let sufijo = if asincrono { "_asincrono" } else { "" };
            let funcion = format!("{TIPO_CLIENTE}.{espanol}{sufijo}");
            let nombre = funcion.clone();
            registro.registrar_funcion_con_vm(
                &nombre,
                Box::new(move |vm: &mut Vm, argumentos: &[Valor]| {
                    let id = crate::red::objetos::id_receptor(&funcion, argumentos, TIPO_CLIENTE)?;
                    let url = arg_texto(&funcion, argumentos, 1)?.to_string();
                    let con_cuerpo = metodos::admite_cuerpo(&verbo);
                    let (datos, configuracion) = if con_cuerpo {
                        (
                            argumentos.get(2).cloned(),
                            argumentos.get(3).cloned(),
                        )
                    } else {
                        (None, argumentos.get(2).cloned())
                    };
                    let esperados = if con_cuerpo { 4 } else { 3 };
                    if argumentos.len() > esperados {
                        return Err(error(
                            "E0210",
                            format!(
                                "'{funcion}' espera {} argumentos como máximo",
                                esperados - 1
                            ),
                        ));
                    }

                    let peticion = combinar_configuracion(
                        &red,
                        id,
                        &verbo,
                        &url,
                        datos,
                        configuracion,
                    )?;
                    ejecutar(vm, &red, id, peticion, asincrono, &funcion)
                }),
            );
        }
    }

    for asincrono in [false, true] {
        let red = red.clone();
        let sufijo = if asincrono { "_asincrono" } else { "" };
        let funcion = format!("{TIPO_CLIENTE}.solicitar{sufijo}");
        let nombre = funcion.clone();
        registro.registrar_funcion_con_vm(
            &nombre,
            Box::new(move |vm: &mut Vm, argumentos: &[Valor]| {
                exigir_aridad(&funcion, &argumentos[1..], 1)?;
                let id = crate::red::objetos::id_receptor(&funcion, argumentos, TIPO_CLIENTE)?;
                let Valor::Jsn(_) = &argumentos[1] else {
                    return Err(error_tipo(format!(
                        "'{funcion}' esperaba un jsn de configuración, pero recibió '{}'",
                        argumentos[1].nombre_tipo()
                    )));
                };
                let peticion = fusionar(&red.configuracion(id), &argumentos[1]);
                ejecutar(vm, &red, id, peticion, asincrono, &funcion)
            }),
        );
    }
}

fn registrar_ajustes(registro: &mut RegistroNativos, red: &RegistroCliente) {
    let red_leer = red.clone();
    registro.registrar_funcion(
        &format!("{TIPO_CLIENTE}.configuracion"),
        Box::new(move |argumentos| {
            const F: &str = "ClienteHttp.configuracion";
            exigir_aridad(F, &argumentos[1..], 0)?;
            let id = crate::red::objetos::id_receptor(F, argumentos, TIPO_CLIENTE)?;
            Ok(red_leer.configuracion(id))
        }),
    );

    let red_configurar = red.clone();
    registro.registrar_funcion(
        &format!("{TIPO_CLIENTE}.configurar"),
        Box::new(move |argumentos| {
            const F: &str = "ClienteHttp.configurar";
            exigir_aridad(F, &argumentos[1..], 2)?;
            let id = crate::red::objetos::id_receptor(F, argumentos, TIPO_CLIENTE)?;
            let clave = arg_texto(F, argumentos, 1)?.to_string();
            if let Valor::Jsn(mapa) = red_configurar.configuracion(id) {
                mapa.borrow_mut().insert(clave, argumentos[2].clone());
            }
            Ok(argumentos[0].clone())
        }),
    );

    for (nombre, es_peticion) in [
        ("interceptar_peticion", true),
        ("interceptar_respuesta", false),
    ] {
        let red = red.clone();
        let funcion = format!("{TIPO_CLIENTE}.{nombre}");
        let nombre_funcion = funcion.clone();
        registro.registrar_funcion(
            &nombre_funcion,
            Box::new(move |argumentos| {
                exigir_aridad(&funcion, &argumentos[1..], 1)?;
                let id = crate::red::objetos::id_receptor(&funcion, argumentos, TIPO_CLIENTE)?;
                let Valor::Funcion(..) = &argumentos[1] else {
                    return Err(error_tipo(format!(
                        "'{funcion}' esperaba una función declarada en Quetzal, pero recibió '{}'",
                        argumentos[1].nombre_tipo()
                    )));
                };
                let mut interceptores = red.interceptores.borrow_mut();
                let entrada = interceptores.entry(id).or_default();
                if es_peticion {
                    entrada.peticion.push(argumentos[1].clone());
                } else {
                    entrada.respuesta.push(argumentos[1].clone());
                }
                Ok(argumentos[0].clone())
            }),
        );
    }
}

// ----- Composición de la configuración -----

/// Une la configuración de la instancia con la de la llamada.
fn combinar_configuracion(
    red: &RegistroCliente,
    id: i64,
    verbo: &str,
    url: &str,
    datos: Option<Valor>,
    configuracion: Option<Valor>,
) -> Result<Valor, Fallo> {
    let base = red.configuracion(id);
    let propia = match configuracion {
        Some(Valor::Jsn(mapa)) => Valor::Jsn(mapa),
        Some(Valor::Nulo) | None => Valor::jsn(IndexMap::new()),
        Some(otro) => {
            return Err(error_tipo(format!(
                "la configuración de la petición debe ser un jsn, pero se recibió '{}'",
                otro.nombre_tipo()
            )));
        }
    };
    let combinada = fusionar(&base, &propia);
    if let Valor::Jsn(mapa) = &combinada {
        let mut mapa = mapa.borrow_mut();
        mapa.insert("metodo".to_string(), Valor::texto(verbo));
        mapa.insert("url".to_string(), Valor::texto(url));
        if let Some(datos) = datos
            && !matches!(datos, Valor::Nulo)
        {
            mapa.insert("datos".to_string(), datos);
        }
    }
    Ok(combinada)
}

/// Copia superficial de dos configuraciones (la segunda gana).
fn fusionar(base: &Valor, propia: &Valor) -> Valor {
    let mut mapa = IndexMap::new();
    if let Valor::Jsn(base) = base {
        for (clave, valor) in base.borrow().iter() {
            mapa.insert(clave.clone(), valor.clone());
        }
    }
    if let Valor::Jsn(propia) = propia {
        for (clave, valor) in propia.borrow().iter() {
            // Las cabeceras se combinan clave por clave.
            if clave == "cabeceras"
                && let (Some(Valor::Jsn(previas)), Valor::Jsn(nuevas)) =
                    (mapa.get("cabeceras"), valor)
            {
                let mut combinadas = previas.borrow().clone();
                for (nombre, valor) in nuevas.borrow().iter() {
                    combinadas.insert(nombre.clone(), valor.clone());
                }
                mapa.insert(clave.clone(), Valor::jsn(combinadas));
                continue;
            }
            mapa.insert(clave.clone(), valor.clone());
        }
    }
    Valor::jsn(mapa)
}

fn leer(configuracion: &Valor, clave: &str) -> Valor {
    match configuracion {
        Valor::Jsn(mapa) => mapa.borrow().get(clave).cloned().unwrap_or(Valor::Nulo),
        _ => Valor::Nulo,
    }
}

fn leer_entero(configuracion: &Valor, clave: &str) -> Option<i64> {
    match leer(configuracion, clave) {
        Valor::Entero(entero) => Some(entero),
        _ => None,
    }
}

fn leer_texto(configuracion: &Valor, clave: &str) -> Option<String> {
    match leer(configuracion, clave) {
        Valor::Texto(texto) => Some(texto.to_string()),
        Valor::Nulo => None,
        otro => Some(texto_de_valor(&otro)),
    }
}

/// Prepara la solicitud: url final, cabeceras y cuerpo serializado.
fn preparar(red: &RegistroCliente, configuracion: &Valor) -> Result<Solicitud, Fallo> {
    let metodo = metodos::normalizar(&leer_texto(configuracion, "metodo").unwrap_or_default());
    let relativa = leer_texto(configuracion, "url").unwrap_or_default();
    let base = leer_texto(configuracion, "base_url").unwrap_or_default();
    let mut url = unir_url(&base, &relativa);

    // Parámetros de consulta.
    let parametros = pares_de_jsn(&leer(configuracion, "parametros"));
    if !parametros.is_empty() {
        let consulta: Vec<String> = parametros
            .iter()
            .map(|(clave, valor)| {
                format!(
                    "{}={}",
                    codificar_componente(clave),
                    codificar_componente(valor)
                )
            })
            .collect();
        let separador = if url.contains('?') { '&' } else { '?' };
        url = format!("{url}{separador}{}", consulta.join("&"));
    }

    let mut cabeceras = pares_de_jsn(&leer(configuracion, "cabeceras"));

    // Cuerpo: jsn y lista viajan como JSON; `Formulario` como multipart;
    // `Bits` y `Archivo` como binario; el resto como texto.
    let cuerpo = match leer(configuracion, "datos") {
        Valor::Nulo => None,
        Valor::Jsn(_) | Valor::Lista(_) => {
            let datos = leer(configuracion, "datos");
            if !cabeceras
                .iter()
                .any(|(clave, _)| clave.eq_ignore_ascii_case("content-type"))
            {
                cabeceras.push((
                    "Content-Type".to_string(),
                    "application/json; charset=utf-8".to_string(),
                ));
            }
            Some(
                maquina_virtual::valores::jsn_a_texto(&datos, false)
                    .into_bytes(),
            )
        }
        valor if formulario::es_formulario(&valor) => {
            let (frontera, cuerpo) = formulario::cuerpo_multipart(&valor)?;
            // La frontera es parte del tipo, así que reemplaza cualquier
            // `Content-Type` que ya viniera en la configuración.
            cabeceras.retain(|(clave, _)| !clave.eq_ignore_ascii_case("content-type"));
            cabeceras.push((
                "Content-Type".to_string(),
                format!("multipart/form-data; boundary={frontera}"),
            ));
            Some(cuerpo)
        }
        valor if formulario::es_archivo(&valor) => {
            let (bytes, nombre, tipo) =
                formulario::contenido_de_archivo("ClienteHttp", &red.guardian, &valor)?;
            cabeceras_binarias(&mut cabeceras, configuracion, &tipo, Some(&nombre));
            Some(bytes)
        }
        valor @ Valor::InstanciaNativa(_) => {
            let bytes = bits::arg_bits("ClienteHttp", std::slice::from_ref(&valor), 0)?;
            cabeceras_binarias(
                &mut cabeceras,
                configuracion,
                "application/octet-stream",
                None,
            );
            Some(bytes)
        }
        otro => {
            if !cabeceras
                .iter()
                .any(|(clave, _)| clave.eq_ignore_ascii_case("content-type"))
            {
                cabeceras.push((
                    "Content-Type".to_string(),
                    "text/plain; charset=utf-8".to_string(),
                ));
            }
            Some(texto_de_valor(&otro).into_bytes())
        }
    };

    if let Valor::Jsn(autenticacion) = leer(configuracion, "autenticacion") {
        let mapa = autenticacion.borrow();
        let usuario = mapa
            .get("usuario")
            .map(texto_de_valor)
            .unwrap_or_default();
        let clave = mapa.get("clave").map(texto_de_valor).unwrap_or_default();
        let credencial = base64_basico(&format!("{usuario}:{clave}"));
        cabeceras.push(("Authorization".to_string(), format!("Basic {credencial}")));
    }

    Ok(Solicitud {
        metodo,
        url,
        cabeceras,
        cuerpo,
        tiempo_limite: leer_entero(configuracion, "tiempo_limite")
            .filter(|limite| *limite > 0)
            .map(|limite| Duration::from_millis(limite as u64)),
        maximo_redirecciones: leer_entero(configuracion, "maximo_redirecciones")
            .filter(|maximo| *maximo >= 0)
            .unwrap_or(10) as usize,
    })
}

/// Cabeceras estándar de un cuerpo binario (`Bits` o `Archivo`): tipo MIME y,
/// cuando hay nombre, `Content-Disposition` con `filename`. El jsn auxiliar
/// de configuración puede forzar ambos con `tipo_contenido`, `nombre_archivo`
/// y `disposicion`.
fn cabeceras_binarias(
    cabeceras: &mut Vec<(String, String)>,
    configuracion: &Valor,
    tipo_por_omision: &str,
    nombre_por_omision: Option<&str>,
) {
    let tiene = |cabeceras: &Vec<(String, String)>, nombre: &str| {
        cabeceras
            .iter()
            .any(|(clave, _)| clave.eq_ignore_ascii_case(nombre))
    };

    match leer_texto(configuracion, "tipo_contenido") {
        Some(tipo) => {
            cabeceras.retain(|(clave, _)| !clave.eq_ignore_ascii_case("content-type"));
            cabeceras.push(("Content-Type".to_string(), http::tipo_mime(&tipo)));
        }
        None if !tiene(cabeceras, "content-type") => {
            cabeceras.push(("Content-Type".to_string(), tipo_por_omision.to_string()));
        }
        None => {}
    }

    if tiene(cabeceras, "content-disposition") {
        return;
    }
    let nombre = leer_texto(configuracion, "nombre_archivo")
        .or_else(|| nombre_por_omision.map(str::to_string));
    let disposicion = match leer_texto(configuracion, "disposicion").as_deref() {
        Some("inline") | Some("en_linea") => Some("inline"),
        Some(_) => Some("attachment"),
        None if nombre.is_some() => Some("attachment"),
        None => None,
    };
    if let Some(disposicion) = disposicion {
        let valor = match &nombre {
            Some(nombre) => format!(
                "{disposicion}; filename=\"{}\"",
                nombre.replace('\\', "\\\\").replace('"', "\\\"")
            ),
            None => disposicion.to_string(),
        };
        cabeceras.push(("Content-Disposition".to_string(), valor));
    }
}

fn base64_basico(texto: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(texto)
}

fn codificar_componente(texto: &str) -> String {
    use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
    utf8_percent_encode(texto, NON_ALPHANUMERIC).to_string()
}

fn unir_url(base: &str, relativa: &str) -> String {
    if base.is_empty() || relativa.starts_with("http://") || relativa.starts_with("https://") {
        return relativa.to_string();
    }
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        relativa.trim_start_matches('/')
    )
}

// ----- Ejecución -----

fn ejecutar(
    vm: &mut Vm,
    red: &RegistroCliente,
    id_cliente: i64,
    configuracion: Valor,
    asincrono: bool,
    funcion: &str,
) -> Result<Valor, Fallo> {
    let configuracion = aplicar_interceptores_peticion(vm, red, id_cliente, configuracion)?;
    let solicitud = preparar(red, &configuracion)?;
    verificar_permiso(red, &solicitud)?;

    let al_subir = funcion_opcional(&configuracion, "al_progreso_subida")?;
    let al_bajar = funcion_opcional(&configuracion, "al_progreso_descarga")?;

    let transferencia = if asincrono {
        ejecutar_asincrono(vm, red, solicitud, al_subir, al_bajar)?
    } else {
        ejecutar_sincrono(vm, solicitud, al_subir, al_bajar)?
    };

    let respuesta = instancia_respuesta(&configuracion, &transferencia);
    let respuesta = aplicar_interceptores_respuesta(vm, red, id_cliente, respuesta)?;
    validar_estado(&configuracion, &respuesta, funcion)?;
    Ok(respuesta)
}

fn funcion_opcional(configuracion: &Valor, clave: &str) -> Result<Option<Valor>, Fallo> {
    match leer(configuracion, clave) {
        Valor::Nulo => Ok(None),
        valor @ Valor::Funcion(..) => Ok(Some(valor)),
        otro => Err(error_tipo(format!(
            "'{clave}' debe ser una función declarada en Quetzal, pero se recibió '{}'",
            otro.nombre_tipo()
        ))),
    }
}

fn verificar_permiso(red: &RegistroCliente, solicitud: &Solicitud) -> Result<(), Fallo> {
    let analizada = url::Url::parse(&solicitud.url).map_err(|fallo| {
        error_red(format!(
            "la dirección '{}' no es una URL válida: {fallo}",
            solicitud.url
        ))
    })?;
    let destino = match analizada.port() {
        Some(puerto) => format!("{}:{puerto}", analizada.host_str().unwrap_or_default()),
        None => analizada.host_str().unwrap_or_default().to_string(),
    };
    red.guardian
        .verificar_red(&format!("conectarse a '{destino}'"))
        .map_err(error_permiso)
}

/// Forma síncrona: la transferencia corre en un hilo y el hilo de la VM
/// consume sus avisos de progreso mientras espera.
fn ejecutar_sincrono(
    vm: &mut Vm,
    solicitud: Solicitud,
    al_subir: Option<Valor>,
    al_bajar: Option<Valor>,
) -> Result<Transferencia, Fallo> {
    let (emisor, receptor_avisos) = channel();
    let reporta_subida = al_subir.is_some();
    let reporta_descarga = al_bajar.is_some();
    std::thread::spawn(move || {
        transferir(solicitud, &emisor, reporta_subida, reporta_descarga);
    });

    loop {
        match receptor_avisos.recv() {
            Ok(Aviso::Progreso(progreso)) => {
                let manejador = match progreso.direccion {
                    "subida" => al_subir.as_ref(),
                    _ => al_bajar.as_ref(),
                };
                if let Some(Valor::Funcion(funcion, entorno)) = manejador {
                    let valor = instancia_progreso(&progreso.carga());
                    vm.llamar_funcion(funcion, entorno, vec![valor], None, None)?;
                }
            }
            Ok(Aviso::Fin(Ok(transferencia))) => return Ok(transferencia),
            Ok(Aviso::Fin(Err(mensaje))) => return Err(error_red(mensaje)),
            Err(_) => {
                return Err(error_red(
                    "la transferencia terminó sin devolver una respuesta",
                ));
            }
        }
    }
}

/// Forma asincrónica: la transferencia usa el bucle de eventos, de modo que
/// el programa sigue atendiendo otro trabajo mientras la petición viaja.
fn ejecutar_asincrono(
    vm: &mut Vm,
    red: &RegistroCliente,
    solicitud: Solicitud,
    al_subir: Option<Valor>,
    al_bajar: Option<Valor>,
) -> Result<Transferencia, Fallo> {
    let id = vm.bucle().nuevo_id();
    let manija = vm.bucle().manija();
    let reporta_subida = al_subir.is_some();
    let reporta_descarga = al_bajar.is_some();
    red.progresos
        .borrow_mut()
        .insert(id, (al_subir, al_bajar));

    let red_despacho = red.clone();
    vm.registrar_despachador(SERVICIO_PROGRESO, move |vm, id_recurso, datos| {
        let manejadores = red_despacho
            .progresos
            .borrow()
            .get(&id_recurso)
            .cloned()
            .unwrap_or((None, None));
        let direccion = campo_de_carga(&datos, "direccion")
            .map(texto_de_carga)
            .unwrap_or_default();
        let manejador = if direccion == "subida" {
            manejadores.0
        } else {
            manejadores.1
        };
        if let Some(Valor::Funcion(funcion, entorno)) = manejador {
            let valor = instancia_progreso(&datos);
            let _ = vm.llamar_funcion(&funcion, &entorno, vec![valor], None, None);
        }
        CargaNativa::Nula
    });

    let manija_hilo = manija.clone();
    std::thread::spawn(move || {
        let (emisor, avisos) = channel();
        let hilo = std::thread::spawn(move || {
            transferir(solicitud, &emisor, reporta_subida, reporta_descarga);
        });
        let mut resultado = Err("la transferencia no devolvió resultado".to_string());
        while let Ok(aviso) = avisos.recv() {
            match aviso {
                Aviso::Progreso(progreso) => {
                    let (respuesta, espera) = channel();
                    manija_hilo.enviar(Mensaje::Solicitud {
                        servicio: SERVICIO_PROGRESO.to_string(),
                        id_recurso: id,
                        datos: progreso.carga(),
                        respuesta,
                    });
                    let _ = espera.recv();
                }
                Aviso::Fin(fin) => {
                    resultado = fin.map(|transferencia| carga_de_transferencia(&transferencia));
                }
            }
        }
        let _ = hilo.join();
        manija_hilo.enviar(Mensaje::TareaLista { id, resultado });
    });

    let tarea = Valor::TareaNativa(Rc::new(maquina_virtual::EstadoTareaNativa { id }));
    let resultado = vm.esperar(tarea, Ubicacion::nueva(0, 0));
    red.progresos.borrow_mut().remove(&id);
    let valor = resultado?;
    Ok(transferencia_de_valor(&valor))
}

fn carga_de_transferencia(transferencia: &Transferencia) -> CargaNativa {
    CargaNativa::Mapa(vec![
        (
            "estado".to_string(),
            CargaNativa::Entero(transferencia.estado),
        ),
        (
            "url".to_string(),
            CargaNativa::Texto(transferencia.url_final.clone()),
        ),
        (
            "cabeceras".to_string(),
            CargaNativa::Mapa(
                transferencia
                    .cabeceras
                    .iter()
                    .map(|(clave, valor)| (clave.clone(), CargaNativa::Texto(valor.clone())))
                    .collect(),
            ),
        ),
        ("cuerpo".to_string(), carga_de_bytes(&transferencia.cuerpo)),
    ])
}

/// Reconstruye la transferencia desde el `jsn` que produjo el bucle.
fn transferencia_de_valor(valor: &Valor) -> Transferencia {
    let estado = match leer(valor, "estado") {
        Valor::Entero(estado) => estado,
        _ => 0,
    };
    let cabeceras = pares_de_jsn(&leer(valor, "cabeceras"));
    let cuerpo = match leer(valor, "cuerpo") {
        Valor::Lista(lista) => lista
            .borrow()
            .iter()
            .filter_map(|valor| match valor {
                Valor::Entero(entero) => u8::try_from(*entero).ok(),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    Transferencia {
        estado,
        url_final: leer_texto(valor, "url").unwrap_or_default(),
        cabeceras,
        cuerpo,
    }
}

/// Hace la petición HTTP real. Corre siempre fuera del hilo de la VM.
fn transferir(
    solicitud: Solicitud,
    emisor: &Sender<Aviso>,
    reporta_subida: bool,
    reporta_descarga: bool,
) {
    let resultado = transferir_interno(solicitud, emisor, reporta_subida, reporta_descarga);
    let _ = emisor.send(Aviso::Fin(resultado));
}

fn transferir_interno(
    solicitud: Solicitud,
    emisor: &Sender<Aviso>,
    reporta_subida: bool,
    reporta_descarga: bool,
) -> Result<Transferencia, String> {
    let mut constructor = reqwest::blocking::Client::builder().redirect(
        if solicitud.maximo_redirecciones == 0 {
            reqwest::redirect::Policy::none()
        } else {
            reqwest::redirect::Policy::limited(solicitud.maximo_redirecciones)
        },
    );
    if let Some(limite) = solicitud.tiempo_limite {
        constructor = constructor.timeout(limite);
    }
    let cliente = constructor
        .build()
        .map_err(|fallo| format!("no se pudo crear el cliente HTTP: {fallo}"))?;

    let metodo = reqwest::Method::from_bytes(solicitud.metodo.as_bytes())
        .map_err(|_| format!("el método '{}' no es válido", solicitud.metodo))?;
    let mut peticion = cliente.request(metodo, &solicitud.url);
    for (clave, valor) in &solicitud.cabeceras {
        peticion = peticion.header(clave.as_str(), valor.as_str());
    }

    if let Some(cuerpo) = solicitud.cuerpo {
        let total = cuerpo.len() as u64;
        if reporta_subida {
            let lector = LectorProgreso {
                interior: std::io::Cursor::new(cuerpo),
                cargado: 0,
                total,
                emisor: emisor.clone(),
            };
            peticion = peticion.body(reqwest::blocking::Body::sized(lector, total));
        } else {
            peticion = peticion.body(cuerpo);
        }
    }

    let mut respuesta = peticion
        .send()
        .map_err(|fallo| format!("la petición a '{}' falló: {fallo}", solicitud.url))?;

    let estado = respuesta.status().as_u16() as i64;
    let url_final = respuesta.url().to_string();
    let cabeceras: Vec<(String, String)> = respuesta
        .headers()
        .iter()
        .map(|(nombre, valor)| {
            (
                nombre.to_string(),
                valor.to_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    let total = respuesta.content_length();

    let mut cuerpo = Vec::new();
    let mut buffer = vec![0u8; TROZO];
    loop {
        let leidos = respuesta
            .read(&mut buffer)
            .map_err(|fallo| format!("no se pudo leer la respuesta: {fallo}"))?;
        if leidos == 0 {
            break;
        }
        cuerpo.extend_from_slice(&buffer[..leidos]);
        if reporta_descarga {
            let _ = emisor.send(Aviso::Progreso(Progreso {
                direccion: "descarga",
                cargado: cuerpo.len() as u64,
                total,
                bytes: leidos as u64,
            }));
        }
    }
    if reporta_descarga && cuerpo.is_empty() {
        let _ = emisor.send(Aviso::Progreso(Progreso {
            direccion: "descarga",
            cargado: 0,
            total: Some(0),
            bytes: 0,
        }));
    }

    Ok(Transferencia {
        estado,
        url_final,
        cabeceras,
        cuerpo,
    })
}

/// Lector que informa cuánto lleva enviado del cuerpo de la petición.
struct LectorProgreso {
    interior: std::io::Cursor<Vec<u8>>,
    cargado: u64,
    total: u64,
    emisor: Sender<Aviso>,
}

impl Read for LectorProgreso {
    fn read(&mut self, destino: &mut [u8]) -> std::io::Result<usize> {
        let leidos = self.interior.read(destino)?;
        if leidos > 0 {
            self.cargado += leidos as u64;
            let _ = self.emisor.send(Aviso::Progreso(Progreso {
                direccion: "subida",
                cargado: self.cargado,
                total: Some(self.total),
                bytes: leidos as u64,
            }));
        }
        Ok(leidos)
    }
}

// ----- Interceptores y validación -----

fn aplicar_interceptores_peticion(
    vm: &mut Vm,
    red: &RegistroCliente,
    id_cliente: i64,
    configuracion: Valor,
) -> Result<Valor, Fallo> {
    let interceptores = red
        .interceptores
        .borrow()
        .get(&id_cliente)
        .map(|entrada| entrada.peticion.clone())
        .unwrap_or_default();
    let mut actual = configuracion;
    for interceptor in interceptores {
        let Valor::Funcion(funcion, entorno) = &interceptor else {
            continue;
        };
        let resultado = vm.llamar_funcion(funcion, entorno, vec![actual.clone()], None, None)?;
        if let Valor::Jsn(_) = resultado {
            actual = resultado;
        }
    }
    Ok(actual)
}

fn aplicar_interceptores_respuesta(
    vm: &mut Vm,
    red: &RegistroCliente,
    id_cliente: i64,
    respuesta: Valor,
) -> Result<Valor, Fallo> {
    let interceptores = red
        .interceptores
        .borrow()
        .get(&id_cliente)
        .map(|entrada| entrada.respuesta.clone())
        .unwrap_or_default();
    let mut actual = respuesta;
    for interceptor in interceptores {
        let Valor::Funcion(funcion, entorno) = &interceptor else {
            continue;
        };
        let resultado = vm.llamar_funcion(funcion, entorno, vec![actual.clone()], None, None)?;
        if let Valor::InstanciaNativa(datos) = &resultado
            && &*datos.tipo == TIPO_RESPUESTA
        {
            actual = resultado;
        }
    }
    Ok(actual)
}

fn validar_estado(configuracion: &Valor, respuesta: &Valor, funcion: &str) -> Result<(), Fallo> {
    if matches!(leer(configuracion, "validar_estado"), Valor::Log(false)) {
        return Ok(());
    }
    let estado = match campo(respuesta, "estado") {
        Valor::Entero(estado) => estado,
        _ => 0,
    };
    if (200..400).contains(&estado) {
        return Ok(());
    }
    let cuerpo = campo_texto(respuesta, "cuerpo_texto");
    let recorte: String = cuerpo.chars().take(200).collect();
    Err(error_red(format!(
        "'{funcion}' recibió el estado {estado}: {}{}",
        codigos::descripcion(estado),
        if recorte.is_empty() {
            String::new()
        } else {
            format!(" — {recorte}")
        }
    )))
}

// ----- Instancias de respuesta y progreso -----

fn instancia_respuesta(configuracion: &Valor, transferencia: &Transferencia) -> Valor {
    let texto = String::from_utf8_lossy(&transferencia.cuerpo).to_string();
    let cabecera = |nombre: &str| {
        transferencia
            .cabeceras
            .iter()
            .find(|(clave, _)| clave.eq_ignore_ascii_case(nombre))
            .map(|(_, valor)| valor.clone())
    };
    // La frontera multipart distingue mayúsculas, así que se lee del valor
    // original y solo el tipo se compara en minúsculas.
    let tipo_original = cabecera("content-type").unwrap_or_default();
    let tipo_contenido = tipo_original.to_ascii_lowercase();
    let frontera = formulario::frontera_de_tipo(&tipo_original);
    let nombre_archivo = cabecera("content-disposition")
        .and_then(|valor| formulario::nombre_en_disposicion(&valor));

    let tipo_respuesta = leer_texto(configuracion, "tipo_respuesta").unwrap_or_default();
    let datos = match tipo_respuesta.as_str() {
        "texto" => Valor::texto(&texto),
        "bits" => bits::instancia(&transferencia.cuerpo),
        "jsn" => analizar_json(&texto),
        "formulario" => match &frontera {
            Some(frontera) => formulario::desde_multipart(&transferencia.cuerpo, frontera),
            None => bits::instancia(&transferencia.cuerpo),
        },
        _ if frontera.is_some() => formulario::desde_multipart(
            &transferencia.cuerpo,
            frontera.as_deref().unwrap_or_default(),
        ),
        _ if tipo_contenido.contains("json") => analizar_json(&texto),
        _ if tipo_contenido.contains("x-www-form-urlencoded") => {
            jsn_de_pares(http::analizar_consulta(&texto))
        }
        _ if tipo_contenido.starts_with("text/") || tipo_contenido.is_empty() => {
            Valor::texto(&texto)
        }
        _ => bits::instancia(&transferencia.cuerpo),
    };

    let estado = transferencia.estado;
    instancia(
        TIPO_RESPUESTA,
        vec![
            ("estado", Valor::Entero(estado)),
            ("razon", Valor::texto(codigos::razon(estado))),
            ("descripcion", Valor::texto(codigos::descripcion(estado))),
            ("ok", Valor::Log((200..300).contains(&estado))),
            (
                "cabeceras",
                jsn_de_pares(transferencia.cabeceras.clone()),
            ),
            ("datos", datos),
            ("cuerpo_texto", Valor::texto(&texto)),
            ("bits", bits::instancia(&transferencia.cuerpo)),
            ("tipo_contenido", Valor::texto(&tipo_original)),
            (
                "nombre_archivo",
                match &nombre_archivo {
                    Some(nombre) => Valor::texto(nombre),
                    None => Valor::Nulo,
                },
            ),
            ("url", Valor::texto(&transferencia.url_final)),
            (
                "metodo",
                Valor::texto(leer_texto(configuracion, "metodo").unwrap_or_default()),
            ),
            (
                "texto",
                Valor::texto(format!(
                    "<RespuestaHttp {estado} {}>",
                    codigos::razon(estado)
                )),
            ),
        ],
    )
}

fn analizar_json(texto: &str) -> Valor {
    match serde_json::from_str::<serde_json::Value>(texto) {
        Ok(json) => maquina_virtual::valores::json_a_valor(&json),
        Err(_) => Valor::texto(texto),
    }
}

/// Construye el `ProgresoPeticion` que recibe el callback.
fn instancia_progreso(datos: &CargaNativa) -> Valor {
    let entero = |nombre: &str| match campo_de_carga(datos, nombre) {
        Some(CargaNativa::Entero(entero)) => Some(*entero),
        _ => None,
    };
    let direccion = campo_de_carga(datos, "direccion")
        .map(texto_de_carga)
        .unwrap_or_else(|| "descarga".to_string());
    let cargado = entero("cargado").unwrap_or(0);
    let total = entero("total");
    let bytes = entero("bytes").unwrap_or(0);

    let (progreso, porcentaje) = match total {
        Some(total) if total > 0 => {
            let fraccion = cargado as f64 / total as f64;
            (
                f64_a_decimal("ProgresoPeticion", fraccion).unwrap_or(Valor::Nulo),
                f64_a_decimal("ProgresoPeticion", (fraccion * 10000.0).round() / 100.0)
                    .unwrap_or(Valor::Nulo),
            )
        }
        _ => (Valor::Nulo, Valor::Nulo),
    };

    instancia(
        TIPO_PROGRESO,
        vec![
            ("direccion", Valor::texto(&direccion)),
            ("cargado", Valor::Entero(cargado)),
            (
                "total",
                match total {
                    Some(total) => Valor::Entero(total),
                    None => Valor::Nulo,
                },
            ),
            ("bytes", Valor::Entero(bytes)),
            ("progreso", progreso),
            ("porcentaje", porcentaje),
            (
                "es_subida",
                Valor::Log(direccion == "subida"),
            ),
            (
                "texto",
                Valor::texto(format!("<ProgresoPeticion {direccion} {cargado}>")),
            ),
        ],
    )
}

// ----- Métodos de RespuestaHttp y ProgresoPeticion -----

fn registrar_respuesta(registro: &mut RegistroNativos) {
    for metodo in [
        "estado",
        "razon",
        "descripcion",
        "ok",
        "cabeceras",
        "datos",
        "cuerpo_texto",
        "bits",
        "tipo_contenido",
        "nombre_archivo",
        "url",
        "metodo",
    ] {
        lector(registro, TIPO_RESPUESTA, metodo);
    }

    registro.registrar_funcion(
        &format!("{TIPO_RESPUESTA}.cabecera"),
        Box::new(move |argumentos| {
            const F: &str = "RespuestaHttp.cabecera";
            exigir_aridad(F, &argumentos[1..], 1)?;
            let respuesta = receptor(F, argumentos, TIPO_RESPUESTA)?;
            let nombre = arg_texto(F, argumentos, 1)?;
            Ok(
                crate::red::objetos::buscar_sin_caso(&campo(&respuesta, "cabeceras"), nombre)
                    .unwrap_or(Valor::Nulo),
            )
        }),
    );
}

fn registrar_progreso(registro: &mut RegistroNativos) {
    for metodo in [
        "direccion",
        "cargado",
        "total",
        "bytes",
        "progreso",
        "porcentaje",
        "es_subida",
    ] {
        lector(registro, TIPO_PROGRESO, metodo);
    }
}

/// Método que solo devuelve un campo guardado en la instancia.
fn lector(registro: &mut RegistroNativos, tipo: &'static str, metodo: &'static str) {
    let funcion = format!("{tipo}.{metodo}");
    let nombre_campo = metodo;
    registro.registrar_funcion(
        &funcion.clone(),
        Box::new(move |argumentos| {
            exigir_aridad(&funcion, &argumentos[1..], 0)?;
            let valor = receptor(&funcion, argumentos, tipo)?;
            Ok(campo(&valor, nombre_campo))
        }),
    );
}
