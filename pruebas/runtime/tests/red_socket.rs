//! Pruebas de `quetzal/red`: `Socket` (cliente TCP) y `ServidorSocket` (con
//! `ConexionSocket`), síncrono y asincrónico.
//!
//! La VM corre en un solo hilo (los valores usan `Rc`, no son `Send`), así
//! que un mismo script de Quetzal no puede tener a la vez un `Socket`
//! cliente haciendo una lectura síncrona (bloquea el hilo de la VM sin
//! bombear el bucle de eventos) y un manejador de `ServidorSocket` que
//! todavía no se despachó (eso sí requiere bombear el bucle): se
//! estancarían mutuamente. Por eso, igual que en `red_http.rs`, cada lado
//! se prueba contra una contraparte cruda hecha con `std::net` desde un
//! hilo de Rust aparte.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;
use std::thread;
use std::time::Duration;

use indexmap::IndexMap;
use maquina_virtual::{Valor, Vm};
use nucleo::Fuente;

// =====================================================================
// Ejecución de código Quetzal con permisos de red configurables
// =====================================================================

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

// =====================================================================
// Contraparte cruda de prueba: servidor TCP de eco con std::net
// =====================================================================

/// Servidor TCP en un puerto efímero de loopback que atiende una conexión:
/// lee una línea terminada en `\n` y responde `"eco:" + línea + "\n"`, luego
/// lee `cantidad_bits` bytes y los devuelve tal cual (eco binario).
fn iniciar_servidor_eco(cantidad_bits: usize) -> u16 {
    let escucha = TcpListener::bind("127.0.0.1:0").expect("bind en puerto efímero");
    let puerto = escucha.local_addr().expect("dirección local").port();
    thread::spawn(move || {
        if let Ok((flujo, _)) = escucha.accept() {
            let mut lector = BufReader::new(flujo.try_clone().expect("clonar el flujo de prueba"));
            let mut escritor = flujo;
            let mut linea = String::new();
            lector.read_line(&mut linea).expect("leer la línea de prueba");
            let linea = linea.trim_end_matches(['\r', '\n']);
            escritor
                .write_all(format!("eco:{linea}\n").as_bytes())
                .expect("escribir la respuesta de prueba");
            let mut bits = vec![0u8; cantidad_bits];
            if cantidad_bits > 0 {
                lector.read_exact(&mut bits).expect("leer los bytes de prueba");
                escritor.write_all(&bits).expect("escribir el eco binario de prueba");
            }
        }
    });
    puerto
}

// =====================================================================
// Socket (cliente): pruebas contra el servidor crudo de prueba
// =====================================================================

#[test]
fn socket_deberia_conectarse_y_hacer_eco_sincrono_de_linea_y_bits() {
    let puerto = iniciar_servidor_eco(5);
    let (_vm, entorno) = ejecutar_con_permiso_cliente(&format!(
        "importar {{ Socket }} desde \"quetzal/red\"\n\
         importar {{ Bits }} desde \"quetzal/bits\"\n\
         Socket base = nuevo Socket()\n\
         Socket cliente = base.conectar(\"127.0.0.1\", {puerto})\n\
         cliente.enviar_texto(\"hola\\n\")\n\
         texto respuesta = cliente.recibir_linea()\n\
         cliente.enviar_bits(Bits.desde_hex(\"48656c6c6f\"))\n\
         Bits eco = cliente.recibir_bits(5)\n\
         texto eco_hex = eco.a_hex()\n\
         cliente.cerrar()\n"
    ));
    assert_eq!(texto_global(&entorno, "respuesta"), "eco:hola");
    assert_eq!(texto_global(&entorno, "eco_hex"), "48656c6c6f");
}

#[test]
fn socket_deberia_conectarse_de_forma_asincrona_y_recibir_bits_asincrono() {
    let puerto = iniciar_servidor_eco(5);
    let (mut vm, entorno) = ejecutar_con_permiso_cliente(&format!(
        "importar {{ Socket }} desde \"quetzal/red\"\n\
         importar {{ Bits }} desde \"quetzal/bits\"\n\
         Socket base = nuevo Socket()\n\
         Socket cliente = esperar base.conectar_asincrono(\"127.0.0.1\", {puerto})\n\
         cliente.enviar_texto(\"async\\n\")\n\
         texto respuesta = cliente.recibir_linea()\n\
         cliente.enviar_bits(Bits.desde_hex(\"48656c6c6f\"))\n\
         Bits eco = esperar cliente.recibir_bits_asincrono(5)\n\
         texto eco_hex = eco.a_hex()\n\
         cliente.cerrar()\n"
    ));
    assert_eq!(texto_global(&entorno, "respuesta"), "eco:async");
    assert_eq!(texto_global(&entorno, "eco_hex"), "48656c6c6f");
    let _ = &mut vm;
}

