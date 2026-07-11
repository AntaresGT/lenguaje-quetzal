//! Lectura acotada y validación del framing de solicitudes HTTP/1.x.
//!
//! Este módulo no conoce la VM, rutas ni manejadores. Su única
//! responsabilidad es convertir bytes del transporte en una petición
//! estructurada o en un error HTTP seguro para devolver al cliente.

use std::io::{BufRead, BufReader, Read};

/// Límite seguro usado cuando el servidor no configura otro valor.
pub(crate) const LIMITE_CUERPO_PREDETERMINADO: usize = 2 * 1024 * 1024;

const LIMITE_LINEA_INICIAL: usize = 8 * 1024;
const LIMITE_LINEA_CABECERA: usize = 8 * 1024;
const LIMITE_SECCION_CABECERAS: usize = 64 * 1024;
const CANTIDAD_MAXIMA_CABECERAS: usize = 100;
const LIMITE_LINEA_TROZO: usize = 128;

/// Límites aplicados antes de reservar memoria para el cuerpo.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LimitesHttp {
    pub(crate) cuerpo: usize,
}

impl Default for LimitesHttp {
    fn default() -> Self {
        Self {
            cuerpo: LIMITE_CUERPO_PREDETERMINADO,
        }
    }
}

/// Solicitud validada, todavía sin interpretar rutas o parámetros.
#[derive(Debug)]
pub(crate) struct PeticionEntrante {
    pub(crate) metodo: String,
    pub(crate) objetivo: String,
    pub(crate) protocolo: String,
    pub(crate) cabeceras: Vec<(String, String)>,
    pub(crate) cuerpo: Vec<u8>,
    pub(crate) peso_declarado: Option<u64>,
}

/// Error de lectura transformable directamente en una respuesta HTTP.
#[derive(Debug)]
pub(crate) struct ErrorLecturaHttp {
    pub(crate) estado: u16,
    pub(crate) mensaje: String,
}

impl ErrorLecturaHttp {
    fn nuevo(estado: u16, mensaje: impl Into<String>) -> Self {
        Self {
            estado,
            mensaje: mensaje.into(),
        }
    }

    fn desde_io(error: std::io::Error, contexto: &str) -> Self {
        let estado = match error.kind() {
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => 408,
            _ => 400,
        };
        Self::nuevo(estado, format!("{contexto}: {error}"))
    }
}

/// Lee una solicitud. `Ok(None)` representa cierre limpio antes de recibir
/// la línea inicial.
pub(crate) fn leer_peticion(
    origen: impl Read,
    limites: LimitesHttp,
) -> Result<Option<PeticionEntrante>, ErrorLecturaHttp> {
    let mut lector = BufReader::new(origen);
    let Some(linea_inicial) = leer_linea_acotada(
        &mut lector,
        LIMITE_LINEA_INICIAL,
        414,
        "línea inicial demasiado grande",
    )?
    else {
        return Ok(None);
    };

    let (metodo, objetivo, protocolo) = analizar_linea_inicial(&linea_inicial)?;
    let cabeceras = leer_cabeceras(&mut lector, &protocolo)?;
    let framing = determinar_framing(&cabeceras, limites.cuerpo)?;
    let (cuerpo, peso_declarado) = match framing {
        FramingCuerpo::Vacio => (Vec::new(), None),
        FramingCuerpo::Longitud(longitud) => {
            let mut cuerpo = vec![0; longitud];
            lector
                .read_exact(&mut cuerpo)
                .map_err(|error| ErrorLecturaHttp::desde_io(error, "cuerpo HTTP incompleto"))?;
            (cuerpo, Some(longitud as u64))
        }
        FramingCuerpo::Trozos => (leer_cuerpo_por_trozos(&mut lector, limites.cuerpo)?, None),
    };

    Ok(Some(PeticionEntrante {
        metodo,
        objetivo,
        protocolo,
        cabeceras,
        cuerpo,
        peso_declarado,
    }))
}

