//! Pruebas de `quetzal/red`: `ClienteRs` y `ServidorRs` (RedSocket, el
//! equivalente en español de WebSocket).
//!
//! `ServidorRs` despacha sus manejadores (`al_conectar`/`al_mensaje`) desde
//! el hilo de la VM a través del bucle de eventos: hace falta bombearlo
//! (`Vm::drenar_bucle_eventos`) para que se ejecuten. Por eso, igual que en
//! `red_http.rs`/`red_socket.rs`, cada lado se prueba contra una
//! contraparte cruda hecha con `tungstenite` síncrono (sin tokio) desde un
//! hilo de Rust aparte: así ninguna lectura bloqueante del lado crudo
//! depende de que el otro lado (Quetzal) esté bombeando nada al mismo
//! tiempo. La contraparte cruda usa el crate `tungstenite` (en inglés)
//! porque implementa el protocolo WebSocket (RFC 6455) tal cual; el nombre
//! en español solo aplica a los tipos que expone nuestra API de Quetzal.

use std::net::TcpListener;
use std::rc::Rc;
use std::thread;

use indexmap::IndexMap;
use maquina_virtual::{Valor, Vm};
use nucleo::Fuente;
use tungstenite::Message;

fn ejecutar_con_permisos(codigo: &str, permisos_json: serde_json::Value) -> (Vm, Rc<maquina_virtual::valores::EntornoModulo>) {
    let fuente = Fuente::nueva("prueba.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("el código de prueba debe parsear");
    semantica::analizar_modulo(&ast).expect("el código de prueba debe pasar la semántica");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar bytecode");

    let permisos = paquetes::Permisos::desde_json(&permisos_json).expect("permisos de prueba válidos");
    let guardian = Rc::new(runtime::GuardianPermisos::denegado());
    guardian.configurar(permisos, std::path::Path::new("."));
    let registro = modulos_nativos::crear_registro_con_permisos(&guardian);

    let mut importaciones = IndexMap::new();
    for importacion in &modulo.importaciones {
        if let Some(nombre_modulo) = importacion.origen.strip_prefix("quetzal/") {
            let modulo_normalizado = maquina_virtual::normalizar_nombre(nombre_modulo).to_lowercase();
            for (nombre, local) in &importacion.simbolos {
                let simbolo = maquina_virtual::normalizar_nombre(nombre).to_lowercase().replace('_', "");
                let destino = modulos_nativos::modulo_de_tipo_exportado(&modulo_normalizado, &simbolo)
                    .map(str::to_string)
                    .unwrap_or_else(|| modulo_normalizado.clone());
                importaciones.insert(
                    local.clone(),
                    maquina_virtual::Variable {
                        valor: Valor::ModuloNativo(Rc::from(destino.as_str())),
                        mutable: false,
                    },
                );
            }
        }
    }

    let mut vm = Vm::nueva(Rc::new(registro));
    let (entorno, _valor) = vm
        .cargar_modulo(Rc::new(modulo), importaciones)
        .expect("el código de prueba debe ejecutar sin errores");
    (vm, entorno)
}

fn ejecutar_con_permiso_cliente(codigo: &str) -> (Vm, Rc<maquina_virtual::valores::EntornoModulo>) {
    ejecutar_con_permisos(codigo, serde_json::json!({ "red": { "habilitado": true, "cliente": true } }))
}

fn ejecutar_con_permiso_servidor(codigo: &str) -> (Vm, Rc<maquina_virtual::valores::EntornoModulo>) {
    ejecutar_con_permisos(codigo, serde_json::json!({ "red": { "habilitado": true, "servidor": true } }))
}

fn texto_global(entorno: &maquina_virtual::valores::EntornoModulo, nombre: &str) -> String {
    match &entorno
        .globales
        .borrow()
        .get(nombre)
        .unwrap_or_else(|| panic!("debe existir '{nombre}'"))
        .valor
    {
        Valor::Texto(texto) => texto.to_string(),
        otro => maquina_virtual::texto_de_valor(otro),
    }
}

fn entero_global(entorno: &maquina_virtual::valores::EntornoModulo, nombre: &str) -> i64 {
    match entorno
        .globales
        .borrow()
        .get(nombre)
        .unwrap_or_else(|| panic!("debe existir '{nombre}'"))
        .valor
    {
        Valor::Entero(numero) => numero,
        ref otro => panic!("se esperaba un entero en '{nombre}', llegó {}", otro.nombre_tipo()),
    }
}

fn logico_global(entorno: &maquina_virtual::valores::EntornoModulo, nombre: &str) -> bool {
    match entorno
        .globales
        .borrow()
        .get(nombre)
        .unwrap_or_else(|| panic!("debe existir '{nombre}'"))
        .valor
    {
        Valor::Log(valor) => valor,
        ref otro => panic!("se esperaba un lógico en '{nombre}', llegó {}", otro.nombre_tipo()),
    }
}

// =====================================================================
// ClienteRs: pruebas contra un servidor WebSocket crudo (tungstenite síncrono)
// =====================================================================

/// Servidor WebSocket de eco en un puerto efímero: atiende una conexión,
/// hace el *handshake* y responde `"eco:" + texto` a los marcos de texto, o
/// el mismo binario a los marcos binarios, hasta que el cliente cierra.
fn iniciar_servidor_rs_eco() -> u16 {
    let escucha = TcpListener::bind("127.0.0.1:0").expect("bind en puerto efímero");
    let puerto = escucha.local_addr().expect("dirección local").port();
    thread::spawn(move || {
        if let Ok((flujo, _)) = escucha.accept() {
            let Ok(mut socket) = tungstenite::accept(flujo) else {
                return;
            };
            loop {
                match socket.read() {
                    Ok(Message::Text(texto)) => {
                        let _ = socket.send(Message::Text(format!("eco:{texto}").into()));
                    }
                    Ok(Message::Binary(bytes)) => {
                        let _ = socket.send(Message::Binary(bytes));
                    }
                    Ok(Message::Close(_)) | Err(_) => break,
                    _ => {}
                }
            }
        }
    });
    puerto
}

#[test]
fn cliente_rs_deberia_conectarse_y_hacer_eco_de_texto_y_binario() {
    let puerto = iniciar_servidor_rs_eco();
    let (_vm, entorno) = ejecutar_con_permiso_cliente(&format!(
        "importar {{ ClienteRs }} desde \"quetzal/red\"\n\
         importar {{ Bits }} desde \"quetzal/bits\"\n\
         ClienteRs base = nuevo ClienteRs()\n\
         log base_conectada = base.esta_conectado()\n\
         ClienteRs cliente = base.conectar(\"ws://127.0.0.1:{puerto}/\")\n\
         log conectada = cliente.esta_conectado()\n\
         cliente.enviar_ping()\n\
         cliente.enviar_texto(\"hola rs\")\n\
         texto respuesta = cliente.recibir()\n\
         cliente.enviar_bits(Bits.desde_hex(\"48656c6c6f\"))\n\
         Bits eco = cliente.recibir()\n\
         texto eco_hex = eco.a_hex()\n\
         cliente.cerrar()\n\
         log cerrada = cliente.esta_conectado()\n"
    ));
    assert!(!logico_global(&entorno, "base_conectada"));
    assert!(logico_global(&entorno, "conectada"));
    assert_eq!(texto_global(&entorno, "respuesta"), "eco:hola rs");
    assert_eq!(texto_global(&entorno, "eco_hex"), "48656c6c6f");
    assert!(!logico_global(&entorno, "cerrada"));
}

#[test]
fn cliente_rs_conectar_sin_permiso_deberia_fallar_con_e0701() {
    let fuente = Fuente::nueva(
        "prueba.qz",
        "importar { ClienteRs } desde \"quetzal/red\"\n\
         ClienteRs base = nuevo ClienteRs()\n\
         base.conectar(\"ws://127.0.0.1:9/\")\n",
    );
    let ast = sintaxis::parsear_modulo(&fuente).expect("el código de prueba debe parsear");
    semantica::analizar_modulo(&ast).expect("el código de prueba debe pasar la semántica");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar bytecode");

    let mut importaciones = IndexMap::new();
    for importacion in &modulo.importaciones {
        if let Some(nombre_modulo) = importacion.origen.strip_prefix("quetzal/") {
            let modulo_normalizado = maquina_virtual::normalizar_nombre(nombre_modulo).to_lowercase();
            for (nombre, local) in &importacion.simbolos {
                let simbolo = maquina_virtual::normalizar_nombre(nombre).to_lowercase().replace('_', "");
                let destino = modulos_nativos::modulo_de_tipo_exportado(&modulo_normalizado, &simbolo)
                    .map(str::to_string)
                    .unwrap_or_else(|| modulo_normalizado.clone());
                importaciones.insert(
                    local.clone(),
                    maquina_virtual::Variable {
                        valor: Valor::ModuloNativo(Rc::from(destino.as_str())),
                        mutable: false,
                    },
                );
            }
        }
    }

    let registro = modulos_nativos::crear_registro();
    let mut vm = Vm::nueva(Rc::new(registro));
    let error = vm
        .cargar_modulo(Rc::new(modulo), importaciones)
        .expect_err("sin permiso de red conectar debe fallar");
    assert_eq!(error.codigo, "E0701");
}

// =====================================================================
// ServidorRs: pruebas contra un cliente WebSocket crudo (tungstenite síncrono)
// =====================================================================

#[test]
fn servidor_rs_deberia_invocar_al_conectar_y_al_mensaje() {
    let (mut vm, entorno) = ejecutar_con_permiso_servidor(
        "importar { ServidorRs, ConexionRs } desde \"quetzal/red\"\n\
         ServidorRs servidor = nuevo ServidorRs()\n\
         entero var conexiones = 0\n\
         vacio al_conectar(ConexionRs conexion) {\n\
         \u{20}   conexiones = conexiones + 1\n\
         }\n\
         vacio al_mensaje(ConexionRs conexion, texto mensaje) {\n\
         \u{20}   conexion.enviar_texto(\"eco:\" + mensaje)\n\
         \u{20}   conexion.cerrar()\n\
         \u{20}   servidor.detener()\n\
         }\n\
         servidor.al_conectar(al_conectar)\n\
         servidor.al_mensaje(al_mensaje)\n\
         servidor.escuchar(0)\n\
         entero puerto = servidor.puerto()\n",
    );
    let puerto = entero_global(&entorno, "puerto") as u16;

    let solicitud = thread::spawn(move || {
        let (mut socket, _respuesta) =
            tungstenite::connect(format!("ws://127.0.0.1:{puerto}/")).expect("el cliente de prueba debe conectarse");
        socket.send(Message::Text("hola rs".into())).expect("enviar el mensaje de prueba");
        match socket.read().expect("leer la respuesta de prueba") {
            Message::Text(texto) => texto.to_string(),
            otro => panic!("se esperaba un marco de texto, llegó {otro:?}"),
        }
    });

    vm.drenar_bucle_eventos();
    let respuesta = solicitud.join().expect("el hilo del cliente no debe entrar en panic");
    assert_eq!(respuesta, "eco:hola rs");
    assert_eq!(entero_global(&entorno, "conexiones"), 1);
    assert!(!vm.bucle().hay_trabajo_activo());
}

#[test]
fn servidor_rs_deberia_enviar_bits_binarios_desde_conexion_rs() {
    let (mut vm, entorno) = ejecutar_con_permiso_servidor(
        "importar { ServidorRs, ConexionRs } desde \"quetzal/red\"\n\
         importar { Bits } desde \"quetzal/bits\"\n\
         ServidorRs servidor = nuevo ServidorRs()\n\
         vacio al_mensaje(ConexionRs conexion, Bits mensaje) {\n\
         \u{20}   conexion.enviar_bits(mensaje)\n\
         \u{20}   conexion.cerrar()\n\
         \u{20}   servidor.detener()\n\
         }\n\
         servidor.al_mensaje(al_mensaje)\n\
         servidor.escuchar(0)\n\
         entero puerto = servidor.puerto()\n",
    );
    let puerto = entero_global(&entorno, "puerto") as u16;

    let solicitud = thread::spawn(move || {
        let (mut socket, _respuesta) =
            tungstenite::connect(format!("ws://127.0.0.1:{puerto}/")).expect("el cliente de prueba debe conectarse");
        socket
            .send(Message::Binary(vec![0x48, 0x65, 0x6c, 0x6c, 0x6f].into()))
            .expect("enviar los bytes de prueba");
        match socket.read().expect("leer el eco binario de prueba") {
            Message::Binary(bytes) => bytes.to_vec(),
            otro => panic!("se esperaba un marco binario, llegó {otro:?}"),
        }
    });

    vm.drenar_bucle_eventos();
    let eco = solicitud.join().expect("el hilo del cliente no debe entrar en panic");
    assert_eq!(eco, vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]);
    let _ = entorno;
}