#[test]
fn socket_sin_permiso_deberia_fallar_con_e0701() {
    let fuente = Fuente::nueva(
        "prueba.qz",
        "importar { Socket } desde \"quetzal/red\"\n\
         Socket base = nuevo Socket()\n\
         base.conectar(\"127.0.0.1\", 9)\n",
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
        .expect_err("sin permiso de red la conexión debe fallar");
    assert_eq!(error.codigo, "E0701");
}

// =====================================================================
// ServidorSocket: pruebas contra un cliente crudo de prueba
// =====================================================================

#[test]
fn servidor_socket_deberia_recibir_conexion_y_hacer_eco_de_linea_y_bits() {
    let (mut vm, entorno) = ejecutar_con_permiso_servidor(
        "importar { ServidorSocket, ConexionSocket } desde \"quetzal/red\"\n\
         importar { Bits } desde \"quetzal/bits\"\n\
         ServidorSocket servidor = nuevo ServidorSocket()\n\
         vacio manejador(ConexionSocket conexion) {\n\
         \u{20}   texto linea = conexion.recibir_linea()\n\
         \u{20}   conexion.enviar_texto(\"eco:\" + linea + \"\\n\")\n\
         \u{20}   Bits datos = conexion.recibir_bits(5)\n\
         \u{20}   conexion.enviar_bits(datos)\n\
         \u{20}   conexion.cerrar()\n\
         \u{20}   servidor.detener()\n\
         }\n\
         servidor.al_conectar(manejador)\n\
         servidor.escuchar(0)\n\
         entero puerto = servidor.puerto()\n",
    );
    let puerto = entero_global(&entorno, "puerto") as u16;

    let solicitud = thread::spawn(move || {
        let flujo = TcpStream::connect(("127.0.0.1", puerto)).expect("el cliente de prueba debe conectarse");
        let mut lector = BufReader::new(flujo.try_clone().expect("clonar el flujo de prueba"));
        let mut escritor = flujo;
        escritor.write_all(b"hola\n").expect("enviar la línea de prueba");
        let mut respuesta = String::new();
        lector.read_line(&mut respuesta).expect("leer la respuesta de prueba");
        escritor
            .write_all(&[0x48, 0x65, 0x6c, 0x6c, 0x6f])
            .expect("enviar los bytes de prueba");
        let mut eco = [0u8; 5];
        lector.read_exact(&mut eco).expect("leer el eco binario de prueba");
        (respuesta.trim_end_matches(['\r', '\n']).to_string(), eco)
    });

    vm.drenar_bucle_eventos();
    let (respuesta, eco) = solicitud.join().expect("el hilo del cliente no debe entrar en panic");
    assert_eq!(respuesta, "eco:hola");
    assert_eq!(eco, [0x48, 0x65, 0x6c, 0x6c, 0x6f]);
    assert!(!vm.bucle().hay_trabajo_activo());
    let _ = entorno;
}

#[test]
fn servidor_socket_sin_permiso_deberia_fallar_con_e0701() {
    let fuente = Fuente::nueva(
        "prueba.qz",
        "importar { ServidorSocket } desde \"quetzal/red\"\n\
         ServidorSocket servidor = nuevo ServidorSocket()\n\
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
        .expect_err("sin permiso de red la creación del servidor debe fallar");
    assert_eq!(error.codigo, "E0701");
}

#[test]
fn socket_deberia_fijar_tiempo_de_espera_y_agotarlo_al_no_recibir_datos() {
    let escucha = TcpListener::bind("127.0.0.1:0").expect("bind en puerto efímero");
    let puerto = escucha.local_addr().expect("dirección local").port();
    // El servidor de prueba acepta la conexión pero nunca envía nada: el
    // cliente debe agotar su tiempo de espera al intentar leer.
    thread::spawn(move || {
        if let Ok((flujo, _)) = escucha.accept() {
            thread::sleep(Duration::from_secs(2));
            drop(flujo);
        }
    });

    let fuente = Fuente::nueva(
        "prueba.qz",
        format!(
            "importar {{ Socket }} desde \"quetzal/red\"\n\
             Socket base = nuevo Socket()\n\
             base.fijar_tiempo_espera(1)\n\
             Socket cliente = base.conectar(\"127.0.0.1\", {puerto})\n\
             texto linea = cliente.recibir_linea()\n"
        ),
    );
    let permisos = paquetes::Permisos::desde_json(&serde_json::json!({
        "red": { "habilitado": true, "cliente": true }
    }))
    .expect("permisos de prueba válidos");
    let guardian = Rc::new(runtime::GuardianPermisos::denegado());
    guardian.configurar(permisos, std::path::Path::new("."));
    let registro = modulos_nativos::crear_registro_con_permisos(&guardian);

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
    let mut vm = Vm::nueva(Rc::new(registro));
    let error = vm
        .cargar_modulo(Rc::new(modulo), importaciones)
        .expect_err("la lectura debe agotar el tiempo de espera y fallar");
    assert_eq!(error.codigo, "E0705");
}
