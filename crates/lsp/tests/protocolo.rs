//! Pruebas del transporte y ciclo de vida LSP.

use std::{
    io::{BufReader, Write},
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use serde_json::{Value, json};

fn iniciar_servidor() -> (
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
    let respuesta = recibir_respuesta(&cliente);
    assert!(respuesta.response_result.is_ok());
    cliente
        .sender
        .send(Notification::new("initialized".to_string(), json!({})).into())
        .expect("debe enviar initialized");
    (cliente, hilo)
}

fn recibir_respuesta(conexion: &Connection) -> Response {
    let mensaje = conexion
        .receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("debe recibir una respuesta");
    match mensaje {
        Message::Response(respuesta) => respuesta,
        otro => panic!("se esperaba respuesta, se recibió: {otro:?}"),
    }
}

fn terminar_servidor(
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
fn initialize_deberia_negociar_utf16() {
    let (servidor, cliente) = Connection::memory();
    let _hilo = thread::spawn(move || lsp::servidor::ejecutar(servidor));
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
    let respuesta = recibir_respuesta(&cliente);
    let resultado = respuesta
        .response_result
        .expect("initialize debe responder");
    assert_eq!(resultado["capabilities"]["positionEncoding"], "utf-16");
}

#[test]
fn solicitud_desconocida_deberia_responder_metodo_no_encontrado() {
    let (cliente, hilo) = iniciar_servidor();
    cliente
        .sender
        .send(Request::new(RequestId::from(2), "quetzal/desconocido".to_string(), ()).into())
        .expect("debe enviar solicitud");
    let respuesta = recibir_respuesta(&cliente);
    let error = respuesta
        .response_result
        .expect_err("debe rechazar el método");
    assert_eq!(error.code, ErrorCode::MethodNotFound as i32);
    terminar_servidor(cliente, hilo);
}

#[test]
fn shutdown_y_exit_deberian_terminar_limpamente() {
    let (cliente, hilo) = iniciar_servidor();
    terminar_servidor(cliente, hilo);
}

#[test]
fn stdio_deberia_transportar_solo_mensajes_lsp() {
    let mut hijo = Command::new(env!("CARGO_BIN_EXE_quetzal-lsp"))
        .arg("--stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("debe iniciar quetzal-lsp");
    let mut entrada = hijo.stdin.take().expect("stdin debe estar disponible");
    for mensaje in [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}),
        json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
        json!({"jsonrpc":"2.0","id":2,"method":"shutdown"}),
        json!({"jsonrpc":"2.0","method":"exit"}),
    ] {
        let contenido = mensaje.to_string();
        write!(
            entrada,
            "Content-Length: {}\r\n\r\n{contenido}",
            contenido.len()
        )
        .expect("debe escribir mensaje LSP");
    }
    drop(entrada);
    let salida = hijo.wait_with_output().expect("debe terminar quetzal-lsp");
    assert!(
        salida.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&salida.stderr)
    );

    let mut lector = BufReader::new(salida.stdout.as_slice());
    let mut respuestas = Vec::new();
    while let Some(mensaje) = Message::read(&mut lector).expect("stdout solo debe contener LSP") {
        if let Message::Response(respuesta) = mensaje {
            respuestas.push(respuesta);
        }
    }
    assert_eq!(respuestas.len(), 2);
}

#[test]
fn parametros_initialize_invalidos_deberian_cerrar_con_error() {
    let (servidor, cliente) = Connection::memory();
    let hilo = thread::spawn(move || lsp::servidor::ejecutar(servidor));
    cliente
        .sender
        .send(Request::new(RequestId::from(1), "initialize".to_string(), Value::Null).into())
        .expect("debe enviar initialize inválido");
    let error = hilo
        .join()
        .expect("el servidor no debe entrar en pánico")
        .expect_err("los parámetros inválidos deben rechazarse");
    assert!(matches!(
        error,
        lsp::servidor::ErrorServidor::Serializacion(_)
    ));
}
