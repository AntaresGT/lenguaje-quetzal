//! Pruebas de hover mediante solicitudes LSP reales.

use std::{thread, time::Duration};

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::{Hover, HoverContents};
use serde_json::{Value, json};

const URI: &str = "file:///hover.qz";

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
    let inicializacion = recibir_respuesta(&cliente)
        .response_result
        .expect("initialize debe responder");
    cliente
        .sender
        .send(Notification::new("initialized".to_string(), json!({})).into())
        .expect("debe enviar initialized");
    (cliente, hilo, inicializacion)
}

fn abrir(cliente: &Connection, texto: &str) {
    cliente
        .sender
        .send(
            Notification::new(
                "textDocument/didOpen".to_string(),
                json!({"textDocument":{"uri":URI,"languageId":"quetzal","version":1,"text":texto}}),
            )
            .into(),
        )
        .expect("debe abrir documento");
}

fn hover(cliente: &Connection, linea: u32, caracter: u32) -> Option<Hover> {
    cliente
        .sender
        .send(
            Request::new(
                RequestId::from(2),
                "textDocument/hover".to_string(),
                json!({"textDocument":{"uri":URI},"position":{"line":linea,"character":caracter}}),
            )
            .into(),
        )
        .expect("debe solicitar hover");
    let resultado = recibir_respuesta(cliente)
        .response_result
        .expect("hover debe responder");
    serde_json::from_value(resultado).expect("la respuesta debe ser hover o null")
}

fn recibir_respuesta(conexion: &Connection) -> Response {
    loop {
        match conexion
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

fn markdown(hover: &Hover) -> &str {
    match &hover.contents {
        HoverContents::Markup(contenido) => &contenido.value,
        otro => panic!("se esperaba Markdown: {otro:?}"),
    }
}

#[test]
fn initialize_deberia_anunciar_hover() {
    let (cliente, hilo, inicializacion) = inicializar();
    assert_eq!(inicializacion["capabilities"]["hoverProvider"], true);
    terminar(cliente, hilo);
}

#[test]
fn hover_deberia_mostrar_tipo_y_mutabilidad_de_una_referencia() {
    let (cliente, hilo, _) = inicializar();
    abrir(&cliente, "entero var saldo = 10\nsaldo = 20");
    let resultado = hover(&cliente, 1, 2).expect("debe encontrar saldo");
    assert_eq!(markdown(&resultado), "```quetzal\nentero var saldo\n```");
    assert_eq!(
        resultado.range.expect("debe incluir rango").start.character,
        0
    );
    terminar(cliente, hilo);
}

#[test]
fn hover_deberia_mostrar_la_firma_de_una_funcion() {
    let (cliente, hilo, _) = inicializar();
    abrir(
        &cliente,
        "entero sumar(entero a, entero b) { retornar a + b }\nsumar(1, 2)",
    );
    let resultado = hover(&cliente, 1, 2).expect("debe encontrar sumar");
    assert_eq!(
        markdown(&resultado),
        "```quetzal\nentero sumar(entero a, entero b)\n```"
    );
    terminar(cliente, hilo);
}

#[test]
fn hover_deberia_funcion_sobre_la_declaracion() {
    let (cliente, hilo, _) = inicializar();
    abrir(&cliente, "texto saludo = \"hola\"");
    assert!(hover(&cliente, 0, 8).is_some());
    terminar(cliente, hilo);
}

#[test]
fn hover_deberia_devolver_null_fuera_de_un_simbolo() {
    let (cliente, hilo, _) = inicializar();
    abrir(&cliente, "entero saldo = 10");
    assert!(hover(&cliente, 0, 0).is_none());
    terminar(cliente, hilo);
}
