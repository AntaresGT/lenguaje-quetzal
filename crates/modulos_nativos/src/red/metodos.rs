//! Traducción entre los métodos HTTP en español y los verbos del protocolo.
//!
//! La API de `quetzal/red` se escribe en español (`obtener`, `publicar`,
//! `consultar`, ...) mientras que por el cable siempre viajan los verbos
//! estándar en mayúsculas (`GET`, `POST`, `QUERY`, ...). `consultar`
//! corresponde al método QUERY definido en el RFC 10008: seguro e
//! idempotente, con el contenido de la consulta en el cuerpo.

/// Métodos expuestos por el enrutador y el cliente, en el orden en que se
/// documentan: (nombre en español, verbo HTTP).
pub(crate) const METODOS: &[(&str, &str)] = &[
    ("obtener", "GET"),
    ("publicar", "POST"),
    ("poner", "PUT"),
    ("parchear", "PATCH"),
    ("borrar", "DELETE"),
    ("cabecera", "HEAD"),
    ("opciones", "OPTIONS"),
    ("consultar", "QUERY"),
    ("rastrear", "TRACE"),
    ("conectar", "CONNECT"),
];

/// Verbo HTTP del nombre en español (`publicar` → `POST`).
pub(crate) fn verbo(nombre_espanol: &str) -> Option<&'static str> {
    METODOS
        .iter()
        .find(|(espanol, _)| *espanol == nombre_espanol)
        .map(|(_, verbo)| *verbo)
}

/// Nombre en español del verbo HTTP (`POST` → `publicar`). Un verbo
/// desconocido se devuelve tal cual, en minúsculas.
pub(crate) fn espanol(verbo: &str) -> String {
    let verbo = verbo.to_ascii_uppercase();
    METODOS
        .iter()
        .find(|(_, estandar)| *estandar == verbo)
        .map(|(espanol, _)| (*espanol).to_string())
        .unwrap_or_else(|| verbo.to_ascii_lowercase())
}

/// Normaliza lo que escribió el programa: acepta tanto el nombre en español
/// (`consultar`) como el verbo HTTP (`QUERY`, `query`).
pub(crate) fn normalizar(metodo: &str) -> String {
    verbo(&metodo.to_ascii_lowercase())
        .map(|verbo| verbo.to_string())
        .unwrap_or_else(|| metodo.to_ascii_uppercase())
}

/// Si el método es seguro (no modifica el recurso) según el RFC 9110 y el
/// RFC 10008 para `QUERY`.
pub(crate) fn es_seguro(verbo: &str) -> bool {
    matches!(
        verbo.to_ascii_uppercase().as_str(),
        "GET" | "HEAD" | "OPTIONS" | "TRACE" | "QUERY"
    )
}

/// Si el método es idempotente según el RFC 9110 y el RFC 10008.
pub(crate) fn es_idempotente(verbo: &str) -> bool {
    matches!(
        verbo.to_ascii_uppercase().as_str(),
        "GET" | "HEAD" | "OPTIONS" | "TRACE" | "PUT" | "DELETE" | "QUERY"
    )
}
