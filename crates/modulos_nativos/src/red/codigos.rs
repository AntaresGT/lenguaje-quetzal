//! Objeto `HttpCodigos` del módulo nativo `quetzal/red`.
//!
//! Traduce los códigos de estado HTTP a español: su descripción, su frase de
//! razón (la que viaja en la respuesta, en inglés) y su categoría. La tabla
//! sigue la referencia de códigos de estado de MDN.

use maquina_virtual::{RegistroNativos, Valor};

use crate::util::{arg_entero, error, exigir_aridad};

/// Nombre del objeto visible para el usuario.
pub(crate) const TIPO: &str = "HttpCodigos";

/// Un código de estado con su frase de razón (protocolo) y su descripción.
struct Codigo {
    numero: i64,
    razon: &'static str,
    descripcion: &'static str,
}

/// Tabla de códigos de estado según la referencia de MDN.
const CODIGOS: &[Codigo] = &[
    Codigo { numero: 100, razon: "Continue", descripcion: "Continuar: el servidor recibió las cabeceras y el cliente debe enviar el cuerpo" },
    Codigo { numero: 101, razon: "Switching Protocols", descripcion: "Cambiando de protocolo: el servidor acepta el cambio pedido en la cabecera Upgrade" },
    Codigo { numero: 102, razon: "Processing", descripcion: "Procesando: la petición se recibió pero aún no hay respuesta (WebDAV, obsoleto)" },
    Codigo { numero: 103, razon: "Early Hints", descripcion: "Indicios tempranos: permite precargar recursos mientras el servidor prepara la respuesta" },

    Codigo { numero: 200, razon: "OK", descripcion: "Correcto: la petición se completó con éxito" },
    Codigo { numero: 201, razon: "Created", descripcion: "Creado: la petición tuvo éxito y se creó un recurso nuevo" },
    Codigo { numero: 202, razon: "Accepted", descripcion: "Aceptado: la petición se recibió pero todavía no se procesó" },
    Codigo { numero: 203, razon: "Non-Authoritative Information", descripcion: "Información no autorizada: la respuesta viene de una copia modificada por un intermediario" },
    Codigo { numero: 204, razon: "No Content", descripcion: "Sin contenido: la petición tuvo éxito y no hay cuerpo que devolver" },
    Codigo { numero: 205, razon: "Reset Content", descripcion: "Restablecer contenido: el cliente debe limpiar el documento que envió la petición" },
    Codigo { numero: 206, razon: "Partial Content", descripcion: "Contenido parcial: se devuelve solo el rango pedido del recurso" },
    Codigo { numero: 207, razon: "Multi-Status", descripcion: "Multiestado: la respuesta contiene varios estados, uno por recurso (WebDAV)" },
    Codigo { numero: 208, razon: "Already Reported", descripcion: "Ya reportado: los miembros de la colección ya se enumeraron antes (WebDAV)" },
    Codigo { numero: 226, razon: "IM Used", descripcion: "IM usado: la respuesta es el resultado de aplicar manipulaciones de instancia" },

    Codigo { numero: 300, razon: "Multiple Choices", descripcion: "Múltiples opciones: la petición tiene más de una respuesta posible" },
    Codigo { numero: 301, razon: "Moved Permanently", descripcion: "Movido permanentemente: el recurso cambió de dirección de forma definitiva" },
    Codigo { numero: 302, razon: "Found", descripcion: "Encontrado: el recurso está temporalmente en otra dirección" },
    Codigo { numero: 303, razon: "See Other", descripcion: "Ver otro: el cliente debe pedir el recurso indicado con el método obtener" },
    Codigo { numero: 304, razon: "Not Modified", descripcion: "No modificado: la copia en cache del cliente sigue siendo válida" },
    Codigo { numero: 305, razon: "Use Proxy", descripcion: "Usar proxy: el recurso debe pedirse a través de un proxy (obsoleto)" },
    Codigo { numero: 307, razon: "Temporary Redirect", descripcion: "Redirección temporal: se repite la petición en otra dirección conservando el método" },
    Codigo { numero: 308, razon: "Permanent Redirect", descripcion: "Redirección permanente: el recurso cambió de dirección conservando el método" },

    Codigo { numero: 400, razon: "Bad Request", descripcion: "Petición incorrecta: el servidor no puede entender la petición por su sintaxis" },
    Codigo { numero: 401, razon: "Unauthorized", descripcion: "No autenticado: hace falta identificarse para obtener el recurso" },
    Codigo { numero: 402, razon: "Payment Required", descripcion: "Pago requerido: reservado para sistemas de pago digital" },
    Codigo { numero: 403, razon: "Forbidden", descripcion: "Prohibido: el cliente está identificado pero no tiene permiso sobre el recurso" },
    Codigo { numero: 404, razon: "Not Found", descripcion: "No encontrado: el servidor no encontró el recurso pedido" },
    Codigo { numero: 405, razon: "Method Not Allowed", descripcion: "Método no permitido: el recurso no acepta ese método HTTP" },
    Codigo { numero: 406, razon: "Not Acceptable", descripcion: "No aceptable: no hay contenido que cumpla los criterios de negociación del cliente" },
    Codigo { numero: 407, razon: "Proxy Authentication Required", descripcion: "Autenticación de proxy requerida: hace falta identificarse ante el proxy" },
    Codigo { numero: 408, razon: "Request Timeout", descripcion: "Tiempo de petición agotado: el servidor cerró una conexión inactiva" },
    Codigo { numero: 409, razon: "Conflict", descripcion: "Conflicto: la petición choca con el estado actual del recurso" },
    Codigo { numero: 410, razon: "Gone", descripcion: "Ya no disponible: el recurso se eliminó de forma permanente" },
    Codigo { numero: 411, razon: "Length Required", descripcion: "Longitud requerida: falta la cabecera Content-Length" },
    Codigo { numero: 412, razon: "Precondition Failed", descripcion: "Precondición fallida: no se cumplen las condiciones enviadas en las cabeceras" },
    Codigo { numero: 413, razon: "Content Too Large", descripcion: "Contenido demasiado grande: el cuerpo excede el límite del servidor" },
    Codigo { numero: 414, razon: "URI Too Long", descripcion: "URI demasiado larga: la dirección pedida supera lo que el servidor acepta" },
    Codigo { numero: 415, razon: "Unsupported Media Type", descripcion: "Tipo de medio no soportado: el formato del cuerpo no está soportado" },
    Codigo { numero: 416, razon: "Range Not Satisfiable", descripcion: "Rango no satisfacible: el rango pedido está fuera del tamaño del recurso" },
    Codigo { numero: 417, razon: "Expectation Failed", descripcion: "Expectativa fallida: no se cumple lo indicado en la cabecera Expect" },
    Codigo { numero: 418, razon: "I'm a teapot", descripcion: "Soy una tetera: el servidor se niega a preparar café" },
    Codigo { numero: 421, razon: "Misdirected Request", descripcion: "Petición mal dirigida: el servidor no puede responder por esa combinación de esquema y autoridad" },
    Codigo { numero: 422, razon: "Unprocessable Content", descripcion: "Contenido no procesable: la petición está bien formada pero es semánticamente inválida" },
    Codigo { numero: 423, razon: "Locked", descripcion: "Bloqueado: el recurso está bloqueado (WebDAV)" },
    Codigo { numero: 424, razon: "Failed Dependency", descripcion: "Dependencia fallida: la petición falló porque otra de la que depende falló (WebDAV)" },
    Codigo { numero: 425, razon: "Too Early", descripcion: "Demasiado pronto: el servidor evita procesar una petición que podría repetirse" },
    Codigo { numero: 426, razon: "Upgrade Required", descripcion: "Actualización requerida: hace falta cambiar de protocolo" },
    Codigo { numero: 428, razon: "Precondition Required", descripcion: "Precondición requerida: la petición debe ser condicional" },
    Codigo { numero: 429, razon: "Too Many Requests", descripcion: "Demasiadas peticiones: se superó el límite de peticiones permitido" },
    Codigo { numero: 431, razon: "Request Header Fields Too Large", descripcion: "Cabeceras demasiado grandes: las cabeceras superan el límite del servidor" },
    Codigo { numero: 451, razon: "Unavailable For Legal Reasons", descripcion: "No disponible por razones legales: el recurso fue censurado" },

    Codigo { numero: 500, razon: "Internal Server Error", descripcion: "Error interno del servidor: el servidor encontró una situación que no sabe manejar" },
    Codigo { numero: 501, razon: "Not Implemented", descripcion: "No implementado: el servidor no soporta el método pedido" },
    Codigo { numero: 502, razon: "Bad Gateway", descripcion: "Puerta de enlace incorrecta: el servidor recibió una respuesta inválida de otro servidor" },
    Codigo { numero: 503, razon: "Service Unavailable", descripcion: "Servicio no disponible: el servidor no está listo para atender la petición" },
    Codigo { numero: 504, razon: "Gateway Timeout", descripcion: "Tiempo de puerta de enlace agotado: otro servidor no respondió a tiempo" },
    Codigo { numero: 505, razon: "HTTP Version Not Supported", descripcion: "Versión HTTP no soportada: el servidor no soporta esa versión del protocolo" },
    Codigo { numero: 506, razon: "Variant Also Negotiates", descripcion: "La variante también negocia: hay un error de configuración en la negociación de contenido" },
    Codigo { numero: 507, razon: "Insufficient Storage", descripcion: "Almacenamiento insuficiente: no hay espacio para completar la operación (WebDAV)" },
    Codigo { numero: 508, razon: "Loop Detected", descripcion: "Bucle detectado: se encontró un ciclo infinito al procesar la petición (WebDAV)" },
    Codigo { numero: 510, razon: "Not Extended", descripcion: "No extendido: la petición necesita más extensiones para completarse" },
    Codigo { numero: 511, razon: "Network Authentication Required", descripcion: "Autenticación de red requerida: hace falta identificarse para acceder a la red" },
];

