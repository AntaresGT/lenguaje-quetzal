//! Módulo nativo `quetzal/red`: `ServidorHttp`, `Enrutador`, `ClienteHttp`,
//! `HttpCodigos` y el permiso `red`.
//!
//! Cubre los criterios de aceptación de la especificación: importación de los
//! objetos documentados, enrutado estilo Express 5 con interceptores y
//! funciones nombradas, el método QUERY del RFC 10008 en servidor y cliente,
//! el cliente estilo Axios con interceptores y progreso, las descripciones en
//! español de `HttpCodigos` y el modelo de permisos de `quetzal.json`.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

use indexmap::IndexMap;
use maquina_virtual::{Valor, Vm};
use nucleo::Fuente;
use paquetes::Permisos;
use runtime::GuardianPermisos;

/// Cada prueba trabaja en su propia raíz temporal, igual que un proyecto.
fn raiz_temporal(nombre: &str) -> PathBuf {
    static CONTADOR: AtomicU32 = AtomicU32::new(0);
    let unico = CONTADOR.fetch_add(1, Ordering::Relaxed);
    let ruta = std::env::temp_dir().join(format!(
        "quetzal_red_{nombre}_{}_{unico}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&ruta);
    std::fs::create_dir_all(&ruta).expect("se puede crear la raíz temporal");
    ruta
}

/// Permiso de red: un único interruptor para cliente y servidor.
fn permisos_red() -> &'static str {
    r#"{"red": {"habilitado": true}}"#
}

fn ejecutar_en(
    raiz: &Path,
    permisos_json: &str,
    codigo: &str,
) -> Rc<maquina_virtual::valores::EntornoModulo> {
    let fuente = Fuente::nueva("prueba.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("el código de prueba debe parsear");
    semantica::analizar_modulo(&ast).expect("el código de prueba debe pasar la semántica");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar bytecode");

    let valor: serde_json::Value =
        serde_json::from_str(permisos_json).expect("permisos de prueba en JSON válido");
    let guardian = Rc::new(GuardianPermisos::denegado());
    guardian.configurar(
        Permisos::desde_json(&valor).expect("permisos de prueba válidos"),
        raiz,
    );

    // Resolución mínima de imports nativos (el motor hace esto en producción).
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

    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro_con_permisos(
        &guardian,
    )));
    let (entorno, _valor) = vm
        .cargar_modulo(Rc::new(modulo), importaciones)
        .expect("el código de prueba debe ejecutar sin errores");
    entorno
}

fn ejecutar(nombre: &str, codigo: &str) -> Rc<maquina_virtual::valores::EntornoModulo> {
    let raiz = raiz_temporal(nombre);
    let entorno = ejecutar_en(&raiz, permisos_red(), codigo);
    let _ = std::fs::remove_dir_all(&raiz);
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

const IMPORTAR: &str = "importar { ServidorHttp, Enrutador, ClienteHttp, PeticionEntrante, \
                        RespuestaSaliente, Continuacion, ErrorHttp, RespuestaHttp, \
                        ProgresoPeticion, HttpCodigos } desde \"quetzal/red\"\n";

/// Importación de las pruebas de archivos y binario: agrega `Formulario`,
/// `ParteArchivo` y los objetos de `quetzal/sistema_archivos`.
const IMPORTAR_ARCHIVOS: &str = "importar { ServidorHttp, ClienteHttp, PeticionEntrante, \
                                 RespuestaSaliente, RespuestaHttp, Formulario, ParteArchivo } \
                                 desde \"quetzal/red\"\n\
                                 importar { SistemaArchivos, Archivo, Bits } desde \
                                 \"quetzal/sistema_archivos\"\n";

/// Permiso de red más acceso total al proyecto, para las pruebas que suben o
/// bajan archivos del disco.
fn permisos_red_y_disco() -> &'static str {
    r#"{"red": {"habilitado": true},
        "sistema_archivos": {"habilitado": true,
            "directorios": [{"ruta": "./", "permiso": "todo"}]}}"#
}

