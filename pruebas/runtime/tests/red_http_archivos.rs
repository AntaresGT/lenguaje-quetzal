//! Pruebas de streaming de archivos y respuestas por trozos en
//! `ServidorHttp` (`Respuestas.archivo`, `ServidorHttp.estaticos`,
//! `Respuestas.flujo`, `PeticionHttp.cabecera`).
//!
//! Usa archivos temporales reales (bajo `std::env::temp_dir()`) y un
//! `ServidorHttp` de Quetzal real en un puerto efímero, igual que
//! `red_http.rs`: las peticiones se disparan con `reqwest` desde un hilo
//! aparte mientras se bombea el bucle de eventos de la VM
//! (`Vm::drenar_bucle_eventos`).

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::thread;
use std::time::Duration;

use indexmap::IndexMap;
use maquina_virtual::{Valor, Vm};
use nucleo::Fuente;

// =====================================================================
// Archivos temporales y ejecución con permisos de red + sistema_archivos
// =====================================================================

/// Directorio temporal exclusivo de esta prueba (se limpia al llamar de
/// nuevo con el mismo nombre, para que las corridas repetidas no arrastren
/// archivos de una ejecución anterior).
fn directorio_temporal(nombre: &str) -> PathBuf {
    let ruta = std::env::temp_dir().join(format!(
        "quetzal_red_http_archivos_{nombre}_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&ruta);
    std::fs::create_dir_all(&ruta).expect("se puede crear el directorio temporal");
    ruta
}

/// Ruta absoluta con `/` como separador (evita que `\t`/`\n` de una ruta
/// de Windows con `\` se interpreten como escapes dentro del código fuente
/// de Quetzal que arma cada prueba).
fn ruta_quetzal(ruta: &Path) -> String {
    ruta.to_string_lossy().replace('\\', "/")
}

fn escribir_archivo(directorio: &Path, nombre: &str, contenido: &[u8]) -> PathBuf {
    let ruta = directorio.join(nombre);
    std::fs::write(&ruta, contenido).expect("se puede escribir el archivo de prueba");
    ruta
}

/// Ejecuta código de Quetzal con permiso de servidor HTTP (sin puertos
/// restringidos) y de lectura de `sistema_archivos` sobre `directorios`
/// (rutas absolutas). Devuelve la VM (para seguir bombeando el bucle de
/// eventos) y el entorno del módulo principal.
fn ejecutar_con_permiso_servidor_y_lectura(
    codigo: &str,
    directorios: &[&Path],
) -> (Vm, Rc<maquina_virtual::valores::EntornoModulo>) {
    let fuente = Fuente::nueva("prueba.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("el código de prueba debe parsear");
    semantica::analizar_modulo(&ast).expect("el código de prueba debe pasar la semántica");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar bytecode");

    let directorios_json: Vec<serde_json::Value> = directorios
        .iter()
        .map(|ruta| serde_json::json!({"ruta": ruta_quetzal(ruta), "permiso": "lectura"}))
        .collect();
    let permisos_json = serde_json::json!({
        "red": { "habilitado": true, "servidor": true },
        "sistema_archivos": { "habilitado": true, "directorios": directorios_json },
    });
    let permisos =
        paquetes::Permisos::desde_json(&permisos_json).expect("permisos de prueba válidos");
    let guardian = Rc::new(runtime::GuardianPermisos::denegado());
    guardian.configurar(permisos, Path::new("."));
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

// =====================================================================
// Cliente HTTP crudo: para probar cabeceras/rutas que un cliente normal
// normalizaría (p. ej. un `..` en la ruta) antes de enviarlas.
// =====================================================================

/// Envía `GET {ruta}` tal cual (sin normalizar) y devuelve `(estado,
/// cabeceras, cuerpo)`. No sigue las reglas de un cliente HTTP real: solo
/// lo suficiente para inspeccionar la respuesta cruda del servidor.
fn peticion_cruda(puerto: u16, metodo: &str, ruta: &str, cabeceras_extra: &[(&str, &str)]) -> (u16, Vec<(String, String)>, Vec<u8>) {
    let mut flujo = TcpStream::connect(("127.0.0.1", puerto)).expect("conectar al servidor de prueba");
    let mut peticion = format!("{metodo} {ruta} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n");
    for (nombre, valor) in cabeceras_extra {
        peticion.push_str(&format!("{nombre}: {valor}\r\n"));
    }
    peticion.push_str("\r\n");
    flujo.write_all(peticion.as_bytes()).expect("enviar la petición cruda");

    let mut crudo = Vec::new();
    flujo.read_to_end(&mut crudo).expect("leer la respuesta cruda");
    let texto = String::from_utf8_lossy(&crudo).into_owned();
    let (cabecera_bloque, cuerpo_texto) = texto.split_once("\r\n\r\n").unwrap_or((texto.as_str(), ""));
    let mut lineas = cabecera_bloque.split("\r\n");
    let estado = lineas
        .next()
        .and_then(|linea| linea.split_whitespace().nth(1))
        .and_then(|codigo| codigo.parse().ok())
        .unwrap_or(0);
    let cabeceras = lineas
        .filter_map(|linea| linea.split_once(':'))
        .map(|(nombre, valor)| (nombre.trim().to_string(), valor.trim().to_string()))
        .collect();
    let inicio_cuerpo = cabecera_bloque.len() + 4;
    let cuerpo = crudo.get(inicio_cuerpo..).unwrap_or(&[]).to_vec();
    let _ = cuerpo_texto;
    (estado, cabeceras, cuerpo)
}

fn valor_cabecera(cabeceras: &[(String, String)], nombre: &str) -> Option<String> {
    cabeceras
        .iter()
        .find(|(clave, _)| clave.eq_ignore_ascii_case(nombre))
        .map(|(_, valor)| valor.clone())
}

// =====================================================================
// Respuestas.archivo: 200 completo, Range/206, 416, If-None-Match/304
// =====================================================================

#[test]
fn respuestas_archivo_deberia_servir_completo_con_mime_correcto() {
    let raiz = directorio_temporal("archivo_200");
    let ruta_archivo = escribir_archivo(&raiz, "saludo.txt", b"hola desde un archivo");
    let ruta_qz = ruta_quetzal(&ruta_archivo);

    let (mut vm, entorno) = ejecutar_con_permiso_servidor_y_lectura(
        &format!(
            "importar {{ ServidorHttp, PeticionHttp, RespuestaServidor, Respuestas }} desde \"quetzal/red\"\n\
             ServidorHttp servidor = nuevo ServidorHttp()\n\
             RespuestaServidor manejador(PeticionHttp peticion) {{\n\
             \u{20}   servidor.detener()\n\
             \u{20}   retornar Respuestas.archivo(\"{ruta_qz}\")\n\
             }}\n\
             servidor.obtener(\"/archivo\", manejador)\n\
             servidor.escuchar(0)\n\
             entero puerto = servidor.puerto()\n"
        ),
        &[&raiz],
    );
    let puerto = entero_global(&entorno, "puerto") as u16;

    let solicitud = thread::spawn(move || {
        let cliente = reqwest::blocking::Client::new();
        let respuesta = cliente
            .get(format!("http://127.0.0.1:{puerto}/archivo"))
            .timeout(Duration::from_secs(5))
            .send()
            .expect("la petición debe completarse");
        let estado = respuesta.status().as_u16();
        let tipo_contenido = respuesta
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let etag = respuesta
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let acepta_rangos = respuesta
            .headers()
            .get("accept-ranges")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let cuerpo = respuesta.text().expect("debe leerse el cuerpo");
        (estado, tipo_contenido, etag, acepta_rangos, cuerpo)
    });

    vm.drenar_bucle_eventos();
    let (estado, tipo_contenido, etag, acepta_rangos, cuerpo) =
        solicitud.join().expect("el hilo de la petición no debe entrar en panic");
    assert_eq!(estado, 200);
    assert!(tipo_contenido.starts_with("text/plain"), "tipo de contenido: {tipo_contenido}");
    assert!(!etag.is_empty(), "debe incluir un ETag");
    assert_eq!(acepta_rangos, "bytes");
    assert_eq!(cuerpo, "hola desde un archivo");
    let _ = std::fs::remove_dir_all(&raiz);
}

#[test]
fn respuestas_archivo_deberia_soportar_range_valido_con_206() {
    let raiz = directorio_temporal("archivo_range");
    let ruta_archivo = escribir_archivo(&raiz, "video.bin", b"0123456789");
    let ruta_qz = ruta_quetzal(&ruta_archivo);

    let (mut vm, entorno) = ejecutar_con_permiso_servidor_y_lectura(
        &format!(
            "importar {{ ServidorHttp, PeticionHttp, RespuestaServidor, Respuestas }} desde \"quetzal/red\"\n\
             ServidorHttp servidor = nuevo ServidorHttp()\n\
             RespuestaServidor manejador(PeticionHttp peticion) {{\n\
             \u{20}   servidor.detener()\n\
             \u{20}   retornar Respuestas.archivo(\"{ruta_qz}\")\n\
             }}\n\
             servidor.obtener(\"/video\", manejador)\n\
             servidor.escuchar(0)\n\
             entero puerto = servidor.puerto()\n"
        ),
        &[&raiz],
    );
    let puerto = entero_global(&entorno, "puerto") as u16;

    let solicitud = thread::spawn(move || {
        let cliente = reqwest::blocking::Client::new();
        let respuesta = cliente
            .get(format!("http://127.0.0.1:{puerto}/video"))
            .header("Range", "bytes=2-5")
            .timeout(Duration::from_secs(5))
            .send()
            .expect("la petición con Range debe completarse");
        let estado = respuesta.status().as_u16();
        let content_range = respuesta
            .headers()
            .get("content-range")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let cuerpo = respuesta.text().expect("debe leerse el cuerpo parcial");
        (estado, content_range, cuerpo)
    });

    vm.drenar_bucle_eventos();
    let (estado, content_range, cuerpo) =
        solicitud.join().expect("el hilo de la petición no debe entrar en panic");
    assert_eq!(estado, 206);
    assert_eq!(content_range, "bytes 2-5/10");
    assert_eq!(cuerpo, "2345");
    let _ = std::fs::remove_dir_all(&raiz);
}

#[test]
fn respuestas_archivo_deberia_responder_416_con_rango_invalido() {
    let raiz = directorio_temporal("archivo_416");
    let ruta_archivo = escribir_archivo(&raiz, "corto.bin", b"12345");
    let ruta_qz = ruta_quetzal(&ruta_archivo);

    let (mut vm, entorno) = ejecutar_con_permiso_servidor_y_lectura(
        &format!(
            "importar {{ ServidorHttp, PeticionHttp, RespuestaServidor, Respuestas }} desde \"quetzal/red\"\n\
             ServidorHttp servidor = nuevo ServidorHttp()\n\
             RespuestaServidor manejador(PeticionHttp peticion) {{\n\
             \u{20}   servidor.detener()\n\
             \u{20}   retornar Respuestas.archivo(\"{ruta_qz}\")\n\
             }}\n\
             servidor.obtener(\"/corto\", manejador)\n\
             servidor.escuchar(0)\n\
             entero puerto = servidor.puerto()\n"
        ),
        &[&raiz],
    );
    let puerto = entero_global(&entorno, "puerto") as u16;

    let solicitud = thread::spawn(move || {
        let cliente = reqwest::blocking::Client::new();
        let respuesta = cliente
            .get(format!("http://127.0.0.1:{puerto}/corto"))
            .header("Range", "bytes=100-200")
            .timeout(Duration::from_secs(5))
            .send()
            .expect("la petición con rango inválido debe completarse");
        let estado = respuesta.status().as_u16();
        let content_range = respuesta
            .headers()
            .get("content-range")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        (estado, content_range)
    });

    vm.drenar_bucle_eventos();
    let (estado, content_range) = solicitud.join().expect("el hilo de la petición no debe entrar en panic");
    assert_eq!(estado, 416);
    assert_eq!(content_range, "bytes */5");
    let _ = std::fs::remove_dir_all(&raiz);
}

#[test]
fn respuestas_archivo_deberia_responder_304_con_if_none_match() {
    let raiz = directorio_temporal("archivo_304");
    let ruta_archivo = escribir_archivo(&raiz, "cacheable.txt", b"contenido cacheable");
    let ruta_qz = ruta_quetzal(&ruta_archivo);

    let (mut vm, entorno) = ejecutar_con_permiso_servidor_y_lectura(
        &format!(
            "importar {{ ServidorHttp, PeticionHttp, RespuestaServidor, Respuestas }} desde \"quetzal/red\"\n\
             ServidorHttp servidor = nuevo ServidorHttp()\n\
             entero var peticiones = 0\n\
             RespuestaServidor manejador(PeticionHttp peticion) {{\n\
             \u{20}   peticiones = peticiones + 1\n\
             \u{20}   si (peticiones >= 2) {{\n\
             \u{20}       servidor.detener()\n\
             \u{20}   }}\n\
             \u{20}   retornar Respuestas.archivo(\"{ruta_qz}\")\n\
             }}\n\
             servidor.obtener(\"/cacheable\", manejador)\n\
             servidor.escuchar(0)\n\
             entero puerto = servidor.puerto()\n"
        ),
        &[&raiz],
    );
    let puerto = entero_global(&entorno, "puerto") as u16;

    let solicitud = thread::spawn(move || {
        let cliente = reqwest::blocking::Client::new();
        let primera = cliente
            .get(format!("http://127.0.0.1:{puerto}/cacheable"))
            .timeout(Duration::from_secs(5))
            .send()
            .expect("la primera petición debe completarse");
        let etag = primera
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let segunda = cliente
            .get(format!("http://127.0.0.1:{puerto}/cacheable"))
            .header("If-None-Match", etag)
            .timeout(Duration::from_secs(5))
            .send()
            .expect("la segunda petición debe completarse");
        let estado_segunda = segunda.status().as_u16();
        let cuerpo_segunda = segunda.bytes().expect("debe leerse el cuerpo (vacío)");
        (estado_segunda, cuerpo_segunda.len())
    });

    vm.drenar_bucle_eventos();
    let (estado_segunda, longitud_cuerpo_segunda) =
        solicitud.join().expect("el hilo de la petición no debe entrar en panic");
    assert_eq!(estado_segunda, 304);
    assert_eq!(longitud_cuerpo_segunda, 0);
    let _ = std::fs::remove_dir_all(&raiz);
}

#[test]
fn respuestas_archivo_sin_permiso_de_lectura_deberia_fallar_con_e0701() {
    let raiz = directorio_temporal("archivo_sin_permiso");
    let ruta_archivo = escribir_archivo(&raiz, "prohibido.txt", b"no deberias leer esto");
    let ruta_qz = ruta_quetzal(&ruta_archivo);

    let fuente = Fuente::nueva(
        "prueba.qz",
        format!(
            "importar {{ Respuestas }} desde \"quetzal/red\"\n\
             Respuestas.archivo(\"{ruta_qz}\")\n"
        ),
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
        .expect_err("sin permiso de lectura, Respuestas.archivo debe fallar");
    assert_eq!(error.codigo, "E0701");
    let _ = std::fs::remove_dir_all(&raiz);
}

// =====================================================================
// ServidorHttp.estaticos: servir carpetas, 404 fuera/traversal, prioridad
// =====================================================================

#[test]
fn estaticos_deberia_servir_archivo_bajo_el_directorio_y_dar_prioridad_a_rutas_explicitas() {
    let raiz = directorio_temporal("estaticos");
    let publico = raiz.join("publico");
    std::fs::create_dir_all(&publico).expect("crear la carpeta publico");
    escribir_archivo(&publico, "normal.txt", b"contenido estatico");
    escribir_archivo(&publico, "especial.txt", b"archivo real (no deberia verse)");
    let publico_qz = ruta_quetzal(&publico);

    let (mut vm, entorno) = ejecutar_con_permiso_servidor_y_lectura(
        &format!(
            "importar {{ ServidorHttp, PeticionHttp }} desde \"quetzal/red\"\n\
             ServidorHttp servidor = nuevo ServidorHttp()\n\
             texto manejador_especial(PeticionHttp peticion) {{\n\
             \u{20}   servidor.detener()\n\
             \u{20}   retornar \"manejador explicito\"\n\
             }}\n\
             servidor.obtener(\"/publico/especial.txt\", manejador_especial)\n\
             servidor.estaticos(\"/publico\", \"{publico_qz}\")\n\
             servidor.escuchar(0)\n\
             entero puerto = servidor.puerto()\n"
        ),
        &[&publico],
    );
    let puerto = entero_global(&entorno, "puerto") as u16;

    let solicitud = thread::spawn(move || {
        let cliente = reqwest::blocking::Client::new();
        let normal = cliente
            .get(format!("http://127.0.0.1:{puerto}/publico/normal.txt"))
            .timeout(Duration::from_secs(5))
            .send()
            .expect("debe servirse el archivo estático");
        let estado_normal = normal.status().as_u16();
        let cuerpo_normal = normal.text().expect("debe leerse el cuerpo");

        let especial = cliente
            .get(format!("http://127.0.0.1:{puerto}/publico/especial.txt"))
            .timeout(Duration::from_secs(5))
            .send()
            .expect("debe responder la ruta explícita");
        let cuerpo_especial = especial.text().expect("debe leerse el cuerpo");

        (estado_normal, cuerpo_normal, cuerpo_especial)
    });

    vm.drenar_bucle_eventos();
    let (estado_normal, cuerpo_normal, cuerpo_especial) =
        solicitud.join().expect("el hilo de la petición no debe entrar en panic");
    assert_eq!(estado_normal, 200);
    assert_eq!(cuerpo_normal, "contenido estatico");
    // La ruta explícita gana aunque también exista un archivo real con ese nombre.
    assert_eq!(cuerpo_especial, "manejador explicito");
    let _ = std::fs::remove_dir_all(&raiz);
}

#[test]
fn estaticos_deberia_responder_404_fuera_del_directorio_y_con_traversal() {
    let raiz = directorio_temporal("estaticos_traversal");
    let publico = raiz.join("publico");
    std::fs::create_dir_all(&publico).expect("crear la carpeta publico");
    escribir_archivo(&raiz, "fuera.txt", b"secreto fuera de publico");
    let publico_qz = ruta_quetzal(&publico);

    let (mut vm, entorno) = ejecutar_con_permiso_servidor_y_lectura(
        &format!(
            "importar {{ ServidorHttp, PeticionHttp }} desde \"quetzal/red\"\n\
             ServidorHttp servidor = nuevo ServidorHttp()\n\
             texto manejador_apagar(PeticionHttp peticion) {{\n\
             \u{20}   servidor.detener()\n\
             \u{20}   retornar \"apagando\"\n\
             }}\n\
             servidor.obtener(\"/apagar\", manejador_apagar)\n\
             servidor.estaticos(\"/publico\", \"{publico_qz}\")\n\
             servidor.escuchar(0)\n\
             entero puerto = servidor.puerto()\n"
        ),
        // Solo se declara permiso de lectura sobre "publico": escapar de ahí
        // (con "..") debe quedar fuera de lo permitido y responder 404.
        &[&publico],
    );
    let puerto = entero_global(&entorno, "puerto") as u16;

    let solicitud = thread::spawn(move || {
        // Petición cruda (no normalizada por un cliente HTTP) con `..`
        // literal en la ruta: intenta escapar de "publico" hacia "fuera.txt".
        let (estado_traversal, _, _) = peticion_cruda(puerto, "GET", "/publico/../fuera.txt", &[]);
        let (estado_inexistente, _, _) = peticion_cruda(puerto, "GET", "/publico/no-existe.txt", &[]);
        let (estado_apagar, _, cuerpo_apagar) = peticion_cruda(puerto, "GET", "/apagar", &[]);
        (estado_traversal, estado_inexistente, estado_apagar, cuerpo_apagar)
    });

    vm.drenar_bucle_eventos();
    let (estado_traversal, estado_inexistente, estado_apagar, cuerpo_apagar) =
        solicitud.join().expect("el hilo de la petición no debe entrar en panic");
    assert_eq!(estado_traversal, 404, "escapar del directorio estático debe dar 404");
    assert_eq!(estado_inexistente, 404);
    assert_eq!(estado_apagar, 200);
    assert_eq!(String::from_utf8_lossy(&cuerpo_apagar), "apagando");
    let _ = std::fs::remove_dir_all(&raiz);
}

// =====================================================================
// Respuestas.flujo: chunked reconstruido == concatenación de trozos
// =====================================================================

#[test]
fn respuestas_flujo_deberia_reconstruir_el_cuerpo_chunked_completo() {
    let (mut vm, entorno) = ejecutar_con_permiso_servidor_y_lectura(
        "importar { ServidorHttp, PeticionHttp, RespuestaServidor, Respuestas } desde \"quetzal/red\"\n\
         ServidorHttp servidor = nuevo ServidorHttp()\n\
         entero var contador = 0\n\
         texto siguiente_trozo() {\n\
         \u{20}   contador = contador + 1\n\
         \u{20}   si (contador > 3) {\n\
         \u{20}       servidor.detener()\n\
         \u{20}       retornar nulo\n\
         \u{20}   }\n\
         \u{20}   retornar \"trozo\" + contador + \";\"\n\
         }\n\
         RespuestaServidor manejador(PeticionHttp peticion) {\n\
         \u{20}   retornar Respuestas.flujo(siguiente_trozo)\n\
         }\n\
         servidor.obtener(\"/flujo\", manejador)\n\
         servidor.escuchar(0)\n\
         entero puerto = servidor.puerto()\n",
        &[],
    );
    let puerto = entero_global(&entorno, "puerto") as u16;

    let solicitud = thread::spawn(move || {
        let cliente = reqwest::blocking::Client::new();
        let respuesta = cliente
            .get(format!("http://127.0.0.1:{puerto}/flujo"))
            .timeout(Duration::from_secs(5))
            .send()
            .expect("la petición al flujo debe completarse");
        let estado = respuesta.status().as_u16();
        let cuerpo = respuesta.text().expect("debe reconstruirse el cuerpo chunked");
        (estado, cuerpo)
    });

    vm.drenar_bucle_eventos();
    let (estado, cuerpo) = solicitud.join().expect("el hilo de la petición no debe entrar en panic");
    assert_eq!(estado, 200);
    assert_eq!(cuerpo, "trozo1;trozo2;trozo3;");
}

#[test]
fn respuestas_flujo_deberia_permitir_content_type_de_eventos_del_servidor() {
    let (mut vm, entorno) = ejecutar_con_permiso_servidor_y_lectura(
        "importar { ServidorHttp, PeticionHttp, RespuestaServidor, Respuestas } desde \"quetzal/red\"\n\
         ServidorHttp servidor = nuevo ServidorHttp()\n\
         entero var contador = 0\n\
         texto siguiente_evento() {\n\
         \u{20}   contador = contador + 1\n\
         \u{20}   si (contador > 2) {\n\
         \u{20}       servidor.detener()\n\
         \u{20}       retornar nulo\n\
         \u{20}   }\n\
         \u{20}   retornar \"data: \" + contador + \"\\n\\n\"\n\
         }\n\
         RespuestaServidor manejador(PeticionHttp peticion) {\n\
         \u{20}   RespuestaServidor respuesta = Respuestas.flujo(siguiente_evento)\n\
         \u{20}   respuesta.fijar_cabecera(\"Content-Type\", \"text/event-stream\")\n\
         \u{20}   retornar respuesta\n\
         }\n\
         servidor.obtener(\"/eventos\", manejador)\n\
         servidor.escuchar(0)\n\
         entero puerto = servidor.puerto()\n",
        &[],
    );
    let puerto = entero_global(&entorno, "puerto") as u16;

    let solicitud = thread::spawn(move || {
        let cliente = reqwest::blocking::Client::new();
        let respuesta = cliente
            .get(format!("http://127.0.0.1:{puerto}/eventos"))
            .timeout(Duration::from_secs(5))
            .send()
            .expect("la petición SSE debe completarse");
        let tipo_contenido = respuesta
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let cuerpo = respuesta.text().expect("debe leerse el cuerpo SSE");
        (tipo_contenido, cuerpo)
    });

    vm.drenar_bucle_eventos();
    let (tipo_contenido, cuerpo) = solicitud.join().expect("el hilo de la petición no debe entrar en panic");
    assert_eq!(tipo_contenido, "text/event-stream");
    assert_eq!(cuerpo, "data: 1\n\ndata: 2\n\n");
}

// =====================================================================
// PeticionHttp.cabecera: búsqueda insensible a mayúsculas
// =====================================================================

#[test]
fn peticion_http_cabecera_deberia_buscar_sin_distinguir_mayusculas() {
    let (mut vm, entorno) = ejecutar_con_permiso_servidor_y_lectura(
        "importar { ServidorHttp, PeticionHttp } desde \"quetzal/red\"\n\
         ServidorHttp servidor = nuevo ServidorHttp()\n\
         texto manejador(PeticionHttp peticion) {\n\
         \u{20}   servidor.detener()\n\
         \u{20}   texto valor = peticion.cabecera(\"x-prueba-mayus\")\n\
         \u{20}   texto ausente = peticion.cabecera(\"x-no-existe\")\n\
         \u{20}   si (ausente == nulo) {\n\
         \u{20}       retornar valor\n\
         \u{20}   }\n\
         \u{20}   retornar \"deberia-ser-nulo\"\n\
         }\n\
         servidor.obtener(\"/cabecera\", manejador)\n\
         servidor.escuchar(0)\n\
         entero puerto = servidor.puerto()\n",
        &[],
    );
    let puerto = entero_global(&entorno, "puerto") as u16;

    let solicitud = thread::spawn(move || {
        let cliente = reqwest::blocking::Client::new();
        cliente
            .get(format!("http://127.0.0.1:{puerto}/cabecera"))
            .header("X-Prueba-Mayus", "valor-encontrado")
            .timeout(Duration::from_secs(5))
            .send()
            .expect("la petición debe completarse")
            .text()
            .expect("debe leerse el cuerpo")
    });

    vm.drenar_bucle_eventos();
    let cuerpo = solicitud.join().expect("el hilo de la petición no debe entrar en panic");
    assert_eq!(cuerpo, "valor-encontrado");
    let _ = valor_cabecera(&[], "");
}