/// Constantes en español expuestas como `HttpCodigos.NOMBRE`.
const CONSTANTES: &[(&str, i64)] = &[
    ("CONTINUAR", 100),
    ("CAMBIANDO_PROTOCOLO", 101),
    ("INDICIOS_TEMPRANOS", 103),
    ("OK", 200),
    ("CREADO", 201),
    ("ACEPTADO", 202),
    ("SIN_CONTENIDO", 204),
    ("CONTENIDO_PARCIAL", 206),
    ("MULTIPLES_OPCIONES", 300),
    ("MOVIDO_PERMANENTEMENTE", 301),
    ("ENCONTRADO", 302),
    ("VER_OTRO", 303),
    ("NO_MODIFICADO", 304),
    ("REDIRECCION_TEMPORAL", 307),
    ("REDIRECCION_PERMANENTE", 308),
    ("PETICION_INCORRECTA", 400),
    ("NO_AUTENTICADO", 401),
    ("PROHIBIDO", 403),
    ("NO_ENCONTRADO", 404),
    ("METODO_NO_PERMITIDO", 405),
    ("NO_ACEPTABLE", 406),
    ("TIEMPO_AGOTADO", 408),
    ("CONFLICTO", 409),
    ("YA_NO_DISPONIBLE", 410),
    ("CONTENIDO_DEMASIADO_GRANDE", 413),
    ("TIPO_NO_SOPORTADO", 415),
    ("CONTENIDO_NO_PROCESABLE", 422),
    ("DEMASIADAS_PETICIONES", 429),
    ("ERROR_INTERNO", 500),
    ("NO_IMPLEMENTADO", 501),
    ("PUERTA_INCORRECTA", 502),
    ("SERVICIO_NO_DISPONIBLE", 503),
    ("TIEMPO_PUERTA_AGOTADO", 504),
];