/// Servidor HTTP mínimo, ajeno a la VM, para probar el cliente síncrono.
/// Devuelve el puerto y responde siempre lo mismo.
fn servidor_de_prueba(cuerpo: &'static str) -> u16 {
    let escucha = TcpListener::bind(("127.0.0.1", 0)).expect("se puede abrir un puerto libre");
    let puerto = escucha.local_addr().expect("dirección local").port();
    std::thread::spawn(move || {
        for conexion in escucha.incoming() {
            let Ok(mut flujo) = conexion else { break };
            let mut entrada = [0u8; 4096];
            let _ = flujo.read(&mut entrada);
            let respuesta = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
                 Connection: close\r\n\r\n{cuerpo}",
                cuerpo.len()
            );
            let _ = flujo.write_all(respuesta.as_bytes());
            let _ = flujo.flush();
        }
    });
    puerto
}

/// Servidor crudo que guarda la petición completa y responde un JSON fijo.
/// Sirve para comprobar qué escribe el cliente en el cable.
fn servidor_capturador() -> (u16, std::sync::mpsc::Receiver<Vec<u8>>) {
    let escucha = TcpListener::bind(("127.0.0.1", 0)).expect("se puede abrir un puerto libre");
    let puerto = escucha.local_addr().expect("dirección local").port();
    let (emisor, receptor) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for conexion in escucha.incoming() {
            let Ok(mut flujo) = conexion else { break };
            let mut peticion = Vec::new();
            let mut buffer = [0u8; 4096];
            loop {
                match flujo.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(leidos) => {
                        peticion.extend_from_slice(&buffer[..leidos]);
                        if peticion_completa(&peticion) {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            let _ = emisor.send(peticion);
            let cuerpo = "{\"ok\":true}";
            let respuesta = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
                 Connection: close\r\n\r\n{cuerpo}",
                cuerpo.len()
            );
            let _ = flujo.write_all(respuesta.as_bytes());
            let _ = flujo.flush();
        }
    });
    (puerto, receptor)
}

/// Si ya llegaron las cabeceras y todo el cuerpo anunciado.
fn peticion_completa(datos: &[u8]) -> bool {
    let Some(fin) = datos.windows(4).position(|ventana| ventana == b"\r\n\r\n") else {
        return false;
    };
    let cabeceras = String::from_utf8_lossy(&datos[..fin]).to_ascii_lowercase();
    let longitud = cabeceras
        .lines()
        .find_map(|linea| linea.strip_prefix("content-length:"))
        .and_then(|valor| valor.trim().parse::<usize>().ok())
        .unwrap_or(0);
    datos.len() >= fin + 4 + longitud
}

// CA-01: la importación expone los objetos documentados del módulo.
#[test]
fn deberia_exponer_los_objetos_del_modulo_red() {
    let entorno = ejecutar(
        "importacion",
        &format!(
            "{IMPORTAR}\
             ServidorHttp servidor = nuevo ServidorHttp()\n\
             Enrutador api = nuevo Enrutador()\n\
             ClienteHttp cliente = nuevo ClienteHttp()\n\
             texto descripcion = HttpCodigos.descripcion(404)\n\
             log escuchando = servidor.esta_escuchando()\n"
        ),
    );
    assert!(texto_global(&entorno, "descripcion").contains("No encontrado"));
    assert_eq!(texto_global(&entorno, "escuchando"), "falso");
}

// CA-02: HttpCodigos describe y clasifica los estados según MDN.
#[test]
fn http_codigos_deberia_describir_y_clasificar_los_estados() {
    let entorno = ejecutar(
        "codigos",
        &format!(
            "{IMPORTAR}\
             texto correcto = HttpCodigos.descripcion(200)\n\
             texto razon = HttpCodigos.razon(418)\n\
             texto categoria = HttpCodigos.categoria(503)\n\
             log exitoso = HttpCodigos.es_exitoso(204)\n\
             log error_cliente = HttpCodigos.es_error_cliente(404)\n\
             log error_servidor = HttpCodigos.es_error_servidor(500)\n\
             log redireccion = HttpCodigos.es_redireccion(301)\n\
             entero creado = HttpCodigos.CREADO\n\
             entero total = HttpCodigos.todos().longitud()\n"
        ),
    );
    assert!(texto_global(&entorno, "correcto").contains("Correcto"));
    assert_eq!(texto_global(&entorno, "razon"), "I'm a teapot");
    assert_eq!(texto_global(&entorno, "categoria"), "error del servidor");
    assert_eq!(texto_global(&entorno, "exitoso"), "verdadero");
    assert_eq!(texto_global(&entorno, "error_cliente"), "verdadero");
    assert_eq!(texto_global(&entorno, "error_servidor"), "verdadero");
    assert_eq!(texto_global(&entorno, "redireccion"), "verdadero");
    assert_eq!(texto_global(&entorno, "creado"), "201");
    assert!(
        texto_global(&entorno, "total")
            .parse::<i64>()
            .unwrap_or(0)
            > 50
    );
}

