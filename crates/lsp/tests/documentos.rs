//! Pruebas de almacenamiento y sincronización de documentos.

use std::{thread, time::Duration};

use lsp::documentos::{AlmacenDocumentos, DocumentoAbierto};
use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use serde_json::{Value, json};

const URI: &str = "file:///prueba.qz";

fn documento(version: i32, contenido: &str) -> DocumentoAbierto {
    DocumentoAbierto::nuevo(URI, version, contenido)
}

#[test]
fn abrir_deberia_conservar_contenido_y_version() {
    let mut almacen = AlmacenDocumentos::default();
    almacen.abrir(documento(7, "entero valor = 1"));
    let abierto = almacen.obtener(URI).expect("el documento debe existir");
    assert_eq!(
        (abierto.version, abierto.contenido.as_str()),
        (7, "entero valor = 1")
    );
}

#[test]
fn cambiar_deberia_conservar_texto_sin_guardar() {
    let mut almacen = AlmacenDocumentos::default();
    almacen.abrir(documento(1, "texto saludo = \"hola\""));
    almacen.cambiar_completo(URI, 2, "texto saludo = \"quetzal\"");
    let abierto = almacen.obtener(URI).expect("el documento debe existir");
    assert_eq!(abierto.contenido, "texto saludo = \"quetzal\"");
    assert!(abierto.modificado);
}

#[test]
fn cambiar_deberia_ignorar_version_anterior() {
    let mut almacen = AlmacenDocumentos::default();
    almacen.abrir(documento(3, "nuevo"));
    assert!(!almacen.cambiar_completo(URI, 2, "viejo"));
}

#[test]
fn guardar_deberia_limpiar_marca_de_modificacion() {
    let mut almacen = AlmacenDocumentos::default();
    almacen.abrir(documento(1, "inicial"));
    almacen.cambiar_completo(URI, 2, "editado");
    almacen.guardar(URI, None);
    assert!(!almacen.obtener(URI).expect("debe existir").modificado);
}

#[test]
fn cerrar_deberia_eliminar_documento() {
    let mut almacen = AlmacenDocumentos::default();
    almacen.abrir(documento(1, "contenido"));
    almacen.cerrar(URI);
    assert!(almacen.obtener(URI).is_none());
}

#[test]
fn almacen_deberia_separar_varios_documentos() {
    let mut almacen = AlmacenDocumentos::default();
    almacen.abrir(documento(1, "uno"));
    almacen.abrir(DocumentoAbierto::nuevo("file:///otro.qz", 4, "dos"));
    assert_eq!(almacen.cantidad(), 2);
}

#[test]
fn cambios_rapidos_deberian_conservar_la_ultima_version() {
    let mut almacen = AlmacenDocumentos::default();
    almacen.abrir(documento(0, ""));
    for version in 1..=100 {
        almacen.cambiar_completo(URI, version, format!("versión {version}"));
    }
    let abierto = almacen.obtener(URI).expect("debe existir");
    assert_eq!(
        (abierto.version, abierto.contenido.as_str()),
        (100, "versión 100")
    );
}

fn recibir_respuesta(conexion: &Connection) -> Response {
    match conexion
        .receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("debe recibir respuesta")
    {
        Message::Response(respuesta) => respuesta,
        otro => panic!("se esperaba respuesta, se recibió: {otro:?}"),
    }
}

fn inicializar() -> (
    Connection,
    thread::JoinHandle<Result<(), lsp::servidor::ErrorServidor>>,
    Value,
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
    let resultado = recibir_respuesta(&cliente)
        .response_result
        .expect("initialize debe responder");
    cliente
        .sender
        .send(Notification::new("initialized".to_string(), json!({})).into())
        .expect("debe enviar initialized");
    (cliente, hilo, resultado)
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

#[test]
fn initialize_deberia_anunciar_sincronizacion_completa() {
    let (_cliente, _hilo, resultado) = inicializar();
    assert_eq!(
        resultado["capabilities"]["textDocumentSync"],
        json!({"openClose": true, "change": 1, "save": {"includeText": false}})
    );
}

#[test]
fn ciclo_de_notificaciones_deberia_mantener_servidor_operativo() {
    let (cliente, hilo, _) = inicializar();
    for notificacion in [
        Notification::new(
            "textDocument/didOpen".to_string(),
            json!({"textDocument":{"uri":URI,"languageId":"quetzal","version":1,"text":"uno"}}),
        ),
        Notification::new(
            "textDocument/didChange".to_string(),
            json!({"textDocument":{"uri":URI,"version":2},"contentChanges":[{"text":"dos"}]}),
        ),
        Notification::new(
            "textDocument/didSave".to_string(),
            json!({"textDocument":{"uri":URI}}),
        ),
        Notification::new(
            "textDocument/didClose".to_string(),
            json!({"textDocument":{"uri":URI}}),
        ),
    ] {
        cliente
            .sender
            .send(notificacion.into())
            .expect("debe enviar notificación");
    }
    terminar(cliente, hilo);
}