fn buscar(numero: i64) -> Option<&'static Codigo> {
    CODIGOS.iter().find(|codigo| codigo.numero == numero)
}

/// Frase de razón que viaja en la línea de estado de la respuesta.
pub(crate) fn razon(numero: i64) -> &'static str {
    buscar(numero).map(|codigo| codigo.razon).unwrap_or("Unknown")
}

/// Descripción en español, o un texto genérico por categoría si el código no
/// está en la tabla pero es un estado HTTP válido.
pub(crate) fn descripcion(numero: i64) -> String {
    if let Some(codigo) = buscar(numero) {
        return codigo.descripcion.to_string();
    }
    match categoria_de(numero) {
        Some(categoria) => format!("código {numero} sin descripción conocida ({categoria})"),
        None => format!("código {numero} fuera del rango de estados HTTP (100-599)"),
    }
}

fn categoria_de(numero: i64) -> Option<&'static str> {
    match numero {
        100..=199 => Some("informativo"),
        200..=299 => Some("exitoso"),
        300..=399 => Some("redirección"),
        400..=499 => Some("error del cliente"),
        500..=599 => Some("error del servidor"),
        _ => None,
    }
}

/// Registra `HttpCodigos` y sus constantes.
pub fn registrar(registro: &mut RegistroNativos) {
    registro.registrar_modulo(TIPO);

    for (nombre, numero) in CONSTANTES {
        registro.registrar_constante(&format!("{TIPO}.{nombre}"), Valor::Entero(*numero));
    }

    consulta(registro, "descripcion", |numero| {
        Valor::texto(descripcion(numero))
    });
    consulta(registro, "razon", |numero| Valor::texto(razon(numero)));
    consulta(registro, "categoria", |numero| {
        Valor::texto(categoria_de(numero).unwrap_or("desconocida"))
    });
    consulta(registro, "existe", |numero| {
        Valor::Log(buscar(numero).is_some())
    });
    consulta(registro, "es_informativo", |numero| {
        Valor::Log((100..=199).contains(&numero))
    });
    consulta(registro, "es_exitoso", |numero| {
        Valor::Log((200..=299).contains(&numero))
    });
    consulta(registro, "es_redireccion", |numero| {
        Valor::Log((300..=399).contains(&numero))
    });
    consulta(registro, "es_error_cliente", |numero| {
        Valor::Log((400..=499).contains(&numero))
    });
    consulta(registro, "es_error_servidor", |numero| {
        Valor::Log((500..=599).contains(&numero))
    });
    consulta(registro, "es_error", |numero| {
        Valor::Log((400..=599).contains(&numero))
    });

    registro.registrar_funcion(
        &format!("{TIPO}.todos"),
        Box::new(move |argumentos| {
            const F: &str = "HttpCodigos.todos";
            exigir_aridad(F, argumentos, 0)?;
            Ok(Valor::lista(
                CODIGOS
                    .iter()
                    .map(|codigo| {
                        let mut mapa = indexmap::IndexMap::new();
                        mapa.insert("codigo".to_string(), Valor::Entero(codigo.numero));
                        mapa.insert("razon".to_string(), Valor::texto(codigo.razon));
                        mapa.insert(
                            "descripcion".to_string(),
                            Valor::texto(codigo.descripcion),
                        );
                        Valor::jsn(mapa)
                    })
                    .collect(),
            ))
        }),
    );
}

/// Registra una función que recibe un código de estado y devuelve un valor.
fn consulta<F>(registro: &mut RegistroNativos, nombre: &'static str, calcular: F)
where
    F: Fn(i64) -> Valor + 'static,
{
    let funcion = format!("{TIPO}.{nombre}");
    registro.registrar_funcion(
        &funcion.clone(),
        Box::new(move |argumentos| {
            exigir_aridad(&funcion, argumentos, 1)?;
            let numero = arg_entero(&funcion, argumentos, 0)?;
            if !(0..=1000).contains(&numero) {
                return Err(error(
                    "E0406",
                    format!("'{funcion}' recibió un código fuera de rango: {numero}"),
                ));
            }
            Ok(calcular(numero))
        }),
    );
}
