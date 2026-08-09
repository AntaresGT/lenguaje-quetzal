//! Objetos `Observador` y `EventoArchivo` del módulo nativo
//! `quetzal/sistema_archivos`.
//!
//! Un `Observador` vigila una ruta y entrega los cambios del sistema de
//! archivos como instancias de `EventoArchivo` (`creado`, `modificado`,
//! `borrado`, `renombrado`). El ciclo lo controla el programa: `iniciar()`
//! empieza a vigilar y `detener()` libera el recurso; nada queda vigilando
//! por su cuenta.
//!
//! La vigilancia ocurre en un hilo de `notify`, que empuja los eventos por
//! un canal. La espera (`esperar_evento`) los toma de ese canal; su forma
//! `_asincrono` hace la espera en el runtime de fondo para no bloquear el
//! bucle de eventos.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use indexmap::IndexMap;
use maquina_virtual::{CargaNativa, DatosInstanciaNativa, Fallo, RegistroNativos, Valor, Vm};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use runtime::GuardianPermisos;

use crate::sistema_archivos::{error_io, error_permiso, tarea};
use crate::util::{arg_entero, arg_texto, error, exigir_aridad};

/// Nombres de los tipos visibles para el usuario.
pub(crate) const TIPO: &str = "Observador";
pub(crate) const TIPO_EVENTO: &str = "EventoArchivo";

// ----- Eventos -----

/// Un cambio del sistema de archivos, ya traducido al español y enviable
/// entre hilos.
#[derive(Clone)]
struct EventoNativo {
    tipo: String,
    ruta: String,
    ruta_anterior: Option<String>,
}

impl EventoNativo {
    fn carga(&self) -> CargaNativa {
        let anterior = match &self.ruta_anterior {
            Some(ruta) => CargaNativa::Texto(ruta.clone()),
            None => CargaNativa::Nula,
        };
        CargaNativa::Instancia {
            tipo: TIPO_EVENTO.to_string(),
            campos: vec![
                ("tipo".to_string(), CargaNativa::Texto(self.tipo.clone())),
                ("ruta".to_string(), CargaNativa::Texto(self.ruta.clone())),
                ("ruta_anterior".to_string(), anterior),
                (
                    "texto".to_string(),
                    CargaNativa::Texto(format!("<{} {}>", self.tipo, self.ruta)),
                ),
            ],
        }
    }

    fn instancia(&self) -> Valor {
        let mut datos = IndexMap::new();
        datos.insert("tipo".to_string(), Valor::texto(&self.tipo));
        datos.insert("ruta".to_string(), Valor::texto(&self.ruta));
        datos.insert(
            "ruta_anterior".to_string(),
            match &self.ruta_anterior {
                Some(ruta) => Valor::texto(ruta),
                None => Valor::Nulo,
            },
        );
        datos.insert(
            "texto".to_string(),
            Valor::texto(format!("<{} {}>", self.tipo, self.ruta)),
        );
        Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
            tipo: Rc::from(TIPO_EVENTO),
            datos: RefCell::new(datos),
        }))
    }
}

/// Traduce un evento de `notify` a cero, uno o más eventos de Quetzal.
///
/// `pendiente_renombre` guarda el origen de un renombrado cuando el sistema
/// operativo lo reporta en dos partes (Windows y Linux lo hacen así), para
/// entregar un solo evento `renombrado` con su ruta anterior.
fn traducir(
    evento: notify::Event,
    pendiente_renombre: &Mutex<Option<String>>,
) -> Vec<EventoNativo> {
    use notify::event::{EventKind, ModifyKind, RenameMode};

    let rutas: Vec<String> = evento
        .paths
        .iter()
        .map(|ruta| ruta.to_string_lossy().to_string())
        .collect();
    if rutas.is_empty() {
        return Vec::new();
    }

    match evento.kind {
        EventKind::Create(_) => uno("creado", &rutas),
        EventKind::Remove(_) => uno("borrado", &rutas),
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) if rutas.len() >= 2 => {
            vec![EventoNativo {
                tipo: "renombrado".to_string(),
                ruta: rutas[1].clone(),
                ruta_anterior: Some(rutas[0].clone()),
            }]
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
            if let Ok(mut pendiente) = pendiente_renombre.lock() {
                *pendiente = Some(rutas[0].clone());
            }
            Vec::new()
        }
        EventKind::Modify(ModifyKind::Name(_)) => {
            let anterior = pendiente_renombre
                .lock()
                .ok()
                .and_then(|mut pendiente| pendiente.take());
            vec![EventoNativo {
                tipo: "renombrado".to_string(),
                ruta: rutas[0].clone(),
                ruta_anterior: anterior,
            }]
        }
        EventKind::Modify(_) | EventKind::Any | EventKind::Other => uno("modificado", &rutas),
        // Los accesos (abrir/leer) no son cambios: no se notifican.
        EventKind::Access(_) => Vec::new(),
    }
}

