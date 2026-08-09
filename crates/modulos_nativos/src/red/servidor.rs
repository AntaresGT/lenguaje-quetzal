//! Servidor HTTP de `quetzal/red`: `ServidorHttp`, `Enrutador`, `Ruta`,
//! `PeticionEntrante`, `RespuestaSaliente` y `Continuacion`.
//!
//! Es el equivalente en Quetzal de Express 5, con dos diferencias de fondo:
//! los manejadores e interceptores son **funciones nombradas** (Quetzal no
//! tiene funciones anónimas) y toda la API está en español, incluidos los
//! métodos HTTP (`obtener`, `publicar`, `consultar`, ...).
//!
//! Cada conexión se atiende en un hilo que lee la petición y la envía al
//! hilo de la VM por el bucle de eventos ([`Mensaje::Solicitud`]); allí se
//! ejecuta la cadena de interceptores y manejadores, y la respuesta vuelve
//! al hilo de la conexión para escribirse en el socket.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::BufReader;
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::time::Duration;

use indexmap::IndexMap;
use maquina_virtual::{CargaNativa, Fallo, Mensaje, RegistroNativos, Valor, Vm, texto_de_valor};
use runtime::GuardianPermisos;

use crate::bits;
use crate::red::codigos;
use crate::red::formulario;
use crate::red::http;
use crate::red::metodos::{self, METODOS};
use crate::red::objetos::{
    bytes_de_carga, campo, campo_de_carga, campo_entero, campo_log, campo_texto, carga_de_bytes,
    error_permiso, error_red, error_tipo, id_receptor, instancia, jsn_de_pares, pares_de_carga,
    poner_campo, receptor, texto_de_carga,
};
use crate::util::{arg_texto, error, exigir_aridad};

pub(crate) const TIPO_SERVIDOR: &str = "ServidorHttp";
pub(crate) const TIPO_ENRUTADOR: &str = "Enrutador";
pub(crate) const TIPO_RUTA: &str = "Ruta";
pub(crate) const TIPO_PETICION: &str = "PeticionEntrante";
pub(crate) const TIPO_RESPUESTA: &str = "RespuestaSaliente";
pub(crate) const TIPO_CONTINUACION: &str = "Continuacion";
pub(crate) const TIPO_ERROR: &str = "ErrorHttp";

/// Servicio con el que el bucle de eventos identifica a los servidores.
const SERVICIO: &str = "servidor_http";

/// Tiempo máximo que una conexión puede quedarse sin enviar datos.
const LIMITE_CONEXION: Duration = Duration::from_secs(120);

/// Anfitrión por defecto: todas las interfaces IPv4 (Docker/Dokploy/Traefik).
const ANFITRION_POR_DEFECTO: &str = "0.0.0.0";

// ----- Estado del enrutado -----

/// A dónde lleva una capa del enrutador.
enum Destino {
    /// Uno o más manejadores/interceptores de Quetzal.
    Manejadores(Vec<Valor>),
    /// Un enrutador montado.
    Enrutador(i64),
    /// Un directorio servido como archivos estáticos.
    Estaticos(String),
}

/// Una capa registrada en un enrutador (ruta, `usar` o montaje).
struct Capa {
    /// Verbo HTTP que atiende; `None` atiende cualquiera.
    metodo: Option<String>,
    patron: crate::red::rutas::Patron,
    /// `true` si se registró con `usar` (coincide por prefijo).
    prefijo: bool,
    destino: Destino,
}

#[derive(Default)]
struct Enrutador {
    capas: Vec<Capa>,
    /// Manejadores de `parametro(nombre, manejador)`.
    parametros: Vec<(String, Valor)>,
}

struct Servidor {
    enrutador: i64,
    configuracion: IndexMap<String, Valor>,
    manejadores_error: Vec<Valor>,
    puerto: u16,
    /// Dirección en la que se abrió la escucha (`0.0.0.0`, `127.0.0.1`, …).
    anfitrion: String,
    escuchando: bool,
    detener: Option<Arc<AtomicBool>>,
}

/// Estado compartido de todos los servidores y enrutadores del programa.
#[derive(Clone)]
pub(crate) struct RegistroRed {
    enrutadores: Rc<RefCell<HashMap<i64, Enrutador>>>,
    servidores: Rc<RefCell<HashMap<i64, Servidor>>>,
    /// Servidor al que pertenece cada recurso del bucle de eventos.
    por_recurso: Rc<RefCell<HashMap<u64, i64>>>,
    siguiente: Rc<Cell<i64>>,
    guardian: Rc<GuardianPermisos>,
}

impl RegistroRed {
    fn nuevo(guardian: &Rc<GuardianPermisos>) -> Self {
        Self {
            enrutadores: Rc::new(RefCell::new(HashMap::new())),
            servidores: Rc::new(RefCell::new(HashMap::new())),
            por_recurso: Rc::new(RefCell::new(HashMap::new())),
            siguiente: Rc::new(Cell::new(1)),
            guardian: Rc::clone(guardian),
        }
    }

    fn nuevo_id(&self) -> i64 {
        let id = self.siguiente.get();
        self.siguiente.set(id + 1);
        id
    }

    fn crear_enrutador(&self) -> i64 {
        let id = self.nuevo_id();
        self.enrutadores
            .borrow_mut()
            .insert(id, Enrutador::default());
        id
    }

    fn crear_servidor(&self) -> i64 {
        let enrutador = self.crear_enrutador();
        let id = self.nuevo_id();
        self.servidores.borrow_mut().insert(
            id,
            Servidor {
                enrutador,
                configuracion: IndexMap::new(),
                manejadores_error: Vec::new(),
                puerto: 0,
                anfitrion: ANFITRION_POR_DEFECTO.to_string(),
                escuchando: false,
                detener: None,
            },
        );
        id
    }

    /// Enrutador raíz de un servidor, o el propio enrutador si el id ya lo es.
    fn enrutador_de(&self, funcion: &str, id: i64, es_servidor: bool) -> Result<i64, Fallo> {
        if !es_servidor {
            return Ok(id);
        }
        self.servidores
            .borrow()
            .get(&id)
            .map(|servidor| servidor.enrutador)
            .ok_or_else(|| error_red(format!("'{funcion}' no encontró el servidor")))
    }

    fn agregar_capa(&self, id_enrutador: i64, capa: Capa) {
        if let Some(enrutador) = self.enrutadores.borrow_mut().get_mut(&id_enrutador) {
            enrutador.capas.push(capa);
        }
    }
}

// ----- Instancias visibles desde Quetzal -----

fn instancia_servidor(id: i64) -> Valor {
    instancia(
        TIPO_SERVIDOR,
        vec![
            ("id", Valor::Entero(id)),
            ("texto", Valor::texto(format!("<ServidorHttp {id}>"))),
        ],
    )
}

fn instancia_enrutador(id: i64) -> Valor {
    instancia(
        TIPO_ENRUTADOR,
        vec![
            ("id", Valor::Entero(id)),
            ("texto", Valor::texto(format!("<Enrutador {id}>"))),
        ],
    )
}

fn instancia_ruta(id_enrutador: i64, camino: &str) -> Valor {
    instancia(
        TIPO_RUTA,
        vec![
            ("id", Valor::Entero(id_enrutador)),
            ("camino", Valor::texto(camino)),
            ("texto", Valor::texto(format!("<Ruta {camino}>"))),
        ],
    )
}

fn instancia_continuacion() -> Valor {
    instancia(
        TIPO_CONTINUACION,
        vec![
            ("estado", Valor::texto("pendiente")),
            ("mensaje", Valor::Nulo),
            ("texto", Valor::texto("<Continuacion>")),
        ],
    )
}

pub(crate) fn instancia_error(mensaje: &str, estado: i64) -> Valor {
    instancia(
        TIPO_ERROR,
        vec![
            ("mensaje", Valor::texto(mensaje)),
            ("estado", Valor::Entero(estado)),
            ("texto", Valor::texto(format!("<ErrorHttp {estado}: {mensaje}>"))),
        ],
    )
}

// ----- Registro de las funciones nativas -----

/// Registra todos los objetos del servidor.
pub(crate) fn registrar(registro: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    let red = RegistroRed::nuevo(guardian);

    for tipo in [
        TIPO_SERVIDOR,
        TIPO_ENRUTADOR,
        TIPO_RUTA,
        TIPO_PETICION,
        TIPO_RESPUESTA,
        TIPO_CONTINUACION,
        TIPO_ERROR,
    ] {
        registro.registrar_modulo(tipo);
    }

    registrar_constructores(registro, &red);
    registrar_rutas(registro, &red);
    registrar_configuracion(registro, &red);
    registrar_escucha(registro, &red);
    registrar_peticion(registro);
    registrar_respuesta(registro, guardian);
    registrar_continuacion(registro);
    registrar_error(registro);
}

fn registrar_constructores(registro: &mut RegistroNativos, red: &RegistroRed) {
    let crear_servidor = red.clone();
    registro.registrar_funcion(
        &format!("{TIPO_SERVIDOR}.constructor"),
        Box::new(move |argumentos| {
            if !argumentos.is_empty() {
                return Err(error(
                    "E0210",
                    "'ServidorHttp' se crea sin argumentos: nuevo ServidorHttp()".to_string(),
                ));
            }
            Ok(instancia_servidor(crear_servidor.crear_servidor()))
        }),
    );

    let crear_enrutador = red.clone();
    registro.registrar_funcion(
        &format!("{TIPO_ENRUTADOR}.constructor"),
        Box::new(move |argumentos| {
            if !argumentos.is_empty() {
                return Err(error(
                    "E0210",
                    "'Enrutador' se crea sin argumentos: nuevo Enrutador()".to_string(),
                ));
            }
            Ok(instancia_enrutador(crear_enrutador.crear_enrutador()))
        }),
    );
}