fn leer_linea_acotada(
    lector: &mut impl BufRead,
    limite: usize,
    estado_exceso: u16,
    mensaje_exceso: &str,
) -> Result<Option<Vec<u8>>, ErrorLecturaHttp> {
    let mut bytes = Vec::with_capacity(limite.min(256));
    let leidos = lector
        .take((limite + 1) as u64)
        .read_until(b'\n', &mut bytes)
        .map_err(|error| ErrorLecturaHttp::desde_io(error, "no se pudo leer una línea HTTP"))?;
    if leidos == 0 {
        return Ok(None);
    }
    if bytes.len() > limite {
        return Err(ErrorLecturaHttp::nuevo(estado_exceso, mensaje_exceso));
    }
    if !bytes.ends_with(b"\r\n") {
        return Err(ErrorLecturaHttp::nuevo(
            400,
            "línea HTTP sin terminación CRLF",
        ));
    }
    bytes.truncate(bytes.len() - 2);
    Ok(Some(bytes))
}

fn analizar_linea_inicial(linea: &[u8]) -> Result<(String, String, String), ErrorLecturaHttp> {
    let texto = std::str::from_utf8(linea)
        .map_err(|_| ErrorLecturaHttp::nuevo(400, "línea inicial HTTP no es ASCII/UTF-8 válida"))?;
    let mut partes = texto.split(' ');
    let metodo = partes.next().unwrap_or_default();
    let objetivo = partes.next().unwrap_or_default();
    let protocolo = partes.next().unwrap_or_default();
    if metodo.is_empty() || objetivo.is_empty() || protocolo.is_empty() || partes.next().is_some() {
        return Err(ErrorLecturaHttp::nuevo(400, "línea inicial HTTP inválida"));
    }
    if !metodo.as_bytes().iter().copied().all(es_caracter_token) {
        return Err(ErrorLecturaHttp::nuevo(400, "método HTTP inválido"));
    }
    if !matches!(protocolo, "HTTP/1.0" | "HTTP/1.1") {
        return Err(ErrorLecturaHttp::nuevo(505, "versión HTTP no soportada"));
    }
    Ok((
        metodo.to_string(),
        objetivo.to_string(),
        protocolo.to_string(),
    ))
}

fn leer_cabeceras(
    lector: &mut impl BufRead,
    protocolo: &str,
) -> Result<Vec<(String, String)>, ErrorLecturaHttp> {
    let mut cabeceras = Vec::new();
    let mut bytes_totales = 0usize;
    loop {
        let linea = leer_linea_acotada(
            lector,
            LIMITE_LINEA_CABECERA,
            431,
            "línea de cabecera demasiado grande",
        )?
        .ok_or_else(|| ErrorLecturaHttp::nuevo(400, "sección de cabeceras incompleta"))?;
        bytes_totales = bytes_totales
            .checked_add(linea.len() + 2)
            .ok_or_else(|| ErrorLecturaHttp::nuevo(431, "sección de cabeceras demasiado grande"))?;
        if bytes_totales > LIMITE_SECCION_CABECERAS {
            return Err(ErrorLecturaHttp::nuevo(
                431,
                "sección de cabeceras demasiado grande",
            ));
        }
        if linea.is_empty() {
            break;
        }
        if cabeceras.len() >= CANTIDAD_MAXIMA_CABECERAS {
            return Err(ErrorLecturaHttp::nuevo(431, "demasiadas cabeceras HTTP"));
        }
        if matches!(linea.first(), Some(b' ' | b'\t')) {
            return Err(ErrorLecturaHttp::nuevo(
                400,
                "continuación obsoleta de cabecera no permitida",
            ));
        }
        let separador = linea
            .iter()
            .position(|byte| *byte == b':')
            .ok_or_else(|| ErrorLecturaHttp::nuevo(400, "cabecera HTTP sin separador ':'"))?;
        let nombre = &linea[..separador];
        let valor = &linea[separador + 1..];
        if nombre.is_empty() || !nombre.iter().copied().all(es_caracter_token) {
            return Err(ErrorLecturaHttp::nuevo(
                400,
                "nombre de cabecera HTTP inválido",
            ));
        }
        if valor
            .iter()
            .any(|byte| (*byte < 0x20 && *byte != b'\t') || *byte == 0x7f)
        {
            return Err(ErrorLecturaHttp::nuevo(
                400,
                "valor de cabecera HTTP inválido",
            ));
        }
        let nombre = std::str::from_utf8(nombre)
            .map_err(|_| ErrorLecturaHttp::nuevo(400, "nombre de cabecera HTTP no válido"))?;
        let valor = String::from_utf8_lossy(valor).trim().to_string();
        cabeceras.push((nombre.to_string(), valor));
    }

    if protocolo == "HTTP/1.1"
        && !cabeceras
            .iter()
            .any(|(nombre, valor)| nombre.eq_ignore_ascii_case("host") && !valor.is_empty())
    {
        return Err(ErrorLecturaHttp::nuevo(
            400,
            "petición HTTP/1.1 sin cabecera Host",
        ));
    }
    Ok(cabeceras)
}

