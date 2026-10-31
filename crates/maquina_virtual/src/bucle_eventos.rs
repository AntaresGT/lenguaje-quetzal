//! Bucle de eventos real del Lenguaje Quetzal.
//!
//! El código Quetzal (bytecode, [`crate::valores::Valor`]) usa `Rc` y solo
//! puede ejecutarse en un único hilo: el hilo de la VM. Toda la entrada/
//! salida (peticiones HTTP, sockets, temporizadores) corre en un runtime de
//! tokio que vive en hilos de fondo; cuando una operación termina, envía un
//! [`Mensaje`] de vuelta al hilo de la VM por un canal.
//!
//! El hilo de la VM nunca se queda "bloqueado sin hacer nada": mientras
//! `esperar` espera una tarea nativa, sigue atendiendo cualquier otro evento
//! que llegue (por ejemplo, una petición entrante a un servidor en escucha).
//! Así ninguna operación de E/S bloquea el resto del programa, aunque el
//! código de Quetzal en sí siga siendo de un solo hilo.

use std::cell::Cell;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use crate::errores::Fallo;

/// Datos que pueden cruzar del hilo de fondo al hilo de la VM: nada de `Rc`
/// ni de `Valor` (no son `Send`). El módulo `crate::valores` sabe convertir
/// esto a un `Valor` real una vez que llega al hilo de la VM
/// (`carga_a_valor`), y viceversa (`valor_a_carga`).
#[derive(Debug, Clone)]
pub enum CargaNativa {
    Nula,
    Entero(i64),
    Log(bool),
    Texto(String),
    Lista(Vec<CargaNativa>),
    Mapa(Vec<(String, CargaNativa)>),
    /// Instancia de un objeto nativo (por ejemplo `RespuestaHttp`): se
    /// reconstruye como un [`crate::valores::DatosInstanciaNativa`] con ese
    /// `tipo` y esos campos ya convertidos, en vez de un `jsn` genérico.
    Instancia {
        tipo: String,
        campos: Vec<(String, CargaNativa)>,
    },
}

/// Mensaje que llega al hilo de la VM desde el fondo.
pub enum Mensaje {
    /// Una tarea nativa asincrónica (petición HTTP, lectura de un socket,
    /// ...) terminó. `id` identifica la tarea que la originó (ver
    /// [`BucleEventos::nuevo_id`]).
    TareaLista {
        id: u64,
        resultado: Result<CargaNativa, String>,
    },
    /// Una solicitud que necesita ejecutar un manejador de Quetzal: por
    /// ejemplo, una petición entrante a un `ServidorHttp` en escucha.
    /// `servicio` identifica qué módulo nativo la atiende (`"servidor_http"`,
    /// `"servidor_socket"`, ...) e `id_recurso` identifica el recurso
    /// concreto (qué servidor). La respuesta del manejador se envía de
    /// vuelta por `respuesta` para que el hilo de fondo la use (por ejemplo,
    /// para escribirla en la conexión).
    Solicitud {
        servicio: String,
        id_recurso: u64,
        datos: CargaNativa,
        respuesta: Sender<CargaNativa>,
    },
}

/// Manija enviable a otros hilos (tokio, hilos de aceptación de conexiones)
/// para reportar mensajes al bucle y para lanzar trabajo en el runtime
/// asincrónico compartido.
#[derive(Clone)]
pub struct ManijaBucle {
    entrada: Sender<Mensaje>,
    runtime: Arc<OnceLock<Result<tokio::runtime::Runtime, String>>>,
}

impl ManijaBucle {
    /// Envía un mensaje al hilo de la VM. Se ignora si el bucle ya cerró
    /// (programa terminado): no hay nadie escuchando.
    pub fn enviar(&self, mensaje: Mensaje) {
        let _ = self.entrada.send(mensaje);
    }

    /// Obtiene el runtime compartido, construyéndolo una única vez en el
    /// primer uso. Las manijas clonadas conservan su propiedad y, con ella,
    /// la vida del runtime mientras exista trabajo que lo necesite.
    pub fn runtime(&self) -> Result<&tokio::runtime::Runtime, Fallo> {
        match self.runtime.get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_name("quetzal-red")
                .build()
                .map_err(|error| error.to_string())
        }) {
            Ok(runtime) => Ok(runtime),
            Err(mensaje) => Err(Fallo::excepcion(
                "E0702",
                format!("no se pudo crear el runtime asincrónico de Quetzal: {mensaje}"),
                Vec::new(),
                None,
            )),
        }
    }
}

