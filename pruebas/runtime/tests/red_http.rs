//! Pruebas de `quetzal/red`: cliente HTTP (`ClienteHttp`/`RespuestaHttp`,
//! síncrono y asincrónico) y servidor HTTP (`ServidorHttp`/`PeticionHttp`/
//! `RespuestaServidor`).
//!
//! Las pruebas del cliente usan un servidor HTTP mínimo hecho con
//! `std::net` (sin dependencias externas ni acceso a internet) que responde
//! con un eco en JSON de lo recibido, o un cuerpo binario/con estado fijo
//! según la ruta. Las pruebas del servidor levantan un `ServidorHttp` real
//! de Quetzal en un puerto efímero y le disparan peticiones con `reqwest`
//! desde un hilo aparte, bombeando el bucle de eventos de la VM
//! (`Vm::drenar_bucle_eventos`) mientras el manejador de la ruta detiene el
//! servidor para poder cerrar la prueba.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;
use std::thread;
use std::time::Duration;

use indexmap::IndexMap;
use maquina_virtual::{Valor, Vm};
use nucleo::Fuente;

// =====================================================================
// Servidor HTTP mínimo de prueba
// =====================================================================

struct PeticionCruda {
    metodo: String,
    ruta: String,
    cabeceras: HashMap<String, String>,
    cuerpo: Vec<u8>,
}

fn leer_linea(lector: &mut impl BufRead) -> String {
    let mut linea = String::new();
    lector.read_line(&mut linea).unwrap_or(0);
    linea.trim_end_matches(['\r', '\n']).to_string()
}

fn leer_peticion(flujo: &TcpStream) -> PeticionCruda {
    let mut lector = BufReader::new(flujo.try_clone().expect("clonar el flujo de prueba"));
    let primera_linea = leer_linea(&mut lector);
    let mut partes = primera_linea.split_whitespace();
    let metodo = partes.next().unwrap_or("").to_string();
    let ruta = partes.next().unwrap_or("/").to_string();

    let mut cabeceras = HashMap::new();
    loop {
        let linea = leer_linea(&mut lector);
        if linea.is_empty() {
            break;
        }
        if let Some((nombre, valor)) = linea.split_once(':') {
            cabeceras.insert(nombre.trim().to_lowercase(), valor.trim().to_string());
        }
    }

    let longitud: usize = cabeceras
        .get("content-length")
        .and_then(|valor| valor.parse().ok())
        .unwrap_or(0);
    let mut cuerpo = vec![0u8; longitud];
    if longitud > 0 {
        lector
            .read_exact(&mut cuerpo)
            .expect("leer el cuerpo de la petición de prueba");
    }

    PeticionCruda {
        metodo,
        ruta,
        cabeceras,
        cuerpo,
    }
}

/// Construye una respuesta HTTP/1.1 cruda con cierre de conexión inmediato.
fn respuesta_cruda(estado: u16, razon: &str, tipo_contenido: &str, cuerpo: &[u8]) -> Vec<u8> {
    let mut salida = format!(
        "HTTP/1.1 {estado} {razon}\r\nContent-Type: {tipo_contenido}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        cuerpo.len()
    )
    .into_bytes();
    salida.extend_from_slice(cuerpo);
    salida
}

/// Cuerpo binario fijo devuelto por la ruta `/binario` (para probar `.bits()`).
const BYTES_BINARIOS: [u8; 5] = [0x48, 0x6f, 0x6c, 0x61, 0x00];

fn manejar_conexion(flujo: TcpStream) {
    let peticion = leer_peticion(&flujo);
    let respuesta = match peticion.ruta.as_str() {
        "/binario" => respuesta_cruda(200, "OK", "application/octet-stream", &BYTES_BINARIOS),
        "/no_encontrado" => respuesta_cruda(404, "NOT FOUND", "text/plain", b"no encontrado"),
        _ => {
            let cuerpo_texto = String::from_utf8_lossy(&peticion.cuerpo).to_string();
            let json = serde_json::json!({
                "metodo": peticion.metodo,
                "ruta": peticion.ruta,
                "cabecera_prueba": peticion.cabeceras.get("x-prueba"),
                "cuerpo": cuerpo_texto,
            });
            respuesta_cruda(200, "OK", "application/json", json.to_string().as_bytes())
        }
    };
    let mut flujo = flujo;
    let _ = flujo.write_all(&respuesta);
    let _ = flujo.flush();
}