/// Registra los métodos de enrutado de `ServidorHttp`, `Enrutador` y `Ruta`.
fn registrar_rutas(registro: &mut RegistroNativos, red: &RegistroRed) {
    for (espanol, verbo) in METODOS {
        for (tipo, es_servidor) in [(TIPO_SERVIDOR, true), (TIPO_ENRUTADOR, false)] {
            let red = red.clone();
            let nombre = format!("{tipo}.{espanol}");
            let funcion = nombre.clone();
            let verbo = (*verbo).to_string();
            registro.registrar_funcion(
                &nombre,
                Box::new(move |argumentos| {
                    let id = id_receptor(&funcion, argumentos, tipo)?;
                    let enrutador = red.enrutador_de(&funcion, id, es_servidor)?;
                    let camino = arg_texto(&funcion, argumentos, 1)?.to_string();
                    let manejadores = manejadores_desde(&funcion, &argumentos[2..])?;
                    red.agregar_capa(
                        enrutador,
                        Capa {
                            metodo: Some(verbo.clone()),
                            patron: crate::red::rutas::Patron::analizar(&camino),
                            prefijo: false,
                            destino: Destino::Manejadores(manejadores),
                        },
                    );
                    Ok(argumentos[0].clone())
                }),
            );
        }

        // `Ruta`: el camino ya quedó fijado por `ruta(camino)`.
        let red = red.clone();
        let nombre = format!("{TIPO_RUTA}.{espanol}");
        let funcion = nombre.clone();
        let verbo = (*verbo).to_string();
        registro.registrar_funcion(
            &nombre,
            Box::new(move |argumentos| {
                let receptor_ruta = receptor(&funcion, argumentos, TIPO_RUTA)?;
                let enrutador = campo_entero(&receptor_ruta, "id");
                let camino = campo_texto(&receptor_ruta, "camino");
                let manejadores = manejadores_desde(&funcion, &argumentos[1..])?;
                red.agregar_capa(
                    enrutador,
                    Capa {
                        metodo: Some(verbo.clone()),
                        patron: crate::red::rutas::Patron::analizar(&camino),
                        prefijo: false,
                        destino: Destino::Manejadores(manejadores),
                    },
                );
                Ok(argumentos[0].clone())
            }),
        );
    }

    // `todos`: cualquier método HTTP sobre la misma ruta.
    for (tipo, es_servidor) in [(TIPO_SERVIDOR, true), (TIPO_ENRUTADOR, false)] {
        let red = red.clone();
        let funcion = format!("{tipo}.todos");
        let nombre = funcion.clone();
        registro.registrar_funcion(
            &nombre,
            Box::new(move |argumentos| {
                let id = id_receptor(&funcion, argumentos, tipo)?;
                let enrutador = red.enrutador_de(&funcion, id, es_servidor)?;
                let camino = arg_texto(&funcion, argumentos, 1)?.to_string();
                let manejadores = manejadores_desde(&funcion, &argumentos[2..])?;
                red.agregar_capa(
                    enrutador,
                    Capa {
                        metodo: None,
                        patron: crate::red::rutas::Patron::analizar(&camino),
                        prefijo: false,
                        destino: Destino::Manejadores(manejadores),
                    },
                );
                Ok(argumentos[0].clone())
            }),
        );
    }

    let red_ruta_todos = red.clone();
    registro.registrar_funcion(
        &format!("{TIPO_RUTA}.todos"),
        Box::new(move |argumentos| {
            const F: &str = "Ruta.todos";
            let receptor_ruta = receptor(F, argumentos, TIPO_RUTA)?;
            let enrutador = campo_entero(&receptor_ruta, "id");
            let camino = campo_texto(&receptor_ruta, "camino");
            let manejadores = manejadores_desde(F, &argumentos[1..])?;
            red_ruta_todos.agregar_capa(
                enrutador,
                Capa {
                    metodo: None,
                    patron: crate::red::rutas::Patron::analizar(&camino),
                    prefijo: false,
                    destino: Destino::Manejadores(manejadores),
                },
            );
            Ok(argumentos[0].clone())
        }),
    );

    registrar_usar(registro, red);
    registrar_auxiliares(registro, red);
}

/// `usar(...)`: interceptores globales y montaje de enrutadores.
fn registrar_usar(registro: &mut RegistroNativos, red: &RegistroRed) {
    for (tipo, es_servidor) in [(TIPO_SERVIDOR, true), (TIPO_ENRUTADOR, false)] {
        let red = red.clone();
        let funcion = format!("{tipo}.usar");
        let nombre = funcion.clone();
        registro.registrar_funcion(
            &nombre,
            Box::new(move |argumentos| {
                let id = id_receptor(&funcion, argumentos, tipo)?;
                let enrutador = red.enrutador_de(&funcion, id, es_servidor)?;
                if argumentos.len() < 2 {
                    return Err(error(
                        "E0210",
                        format!("'{funcion}' necesita al menos un interceptor o enrutador"),
                    ));
                }

                // El primer argumento puede ser la ruta de montaje.
                let (camino, resto) = match &argumentos[1] {
                    Valor::Texto(texto) => (texto.to_string(), &argumentos[2..]),
                    _ => ("/".to_string(), &argumentos[1..]),
                };
                if resto.is_empty() {
                    return Err(error(
                        "E0210",
                        format!("'{funcion}' necesita al menos un interceptor o enrutador"),
                    ));
                }

                for valor in resto {
                    let destino = match valor {
                        Valor::InstanciaNativa(datos) if &*datos.tipo == TIPO_ENRUTADOR => {
                            Destino::Enrutador(campo_entero(valor, "id"))
                        }
                        _ if valor.partes_callable().is_some() => {
                            Destino::Manejadores(vec![valor.clone()])
                        }
                        otro => {
                            return Err(error_tipo(format!(
                                "'{funcion}' esperaba una función o un Enrutador, pero recibió '{}'",
                                otro.nombre_tipo()
                            )));
                        }
                    };
                    red.agregar_capa(
                        enrutador,
                        Capa {
                            metodo: None,
                            patron: crate::red::rutas::Patron::analizar(&camino),
                            prefijo: true,
                            destino,
                        },
                    );
                }
                Ok(argumentos[0].clone())
            }),
        );
    }
}

/// `ruta`, `parametro`, `estaticos` y `manejar_errores`.
fn registrar_auxiliares(registro: &mut RegistroNativos, red: &RegistroRed) {
    for (tipo, es_servidor) in [(TIPO_SERVIDOR, true), (TIPO_ENRUTADOR, false)] {
        let red_ruta = red.clone();
        let funcion_ruta = format!("{tipo}.ruta");
        let nombre_ruta = funcion_ruta.clone();
        registro.registrar_funcion(
            &nombre_ruta,
            Box::new(move |argumentos| {
                exigir_aridad(&funcion_ruta, &argumentos[1..], 1)?;
                let id = id_receptor(&funcion_ruta, argumentos, tipo)?;
                let enrutador = red_ruta.enrutador_de(&funcion_ruta, id, es_servidor)?;
                let camino = arg_texto(&funcion_ruta, argumentos, 1)?;
                Ok(instancia_ruta(enrutador, camino))
            }),
        );

        let red_parametro = red.clone();
        let funcion_parametro = format!("{tipo}.parametro");
        let nombre_parametro = funcion_parametro.clone();
        registro.registrar_funcion(
            &nombre_parametro,
            Box::new(move |argumentos| {
                exigir_aridad(&funcion_parametro, &argumentos[1..], 2)?;
                let id = id_receptor(&funcion_parametro, argumentos, tipo)?;
                let enrutador = red_parametro.enrutador_de(&funcion_parametro, id, es_servidor)?;
                let nombre = arg_texto(&funcion_parametro, argumentos, 1)?.to_string();
                let manejador = exigir_funcion(&funcion_parametro, &argumentos[2])?;
                if let Some(destino) = red_parametro
                    .enrutadores
                    .borrow_mut()
                    .get_mut(&enrutador)
                {
                    destino.parametros.push((nombre, manejador));
                }
                Ok(argumentos[0].clone())
            }),
        );

        let red_estaticos = red.clone();
        let funcion_estaticos = format!("{tipo}.estaticos");
        let nombre_estaticos = funcion_estaticos.clone();
        registro.registrar_funcion(
            &nombre_estaticos,
            Box::new(move |argumentos| {
                let id = id_receptor(&funcion_estaticos, argumentos, tipo)?;
                let enrutador = red_estaticos.enrutador_de(&funcion_estaticos, id, es_servidor)?;
                let (camino, directorio) = match argumentos.len() {
                    2 => ("/".to_string(), arg_texto(&funcion_estaticos, argumentos, 1)?.to_string()),
                    3 => (
                        arg_texto(&funcion_estaticos, argumentos, 1)?.to_string(),
                        arg_texto(&funcion_estaticos, argumentos, 2)?.to_string(),
                    ),
                    _ => {
                        return Err(error(
                            "E0210",
                            format!("'{funcion_estaticos}' espera (directorio) o (ruta, directorio)"),
                        ));
                    }
                };
                red_estaticos.agregar_capa(
                    enrutador,
                    Capa {
                        metodo: None,
                        patron: crate::red::rutas::Patron::analizar(&camino),
                        prefijo: true,
                        destino: Destino::Estaticos(directorio),
                    },
                );
                Ok(argumentos[0].clone())
            }),
        );
    }

    let red_errores = red.clone();
    registro.registrar_funcion(
        &format!("{TIPO_SERVIDOR}.manejar_errores"),
        Box::new(move |argumentos| {
            const F: &str = "ServidorHttp.manejar_errores";
            exigir_aridad(F, &argumentos[1..], 1)?;
            let id = id_receptor(F, argumentos, TIPO_SERVIDOR)?;
            let manejador = exigir_funcion(F, &argumentos[1])?;
            if let Some(servidor) = red_errores.servidores.borrow_mut().get_mut(&id) {
                servidor.manejadores_error.push(manejador);
            }
            Ok(argumentos[0].clone())
        }),
    );

    let red_enrutador_raiz = red.clone();
    registro.registrar_funcion(
        &format!("{TIPO_SERVIDOR}.enrutador"),
        Box::new(move |argumentos| {
            const F: &str = "ServidorHttp.enrutador";
            exigir_aridad(F, &argumentos[1..], 0)?;
            let id = id_receptor(F, argumentos, TIPO_SERVIDOR)?;
            let enrutador = red_enrutador_raiz.enrutador_de(F, id, true)?;
            Ok(instancia_enrutador(enrutador))
        }),
    );
}

