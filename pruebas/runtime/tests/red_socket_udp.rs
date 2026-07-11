//! Pruebas de `quetzal/red`: `SocketUdp` (datagramas UDP), síncrono y
//! asincrónico.
//!
//! Al ser sin conexión, un mismo script de Quetzal puede tener dos
//! `SocketUdp` (uno enlazado para recibir, otro para enviar) sin el riesgo
//! de estancamiento de TCP (`red_socket.rs`): `enviar_a` nunca espera a que
//! el otro lado lea nada, así que no hace falta bombear el bucle de
//! eventos entre el envío y la recepción.

use std::rc::Rc;

use indexmap::IndexMap;
use maquina_virtual::{Valor, Vm};
use nucleo::Fuente;

fn ejecutar_con_permisos(codigo: &str, permisos_json: serde_json::Value) -> Rc<maquina_virtual::valores::EntornoModulo> {
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
    entorno
}

fn ejecutar_con_permiso_completo(codigo: &str) -> Rc<maquina_virtual::valores::EntornoModulo> {
    ejecutar_con_permisos(
        codigo,
        serde_json::json!({ "red": { "habilitado": true, "cliente": true, "servidor": true } }),
    )
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

#[test]
fn socket_udp_deberia_exponer_estado_direccion_y_difusion() {
    let entorno = ejecutar_con_permiso_completo(
        "importar { SocketUdp } desde \"quetzal/red\"\n\
         SocketUdp socket = nuevo SocketUdp()\n\
         log antes = socket.esta_enlazado()\n\
         socket.enlazar(0)\n\
         log durante = socket.esta_enlazado()\n\
         jsn direccion = socket.direccion_local()\n\
         entero puerto_local = direccion.puerto\n\
         socket.permitir_difusion(verdadero)\n\
         socket.cerrar()\n\
         log despues = socket.esta_enlazado()\n",
    );
    assert!(!logico_global(&entorno, "antes"));
    assert!(logico_global(&entorno, "durante"));
    assert!(entero_global(&entorno, "puerto_local") > 0);
    assert!(!logico_global(&entorno, "despues"));
}

#[test]
fn socket_udp_deberia_enviar_y_recibir_texto_sincrono() {
    let entorno = ejecutar_con_permiso_completo(
        "importar { SocketUdp } desde \"quetzal/red\"\n\
         importar { Bits } desde \"quetzal/bits\"\n\
         SocketUdp receptor = nuevo SocketUdp()\n\
         receptor.enlazar(0)\n\
         entero puerto = receptor.puerto()\n\
         SocketUdp emisor = nuevo SocketUdp()\n\
         emisor.enviar_a(\"127.0.0.1\", puerto, \"hola udp\")\n\
         jsn datagrama = receptor.recibir()\n\
         texto origen = datagrama.origen\n\
         entero puerto_origen = datagrama.puerto\n\
         Bits datos = datagrama.datos\n\
         texto mensaje = datos.a_texto()\n\
         receptor.cerrar()\n\
         emisor.cerrar()\n",
    );
    assert_eq!(texto_global(&entorno, "origen"), "127.0.0.1");
    assert!(entero_global(&entorno, "puerto_origen") > 0);
    assert_eq!(texto_global(&entorno, "mensaje"), "hola udp");
}

#[test]
fn socket_udp_deberia_enviar_y_recibir_bits_binarios() {
    let entorno = ejecutar_con_permiso_completo(
        "importar { SocketUdp } desde \"quetzal/red\"\n\
         importar { Bits } desde \"quetzal/bits\"\n\
         SocketUdp receptor = nuevo SocketUdp()\n\
         receptor.enlazar(0)\n\
         entero puerto = receptor.puerto()\n\
         SocketUdp emisor = nuevo SocketUdp()\n\
         emisor.enviar_a(\"127.0.0.1\", puerto, Bits.desde_hex(\"48656c6c6f\"))\n\
         jsn datagrama = receptor.recibir()\n\
         Bits datos = datagrama.datos\n\
         texto hex = datos.a_hex()\n\
         receptor.cerrar()\n\
         emisor.cerrar()\n",
    );
    assert_eq!(texto_global(&entorno, "hex"), "48656c6c6f");
}

#[test]
fn socket_udp_deberia_recibir_de_forma_asincrona_con_esperar() {
    let entorno = ejecutar_con_permiso_completo(
        "importar { SocketUdp } desde \"quetzal/red\"\n\
         importar { Bits } desde \"quetzal/bits\"\n\
         SocketUdp receptor = nuevo SocketUdp()\n\
         receptor.enlazar(0)\n\
         entero puerto = receptor.puerto()\n\
         SocketUdp emisor = nuevo SocketUdp()\n\
         emisor.enviar_a(\"127.0.0.1\", puerto, \"async udp\")\n\
         jsn datagrama = esperar receptor.recibir_asincrono()\n\
         Bits datos = datagrama.datos\n\
         texto mensaje = datos.a_texto()\n\
         receptor.cerrar()\n\
         emisor.cerrar()\n",
    );
    assert_eq!(texto_global(&entorno, "mensaje"), "async udp");
}

#[test]
fn socket_udp_deberia_agotar_el_tiempo_de_espera_al_no_recibir_datos() {
    let fuente = Fuente::nueva(
        "prueba.qz",
        "importar { SocketUdp } desde \"quetzal/red\"\n\
         SocketUdp receptor = nuevo SocketUdp()\n\
         receptor.fijar_tiempo_espera(1)\n\
         receptor.enlazar(0)\n\
         jsn datagrama = receptor.recibir()\n",
    );
    let permisos = paquetes::Permisos::desde_json(&serde_json::json!({
        "red": { "habilitado": true, "servidor": true }
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
        .expect_err("la recepción debe agotar el tiempo de espera y fallar");
    assert_eq!(error.codigo, "E0705");
}

#[test]
fn socket_udp_enlazar_sin_permiso_deberia_fallar_con_e0701() {
    let fuente = Fuente::nueva(
        "prueba.qz",
        "importar { SocketUdp } desde \"quetzal/red\"\n\
         SocketUdp receptor = nuevo SocketUdp()\n\
         receptor.enlazar(0)\n",
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
        .expect_err("sin permiso de red enlazar debe fallar");
    assert_eq!(error.codigo, "E0701");
}

#[test]
fn socket_udp_enviar_sin_permiso_deberia_fallar_con_e0701() {
    let fuente = Fuente::nueva(
        "prueba.qz",
        "importar { SocketUdp } desde \"quetzal/red\"\n\
         SocketUdp emisor = nuevo SocketUdp()\n\
         emisor.enviar_a(\"127.0.0.1\", 9, \"hola\")\n",
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
        .expect_err("sin permiso de red enviar debe fallar");
    assert_eq!(error.codigo, "E0701");
}
