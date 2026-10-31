//! Transporte JSON-RPC y ciclo de vida del servidor LSP.

use lsp_server::{Connection, ErrorCode, Message, Notification, ProtocolError, Request, Response};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, InitializeParams, InitializeResult, PositionEncodingKind,
    PublishDiagnosticsParams, SaveOptions, ServerCapabilities, ServerInfo,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    TextDocumentSyncSaveOptions,
};

use crate::analisis::analizar_documento;
use crate::documentos::{AlmacenDocumentos, DocumentoAbierto};
use crate::servicios::diagnosticos::convertir_diagnosticos;

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

    let mut documentos = AlmacenDocumentos::default();
    for mensaje in &conexion.receiver {
        match mensaje {
            Message::Request(solicitud) => {
                if conexion.handle_shutdown(&solicitud)? {
                    return Ok(());
                }
                responder_metodo_desconocido(&conexion, solicitud);
            }
            Message::Notification(notificacion) => {
                procesar_notificacion(&conexion, &mut documentos, notificacion);
            }
            Message::Response(_) => {}
        }
    }

    Ok(())
}

fn capacidades() -> ServerCapabilities {
    ServerCapabilities {
        position_encoding: Some(PositionEncodingKind::UTF16),
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::FULL),
                save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                    include_text: Some(false),
                })),
                ..TextDocumentSyncOptions::default()
            },
        )),
        ..ServerCapabilities::default()
    }
}

fn procesar_notificacion(
    conexion: &Connection,
    documentos: &mut AlmacenDocumentos,
    notificacion: Notification,
) {
    let resultado = match notificacion.method.as_str() {
        "textDocument/didOpen" => serde_json::from_value::<DidOpenTextDocumentParams>(
            notificacion.params,
        )
        .map(|parametros| {
            let documento = parametros.text_document;
            let uri = documento.uri.to_string();
            documentos.abrir(DocumentoAbierto::nuevo(
                uri.clone(),
                documento.version,
                documento.text,
            ));
            if let Some(documento) = documentos.obtener(&uri) {
                publicar_diagnosticos(conexion, documento);
            }
        }),
        "textDocument/didChange" => serde_json::from_value::<DidChangeTextDocumentParams>(
            notificacion.params,
        )
        .map(|parametros| {
            let uri = parametros.text_document.uri.to_string();
            let version = parametros.text_document.version;
            match parametros.content_changes.as_slice() {
                [cambio] if cambio.range.is_none() => {
                    if documentos.cambiar_completo(&uri, version, cambio.text.clone())
                        && let Some(documento) = documentos.obtener(&uri)
                    {
                        publicar_diagnosticos(conexion, documento);
                    }
                }
                _ => eprintln!("Cambio LSP ignorado: se esperaba contenido completo"),
            }
        }),
        "textDocument/didSave" => serde_json::from_value::<DidSaveTextDocumentParams>(
            notificacion.params,
        )
        .map(|parametros| {
            let uri = parametros.text_document.uri.to_string();
            if documentos.guardar(&uri, parametros.text)
                && let Some(documento) = documentos.obtener(&uri)
            {
                publicar_diagnosticos(conexion, documento);
            }
        }),
        "textDocument/didClose" => serde_json::from_value::<DidCloseTextDocumentParams>(
            notificacion.params,
        )
        .map(|parametros| {
            let uri = parametros.text_document.uri.to_string();
            if documentos.cerrar(&uri).is_some() {
                limpiar_diagnosticos(conexion, &uri);
            }
        }),
        _ => return,
    };

    if let Err(error) = resultado {
        eprintln!("Notificación LSP inválida: {error}");
    }
}

fn publicar_diagnosticos(conexion: &Connection, documento: &DocumentoAbierto) {
    let Ok(uri) = documento.uri.parse() else {
        eprintln!("URI de documento inválida: {}", documento.uri);
        return;
    };
    let errores = analizar_documento(&documento.uri, &documento.contenido);
    let diagnostics = convertir_diagnosticos(&uri, &documento.contenido, errores);
    let parametros = PublishDiagnosticsParams {
        uri,
        diagnostics,
        version: Some(documento.version),
    };
    if conexion
        .sender
        .send(Notification::new("textDocument/publishDiagnostics".to_string(), parametros).into())
        .is_err()
    {
        eprintln!("No se pudieron publicar diagnósticos: canal desconectado");
    }
}

fn limpiar_diagnosticos(conexion: &Connection, uri: &str) {
    let Ok(uri) = uri.parse() else {
        eprintln!("URI de documento inválida: {uri}");
        return;
    };
    let parametros = PublishDiagnosticsParams {
        uri,
        diagnostics: Vec::new(),
        version: None,
    };
    if conexion
        .sender
        .send(Notification::new("textDocument/publishDiagnostics".to_string(), parametros).into())
        .is_err()
    {
        eprintln!("No se pudieron limpiar diagnósticos: canal desconectado");
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