/// Ajustes del servidor (`configurar`, `habilitar`, ...).
fn registrar_configuracion(registro: &mut RegistroNativos, red: &RegistroRed) {
    let red_configurar = red.clone();
    registro.registrar_funcion(
        &format!("{TIPO_SERVIDOR}.configurar"),
        Box::new(move |argumentos| {
            const F: &str = "ServidorHttp.configurar";
            exigir_aridad(F, &argumentos[1..], 2)?;
            let id = id_receptor(F, argumentos, TIPO_SERVIDOR)?;
            let clave = arg_texto(F, argumentos, 1)?.to_string();
            if let Some(servidor) = red_configurar.servidores.borrow_mut().get_mut(&id) {
                servidor.configuracion.insert(clave, argumentos[2].clone());
            }
            Ok(argumentos[0].clone())
        }),
    );

    let red_leer = red.clone();
    registro.registrar_funcion(
        &format!("{TIPO_SERVIDOR}.configuracion"),
        Box::new(move |argumentos| {
            const F: &str = "ServidorHttp.configuracion";
            exigir_aridad(F, &argumentos[1..], 1)?;
            let id = id_receptor(F, argumentos, TIPO_SERVIDOR)?;
            let clave = arg_texto(F, argumentos, 1)?;
            Ok(red_leer
                .servidores
                .borrow()
                .get(&id)
                .and_then(|servidor| servidor.configuracion.get(clave).cloned())
                .unwrap_or(Valor::Nulo))
        }),
    );

    for (nombre, valor) in [("habilitar", true), ("deshabilitar", false)] {
        let red = red.clone();
        let funcion = format!("{TIPO_SERVIDOR}.{nombre}");
        let nombre_funcion = funcion.clone();
        registro.registrar_funcion(
            &nombre_funcion,
            Box::new(move |argumentos| {
                exigir_aridad(&funcion, &argumentos[1..], 1)?;
                let id = id_receptor(&funcion, argumentos, TIPO_SERVIDOR)?;
                let clave = arg_texto(&funcion, argumentos, 1)?.to_string();
                if let Some(servidor) = red.servidores.borrow_mut().get_mut(&id) {
                    servidor.configuracion.insert(clave, Valor::Log(valor));
                }
                Ok(argumentos[0].clone())
            }),
        );
    }

    for (nombre, esperado) in [("esta_habilitado", true), ("esta_deshabilitado", false)] {
        let red = red.clone();
        let funcion = format!("{TIPO_SERVIDOR}.{nombre}");
        let nombre_funcion = funcion.clone();
        registro.registrar_funcion(
            &nombre_funcion,
            Box::new(move |argumentos| {
                exigir_aridad(&funcion, &argumentos[1..], 1)?;
                let id = id_receptor(&funcion, argumentos, TIPO_SERVIDOR)?;
                let clave = arg_texto(&funcion, argumentos, 1)?;
                let activo = red
                    .servidores
                    .borrow()
                    .get(&id)
                    .and_then(|servidor| servidor.configuracion.get(clave).cloned())
                    .map(|valor| matches!(valor, Valor::Log(true)))
                    .unwrap_or(false);
                Ok(Valor::Log(activo == esperado))
            }),
        );
    }

    let red_puerto = red.clone();
    registro.registrar_funcion(
        &format!("{TIPO_SERVIDOR}.puerto"),
        Box::new(move |argumentos| {
            const F: &str = "ServidorHttp.puerto";
            exigir_aridad(F, &argumentos[1..], 0)?;
            let id = id_receptor(F, argumentos, TIPO_SERVIDOR)?;
            Ok(Valor::Entero(
                red_puerto
                    .servidores
                    .borrow()
                    .get(&id)
                    .map(|servidor| servidor.puerto as i64)
                    .unwrap_or(0),
            ))
        }),
    );

    let red_escuchando = red.clone();
    registro.registrar_funcion(
        &format!("{TIPO_SERVIDOR}.esta_escuchando"),
        Box::new(move |argumentos| {
            const F: &str = "ServidorHttp.esta_escuchando";
            exigir_aridad(F, &argumentos[1..], 0)?;
            let id = id_receptor(F, argumentos, TIPO_SERVIDOR)?;
            Ok(Valor::Log(
                red_escuchando
                    .servidores
                    .borrow()
                    .get(&id)
                    .map(|servidor| servidor.escuchando)
                    .unwrap_or(false),
            ))
        }),
    );
}

// ----- Escucha y ciclo de vida -----

fn registrar_escucha(registro: &mut RegistroNativos, red: &RegistroRed) {
    let red_escuchar = red.clone();
    registro.registrar_funcion_con_vm(
        &format!("{TIPO_SERVIDOR}.escuchar"),
        Box::new(move |vm: &mut Vm, argumentos: &[Valor]| {
            const F: &str = "ServidorHttp.escuchar";
            if !(2..=4).contains(&argumentos.len()) {
                return Err(error(
                    "E0210",
                    format!(
                        "'{F}' espera (puerto), (puerto, funcion), (puerto, anfitrion) \
                         o (puerto, anfitrion, funcion)"
                    ),
                ));
            }
            let id = id_receptor(F, argumentos, TIPO_SERVIDOR)?;
            let puerto = match &argumentos[1] {
                Valor::Entero(puerto) if (0..=65535).contains(puerto) => *puerto as u16,
                otro => {
                    return Err(error_tipo(format!(
                        "'{F}' esperaba un puerto entre 0 y 65535, pero recibió '{}'",
                        otro.nombre_tipo()
                    )));
                }
            };

            let (anfitrion, manejador_arranque) = parsear_args_escucha(F, argumentos)?;

            red_escuchar
                .guardian
                .verificar_red(&format!("escuchar en {anfitrion}:{puerto}"))
                .map_err(error_permiso)?;

            let escucha = TcpListener::bind((anfitrion.as_str(), puerto)).map_err(|fallo| {
                error_red(format!(
                    "no se pudo escuchar en {anfitrion}:{puerto}: {fallo}"
                ))
            })?;
            let puerto_real = escucha
                .local_addr()
                .map(|direccion| direccion.port())
                .unwrap_or(puerto);

            let id_recurso = vm.bucle().nuevo_id();
            let detener = Arc::new(AtomicBool::new(false));
            {
                let mut servidores = red_escuchar.servidores.borrow_mut();
                let Some(servidor) = servidores.get_mut(&id) else {
                    return Err(error_red(format!("'{F}' no encontró el servidor")));
                };
                if servidor.escuchando {
                    return Err(error_red(format!(
                        "'{F}': el servidor ya está escuchando en el puerto {}",
                        servidor.puerto
                    )));
                }
                servidor.puerto = puerto_real;
                servidor.anfitrion = anfitrion;
                servidor.escuchando = true;
                servidor.detener = Some(Arc::clone(&detener));
            }
            red_escuchar
                .por_recurso
                .borrow_mut()
                .insert(id_recurso, id);

            let manija = vm.bucle().manija();
            let bandera = Arc::clone(&detener);
            std::thread::spawn(move || {
                for conexion in escucha.incoming() {
                    if bandera.load(Ordering::Relaxed) {
                        break;
                    }
                    match conexion {
                        Ok(flujo) => {
                            let manija = manija.clone();
                            std::thread::spawn(move || {
                                atender_conexion(flujo, manija, id_recurso);
                            });
                        }
                        Err(_) => break,
                    }
                }
            });

            let red_despacho = red_escuchar.clone();
            vm.registrar_despachador(SERVICIO, move |vm, id_recurso, datos| {
                let id_servidor = red_despacho
                    .por_recurso
                    .borrow()
                    .get(&id_recurso)
                    .copied();
                match id_servidor {
                    Some(id) => atender_peticion(vm, &red_despacho, id, datos),
                    None => respuesta_simple(500, "no hay un servidor para esta petición"),
                }
            });
            vm.bucle().registrar_trabajo_activo();

            // Callback opcional de arranque, al estilo de `app.listen(puerto, fn)`.
            if let Some(manejador) = manejador_arranque {
                if let Some((funcion, entorno, esto)) = manejador.partes_callable() {
                    vm.llamar_funcion(funcion, entorno, Vec::new(), esto.cloned(), None)?;
                }
            }

            Ok(argumentos[0].clone())
        }),
    );

    let red_cerrar = red.clone();
    registro.registrar_funcion_con_vm(
        &format!("{TIPO_SERVIDOR}.cerrar"),
        Box::new(move |vm: &mut Vm, argumentos: &[Valor]| {
            const F: &str = "ServidorHttp.cerrar";
            exigir_aridad(F, &argumentos[1..], 0)?;
            let id = id_receptor(F, argumentos, TIPO_SERVIDOR)?;
            let (puerto, anfitrion) = {
                let mut servidores = red_cerrar.servidores.borrow_mut();
                let Some(servidor) = servidores.get_mut(&id) else {
                    return Ok(Valor::Log(false));
                };
                if !servidor.escuchando {
                    return Ok(Valor::Log(false));
                }
                servidor.escuchando = false;
                if let Some(detener) = servidor.detener.take() {
                    detener.store(true, Ordering::Relaxed);
                }
                (servidor.puerto, servidor.anfitrion.clone())
            };
            // Una conexión de cortesía despierta el hilo que acepta para que
            // vea la bandera de detención y termine.
            let host = host_para_despertar(&anfitrion);
            let _ = TcpStream::connect((host, puerto));
            vm.bucle().liberar_trabajo_activo();
            Ok(Valor::Log(true))
        }),
    );
}