// CA-03: el servidor enruta con parámetros, interceptores y enrutadores
// montados, y el cliente asincrónico conversa con él sin bloquear el bucle.
#[test]
fn servidor_y_cliente_deberian_conversar_con_funciones_nombradas() {
    let entorno = ejecutar(
        "servidor_cliente",
        &format!(
            "{IMPORTAR}\
             texto var traza = \"\"\n\
             vacio anotar(PeticionEntrante peticion, RespuestaSaliente respuesta, \
             Continuacion siguiente) {{\n\
                 traza += peticion.metodo_espanol() + \" \" + peticion.ruta() + \";\"\n\
                 siguiente.siguiente()\n\
             }}\n\
             vacio verUsuario(PeticionEntrante peticion, RespuestaSaliente respuesta) {{\n\
                 respuesta.jsn({{ id: peticion.parametro(\"id\") }})\n\
             }}\n\
             vacio crearUsuario(PeticionEntrante peticion, RespuestaSaliente respuesta) {{\n\
                 respuesta.estado(201).jsn({{ recibido: peticion.cuerpo() }})\n\
             }}\n\
             ServidorHttp servidor = nuevo ServidorHttp()\n\
             Enrutador api = nuevo Enrutador()\n\
             api.usar(anotar)\n\
             api.obtener(\"/usuarios/:id\", verUsuario)\n\
             api.publicar(\"/usuarios\", crearUsuario)\n\
             servidor.usar(\"/api\", api)\n\
             servidor.escuchar(0)\n\
             entero puerto = servidor.puerto()\n\
             ClienteHttp cliente = nuevo ClienteHttp({{ base_url: \"http://127.0.0.1:\" + \
             puerto.texto() }})\n\
             RespuestaHttp uno = esperar cliente.obtener_asincrono(\"/api/usuarios/42\")\n\
             RespuestaHttp dos = esperar cliente.publicar_asincrono(\"/api/usuarios\", \
             {{ nombre: \"Ana\" }})\n\
             texto id = uno.datos().id\n\
             entero estado_creado = dos.estado()\n\
             texto nombre_creado = dos.datos().recibido.nombre\n\
             log ok = uno.ok()\n\
             servidor.cerrar()\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "id"), "42");
    assert_eq!(texto_global(&entorno, "estado_creado"), "201");
    assert_eq!(texto_global(&entorno, "nombre_creado"), "Ana");
    assert_eq!(texto_global(&entorno, "ok"), "verdadero");
    assert_eq!(
        texto_global(&entorno, "traza"),
        "obtener /api/usuarios/42;publicar /api/usuarios;"
    );
}

// CA-04: el método QUERY del RFC 10008 viaja con su cuerpo en cliente y
// servidor, y se declara seguro e idempotente.
#[test]
fn consultar_deberia_usar_el_metodo_query_con_cuerpo() {
    let entorno = ejecutar(
        "query",
        &format!(
            "{IMPORTAR}\
             vacio buscar(PeticionEntrante peticion, RespuestaSaliente respuesta) {{\n\
                 respuesta.jsn({{ metodo: peticion.metodo(), \
                 seguro: peticion.es_seguro(), \
                 idempotente: peticion.es_idempotente(), \
                 termino: peticion.cuerpo().termino }})\n\
             }}\n\
             ServidorHttp servidor = nuevo ServidorHttp()\n\
             servidor.consultar(\"/busqueda\", buscar)\n\
             servidor.escuchar(0)\n\
             entero puerto = servidor.puerto()\n\
             ClienteHttp cliente = nuevo ClienteHttp()\n\
             RespuestaHttp resultado = esperar cliente.consultar_asincrono(\
             \"http://127.0.0.1:\" + puerto.texto() + \"/busqueda\", \
             {{ termino: \"quetzal\" }})\n\
             texto metodo = resultado.datos().metodo\n\
             log seguro = resultado.datos().seguro\n\
             log idempotente = resultado.datos().idempotente\n\
             texto termino = resultado.datos().termino\n\
             servidor.cerrar()\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "metodo"), "QUERY");
    assert_eq!(texto_global(&entorno, "seguro"), "verdadero");
    assert_eq!(texto_global(&entorno, "idempotente"), "verdadero");
    assert_eq!(texto_global(&entorno, "termino"), "quetzal");
}

// CA-05: una ruta sin manejador responde 404 y el estado se puede leer sin
// que el cliente lance, gracias a `validar_estado`.
#[test]
fn una_ruta_desconocida_deberia_responder_404() {
    let entorno = ejecutar(
        "no_encontrada",
        &format!(
            "{IMPORTAR}\
             ServidorHttp servidor = nuevo ServidorHttp()\n\
             servidor.escuchar(0)\n\
             entero puerto = servidor.puerto()\n\
             ClienteHttp cliente = nuevo ClienteHttp({{ base_url: \"http://127.0.0.1:\" + \
             puerto.texto(), validar_estado: falso }})\n\
             RespuestaHttp faltante = esperar cliente.obtener_asincrono(\"/nada\")\n\
             entero estado = faltante.estado()\n\
             texto descripcion = HttpCodigos.descripcion(faltante.estado())\n\
             texto var lanzado = \"\"\n\
             intentar {{\n\
                 RespuestaHttp estricta = esperar cliente.obtener_asincrono(\"/nada\", \
                 {{ validar_estado: verdadero }})\n\
             }} capturar (excepcion e) {{\n\
                 lanzado = e.mensaje\n\
             }}\n\
             servidor.cerrar()\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "estado"), "404");
    assert!(texto_global(&entorno, "descripcion").contains("No encontrado"));
    assert!(texto_global(&entorno, "lanzado").contains("404"));
}

// CA-06: los interceptores de error atienden lo que lanza un manejador.
#[test]
fn un_manejador_que_lanza_deberia_llegar_al_interceptor_de_error() {
    let entorno = ejecutar(
        "errores",
        &format!(
            "{IMPORTAR}\
             vacio fallar(PeticionEntrante peticion, RespuestaSaliente respuesta) {{\n\
                 lanzar \"algo salió mal\"\n\
             }}\n\
             vacio atender(ErrorHttp fallo, PeticionEntrante peticion, \
             RespuestaSaliente respuesta, Continuacion siguiente) {{\n\
                 respuesta.estado(500).jsn({{ error: fallo.mensaje() }})\n\
             }}\n\
             ServidorHttp servidor = nuevo ServidorHttp()\n\
             servidor.obtener(\"/falla\", fallar)\n\
             servidor.manejar_errores(atender)\n\
             servidor.escuchar(0)\n\
             entero puerto = servidor.puerto()\n\
             ClienteHttp cliente = nuevo ClienteHttp({{ base_url: \"http://127.0.0.1:\" + \
             puerto.texto(), validar_estado: falso }})\n\
             RespuestaHttp respuesta = esperar cliente.obtener_asincrono(\"/falla\")\n\
             entero estado = respuesta.estado()\n\
             texto mensaje = respuesta.datos().error\n\
             servidor.cerrar()\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "estado"), "500");
    assert_eq!(texto_global(&entorno, "mensaje"), "algo salió mal");
}

// CA-07: el cliente síncrono, sus interceptores y el progreso de descarga
// (con porcentaje) contra un servidor ajeno a la VM.
#[test]
fn el_cliente_deberia_interceptar_y_reportar_progreso() {
    let puerto = servidor_de_prueba("{\"saludo\":\"hola\"}");
    let entorno = ejecutar(
        "cliente_progreso",
        &format!(
            "{IMPORTAR}\
             texto var visto = \"\"\n\
             entero var avisos = 0\n\
             jsn agregarClave(jsn var configuracion) {{\n\
                 configuracion.cabeceras = {{ \"X-Clave\": \"secreta\" }}\n\
                 retornar configuracion\n\
             }}\n\
             RespuestaHttp anotarRespuesta(RespuestaHttp respuesta) {{\n\
                 visto = respuesta.estado().texto()\n\
                 retornar respuesta\n\
             }}\n\
             vacio alDescargar(ProgresoPeticion progreso) {{\n\
                 avisos += 1\n\
             }}\n\
             ClienteHttp cliente = nuevo ClienteHttp({{ base_url: \"http://127.0.0.1:{puerto}\" }})\n\
             cliente.interceptar_peticion(agregarClave)\n\
             cliente.interceptar_respuesta(anotarRespuesta)\n\
             RespuestaHttp respuesta = cliente.obtener(\"/datos\", \
             {{ al_progreso_descarga: alDescargar }})\n\
             texto saludo = respuesta.datos().saludo\n\
             entero estado = respuesta.estado()\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "saludo"), "hola");
    assert_eq!(texto_global(&entorno, "estado"), "200");
    assert_eq!(texto_global(&entorno, "visto"), "200");
    assert!(
        texto_global(&entorno, "avisos")
            .parse::<i64>()
            .unwrap_or(0)
            >= 1
    );
}

// CA-08: sin el permiso `red`, ninguna operación de red procede.
#[test]
fn sin_permiso_de_red_deberian_fallar_servidor_y_cliente() {
    let raiz = raiz_temporal("sin_permiso");
    let entorno = ejecutar_en(
        &raiz,
        r#"{"red": {"habilitado": false}}"#,
        &format!(
            "{IMPORTAR}\
             texto var sin_servidor = \"\"\n\
             texto var sin_cliente = \"\"\n\
             ServidorHttp servidor = nuevo ServidorHttp()\n\
             intentar {{\n\
                 servidor.escuchar(0)\n\
             }} capturar (excepcion e) {{\n\
                 sin_servidor = e.mensaje\n\
             }}\n\
             ClienteHttp cliente = nuevo ClienteHttp()\n\
             intentar {{\n\
                 RespuestaHttp respuesta = cliente.obtener(\"http://127.0.0.1:9/nada\")\n\
             }} capturar (excepcion e) {{\n\
                 sin_cliente = e.mensaje\n\
             }}\n"
        ),
    );
    assert!(texto_global(&entorno, "sin_servidor").contains("permiso de red"));
    assert!(texto_global(&entorno, "sin_cliente").contains("permiso de red"));
    let _ = std::fs::remove_dir_all(&raiz);
}

// CA-09: sin declarar ningún permiso, la red tampoco procede.
#[test]
fn un_proyecto_sin_permisos_no_deberia_alcanzar_la_red() {
    let raiz = raiz_temporal("sin_declarar");
    let entorno = ejecutar_en(
        &raiz,
        "{}",
        &format!(
            "{IMPORTAR}\
             texto var motivo = \"\"\n\
             ClienteHttp cliente = nuevo ClienteHttp()\n\
             intentar {{\n\
                 RespuestaHttp fuera = cliente.obtener(\"http://127.0.0.1:9/nada\")\n\
             }} capturar (excepcion e) {{\n\
                 motivo = e.mensaje\n\
             }}\n"
        ),
    );
    assert!(texto_global(&entorno, "motivo").contains("permiso de red"));
    let _ = std::fs::remove_dir_all(&raiz);
}

// CA-11: un `Bits` viaja como cuerpo binario con `Content-Type` y
// `Content-Disposition`, y vuelve igual desde el servidor.
#[test]
fn bits_deberian_viajar_como_cuerpo_binario_estandar() {
    let entorno = ejecutar(
        "cuerpo_binario",
        &format!(
            "{IMPORTAR_ARCHIVOS}\
             texto var tipo_recibido = \"\"\n\
             texto var nombre_recibido = \"\"\n\
             entero var bytes_recibidos = 0\n\
             vacio eco(PeticionEntrante peticion, RespuestaSaliente respuesta) {{\n\
                 tipo_recibido = peticion.tipo_contenido()\n\
                 nombre_recibido = peticion.nombre_archivo()\n\
                 bytes_recibidos = peticion.cuerpo_bits().longitud()\n\
                 respuesta.enviar(peticion.cuerpo_bits(), {{ nombre_archivo: \"eco.bin\" }})\n\
             }}\n\
             ServidorHttp servidor = nuevo ServidorHttp()\n\
             servidor.publicar(\"/subir\", eco)\n\
             servidor.escuchar(0)\n\
             entero puerto = servidor.puerto()\n\
             ClienteHttp cliente = nuevo ClienteHttp({{ base_url: \"http://127.0.0.1:\" + \
             puerto.texto() }})\n\
             Bits carga = Bits.desde_texto(\"hola binario\")\n\
             RespuestaHttp respuesta = esperar cliente.publicar_asincrono(\"/subir\", carga, \
             {{ nombre_archivo: \"saludo.bin\" }})\n\
             texto devuelto = respuesta.bits().texto()\n\
             texto nombre_devuelto = respuesta.nombre_archivo()\n\
             texto tipo_devuelto = respuesta.tipo_contenido()\n\
             servidor.cerrar()\n"
        ),
    );
    assert_eq!(
        texto_global(&entorno, "tipo_recibido"),
        "application/octet-stream"
    );
    assert_eq!(texto_global(&entorno, "nombre_recibido"), "saludo.bin");
    assert_eq!(texto_global(&entorno, "bytes_recibidos"), "12");
    assert_eq!(texto_global(&entorno, "devuelto"), "hola binario");
    assert_eq!(texto_global(&entorno, "nombre_devuelto"), "eco.bin");
    assert_eq!(
        texto_global(&entorno, "tipo_devuelto"),
        "application/octet-stream"
    );
}

// CA-12: un `Formulario` con campos y archivos viaja como multipart y el
// servidor lo entrega ya analizado en `peticion.cuerpo()`.
#[test]
fn un_formulario_deberia_viajar_como_multipart() {
    let entorno = ejecutar(
        "multipart",
        &format!(
            "{IMPORTAR_ARCHIVOS}\
             texto var titulo_recibido = \"\"\n\
             texto var nombre_recibido = \"\"\n\
             texto var tipo_recibido = \"\"\n\
             texto var contenido_recibido = \"\"\n\
             entero var archivos_recibidos = 0\n\
             vacio subir(PeticionEntrante peticion, RespuestaSaliente respuesta) {{\n\
                 Formulario formulario = peticion.cuerpo()\n\
                 titulo_recibido = formulario.campo_texto(\"titulo\")\n\
                 archivos_recibidos = formulario.archivos().longitud()\n\
                 ParteArchivo documento = formulario.archivo_parte(\"documento\")\n\
                 nombre_recibido = documento.nombre()\n\
                 tipo_recibido = documento.tipo()\n\
                 contenido_recibido = documento.bits().texto()\n\
                 respuesta.jsn({{ campos: formulario.campos() }})\n\
             }}\n\
             ServidorHttp servidor = nuevo ServidorHttp()\n\
             servidor.publicar(\"/subir\", subir)\n\
             servidor.escuchar(0)\n\
             entero puerto = servidor.puerto()\n\
             ClienteHttp cliente = nuevo ClienteHttp({{ base_url: \"http://127.0.0.1:\" + \
             puerto.texto() }})\n\
             Formulario formulario = nuevo Formulario()\n\
             formulario.campo(\"titulo\", \"informe\")\n\
             formulario.campo(\"autor\", \"Ana\")\n\
             formulario.archivo(\"documento\", Bits.desde_texto(\"contenido del documento\"), \
             {{ nombre: \"informe.txt\", tipo: \"text/plain\" }})\n\
             RespuestaHttp respuesta = esperar cliente.publicar_asincrono(\"/subir\", formulario)\n\
             texto autor = respuesta.datos().campos.autor\n\
             servidor.cerrar()\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "titulo_recibido"), "informe");
    assert_eq!(texto_global(&entorno, "archivos_recibidos"), "1");
    assert_eq!(texto_global(&entorno, "nombre_recibido"), "informe.txt");
    assert_eq!(texto_global(&entorno, "tipo_recibido"), "text/plain");
    assert_eq!(
        texto_global(&entorno, "contenido_recibido"),
        "contenido del documento"
    );
    assert_eq!(texto_global(&entorno, "autor"), "Ana");
}

// CA-13: lo que el cliente escribe en el cable es `multipart/form-data` del
// RFC 7578, legible por cualquier cliente o servidor de otro lenguaje.
#[test]
fn el_cliente_deberia_escribir_multipart_estandar_en_el_cable() {
    let (puerto, peticiones) = servidor_capturador();
    ejecutar(
        "multipart_cable",
        &format!(
            "{IMPORTAR_ARCHIVOS}\
             ClienteHttp cliente = nuevo ClienteHttp({{ base_url: \"http://127.0.0.1:{puerto}\" }})\n\
             Formulario formulario = nuevo Formulario()\n\
             formulario.campo(\"titulo\", \"informe\")\n\
             formulario.archivo(\"documento\", Bits.desde_texto(\"bytes del documento\"), \
             {{ nombre: \"informe.txt\", tipo: \"text/plain\" }})\n\
             RespuestaHttp respuesta = cliente.publicar(\"/subir\", formulario)\n"
        ),
    );

    let peticion = peticiones
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("el servidor debe recibir la petición");
    let texto = String::from_utf8_lossy(&peticion).to_string();
    let frontera = texto
        .lines()
        .find_map(|linea| {
            linea
                .to_ascii_lowercase()
                .starts_with("content-type: multipart/form-data; boundary=")
                .then(|| linea.rsplit('=').next().unwrap_or_default().to_string())
        })
        .expect("el cliente debe anunciar multipart/form-data con frontera");
    assert!(texto.contains(&format!("--{frontera}\r\n")));
    assert!(texto.contains("Content-Disposition: form-data; name=\"titulo\"\r\n\r\ninforme"));
    assert!(
        texto.contains(
            "Content-Disposition: form-data; name=\"documento\"; filename=\"informe.txt\""
        )
    );
    assert!(texto.contains("Content-Type: text/plain"));
    assert!(texto.contains("bytes del documento"));
    assert!(texto.ends_with(&format!("--{frontera}--\r\n")));
}

// CA-14: el servidor entiende un multipart escrito a mano (curl, fetch, otro
// lenguaje), no solo el que produce el cliente de Quetzal.
#[test]
fn el_servidor_deberia_entender_un_multipart_ajeno() {
    let mut codigo = String::from(IMPORTAR_ARCHIVOS);
    codigo.push_str(
        r#"
texto var titulo = ""
texto var nombre = ""
texto var contenido = ""
vacio subir(PeticionEntrante peticion, RespuestaSaliente respuesta) {
    Formulario formulario = peticion.cuerpo()
    titulo = formulario.campo_texto("titulo")
    ParteArchivo nota = formulario.archivo_parte("archivo")
    nombre = nota.nombre()
    contenido = nota.bits().texto()
    respuesta.texto("recibido")
}
ServidorHttp servidor = nuevo ServidorHttp()
servidor.publicar("/curl", subir)
servidor.escuchar(0)
entero puerto = servidor.puerto()
ClienteHttp cliente = nuevo ClienteHttp({ base_url: "http://127.0.0.1:" + puerto.texto() })
texto cuerpo = "--FRONTERA\r\nContent-Disposition: form-data; name=\"titulo\"\r\n\r\ndesde curl\r\n--FRONTERA\r\nContent-Disposition: form-data; name=\"archivo\"; filename=\"nota.txt\"\r\nContent-Type: text/plain\r\n\r\nhola mundo\r\n--FRONTERA--\r\n"
RespuestaHttp respuesta = esperar cliente.publicar_asincrono("/curl", cuerpo, { cabeceras: { "Content-Type": "multipart/form-data; boundary=FRONTERA" } })
texto eco = respuesta.datos()
servidor.cerrar()
"#,
    );
    let entorno = ejecutar("multipart_ajeno", &codigo);
    assert_eq!(texto_global(&entorno, "titulo"), "desde curl");
    assert_eq!(texto_global(&entorno, "nombre"), "nota.txt");
    assert_eq!(texto_global(&entorno, "contenido"), "hola mundo");
    assert_eq!(texto_global(&entorno, "eco"), "recibido");
}

// CA-15: un `Archivo` del disco se sube tal cual y el servidor devuelve otro
// archivo como descarga, con el permiso `sistema-archivos` de por medio.
#[test]
fn un_archivo_del_disco_deberia_subirse_y_descargarse() {
    let raiz = raiz_temporal("archivo_http");
    std::fs::write(raiz.join("informe.txt"), "ventas: 120").expect("se puede crear el informe");
    std::fs::write(raiz.join("respuesta.txt"), "gracias").expect("se puede crear la respuesta");

    let entorno = ejecutar_en(
        &raiz,
        permisos_red_y_disco(),
        &format!(
            "{IMPORTAR_ARCHIVOS}\
             texto var nombre_subido = \"\"\n\
             texto var contenido_subido = \"\"\n\
             vacio recibir(PeticionEntrante peticion, RespuestaSaliente respuesta) {{\n\
                 nombre_subido = peticion.nombre_archivo()\n\
                 contenido_subido = peticion.cuerpo_bits().texto()\n\
                 Archivo salida = SistemaArchivos.abrir(\"./respuesta.txt\")\n\
                 respuesta.enviar(salida)\n\
             }}\n\
             ServidorHttp servidor = nuevo ServidorHttp()\n\
             servidor.publicar(\"/documentos\", recibir)\n\
             servidor.escuchar(0)\n\
             entero puerto = servidor.puerto()\n\
             ClienteHttp cliente = nuevo ClienteHttp({{ base_url: \"http://127.0.0.1:\" + \
             puerto.texto() }})\n\
             Archivo informe = SistemaArchivos.abrir(\"./informe.txt\")\n\
             RespuestaHttp respuesta = esperar cliente.publicar_asincrono(\"/documentos\", informe)\n\
             texto nombre_bajado = respuesta.nombre_archivo()\n\
             texto tipo_bajado = respuesta.tipo_contenido()\n\
             texto contenido_bajado = respuesta.bits().texto()\n\
             servidor.cerrar()\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "nombre_subido"), "informe.txt");
    assert_eq!(texto_global(&entorno, "contenido_subido"), "ventas: 120");
    assert_eq!(texto_global(&entorno, "nombre_bajado"), "respuesta.txt");
    assert!(texto_global(&entorno, "tipo_bajado").starts_with("text/plain"));
    assert_eq!(texto_global(&entorno, "contenido_bajado"), "gracias");
    let _ = std::fs::remove_dir_all(&raiz);
}

// CA-16: mandar un archivo del disco necesita el permiso `sistema-archivos`
// además del de red: solo con `red` habilitado, la subida no procede.
#[test]
fn subir_un_archivo_sin_permiso_de_disco_deberia_fallar() {
    let raiz = raiz_temporal("archivo_sin_disco");
    std::fs::write(raiz.join("informe.txt"), "ventas: 120").expect("se puede crear el informe");
    let (puerto, _peticiones) = servidor_capturador();

    let entorno = ejecutar_en(
        &raiz,
        permisos_red(),
        &format!(
            "{IMPORTAR_ARCHIVOS}\
             texto var motivo = \"\"\n\
             ClienteHttp cliente = nuevo ClienteHttp({{ base_url: \"http://127.0.0.1:{puerto}\" }})\n\
             intentar {{\n\
                 Archivo informe = SistemaArchivos.abrir(\"./informe.txt\")\n\
                 RespuestaHttp respuesta = cliente.publicar(\"/documentos\", informe)\n\
             }} capturar (excepcion e) {{\n\
                 motivo = e.mensaje\n\
             }}\n"
        ),
    );
    assert!(
        texto_global(&entorno, "motivo").contains("permiso de sistema de archivos"),
        "se esperaba un fallo de permiso de disco, pero se obtuvo: {}",
        texto_global(&entorno, "motivo")
    );
    let _ = std::fs::remove_dir_all(&raiz);
}

// CA-10: los callbacks son funciones nombradas de primera clase; el tipo
// `funcion` permite pasarlas y llamarlas.
#[test]
fn una_funcion_nombrada_deberia_pasarse_como_callback() {
    let entorno = ejecutar(
        "callbacks",
        "entero doble(entero valor) {\n    retornar valor * 2\n}\n\
         entero aplicar(funcion accion, entero valor) {\n    retornar accion(valor)\n}\n\
         entero resultado = aplicar(doble, 21)\n",
    );
    assert_eq!(texto_global(&entorno, "resultado"), "42");
}