#[test]
fn servidor_rs_escuchar_sin_permiso_deberia_fallar_con_e0701() {
    let fuente = Fuente::nueva(
        "prueba.qz",
        "importar { ServidorRs } desde \"quetzal/red\"\n\
         ServidorRs servidor = nuevo ServidorRs()\n\
         servidor.escuchar(0)\n",
    );
    let ast = sintaxis::parsear_modulo(&fuente).expect("el código de prueba debe parsear");
    semantica::analizar_modulo(&ast).expect("el código de prueba debe pasar la semántica");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar bytecode");

    let mut importaciones = IndexMap::new();
    for importacion in &modulo.importaciones {
        if let Some(nombre_modulo) = importacion.origen.strip_prefix("quetzal/") {
            let modulo_normalizado = maquina_virtual::normalizar_nombre(nombre_modulo).to_lowercase();
            for (nombre, local) in &importacion.simbolos {
                let simbolo = maquina_virtual::normalizar_nombre(nombre).to_lowercase().replace('_', "");
                let destino = modulos_nativos::modulo_de_tipo_exportado(&modulo_normalizado, &simbolo)
                    .map(str::to_string)
                    .unwrap_or_else(|| modulo_normalizado.clone());
                importaciones.insert(
                    local.clone(),
                    maquina_virtual::Variable {
                        valor: Valor::ModuloNativo(Rc::from(destino.as_str())),
                        mutable: false,
                    },
                );
            }
        }
    }

    let registro = modulos_nativos::crear_registro();
    let mut vm = Vm::nueva(Rc::new(registro));
    let error = vm
        .cargar_modulo(Rc::new(modulo), importaciones)
        .expect_err("sin permiso de red escuchar debe fallar");
    assert_eq!(error.codigo, "E0701");
}