/// Interpreta `(puerto[, anfitrion][, alArrancar])` tras el receptor.
fn parsear_args_escucha(
    funcion: &str,
    argumentos: &[Valor],
) -> Result<(String, Option<Valor>), Fallo> {
    let mut anfitrion = ANFITRION_POR_DEFECTO.to_string();
    let mut manejador: Option<Valor> = None;

    match argumentos.len() {
        2 => {}
        3 => match &argumentos[2] {
            valor if valor.partes_callable().is_some() => {
                manejador = Some(exigir_funcion(funcion, valor)?);
            }
            Valor::Texto(texto) if !texto.is_empty() => {
                anfitrion = texto.to_string();
            }
            Valor::Texto(_) => {
                return Err(error_tipo(format!(
                    "'{funcion}' esperaba un anfitrión no vacío"
                )));
            }
            otro => {
                return Err(error_tipo(format!(
                    "'{funcion}' esperaba un anfitrión (texto) o una función, pero recibió '{}'",
                    otro.nombre_tipo()
                )));
            }
        },
        4 => {
            anfitrion = match &argumentos[2] {
                Valor::Texto(texto) if !texto.is_empty() => texto.to_string(),
                Valor::Texto(_) => {
                    return Err(error_tipo(format!(
                        "'{funcion}' esperaba un anfitrión no vacío"
                    )));
                }
                otro => {
                    return Err(error_tipo(format!(
                        "'{funcion}' esperaba un anfitrión (texto) como segundo argumento, \
                         pero recibió '{}'",
                        otro.nombre_tipo()
                    )));
                }
            };
            manejador = Some(exigir_funcion(funcion, &argumentos[3])?);
        }
        _ => unreachable!("aridad ya validada en escuchar"),
    }

    Ok((anfitrion, manejador))
}

/// Host al que conectar para despertar el `accept` al cerrar.
///
/// Escuchar en `0.0.0.0` / `::` no admite conectar a esa misma dirección;
/// se usa el loopback correspondiente.
fn host_para_despertar(anfitrion: &str) -> &str {
    match anfitrion {
        "0.0.0.0" => "127.0.0.1",
        "::" => "::1",
        otro => otro,
    }
}

// ----- Hilo de conexión -----

fn atender_conexion(flujo: TcpStream, manija: maquina_virtual::ManijaBucle, id_recurso: u64) {
    let _ = flujo.set_read_timeout(Some(LIMITE_CONEXION));
    let _ = flujo.set_write_timeout(Some(LIMITE_CONEXION));
    let ip = flujo
        .peer_addr()
        .map(|direccion| direccion.ip().to_string())
        .unwrap_or_default();
    let mut escritura = match flujo.try_clone() {
        Ok(copia) => copia,
        Err(_) => return,
    };
    let mut lectura = BufReader::new(flujo);

    loop {
        let peticion = match http::leer_peticion(&mut lectura) {
            Ok(Some(peticion)) => peticion,
            Ok(None) => return,
            Err(mensaje) => {
                let _ = http::escribir_respuesta(
                    &mut escritura,
                    400,
                    codigos::razon(400),
                    &[("Content-Type".to_string(), "text/plain; charset=utf-8".to_string())],
                    mensaje.as_bytes(),
                    false,
                    true,
                );
                return;
            }
        };

        let persistente = peticion.conexion_persistente();
        let sin_cuerpo = peticion.metodo.eq_ignore_ascii_case("HEAD");
        let datos = CargaNativa::Mapa(vec![
            ("metodo".to_string(), CargaNativa::Texto(peticion.metodo.clone())),
            ("destino".to_string(), CargaNativa::Texto(peticion.destino.clone())),
            ("ruta".to_string(), CargaNativa::Texto(peticion.ruta.clone())),
            (
                "consulta".to_string(),
                CargaNativa::Texto(peticion.consulta.clone()),
            ),
            ("version".to_string(), CargaNativa::Texto(peticion.version.clone())),
            (
                "cabeceras".to_string(),
                CargaNativa::Mapa(
                    peticion
                        .cabeceras
                        .iter()
                        .map(|(clave, valor)| (clave.clone(), CargaNativa::Texto(valor.clone())))
                        .collect(),
                ),
            ),
            ("cuerpo".to_string(), carga_de_bytes(&peticion.cuerpo)),
            ("ip".to_string(), CargaNativa::Texto(ip.clone())),
        ]);

        let (emisor, receptor_respuesta) = channel();
        manija.enviar(Mensaje::Solicitud {
            servicio: SERVICIO.to_string(),
            id_recurso,
            datos,
            respuesta: emisor,
        });
        let Ok(respuesta) = receptor_respuesta.recv() else {
            return;
        };

        let estado = match campo_de_carga(&respuesta, "estado") {
            Some(CargaNativa::Entero(estado)) => *estado,
            _ => 200,
        };
        let mut cabeceras: Vec<(String, String)> = campo_de_carga(&respuesta, "cabeceras")
            .map(pares_de_carga)
            .unwrap_or_default();
        if let Some(CargaNativa::Lista(galletas)) = campo_de_carga(&respuesta, "galletas") {
            for galleta in galletas {
                cabeceras.push(("Set-Cookie".to_string(), texto_de_carga(galleta)));
            }
        }
        let cuerpo = campo_de_carga(&respuesta, "cuerpo")
            .map(bytes_de_carga)
            .unwrap_or_default();

        let escrito = http::escribir_respuesta(
            &mut escritura,
            estado,
            codigos::razon(estado),
            &cabeceras,
            &cuerpo,
            persistente,
            !sin_cuerpo,
        );
        if escrito.is_err() || !persistente {
            return;
        }
    }
}

fn respuesta_simple(estado: i64, mensaje: &str) -> CargaNativa {
    CargaNativa::Mapa(vec![
        ("estado".to_string(), CargaNativa::Entero(estado)),
        (
            "cabeceras".to_string(),
            CargaNativa::Mapa(vec![(
                "Content-Type".to_string(),
                CargaNativa::Texto("text/plain; charset=utf-8".to_string()),
            )]),
        ),
        ("cuerpo".to_string(), carga_de_bytes(mensaje.as_bytes())),
    ])
}

// ----- Atención de la petición en el hilo de la VM -----

/// Qué debe hacer el enrutador tras ejecutar un manejador.
enum Control {
    /// Seguir con la siguiente capa.
    Siguiente,
    /// Saltar el resto de manejadores de esta ruta.
    SiguienteRuta,
    /// La respuesta ya está lista.
    Terminado,
}

fn atender_peticion(
    vm: &mut Vm,
    red: &RegistroRed,
    id_servidor: i64,
    datos: CargaNativa,
) -> CargaNativa {
    let peticion = construir_peticion(&datos);
    let respuesta = construir_respuesta();

    let (enrutador, manejadores_error) = {
        let servidores = red.servidores.borrow();
        match servidores.get(&id_servidor) {
            Some(servidor) => (servidor.enrutador, servidor.manejadores_error.clone()),
            None => return respuesta_simple(500, "el servidor ya no existe"),
        }
    };

    let ruta = campo_texto(&peticion, "ruta");
    let segmentos = crate::red::rutas::segmentos_de(&ruta);
    let resultado = ejecutar_enrutador(vm, red, enrutador, &peticion, &respuesta, &segmentos, "");

    match resultado {
        Ok(Control::Terminado) => {}
        Ok(_) if campo_log(&respuesta, "terminada") => {}
        Ok(_) => {
            let metodo = campo_texto(&peticion, "metodo");
            responder_texto(
                &respuesta,
                404,
                &format!("no se encontró la ruta '{metodo} {ruta}'"),
            );
        }
        Err(fallo) => {
            let mensaje = mensaje_de_fallo(&fallo);
            let manejado = ejecutar_manejadores_error(
                vm,
                &manejadores_error,
                &mensaje,
                &peticion,
                &respuesta,
            );
            if !manejado && !campo_log(&respuesta, "terminada") {
                responder_texto(&respuesta, 500, &mensaje);
            }
        }
    }

    serializar_respuesta(&respuesta)
}

fn mensaje_de_fallo(fallo: &Fallo) -> String {
    match fallo {
        Fallo::Excepcion(datos) => datos.mensaje.clone(),
        Fallo::Error(error) => error.mensaje.clone(),
    }
}

fn ejecutar_manejadores_error(
    vm: &mut Vm,
    manejadores: &[Valor],
    mensaje: &str,
    peticion: &Valor,
    respuesta: &Valor,
) -> bool {
    for manejador in manejadores {
        let continuacion = instancia_continuacion();
        let error = instancia_error(mensaje, 500);
        let argumentos = vec![
            error,
            peticion.clone(),
            respuesta.clone(),
            continuacion.clone(),
        ];
        let Some((funcion, entorno, esto)) = manejador.partes_callable() else {
            continue;
        };
        let esperados = funcion.parametros.len();
        let argumentos = argumentos.into_iter().take(esperados.max(1)).collect();
        if vm
            .llamar_funcion(funcion, entorno, argumentos, esto.cloned(), None)
            .is_err()
        {
            continue;
        }
        if campo_log(respuesta, "terminada") {
            return true;
        }
        if campo_texto(&continuacion, "estado") == "pendiente" {
            return true;
        }
    }
    false
}

