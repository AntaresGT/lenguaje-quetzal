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

use std::rc::Rc;
use std::time::Duration;

use indexmap::IndexMap;
use maquina_virtual::bucle_eventos::CargaNativa;
use maquina_virtual::{DatosInstanciaNativa, Fallo, Mensaje, RegistroNativos, Valor, Vm};
use runtime::GuardianPermisos;

use crate::util::{arg_texto, error, exigir_aridad};

use super::{bytes_y_tipo_contenido, metodo_http_desde_texto, permiso_denegado};

const TIPO_CLIENTE: &str = "ClienteHttp";
const TIPO_RESPUESTA: &str = "RespuestaHttp";

/// Tiempo de espera por defecto (segundos) de un `ClienteHttp` nuevo.
const TIEMPO_ESPERA_DEFECTO: i64 = 30;

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
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_CLIENTE),
        datos: std::cell::RefCell::new(datos),
    }))
}

fn receptor_cliente(funcion: &str, argumentos: &[Valor]) -> Result<Rc<DatosInstanciaNativa>, Fallo> {
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
fn config_cliente(instancia: &DatosInstanciaNativa) -> (Vec<(String, String)>, u64) {
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
    (cabeceras, tiempo_espera)
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
        _ => return Err(error("E0406", format!("'{F}' recibió un ClienteHttp sin cabeceras internas"))),
    }
    Ok(Valor::Nulo)
}

fn metodo_fijar_tiempo_espera(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ClienteHttp.fijar_tiempo_espera";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let instancia = receptor_cliente(F, argumentos)?;
    let segundos = match argumentos.get(1) {
        Some(Valor::Entero(segundos)) if *segundos >= 0 => *segundos,
        Some(otro) => {
            return Err(error(
                "E0406",
                format!(
                    "'{F}' espera un entero positivo de segundos, pero recibió '{}'",
                    otro.nombre_tipo()
                ),
            ));
        }
        None => return Err(error("E0210", format!("'{F}' necesita al menos 1 argumento"))),
    };
    instancia
        .datos
        .borrow_mut()
        .insert("tiempo_espera".to_string(), Valor::Entero(segundos));
    Ok(Valor::Nulo)
}

/// Petición ya validada y lista para enviarse: solo datos `Send`, para que
/// las variantes asincrónicas puedan lanzarla en el runtime de tokio del
/// bucle de eventos.
struct PeticionPreparada {
    metodo: reqwest::Method,
    url: String,
    cabeceras: Vec<(String, String)>,
    cuerpo: Vec<u8>,
    tiempo_espera: u64,
}

fn preparar_peticion(
    funcion: &str,
    guardian: &GuardianPermisos,
    receptor: &DatosInstanciaNativa,
    metodo: reqwest::Method,
    url: &str,
    cuerpo: Option<&Valor>,
) -> Result<PeticionPreparada, Fallo> {
    let url_analizada = reqwest::Url::parse(url)
        .map_err(|causa| error("E0406", format!("'{funcion}' recibió una URL inválida '{url}': {causa}")))?;
    let anfitrion = url_analizada.host_str().unwrap_or("").to_string();
    let puerto = url_analizada.port_or_known_default().unwrap_or(0);
    guardian
        .verificar_red_cliente(&anfitrion, puerto)
        .map_err(permiso_denegado)?;

    let (mut cabeceras, tiempo_espera) = config_cliente(receptor);
    let (cuerpo_bytes, tipo_contenido) = match cuerpo {
        Some(valor) => bytes_y_tipo_contenido(funcion, valor)?,
        None => (Vec::new(), None),
    };
    if let Some(tipo_contenido) = tipo_contenido
        && !cabeceras.iter().any(|(nombre, _)| nombre.eq_ignore_ascii_case("content-type"))
    {
        cabeceras.push(("Content-Type".to_string(), tipo_contenido.to_string()));
    }

    Ok(PeticionPreparada {
        metodo,
        url: url.to_string(),
        cabeceras,
        cuerpo: cuerpo_bytes,
        tiempo_espera,
    })
}

// ----- Ejecución síncrona -----