fn es_caracter_token(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}

enum FramingCuerpo {
    Vacio,
    Longitud(usize),
    Trozos,
}

fn determinar_framing(
    cabeceras: &[(String, String)],
    limite_cuerpo: usize,
) -> Result<FramingCuerpo, ErrorLecturaHttp> {
    let longitudes: Vec<&str> = cabeceras
        .iter()
        .filter(|(nombre, _)| nombre.eq_ignore_ascii_case("content-length"))
        .map(|(_, valor)| valor.as_str())
        .collect();
    let transferencias: Vec<&str> = cabeceras
        .iter()
        .filter(|(nombre, _)| nombre.eq_ignore_ascii_case("transfer-encoding"))
        .map(|(_, valor)| valor.as_str())
        .collect();

    if !longitudes.is_empty() && !transferencias.is_empty() {
        return Err(ErrorLecturaHttp::nuevo(
            400,
            "Content-Length y Transfer-Encoding no pueden combinarse",
        ));
    }
    if !transferencias.is_empty() {
        let codificaciones: Vec<String> = transferencias
            .iter()
            .flat_map(|valor| valor.split(','))
            .map(|valor| valor.trim().to_ascii_lowercase())
            .filter(|valor| !valor.is_empty())
            .collect();
        if codificaciones.as_slice() != ["chunked"] {
            return Err(ErrorLecturaHttp::nuevo(
                501,
                "codificación de transferencia no soportada",
            ));
        }
        return Ok(FramingCuerpo::Trozos);
    }
    if longitudes.is_empty() {
        return Ok(FramingCuerpo::Vacio);
    }
    if longitudes.windows(2).any(|par| par[0] != par[1]) {
        return Err(ErrorLecturaHttp::nuevo(
            400,
            "cabeceras Content-Length contradictorias",
        ));
    }
    let longitud_u64 = longitudes[0]
        .parse::<u64>()
        .map_err(|_| ErrorLecturaHttp::nuevo(400, "Content-Length inválido"))?;
    if longitud_u64 > limite_cuerpo as u64 {
        return Err(ErrorLecturaHttp::nuevo(
            413,
            "cuerpo de la petición demasiado grande",
        ));
    }
    let longitud = usize::try_from(longitud_u64)
        .map_err(|_| ErrorLecturaHttp::nuevo(413, "cuerpo de la petición demasiado grande"))?;
    Ok(FramingCuerpo::Longitud(longitud))
}

fn leer_cuerpo_por_trozos(
    lector: &mut impl BufRead,
    limite_cuerpo: usize,
) -> Result<Vec<u8>, ErrorLecturaHttp> {
    let mut cuerpo = Vec::new();
    loop {
        let linea = leer_linea_acotada(
            lector,
            LIMITE_LINEA_TROZO,
            400,
            "línea de tamaño de trozo demasiado grande",
        )?
        .ok_or_else(|| ErrorLecturaHttp::nuevo(400, "cuerpo por trozos incompleto"))?;
        let tamano_hex = linea.split(|byte| *byte == b';').next().unwrap_or_default();
        let tamano_hex = std::str::from_utf8(tamano_hex)
            .map_err(|_| ErrorLecturaHttp::nuevo(400, "tamaño de trozo inválido"))?
            .trim();
        let tamano = u64::from_str_radix(tamano_hex, 16)
            .map_err(|_| ErrorLecturaHttp::nuevo(400, "tamaño de trozo inválido"))?;
        if tamano == 0 {
            leer_trailers(lector)?;
            return Ok(cuerpo);
        }
        let nuevo_total = (cuerpo.len() as u64).checked_add(tamano).ok_or_else(|| {
            ErrorLecturaHttp::nuevo(413, "cuerpo de la petición demasiado grande")
        })?;
        if nuevo_total > limite_cuerpo as u64 {
            return Err(ErrorLecturaHttp::nuevo(
                413,
                "cuerpo de la petición demasiado grande",
            ));
        }
        let tamano = usize::try_from(tamano)
            .map_err(|_| ErrorLecturaHttp::nuevo(413, "cuerpo de la petición demasiado grande"))?;
        let inicio = cuerpo.len();
        cuerpo.resize(inicio + tamano, 0);
        lector
            .read_exact(&mut cuerpo[inicio..])
            .map_err(|error| ErrorLecturaHttp::desde_io(error, "trozo HTTP incompleto"))?;
        let mut terminacion = [0u8; 2];
        lector
            .read_exact(&mut terminacion)
            .map_err(|error| ErrorLecturaHttp::desde_io(error, "trozo HTTP sin terminación"))?;
        if terminacion != *b"\r\n" {
            return Err(ErrorLecturaHttp::nuevo(
                400,
                "trozo HTTP sin terminación CRLF",
            ));
        }
    }
}