/// Ejecuta las capas de un enrutador sobre la ruta restante.
fn ejecutar_enrutador(
    vm: &mut Vm,
    red: &RegistroRed,
    id_enrutador: i64,
    peticion: &Valor,
    respuesta: &Valor,
    segmentos: &[String],
    base: &str,
) -> Result<Control, Fallo> {
    let metodo = campo_texto(peticion, "metodo");
    let cantidad = red
        .enrutadores
        .borrow()
        .get(&id_enrutador)
        .map(|enrutador| enrutador.capas.len())
        .unwrap_or(0);

    for indice in 0..cantidad {
        // Los datos se copian por capa: un manejador puede registrar rutas
        // nuevas mientras esta petición se atiende.
        let (metodo_capa, coincidencia, destino) = {
            let enrutadores = red.enrutadores.borrow();
            let Some(enrutador) = enrutadores.get(&id_enrutador) else {
                break;
            };
            let Some(capa) = enrutador.capas.get(indice) else {
                break;
            };
            let coincidencia = capa.patron.coincide(segmentos, capa.prefijo);
            let destino = match &capa.destino {
                Destino::Manejadores(manejadores) => DestinoCopia::Manejadores(manejadores.clone()),
                Destino::Enrutador(id) => DestinoCopia::Enrutador(*id),
                Destino::Estaticos(directorio) => DestinoCopia::Estaticos(directorio.clone()),
            };
            (capa.metodo.clone(), coincidencia, destino)
        };

        let Some(coincidencia) = coincidencia else {
            continue;
        };
        if let Some(esperado) = &metodo_capa
            && !esperado.eq_ignore_ascii_case(&metodo)
            && !(esperado == "GET" && metodo.eq_ignore_ascii_case("HEAD"))
        {
            continue;
        }

        aplicar_parametros(peticion, &coincidencia.parametros);
        ejecutar_manejadores_parametro(vm, red, id_enrutador, peticion, respuesta, &coincidencia)?;

        match destino {
            DestinoCopia::Manejadores(manejadores) => {
                match ejecutar_cadena(vm, &manejadores, peticion, respuesta)? {
                    Control::Terminado => return Ok(Control::Terminado),
                    Control::SiguienteRuta | Control::Siguiente => continue,
                }
            }
            DestinoCopia::Enrutador(hijo) => {
                let restantes: Vec<String> = segmentos[coincidencia.consumidos..].to_vec();
                let nueva_base = unir_base(base, &segmentos[..coincidencia.consumidos]);
                let base_anterior = campo_texto(peticion, "ruta_base");
                poner_campo(peticion, "ruta_base", Valor::texto(&nueva_base));
                let control = ejecutar_enrutador(
                    vm,
                    red,
                    hijo,
                    peticion,
                    respuesta,
                    &restantes,
                    &nueva_base,
                )?;
                if let Control::Terminado = control {
                    return Ok(Control::Terminado);
                }
                poner_campo(peticion, "ruta_base", Valor::texto(base_anterior));
            }
            DestinoCopia::Estaticos(directorio) => {
                let restantes: Vec<String> = segmentos[coincidencia.consumidos..].to_vec();
                if servir_estatico(red, &directorio, &restantes, respuesta) {
                    return Ok(Control::Terminado);
                }
            }
        }
    }

    Ok(Control::Siguiente)
}

/// Copia de un destino para no sostener el préstamo del registro mientras se
/// ejecuta código de Quetzal.
enum DestinoCopia {
    Manejadores(Vec<Valor>),
    Enrutador(i64),
    Estaticos(String),
}

fn unir_base(base: &str, consumidos: &[String]) -> String {
    let mut nueva = base.trim_end_matches('/').to_string();
    for segmento in consumidos {
        nueva.push('/');
        nueva.push_str(segmento);
    }
    if nueva.is_empty() {
        "/".to_string()
    } else {
        nueva
    }
}

fn aplicar_parametros(peticion: &Valor, parametros: &[(String, String)]) {
    if parametros.is_empty() {
        return;
    }
    if let Valor::Jsn(mapa) = campo(peticion, "parametros") {
        let mut mapa = mapa.borrow_mut();
        for (nombre, valor) in parametros {
            mapa.insert(nombre.clone(), Valor::texto(http::decodificar(valor)));
        }
    }
}

fn ejecutar_manejadores_parametro(
    vm: &mut Vm,
    red: &RegistroRed,
    id_enrutador: i64,
    peticion: &Valor,
    respuesta: &Valor,
    coincidencia: &crate::red::rutas::Coincidencia,
) -> Result<(), Fallo> {
    let manejadores: Vec<(String, Valor)> = red
        .enrutadores
        .borrow()
        .get(&id_enrutador)
        .map(|enrutador| enrutador.parametros.clone())
        .unwrap_or_default();
    if manejadores.is_empty() {
        return Ok(());
    }

    for (nombre, valor) in &coincidencia.parametros {
        for (esperado, manejador) in &manejadores {
            if esperado != nombre {
                continue;
            }
            let Some((funcion, entorno, esto)) = manejador.partes_callable() else {
                continue;
            };
            let continuacion = instancia_continuacion();
            let argumentos = vec![
                peticion.clone(),
                respuesta.clone(),
                continuacion,
                Valor::texto(valor),
            ];
            let esperados = funcion.parametros.len().min(argumentos.len());
            let argumentos = argumentos.into_iter().take(esperados).collect();
            vm.llamar_funcion(funcion, entorno, argumentos, esto.cloned(), None)?;
        }
    }
    Ok(())
}

/// Ejecuta los manejadores de una ruta en orden, respetando lo que cada
/// interceptor haga con su `Continuacion`.
fn ejecutar_cadena(
    vm: &mut Vm,
    manejadores: &[Valor],
    peticion: &Valor,
    respuesta: &Valor,
) -> Result<Control, Fallo> {
    for manejador in manejadores {
        let Some((funcion, entorno, esto)) = manejador.partes_callable() else {
            return Err(error_tipo(
                "los manejadores de una ruta deben ser funciones declaradas en Quetzal",
            ));
        };
        let aridad = funcion.parametros.len();
        let continuacion = instancia_continuacion();
        let mut argumentos = vec![
            peticion.clone(),
            respuesta.clone(),
            continuacion.clone(),
        ];
        argumentos.truncate(aridad.min(3));
        vm.llamar_funcion(funcion, entorno, argumentos, esto.cloned(), None)?;

        if aridad < 3 {
            // Manejador final: no hay continuación, la ruta queda atendida.
            return Ok(Control::Terminado);
        }
        match campo_texto(&continuacion, "estado").as_str() {
            "siguiente" => continue,
            "ruta" => return Ok(Control::SiguienteRuta),
            "error" => {
                let mensaje = campo_texto(&continuacion, "mensaje");
                return Err(error_red(mensaje));
            }
            // No llamó a `siguiente`: el interceptor decidió responder.
            _ => return Ok(Control::Terminado),
        }
    }
    // Todos pidieron continuar: la búsqueda sigue en la próxima capa.
    Ok(Control::Siguiente)
}

fn servir_estatico(
    red: &RegistroRed,
    directorio: &str,
    restantes: &[String],
    respuesta: &Valor,
) -> bool {
    let mut ruta = std::path::PathBuf::from(directorio);
    for segmento in restantes {
        let decodificado = http::decodificar(segmento);
        if decodificado.contains("..") || decodificado.contains('\\') {
            return false;
        }
        ruta.push(decodificado);
    }
    if restantes.is_empty() {
        ruta.push("index.html");
    }

    let Ok(resuelta) = red.guardian.verificar_lectura(&ruta.to_string_lossy()) else {
        return false;
    };
    if resuelta.is_dir() {
        return false;
    }
    let Ok(bytes) = std::fs::read(&resuelta) else {
        return false;
    };
    let extension = resuelta
        .extension()
        .map(|extension| extension.to_string_lossy().to_string())
        .unwrap_or_default();
    poner_cabecera(respuesta, "Content-Type", http::tipo_por_extension(&extension));
    poner_campo(respuesta, "estado", Valor::Entero(200));
    poner_campo(respuesta, "cuerpo", bits::instancia(&bytes));
    poner_campo(respuesta, "terminada", Valor::Log(true));
    true
}

// ----- Construcción de la petición y la respuesta -----

fn construir_peticion(datos: &CargaNativa) -> Valor {
    let texto_campo = |nombre: &str| {
        campo_de_carga(datos, nombre)
            .map(texto_de_carga)
            .unwrap_or_default()
    };

    let metodo = texto_campo("metodo").to_ascii_uppercase();
    let destino = texto_campo("destino");
    let ruta = http::decodificar(&texto_campo("ruta"));
    let consulta = texto_campo("consulta");
    let cabeceras = campo_de_carga(datos, "cabeceras")
        .map(pares_de_carga)
        .unwrap_or_default();
    let cuerpo = campo_de_carga(datos, "cuerpo")
        .map(bytes_de_carga)
        .unwrap_or_default();

    let buscar_cabecera = |nombre: &str| {
        cabeceras
            .iter()
            .find(|(clave, _)| clave.eq_ignore_ascii_case(nombre))
            .map(|(_, valor)| valor.clone())
    };
    let tipo_contenido = buscar_cabecera("content-type").unwrap_or_default();
    let anfitrion = buscar_cabecera("host").unwrap_or_default();
    let galletas = buscar_cabecera("cookie")
        .map(|valor| http::analizar_galletas(&valor))
        .unwrap_or_default();

    let texto_cuerpo = String::from_utf8_lossy(&cuerpo).to_string();
    let cuerpo_analizado = analizar_cuerpo(&tipo_contenido, &texto_cuerpo, &cuerpo);
    let disposicion = buscar_cabecera("content-disposition").unwrap_or_default();
    let nombre_archivo = formulario::nombre_en_disposicion(&disposicion);

    instancia(
        TIPO_PETICION,
        vec![
            ("metodo", Valor::texto(&metodo)),
            ("metodo_espanol", Valor::texto(metodos::espanol(&metodo))),
            ("url", Valor::texto(&destino)),
            ("url_original", Valor::texto(&destino)),
            ("ruta", Valor::texto(&ruta)),
            ("ruta_base", Valor::texto("")),
            ("parametros", Valor::jsn(IndexMap::new())),
            ("consulta", jsn_de_pares(http::analizar_consulta(&consulta))),
            ("cabeceras", jsn_de_pares(cabeceras)),
            ("galletas", jsn_de_pares(galletas)),
            ("cuerpo", cuerpo_analizado),
            ("cuerpo_texto", Valor::texto(&texto_cuerpo)),
            ("cuerpo_bits", bits::instancia(&cuerpo)),
            ("tipo_contenido", Valor::texto(&tipo_contenido)),
            (
                "nombre_archivo",
                match &nombre_archivo {
                    Some(nombre) => Valor::texto(nombre),
                    None => Valor::Nulo,
                },
            ),
            ("protocolo", Valor::texto("http")),
            ("ip", Valor::texto(texto_campo("ip"))),
            ("anfitrion", Valor::texto(&anfitrion)),
            ("version", Valor::texto(texto_campo("version"))),
            ("locales", Valor::jsn(IndexMap::new())),
            ("es_seguro", Valor::Log(metodos::es_seguro(&metodo))),
            (
                "es_idempotente",
                Valor::Log(metodos::es_idempotente(&metodo)),
            ),
            (
                "texto",
                Valor::texto(format!("<PeticionEntrante {metodo} {ruta}>")),
            ),
        ],
    )
}