fn uno(tipo: &str, rutas: &[String]) -> Vec<EventoNativo> {
    rutas
        .iter()
        .map(|ruta| EventoNativo {
            tipo: tipo.to_string(),
            ruta: ruta.clone(),
            ruta_anterior: None,
        })
        .collect()
}

// ----- Cola de eventos -----

/// Eventos recibidos del hilo de vigilancia: los ya sacados del canal
/// (`pendientes`) y el canal mismo. Es `Send` + `Sync` para que la espera
/// asincrónica pueda hacerse en el runtime de fondo.
struct Cola {
    pendientes: Mutex<VecDeque<EventoNativo>>,
    receptor: Mutex<Receiver<EventoNativo>>,
}

impl Cola {
    /// Próximo evento; espera como máximo `limite` (sin límite si es `None`).
    fn siguiente(&self, limite: Option<Duration>) -> Option<EventoNativo> {
        if let Ok(mut pendientes) = self.pendientes.lock()
            && let Some(evento) = pendientes.pop_front()
        {
            return Some(evento);
        }
        let receptor = self.receptor.lock().ok()?;
        match limite {
            Some(limite) => match receptor.recv_timeout(limite) {
                Ok(evento) => Some(evento),
                Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => None,
            },
            None => receptor.recv().ok(),
        }
    }

    /// Cuántos eventos hay listos para consumir sin esperar.
    fn pendientes(&self) -> usize {
        let Ok(mut pendientes) = self.pendientes.lock() else {
            return 0;
        };
        if let Ok(receptor) = self.receptor.lock() {
            while let Ok(evento) = receptor.try_recv() {
                pendientes.push_back(evento);
            }
        }
        pendientes.len()
    }
}

// ----- Registro de observadores -----

/// Un observador vivo: su ruta, su cola de eventos y, mientras está activo,
/// el vigilante de `notify` que la alimenta.
struct Recurso {
    ruta: PathBuf,
    recursivo: bool,
    cola: Arc<Cola>,
    emisor: Sender<EventoNativo>,
    vigilante: Option<RecommendedWatcher>,
}

impl Recurso {
    fn activo(&self) -> bool {
        self.vigilante.is_some()
    }
}

#[derive(Clone, Default)]
struct RegistroObservadores {
    creados: Rc<RefCell<HashMap<i64, Recurso>>>,
    siguiente: Rc<std::cell::Cell<i64>>,
}

impl RegistroObservadores {
    fn crear(&self, ruta: &Path, recursivo: bool) -> i64 {
        let (emisor, receptor) = channel();
        let id = self.siguiente.get() + 1;
        self.siguiente.set(id);
        self.creados.borrow_mut().insert(
            id,
            Recurso {
                ruta: ruta.to_path_buf(),
                recursivo,
                cola: Arc::new(Cola {
                    pendientes: Mutex::new(VecDeque::new()),
                    receptor: Mutex::new(receptor),
                }),
                emisor,
                vigilante: None,
            },
        );
        id
    }

