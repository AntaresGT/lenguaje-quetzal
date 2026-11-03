//! Pruebas de publicación de diagnósticos reales del compilador.

use std::{thread, time::Duration};

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::{NumberOrString, PublishDiagnosticsParams};
use serde_json::json;

const URI: &str = "file:///diagnostico.qz";

fn inicializar() -> (
    Connection,
    thread::JoinHandle<Result<(), lsp::servidor::ErrorServidor>>,
) {
    let (servidor, cliente) = Connection::memory();
    let hilo = thread::spawn(move || lsp::servidor::ejecutar(servidor));
    cliente
        .sender
        .send(
            Request::new(
                RequestId::from(1),
                "initialize".to_string(),
                json!({"processId": null, "rootUri": null, "capabilities": {}}),
            )
            .into(),
        )
        .expect("debe enviar initialize");
    match cliente
        .receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("debe responder initialize")
    {
        Message::Response(respuesta) => assert!(respuesta.response_result.is_ok()),
        otro => panic!("se esperaba respuesta: {otro:?}"),
    }
    cliente
        .sender
        .send(Notification::new("initialized".to_string(), json!({})).into())
        .expect("debe enviar initialized");
    (cliente, hilo)
}

fn abrir(cliente: &Connection, version: i32, texto: &str) {
    cliente
        .sender
        .send(
            Notification::new(
                "textDocument/didOpen".to_string(),
                json!({"textDocument":{"uri":URI,"languageId":"quetzal","version":version,"text":texto}}),
            )
            .into(),
        )
        .expect("debe abrir documento");
}

fn cambiar(cliente: &Connection, version: i32, texto: &str) {
    cliente
        .sender
        .send(
            Notification::new(
                "textDocument/didChange".to_string(),
                json!({"textDocument":{"uri":URI,"version":version},"contentChanges":[{"text":texto}]}),
            )
            .into(),
        )
        .expect("debe cambiar documento");
}

fn recibir_publicacion(cliente: &Connection) -> PublishDiagnosticsParams {
    match cliente
        .receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("debe publicar diagnósticos")
    {
        Message::Notification(notificacion)
            if notificacion.method == "textDocument/publishDiagnostics" =>
        {
            serde_json::from_value(notificacion.params).expect("los parámetros deben ser válidos")
        }
        _ => panic!("se esperaba publishDiagnostics"),
    }
}

fn recibir_respuesta(cliente: &Connection) -> Response {
    loop {
        match cliente
            .receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("debe recibir respuesta")
        {
            Message::Response(respuesta) => return respuesta,
            Message::Notification(_) => {}
            otro => panic!("se esperaba respuesta: {otro:?}"),
        }
    }
}

fn terminar(
    cliente: Connection,
    hilo: thread::JoinHandle<Result<(), lsp::servidor::ErrorServidor>>,
) {
    cliente
        .sender
        .send(Request::new(RequestId::from(90), "shutdown".to_string(), ()).into())
        .expect("debe enviar shutdown");
    assert!(recibir_respuesta(&cliente).response_result.is_ok());
    cliente
        .sender
        .send(Notification::new("exit".to_string(), ()).into())
        .expect("debe enviar exit");
    hilo.join()
        .expect("el servidor no debe entrar en pánico")
        .expect("el servidor debe terminar sin errores");
}

fn codigo(publicacion: &PublishDiagnosticsParams) -> &str {
    match &publicacion.diagnostics[0].code {
        Some(NumberOrString::String(codigo)) => codigo,
        otro => panic!("código inesperado: {otro:?}"),
    }
}

#[test]
fn apertura_deberia_publicar_error_lexico_real() {
    let (cliente, hilo) = inicializar();
    abrir(&cliente, 1, "@");
    let publicacion = recibir_publicacion(&cliente);
    assert_eq!(codigo(&publicacion), "E0001");
    assert_eq!(
        publicacion.diagnostics[0].source.as_deref(),
        Some("quetzal")
    );
    terminar(cliente, hilo);
}

#[test]
fn apertura_deberia_publicar_error_sintactico_real() {
    let (cliente, hilo) = inicializar();
    abrir(&cliente, 1, "entero valor =");
    assert_eq!(codigo(&recibir_publicacion(&cliente)), "E0102");
    terminar(cliente, hilo);
}

#[test]
fn apertura_deberia_publicar_error_semantico_real() {
    let (cliente, hilo) = inicializar();
    abrir(&cliente, 1, "entero valor = \"texto\"");
    assert_eq!(codigo(&recibir_publicacion(&cliente)), "E0204");
    terminar(cliente, hilo);
}

#[test]
fn correccion_deberia_eliminar_diagnosticos_obsoletos() {
    let (cliente, hilo) = inicializar();
    abrir(&cliente, 1, "entero valor = \"texto\"");
    recibir_publicacion(&cliente);
    cambiar(&cliente, 2, "entero valor = 1");
    let publicacion = recibir_publicacion(&cliente);
    assert!(publicacion.diagnostics.is_empty());
    assert_eq!(publicacion.version, Some(2));
    terminar(cliente, hilo);
}

#[test]
fn cerrar_deberia_limpiar_diagnosticos() {
    let (cliente, hilo) = inicializar();
    abrir(&cliente, 1, "entero valor = \"texto\"");
    recibir_publicacion(&cliente);
    cliente
        .sender
        .send(
            Notification::new(
                "textDocument/didClose".to_string(),
                json!({"textDocument":{"uri":URI}}),
            )
            .into(),
        )
        .expect("debe cerrar documento");
    assert!(recibir_publicacion(&cliente).diagnostics.is_empty());
    terminar(cliente, hilo);
}

#[test]
fn rango_deberia_usar_unidades_utf16_tras_enie() {
    let (cliente, hilo) = inicializar();
    abrir(&cliente, 1, "entero ñ = \"mal\"");
    let publicacion = recibir_publicacion(&cliente);
    let rango = publicacion.diagnostics[0].range;
    assert_eq!((rango.start.line, rango.start.character), (0, 11));
    assert_eq!((rango.end.line, rango.end.character), (0, 16));
    terminar(cliente, hilo);
}

#[test]
fn cambio_de_version_vieja_no_deberia_publicarse() {
    let (cliente, hilo) = inicializar();
    abrir(&cliente, 1, "entero valor = 1");
    recibir_publicacion(&cliente);
    cambiar(&cliente, 2, "entero valor = \"malo\"");
    recibir_publicacion(&cliente);
    cambiar(&cliente, 2, "entero valor = 1");
    cliente
        .sender
        .send(Request::new(RequestId::from(90), "shutdown".to_string(), ()).into())
        .expect("debe enviar shutdown");
    let mensaje = cliente
        .receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("debe responder shutdown sin diagnóstico intermedio");
    assert!(matches!(mensaje, Message::Response(_)));
    cliente
        .sender
        .send(Notification::new("exit".to_string(), ()).into())
        .expect("debe enviar exit");
    hilo.join()
        .expect("el servidor no debe entrar en pánico")
        .expect("el servidor debe terminar sin errores");
}