fn leer_trailers(lector: &mut impl BufRead) -> Result<(), ErrorLecturaHttp> {
    let mut total = 0usize;
    let mut cantidad = 0usize;
    loop {
        let linea = leer_linea_acotada(
            lector,
            LIMITE_LINEA_CABECERA,
            431,
            "trailer HTTP demasiado grande",
        )?
        .ok_or_else(|| ErrorLecturaHttp::nuevo(400, "trailers HTTP incompletos"))?;
        total = total
            .checked_add(linea.len() + 2)
            .ok_or_else(|| ErrorLecturaHttp::nuevo(431, "trailers HTTP demasiado grandes"))?;
        if total > LIMITE_SECCION_CABECERAS || cantidad >= CANTIDAD_MAXIMA_CABECERAS {
            return Err(ErrorLecturaHttp::nuevo(
                431,
                "trailers HTTP demasiado grandes",
            ));
        }
        if linea.is_empty() {
            return Ok(());
        }
        cantidad += 1;
        if !linea.contains(&b':') {
            return Err(ErrorLecturaHttp::nuevo(400, "trailer HTTP inválido"));
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn limites(cuerpo: usize) -> LimitesHttp {
        LimitesHttp { cuerpo }
    }

    #[test]
    fn deberia_rechazar_content_length_mayor_al_limite_antes_de_leer_cuerpo() {
        let peticion = b"POST / HTTP/1.1\r\nHost: local\r\nContent-Length: 99\r\n\r\n";
        let error = leer_peticion(peticion.as_slice(), limites(10)).unwrap_err();
        assert_eq!(error.estado, 413);
    }

    #[test]
    fn deberia_rechazar_framing_ambiguo() {
        let peticion = b"POST / HTTP/1.1\r\nHost: local\r\nContent-Length: 4\r\nTransfer-Encoding: chunked\r\n\r\n";
        let error = leer_peticion(peticion.as_slice(), limites(10)).unwrap_err();
        assert_eq!(error.estado, 400);
    }

    #[test]
    fn deberia_rechazar_content_length_con_valores_distintos() {
        let peticion =
            b"POST / HTTP/1.1\r\nHost: local\r\nContent-Length: 4\r\nContent-Length: 5\r\n\r\n";
        let error = leer_peticion(peticion.as_slice(), limites(10)).unwrap_err();
        assert_eq!(error.estado, 400);
    }

    #[test]
    fn deberia_reconstruir_cuerpo_por_trozos() {
        let peticion = b"POST / HTTP/1.1\r\nHost: local\r\nTransfer-Encoding: chunked\r\n\r\n4\r\nhola\r\n6\r\n mundo\r\n0\r\n\r\n";
        let resultado = leer_peticion(peticion.as_slice(), limites(20))
            .unwrap()
            .unwrap();
        assert_eq!(resultado.cuerpo, b"hola mundo");
    }

    #[test]
    fn deberia_conservar_protocolo_y_peso_declarado() {
        let peticion = b"POST / HTTP/1.0\r\nContent-Length: 4\r\n\r\nhola";
        let resultado = leer_peticion(peticion.as_slice(), limites(10))
            .unwrap()
            .unwrap();
        assert_eq!(
            (resultado.protocolo.as_str(), resultado.peso_declarado),
            ("HTTP/1.0", Some(4))
        );
    }
}
