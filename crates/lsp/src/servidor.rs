//! Transporte JSON-RPC y ciclo de vida del servidor LSP.

use lsp_server::{Connection, ErrorCode, Message, Notification, ProtocolError, Request, Response};
use lsp_types::{
    CompletionOptions, CompletionParams, CompletionResponse, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    GotoDefinitionParams, GotoDefinitionResponse, HoverParams, HoverProviderCapability,
    InitializeParams, InitializeResult, OneOf, PositionEncodingKind, PublishDiagnosticsParams,
    SaveOptions, ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, TextDocumentSyncSaveOptions,
};

use crate::analisis::{analizar_documento, analizar_documento_para_herramientas};
use crate::documentos::{AlmacenDocumentos, ConvertidorPosiciones, DocumentoAbierto};
use crate::servicios::autocompletado::completar;
use crate::servicios::definicion::ubicacion_definicion;
use crate::servicios::diagnosticos::convertir_diagnosticos;
use crate::servicios::hover::informacion;

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
                procesar_solicitud(&conexion, &documentos, solicitud);
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
        completion_provider: Some(CompletionOptions::default()),
        definition_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
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

fn procesar_solicitud(conexion: &Connection, documentos: &AlmacenDocumentos, solicitud: Request) {
    let Request { id, method, params } = solicitud;
    let resultado = match method.as_str() {
        "textDocument/completion" => serde_json::from_value::<CompletionParams>(params)
            .map_err(|error| (ErrorCode::InvalidParams, error.to_string()))
            .and_then(|parametros| {
                let uri = parametros.text_document_position.text_document.uri;
                let elementos = documentos
                    .obtener(&uri.to_string())
                    .map(|documento| {
                        let convertidor = ConvertidorPosiciones::nuevo(&documento.contenido);
                        let offset = convertidor
                            .posicion_a_offset(parametros.text_document_position.position)
                            .unwrap_or(documento.contenido.len());
                        let analisis = analizar_documento_para_herramientas(
                            &documento.uri,
                            &documento.contenido,
                        );
                        completar(&analisis, &documento.contenido, offset)
                    })
                    .unwrap_or_default();
                serde_json::to_value(CompletionResponse::Array(elementos))
                    .map_err(|error| (ErrorCode::InternalError, error.to_string()))
            }),
        "textDocument/hover" => serde_json::from_value::<HoverParams>(params)
            .map_err(|error| (ErrorCode::InvalidParams, error.to_string()))
            .and_then(|parametros| {
                let posicion = parametros.text_document_position_params;
                let resultado = documentos
                    .obtener(&posicion.text_document.uri.to_string())
                    .and_then(|documento| {
                        let offset = ConvertidorPosiciones::nuevo(&documento.contenido)
                            .posicion_a_offset(posicion.position)?;
                        let analisis = analizar_documento_para_herramientas(
                            &documento.uri,
                            &documento.contenido,
                        );
                        informacion(&analisis, &documento.contenido, offset)
                    });
                serde_json::to_value(resultado)
                    .map_err(|error| (ErrorCode::InternalError, error.to_string()))
            }),
        "textDocument/definition" => serde_json::from_value::<GotoDefinitionParams>(params)
            .map_err(|error| (ErrorCode::InvalidParams, error.to_string()))
            .and_then(|parametros| {
                let posicion = parametros.text_document_position_params;
                let resultado = documentos
                    .obtener(&posicion.text_document.uri.to_string())
                    .and_then(|documento| {
                        let offset = ConvertidorPosiciones::nuevo(&documento.contenido)
                            .posicion_a_offset(posicion.position)?;
                        let analisis = analizar_documento_para_herramientas(
                            &documento.uri,
                            &documento.contenido,
                        );
                        ubicacion_definicion(
                            &posicion.text_document.uri,
                            &analisis,
                            &documento.contenido,
                            offset,
                        )
                    })
                    .map(GotoDefinitionResponse::Scalar);
                serde_json::to_value(resultado)
                    .map_err(|error| (ErrorCode::InternalError, error.to_string()))
            }),
        _ => Err((
            ErrorCode::MethodNotFound,
            format!("Método no implementado: {method}"),
        )),
    };
    let respuesta = match resultado {
        Ok(valor) => Response::new_ok(id, valor),
        Err((codigo, mensaje)) => Response::new_err(id, codigo as i32, mensaje),
    };
    if conexion.sender.send(respuesta.into()).is_err() {
        eprintln!("No se pudo enviar la respuesta LSP: canal desconectado");
    }
}
