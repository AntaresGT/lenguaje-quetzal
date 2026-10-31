//! Transporte JSON-RPC y ciclo de vida del servidor LSP.

use lsp_server::{Connection, ErrorCode, Message, ProtocolError, Request, Response};
use lsp_types::{
    InitializeParams, InitializeResult, PositionEncodingKind, ServerCapabilities, ServerInfo,
};

/// Error de transporte o inicialización del servidor.
#[derive(Debug, thiserror::Error)]
pub enum ErrorServidor {
    #[error("error de E/S: {0}")]
    Io(#[from] std::io::Error),
    #[error("error de protocolo: {0}")]
    Protocolo(#[from] ProtocolError),
    #[error("error de serialización: {0}")]
    Serializacion(#[from] serde_json::Error),
}

/// Ejecuta el servidor usando exclusivamente la entrada y salida estándar.
pub fn ejecutar_stdio() -> Result<(), ErrorServidor> {
    let (conexion, hilos) = Connection::stdio();
    ejecutar(conexion)?;
    hilos.join()?;
    Ok(())
}

/// Ejecuta el ciclo de vida LSP sobre una conexión ya creada.
pub fn ejecutar(conexion: Connection) -> Result<(), ErrorServidor> {
    let (id, parametros) = conexion.initialize_start()?;
    let _parametros: InitializeParams = serde_json::from_value(parametros)?;
    let resultado = InitializeResult {
        capabilities: capacidades(),
        server_info: Some(ServerInfo {
            name: "quetzal-lsp".to_string(),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
        }),
    };
    conexion.initialize_finish(id, serde_json::to_value(resultado)?)?;

    for mensaje in &conexion.receiver {
        match mensaje {
            Message::Request(solicitud) => {
                if conexion.handle_shutdown(&solicitud)? {
                    return Ok(());
                }
                responder_metodo_desconocido(&conexion, solicitud);
            }
            Message::Notification(_) | Message::Response(_) => {}
        }
    }

    Ok(())
}

fn capacidades() -> ServerCapabilities {
    ServerCapabilities {
        position_encoding: Some(PositionEncodingKind::UTF16),
        ..ServerCapabilities::default()
    }
}

fn responder_metodo_desconocido(conexion: &Connection, solicitud: Request) {
    let respuesta = Response::new_err(
        solicitud.id,
        ErrorCode::MethodNotFound as i32,
        format!("Método no implementado: {}", solicitud.method),
    );
    if conexion.sender.send(respuesta.into()).is_err() {
        eprintln!("No se pudo enviar la respuesta LSP: canal desconectado");
    }
}
