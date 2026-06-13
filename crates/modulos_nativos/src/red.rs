//! Módulo nativo `quetzal/red`: peticiones HTTP simples.
//!
//! Cada operación consulta el [`GuardianPermisos`] del runtime; sin
//! `"red": {"habilitado": true}` en `quetzal.json` la operación falla con
//! `E0701`.

use std::rc::Rc;
use std::time::Duration;

use maquina_virtual::{RegistroNativos, Valor};
use runtime::GuardianPermisos;

use crate::util::{arg_texto, error, exigir_aridad};

fn permiso_denegado(mensaje: String) -> maquina_virtual::Fallo {
    error("E0701", mensaje)
}

fn cliente() -> Result<reqwest::blocking::Client, maquina_virtual::Fallo> {
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

    // red.obtener(url) -> texto (HTTP GET)
    let permisos = Rc::clone(guardian);
    registro.registrar_funcion(
        "red.obtener",
        Box::new(move |argumentos| {
            exigir_aridad("red.obtener", argumentos, 1)?;
            let url = arg_texto("red.obtener", argumentos, 0)?;
            permisos.verificar_red().map_err(permiso_denegado)?;
            cuerpo_respuesta(url, cliente()?.get(url).send())
        }),
    );

    // red.enviar(url, cuerpo) -> texto (HTTP POST con cuerpo de texto)
    let permisos = Rc::clone(guardian);
    registro.registrar_funcion(
        "red.enviar",
        Box::new(move |argumentos| {
            exigir_aridad("red.enviar", argumentos, 2)?;
            let url = arg_texto("red.enviar", argumentos, 0)?;
            let cuerpo = arg_texto("red.enviar", argumentos, 1)?.to_string();
            permisos.verificar_red().map_err(permiso_denegado)?;
            cuerpo_respuesta(url, cliente()?.post(url).body(cuerpo).send())
        }),
    );
}