    /// Empieza a vigilar. Repetir `iniciar` sobre un observador activo no
    /// hace nada (es idempotente).
    fn iniciar(&self, funcion: &str, id: i64) -> Result<bool, Fallo> {
        let mut creados = self.creados.borrow_mut();
        let recurso = creados
            .get_mut(&id)
            .ok_or_else(|| error_io(format!("'{funcion}' no encontró el observador")))?;
        if recurso.activo() {
            return Ok(false);
        }

        let emisor = recurso.emisor.clone();
        let pendiente_renombre = Mutex::new(None);
        let mut vigilante =
            notify::recommended_watcher(move |resultado: notify::Result<notify::Event>| {
                let Ok(evento) = resultado else {
                    return;
                };
                for traducido in traducir(evento, &pendiente_renombre) {
                    // Si el programa ya soltó el observador, no hay a quién
                    // notificar: el error del canal se ignora a propósito.
                    let _ = emisor.send(traducido);
                }
            })
            .map_err(|fallo| {
                error_io(format!(
                    "'{funcion}' no pudo crear el observador de '{}': {fallo}",
                    recurso.ruta.display()
                ))
            })?;

        let modo = if recurso.recursivo {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        vigilante.watch(&recurso.ruta, modo).map_err(|fallo| {
            error_io(format!(
                "'{funcion}' no pudo vigilar '{}': {fallo}",
                recurso.ruta.display()
            ))
        })?;
        recurso.vigilante = Some(vigilante);
        Ok(true)
    }

    /// Deja de vigilar. Devuelve si había un vigilante activo.
    fn detener(&self, id: i64) -> bool {
        let mut creados = self.creados.borrow_mut();
        match creados.get_mut(&id) {
            Some(recurso) => recurso.vigilante.take().is_some(),
            None => false,
        }
    }

    fn activo(&self, id: i64) -> bool {
        self.creados
            .borrow()
            .get(&id)
            .map(Recurso::activo)
            .unwrap_or(false)
    }

    fn cola(&self, funcion: &str, id: i64) -> Result<Arc<Cola>, Fallo> {
        let creados = self.creados.borrow();
        let recurso = creados
            .get(&id)
            .ok_or_else(|| error_io(format!("'{funcion}' no encontró el observador")))?;
        if !recurso.activo() {
            return Err(error_io(format!(
                "'{funcion}' necesita un observador activo; llama a 'iniciar' antes de esperar \
                 eventos"
            )));
        }
        Ok(Arc::clone(&recurso.cola))
    }
}

// ----- Instancias -----

fn instancia(id: i64, ruta: &Path, recursivo: bool) -> Valor {
    let mut datos = IndexMap::new();
    datos.insert("id".to_string(), Valor::Entero(id));
    datos.insert("ruta".to_string(), Valor::texto(ruta.to_string_lossy()));
    datos.insert("recursivo".to_string(), Valor::Log(recursivo));
    datos.insert(
        "texto".to_string(),
        Valor::texto(format!("<Observador {}>", ruta.display())),
    );
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO),
        datos: RefCell::new(datos),
    }))
}

fn id_del_receptor(funcion: &str, argumentos: &[Valor]) -> Result<i64, Fallo> {
    match argumentos.first() {
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == TIPO => {
            match instancia.datos.borrow().get("id") {
                Some(Valor::Entero(id)) => Ok(*id),
                _ => Err(error(
                    "E0406",
                    format!("'{funcion}' recibió un Observador sin identificador válido"),
                )),
            }
        }
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba un Observador, pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
        None => Err(error("E0210", format!("'{funcion}' necesita un receptor"))),
    }
}

/// Límite de espera en milisegundos; `0` significa esperar sin límite.
fn limite_de(funcion: &str, argumentos: &[Valor]) -> Result<Option<Duration>, Fallo> {
    let milisegundos = arg_entero(funcion, argumentos, 1)?;
    if milisegundos < 0 {
        return Err(error(
            "E0406",
            format!("'{funcion}' recibió un límite negativo ({milisegundos} ms)"),
        ));
    }
    Ok(if milisegundos == 0 {
        None
    } else {
        Some(Duration::from_millis(milisegundos as u64))
    })
}

// ----- Registro de las funciones nativas -----

/// `SistemaArchivos.observar(ruta, recursivo)` y los métodos de
/// `Observador` y `EventoArchivo`. Lo llama el módulo `sistema_archivos`.
pub fn registrar_apertura(registro: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    registro.registrar_modulo("observador");
    registro.registrar_modulo("eventoarchivo");

    let observadores = RegistroObservadores::default();

    let guardian_observar = Rc::clone(guardian);
    let registro_observar = observadores.clone();
    registro.registrar_funcion(
        "sistema_archivos.observar",
        Box::new(move |argumentos| {
            const F: &str = "SistemaArchivos.observar";
            exigir_aridad(F, argumentos, 2)?;
            let ruta = arg_texto(F, argumentos, 0)?;
            let recursivo = match argumentos.get(1) {
                Some(Valor::Log(valor)) => *valor,
                Some(otro) => {
                    return Err(error(
                        "E0406",
                        format!(
                            "'{F}' esperaba un log en el argumento 2 (recursivo), pero recibió '{}'",
                            otro.nombre_tipo()
                        ),
                    ));
                }
                None => false,
            };
            // Observar cambios exige el mismo nivel que modificarlos.
            let resuelta = guardian_observar
                .verificar_escritura(ruta)
                .map_err(error_permiso)?;
            let id = registro_observar.crear(&resuelta, recursivo);
            Ok(instancia(id, &resuelta, recursivo))
        }),
    );

    registrar_ciclo(registro, &observadores);
    registrar_espera(registro, &observadores);
    registrar_consultas(registro, &observadores);
    registrar_evento(registro);
}

