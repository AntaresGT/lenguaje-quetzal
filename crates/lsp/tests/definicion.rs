//! Pruebas de navegación a definiciones mediante LSP.

use std::{thread, time::Duration};

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::{GotoDefinitionResponse, Location};
use serde_json::{Value, json};

const URI: &str = "file:///definicion.qz";

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

fn definicion(cliente: &Connection, linea: u32, caracter: u32) -> Option<Location> {
    cliente
        .sender
        .send(
            Request::new(
                RequestId::from(2),
                "textDocument/definition".to_string(),
                json!({"textDocument":{"uri":URI},"position":{"line":linea,"character":caracter}}),
            )
            .into(),
        )
        .expect("debe solicitar definición");
    let resultado = recibir_respuesta(cliente)
        .response_result
        .expect("definition debe responder");
    let respuesta: Option<GotoDefinitionResponse> =
        serde_json::from_value(resultado).expect("la respuesta debe ser definition o null");
    match respuesta {
        Some(GotoDefinitionResponse::Scalar(ubicacion)) => Some(ubicacion),
        Some(otro) => panic!("se esperaba una única ubicación: {otro:?}"),
        None => None,
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
fn initialize_deberia_anunciar_definiciones() {
    let (cliente, hilo, inicializacion) = inicializar();
    assert_eq!(inicializacion["capabilities"]["definitionProvider"], true);
    terminar(cliente, hilo);
}

#[test]
fn definition_deberia_saltar_al_nombre_de_una_variable() {
    let (cliente, hilo, _) = inicializar();
    abrir(&cliente, "entero var saldo = 10\nsaldo = 20");
    let ubicacion = definicion(&cliente, 1, 2).expect("debe encontrar la definición");
    assert_eq!(ubicacion.uri.to_string(), URI);
    assert_eq!(ubicacion.range.start.line, 0);
    assert_eq!(ubicacion.range.start.character, 11);
    assert_eq!(ubicacion.range.end.character, 16);
    terminar(cliente, hilo);
}

#[test]
fn definition_deberia_saltar_a_una_funcion() {
    let (cliente, hilo, _) = inicializar();
    abrir(
        &cliente,
        "entero sumar(entero a, entero b) { retornar a + b }\nsumar(1, 2)",
    );
    let ubicacion = definicion(&cliente, 1, 2).expect("debe encontrar la función");
    assert_eq!(ubicacion.range.start.line, 0);
    assert_eq!(ubicacion.range.start.character, 7);
    assert_eq!(ubicacion.range.end.character, 12);
    terminar(cliente, hilo);
}

#[test]
fn definition_deberia_reconocer_la_propia_declaracion() {
    let (cliente, hilo, _) = inicializar();
    abrir(&cliente, "texto saludo = \"hola\"");
    let ubicacion = definicion(&cliente, 0, 8).expect("debe encontrar la declaración");
    assert_eq!(ubicacion.range.start.character, 6);
    terminar(cliente, hilo);
}

#[test]
fn definition_deberia_respetar_el_simbolo_del_ambito_local() {
    let (cliente, hilo, _) = inicializar();
    abrir(
        &cliente,
        "entero var valor = 1\nvacio calcular() {\nentero var valor = 2\nvalor = 3\n}\nvalor = 4",
    );
    let ubicacion = definicion(&cliente, 3, 2).expect("debe encontrar la variable local");
    assert_eq!(ubicacion.range.start.line, 2);
    terminar(cliente, hilo);
}

#[test]
fn definition_deberia_devolver_null_para_un_simbolo_desconocido() {
    let (cliente, hilo, _) = inicializar();
    abrir(&cliente, "desconocido");
    assert!(definicion(&cliente, 0, 4).is_none());
    terminar(cliente, hilo);
}
