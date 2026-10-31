//! Pruebas de autocompletado mediante solicitudes LSP reales.

use std::{thread, time::Duration};

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::CompletionResponse;
use serde_json::{Value, json};

const URI: &str = "file:///autocompletado.qz";

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

fn completar(cliente: &Connection, linea: u32, caracter: u32) -> Vec<Value> {
    cliente
        .sender
        .send(
            Request::new(
                RequestId::from(2),
                "textDocument/completion".to_string(),
                json!({"textDocument":{"uri":URI},"position":{"line":linea,"character":caracter}}),
            )
            .into(),
        )
        .expect("debe solicitar autocompletado");
    let resultado = recibir_respuesta(cliente)
        .response_result
        .expect("completion debe responder");
    match serde_json::from_value(resultado).expect("la respuesta debe ser completion") {
        CompletionResponse::Array(elementos) => elementos
            .into_iter()
            .map(|elemento| serde_json::to_value(elemento).expect("debe serializar el elemento"))
            .collect(),
        CompletionResponse::List(_) => panic!("se esperaba un arreglo de completion"),
    }
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

#[test]
fn initialize_deberia_anunciar_autocompletado() {
    let (cliente, hilo, inicializacion) = inicializar();
    assert!(inicializacion["capabilities"]["completionProvider"].is_object());
    terminar(cliente, hilo);
}

#[test]
fn completion_deberia_ofrecer_simbolos_con_tipo() {
    let (cliente, hilo, _) = inicializar();
    abrir(&cliente, "entero total = 1\ntot");
    let elementos = completar(&cliente, 1, 3);
    let total = elementos
        .iter()
        .find(|elemento| elemento["label"] == "total")
        .expect("debe incluir total");
    assert_eq!(total["detail"], "entero total");
    terminar(cliente, hilo);
}

#[test]
fn completion_deberia_ofrecer_palabras_clave() {
    let (cliente, hilo, _) = inicializar();
    abrir(&cliente, "ret");
    let elementos = completar(&cliente, 0, 3);
    assert!(
        elementos
            .iter()
            .any(|elemento| elemento["label"] == "retornar")
    );
    terminar(cliente, hilo);
}

#[test]
fn completion_deberia_respetar_prefijos_unicode() {
    let (cliente, hilo, _) = inicializar();
    abrir(&cliente, "entero año = 1\nañ");
    let elementos = completar(&cliente, 1, 2);
    assert!(elementos.iter().any(|elemento| elemento["label"] == "año"));
    terminar(cliente, hilo);
}

#[test]
fn completion_deberia_aceptar_posiciones_al_final_de_linea() {
    let (cliente, hilo, _) = inicializar();
    abrir(&cliente, "entero total = 1\ntotal");
    let elementos = completar(&cliente, 1, 5);
    assert!(
        elementos
            .iter()
            .any(|elemento| elemento["label"] == "total")
    );
    terminar(cliente, hilo);
}
