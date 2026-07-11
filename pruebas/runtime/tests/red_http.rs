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
        "/redirigir" => b"HTTP/1.1 302 Found\r\nLocation: /destino\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
        "/galletas" => b"HTTP/1.1 200 OK\r\nSet-Cookie: a=1\r\nSet-Cookie: b=2\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
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
        "red": { "habilitado": true, "cliente": true },
        "sistema_archivos": {
            "habilitado": true,
            "directorios": [{"ruta": ".", "permiso": "todo"}]
        }
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
fn ejecutar_con_permiso_servidor(
    codigo: &str,
) -> (Vm, Rc<maquina_virtual::valores::EntornoModulo>) {
    let fuente = Fuente::nueva("prueba.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("el código de prueba debe parsear");
    semantica::analizar_modulo(&ast).expect("el código de prueba debe pasar la semántica");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar bytecode");

    let permisos_json = serde_json::json!({
        "red": { "habilitado": true, "servidor": true, "cliente": true }
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
        ref otro => panic!(
            "se esperaba un entero en '{nombre}', llegó {}",
            otro.nombre_tipo()
        ),
    }
}

fn enviar_peticion_cruda(puerto: u16, peticion: &[u8]) -> Vec<u8> {
    let mut flujo = TcpStream::connect(("127.0.0.1", puerto))
        .expect("debe conectar con el servidor HTTP de Quetzal");
    flujo
        .write_all(peticion)
        .expect("debe escribir la petición HTTP cruda");
    flujo.flush().expect("debe vaciar la petición HTTP cruda");
    let mut respuesta = Vec::new();
    flujo
        .read_to_end(&mut respuesta)
        .expect("debe leer la respuesta HTTP cruda");
    respuesta
}

#[test]
fn servidor_http_deberia_leer_cuerpo_chunked_y_exponer_metadatos() {
    let (mut vm, entorno) = ejecutar_con_permiso_servidor(
        "importar { ServidorHttp, PeticionHttp } desde \"quetzal/red\"\n\
         ServidorHttp servidor = nuevo ServidorHttp()\n\
         texto manejar(PeticionHttp peticion) {\n\
         \u{20}   servidor.detener()\n\
         \u{20}   texto protocolo = peticion.protocolo()\n\
         \u{20}   texto cuerpo = peticion.texto()\n\
         \u{20}   retornar t\"{protocolo}|{cuerpo}\"\n\
         }\n\
         servidor.publicar(\"/trozos\", manejar)\n\
         servidor.escuchar(0)\n\
         entero puerto = servidor.puerto()\n",
    );
    let puerto = entero_global(&entorno, "puerto") as u16;
    let solicitud = thread::spawn(move || {
        enviar_peticion_cruda(
            puerto,
            b"POST /trozos HTTP/1.1\r\nHost: local\r\nTransfer-Encoding: chunked\r\n\r\n4\r\nhola\r\n6\r\n mundo\r\n0\r\n\r\n",
        )
    });

    vm.drenar_bucle_eventos();
    let respuesta = solicitud
        .join()
        .expect("el hilo de la petición no debe entrar en panic");
    let texto = String::from_utf8(respuesta).expect("la respuesta debe ser UTF-8");
    assert!(
        texto.ends_with("HTTP/1.1|hola mundo"),
        "respuesta inesperada: {texto}"
    );
}

#[test]
fn servidor_http_deberia_rechazar_cuerpo_declarado_antes_de_reservarlo() {
    let (mut vm, entorno) = ejecutar_con_permiso_servidor(
        "importar { ServidorHttp, PeticionHttp } desde \"quetzal/red\"\n\
         ServidorHttp servidor = nuevo ServidorHttp({ limite_cuerpo: 3 })\n\
         texto detener(PeticionHttp peticion) {\n\
         \u{20}   servidor.detener()\n\
         \u{20}   retornar \"listo\"\n\
         }\n\
         servidor.obtener(\"/detener\", detener)\n\
         servidor.escuchar(0)\n\
         entero puerto = servidor.puerto()\n",
    );
    let puerto = entero_global(&entorno, "puerto") as u16;
    let solicitud = thread::spawn(move || {
        let rechazo = enviar_peticion_cruda(
            puerto,
            b"POST /nunca HTTP/1.1\r\nHost: local\r\nContent-Length: 1000000000\r\n\r\n",
        );
        let _ = reqwest::blocking::get(format!("http://127.0.0.1:{puerto}/detener"))
            .expect("la petición que detiene el servidor debe completarse");
        rechazo
    });

    vm.drenar_bucle_eventos();
    let respuesta = solicitud
        .join()
        .expect("el hilo de la petición no debe entrar en panic");
    let texto = String::from_utf8(respuesta).expect("la respuesta debe ser UTF-8");
    assert!(
        texto.starts_with("HTTP/1.1 413 Payload Too Large"),
        "respuesta inesperada: {texto}"
    );
}

#[test]
fn servidor_http_deberia_resolver_head_405_y_opciones_automaticamente() {
    let (mut vm, entorno) = ejecutar_con_permiso_servidor(
        "importar { ServidorHttp, PeticionHttp } desde \"quetzal/red\"\n\
         ServidorHttp servidor = nuevo ServidorHttp()\n\
         texto recurso(PeticionHttp peticion) {\n\
         \u{20}   retornar \"contenido\"\n\
         }\n\
         texto detener(PeticionHttp peticion) {\n\
         \u{20}   servidor.detener()\n\
         \u{20}   retornar \"listo\"\n\
         }\n\
         servidor.obtener(\"/recurso\", recurso)\n\
         servidor.obtener(\"/detener\", detener)\n\
         servidor.escuchar(0)\n\
         entero puerto = servidor.puerto()\n",
    );
    let puerto = entero_global(&entorno, "puerto") as u16;
    let solicitud = thread::spawn(move || {
        let cliente = reqwest::blocking::Client::new();
        let cabeza = cliente
            .head(format!("http://127.0.0.1:{puerto}/recurso"))
            .send()
            .expect("HEAD debe completarse");
        let longitud_cabeza = cabeza
            .headers()
            .get("content-length")
            .and_then(|valor| valor.to_str().ok())
            .and_then(|valor| valor.parse::<u64>().ok());
        let cuerpo_cabeza = cabeza.bytes().expect("debe leer HEAD").to_vec();
        let no_permitido = cliente
            .post(format!("http://127.0.0.1:{puerto}/recurso"))
            .send()
            .expect("POST debe completarse");
        let estado_no_permitido = no_permitido.status().as_u16();
        let allow_405 = no_permitido
            .headers()
            .get("allow")
            .and_then(|valor| valor.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let opciones = cliente
            .request(
                reqwest::Method::OPTIONS,
                format!("http://127.0.0.1:{puerto}/recurso"),
            )
            .send()
            .expect("OPTIONS debe completarse");
        let estado_opciones = opciones.status().as_u16();
        let allow_opciones = opciones
            .headers()
            .get("allow")
            .and_then(|valor| valor.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let _ = cliente
            .get(format!("http://127.0.0.1:{puerto}/detener"))
            .send()
            .expect("debe detener el servidor");
        (
            longitud_cabeza,
            cuerpo_cabeza,
            estado_no_permitido,
            allow_405,
            estado_opciones,
            allow_opciones,
        )
    });

    vm.drenar_bucle_eventos();
    let (longitud, cuerpo, estado_405, allow_405, estado_opciones, allow_opciones) = solicitud
        .join()
        .expect("el hilo de la petición no debe entrar en panic");
    assert_eq!(longitud, Some(9));
    assert!(cuerpo.is_empty());
    assert_eq!(estado_405, 405);
    assert_eq!(allow_405, "GET, HEAD, OPTIONS");
    assert_eq!(estado_opciones, 204);
    assert_eq!(allow_opciones, "GET, HEAD, OPTIONS");
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
    assert!(
        puerto > 0,
        "el sistema operativo debe asignar un puerto real"
    );

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
    let cuerpo = solicitud
        .join()
        .expect("el hilo de la petición no debe entrar en panic");
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
    let respuesta = solicitud
        .join()
        .expect("el hilo de la petición no debe entrar en panic");
    assert_eq!(respuesta.status().as_u16(), 201);
    assert_eq!(
        respuesta
            .headers()
            .get("x-creado-por")
            .map(|v| v.to_str().unwrap_or("")),
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
    let cuerpo = solicitud
        .join()
        .expect("el hilo de la petición no debe entrar en panic");
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
    let (codigo_no_encontrado, cuerpo_existente) = solicitud
        .join()
        .expect("el hilo de la petición no debe entrar en panic");
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
         texto url_final = respuesta.url()\n\
         texto protocolo = respuesta.protocolo()\n\
         jsn cuerpo = respuesta.jsn()\n\
         texto metodo = cuerpo[\"metodo\"]\n\
         texto ruta = cuerpo[\"ruta\"]\n"
    ));
    assert_eq!(texto_global(&entorno, "estado"), "200");
    assert_eq!(texto_global(&entorno, "metodo"), "GET");
    assert_eq!(texto_global(&entorno, "ruta"), "/saludo");
    assert!(texto_global(&entorno, "url_final").ends_with("/saludo"));
    assert_eq!(texto_global(&entorno, "protocolo"), "HTTP/1.1");
}

#[test]
fn cliente_http_asincrono_deberia_respetar_politica_de_redireccion() {
    let puerto = iniciar_servidor(3);
    let entorno = ejecutar_con_permiso_cliente(&format!(
        "importar {{ ClienteHttp, RespuestaHttp }} desde \"quetzal/red\"\n\
         ClienteHttp sin_seguir = nuevo ClienteHttp()\n\
         sin_seguir.redirigir(0)\n\
         RespuestaHttp primera = esperar sin_seguir.obtener_asincrono(\"http://127.0.0.1:{puerto}/redirigir\")\n\
         entero estado_sin_seguir = primera.estado()\n\
         ClienteHttp siguiendo = nuevo ClienteHttp()\n\
         RespuestaHttp segunda = esperar siguiendo.obtener_asincrono(\"http://127.0.0.1:{puerto}/redirigir\")\n\
         entero estado_siguiendo = segunda.estado()\n\
         texto url_final = segunda.url()\n"
    ));
    assert_eq!(entero_global(&entorno, "estado_sin_seguir"), 302);
    assert_eq!(entero_global(&entorno, "estado_siguiendo"), 200);
    assert!(texto_global(&entorno, "url_final").ends_with("/destino"));
}

#[test]
fn cliente_http_deberia_rechazar_respuesta_mayor_al_limite() {
    let puerto = iniciar_servidor(1);
    let entorno = ejecutar_con_permiso_cliente(&format!(
        "importar {{ ClienteHttp }} desde \"quetzal/red\"\n\
         ClienteHttp cliente = nuevo ClienteHttp()\n\
         cliente.limite_respuesta(4)\n\
         texto var mensaje = \"\"\n\
         intentar {{\n\
         \u{20}   cliente.obtener(\"http://127.0.0.1:{puerto}/binario\")\n\
         }} capturar (excepcion e) {{\n\
         \u{20}   mensaje = e.mensaje\n\
         }}\n"
    ));
    assert!(texto_global(&entorno, "mensaje").contains("excede el límite de 4 bytes"));
}

#[test]
fn cliente_http_deberia_conservar_cabeceras_repetidas() {
    let puerto = iniciar_servidor(1);
    let entorno = ejecutar_con_permiso_cliente(&format!(
        "importar {{ ClienteHttp, RespuestaHttp }} desde \"quetzal/red\"\n\
         ClienteHttp cliente = nuevo ClienteHttp()\n\
         RespuestaHttp respuesta = cliente.obtener(\"http://127.0.0.1:{puerto}/galletas\")\n\
         lista<texto> galletas = respuesta.cabeceras_todas(\"Set-Cookie\")\n\
         entero cantidad = galletas.longitud()\n\
         texto primera = galletas[0]\n\
         texto ultima = respuesta.cabecera(\"Set-Cookie\")\n"
    ));
    assert_eq!(entero_global(&entorno, "cantidad"), 2);
    assert_eq!(texto_global(&entorno, "primera"), "a=1");
    assert_eq!(texto_global(&entorno, "ultima"), "b=2");
}

#[test]
fn servidor_http_deberia_enviar_multiples_galletas_sin_sobrescribir() {
    let (mut vm, entorno) = ejecutar_con_permiso_servidor(
        "importar { ServidorHttp, PeticionHttp, RespuestaServidor, Respuestas } desde \"quetzal/red\"\n\
         ServidorHttp servidor = nuevo ServidorHttp()\n\
         RespuestaServidor manejar(PeticionHttp peticion) {\n\
         \u{20}   servidor.detener()\n\
         \u{20}   RespuestaServidor respuesta = Respuestas.texto(\"galletas\")\n\
         \u{20}   respuesta.fijar_galleta(\"a\", \"1\")\n\
         \u{20}   respuesta.fijar_galleta(\"b\", \"2\")\n\
         \u{20}   retornar respuesta\n\
         }\n\
         servidor.obtener(\"/galletas\", manejar)\n\
         servidor.escuchar(0)\n\
         entero puerto = servidor.puerto()\n",
    );
    let puerto = entero_global(&entorno, "puerto") as u16;
    let solicitud = thread::spawn(move || {
        reqwest::blocking::get(format!("http://127.0.0.1:{puerto}/galletas"))
            .expect("la petición debe completarse")
            .headers()
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .map(|valor| valor.to_str().unwrap_or_default().to_string())
            .collect::<Vec<_>>()
    });

    vm.drenar_bucle_eventos();
    let galletas = solicitud
        .join()
        .expect("el hilo de la petición no debe entrar en panic");
    assert_eq!(galletas, vec!["a=1", "b=2"]);
}

#[test]
fn servidor_http_deberia_conservar_cabeceras_y_consultas_repetidas() {
    let (mut vm, entorno) = ejecutar_con_permiso_servidor(
        "importar { ServidorHttp, PeticionHttp } desde \"quetzal/red\"\n\
         ServidorHttp servidor = nuevo ServidorHttp()\n\
         texto manejar(PeticionHttp peticion) {\n\
         \u{20}   servidor.detener()\n\
         \u{20}   lista<texto> cabeceras = peticion.cabeceras_todas(\"X-Repetida\")\n\
         \u{20}   lista<texto> consultas = peticion.consulta_todas(\"a\")\n\
         \u{20}   retornar t\"{cabeceras.longitud()}|{cabeceras[0]}|{cabeceras[1]}|{consultas.longitud()}|{consultas[0]}|{consultas[1]}\"\n\
         }\n\
         servidor.obtener(\"/repetidos\", manejar)\n\
         servidor.escuchar(0)\n\
         entero puerto = servidor.puerto()\n",
    );
    let puerto = entero_global(&entorno, "puerto") as u16;
    let solicitud = thread::spawn(move || {
        enviar_peticion_cruda(
            puerto,
            b"GET /repetidos?a=1&a=2 HTTP/1.1\r\nHost: local\r\nX-Repetida: uno\r\nX-Repetida: dos\r\n\r\n",
        )
    });

    vm.drenar_bucle_eventos();
    let respuesta = solicitud
        .join()
        .expect("el hilo de la petición no debe entrar en panic");
    let texto = String::from_utf8(respuesta).expect("la respuesta debe ser UTF-8");
    assert!(
        texto.ends_with("2|uno|dos|2|1|2"),
        "respuesta inesperada: {texto}"
    );
}

#[test]
fn servidor_http_deberia_aislar_middlewares_en_peticiones_reentrantes() {
    let (mut vm, entorno) = ejecutar_con_permiso_servidor(
        "importar { ServidorHttp, PeticionHttp, RespuestaServidor, Respuestas, ClienteHttp, RespuestaHttp } desde \"quetzal/red\"\n\
         objeto Interceptores {\n\
         \u{20}   publico:\n\
         \u{20}   libre asincrono RespuestaServidor pasar(PeticionHttp peticion, RespuestaServidor respuesta, funcion siguiente) {\n\
         \u{20}       retornar esperar siguiente(peticion, respuesta)\n\
         \u{20}   }\n\
         }\n\
         ServidorHttp servidor = nuevo ServidorHttp()\n\
         ClienteHttp cliente = nuevo ClienteHttp()\n\
         entero var puerto = 0\n\
         RespuestaServidor interno(PeticionHttp peticion) {\n\
         \u{20}   retornar Respuestas.texto(\"interno\")\n\
         }\n\
         asincrono RespuestaServidor externo(PeticionHttp peticion) {\n\
         \u{20}   RespuestaHttp respuesta = esperar cliente.obtener_asincrono(t\"http://127.0.0.1:{puerto}/interno\")\n\
         \u{20}   servidor.detener()\n\
         \u{20}   retornar Respuestas.texto(t\"externo:{respuesta.texto()}\")\n\
         }\n\
         servidor.usar(Interceptores.pasar)\n\
         servidor.obtener(\"/interno\", interno)\n\
         servidor.obtener(\"/externo\", externo)\n\
         servidor.escuchar(0)\n\
         puerto = servidor.puerto()\n",
    );
    let puerto = entero_global(&entorno, "puerto") as u16;
    let solicitud = thread::spawn(move || {
        reqwest::blocking::get(format!("http://127.0.0.1:{puerto}/externo"))
            .expect("la petición reentrante debe completarse")
            .text()
            .expect("la respuesta debe ser texto")
    });

    vm.drenar_bucle_eventos();
    let cuerpo = solicitud
        .join()
        .expect("el hilo de la petición no debe entrar en panic");
    assert_eq!(cuerpo, "externo:interno");
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
fn cliente_http_deberia_enviar_archivo_por_flujo_sincrono_y_asincrono() {
    let puerto = iniciar_servidor(2);
    let directorio = std::path::Path::new("target").join("pruebas_red_http");
    std::fs::create_dir_all(&directorio).expect("debe crear el directorio temporal");
    let ruta = directorio.join(format!("subida_{}.txt", std::process::id()));
    std::fs::write(&ruta, b"contenido transmitido por flujo")
        .expect("debe crear el archivo de subida");
    let ruta_quetzal = ruta.to_string_lossy().replace('\\', "/");

    let entorno = ejecutar_con_permiso_cliente(&format!(
        "importar {{ ClienteHttp, RespuestaHttp }} desde \"quetzal/red\"\n\
         ClienteHttp cliente = nuevo ClienteHttp()\n\
         jsn sincronica = cliente.enviar_archivo(\"http://127.0.0.1:{puerto}/subida\", \"{ruta_quetzal}\").jsn()\n\
         RespuestaHttp respuesta_asincrona = esperar cliente.enviar_archivo_asincrono(\"http://127.0.0.1:{puerto}/subida\", \"{ruta_quetzal}\")\n\
         jsn asincronica = respuesta_asincrona.jsn()\n\
         texto cuerpo_sincrono = sincronica[\"cuerpo\"]\n\
         texto cuerpo_asincrono = asincronica[\"cuerpo\"]\n"
    ));
    assert_eq!(
        texto_global(&entorno, "cuerpo_sincrono"),
        "contenido transmitido por flujo"
    );
    assert_eq!(
        texto_global(&entorno, "cuerpo_asincrono"),
        "contenido transmitido por flujo"
    );
    let _ = std::fs::remove_file(ruta);
}

#[test]
fn cliente_http_deberia_descargar_por_flujo_sincrono_y_asincrono() {
    let puerto = iniciar_servidor(2);
    let directorio = std::path::Path::new("target").join("pruebas_red_http");
    std::fs::create_dir_all(&directorio).expect("debe crear el directorio temporal");
    let ruta_sincrona = directorio.join(format!("descarga_sincrona_{}.bin", std::process::id()));
    let ruta_asincrona = directorio.join(format!("descarga_asincrona_{}.bin", std::process::id()));
    let ruta_sincrona_qz = ruta_sincrona.to_string_lossy().replace('\\', "/");
    let ruta_asincrona_qz = ruta_asincrona.to_string_lossy().replace('\\', "/");

    let entorno = ejecutar_con_permiso_cliente(&format!(
        "importar {{ ClienteHttp, RespuestaHttp }} desde \"quetzal/red\"\n\
         ClienteHttp cliente = nuevo ClienteHttp()\n\
         RespuestaHttp sincronica = cliente.descargar(\"http://127.0.0.1:{puerto}/binario\", \"{ruta_sincrona_qz}\")\n\
         RespuestaHttp asincronica = esperar cliente.descargar_asincrono(\"http://127.0.0.1:{puerto}/binario\", \"{ruta_asincrona_qz}\")\n\
         entero peso_sincrono = sincronica.peso()\n\
         entero peso_asincrono = asincronica.peso()\n"
    ));
    assert_eq!(entero_global(&entorno, "peso_sincrono"), 5);
    assert_eq!(entero_global(&entorno, "peso_asincrono"), 5);
    assert_eq!(std::fs::read(&ruta_sincrona).unwrap(), BYTES_BINARIOS);
    assert_eq!(std::fs::read(&ruta_asincrona).unwrap(), BYTES_BINARIOS);
    let _ = std::fs::remove_file(ruta_sincrona);
    let _ = std::fs::remove_file(ruta_asincrona);
}

#[test]
fn cliente_http_descarga_fallida_deberia_conservar_archivo_anterior() {
    let puerto = iniciar_servidor(1);
    let directorio = std::path::Path::new("target").join("pruebas_red_http");
    std::fs::create_dir_all(&directorio).expect("debe crear el directorio temporal");
    let ruta = directorio.join(format!("destino_existente_{}.bin", std::process::id()));
    std::fs::write(&ruta, b"anterior").expect("debe crear el destino anterior");
    let ruta_qz = ruta.to_string_lossy().replace('\\', "/");

    let entorno = ejecutar_con_permiso_cliente(&format!(
        "importar {{ ClienteHttp }} desde \"quetzal/red\"\n\
         ClienteHttp cliente = nuevo ClienteHttp()\n\
         cliente.limite_respuesta(4)\n\
         texto var mensaje = \"\"\n\
         intentar {{\n\
         \u{20}   cliente.descargar(\"http://127.0.0.1:{puerto}/binario\", \"{ruta_qz}\")\n\
         }} capturar (excepcion e) {{\n\
         \u{20}   mensaje = e.mensaje\n\
         }}\n"
    ));
    assert!(texto_global(&entorno, "mensaje").contains("excede el límite"));
    assert_eq!(std::fs::read(&ruta).unwrap(), b"anterior");
    let _ = std::fs::remove_file(ruta);
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