fn registrar_ciclo(registro: &mut RegistroNativos, observadores: &RegistroObservadores) {
    let registro_iniciar = observadores.clone();
    registro.registrar_funcion_con_vm(
        "Observador.iniciar",
        Box::new(move |vm: &mut Vm, argumentos: &[Valor]| {
            const F: &str = "Observador.iniciar";
            exigir_aridad(F, &argumentos[1..], 0)?;
            let id = id_del_receptor(F, argumentos)?;
            if registro_iniciar.iniciar(F, id)? {
                // Mientras vigila, el programa no termina por su cuenta.
                vm.bucle().registrar_trabajo_activo();
            }
            Ok(Valor::Nulo)
        }),
    );

    let registro_detener = observadores.clone();
    registro.registrar_funcion_con_vm(
        "Observador.detener",
        Box::new(move |vm: &mut Vm, argumentos: &[Valor]| {
            const F: &str = "Observador.detener";
            exigir_aridad(F, &argumentos[1..], 0)?;
            let id = id_del_receptor(F, argumentos)?;
            if registro_detener.detener(id) {
                vm.bucle().liberar_trabajo_activo();
            }
            Ok(Valor::Nulo)
        }),
    );
}

fn registrar_espera(registro: &mut RegistroNativos, observadores: &RegistroObservadores) {
    const F: &str = "Observador.esperar_evento";

    let registro_sincrono = observadores.clone();
    registro.registrar_funcion(
        "Observador.esperar_evento",
        Box::new(move |argumentos| {
            exigir_aridad(F, &argumentos[1..], 1)?;
            let cola = registro_sincrono.cola(F, id_del_receptor(F, argumentos)?)?;
            let limite = limite_de(F, argumentos)?;
            Ok(match cola.siguiente(limite) {
                Some(evento) => evento.instancia(),
                None => Valor::Nulo,
            })
        }),
    );

    let registro_asincrono = observadores.clone();
    registro.registrar_funcion_con_vm(
        "Observador.esperar_evento_asincrono",
        Box::new(move |vm, argumentos| {
            exigir_aridad(F, &argumentos[1..], 1)?;
            let cola = registro_asincrono.cola(F, id_del_receptor(F, argumentos)?)?;
            let limite = limite_de(F, argumentos)?;
            tarea(vm, move || {
                Ok(match cola.siguiente(limite) {
                    Some(evento) => evento.carga(),
                    None => CargaNativa::Nula,
                })
            })
        }),
    );
}

fn registrar_consultas(registro: &mut RegistroNativos, observadores: &RegistroObservadores) {
    let registro_activo = observadores.clone();
    registro.registrar_funcion(
        "Observador.esta_activo",
        Box::new(move |argumentos| {
            const F: &str = "Observador.esta_activo";
            exigir_aridad(F, &argumentos[1..], 0)?;
            Ok(Valor::Log(
                registro_activo.activo(id_del_receptor(F, argumentos)?),
            ))
        }),
    );

    let registro_pendientes = observadores.clone();
    registro.registrar_funcion(
        "Observador.eventos_pendientes",
        Box::new(move |argumentos| {
            const F: &str = "Observador.eventos_pendientes";
            exigir_aridad(F, &argumentos[1..], 0)?;
            let cola = registro_pendientes.cola(F, id_del_receptor(F, argumentos)?)?;
            Ok(Valor::Entero(cola.pendientes() as i64))
        }),
    );

    registrar_dato(registro, TIPO, "ruta", "ruta");
    registrar_dato(registro, TIPO, "es_recursivo", "recursivo");
}

fn registrar_evento(registro: &mut RegistroNativos) {
    registrar_dato(registro, TIPO_EVENTO, "tipo", "tipo");
    registrar_dato(registro, TIPO_EVENTO, "ruta", "ruta");
    registrar_dato(registro, TIPO_EVENTO, "ruta_anterior", "ruta_anterior");
}

/// Método que solo devuelve un dato guardado en la instancia.
fn registrar_dato(
    registro: &mut RegistroNativos,
    tipo: &'static str,
    metodo: &'static str,
    campo: &'static str,
) {
    let funcion = format!("{tipo}.{metodo}");
    registro.registrar_funcion(
        &format!("{tipo}.{metodo}"),
        Box::new(move |argumentos| {
            exigir_aridad(&funcion, &argumentos[1..], 0)?;
            match argumentos.first() {
                Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == tipo => {
                    Ok(instancia
                        .datos
                        .borrow()
                        .get(campo)
                        .cloned()
                        .unwrap_or(Valor::Nulo))
                }
                _ => Err(error(
                    "E0406",
                    format!("'{funcion}' esperaba un {tipo} como receptor"),
                )),
            }
        }),
    );
}
