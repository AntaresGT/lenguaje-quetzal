//! Lectura y escritura de mensajes HTTP/1.1 para el servidor de
//! `quetzal/red`, más las utilidades de URL que comparte con el cliente.
//!
//! El servidor habla HTTP/1.1 sobre `std::net`: cada conexión se atiende en
//! un hilo que lee la petición con estas funciones, la manda al hilo de la
//! VM por el bucle de eventos y escribe de vuelta la respuesta que produjo
//! el manejador de Quetzal.

use std::io::{BufRead, Write};

use percent_encoding::percent_decode_str;

/// Tamaño máximo de la línea de petición y de cada cabecera.
const LIMITE_LINEA: usize = 16 * 1024;
/// Tamaño máximo del cuerpo aceptado por el servidor (10 MiB).
const LIMITE_CUERPO: usize = 10 * 1024 * 1024;

/// Una petición HTTP tal como llegó por la conexión.
pub(crate) struct PeticionCruda {
    pub metodo: String,
    /// Objetivo completo (`/ruta?a=1`), tal como lo envió el cliente.
    pub destino: String,
    pub ruta: String,
    pub consulta: String,
    pub version: String,
    pub cabeceras: Vec<(String, String)>,
    pub cuerpo: Vec<u8>,
}

impl PeticionCruda {
    pub fn cabecera(&self, nombre: &str) -> Option<&str> {
        self.cabeceras
            .iter()
            .find(|(clave, _)| clave.eq_ignore_ascii_case(nombre))
            .map(|(_, valor)| valor.as_str())
    }

    /// Si el cliente pidió mantener la conexión abierta.
    pub fn conexion_persistente(&self) -> bool {
        match self.cabecera("connection") {
            Some(valor) if valor.eq_ignore_ascii_case("close") => false,
            Some(valor) if valor.eq_ignore_ascii_case("keep-alive") => true,
            _ => self.version != "HTTP/1.0",
        }
    }
}

/// Lee una petición completa. Devuelve `None` cuando la conexión se cerró
/// limpiamente antes de enviar nada.
pub(crate) fn leer_peticion<L: BufRead>(lector: &mut L) -> Result<Option<PeticionCruda>, String> {
    let Some(linea) = leer_linea(lector)? else {
        return Ok(None);
    };
    if linea.trim().is_empty() {
        return Ok(None);
    }

    let mut partes = linea.trim_end().splitn(3, ' ');
    let metodo = partes
        .next()
        .filter(|metodo| !metodo.is_empty())
        .ok_or("la línea de petición no trae método")?
        .to_string();
    let destino = partes
        .next()
        .ok_or("la línea de petición no trae objetivo")?
        .to_string();
    let version = partes.next().unwrap_or("HTTP/1.1").trim().to_string();

    let mut cabeceras = Vec::new();
    loop {
        let Some(linea) = leer_linea(lector)? else {
            break;
        };
        let linea = linea.trim_end_matches(['\r', '\n']);
        if linea.is_empty() {
            break;
        }
        if let Some((clave, valor)) = linea.split_once(':') {
            cabeceras.push((clave.trim().to_string(), valor.trim().to_string()));
        }
    }

    let (ruta, consulta) = match destino.split_once('?') {
        Some((ruta, consulta)) => (ruta.to_string(), consulta.to_string()),
        None => (destino.clone(), String::new()),
    };

    let mut peticion = PeticionCruda {
        metodo,
        destino,
        ruta,
        consulta,
        version,
        cabeceras,
        cuerpo: Vec::new(),
    };
    peticion.cuerpo = leer_cuerpo(lector, &peticion)?;
    Ok(Some(peticion))
}

fn leer_linea<L: BufRead>(lector: &mut L) -> Result<Option<String>, String> {
    let mut bytes = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        match lector.read(&mut byte) {
            Ok(0) => {
                return if bytes.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(String::from_utf8_lossy(&bytes).to_string()))
                };
            }
            Ok(_) => {
                if byte[0] == b'\n' {
                    return Ok(Some(String::from_utf8_lossy(&bytes).to_string()));
                }
                bytes.push(byte[0]);
                if bytes.len() > LIMITE_LINEA {
                    return Err("la cabecera de la petición es demasiado larga".to_string());
                }
            }
            Err(fallo) => return Err(format!("no se pudo leer la petición: {fallo}")),
        }
    }
}

fn leer_cuerpo<L: BufRead>(lector: &mut L, peticion: &PeticionCruda) -> Result<Vec<u8>, String> {
    if peticion
        .cabecera("transfer-encoding")
        .map(|valor| valor.to_ascii_lowercase().contains("chunked"))
        .unwrap_or(false)
    {
        return leer_cuerpo_por_trozos(lector);
    }

    let longitud = match peticion.cabecera("content-length") {
        Some(valor) => valor
            .trim()
            .parse::<usize>()
            .map_err(|_| "la cabecera Content-Length no es un número".to_string())?,
        None => return Ok(Vec::new()),
    };
    if longitud > LIMITE_CUERPO {
        return Err(format!(
            "el cuerpo de la petición supera el límite de {LIMITE_CUERPO} bytes"
        ));
    }
    let mut cuerpo = vec![0u8; longitud];
    lector
        .read_exact(&mut cuerpo)
        .map_err(|fallo| format!("no se pudo leer el cuerpo de la petición: {fallo}"))?;
    Ok(cuerpo)
}