/// El bucle de eventos: vive en el hilo de la VM.
pub struct BucleEventos {
    salida: Receiver<Mensaje>,
    manija: ManijaBucle,
    /// Cantidad de servidores/temporizadores activos: mientras sea mayor que
    /// cero, el programa sigue vivo aunque el código principal ya haya
    /// terminado (igual que el bucle de eventos de Node.js).
    trabajo_activo: Cell<u64>,
    siguiente_id: Cell<u64>,
}

impl Default for BucleEventos {
    fn default() -> Self {
        Self::nuevo()
    }
}

impl BucleEventos {
    pub fn nuevo() -> Self {
        let (entrada, salida) = mpsc::channel();
        Self {
            salida,
            manija: ManijaBucle {
                entrada,
                runtime: Arc::new(OnceLock::new()),
            },
            trabajo_activo: Cell::new(0),
            siguiente_id: Cell::new(1),
        }
    }

    /// Copia clonable y enviable a otros hilos para reportar eventos.
    pub fn manija(&self) -> ManijaBucle {
        self.manija.clone()
    }

    /// Identificador único para una nueva tarea nativa o un nuevo recurso
    /// (servidor, conexión).
    pub fn nuevo_id(&self) -> u64 {
        let id = self.siguiente_id.get();
        self.siguiente_id.set(id + 1);
        id
    }

    /// Marca un servidor o temporizador como activo: mantiene vivo el bucle
    /// aunque el programa principal ya terminó de ejecutarse.
    pub fn registrar_trabajo_activo(&self) {
        self.trabajo_activo.set(self.trabajo_activo.get() + 1);
    }

    /// Libera un trabajo activo (por ejemplo, `servidor.detener()`).
    pub fn liberar_trabajo_activo(&self) {
        self.trabajo_activo
            .set(self.trabajo_activo.get().saturating_sub(1));
    }

    pub fn hay_trabajo_activo(&self) -> bool {
        self.trabajo_activo.get() > 0
    }

    /// Espera el próximo mensaje, bloqueando el hilo de la VM. No es una
    /// espera desperdiciada: es la esencia del bucle de eventos, que se
    /// libera en cuanto llega cualquier evento (una tarea que termina o una
    /// petición entrante a un servidor).
    pub fn recibir(&self) -> Option<Mensaje> {
        self.salida.recv().ok()
    }

    /// Igual que [`Self::recibir`], pero con límite de tiempo. Se usa al
    /// drenar el bucle al final del programa, para poder revisar
    /// periódicamente si sigue habiendo trabajo activo.
    pub fn recibir_con_limite(&self, limite: Duration) -> Option<Mensaje> {
        self.salida.recv_timeout(limite).ok()
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn nuevo_no_deberia_inicializar_el_runtime() {
        for _ in 0..4 {
            let bucle = BucleEventos::nuevo();
            assert!(bucle.manija.runtime.get().is_none());
        }
    }

    #[test]
    fn manijas_deberian_compartir_la_primera_inicializacion() {
        let bucle = BucleEventos::nuevo();
        let primera = bucle.manija();
        let segunda = primera.clone();
        assert!(primera.runtime.get().is_none());

        let runtime_a = primera.runtime().expect("runtime disponible");
        let runtime_b = segunda.runtime().expect("runtime ya inicializado");
        assert!(std::ptr::eq(runtime_a, runtime_b));
        assert!(primera.runtime.get().is_some());
    }

    #[test]
    fn accesos_concurrentes_deberian_obtener_el_mismo_runtime() {
        let bucle = BucleEventos::nuevo();
        let direcciones = std::thread::scope(|ambito| {
            let mut hilos = Vec::new();
            for _ in 0..8 {
                let manija = bucle.manija();
                hilos.push(ambito.spawn(move || {
                    let runtime = manija.runtime().expect("inicialización concurrente");
                    runtime as *const tokio::runtime::Runtime as usize
                }));
            }
            hilos
                .into_iter()
                .map(|hilo| hilo.join().expect("hilo de inicialización"))
                .collect::<Vec<_>>()
        });

        assert!(
            direcciones
                .iter()
                .all(|direccion| *direccion == direcciones[0])
        );
    }
}