/// Inicia un servidor de prueba en un puerto efímero de loopback que atiende
/// exactamente `cantidad` peticiones y luego termina el hilo.
fn iniciar_servidor(cantidad: usize) -> u16 {
    let escucha = TcpListener::bind("127.0.0.1:0").expect("bind en puerto efímero");
    let puerto = escucha.local_addr().expect("dirección local").port();
    thread::spawn(move || {
        for flujo in escucha.incoming().take(cantidad).flatten() {
            manejar_conexion(flujo);
        }
    });
    puerto
}

// =====================================================================
// Ejecución de código Quetzal con permiso de red (cliente) habilitado
// =====================================================================

fn ejecutar_con_permiso_cliente(codigo: &str) -> Rc<maquina_virtual::valores::EntornoModulo> {
    let fuente = Fuente::nueva("prueba.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("el código de prueba debe parsear");
    semantica::analizar_modulo(&ast).expect("el código de prueba debe pasar la semántica");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar bytecode");

    let permisos_json = serde_json::json!({
        "red": { "habilitado": true, "cliente": true }
    });
    let permisos =
        paquetes::Permisos::desde_json(&permisos_json).expect("permisos de prueba válidos");
    let guardian = Rc::new(runtime::GuardianPermisos::denegado());
    guardian.configurar(permisos, std::path::Path::new("."));
    let registro = modulos_nativos::crear_registro_con_permisos(&guardian);

    let mut importaciones = IndexMap::new();
    for importacion in &modulo.importaciones {
        if let Some(nombre_modulo) = importacion.origen.strip_prefix("quetzal/") {
            let modulo_normalizado =
                maquina_virtual::normalizar_nombre(nombre_modulo).to_lowercase();
            for (nombre, local) in &importacion.simbolos {
                let simbolo = maquina_virtual::normalizar_nombre(nombre)
                    .to_lowercase()
                    .replace('_', "");
                let destino =
                    modulos_nativos::modulo_de_tipo_exportado(&modulo_normalizado, &simbolo)
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
    entorno
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

// =====================================================================
// ServidorHttp: ejecución con permiso de servidor habilitado
// =====================================================================

/// Igual que [`ejecutar_con_permiso_cliente`], pero habilitando el permiso
/// de servidor (sin restricción de puertos, para poder usar un puerto
/// efímero con `escuchar(0)`) y devolviendo también la VM: las pruebas de
/// `ServidorHttp` necesitan seguir bombeando el bucle de eventos después de
/// que el código principal terminó (`Vm::drenar_bucle_eventos`), para
/// atender las peticiones entrantes.
fn ejecutar_con_permiso_servidor(codigo: &str) -> (Vm, Rc<maquina_virtual::valores::EntornoModulo>) {
    let fuente = Fuente::nueva("prueba.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("el código de prueba debe parsear");
    semantica::analizar_modulo(&ast).expect("el código de prueba debe pasar la semántica");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar bytecode");

    let permisos_json = serde_json::json!({
        "red": { "habilitado": true, "servidor": true }
    });
    let permisos =
        paquetes::Permisos::desde_json(&permisos_json).expect("permisos de prueba válidos");
    let guardian = Rc::new(runtime::GuardianPermisos::denegado());
    guardian.configurar(permisos, std::path::Path::new("."));
    let registro = modulos_nativos::crear_registro_con_permisos(&guardian);

    let mut importaciones = IndexMap::new();
    for importacion in &modulo.importaciones {
        if let Some(nombre_modulo) = importacion.origen.strip_prefix("quetzal/") {
            let modulo_normalizado =
                maquina_virtual::normalizar_nombre(nombre_modulo).to_lowercase();
            for (nombre, local) in &importacion.simbolos {
                let simbolo = maquina_virtual::normalizar_nombre(nombre)
                    .to_lowercase()
                    .replace('_', "");
                let destino =
                    modulos_nativos::modulo_de_tipo_exportado(&modulo_normalizado, &simbolo)
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

#[test]
fn servidor_http_deberia_responder_con_parametros_de_ruta() {
    let (mut vm, entorno) = ejecutar_con_permiso_servidor(
        "importar { ServidorHttp, PeticionHttp } desde \"quetzal/red\"\n\
         ServidorHttp servidor = nuevo ServidorHttp()\n\
         texto manejador_saludo(PeticionHttp peticion) {\n\
         \u{20}   servidor.detener()\n\
         \u{20}   jsn parametros = peticion.parametros()\n\
         \u{20}   texto nombre = parametros[\"nombre\"]\n\
         \u{20}   retornar \"hola \" + nombre\n\
         }\n\
         servidor.obtener(\"/saludo/:nombre\", manejador_saludo)\n\
         servidor.escuchar(0)\n\
         entero puerto = servidor.puerto()\n",
    );
    let puerto = entero_global(&entorno, "puerto") as u16;
    assert!(puerto > 0, "el sistema operativo debe asignar un puerto real");

    let solicitud = thread::spawn(move || {
        let cliente = reqwest::blocking::Client::new();
        cliente
            .get(format!("http://127.0.0.1:{puerto}/saludo/Quetzal"))
            .send()
            .expect("la petición al servidor de prueba debe completarse")
            .text()
            .expect("debe poder leerse el cuerpo de la respuesta")
    });

    vm.drenar_bucle_eventos();
    let cuerpo = solicitud.join().expect("el hilo de la petición no debe entrar en panic");
    assert_eq!(cuerpo, "hola Quetzal");
    assert!(!vm.bucle().hay_trabajo_activo());
}

#[test]
fn servidor_http_deberia_leer_cuerpo_jsn_y_responder_con_respuestas_crear() {
    let (mut vm, entorno) = ejecutar_con_permiso_servidor(
        "importar { ServidorHttp, PeticionHttp, RespuestaServidor, Respuestas } desde \"quetzal/red\"\n\
         ServidorHttp servidor = nuevo ServidorHttp()\n\
         RespuestaServidor manejador_crear(PeticionHttp peticion) {\n\
         \u{20}   servidor.detener()\n\
         \u{20}   jsn cuerpo = peticion.jsn()\n\
         \u{20}   texto nombre = cuerpo[\"nombre\"]\n\
         \u{20}   RespuestaServidor respuesta = Respuestas.crear(201, \"creado: \" + nombre)\n\
         \u{20}   respuesta.fijar_cabecera(\"X-Creado-Por\", \"quetzal\")\n\
         \u{20}   retornar respuesta\n\
         }\n\
         servidor.publicar(\"/elementos\", manejador_crear)\n\
         servidor.escuchar(0)\n\
         entero puerto = servidor.puerto()\n",
    );
    let puerto = entero_global(&entorno, "puerto") as u16;

    let solicitud = thread::spawn(move || {
        let cliente = reqwest::blocking::Client::new();
        cliente
            .post(format!("http://127.0.0.1:{puerto}/elementos"))
            .header("Content-Type", "application/json")
            .body("{\"nombre\": \"ave\"}")
            .send()
            .expect("la petición al servidor de prueba debe completarse")
    });

    vm.drenar_bucle_eventos();
    let respuesta = solicitud.join().expect("el hilo de la petición no debe entrar en panic");
    assert_eq!(respuesta.status().as_u16(), 201);
    assert_eq!(
        respuesta.headers().get("x-creado-por").map(|v| v.to_str().unwrap_or("")),
        Some("quetzal")
    );
    let cuerpo = respuesta.text().expect("debe leerse el cuerpo");
    assert_eq!(cuerpo, "creado: ave");
    let _ = entorno;
}

#[test]
fn servidor_http_deberia_responder_bits_binarios() {
    let (mut vm, entorno) = ejecutar_con_permiso_servidor(
        "importar { ServidorHttp, PeticionHttp } desde \"quetzal/red\"\n\
         importar { Bits } desde \"quetzal/bits\"\n\
         ServidorHttp servidor = nuevo ServidorHttp()\n\
         Bits manejador_binario(PeticionHttp peticion) {\n\
         \u{20}   servidor.detener()\n\
         \u{20}   retornar Bits.desde_hex(\"48656c6c6f\")\n\
         }\n\
         servidor.obtener(\"/binario\", manejador_binario)\n\
         servidor.escuchar(0)\n\
         entero puerto = servidor.puerto()\n",
    );
    let puerto = entero_global(&entorno, "puerto") as u16;

    let solicitud = thread::spawn(move || {
        let cliente = reqwest::blocking::Client::new();
        cliente
            .get(format!("http://127.0.0.1:{puerto}/binario"))
            .send()
            .expect("la petición al servidor de prueba debe completarse")
            .bytes()
            .expect("debe poder leerse el cuerpo binario")
            .to_vec()
    });

    vm.drenar_bucle_eventos();
    let cuerpo = solicitud.join().expect("el hilo de la petición no debe entrar en panic");
    assert_eq!(cuerpo, b"Hello");
    let _ = entorno;
}

#[test]
fn servidor_http_ruta_no_encontrada_deberia_responder_404() {
    // El manejador registrado en "/existe" es el que finalmente detiene el
    // servidor (para poder cerrar la prueba): antes se comprueba que una
    // ruta sin coincidencia responde 404 sin detener nada.
    let (mut vm, entorno) = ejecutar_con_permiso_servidor(
        "importar { ServidorHttp, PeticionHttp } desde \"quetzal/red\"\n\
         ServidorHttp servidor = nuevo ServidorHttp()\n\
         texto manejador(PeticionHttp peticion) {\n\
         \u{20}   servidor.detener()\n\
         \u{20}   retornar \"listo\"\n\
         }\n\
         servidor.obtener(\"/existe\", manejador)\n\
         servidor.escuchar(0)\n\
         entero puerto = servidor.puerto()\n",
    );
    let puerto = entero_global(&entorno, "puerto") as u16;

    let solicitud = thread::spawn(move || {
        let cliente = reqwest::blocking::Client::new();
        let codigo_no_encontrado = cliente
            .get(format!("http://127.0.0.1:{puerto}/no-existe"))
            .timeout(Duration::from_secs(5))
            .send()
            .expect("la petición debe completarse aunque la ruta no exista")
            .status()
            .as_u16();
        let cuerpo_existente = cliente
            .get(format!("http://127.0.0.1:{puerto}/existe"))
            .timeout(Duration::from_secs(5))
            .send()
            .expect("la segunda petición debe completarse")
            .text()
            .expect("debe leerse el cuerpo");
        (codigo_no_encontrado, cuerpo_existente)
    });

    vm.drenar_bucle_eventos();
    let (codigo_no_encontrado, cuerpo_existente) =
        solicitud.join().expect("el hilo de la petición no debe entrar en panic");
    assert_eq!(codigo_no_encontrado, 404);
    assert_eq!(cuerpo_existente, "listo");
    let _ = entorno;
}

#[test]
fn servidor_http_sin_permiso_deberia_fallar_con_e0701() {
    let fuente = Fuente::nueva(
        "prueba.qz",
        "importar { ServidorHttp } desde \"quetzal/red\"\n\
         ServidorHttp servidor = nuevo ServidorHttp()\n\
         servidor.escuchar(0)\n",
    );
    let ast = sintaxis::parsear_modulo(&fuente).expect("el código de prueba debe parsear");
    semantica::analizar_modulo(&ast).expect("el código de prueba debe pasar la semántica");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar bytecode");

    let mut importaciones = IndexMap::new();
    for importacion in &modulo.importaciones {
        if let Some(nombre_modulo) = importacion.origen.strip_prefix("quetzal/") {
            let modulo_normalizado =
                maquina_virtual::normalizar_nombre(nombre_modulo).to_lowercase();
            for (nombre, local) in &importacion.simbolos {
                let simbolo = maquina_virtual::normalizar_nombre(nombre)
                    .to_lowercase()
                    .replace('_', "");
                let destino =
                    modulos_nativos::modulo_de_tipo_exportado(&modulo_normalizado, &simbolo)
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
        .expect_err("sin permiso de red la creación del servidor debe fallar");
    assert_eq!(error.codigo, "E0701");
}

// =====================================================================
// Pruebas
// =====================================================================

#[test]
fn cliente_http_deberia_hacer_get_sincrono() {
    let puerto = iniciar_servidor(1);
    let entorno = ejecutar_con_permiso_cliente(&format!(
        "importar {{ ClienteHttp, RespuestaHttp }} desde \"quetzal/red\"\n\
         ClienteHttp cliente = nuevo ClienteHttp()\n\
         RespuestaHttp respuesta = cliente.obtener(\"http://127.0.0.1:{puerto}/saludo\")\n\
         entero estado = respuesta.estado()\n\
         jsn cuerpo = respuesta.jsn()\n\
         texto metodo = cuerpo[\"metodo\"]\n\
         texto ruta = cuerpo[\"ruta\"]\n"
    ));
    assert_eq!(texto_global(&entorno, "estado"), "200");
    assert_eq!(texto_global(&entorno, "metodo"), "GET");
    assert_eq!(texto_global(&entorno, "ruta"), "/saludo");
}

#[test]
fn cliente_http_deberia_enviar_cuerpo_y_cabecera_personalizada() {
    let puerto = iniciar_servidor(1);
    let entorno = ejecutar_con_permiso_cliente(&format!(
        "importar {{ ClienteHttp, RespuestaHttp }} desde \"quetzal/red\"\n\
         ClienteHttp cliente = nuevo ClienteHttp()\n\
         cliente.fijar_cabecera(\"X-Prueba\", \"valor-personalizado\")\n\
         RespuestaHttp respuesta = cliente.publicar(\"http://127.0.0.1:{puerto}/eco\", \"hola mundo\")\n\
         jsn cuerpo = respuesta.jsn()\n\
         texto metodo = cuerpo[\"metodo\"]\n\
         texto eco = cuerpo[\"cuerpo\"]\n\
         texto cabecera = cuerpo[\"cabecera_prueba\"]\n"
    ));
    assert_eq!(texto_global(&entorno, "metodo"), "POST");
    assert_eq!(texto_global(&entorno, "eco"), "hola mundo");
    assert_eq!(texto_global(&entorno, "cabecera"), "valor-personalizado");
}

#[test]
fn cliente_http_deberia_soportar_todos_los_verbos_en_espanol() {
    let puerto = iniciar_servidor(4);
    let entorno = ejecutar_con_permiso_cliente(&format!(
        "importar {{ ClienteHttp }} desde \"quetzal/red\"\n\
         ClienteHttp cliente = nuevo ClienteHttp()\n\
         jsn r1 = cliente.poner(\"http://127.0.0.1:{puerto}/eco\", \"a\").jsn()\n\
         jsn r2 = cliente.parchar(\"http://127.0.0.1:{puerto}/eco\", \"b\").jsn()\n\
         jsn r3 = cliente.eliminar(\"http://127.0.0.1:{puerto}/eco\").jsn()\n\
         jsn r4 = cliente.pedir(\"OBTENER\", \"http://127.0.0.1:{puerto}/eco\", nulo).jsn()\n\
         texto m1 = r1[\"metodo\"]\n\
         texto m2 = r2[\"metodo\"]\n\
         texto m3 = r3[\"metodo\"]\n\
         texto m4 = r4[\"metodo\"]\n"
    ));
    assert_eq!(texto_global(&entorno, "m1"), "PUT");
    assert_eq!(texto_global(&entorno, "m2"), "PATCH");
    assert_eq!(texto_global(&entorno, "m3"), "DELETE");
    assert_eq!(texto_global(&entorno, "m4"), "GET");
}

#[test]
fn cliente_http_deberia_leer_cuerpo_binario_con_bits() {
    let puerto = iniciar_servidor(1);
    let entorno = ejecutar_con_permiso_cliente(&format!(
        "importar {{ ClienteHttp, RespuestaHttp }} desde \"quetzal/red\"\n\
         importar {{ Bits }} desde \"quetzal/bits\"\n\
         ClienteHttp cliente = nuevo ClienteHttp()\n\
         RespuestaHttp respuesta = cliente.obtener(\"http://127.0.0.1:{puerto}/binario\")\n\
         Bits cuerpo = respuesta.bits()\n\
         texto hex = cuerpo.a_hex()\n\
         entero largo = cuerpo.longitud()\n"
    ));
    assert_eq!(texto_global(&entorno, "hex"), "486f6c6100");
    assert_eq!(texto_global(&entorno, "largo"), "5");
}

#[test]
fn cliente_http_deberia_exponer_estado_no_exitoso() {
    let puerto = iniciar_servidor(1);
    let entorno = ejecutar_con_permiso_cliente(&format!(
        "importar {{ ClienteHttp, RespuestaHttp }} desde \"quetzal/red\"\n\
         ClienteHttp cliente = nuevo ClienteHttp()\n\
         RespuestaHttp respuesta = cliente.obtener(\"http://127.0.0.1:{puerto}/no_encontrado\")\n\
         entero estado = respuesta.estado()\n\
         texto cuerpo_error = respuesta.texto()\n"
    ));
    assert_eq!(texto_global(&entorno, "estado"), "404");
    assert_eq!(texto_global(&entorno, "cuerpo_error"), "no encontrado");
}

#[test]
fn cliente_http_deberia_resolverse_de_forma_asincrona_con_esperar() {
    let puerto = iniciar_servidor(1);
    let entorno = ejecutar_con_permiso_cliente(&format!(
        "importar {{ ClienteHttp, RespuestaHttp }} desde \"quetzal/red\"\n\
         ClienteHttp cliente = nuevo ClienteHttp()\n\
         RespuestaHttp respuesta = esperar cliente.obtener_asincrono(\"http://127.0.0.1:{puerto}/saludo\")\n\
         entero estado = respuesta.estado()\n\
         jsn cuerpo = respuesta.jsn()\n\
         texto ruta = cuerpo[\"ruta\"]\n"
    ));
    assert_eq!(texto_global(&entorno, "estado"), "200");
    assert_eq!(texto_global(&entorno, "ruta"), "/saludo");
}

#[test]
fn cliente_http_sin_permiso_deberia_fallar_con_e0701() {
    let fuente = Fuente::nueva(
        "prueba.qz",
        "importar { ClienteHttp } desde \"quetzal/red\"\n\
         ClienteHttp cliente = nuevo ClienteHttp()\n\
         cliente.obtener(\"http://127.0.0.1:9/lo-que-sea\")\n",
    );
    let ast = sintaxis::parsear_modulo(&fuente).expect("el código de prueba debe parsear");
    semantica::analizar_modulo(&ast).expect("el código de prueba debe pasar la semántica");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar bytecode");

    let mut importaciones = IndexMap::new();
    for importacion in &modulo.importaciones {
        if let Some(nombre_modulo) = importacion.origen.strip_prefix("quetzal/") {
            let modulo_normalizado =
                maquina_virtual::normalizar_nombre(nombre_modulo).to_lowercase();
            for (nombre, local) in &importacion.simbolos {
                let simbolo = maquina_virtual::normalizar_nombre(nombre)
                    .to_lowercase()
                    .replace('_', "");
                let destino =
                    modulos_nativos::modulo_de_tipo_exportado(&modulo_normalizado, &simbolo)
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

    // Sin configurar ningún permiso: el guardián deniega todo por defecto.
    let registro = modulos_nativos::crear_registro();
    let mut vm = Vm::nueva(Rc::new(registro));
    let error = vm
        .cargar_modulo(Rc::new(modulo), importaciones)
        .expect_err("sin permiso de red la petición debe fallar");
    assert_eq!(error.codigo, "E0701");
}