fn ejecutar_peticion_bloqueante(peticion: PeticionPreparada) -> Result<Valor, Fallo> {
    let cliente = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(peticion.tiempo_espera))
        .build()
        .map_err(|causa| error_red(format!("no se pudo crear el cliente HTTP: {causa}")))?;

    let mut peticion_construida = cliente.request(peticion.metodo, &peticion.url);
    for (nombre, valor) in &peticion.cabeceras {
        peticion_construida = peticion_construida.header(nombre, valor);
    }
    if !peticion.cuerpo.is_empty() {
        peticion_construida = peticion_construida.body(peticion.cuerpo);
    }

    let respuesta = peticion_construida
        .send()
        .map_err(|causa| error_red(format!("falló la petición a '{}': {causa}", peticion.url)))?;
    let estado = respuesta.status().as_u16();
    let cabeceras_respuesta: Vec<(String, String)> = respuesta
        .headers()
        .iter()
        .map(|(nombre, valor)| {
            (
                nombre.to_string(),
                valor.to_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    let cuerpo = respuesta
        .bytes()
        .map_err(|causa| error_red(format!("no se pudo leer la respuesta: {causa}")))?
        .to_vec();

    Ok(valor_respuesta_http(estado, cabeceras_respuesta, cuerpo))
}

fn valor_respuesta_http(estado: u16, cabeceras: Vec<(String, String)>, cuerpo: Vec<u8>) -> Valor {
    let mut mapa_cabeceras = IndexMap::new();
    for (nombre, valor) in cabeceras {
        mapa_cabeceras.insert(nombre, Valor::texto(valor));
    }
    let mut datos = IndexMap::new();
    datos.insert("estado".to_string(), Valor::Entero(i64::from(estado)));
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

async fn enviar_peticion_async(peticion: PeticionPreparada) -> Result<CargaNativa, String> {
    let cliente = reqwest::Client::builder()
        .timeout(Duration::from_secs(peticion.tiempo_espera))
        .build()
        .map_err(|causa| format!("no se pudo crear el cliente HTTP: {causa}"))?;

    let mut peticion_construida = cliente.request(peticion.metodo, &peticion.url);
    for (nombre, valor) in &peticion.cabeceras {
        peticion_construida = peticion_construida.header(nombre, valor);
    }
    if !peticion.cuerpo.is_empty() {
        peticion_construida = peticion_construida.body(peticion.cuerpo);
    }

    let respuesta = peticion_construida
        .send()
        .await
        .map_err(|causa| format!("falló la petición a '{}': {causa}", peticion.url))?;
    let estado = i64::from(respuesta.status().as_u16());
    let cabeceras: Vec<(String, CargaNativa)> = respuesta
        .headers()
        .iter()
        .map(|(nombre, valor)| {
            (
                nombre.to_string(),
                CargaNativa::Texto(valor.to_str().unwrap_or_default().to_string()),
            )
        })
        .collect();
    let cuerpo = respuesta
        .bytes()
        .await
        .map_err(|causa| format!("no se pudo leer la respuesta: {causa}"))?
        .to_vec();

    // Se reconstruye como una instancia real `RespuestaHttp` (no un `jsn`
    // genérico) para que `esperar` devuelva el mismo tipo que la variante
    // síncrona, con los mismos métodos (`.estado()`, `.texto()`, `.bits()`, ...).
    Ok(CargaNativa::Instancia {
        tipo: TIPO_RESPUESTA.to_string(),
        campos: vec![
            ("estado".to_string(), CargaNativa::Entero(estado)),
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
        let peticion = preparar_peticion(funcion, &guardian, &instancia, metodo_http.clone(), url, None)?;
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
        let peticion = preparar_peticion(funcion, &guardian, &instancia, metodo_http.clone(), url, cuerpo)?;
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
        let peticion = preparar_peticion(funcion, &guardian, &instancia, metodo_http.clone(), url, None)?;
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
        let peticion = preparar_peticion(funcion, &guardian, &instancia, metodo_http.clone(), url, cuerpo.as_ref())?;
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
        let peticion = preparar_peticion(F, &guardian, &instancia, metodo_http, url, cuerpo.as_ref())?;
        Ok(ejecutar_peticion_asincrona(vm, peticion))
    })
}

// ----- RespuestaHttp: métodos -----

fn receptor_respuesta(funcion: &str, argumentos: &[Valor]) -> Result<Rc<DatosInstanciaNativa>, Fallo> {
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
    String::from_utf8(bytes)
        .map(Valor::texto)
        .map_err(|_| error("E0406", format!("'{F}' no pudo decodificar el cuerpo como texto UTF-8 válido")))
}

fn metodo_jsn(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "RespuestaHttp.jsn";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let bytes = bytes_cuerpo(F, argumentos)?;
    let texto = String::from_utf8(bytes)
        .map_err(|_| error("E0406", format!("'{F}' no pudo decodificar el cuerpo como texto UTF-8 válido")))?;
    let json: serde_json::Value = serde_json::from_str(&texto)
        .map_err(|causa| error("E0406", format!("'{F}': el cuerpo no es JSON válido: {causa}")))?;
    Ok(maquina_virtual::valores::json_a_valor(&json))
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

    type MetodoRespuesta = fn(&[Valor]) -> Result<Valor, Fallo>;
    let metodos_respuesta: [(&str, MetodoRespuesta); 5] = [
        ("estado", metodo_estado),
        ("cabeceras", metodo_cabeceras),
        ("bits", metodo_bits),
        ("texto", metodo_texto),
        ("jsn", metodo_jsn),
    ];
    for (nombre, funcion) in metodos_respuesta {
        registro.registrar_funcion(&format!("{TIPO_RESPUESTA}.{nombre}"), Box::new(funcion));
    }
}
