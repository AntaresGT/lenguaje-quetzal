//! Módulo nativo `quetzal/red`: interconexiones (HTTP, sockets, ...).
//!
//! Cada operación consulta el [`GuardianPermisos`] del runtime; sin permiso
//! de red en `quetzal.json` la operación falla con `E0701`. Ver
//! `crates/runtime/src/permisos.rs` para la forma granular
//! (`cliente`/`servidor`/`anfitriones`/`puertos`).
//!
//! - `red.obtener` / `red.enviar`: funciones simples originales (se
//!   mantienen intactas por compatibilidad).
//! - [`ClienteHttp`](cliente_http): cliente HTTP completo, con variantes
//!   asincrónicas que no bloquean el bucle de eventos.
//! - [`ServidorHttp`](servidor_http): servidor HTTP con rutas en español y
//!   manejadores de Quetzal pasados por referencia, atendidos por el bucle
//!   de eventos (patrón reactor).
//! - [`Socket`/`ServidorSocket`](socket): conexiones TCP crudas con
//!   transferencia binaria directa vía `Bits`, mismo patrón reactor.
//! - [`SocketUdp`](socket_udp): datagramas UDP con transferencia binaria
//!   directa vía `Bits`.
//! - [`ClienteRs`/`ServidorRs`](socket_rs): RedSocket (equivalente en
//!   español de WebSocket) con transferencia binaria directa vía `Bits`,
//!   mismo patrón reactor.

pub mod cliente_http;
mod protocolo_http;
pub mod servidor_http;
pub mod socket;
pub mod socket_rs;
pub mod socket_udp;

use std::rc::Rc;
use std::time::Duration;

use maquina_virtual::{Fallo, RegistroNativos, Valor};
use runtime::GuardianPermisos;

use crate::util::{arg_texto, error, exigir_aridad};

pub(crate) fn permiso_denegado(mensaje: String) -> Fallo {
    error("E0701", mensaje)
}

/// Interpreta un método HTTP en español o inglés (`OBTENER`/`GET`, ...).
/// Compartido entre el cliente (verbo de la petición) y el servidor (verbo
/// de la ruta registrada).
pub(crate) fn metodo_http_desde_texto(
    funcion: &str,
    texto: &str,
) -> Result<reqwest::Method, Fallo> {
    match maquina_virtual::normalizar_nombre(texto)
        .to_uppercase()
        .as_str()
    {
        "OBTENER" | "GET" => Ok(reqwest::Method::GET),
        "PUBLICAR" | "POST" => Ok(reqwest::Method::POST),
        "PONER" | "PUT" => Ok(reqwest::Method::PUT),
        "PARCHAR" | "PATCH" => Ok(reqwest::Method::PATCH),
        "ELIMINAR" | "DELETE" => Ok(reqwest::Method::DELETE),
        "CABEZA" | "HEAD" => Ok(reqwest::Method::HEAD),
        "OPCIONES" | "OPTIONS" => Ok(reqwest::Method::OPTIONS),
        otro => Err(error(
            "E0406",
            format!(
                "'{funcion}' no reconoce el método '{otro}' (usa OBTENER, PUBLICAR, PONER, PARCHAR, ELIMINAR, CABEZA u OPCIONES)"
            ),
        )),
    }
}

/// Bytes y tipo de contenido de un valor usado como cuerpo HTTP: texto, jsn
/// (serializado como JSON), `Bits` (binario) o nulo (sin cuerpo). Compartido
/// entre el cuerpo de una petición del cliente y el cuerpo de una respuesta
/// del servidor.
pub(crate) fn bytes_y_tipo_contenido(
    funcion: &str,
    valor: &Valor,
) -> Result<(Vec<u8>, Option<&'static str>), Fallo> {
    match valor {
        Valor::Nulo => Ok((Vec::new(), None)),
        Valor::Texto(texto) => Ok((texto.as_bytes().to_vec(), Some("text/plain; charset=utf-8"))),
        Valor::Jsn(_) => Ok((
            maquina_virtual::valores::jsn_a_texto(valor, false).into_bytes(),
            Some("application/json"),
        )),
        otro => match maquina_virtual::bytes_de_bits(otro) {
            Some(bytes) => Ok((bytes, Some("application/octet-stream"))),
            None => Err(error(
                "E0406",
                format!(
                    "'{funcion}' espera texto, jsn, Bits o nulo, pero recibió '{}'",
                    otro.nombre_tipo()
                ),
            )),
        },
    }
}

fn cliente_bloqueante() -> Result<reqwest::blocking::Client, maquina_virtual::Fallo> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|fallo| {
            error(
                "E0703",
                format!("no se pudo crear el cliente HTTP: {fallo}"),
            )
        })
}

fn cuerpo_respuesta(
    url: &str,
    respuesta: Result<reqwest::blocking::Response, reqwest::Error>,
) -> Result<Valor, maquina_virtual::Fallo> {
    let respuesta = respuesta
        .map_err(|fallo| error("E0703", format!("falló la petición a '{url}': {fallo}")))?;
    let texto = respuesta.text().map_err(|fallo| {
        error(
            "E0703",
            format!("no se pudo leer la respuesta de '{url}': {fallo}"),
        )
    })?;
    Ok(Valor::texto(texto))
}

pub fn registrar(registro: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    registro.registrar_modulo("red");

    // red.obtener(url) -> texto (HTTP GET) — API original, sin cambios.
    let permisos = Rc::clone(guardian);
    registro.registrar_funcion(
        "red.obtener",
        Box::new(move |argumentos| {
            exigir_aridad("red.obtener", argumentos, 1)?;
            let url = arg_texto("red.obtener", argumentos, 0)?;
            permisos.verificar_red().map_err(permiso_denegado)?;
            cuerpo_respuesta(url, cliente_bloqueante()?.get(url).send())
        }),
    );

    // red.enviar(url, cuerpo) -> texto (HTTP POST con cuerpo de texto) — API
    // original, sin cambios.
    let permisos = Rc::clone(guardian);
    registro.registrar_funcion(
        "red.enviar",
        Box::new(move |argumentos| {
            exigir_aridad("red.enviar", argumentos, 2)?;
            let url = arg_texto("red.enviar", argumentos, 0)?;
            let cuerpo = arg_texto("red.enviar", argumentos, 1)?.to_string();
            permisos.verificar_red().map_err(permiso_denegado)?;
            cuerpo_respuesta(url, cliente_bloqueante()?.post(url).body(cuerpo).send())
        }),
    );

    cliente_http::registrar(registro, guardian);
    servidor_http::registrar(registro, guardian);
    socket::registrar(registro, guardian);
    socket_udp::registrar(registro, guardian);
    socket_rs::registrar(registro, guardian);
}