/// Interpreta el cuerpo según el tipo de contenido, como hacen los
/// interceptores `express.json()` y `express.urlencoded()` incorporados.
fn analizar_cuerpo(tipo_contenido: &str, texto: &str, bytes: &[u8]) -> Valor {
    let tipo = tipo_contenido.to_ascii_lowercase();
    if bytes.is_empty() {
        return Valor::Nulo;
    }
    // La frontera distingue mayúsculas: se lee del valor original.
    if let Some(frontera) = formulario::frontera_de_tipo(tipo_contenido) {
        return formulario::desde_multipart(bytes, &frontera);
    }
    if !tipo.is_empty() && !tipo.starts_with("text/") && !es_tipo_textual(&tipo) {
        return bits::instancia(bytes);
    }
    if texto.is_empty() {
        return Valor::Nulo;
    }
    if tipo.contains("json") {
        return match serde_json::from_str::<serde_json::Value>(texto) {
            Ok(json) => maquina_virtual::valores::json_a_valor(&json),
            Err(_) => Valor::texto(texto),
        };
    }
    if tipo.contains("x-www-form-urlencoded") {
        return jsn_de_pares(http::analizar_consulta(texto));
    }
    Valor::texto(texto)
}

/// Tipos que, sin ser `text/*`, traen texto y se analizan como tal.
fn es_tipo_textual(tipo: &str) -> bool {
    tipo.contains("json") || tipo.contains("x-www-form-urlencoded") || tipo.contains("xml")
}

fn construir_respuesta() -> Valor {
    instancia(
        TIPO_RESPUESTA,
        vec![
            ("estado", Valor::Entero(200)),
            ("cabeceras", Valor::jsn(IndexMap::new())),
            ("galletas", Valor::lista(Vec::new())),
            ("cuerpo", Valor::texto("")),
            ("terminada", Valor::Log(false)),
            ("locales", Valor::jsn(IndexMap::new())),
            ("texto", Valor::texto("<RespuestaSaliente>")),
        ],
    )
}

fn poner_cabecera(respuesta: &Valor, nombre: &str, valor: impl Into<String>) {
    if let Valor::Jsn(mapa) = campo(respuesta, "cabeceras") {
        mapa.borrow_mut()
            .insert(nombre.to_string(), Valor::texto(valor.into()));
    }
}

fn tiene_cabecera(respuesta: &Valor, nombre: &str) -> bool {
    match campo(respuesta, "cabeceras") {
        Valor::Jsn(mapa) => mapa
            .borrow()
            .keys()
            .any(|clave| clave.eq_ignore_ascii_case(nombre)),
        _ => false,
    }
}

/// Cabeceras estándar de un cuerpo binario en la respuesta: tipo MIME y, si
/// hay nombre, `Content-Disposition` con `filename`. El jsn opcional de
/// `enviar` puede forzarlas con `tipo_contenido`, `nombre_archivo` y
/// `disposicion`.
fn cabeceras_binarias(
    respuesta: &Valor,
    opciones: &Valor,
    tipo_por_omision: &str,
    nombre_por_omision: Option<&str>,
) {
    let opcion = |clave: &str| match opciones {
        Valor::Jsn(mapa) => mapa.borrow().get(clave).and_then(|valor| match valor {
            Valor::Nulo => None,
            otro => Some(texto_de_valor(otro)),
        }),
        _ => None,
    };

    match opcion("tipo_contenido") {
        Some(tipo) => poner_cabecera(respuesta, "Content-Type", http::tipo_mime(&tipo)),
        None if !tiene_cabecera(respuesta, "content-type") => {
            poner_cabecera(respuesta, "Content-Type", tipo_por_omision)
        }
        None => {}
    }

    if tiene_cabecera(respuesta, "content-disposition") {
        return;
    }
    let nombre = opcion("nombre_archivo").or_else(|| nombre_por_omision.map(str::to_string));
    let disposicion = match opcion("disposicion").as_deref() {
        Some("inline") | Some("en_linea") => Some("inline"),
        Some(_) => Some("attachment"),
        None if nombre.is_some() => Some("attachment"),
        None => None,
    };
    if let Some(disposicion) = disposicion {
        let valor = match &nombre {
            Some(nombre) => format!(
                "{disposicion}; filename=\"{}\"",
                nombre.replace('\\', "\\\\").replace('"', "\\\"")
            ),
            None => disposicion.to_string(),
        };
        poner_cabecera(respuesta, "Content-Disposition", valor);
    }
}

fn responder_texto(respuesta: &Valor, estado: i64, mensaje: &str) {
    poner_campo(respuesta, "estado", Valor::Entero(estado));
    poner_cabecera(respuesta, "Content-Type", "text/plain; charset=utf-8");
    poner_campo(respuesta, "cuerpo", Valor::texto(mensaje));
    poner_campo(respuesta, "terminada", Valor::Log(true));
}

fn serializar_respuesta(respuesta: &Valor) -> CargaNativa {
    let estado = campo_entero(respuesta, "estado");
    let cuerpo = bytes_del_cuerpo(&campo(respuesta, "cuerpo"));
    let mut cabeceras: Vec<(String, CargaNativa)> = match campo(respuesta, "cabeceras") {
        Valor::Jsn(mapa) => mapa
            .borrow()
            .iter()
            .map(|(clave, valor)| (clave.clone(), CargaNativa::Texto(texto_de_valor(valor))))
            .collect(),
        _ => Vec::new(),
    };
    if !cabeceras
        .iter()
        .any(|(clave, _)| clave.eq_ignore_ascii_case("content-type"))
        && !cuerpo.is_empty()
    {
        cabeceras.push((
            "Content-Type".to_string(),
            CargaNativa::Texto("text/plain; charset=utf-8".to_string()),
        ));
    }

    let galletas = match campo(respuesta, "galletas") {
        Valor::Lista(lista) => CargaNativa::Lista(
            lista
                .borrow()
                .iter()
                .map(|valor| CargaNativa::Texto(texto_de_valor(valor)))
                .collect(),
        ),
        _ => CargaNativa::Lista(Vec::new()),
    };

    CargaNativa::Mapa(vec![
        ("estado".to_string(), CargaNativa::Entero(estado)),
        ("cabeceras".to_string(), CargaNativa::Mapa(cabeceras)),
        ("galletas".to_string(), galletas),
        ("cuerpo".to_string(), carga_de_bytes(&cuerpo)),
    ])
}

/// Bytes de un cuerpo, que puede ser texto, `Bits` o un valor cualquiera.
fn bytes_del_cuerpo(cuerpo: &Valor) -> Vec<u8> {
    match cuerpo {
        Valor::Texto(texto) => texto.as_bytes().to_vec(),
        Valor::Nulo => Vec::new(),
        Valor::InstanciaNativa(datos) if &*datos.tipo == "Bits" => {
            bits::arg_bits("RespuestaSaliente.enviar", std::slice::from_ref(cuerpo), 0)
                .unwrap_or_default()
        }
        otro => texto_de_valor(otro).into_bytes(),
    }
}

// ----- Métodos de PeticionEntrante -----