fn leer_cuerpo_por_trozos<L: BufRead>(lector: &mut L) -> Result<Vec<u8>, String> {
    let mut cuerpo = Vec::new();
    loop {
        let Some(linea) = leer_linea(lector)? else {
            break;
        };
        let tamano_texto = linea.trim().split(';').next().unwrap_or("").trim();
        if tamano_texto.is_empty() {
            continue;
        }
        let tamano = usize::from_str_radix(tamano_texto, 16)
            .map_err(|_| "el tamaño de un trozo no es hexadecimal válido".to_string())?;
        if tamano == 0 {
            // Cabeceras finales (trailers) hasta la línea vacía.
            while let Some(linea) = leer_linea(lector)? {
                if linea.trim().is_empty() {
                    break;
                }
            }
            break;
        }
        if cuerpo.len() + tamano > LIMITE_CUERPO {
            return Err(format!(
                "el cuerpo de la petición supera el límite de {LIMITE_CUERPO} bytes"
            ));
        }
        let mut trozo = vec![0u8; tamano];
        lector
            .read_exact(&mut trozo)
            .map_err(|fallo| format!("no se pudo leer el cuerpo de la petición: {fallo}"))?;
        cuerpo.extend_from_slice(&trozo);
        // Salto de línea que cierra el trozo.
        let _ = leer_linea(lector)?;
    }
    Ok(cuerpo)
}

/// Escribe una respuesta HTTP/1.1 completa en la conexión.
pub(crate) fn escribir_respuesta<E: Write>(
    escritor: &mut E,
    estado: i64,
    razon: &str,
    cabeceras: &[(String, String)],
    cuerpo: &[u8],
    persistente: bool,
    incluir_cuerpo: bool,
) -> Result<(), String> {
    let mut salida = Vec::new();
    salida.extend_from_slice(format!("HTTP/1.1 {estado} {razon}\r\n").as_bytes());
    for (clave, valor) in cabeceras {
        salida.extend_from_slice(format!("{clave}: {valor}\r\n").as_bytes());
    }
    salida.extend_from_slice(format!("Content-Length: {}\r\n", cuerpo.len()).as_bytes());
    salida.extend_from_slice(
        if persistente {
            "Connection: keep-alive\r\n"
        } else {
            "Connection: close\r\n"
        }
        .as_bytes(),
    );
    salida.extend_from_slice(b"\r\n");
    if incluir_cuerpo {
        salida.extend_from_slice(cuerpo);
    }
    escritor
        .write_all(&salida)
        .and_then(|_| escritor.flush())
        .map_err(|fallo| format!("no se pudo escribir la respuesta: {fallo}"))
}

// ----- Utilidades de URL -----

/// Decodifica los `%XX` de un componente de URL.
pub(crate) fn decodificar(texto: &str) -> String {
    percent_decode_str(texto).decode_utf8_lossy().to_string()
}

/// Decodifica un valor de la cadena de consulta (donde `+` es un espacio).
pub(crate) fn decodificar_consulta(texto: &str) -> String {
    decodificar(&texto.replace('+', " "))
}

/// Separa una cadena de consulta (`a=1&b=2`) en pares ya decodificados.
pub(crate) fn analizar_consulta(consulta: &str) -> Vec<(String, String)> {
    consulta
        .split('&')
        .filter(|par| !par.is_empty())
        .map(|par| match par.split_once('=') {
            Some((clave, valor)) => (decodificar_consulta(clave), decodificar_consulta(valor)),
            None => (decodificar_consulta(par), String::new()),
        })
        .collect()
}

/// Separa la cabecera `Cookie` en pares nombre/valor.
pub(crate) fn analizar_galletas(cabecera: &str) -> Vec<(String, String)> {
    cabecera
        .split(';')
        .filter_map(|par| par.trim().split_once('='))
        .map(|(nombre, valor)| (nombre.trim().to_string(), decodificar(valor.trim())))
        .collect()
}

/// Tipo de contenido asociado a una extensión de archivo.
pub(crate) fn tipo_por_extension(extension: &str) -> &'static str {
    match extension.to_ascii_lowercase().as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "txt" | "md" => "text/plain; charset=utf-8",
        "csv" => "text/csv; charset=utf-8",
        "xml" => "application/xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}

/// Expande un tipo abreviado (`jsn`, `html`, `texto`) al tipo MIME completo;
/// si ya trae `/`, se usa tal cual. Equivale a `res.type()` de Express.
pub(crate) fn tipo_mime(tipo: &str) -> String {
    if tipo.contains('/') {
        return tipo.to_string();
    }
    match tipo.to_ascii_lowercase().as_str() {
        "jsn" | "json" => "application/json; charset=utf-8".to_string(),
        "texto" | "text" | "txt" => "text/plain; charset=utf-8".to_string(),
        "html" => "text/html; charset=utf-8".to_string(),
        "formulario" | "form" => "application/x-www-form-urlencoded".to_string(),
        otro => tipo_por_extension(otro).to_string(),
    }
}