fn registrar_peticion(registro: &mut RegistroNativos) {
    for (metodo, campo_datos) in [
        ("metodo", "metodo"),
        ("metodo_espanol", "metodo_espanol"),
        ("url", "url"),
        ("url_original", "url_original"),
        ("ruta", "ruta"),
        ("ruta_base", "ruta_base"),
        ("parametros", "parametros"),
        ("consulta", "consulta"),
        ("cabeceras", "cabeceras"),
        ("galletas", "galletas"),
        ("cuerpo", "cuerpo"),
        ("cuerpo_texto", "cuerpo_texto"),
        ("cuerpo_bits", "cuerpo_bits"),
        ("tipo_contenido", "tipo_contenido"),
        ("nombre_archivo", "nombre_archivo"),
        ("protocolo", "protocolo"),
        ("ip", "ip"),
        ("anfitrion", "anfitrion"),
        ("version", "version"),
        ("locales", "locales"),
        ("es_seguro", "es_seguro"),
        ("es_idempotente", "es_idempotente"),
    ] {
        lector(registro, TIPO_PETICION, metodo, campo_datos);
    }

    registro.registrar_funcion(
        &format!("{TIPO_PETICION}.parametro"),
        Box::new(move |argumentos| {
            const F: &str = "PeticionEntrante.parametro";
            exigir_aridad(F, &argumentos[1..], 1)?;
            let peticion = receptor(F, argumentos, TIPO_PETICION)?;
            let nombre = arg_texto(F, argumentos, 1)?;
            Ok(buscar_en_jsn(&campo(&peticion, "parametros"), nombre))
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_PETICION}.consulta_valor"),
        Box::new(move |argumentos| {
            const F: &str = "PeticionEntrante.consulta_valor";
            exigir_aridad(F, &argumentos[1..], 1)?;
            let peticion = receptor(F, argumentos, TIPO_PETICION)?;
            let nombre = arg_texto(F, argumentos, 1)?;
            Ok(buscar_en_jsn(&campo(&peticion, "consulta"), nombre))
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_PETICION}.cabecera"),
        Box::new(move |argumentos| {
            const F: &str = "PeticionEntrante.cabecera";
            exigir_aridad(F, &argumentos[1..], 1)?;
            let peticion = receptor(F, argumentos, TIPO_PETICION)?;
            let nombre = arg_texto(F, argumentos, 1)?;
            Ok(
                crate::red::objetos::buscar_sin_caso(&campo(&peticion, "cabeceras"), nombre)
                    .unwrap_or(Valor::Nulo),
            )
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_PETICION}.galleta"),
        Box::new(move |argumentos| {
            const F: &str = "PeticionEntrante.galleta";
            exigir_aridad(F, &argumentos[1..], 1)?;
            let peticion = receptor(F, argumentos, TIPO_PETICION)?;
            let nombre = arg_texto(F, argumentos, 1)?;
            Ok(buscar_en_jsn(&campo(&peticion, "galletas"), nombre))
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_PETICION}.tipo_es"),
        Box::new(move |argumentos| {
            const F: &str = "PeticionEntrante.tipo_es";
            exigir_aridad(F, &argumentos[1..], 1)?;
            let peticion = receptor(F, argumentos, TIPO_PETICION)?;
            let esperado = http::tipo_mime(arg_texto(F, argumentos, 1)?);
            let esperado = esperado.split(';').next().unwrap_or("").to_string();
            let actual = match crate::red::objetos::buscar_sin_caso(
                &campo(&peticion, "cabeceras"),
                "content-type",
            ) {
                Some(valor) => texto_de_valor(&valor),
                None => String::new(),
            };
            Ok(Valor::Log(
                actual.to_ascii_lowercase().contains(&esperado.to_ascii_lowercase()),
            ))
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_PETICION}.acepta"),
        Box::new(move |argumentos| {
            const F: &str = "PeticionEntrante.acepta";
            exigir_aridad(F, &argumentos[1..], 1)?;
            let peticion = receptor(F, argumentos, TIPO_PETICION)?;
            let esperado = http::tipo_mime(arg_texto(F, argumentos, 1)?);
            let esperado = esperado.split(';').next().unwrap_or("").to_ascii_lowercase();
            let acepta = match crate::red::objetos::buscar_sin_caso(
                &campo(&peticion, "cabeceras"),
                "accept",
            ) {
                Some(valor) => texto_de_valor(&valor).to_ascii_lowercase(),
                None => String::new(),
            };
            Ok(Valor::Log(
                acepta.is_empty() || acepta.contains("*/*") || acepta.contains(&esperado),
            ))
        }),
    );
}

fn buscar_en_jsn(valor: &Valor, clave: &str) -> Valor {
    match valor {
        Valor::Jsn(mapa) => mapa.borrow().get(clave).cloned().unwrap_or(Valor::Nulo),
        _ => Valor::Nulo,
    }
}

/// Método que solo devuelve un campo guardado en la instancia.
fn lector(
    registro: &mut RegistroNativos,
    tipo: &'static str,
    metodo: &'static str,
    nombre_campo: &'static str,
) {
    let funcion = format!("{tipo}.{metodo}");
    registro.registrar_funcion(
        &funcion.clone(),
        Box::new(move |argumentos| {
            exigir_aridad(&funcion, &argumentos[1..], 0)?;
            let valor = receptor(&funcion, argumentos, tipo)?;
            Ok(campo(&valor, nombre_campo))
        }),
    );
}

// ----- Métodos de RespuestaSaliente -----

fn registrar_respuesta(registro: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    registro.registrar_funcion(
        &format!("{TIPO_RESPUESTA}.estado"),
        Box::new(move |argumentos| {
            const F: &str = "RespuestaSaliente.estado";
            let respuesta = receptor(F, argumentos, TIPO_RESPUESTA)?;
            match argumentos.len() {
                1 => Ok(campo(&respuesta, "estado")),
                2 => {
                    let estado = exigir_estado(F, &argumentos[1])?;
                    poner_campo(&respuesta, "estado", Valor::Entero(estado));
                    Ok(argumentos[0].clone())
                }
                _ => Err(error(
                    "E0210",
                    format!("'{F}' espera () para leer o (codigo) para asignar"),
                )),
            }
        }),
    );

    let guardian_enviar = Rc::clone(guardian);
    registro.registrar_funcion(
        &format!("{TIPO_RESPUESTA}.enviar"),
        Box::new(move |argumentos| {
            const F: &str = "RespuestaSaliente.enviar";
            if argumentos.len() < 2 || argumentos.len() > 3 {
                return Err(error(
                    "E0210",
                    format!("'{F}' espera (cuerpo) o (cuerpo, opciones)"),
                ));
            }
            let respuesta = receptor(F, argumentos, TIPO_RESPUESTA)?;
            let opciones = argumentos.get(2).cloned().unwrap_or(Valor::Nulo);
            let cuerpo = argumentos[1].clone();

            let cuerpo = if formulario::es_formulario(&cuerpo) {
                let (frontera, bytes) = formulario::cuerpo_multipart(&cuerpo)?;
                poner_cabecera(
                    &respuesta,
                    "Content-Type",
                    format!("multipart/form-data; boundary={frontera}"),
                );
                bits::instancia(&bytes)
            } else if formulario::es_archivo(&cuerpo) {
                let (bytes, nombre, tipo) =
                    formulario::contenido_de_archivo(F, &guardian_enviar, &cuerpo)?;
                cabeceras_binarias(&respuesta, &opciones, &tipo, Some(&nombre));
                bits::instancia(&bytes)
            } else if formulario::es_bits(&cuerpo) {
                cabeceras_binarias(&respuesta, &opciones, "application/octet-stream", None);
                cuerpo
            } else {
                if !tiene_cabecera(&respuesta, "content-type") {
                    let tipo = match &cuerpo {
                        Valor::Jsn(_) | Valor::Lista(_) => "application/json; charset=utf-8",
                        _ => "text/html; charset=utf-8",
                    };
                    poner_cabecera(&respuesta, "Content-Type", tipo);
                }
                match &cuerpo {
                    Valor::Jsn(_) | Valor::Lista(_) => {
                        Valor::texto(maquina_virtual::valores::jsn_a_texto(&cuerpo, false))
                    }
                    otro => otro.clone(),
                }
            };

            poner_campo(&respuesta, "cuerpo", cuerpo);
            poner_campo(&respuesta, "terminada", Valor::Log(true));
            Ok(argumentos[0].clone())
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_RESPUESTA}.jsn"),
        Box::new(move |argumentos| {
            const F: &str = "RespuestaSaliente.jsn";
            exigir_aridad(F, &argumentos[1..], 1)?;
            let respuesta = receptor(F, argumentos, TIPO_RESPUESTA)?;
            poner_cabecera(&respuesta, "Content-Type", "application/json; charset=utf-8");
            poner_campo(
                &respuesta,
                "cuerpo",
                Valor::texto(maquina_virtual::valores::jsn_a_texto(&argumentos[1], false)),
            );
            poner_campo(&respuesta, "terminada", Valor::Log(true));
            Ok(argumentos[0].clone())
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_RESPUESTA}.texto"),
        Box::new(move |argumentos| {
            const F: &str = "RespuestaSaliente.texto";
            exigir_aridad(F, &argumentos[1..], 1)?;
            let respuesta = receptor(F, argumentos, TIPO_RESPUESTA)?;
            poner_cabecera(&respuesta, "Content-Type", "text/plain; charset=utf-8");
            poner_campo(
                &respuesta,
                "cuerpo",
                Valor::texto(texto_de_valor(&argumentos[1])),
            );
            poner_campo(&respuesta, "terminada", Valor::Log(true));
            Ok(argumentos[0].clone())
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_RESPUESTA}.enviar_estado"),
        Box::new(move |argumentos| {
            const F: &str = "RespuestaSaliente.enviar_estado";
            exigir_aridad(F, &argumentos[1..], 1)?;
            let respuesta = receptor(F, argumentos, TIPO_RESPUESTA)?;
            let estado = exigir_estado(F, &argumentos[1])?;
            responder_texto(&respuesta, estado, codigos::razon(estado));
            Ok(argumentos[0].clone())
        }),
    );

    for nombre in ["establecer", "cabecera"] {
        let funcion = format!("{TIPO_RESPUESTA}.{nombre}");
        let nombre_funcion = funcion.clone();
        registro.registrar_funcion(
            &nombre_funcion,
            Box::new(move |argumentos| {
                let respuesta = receptor(&funcion, argumentos, TIPO_RESPUESTA)?;
                match argumentos.len() {
                    2 => {
                        let nombre = arg_texto(&funcion, argumentos, 1)?;
                        Ok(
                            crate::red::objetos::buscar_sin_caso(
                                &campo(&respuesta, "cabeceras"),
                                nombre,
                            )
                            .unwrap_or(Valor::Nulo),
                        )
                    }
                    3 => {
                        let nombre = arg_texto(&funcion, argumentos, 1)?;
                        poner_cabecera(&respuesta, nombre, texto_de_valor(&argumentos[2]));
                        Ok(argumentos[0].clone())
                    }
                    _ => Err(error(
                        "E0210",
                        format!("'{funcion}' espera (nombre) o (nombre, valor)"),
                    )),
                }
            }),
        );
    }

    registro.registrar_funcion(
        &format!("{TIPO_RESPUESTA}.agregar"),
        Box::new(move |argumentos| {
            const F: &str = "RespuestaSaliente.agregar";
            exigir_aridad(F, &argumentos[1..], 2)?;
            let respuesta = receptor(F, argumentos, TIPO_RESPUESTA)?;
            let nombre = arg_texto(F, argumentos, 1)?;
            let nuevo = texto_de_valor(&argumentos[2]);
            let actual = crate::red::objetos::buscar_sin_caso(&campo(&respuesta, "cabeceras"), nombre)
                .map(|valor| texto_de_valor(&valor));
            let combinado = match actual {
                Some(previo) if !previo.is_empty() => format!("{previo}, {nuevo}"),
                _ => nuevo,
            };
            poner_cabecera(&respuesta, nombre, combinado);
            Ok(argumentos[0].clone())
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_RESPUESTA}.tipo"),
        Box::new(move |argumentos| {
            const F: &str = "RespuestaSaliente.tipo";
            exigir_aridad(F, &argumentos[1..], 1)?;
            let respuesta = receptor(F, argumentos, TIPO_RESPUESTA)?;
            poner_cabecera(
                &respuesta,
                "Content-Type",
                http::tipo_mime(arg_texto(F, argumentos, 1)?),
            );
            Ok(argumentos[0].clone())
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_RESPUESTA}.ubicacion"),
        Box::new(move |argumentos| {
            const F: &str = "RespuestaSaliente.ubicacion";
            exigir_aridad(F, &argumentos[1..], 1)?;
            let respuesta = receptor(F, argumentos, TIPO_RESPUESTA)?;
            poner_cabecera(&respuesta, "Location", arg_texto(F, argumentos, 1)?);
            Ok(argumentos[0].clone())
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_RESPUESTA}.redirigir"),
        Box::new(move |argumentos| {
            const F: &str = "RespuestaSaliente.redirigir";
            let respuesta = receptor(F, argumentos, TIPO_RESPUESTA)?;
            let (estado, destino) = match argumentos.len() {
                2 => (302, arg_texto(F, argumentos, 1)?.to_string()),
                3 => (
                    exigir_estado(F, &argumentos[1])?,
                    arg_texto(F, argumentos, 2)?.to_string(),
                ),
                _ => {
                    return Err(error(
                        "E0210",
                        format!("'{F}' espera (destino) o (codigo, destino)"),
                    ));
                }
            };
            poner_cabecera(&respuesta, "Location", destino.clone());
            responder_texto(&respuesta, estado, &format!("redirigiendo a {destino}"));
            Ok(argumentos[0].clone())
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_RESPUESTA}.galleta"),
        Box::new(move |argumentos| {
            const F: &str = "RespuestaSaliente.galleta";
            let respuesta = receptor(F, argumentos, TIPO_RESPUESTA)?;
            if argumentos.len() < 3 || argumentos.len() > 4 {
                return Err(error(
                    "E0210",
                    format!("'{F}' espera (nombre, valor) o (nombre, valor, opciones)"),
                ));
            }
            let nombre = arg_texto(F, argumentos, 1)?;
            let valor = texto_de_valor(&argumentos[2]);
            let mut galleta = format!("{nombre}={valor}");
            if let Some(Valor::Jsn(opciones)) = argumentos.get(3) {
                for (clave, valor) in opciones.borrow().iter() {
                    let texto = texto_de_valor(valor);
                    match clave.to_ascii_lowercase().as_str() {
                        "ruta" | "path" => galleta.push_str(&format!("; Path={texto}")),
                        "dominio" | "domain" => galleta.push_str(&format!("; Domain={texto}")),
                        "expira" | "expires" => galleta.push_str(&format!("; Expires={texto}")),
                        "edad_maxima" | "maxage" | "max_edad" => {
                            galleta.push_str(&format!("; Max-Age={texto}"));
                        }
                        "solo_http" | "httponly" => {
                            if matches!(valor, Valor::Log(true)) {
                                galleta.push_str("; HttpOnly");
                            }
                        }
                        "segura" | "secure" => {
                            if matches!(valor, Valor::Log(true)) {
                                galleta.push_str("; Secure");
                            }
                        }
                        "mismo_sitio" | "samesite" => {
                            galleta.push_str(&format!("; SameSite={texto}"));
                        }
                        _ => {}
                    }
                }
            }
            if let Valor::Lista(lista) = campo(&respuesta, "galletas") {
                lista.borrow_mut().push(Valor::texto(galleta));
            }
            Ok(argumentos[0].clone())
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_RESPUESTA}.borrar_galleta"),
        Box::new(move |argumentos| {
            const F: &str = "RespuestaSaliente.borrar_galleta";
            exigir_aridad(F, &argumentos[1..], 1)?;
            let respuesta = receptor(F, argumentos, TIPO_RESPUESTA)?;
            let nombre = arg_texto(F, argumentos, 1)?;
            if let Valor::Lista(lista) = campo(&respuesta, "galletas") {
                lista.borrow_mut().push(Valor::texto(format!(
                    "{nombre}=; Max-Age=0; Path=/"
                )));
            }
            Ok(argumentos[0].clone())
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_RESPUESTA}.variar"),
        Box::new(move |argumentos| {
            const F: &str = "RespuestaSaliente.variar";
            exigir_aridad(F, &argumentos[1..], 1)?;
            let respuesta = receptor(F, argumentos, TIPO_RESPUESTA)?;
            poner_cabecera(&respuesta, "Vary", arg_texto(F, argumentos, 1)?);
            Ok(argumentos[0].clone())
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_RESPUESTA}.terminar"),
        Box::new(move |argumentos| {
            const F: &str = "RespuestaSaliente.terminar";
            exigir_aridad(F, &argumentos[1..], 0)?;
            let respuesta = receptor(F, argumentos, TIPO_RESPUESTA)?;
            poner_campo(&respuesta, "terminada", Valor::Log(true));
            Ok(argumentos[0].clone())
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_RESPUESTA}.local"),
        Box::new(move |argumentos| {
            const F: &str = "RespuestaSaliente.local";
            let respuesta = receptor(F, argumentos, TIPO_RESPUESTA)?;
            match argumentos.len() {
                2 => {
                    let nombre = arg_texto(F, argumentos, 1)?;
                    Ok(buscar_en_jsn(&campo(&respuesta, "locales"), nombre))
                }
                3 => {
                    let nombre = arg_texto(F, argumentos, 1)?.to_string();
                    if let Valor::Jsn(mapa) = campo(&respuesta, "locales") {
                        mapa.borrow_mut().insert(nombre, argumentos[2].clone());
                    }
                    Ok(argumentos[0].clone())
                }
                _ => Err(error(
                    "E0210",
                    format!("'{F}' espera (nombre) o (nombre, valor)"),
                )),
            }
        }),
    );

    lector(registro, TIPO_RESPUESTA, "cabeceras", "cabeceras");
    lector(registro, TIPO_RESPUESTA, "locales", "locales");
    lector(registro, TIPO_RESPUESTA, "esta_terminada", "terminada");
}

fn exigir_estado(funcion: &str, valor: &Valor) -> Result<i64, Fallo> {
    match valor {
        Valor::Entero(estado) if (100..=599).contains(estado) => Ok(*estado),
        Valor::Entero(estado) => Err(error_tipo(format!(
            "'{funcion}' recibió el código {estado}, fuera del rango 100-599"
        ))),
        otro => Err(error_tipo(format!(
            "'{funcion}' esperaba un código de estado entero, pero recibió '{}'",
            otro.nombre_tipo()
        ))),
    }
}

// ----- Métodos de Continuacion y ErrorHttp -----

fn registrar_continuacion(registro: &mut RegistroNativos) {
    registro.registrar_funcion(
        &format!("{TIPO_CONTINUACION}.siguiente"),
        Box::new(move |argumentos| {
            const F: &str = "Continuacion.siguiente";
            exigir_aridad(F, &argumentos[1..], 0)?;
            let continuacion = receptor(F, argumentos, TIPO_CONTINUACION)?;
            poner_campo(&continuacion, "estado", Valor::texto("siguiente"));
            Ok(Valor::Nulo)
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_CONTINUACION}.siguiente_ruta"),
        Box::new(move |argumentos| {
            const F: &str = "Continuacion.siguiente_ruta";
            exigir_aridad(F, &argumentos[1..], 0)?;
            let continuacion = receptor(F, argumentos, TIPO_CONTINUACION)?;
            poner_campo(&continuacion, "estado", Valor::texto("ruta"));
            Ok(Valor::Nulo)
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_CONTINUACION}.siguiente_con_error"),
        Box::new(move |argumentos| {
            const F: &str = "Continuacion.siguiente_con_error";
            exigir_aridad(F, &argumentos[1..], 1)?;
            let continuacion = receptor(F, argumentos, TIPO_CONTINUACION)?;
            poner_campo(&continuacion, "estado", Valor::texto("error"));
            poner_campo(
                &continuacion,
                "mensaje",
                Valor::texto(texto_de_valor(&argumentos[1])),
            );
            Ok(Valor::Nulo)
        }),
    );
}

fn registrar_error(registro: &mut RegistroNativos) {
    lector(registro, TIPO_ERROR, "mensaje", "mensaje");
    lector(registro, TIPO_ERROR, "estado", "estado");
}

// ----- Utilidades de argumentos -----

fn manejadores_desde(funcion: &str, argumentos: &[Valor]) -> Result<Vec<Valor>, Fallo> {
    if argumentos.is_empty() {
        return Err(error(
            "E0210",
            format!("'{funcion}' necesita al menos un manejador"),
        ));
    }
    argumentos
        .iter()
        .map(|valor| exigir_funcion(funcion, valor))
        .collect()
}

fn exigir_funcion(funcion: &str, valor: &Valor) -> Result<Valor, Fallo> {
    match valor.partes_callable() {
        Some(_) => Ok(valor.clone()),
        None => Err(error_tipo(format!(
            "'{funcion}' esperaba una función declarada en Quetzal, pero recibió '{}'; \
             declara la función fuera y pásala por su nombre, o usa un método de una instancia",
            valor.nombre_tipo()
        ))),
    }
}
