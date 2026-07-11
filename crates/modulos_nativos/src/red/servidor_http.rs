//! Objeto `ServidorHttp`: servidor HTTP con rutas en español y manejadores
//! de Quetzal pasados por referencia (nunca copiados: los valores `Valor`
//! de tipo función son `Rc`, así que registrarlos como manejador es un
//! `Rc::clone` barato, no una copia de la función).
//!
//! Arquitectura reactor (igual que el resto de `quetzal/red`): cada
//! `ServidorHttp.escuchar(puerto)` abre un `TcpListener` y acepta conexiones
//! en hilos del sistema operativo (no en el runtime de tokio: la conexión se
//! atiende de forma síncrona, bloqueando solo ese hilo dedicado). Cada
//! petición se envía como [`Mensaje::Solicitud`] al hilo de la VM, que la
//! despacha a la ruta que corresponda, invoca el manejador de Quetzal
//! (`Vm::llamar_funcion`, con acceso completo al bucle de eventos: el
//! manejador puede usar `esperar` normalmente) y devuelve la respuesta por
//! el canal de vuelta para que el hilo de la conexión la escriba en el
//! socket. Así ninguna petición bloquea el hilo de la VM ni al resto del
//! programa.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use indexmap::IndexMap;
use maquina_virtual::bucle_eventos::CargaNativa;
use maquina_virtual::{
    DatosInstanciaNativa, Fallo, ManijaBucle, Mensaje, RegistroNativos, Valor, Vm,
};
use runtime::GuardianPermisos;

use crate::util::{arg_entero, arg_jsn, arg_ruta, arg_texto, error, error_aridad, exigir_aridad};

use super::protocolo_http::{LIMITE_CUERPO_PREDETERMINADO, LimitesHttp, leer_peticion};
use super::{bytes_y_tipo_contenido, metodo_http_desde_texto, permiso_denegado};
use crate::sistema_archivos::{FlujoArchivo, metadatos_archivo, tipo_contenido};

const TIPO_SERVIDOR: &str = "ServidorHttp";
const TIPO_PETICION: &str = "PeticionHttp";
const TIPO_RESPUESTA_SERVIDOR: &str = "RespuestaServidor";
const TIPO_PETICION_CRUDA: &str = "_peticion_cruda";
const TIPO_RESPUESTA_CRUDA: &str = "_respuesta_cruda";
/// Respuesta de [`Respuestas::archivo`] o de una ruta estática: en vez de un
/// cuerpo ya armado en memoria, lleva la ruta en disco y los metadatos de
/// caché; el hilo de la conexión transmite el archivo directamente al
/// socket por trozos (nunca lo carga completo en RAM), y resuelve `Range`
/// (`206`), condicionales de caché (`304`) y rangos inválidos (`416`).
const TIPO_RESPUESTA_ARCHIVO: &str = "_respuesta_archivo";
/// Respuesta de [`Respuestas::flujo`]: el cuerpo se produce en trozos (uno
/// por llamada a la función generadora de Quetzal) y se envía con
/// `Transfer-Encoding: chunked`, sin conocer el tamaño total de antemano
/// (útil para *server-sent events* o cualquier flujo en vivo).
const TIPO_RESPUESTA_FLUJO: &str = "_respuesta_flujo";
/// Tamaño de cada trozo al transmitir un archivo del disco al socket: ni el
/// archivo completo ni cada respuesta de `Respuestas.archivo` viven enteros
/// en memoria, sin importar cuán grande sea el archivo (video, etc.).
const TAMANO_TROZO_ARCHIVO: usize = 64 * 1024;

/// Estado de la cadena de interceptores en ejecución. Vive en el hilo de
/// la VM (una sola cadena a la vez por hilo). `siguiente` lo consume
/// llamando al primer item de `restantes` y reemplazando el estado por los
/// que queden.
struct EstadoCadena {
    restantes: Vec<Valor>,
}

type EstadoCadenaCompartido = Rc<RefCell<HashMap<u64, EstadoCadena>>>;

/// Nombre del servicio del despachador (`Vm::registrar_despachador`),
/// compartido por todas las instancias de `ServidorHttp`: cada una se
/// distingue por su `id_recurso`.
const SERVICIO: &str = "servidor_http";

/// Registro compartido: cada `ServidorHttp` en escucha se identifica por el
/// `id_recurso` que le asignó el bucle de eventos. El despachador único del
/// servicio lo usa para encontrar la instancia (y su tabla de rutas)
/// correspondiente a cada petición entrante.
type RegistroServidores = Rc<RefCell<HashMap<u64, Rc<DatosInstanciaNativa>>>>;

/// Bandera para detener limpiamente el hilo de aceptación de conexiones de
/// cada servidor (`ServidorHttp.detener()`).
type RegistroBanderas = Rc<RefCell<HashMap<u64, Arc<AtomicBool>>>>;

/// Generadores de `Respuestas.flujo(...)` pendientes, por `id_flujo`. Vive
/// exclusivamente en el hilo de la VM (la función de Quetzal es `Rc`, no es
/// `Send`): el hilo de la conexión solo conoce el `id_flujo` y pide "el
/// siguiente trozo" mandando un evento al despachador, igual que una
/// petición HTTP normal. Se limpia cuando el generador devuelve `nulo` o
/// cuando la conexión se cierra (evento `cerrar_flujo`), para no filtrar
/// memoria si el cliente se desconecta a mitad de un flujo.
type RegistroFlujos = Rc<RefCell<HashMap<u64, Valor>>>;

fn error_servidor(mensaje: impl Into<String>) -> Fallo {
    error("E0704", mensaje.into())
}

// =====================================================================
// ServidorHttp: instancia y registro de rutas
// =====================================================================

fn nuevo_servidor() -> Valor {
    let mut datos = IndexMap::new();
    datos.insert("rutas".to_string(), Valor::lista(Vec::new()));
    datos.insert("estaticos".to_string(), Valor::lista(Vec::new()));
    datos.insert("interceptores".to_string(), Valor::lista(Vec::new()));
    datos.insert("manejador_errores".to_string(), Valor::Nulo);
    datos.insert("activo".to_string(), Valor::Log(false));
    datos.insert("puerto".to_string(), Valor::Nulo);
    datos.insert(
        "limite_cuerpo".to_string(),
        Valor::Entero(LIMITE_CUERPO_PREDETERMINADO as i64),
    );
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_SERVIDOR),
        datos: RefCell::new(datos),
    }))
}

/// Lee un campo entero opcional de un `jsn` de opciones. `nulo` o ausente
/// -> `None`; cualquier otro tipo -> error `E0406`.
fn entero_opcional_de_jsn(
    funcion: &str,
    opciones: &IndexMap<String, Valor>,
    campo: &str,
) -> Result<Option<i64>, Fallo> {
    match opciones.get(campo) {
        Some(Valor::Nulo) | None => Ok(None),
        Some(Valor::Entero(n)) => Ok(Some(*n)),
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}': el campo '{campo}' de las opciones debe ser un entero, pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
    }
}

/// Lee un campo texto opcional de un `jsn` de opciones.
fn texto_opcional_de_jsn(
    funcion: &str,
    opciones: &IndexMap<String, Valor>,
    campo: &str,
) -> Result<Option<String>, Fallo> {
    match opciones.get(campo) {
        Some(Valor::Nulo) | None => Ok(None),
        Some(Valor::Texto(t)) => Ok(Some(t.to_string())),
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}': el campo '{campo}' de las opciones debe ser un texto, pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
    }
}

/// Lee un campo logico opcional de un `jsn` de opciones.
fn logico_opcional_de_jsn(
    funcion: &str,
    opciones: &IndexMap<String, Valor>,
    campo: &str,
) -> Result<Option<bool>, Fallo> {
    match opciones.get(campo) {
        Some(Valor::Nulo) | None => Ok(None),
        Some(Valor::Log(b)) => Ok(Some(*b)),
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}': el campo '{campo}' de las opciones debe ser un logico, pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
    }
}

fn constructor(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "nuevo ServidorHttp";
    if argumentos.len() > 1 {
        return Err(error_aridad(F, 0, argumentos.len()));
    }
    let servidor = nuevo_servidor();
    if argumentos.len() == 1 {
        let opciones = arg_jsn(F, argumentos, 0)?;
        let opciones = opciones.borrow();
        let instancia = match &servidor {
            Valor::InstanciaNativa(i) => Rc::clone(i),
            _ => unreachable!(),
        };
        if let Some(limite) = entero_opcional_de_jsn(F, &opciones, "limite_cuerpo")? {
            validar_limite_cuerpo(F, limite)?;
            instancia
                .datos
                .borrow_mut()
                .insert("limite_cuerpo".to_string(), Valor::Entero(limite));
        }
    }
    Ok(servidor)
}

fn receptor_servidor(
    funcion: &str,
    argumentos: &[Valor],
) -> Result<Rc<DatosInstanciaNativa>, Fallo> {
    match argumentos.first() {
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == TIPO_SERVIDOR => {
            Ok(Rc::clone(instancia))
        }
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba un ServidorHttp, pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
        None => Err(error("E0210", format!("'{funcion}' necesita el receptor"))),
    }
}

/// Un manejador debe pasarse por referencia (una función de Quetzal ya
/// definida): nunca se copia, solo se clona el `Rc` interno del `Valor`.
fn exigir_manejador(funcion: &str, valor: &Valor) -> Result<Valor, Fallo> {
    match valor {
        Valor::Funcion(..) => Ok(valor.clone()),
        otro => Err(error(
            "E0406",
            format!(
                "'{funcion}' espera una función definida como manejador, pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
    }
}

fn agregar_ruta(
    instancia: &DatosInstanciaNativa,
    metodo: &reqwest::Method,
    patron: &str,
    manejador: Valor,
    interceptores: Valor,
) {
    let mut entrada = IndexMap::new();
    entrada.insert("metodo".to_string(), Valor::texto(metodo.as_str()));
    entrada.insert("patron".to_string(), Valor::texto(patron));
    entrada.insert("manejador".to_string(), manejador);
    entrada.insert("interceptores".to_string(), interceptores);
    if let Some(Valor::Lista(rutas)) = instancia.datos.borrow().get("rutas") {
        rutas.borrow_mut().push(Valor::jsn(entrada));
    }
}

fn metodo_ruta_fija(
    funcion: &'static str,
    metodo_http: reqwest::Method,
) -> maquina_virtual::FuncionNativa {
    Box::new(move |argumentos| {
        // 2 args: (patron, manejador) — sin interceptores
        // 3 args: (patron, [interceptores], manejador) — con interceptores de ruta
        if argumentos.len() < 3 || argumentos.len() > 4 {
            return Err(error_aridad(funcion, 2, argumentos.len() - 1));
        }
        let instancia = receptor_servidor(funcion, argumentos)?;
        let patron = arg_texto(funcion, argumentos, 1)?.to_string();
        let (interceptores, manejador) = if argumentos.len() == 3 {
            (Valor::lista(Vec::new()), argumentos[2].clone())
        } else {
            let interceptores = match &argumentos[2] {
                Valor::Lista(_) => argumentos[2].clone(),
                _ => {
                    return Err(error(
                        "E0406",
                        format!(
                            "'{funcion}': el segundo argumento debe ser una lista de interceptores"
                        ),
                    ));
                }
            };
            (interceptores, argumentos[3].clone())
        };
        let manejador = exigir_manejador(funcion, &manejador)?;
        agregar_ruta(&instancia, &metodo_http, &patron, manejador, interceptores);
        Ok(Valor::Nulo)
    })
}

fn metodo_ruta_generica(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ServidorHttp.ruta";
    if argumentos.len() < 4 || argumentos.len() > 5 {
        return Err(error_aridad(F, 3, argumentos.len() - 1));
    }
    let instancia = receptor_servidor(F, argumentos)?;
    let metodo_texto = arg_texto(F, argumentos, 1)?;
    let metodo_http = metodo_http_desde_texto(F, metodo_texto)?;
    let patron = arg_texto(F, argumentos, 2)?.to_string();
    let (interceptores, manejador) = if argumentos.len() == 4 {
        (Valor::lista(Vec::new()), argumentos[3].clone())
    } else {
        let interceptores = match &argumentos[3] {
            Valor::Lista(_) => argumentos[3].clone(),
            _ => {
                return Err(error(
                    "E0406",
                    format!("'{F}': el tercer argumento debe ser una lista de interceptores"),
                ));
            }
        };
        (interceptores, argumentos[4].clone())
    };
    let manejador = exigir_manejador(F, &manejador)?;
    agregar_ruta(&instancia, &metodo_http, &patron, manejador, interceptores);
    Ok(Valor::Nulo)
}

/// `ServidorHttp.todo(patron, manejador)` o
/// `ServidorHttp.todo(patron, [interceptores], manejador)`: registra una
/// ruta que coincide con cualquier método HTTP.
fn metodo_ruta_todo(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ServidorHttp.todo";
    if argumentos.len() < 3 || argumentos.len() > 4 {
        return Err(error_aridad(F, 2, argumentos.len() - 1));
    }
    let instancia = receptor_servidor(F, argumentos)?;
    let patron = arg_texto(F, argumentos, 1)?.to_string();
    let (interceptores, manejador) = if argumentos.len() == 3 {
        (Valor::lista(Vec::new()), argumentos[2].clone())
    } else {
        let interceptores = match &argumentos[2] {
            Valor::Lista(_) => argumentos[2].clone(),
            _ => {
                return Err(error(
                    "E0406",
                    format!("'{F}': el segundo argumento debe ser una lista de interceptores"),
                ));
            }
        };
        (interceptores, argumentos[4].clone())
    };
    let manejador = exigir_manejador(F, &manejador)?;
    let mut entrada = IndexMap::new();
    entrada.insert("metodo".to_string(), Valor::texto("*"));
    entrada.insert("patron".to_string(), Valor::texto(patron));
    entrada.insert("manejador".to_string(), manejador);
    entrada.insert("interceptores".to_string(), interceptores);
    if let Some(Valor::Lista(rutas)) = instancia.datos.borrow().get("rutas") {
        rutas.borrow_mut().push(Valor::jsn(entrada));
    }
    Ok(Valor::Nulo)
}

/// Registra una carpeta estática: toda petición `GET` cuya ruta empiece con
/// `prefijo` que no coincida con ninguna ruta explícita se resuelve como el
/// archivo `directorio/resto` (protegido contra *path traversal* por el
/// propio [`GuardianPermisos`], que canonicaliza la ruta antes de servirla).
fn agregar_estatico(instancia: &DatosInstanciaNativa, prefijo: &str, directorio: &str) {
    let mut entrada = IndexMap::new();
    entrada.insert("prefijo".to_string(), Valor::texto(prefijo));
    entrada.insert("directorio".to_string(), Valor::texto(directorio));
    if let Some(Valor::Lista(estaticos)) = instancia.datos.borrow().get("estaticos") {
        estaticos.borrow_mut().push(Valor::jsn(entrada));
    }
}

fn metodo_estaticos(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ServidorHttp.estaticos";
    exigir_aridad(F, &argumentos[1..], 2)?;
    let instancia = receptor_servidor(F, argumentos)?;
    let prefijo = arg_texto(F, argumentos, 1)?.trim_matches('/').to_string();
    let directorio = arg_texto(F, argumentos, 2)?
        .trim_end_matches('/')
        .to_string();
    agregar_estatico(&instancia, &prefijo, &directorio);
    Ok(Valor::Nulo)
}

fn metodo_puerto(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ServidorHttp.puerto";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let instancia = receptor_servidor(F, argumentos)?;
    Ok(instancia
        .datos
        .borrow()
        .get("puerto")
        .cloned()
        .unwrap_or(Valor::Nulo))
}

fn metodo_esta_escuchando(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ServidorHttp.esta_escuchando";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let instancia = receptor_servidor(F, argumentos)?;
    let activo = matches!(
        instancia.datos.borrow().get("activo"),
        Some(Valor::Log(true))
    );
    Ok(Valor::Log(activo))
}

/// `ServidorHttp.limite_cuerpo(bytes)`: fija el máximo de bytes del cuerpo
/// de una petición; si se excede, se responde `413` sin invocar el manejador.
fn metodo_limite_cuerpo(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ServidorHttp.limite_cuerpo";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let instancia = receptor_servidor(F, argumentos)?;
    let bytes = arg_entero(F, argumentos, 1)?;
    validar_limite_cuerpo(F, bytes)?;
    if matches!(
        instancia.datos.borrow().get("activo"),
        Some(Valor::Log(true))
    ) {
        return Err(error_servidor(format!(
            "'{F}' debe configurarse antes de escuchar"
        )));
    }
    instancia
        .datos
        .borrow_mut()
        .insert("limite_cuerpo".to_string(), Valor::Entero(bytes));
    Ok(Valor::Nulo)
}

fn validar_limite_cuerpo(funcion: &str, bytes: i64) -> Result<(), Fallo> {
    if bytes < 0 {
        return Err(error(
            "E0406",
            format!("'{funcion}' espera un límite de cuerpo mayor o igual a cero"),
        ));
    }
    usize::try_from(bytes).map(|_| ()).map_err(|_| {
        error(
            "E0406",
            format!("'{funcion}' recibió un límite de cuerpo no representable"),
        )
    })
}

/// `ServidorHttp.usar(interceptor)`: registra un interceptor global que se
/// ejecuta antes de cada ruta. El interceptor recibe `(peticion, respuesta,
/// siguiente)` y debe llamar a `siguiente(peticion, respuesta)` para
/// continuar la cadena, o retornar una `RespuestaServidor` para cortocircuitar.
fn metodo_usar(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ServidorHttp.usar";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let instancia = receptor_servidor(F, argumentos)?;
    let interceptor = exigir_manejador(F, &argumentos[1])?;
    if let Some(Valor::Lista(interceptores)) = instancia.datos.borrow().get("interceptores") {
        interceptores.borrow_mut().push(interceptor);
    }
    Ok(Valor::Nulo)
}

/// `ServidorHttp.manejar_errores(fun)`: registra un manejador global de
/// errores. Si un manejador o interceptor lanza una excepción, se invoca
/// `fun(error, peticion)` y su retorno se usa como respuesta.
fn metodo_manejar_errores(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ServidorHttp.manejar_errores";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let instancia = receptor_servidor(F, argumentos)?;
    let manejador = exigir_manejador(F, &argumentos[1])?;
    instancia
        .datos
        .borrow_mut()
        .insert("manejador_errores".to_string(), manejador);
    Ok(Valor::Nulo)
}

// =====================================================================
// ServidorHttp.escuchar(puerto) / .detener()
// =====================================================================

fn metodo_escuchar(
    guardian: &Rc<GuardianPermisos>,
    registro_servidores: &RegistroServidores,
    banderas: &RegistroBanderas,
    registro_flujos: &RegistroFlujos,
    estado_cadena: &EstadoCadenaCompartido,
) -> maquina_virtual::FuncionNativaConVm {
    const F: &str = "ServidorHttp.escuchar";
    let guardian = Rc::clone(guardian);
    let registro_servidores = Rc::clone(registro_servidores);
    let banderas = Rc::clone(banderas);
    let registro_flujos = Rc::clone(registro_flujos);
    let estado_cadena = Rc::clone(estado_cadena);
    Box::new(move |vm, argumentos| {
        if argumentos.len() < 2 || argumentos.len() > 3 {
            return Err(error_aridad(F, 1, argumentos.len() - 1));
        }
        let instancia = receptor_servidor(F, argumentos)?;
        let puerto_solicitado = arg_entero(F, argumentos, 1)?;
        if !(0..=65535).contains(&puerto_solicitado) {
            return Err(error(
                "E0406",
                format!("'{F}' espera un puerto entre 0 y 65535, pero recibió {puerto_solicitado}"),
            ));
        }
        let puerto = puerto_solicitado as u16;
        guardian
            .verificar_red_servidor(puerto)
            .map_err(permiso_denegado)?;

        let ya_activo = matches!(
            instancia.datos.borrow().get("activo"),
            Some(Valor::Log(true))
        );
        if ya_activo {
            return Err(error_servidor(format!(
                "'{F}': el servidor ya está escuchando"
            )));
        }

        let escucha = TcpListener::bind(("0.0.0.0", puerto)).map_err(|causa| {
            error_servidor(format!(
                "no se pudo escuchar en el puerto {puerto}: {causa}"
            ))
        })?;
        escucha.set_nonblocking(true).map_err(|causa| {
            error_servidor(format!("no se pudo configurar el servidor: {causa}"))
        })?;
        let puerto_real = escucha
            .local_addr()
            .map_err(|causa| {
                error_servidor(format!("no se pudo leer el puerto del servidor: {causa}"))
            })?
            .port();
        let limite_cuerpo = match instancia.datos.borrow().get("limite_cuerpo") {
            Some(Valor::Entero(bytes)) => usize::try_from(*bytes).map_err(|_| {
                error_servidor("el límite de cuerpo configurado no es representable")
            })?,
            _ => LIMITE_CUERPO_PREDETERMINADO,
        };
        let limites = LimitesHttp {
            cuerpo: limite_cuerpo,
        };

        let id = vm.bucle().nuevo_id();
        registro_servidores
            .borrow_mut()
            .insert(id, Rc::clone(&instancia));
        let bandera = Arc::new(AtomicBool::new(true));
        banderas.borrow_mut().insert(id, Arc::clone(&bandera));

        {
            let mut datos = instancia.datos.borrow_mut();
            datos.insert("activo".to_string(), Valor::Log(true));
            datos.insert("puerto".to_string(), Valor::Entero(i64::from(puerto_real)));
            datos.insert("id_interno".to_string(), Valor::Entero(id as i64));
        }

        vm.registrar_despachador(SERVICIO, {
            let registro_servidores = Rc::clone(&registro_servidores);
            let registro_flujos = Rc::clone(&registro_flujos);
            let guardian = Rc::clone(&guardian);
            let estado_cadena = Rc::clone(&estado_cadena);
            move |vm, id_recurso, datos| {
                despachar_peticion(
                    vm,
                    id_recurso,
                    &registro_servidores,
                    &registro_flujos,
                    &guardian,
                    &estado_cadena,
                    datos,
                )
            }
        });
        vm.bucle().registrar_trabajo_activo();

        let manija = vm.bucle().manija();
        thread::spawn(move || bucle_aceptacion(escucha, id, manija, bandera, limites));

        Ok(Valor::Nulo)
    })
}

fn metodo_detener(
    banderas: &RegistroBanderas,
    registro_servidores: &RegistroServidores,
) -> maquina_virtual::FuncionNativaConVm {
    const F: &str = "ServidorHttp.detener";
    let banderas = Rc::clone(banderas);
    let registro_servidores = Rc::clone(registro_servidores);
    Box::new(move |vm, argumentos| {
        exigir_aridad(F, &argumentos[1..], 0)?;
        let instancia = receptor_servidor(F, argumentos)?;
        let id = match instancia.datos.borrow().get("id_interno") {
            Some(Valor::Entero(id)) => Some(*id as u64),
            _ => None,
        };
        let activo = matches!(
            instancia.datos.borrow().get("activo"),
            Some(Valor::Log(true))
        );
        if !activo {
            return Ok(Valor::Nulo);
        }
        instancia
            .datos
            .borrow_mut()
            .insert("activo".to_string(), Valor::Log(false));
        if let Some(id) = id {
            if let Some(bandera) = banderas.borrow_mut().remove(&id) {
                bandera.store(false, Ordering::Relaxed);
            }
            registro_servidores.borrow_mut().remove(&id);
        }
        vm.bucle().liberar_trabajo_activo();
        Ok(Valor::Nulo)
    })
}

/// Hilo de aceptación de conexiones: se detiene en cuanto `bandera` pasa a
/// `false` (`ServidorHttp.detener()`). Cada conexión se atiende en su propio
/// hilo para no bloquear la aceptación de las siguientes.
fn bucle_aceptacion(
    escucha: TcpListener,
    id_recurso: u64,
    manija: ManijaBucle,
    bandera: Arc<AtomicBool>,
    limites: LimitesHttp,
) {
    while bandera.load(Ordering::Relaxed) {
        match escucha.accept() {
            Ok((flujo, _)) => {
                let _ = flujo.set_read_timeout(Some(Duration::from_secs(30)));
                let _ = flujo.set_write_timeout(Some(Duration::from_secs(30)));
                let manija = manija.clone();
                thread::spawn(move || manejar_conexion(flujo, id_recurso, manija, limites));
            }
            Err(fallo) if fallo.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(_) => break,
        }
    }
}

// =====================================================================
// Conexión: lectura cruda de la petición y escritura de la respuesta
// =====================================================================

fn decodificar_percentual(texto: &str) -> String {
    percent_encoding::percent_decode_str(&texto.replace('+', " "))
        .decode_utf8_lossy()
        .into_owned()
}

fn analizar_consulta(consulta: &str) -> Vec<(String, String)> {
    consulta
        .split('&')
        .filter(|par| !par.is_empty())
        .map(|par| match par.split_once('=') {
            Some((clave, valor)) => (decodificar_percentual(clave), decodificar_percentual(valor)),
            None => (decodificar_percentual(par), String::new()),
        })
        .collect()
}

fn razon_estado(estado: u16) -> &'static str {
    match estado {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        206 => "Partial Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        413 => "Payload Too Large",
        414 => "URI Too Long",
        416 => "Range Not Satisfiable",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        505 => "HTTP Version Not Supported",
        _ if (200..300).contains(&estado) => "OK",
        _ if (400..500).contains(&estado) => "Bad Request",
        _ => "Internal Server Error",
    }
}

fn construir_bytes_http(estado: u16, cabeceras: &[(String, String)], cuerpo: &[u8]) -> Vec<u8> {
    let mut salida = format!("HTTP/1.1 {estado} {}\r\n", razon_estado(estado)).into_bytes();
    for (nombre, valor) in cabeceras {
        salida.extend_from_slice(format!("{nombre}: {valor}\r\n").as_bytes());
    }
    if !cabeceras
        .iter()
        .any(|(nombre, _)| nombre.eq_ignore_ascii_case("content-length"))
    {
        salida.extend_from_slice(format!("Content-Length: {}\r\n", cuerpo.len()).as_bytes());
    }
    salida.extend_from_slice(b"Connection: close\r\n\r\n");
    salida.extend_from_slice(cuerpo);
    salida
}

/// Busca una cabecera por nombre, sin distinguir mayúsculas/minúsculas
/// (`Range`, `range` y `RANGE` son la misma cabecera para HTTP).
fn buscar_cabecera<'a>(cabeceras: &'a [(String, String)], nombre: &str) -> Option<&'a str> {
    cabeceras
        .iter()
        .find(|(clave, _)| clave.eq_ignore_ascii_case(nombre))
        .map(|(_, valor)| valor.as_str())
}

/// Escribe únicamente las cabeceras de la respuesta (línea de estado +
/// cabeceras + línea vacía) directamente en el socket, sin cuerpo. Lo usan
/// las respuestas de archivo/flujo, que transmiten el cuerpo por su cuenta
/// (streaming) en vez de construirlo entero en un `Vec<u8>`.
fn escribir_encabezados(
    flujo: &mut TcpStream,
    estado: u16,
    cabeceras: &[(String, String)],
) -> std::io::Result<()> {
    let mut salida = format!("HTTP/1.1 {estado} {}\r\n", razon_estado(estado)).into_bytes();
    for (nombre, valor) in cabeceras {
        salida.extend_from_slice(format!("{nombre}: {valor}\r\n").as_bytes());
    }
    salida.extend_from_slice(b"Connection: close\r\n\r\n");
    flujo.write_all(&salida)
}

/// Copia `longitud` bytes del `flujo` (ya posicionado con `buscar`) al
/// socket, en trozos de [`TAMANO_TROZO_ARCHIVO`]: nunca materializa el
/// archivo completo en memoria, sin importar su tamaño. Delega la lectura
/// del disco a `sistema_archivos::FlujoArchivo` (SRP: `red` no lee archivos).
fn transmitir_flujo(
    flujo: &mut FlujoArchivo,
    socket: &mut TcpStream,
    longitud: u64,
) -> std::io::Result<()> {
    let mut restantes = longitud;
    let mut buffer = [0u8; TAMANO_TROZO_ARCHIVO];
    while restantes > 0 {
        let a_leer = restantes.min(TAMANO_TROZO_ARCHIVO as u64) as usize;
        let leidos = flujo.leer_trozo(&mut buffer[..a_leer])?;
        if leidos == 0 {
            break;
        }
        socket.write_all(&buffer[..leidos])?;
        restantes -= leidos as u64;
    }
    Ok(())
}

/// Marca de tiempo (milisegundos Unix) formateada como fecha HTTP
/// (`Sun, 06 Nov 1994 08:49:37 GMT`, el formato preferido de RFC 7231) para
/// `Last-Modified`. No depende de una crate nueva: usa `chrono`, que ya es
/// dependencia del módulo `tiempo`.
fn formatear_fecha_http(marca_ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(marca_ms.max(0) / 1000, 0)
        .map(|fecha| fecha.format("%a, %d %b %Y %H:%M:%S GMT").to_string())
        .unwrap_or_else(|| "Thu, 01 Jan 1970 00:00:00 GMT".to_string())
}

/// Interpreta una fecha HTTP (`If-Modified-Since`) como segundos Unix; si
/// el formato no se reconoce, devuelve `None` (la condición se ignora en
/// vez de fallar la petición).
fn analizar_fecha_http(valor: &str) -> Option<i64> {
    chrono::NaiveDateTime::parse_from_str(valor.trim(), "%a, %d %b %Y %H:%M:%S GMT")
        .ok()
        .map(|fecha| fecha.and_utc().timestamp())
}

/// ETag débil: no garantiza igualdad byte a byte, pero cambia si cambia el
/// tamaño o la fecha de modificación (suficiente para caché condicional).
fn calcular_etag(tamano: u64, mtime_ms: i64) -> String {
    format!("W/\"{tamano:x}-{mtime_ms:x}\"")
}

/// `true` si la petición ya tiene una copia válida en caché (`If-None-Match`
/// con el mismo `ETag`, o `If-Modified-Since` no anterior a `mtime_ms`) y
/// debe responderse `304 Not Modified` sin cuerpo.
fn coincide_no_modificado(
    cabeceras_peticion: &[(String, String)],
    etag: &str,
    mtime_ms: i64,
) -> bool {
    if let Some(valor) = buscar_cabecera(cabeceras_peticion, "If-None-Match") {
        return valor.trim() == "*" || valor.split(',').any(|parte| parte.trim() == etag);
    }
    if let Some(valor) = buscar_cabecera(cabeceras_peticion, "If-Modified-Since")
        && let Some(segundos) = analizar_fecha_http(valor)
    {
        return mtime_ms / 1000 <= segundos;
    }
    false
}

/// Resultado de interpretar la cabecera `Range` contra el tamaño real del
/// archivo.
enum Rango {
    /// Sin `Range`, o con varios rangos (`bytes=0-1,2-3`): no soportado, se
    /// sirve el archivo completo con `200`.
    Ninguno,
    /// Un solo rango válido `inicio..=fin` (ambos inclusive).
    Valido(u64, u64),
    /// `Range` presente pero fuera de los límites del archivo: `416`.
    Invalido,
}

/// Interpreta `Range: bytes=inicio-fin` (también `bytes=inicio-` y
/// `bytes=-sufijo`, las formas abiertas de RFC 7233).
fn analizar_rango(cabecera: Option<&str>, tamano: u64) -> Rango {
    let Some(valor) = cabecera else {
        return Rango::Ninguno;
    };
    let Some(especificacion) = valor.trim().strip_prefix("bytes=") else {
        return Rango::Ninguno;
    };
    if especificacion.contains(',') || tamano == 0 {
        return Rango::Ninguno;
    }
    let Some((inicio_texto, fin_texto)) = especificacion.split_once('-') else {
        return Rango::Invalido;
    };
    let extremos = match (inicio_texto.trim(), fin_texto.trim()) {
        ("", "") => None,
        ("", sufijo) => sufijo
            .parse::<u64>()
            .ok()
            .map(|n| (tamano.saturating_sub(n.min(tamano)), tamano - 1)),
        (inicio, "") => inicio
            .parse::<u64>()
            .ok()
            .map(|inicio| (inicio, tamano - 1)),
        (inicio, fin) => match (inicio.parse::<u64>(), fin.parse::<u64>()) {
            (Ok(inicio), Ok(fin)) => Some((inicio, fin.min(tamano - 1))),
            _ => None,
        },
    };
    match extremos {
        Some((inicio, fin)) if inicio <= fin && inicio < tamano => Rango::Valido(inicio, fin),
        _ => Rango::Invalido,
    }
}

/// Cabeceras extra (`Respuestas.crear`/`fijar_cabecera`) desde una
/// [`CargaNativa::Mapa`], como pares de texto listos para escribir.
fn cabeceras_desde_mapa(carga: Option<&CargaNativa>) -> Vec<(String, String)> {
    match carga {
        Some(CargaNativa::Mapa(pares)) => pares
            .iter()
            .filter_map(|(nombre, valor)| match valor {
                CargaNativa::Texto(texto) => Some((nombre.clone(), texto.clone())),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Responde una petición con `Respuestas.archivo(...)` (o una ruta
/// estática): resuelve caché condicional (`304`), `Range` (`206`/`416`) y
/// transmite el cuerpo del disco al socket sin cargarlo completo en
/// memoria. Corre en el hilo de la conexión.
fn responder_archivo(
    flujo: &mut TcpStream,
    cabeceras_peticion: &[(String, String)],
    campos: Vec<(String, CargaNativa)>,
    omitir_cuerpo: bool,
) {
    let mapa: HashMap<String, CargaNativa> = campos.into_iter().collect();
    let texto_de = |clave: &str| match mapa.get(clave) {
        Some(CargaNativa::Texto(texto)) => texto.clone(),
        _ => String::new(),
    };
    let entero_de = |clave: &str| match mapa.get(clave) {
        Some(CargaNativa::Entero(numero)) => *numero,
        _ => 0,
    };

    let ruta = texto_de("ruta");
    let mtime_ms = entero_de("mtime_ms");
    let etag = texto_de("etag");
    let mime = texto_de("mime");
    let mut cabeceras_extra = cabeceras_desde_mapa(mapa.get("cabeceras"));
    let fecha_modificacion = formatear_fecha_http(mtime_ms);

    if coincide_no_modificado(cabeceras_peticion, &etag, mtime_ms) {
        let mut cabeceras = vec![
            ("ETag".to_string(), etag),
            ("Last-Modified".to_string(), fecha_modificacion),
            ("Accept-Ranges".to_string(), "bytes".to_string()),
        ];
        cabeceras.append(&mut cabeceras_extra);
        let _ = escribir_encabezados(flujo, 304, &cabeceras);
        return;
    }

    let mut flujo_archivo = match FlujoArchivo::abrir(&ruta) {
        Ok(fa) => fa,
        Err(causa) => {
            let bytes = construir_bytes_http(
                500,
                &[(
                    "Content-Type".to_string(),
                    "text/plain; charset=utf-8".to_string(),
                )],
                format!("no se pudo transmitir el archivo: {causa}").as_bytes(),
            );
            let _ = flujo.write_all(&bytes);
            return;
        }
    };
    let tamano = flujo_archivo.tamaño();

    let tipo_contenido = cabeceras_extra
        .iter()
        .find(|(nombre, _)| nombre.eq_ignore_ascii_case("content-type"))
        .map(|(_, valor)| valor.clone())
        .unwrap_or(mime);

    match analizar_rango(buscar_cabecera(cabeceras_peticion, "Range"), tamano) {
        Rango::Valido(inicio, fin) => {
            let longitud = fin - inicio + 1;
            if flujo_archivo.buscar(inicio).is_err() {
                let _ = escribir_encabezados(flujo, 500, &[]);
                return;
            }
            let mut cabeceras = vec![
                ("Content-Type".to_string(), tipo_contenido),
                ("Content-Length".to_string(), longitud.to_string()),
                (
                    "Content-Range".to_string(),
                    format!("bytes {inicio}-{fin}/{tamano}"),
                ),
                ("Accept-Ranges".to_string(), "bytes".to_string()),
                ("ETag".to_string(), etag),
                ("Last-Modified".to_string(), fecha_modificacion),
            ];
            cabeceras.append(&mut cabeceras_extra);
            if !omitir_cuerpo && escribir_encabezados(flujo, 206, &cabeceras).is_ok() {
                let _ = transmitir_flujo(&mut flujo_archivo, flujo, longitud);
            } else if omitir_cuerpo {
                let _ = escribir_encabezados(flujo, 206, &cabeceras);
            }
        }
        Rango::Invalido => {
            let cabeceras = vec![
                ("Content-Range".to_string(), format!("bytes */{tamano}")),
                ("Accept-Ranges".to_string(), "bytes".to_string()),
            ];
            let _ = escribir_encabezados(flujo, 416, &cabeceras);
        }
        Rango::Ninguno => {
            let mut cabeceras = vec![
                ("Content-Type".to_string(), tipo_contenido),
                ("Content-Length".to_string(), tamano.to_string()),
                ("Accept-Ranges".to_string(), "bytes".to_string()),
                ("ETag".to_string(), etag),
                ("Last-Modified".to_string(), fecha_modificacion),
            ];
            cabeceras.append(&mut cabeceras_extra);
            if !omitir_cuerpo && escribir_encabezados(flujo, 200, &cabeceras).is_ok() {
                let _ = transmitir_flujo(&mut flujo_archivo, flujo, tamano);
            } else if omitir_cuerpo {
                let _ = escribir_encabezados(flujo, 200, &cabeceras);
            }
        }
    }
}

/// Responde una petición con `Respuestas.flujo(...)`: escribe las cabeceras
/// con `Transfer-Encoding: chunked` y, por cada trozo, pide al hilo de la VM
/// (mismo despachador `servidor_http`, evento `"trozo"`) el siguiente valor
/// que produzca la función generadora de Quetzal, hasta que devuelva `nulo`.
/// Corre en el hilo de la conexión.
fn responder_flujo(
    flujo: &mut TcpStream,
    id_recurso: u64,
    manija: &ManijaBucle,
    campos: Vec<(String, CargaNativa)>,
    omitir_cuerpo: bool,
) {
    let mapa: HashMap<String, CargaNativa> = campos.into_iter().collect();
    let estado = match mapa.get("estado") {
        Some(CargaNativa::Entero(numero)) => (*numero).clamp(100, 599) as u16,
        _ => 200,
    };
    let Some(CargaNativa::Entero(id_flujo)) = mapa.get("id_flujo") else {
        return;
    };
    let id_flujo = *id_flujo;

    let mut cabeceras = cabeceras_desde_mapa(mapa.get("cabeceras"));
    if !cabeceras
        .iter()
        .any(|(nombre, _)| nombre.eq_ignore_ascii_case("content-type"))
    {
        cabeceras.push((
            "Content-Type".to_string(),
            "application/octet-stream".to_string(),
        ));
    }
    cabeceras.push(("Transfer-Encoding".to_string(), "chunked".to_string()));

    if escribir_encabezados(flujo, estado, &cabeceras).is_err() {
        return;
    }

    if !omitir_cuerpo {
        loop {
            let (respuesta_tx, respuesta_rx) = mpsc::channel();
            manija.enviar(Mensaje::Solicitud {
                servicio: SERVICIO.to_string(),
                id_recurso,
                datos: CargaNativa::Mapa(vec![
                    (
                        "evento".to_string(),
                        CargaNativa::Texto("trozo".to_string()),
                    ),
                    ("id_flujo".to_string(), CargaNativa::Entero(id_flujo)),
                ]),
                respuesta: respuesta_tx,
            });
            let Ok(CargaNativa::Bytes(trozo)) = respuesta_rx.recv_timeout(Duration::from_secs(30))
            else {
                break;
            };
            if !trozo.is_empty() {
                let cabecera_trozo = format!("{:x}\r\n", trozo.len());
                let escrito = flujo.write_all(cabecera_trozo.as_bytes()).is_ok()
                    && flujo.write_all(&trozo).is_ok()
                    && flujo.write_all(b"\r\n").is_ok();
                if !escrito {
                    break;
                }
            }
        }
        let _ = flujo.write_all(b"0\r\n\r\n");
    }

    // Limpieza defensiva: si el bucle terminó por desconexión, error de
    // escritura o tiempo de espera agotado (no porque el generador devolvió
    // `nulo`), el generador puede seguir registrado en el hilo de la VM; se
    // avisa para liberarlo y no filtrar memoria.
    let (respuesta_tx, _) = mpsc::channel();
    manija.enviar(Mensaje::Solicitud {
        servicio: SERVICIO.to_string(),
        id_recurso,
        datos: CargaNativa::Mapa(vec![
            (
                "evento".to_string(),
                CargaNativa::Texto("cerrar_flujo".to_string()),
            ),
            ("id_flujo".to_string(), CargaNativa::Entero(id_flujo)),
        ]),
        respuesta: respuesta_tx,
    });
}

/// Reconstruye los bytes HTTP a partir de la [`CargaNativa`] que produjo el
/// despachador (en el hilo de la VM). Corre en el hilo de la conexión: solo
/// usa tipos `Send`, sin volver a tocar la VM.
fn serializar_respuesta_cruda(carga: CargaNativa, omitir_cuerpo: bool) -> Vec<u8> {
    let CargaNativa::Instancia { campos, .. } = carga else {
        return construir_bytes_http(
            500,
            &[(
                "Content-Type".to_string(),
                "text/plain; charset=utf-8".to_string(),
            )],
            b"error interno del servidor",
        );
    };
    let mut estado = 200u16;
    let mut cabeceras = Vec::new();
    let mut cuerpo = Vec::new();
    for (clave, valor) in campos {
        match (clave.as_str(), valor) {
            ("estado", CargaNativa::Entero(numero)) => estado = numero.clamp(100, 599) as u16,
            ("cabeceras", CargaNativa::Mapa(pares)) => {
                for (nombre, valor) in pares {
                    if let CargaNativa::Texto(texto) = valor {
                        cabeceras.push((nombre, texto));
                    }
                }
            }
            ("cuerpo", CargaNativa::Bytes(bytes)) => cuerpo = bytes,
            _ => {}
        }
    }
    if omitir_cuerpo {
        if !cabeceras
            .iter()
            .any(|(nombre, _)| nombre.eq_ignore_ascii_case("content-length"))
        {
            cabeceras.push(("Content-Length".to_string(), cuerpo.len().to_string()));
        }
        construir_bytes_http(estado, &cabeceras, &[])
    } else {
        construir_bytes_http(estado, &cabeceras, &cuerpo)
    }
}

fn manejar_conexion(flujo: TcpStream, id_recurso: u64, manija: ManijaBucle, limites: LimitesHttp) {
    let _ = flujo.set_nonblocking(false);
    let copia_lectura = match flujo.try_clone() {
        Ok(copia) => copia,
        Err(_) => return,
    };
    let peticion = match leer_peticion(copia_lectura, limites) {
        Ok(Some(peticion)) => peticion,
        Ok(None) => return,
        Err(fallo) => {
            let mut flujo = flujo;
            let bytes = construir_bytes_http(
                fallo.estado,
                &[(
                    "Content-Type".to_string(),
                    "text/plain; charset=utf-8".to_string(),
                )],
                fallo.mensaje.as_bytes(),
            );
            let _ = flujo.write_all(&bytes);
            let _ = flujo.flush();
            return;
        }
    };
    let omitir_cuerpo = peticion.metodo.eq_ignore_ascii_case("HEAD");
    let (ruta, consulta) = match peticion.objetivo.split_once('?') {
        Some((ruta, consulta)) => (ruta.to_string(), analizar_consulta(consulta)),
        None => (peticion.objetivo.clone(), Vec::new()),
    };
    // Se necesitan más adelante (Range, If-None-Match...) para resolver una
    // respuesta de archivo, pero `peticion` se consume al construir `datos`.
    let cabeceras_peticion = peticion.cabeceras.clone();
    let consulta_cruda = consulta
        .iter()
        .map(|(nombre, valor)| {
            CargaNativa::Mapa(vec![
                ("nombre".to_string(), CargaNativa::Texto(nombre.clone())),
                ("valor".to_string(), CargaNativa::Texto(valor.clone())),
            ])
        })
        .collect();
    let cabeceras_crudas = peticion
        .cabeceras
        .iter()
        .map(|(nombre, valor)| {
            CargaNativa::Mapa(vec![
                ("nombre".to_string(), CargaNativa::Texto(nombre.clone())),
                ("valor".to_string(), CargaNativa::Texto(valor.clone())),
            ])
        })
        .collect();

    let datos = CargaNativa::Instancia {
        tipo: TIPO_PETICION_CRUDA.to_string(),
        campos: vec![
            ("metodo".to_string(), CargaNativa::Texto(peticion.metodo)),
            ("ruta".to_string(), CargaNativa::Texto(ruta)),
            (
                "protocolo".to_string(),
                CargaNativa::Texto(peticion.protocolo),
            ),
            (
                "peso_declarado".to_string(),
                peticion
                    .peso_declarado
                    .map(|peso| CargaNativa::Entero(peso.min(i64::MAX as u64) as i64))
                    .unwrap_or(CargaNativa::Nula),
            ),
            (
                "consulta".to_string(),
                CargaNativa::Mapa(
                    consulta
                        .into_iter()
                        .map(|(clave, valor)| (clave, CargaNativa::Texto(valor)))
                        .collect(),
                ),
            ),
            (
                "consulta_cruda".to_string(),
                CargaNativa::Lista(consulta_cruda),
            ),
            (
                "cabeceras".to_string(),
                CargaNativa::Mapa(
                    peticion
                        .cabeceras
                        .into_iter()
                        .map(|(clave, valor)| (clave, CargaNativa::Texto(valor)))
                        .collect(),
                ),
            ),
            (
                "cabeceras_crudas".to_string(),
                CargaNativa::Lista(cabeceras_crudas),
            ),
            ("cuerpo".to_string(), CargaNativa::Bytes(peticion.cuerpo)),
        ],
    };

    let (respuesta_tx, respuesta_rx) = mpsc::channel();
    manija.enviar(Mensaje::Solicitud {
        servicio: SERVICIO.to_string(),
        id_recurso,
        datos,
        respuesta: respuesta_tx,
    });
    let respuesta = respuesta_rx
        .recv_timeout(Duration::from_secs(30))
        .unwrap_or(CargaNativa::Nula);
    let mut flujo = flujo;
    match respuesta {
        CargaNativa::Instancia { tipo, campos } if tipo == TIPO_RESPUESTA_ARCHIVO => {
            responder_archivo(&mut flujo, &cabeceras_peticion, campos, omitir_cuerpo);
        }
        CargaNativa::Instancia { tipo, campos } if tipo == TIPO_RESPUESTA_FLUJO => {
            responder_flujo(&mut flujo, id_recurso, &manija, campos, omitir_cuerpo);
        }
        otra => {
            let bytes = serializar_respuesta_cruda(otra, omitir_cuerpo);
            let _ = flujo.write_all(&bytes);
        }
    }
    let _ = flujo.flush();
}

// =====================================================================
// Despachador (hilo de la VM): enrutamiento e invocación del manejador
// =====================================================================

/// Segmentos no vacíos de una ruta (`/a/b/` -> `["a", "b"]`).
fn segmentos(ruta: &str) -> Vec<&str> {
    ruta.trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect()
}

/// Si `patron` coincide con `ruta` (soporta `:nombre` como comodín),
/// devuelve los parámetros capturados.
fn coincide_ruta(patron: &str, ruta: &str) -> Option<IndexMap<String, Valor>> {
    let patron_segmentos = segmentos(patron);
    let ruta_segmentos = segmentos(ruta);
    if patron_segmentos.len() != ruta_segmentos.len() {
        return None;
    }
    let mut parametros = IndexMap::new();
    for (segmento_patron, segmento_ruta) in patron_segmentos.iter().zip(ruta_segmentos.iter()) {
        if let Some(nombre) = segmento_patron.strip_prefix(':') {
            parametros.insert(nombre.to_string(), Valor::texto(*segmento_ruta));
        } else if segmento_patron != segmento_ruta {
            return None;
        }
    }
    Some(parametros)
}

/// Busca la primera ruta registrada que coincida con `metodo` y `ruta`.
/// Devuelve `(manejador, parametros, interceptores_de_ruta)`.
fn buscar_ruta(
    instancia: &DatosInstanciaNativa,
    metodo: &str,
    ruta: &str,
) -> Option<(Valor, IndexMap<String, Valor>, Valor)> {
    let datos = instancia.datos.borrow();
    let Some(Valor::Lista(rutas)) = datos.get("rutas") else {
        return None;
    };
    for entrada in rutas.borrow().iter() {
        let Valor::Jsn(mapa) = entrada else { continue };
        let mapa = mapa.borrow();
        let coincide_metodo = matches!(mapa.get("metodo"), Some(Valor::Texto(m)) if &**m == "*" || m.eq_ignore_ascii_case(metodo));
        if !coincide_metodo {
            continue;
        }
        let Some(Valor::Texto(patron)) = mapa.get("patron") else {
            continue;
        };
        if let Some(parametros) = coincide_ruta(patron, ruta) {
            let manejador = mapa.get("manejador").cloned().unwrap_or(Valor::Nulo);
            let interceptores = mapa
                .get("interceptores")
                .cloned()
                .unwrap_or_else(|| Valor::lista(Vec::new()));
            return Some((manejador, parametros, interceptores));
        }
    }
    None
}

fn metodos_permitidos(instancia: &DatosInstanciaNativa, ruta: &str) -> Vec<String> {
    let datos = instancia.datos.borrow();
    let Some(Valor::Lista(rutas)) = datos.get("rutas") else {
        return Vec::new();
    };
    let mut metodos = Vec::new();
    for entrada in rutas.borrow().iter() {
        let Valor::Jsn(mapa) = entrada else { continue };
        let mapa = mapa.borrow();
        let Some(Valor::Texto(patron)) = mapa.get("patron") else {
            continue;
        };
        if coincide_ruta(patron, ruta).is_none() {
            continue;
        }
        let Some(Valor::Texto(metodo)) = mapa.get("metodo") else {
            continue;
        };
        if &**metodo == "*" {
            return Vec::new();
        }
        let metodo = metodo.to_ascii_uppercase();
        if !metodos.contains(&metodo) {
            metodos.push(metodo);
        }
    }
    if metodos.iter().any(|metodo| metodo == "GET")
        && !metodos.iter().any(|metodo| metodo == "HEAD")
    {
        metodos.push("HEAD".to_string());
    }
    if !metodos.is_empty() && !metodos.iter().any(|metodo| metodo == "OPTIONS") {
        metodos.push("OPTIONS".to_string());
    }
    metodos.sort_unstable();
    metodos
}

fn respuesta_metodo_no_permitido(metodos: &[String]) -> CargaNativa {
    construir_carga_respuesta(
        405,
        Some("text/plain; charset=utf-8"),
        vec![("Allow".to_string(), metodos.join(", "))],
        b"metodo HTTP no permitido para esta ruta".to_vec(),
    )
}

fn respuesta_opciones(metodos: &[String]) -> CargaNativa {
    construir_carga_respuesta(
        204,
        None,
        vec![("Allow".to_string(), metodos.join(", "))],
        Vec::new(),
    )
}

fn invocar_manejador(vm: &mut Vm, manejador: &Valor, peticion: Valor) -> Result<Valor, Fallo> {
    match manejador {
        Valor::Funcion(funcion, entorno) => {
            vm.llamar_funcion(funcion, entorno, vec![peticion], None, None)
        }
        otro => Err(error(
            "E0406",
            format!(
                "el manejador de la ruta no es una función válida (tipo '{}')",
                otro.nombre_tipo()
            ),
        )),
    }
}

/// Llama un item de la cadena (interceptor o manejador) con el número de
/// argumentos adecuado: 1 parámetro -> (peticion), 2 -> (peticion, respuesta),
/// 3 -> (peticion, respuesta, siguiente). Así los manejadores existentes que
/// solo aceptan `peticion` siguen funcionando sin cambios.
fn invocar_item_cadena(
    vm: &mut Vm,
    item: &Valor,
    peticion: Valor,
    respuesta: &Valor,
    siguiente: &Valor,
) -> Result<Valor, Fallo> {
    match item {
        Valor::Funcion(funcion, entorno) => {
            let num_params = funcion.parametros.len();
            let argumentos = match num_params {
                1 => vec![peticion],
                2 => vec![peticion, respuesta.clone()],
                3 => vec![peticion, respuesta.clone(), siguiente.clone()],
                n => {
                    return Err(error(
                        "E0406",
                        format!(
                            "el interceptor/manejador espera 1, 2 o 3 parámetros, pero declara {n}"
                        ),
                    ));
                }
            };
            vm.llamar_funcion(funcion, entorno, argumentos, None, None)
        }
        otro => Err(error(
            "E0406",
            format!(
                "el item de la cadena no es una función (tipo '{}')",
                otro.nombre_tipo()
            ),
        )),
    }
}

/// Crea la función nativa `__siguiente` que continúa la cadena de
/// interceptores. Cuando un interceptor llama `siguiente(peticion, respuesta)`,
/// esta función saca el próximo item del estado compartido y lo invoca.
fn crear_siguiente(estado: EstadoCadenaCompartido) -> maquina_virtual::FuncionNativaConVm {
    Box::new(move |vm, argumentos| {
        if argumentos.len() < 2 {
            return Err(error("E0210", "'siguiente' espera (peticion, respuesta)"));
        }
        let peticion = argumentos[0].clone();
        let respuesta = argumentos[1].clone();
        let siguiente = Valor::Nativa(Rc::from("__siguiente"));
        let id_cadena = match &peticion {
            Valor::InstanciaNativa(instancia) if &*instancia.tipo == TIPO_PETICION => {
                match instancia.datos.borrow().get("id_cadena") {
                    Some(Valor::Entero(id)) => *id as u64,
                    _ => {
                        return Err(error(
                            "E0704",
                            "la petición no pertenece a una cadena de interceptores",
                        ));
                    }
                }
            }
            _ => {
                return Err(error(
                    "E0704",
                    "'siguiente' recibió una petición HTTP inválida",
                ));
            }
        };

        let item = {
            let mut estado_ref = estado.borrow_mut();
            let Some(est) = estado_ref.remove(&id_cadena) else {
                return Err(error(
                    "E0704",
                    "no hay más interceptores en la cadena (siguiente llamada fuera de contexto)",
                ));
            };
            if est.restantes.is_empty() {
                return Err(error("E0704", "no hay más interceptores en la cadena"));
            }
            let mut restantes = est.restantes;
            let item = restantes.remove(0);
            if !restantes.is_empty() {
                estado_ref.insert(id_cadena, EstadoCadena { restantes });
            }
            item
        };

        invocar_item_cadena(vm, &item, peticion, &respuesta, &siguiente)
    })
}

/// Ejecuta la cadena de interceptores + manejador para una petición.
/// Construye la cadena como `[interceptores_globales..., interceptores_de_ruta...,
/// manejador]`, pone el estado compartido y llama al primer item.
fn ejecutar_cadena(
    vm: &mut Vm,
    estado: &EstadoCadenaCompartido,
    interceptores_globales: &[Valor],
    interceptores_ruta: &[Valor],
    manejador: &Valor,
    peticion: Valor,
) -> Result<Valor, Fallo> {
    let mut cadena: Vec<Valor> = Vec::new();
    cadena.extend(interceptores_globales.iter().cloned());
    cadena.extend(interceptores_ruta.iter().cloned());
    cadena.push(manejador.clone());

    if cadena.len() == 1 {
        // Sin interceptores: invocar manejador directamente.
        return invocar_manejador(vm, &cadena[0], peticion);
    }

    let mut restantes = cadena;
    let primer_item = restantes.remove(0);
    let siguiente = Valor::Nativa(Rc::from("__siguiente"));

    // Crear respuesta por defecto para los interceptores (200, sin cuerpo).
    let mut datos_resp = IndexMap::new();
    datos_resp.insert("estado".to_string(), Valor::Entero(200));
    datos_resp.insert("cabeceras".to_string(), Valor::jsn(IndexMap::new()));
    datos_resp.insert("cuerpo".to_string(), Valor::Nulo);
    let respuesta = Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_RESPUESTA_SERVIDOR),
        datos: RefCell::new(datos_resp),
    }));

    let id_cadena = vm.bucle().nuevo_id();
    if let Valor::InstanciaNativa(instancia) = &peticion {
        instancia
            .datos
            .borrow_mut()
            .insert("id_cadena".to_string(), Valor::Entero(id_cadena as i64));
    }
    estado
        .borrow_mut()
        .insert(id_cadena, EstadoCadena { restantes });
    let resultado = invocar_item_cadena(vm, &primer_item, peticion, &respuesta, &siguiente);
    estado.borrow_mut().remove(&id_cadena);
    resultado
}

fn construir_carga_respuesta(
    estado: i64,
    tipo_contenido: Option<&str>,
    mut cabeceras: Vec<(String, String)>,
    cuerpo: Vec<u8>,
) -> CargaNativa {
    if let Some(tipo) = tipo_contenido
        && !cabeceras
            .iter()
            .any(|(nombre, _)| nombre.eq_ignore_ascii_case("content-type"))
    {
        cabeceras.push(("Content-Type".to_string(), tipo.to_string()));
    }
    CargaNativa::Instancia {
        tipo: TIPO_RESPUESTA_CRUDA.to_string(),
        campos: vec![
            ("estado".to_string(), CargaNativa::Entero(estado)),
            (
                "cabeceras".to_string(),
                CargaNativa::Mapa(
                    cabeceras
                        .into_iter()
                        .map(|(clave, valor)| (clave, CargaNativa::Texto(valor)))
                        .collect(),
                ),
            ),
            ("cuerpo".to_string(), CargaNativa::Bytes(cuerpo)),
        ],
    }
}

/// Cabeceras extra puestas con `RespuestaServidor.fijar_cabecera(...)`,
/// convertidas a la representación que cruza al hilo de la conexión.
fn cabeceras_extra_a_carga(datos: &IndexMap<String, Valor>) -> CargaNativa {
    let mut cabeceras = Vec::new();
    if let Some(Valor::Jsn(mapa)) = datos.get("cabeceras") {
        for (clave, valor) in mapa.borrow().iter() {
            match valor {
                Valor::Lista(valores) => {
                    cabeceras.extend(valores.borrow().iter().map(|valor| {
                        (
                            clave.clone(),
                            CargaNativa::Texto(maquina_virtual::texto_de_valor(valor)),
                        )
                    }));
                }
                valor => cabeceras.push((
                    clave.clone(),
                    CargaNativa::Texto(maquina_virtual::texto_de_valor(valor)),
                )),
            }
        }
    }
    CargaNativa::Mapa(cabeceras)
}

/// Convierte una `RespuestaServidor` de `Respuestas.archivo(...)` (o de una
/// ruta estática) en la carga que cruza al hilo de la conexión: solo la
/// ruta y los metadatos viajan, el archivo se transmite del disco al socket
/// allá (ver [`responder_archivo`]).
fn respuesta_archivo_a_carga(datos: &IndexMap<String, Valor>) -> CargaNativa {
    let texto_de = |clave: &str| match datos.get(clave) {
        Some(Valor::Texto(texto)) => texto.to_string(),
        _ => String::new(),
    };
    let entero_de = |clave: &str| match datos.get(clave) {
        Some(Valor::Entero(numero)) => *numero,
        _ => 0,
    };
    CargaNativa::Instancia {
        tipo: TIPO_RESPUESTA_ARCHIVO.to_string(),
        campos: vec![
            (
                "ruta".to_string(),
                CargaNativa::Texto(texto_de("archivo_ruta")),
            ),
            (
                "tamano".to_string(),
                CargaNativa::Entero(entero_de("archivo_tamano")),
            ),
            (
                "mtime_ms".to_string(),
                CargaNativa::Entero(entero_de("archivo_mtime_ms")),
            ),
            (
                "etag".to_string(),
                CargaNativa::Texto(texto_de("archivo_etag")),
            ),
            (
                "mime".to_string(),
                CargaNativa::Texto(texto_de("archivo_mime")),
            ),
            ("cabeceras".to_string(), cabeceras_extra_a_carga(datos)),
        ],
    }
}

/// Convierte una `RespuestaServidor` de `Respuestas.flujo(...)` en la carga
/// que cruza al hilo de la conexión: la función generadora (`Rc`, no
/// `Send`) se queda registrada en `registro_flujos` bajo un `id_flujo`
/// nuevo, y solo ese identificador viaja.
fn respuesta_flujo_a_carga(
    vm: &mut Vm,
    registro_flujos: &RegistroFlujos,
    datos: &IndexMap<String, Valor>,
) -> CargaNativa {
    let estado = match datos.get("estado") {
        Some(Valor::Entero(numero)) => *numero,
        _ => 200,
    };
    let generador = datos.get("generador").cloned().unwrap_or(Valor::Nulo);
    let cabeceras = cabeceras_extra_a_carga(datos);
    let id_flujo = vm.bucle().nuevo_id();
    registro_flujos.borrow_mut().insert(id_flujo, generador);
    CargaNativa::Instancia {
        tipo: TIPO_RESPUESTA_FLUJO.to_string(),
        campos: vec![
            ("estado".to_string(), CargaNativa::Entero(estado)),
            ("id_flujo".to_string(), CargaNativa::Entero(id_flujo as i64)),
            ("cabeceras".to_string(), cabeceras),
        ],
    }
}

/// Convierte lo que devolvió el manejador de Quetzal en una respuesta cruda:
/// una instancia `RespuestaServidor` (control total, incluyendo
/// `Respuestas.archivo`/`Respuestas.flujo`) o directamente texto, jsn,
/// `Bits` o nulo (200 implícito, tipo de contenido inferido).
fn respuesta_desde_valor(
    vm: &mut Vm,
    registro_flujos: &RegistroFlujos,
    valor: &Valor,
) -> CargaNativa {
    if let Valor::InstanciaNativa(instancia) = valor
        && &*instancia.tipo == TIPO_RESPUESTA_SERVIDOR
    {
        let datos = instancia.datos.borrow();
        if matches!(datos.get("es_archivo"), Some(Valor::Log(true))) {
            return respuesta_archivo_a_carga(&datos);
        }
        if matches!(datos.get("es_flujo"), Some(Valor::Log(true))) {
            return respuesta_flujo_a_carga(vm, registro_flujos, &datos);
        }
        let estado = match datos.get("estado") {
            Some(Valor::Entero(numero)) => *numero,
            _ => 200,
        };
        let cabeceras = match cabeceras_extra_a_carga(&datos) {
            CargaNativa::Mapa(cabeceras) => cabeceras
                .into_iter()
                .filter_map(|(nombre, valor)| match valor {
                    CargaNativa::Texto(valor) => Some((nombre, valor)),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        let cuerpo_valor = datos.get("cuerpo").cloned().unwrap_or(Valor::Nulo);
        let (bytes, tipo_contenido) = bytes_y_tipo_contenido("RespuestaServidor", &cuerpo_valor)
            .unwrap_or_else(|_| {
                (
                    maquina_virtual::texto_de_valor(&cuerpo_valor).into_bytes(),
                    Some("text/plain; charset=utf-8"),
                )
            });
        return construir_carga_respuesta(estado, tipo_contenido, cabeceras, bytes);
    }

    let (bytes, tipo_contenido) = match bytes_y_tipo_contenido("el manejador", valor) {
        Ok(par) => par,
        Err(_) => (
            maquina_virtual::texto_de_valor(valor).into_bytes(),
            Some("text/plain; charset=utf-8"),
        ),
    };
    construir_carga_respuesta(200, tipo_contenido, Vec::new(), bytes)
}

fn respuesta_desde_error(fallo: Fallo) -> CargaNativa {
    let mensaje = match fallo {
        Fallo::Excepcion(datos) => datos.mensaje,
        Fallo::Error(error) => error.mensaje,
    };
    construir_carga_respuesta(
        500,
        Some("text/plain; charset=utf-8"),
        Vec::new(),
        format!("error interno: {mensaje}").into_bytes(),
    )
}

fn respuesta_no_encontrada(metodo: &str, ruta: &str) -> CargaNativa {
    construir_carga_respuesta(
        404,
        Some("text/plain; charset=utf-8"),
        Vec::new(),
        format!("ruta no encontrada: {metodo} {ruta}").into_bytes(),
    )
}

struct DatosPeticionHttp {
    metodo: String,
    ruta: String,
    protocolo: String,
    peso_declarado: Valor,
    parametros: IndexMap<String, Valor>,
    consulta: Valor,
    consulta_cruda: Valor,
    cabeceras: Valor,
    cabeceras_crudas: Valor,
    cuerpo: Valor,
}

fn construir_peticion_http(campos: DatosPeticionHttp) -> Valor {
    let mut datos = IndexMap::new();
    datos.insert("metodo".to_string(), Valor::texto(campos.metodo));
    datos.insert("ruta".to_string(), Valor::texto(campos.ruta));
    datos.insert("protocolo".to_string(), Valor::texto(campos.protocolo));
    datos.insert("peso_declarado".to_string(), campos.peso_declarado);
    datos.insert("parametros".to_string(), Valor::jsn(campos.parametros));
    datos.insert("consulta".to_string(), campos.consulta);
    datos.insert("consulta_cruda".to_string(), campos.consulta_cruda);
    datos.insert("cabeceras".to_string(), campos.cabeceras);
    datos.insert("cabeceras_crudas".to_string(), campos.cabeceras_crudas);
    datos.insert("cuerpo".to_string(), campos.cuerpo);
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_PETICION),
        datos: RefCell::new(datos),
    }))
}

/// Pide al generador de `Respuestas.flujo(...)` registrado bajo `id_flujo`
/// su siguiente trozo (invocando la función de Quetzal, sin argumentos).
/// `nulo` termina el flujo y libera el registro; cualquier otro valor se
/// convierte a bytes (texto o `Bits`) como un trozo más.
fn siguiente_trozo(vm: &mut Vm, registro_flujos: &RegistroFlujos, id_flujo: u64) -> CargaNativa {
    let Some(generador) = registro_flujos.borrow().get(&id_flujo).cloned() else {
        return CargaNativa::Nula;
    };
    let Valor::Funcion(funcion, entorno) = &generador else {
        registro_flujos.borrow_mut().remove(&id_flujo);
        return CargaNativa::Nula;
    };
    match vm.llamar_funcion(funcion, entorno, Vec::new(), None, None) {
        Ok(Valor::Nulo) => {
            registro_flujos.borrow_mut().remove(&id_flujo);
            CargaNativa::Nula
        }
        Ok(valor) => match bytes_y_tipo_contenido("Respuestas.flujo", &valor) {
            Ok((bytes, _)) => CargaNativa::Bytes(bytes),
            Err(_) => CargaNativa::Bytes(maquina_virtual::texto_de_valor(&valor).into_bytes()),
        },
        Err(_fallo) => {
            registro_flujos.borrow_mut().remove(&id_flujo);
            CargaNativa::Nula
        }
    }
}

/// Atiende los eventos internos del hilo de conexión de una respuesta por
/// trozos (`"trozo"`: siguiente valor del generador; `"cerrar_flujo"`:
/// limpieza al terminar o desconectarse), distintos de una petición HTTP
/// normal (que llega como [`CargaNativa::Instancia`], no como
/// [`CargaNativa::Mapa`]).
fn despachar_evento_flujo(
    vm: &mut Vm,
    registro_flujos: &RegistroFlujos,
    campos: Vec<(String, CargaNativa)>,
) -> CargaNativa {
    let mapa: HashMap<String, CargaNativa> = campos.into_iter().collect();
    let Some(CargaNativa::Entero(id_flujo)) = mapa.get("id_flujo") else {
        return CargaNativa::Nula;
    };
    let id_flujo = *id_flujo as u64;
    match mapa.get("evento") {
        Some(CargaNativa::Texto(evento)) if evento == "trozo" => {
            siguiente_trozo(vm, registro_flujos, id_flujo)
        }
        Some(CargaNativa::Texto(evento)) if evento == "cerrar_flujo" => {
            registro_flujos.borrow_mut().remove(&id_flujo);
            CargaNativa::Nula
        }
        _ => CargaNativa::Nula,
    }
}

/// Busca `ruta` entre las carpetas registradas con `ServidorHttp.estaticos`
/// (solo para `GET`, y solo si ninguna ruta explícita coincidió antes: esas
/// tienen prioridad). La protección contra *path traversal* la hace
/// [`GuardianPermisos::verificar_lectura`] al canonicalizar la ruta final:
/// si queda fuera de los directorios declarados en `quetzal.json`, se
/// deniega igual que cualquier otra lectura de archivo.
fn intentar_estatico(
    guardian: &GuardianPermisos,
    instancia: &DatosInstanciaNativa,
    ruta: &str,
) -> Option<CargaNativa> {
    let datos = instancia.datos.borrow();
    let Some(Valor::Lista(lista)) = datos.get("estaticos") else {
        return None;
    };
    let segmentos_ruta = segmentos(ruta);
    for entrada in lista.borrow().iter() {
        let Valor::Jsn(mapa) = entrada else { continue };
        let mapa = mapa.borrow();
        let (Some(Valor::Texto(prefijo)), Some(Valor::Texto(directorio))) =
            (mapa.get("prefijo"), mapa.get("directorio"))
        else {
            continue;
        };
        let segmentos_prefijo = segmentos(prefijo);
        if segmentos_ruta.len() < segmentos_prefijo.len()
            || segmentos_ruta[..segmentos_prefijo.len()] != segmentos_prefijo[..]
        {
            continue;
        }
        let resto = segmentos_ruta[segmentos_prefijo.len()..].join("/");
        let ruta_completa = if resto.is_empty() {
            directorio.to_string()
        } else {
            format!("{directorio}/{resto}")
        };
        if guardian.verificar_lectura(&ruta_completa).is_err() {
            continue;
        }
        let Ok((tamano, mtime_ms, es_archivo)) = metadatos_archivo(&ruta_completa) else {
            continue;
        };
        if !es_archivo {
            continue;
        }
        return Some(CargaNativa::Instancia {
            tipo: TIPO_RESPUESTA_ARCHIVO.to_string(),
            campos: vec![
                (
                    "ruta".to_string(),
                    CargaNativa::Texto(ruta_completa.clone()),
                ),
                ("tamano".to_string(), CargaNativa::Entero(tamano as i64)),
                ("mtime_ms".to_string(), CargaNativa::Entero(mtime_ms)),
                (
                    "etag".to_string(),
                    CargaNativa::Texto(calcular_etag(tamano, mtime_ms)),
                ),
                (
                    "mime".to_string(),
                    CargaNativa::Texto(tipo_contenido(&ruta_completa)),
                ),
                ("cabeceras".to_string(), CargaNativa::Mapa(Vec::new())),
            ],
        });
    }
    None
}

/// Despachador único del servicio `"servidor_http"`: corre en el hilo de la
/// VM (`Vm::despachar_solicitud`) para cada petición entrante de cualquier
/// `ServidorHttp` en escucha, identificado por `id_recurso`, y también para
/// los eventos internos de una respuesta por trozos (`Respuestas.flujo`).
fn despachar_peticion(
    vm: &mut Vm,
    id_recurso: u64,
    registro_servidores: &RegistroServidores,
    registro_flujos: &RegistroFlujos,
    guardian: &Rc<GuardianPermisos>,
    estado_cadena: &EstadoCadenaCompartido,
    datos: CargaNativa,
) -> CargaNativa {
    // Los eventos de un flujo por trozos (`"trozo"`/`"cerrar_flujo"`) viajan
    // como `Mapa`; una petición HTTP normal siempre llega como `Instancia`
    // (`_peticion_cruda`), así que no hay ambigüedad posible.
    let CargaNativa::Instancia { .. } = &datos else {
        let CargaNativa::Mapa(campos) = datos else {
            return CargaNativa::Nula;
        };
        return despachar_evento_flujo(vm, registro_flujos, campos);
    };

    let Some(instancia) = registro_servidores.borrow().get(&id_recurso).cloned() else {
        return construir_carga_respuesta(
            503,
            Some("text/plain; charset=utf-8"),
            Vec::new(),
            b"el servidor ya no esta disponible".to_vec(),
        );
    };

    let peticion_cruda = maquina_virtual::carga_a_valor(datos);
    let Valor::InstanciaNativa(cruda) = &peticion_cruda else {
        return respuesta_desde_error(error_servidor(
            "no se pudo interpretar la petición entrante",
        ));
    };
    let (
        metodo,
        ruta,
        protocolo,
        peso_declarado,
        consulta,
        consulta_cruda,
        cabeceras,
        cabeceras_crudas,
        cuerpo,
    ) = {
        let datos_cruda = cruda.datos.borrow();
        let metodo = match datos_cruda.get("metodo") {
            Some(Valor::Texto(texto)) => texto.to_string(),
            _ => String::new(),
        };
        let ruta = match datos_cruda.get("ruta") {
            Some(Valor::Texto(texto)) => texto.to_string(),
            _ => "/".to_string(),
        };
        let protocolo = match datos_cruda.get("protocolo") {
            Some(Valor::Texto(texto)) => texto.to_string(),
            _ => "HTTP/1.1".to_string(),
        };
        let peso_declarado = datos_cruda
            .get("peso_declarado")
            .cloned()
            .unwrap_or(Valor::Nulo);
        let consulta = datos_cruda
            .get("consulta")
            .cloned()
            .unwrap_or_else(|| Valor::jsn(IndexMap::new()));
        let consulta_cruda = datos_cruda
            .get("consulta_cruda")
            .cloned()
            .unwrap_or_else(|| Valor::lista(Vec::new()));
        let cabeceras = datos_cruda
            .get("cabeceras")
            .cloned()
            .unwrap_or_else(|| Valor::jsn(IndexMap::new()));
        let cabeceras_crudas = datos_cruda
            .get("cabeceras_crudas")
            .cloned()
            .unwrap_or_else(|| Valor::lista(Vec::new()));
        let cuerpo = datos_cruda.get("cuerpo").cloned().unwrap_or(Valor::Nulo);
        (
            metodo,
            ruta,
            protocolo,
            peso_declarado,
            consulta,
            consulta_cruda,
            cabeceras,
            cabeceras_crudas,
            cuerpo,
        )
    };

    // Verificar límite global de cuerpo (ServidorHttp.limite_cuerpo).
    let limite = match instancia.datos.borrow().get("limite_cuerpo") {
        Some(Valor::Entero(n)) => Some(*n),
        _ => None,
    };
    if let Some(limite) = limite {
        let peso = match &cuerpo {
            Valor::InstanciaNativa(i) if &*i.tipo == "Bits" => {
                match i.datos.borrow().get("datos") {
                    Some(Valor::Lista(lista)) => lista.borrow().len() as i64,
                    _ => 0,
                }
            }
            _ => 0,
        };
        if peso > limite {
            return construir_carga_respuesta(
                413,
                Some("text/plain; charset=utf-8"),
                Vec::new(),
                b"cuerpo de la peticion demasiado grande".to_vec(),
            );
        }
    }

    let ruta_encontrada = buscar_ruta(&instancia, &metodo, &ruta).or_else(|| {
        metodo
            .eq_ignore_ascii_case("HEAD")
            .then(|| buscar_ruta(&instancia, "GET", &ruta))
            .flatten()
    });
    let Some((manejador, parametros, interceptores_ruta)) = ruta_encontrada else {
        if (metodo.eq_ignore_ascii_case("GET") || metodo.eq_ignore_ascii_case("HEAD"))
            && let Some(carga) = intentar_estatico(guardian, &instancia, &ruta)
        {
            return carga;
        }
        let permitidos = metodos_permitidos(&instancia, &ruta);
        if metodo.eq_ignore_ascii_case("OPTIONS") && !permitidos.is_empty() {
            return respuesta_opciones(&permitidos);
        }
        if !permitidos.is_empty() {
            return respuesta_metodo_no_permitido(&permitidos);
        }
        return respuesta_no_encontrada(&metodo, &ruta);
    };

    let peticion_http = construir_peticion_http(DatosPeticionHttp {
        metodo,
        ruta,
        protocolo,
        peso_declarado,
        parametros,
        consulta,
        consulta_cruda,
        cabeceras,
        cabeceras_crudas,
        cuerpo,
    });

    // Recoger interceptores globales del servidor.
    let interceptores_globales: Vec<Valor> = match instancia.datos.borrow().get("interceptores") {
        Some(Valor::Lista(lista)) => lista.borrow().clone(),
        _ => Vec::new(),
    };
    let interceptores_ruta_vec: Vec<Valor> = match &interceptores_ruta {
        Valor::Lista(lista) => lista.borrow().clone(),
        _ => Vec::new(),
    };

    let resultado = ejecutar_cadena(
        vm,
        estado_cadena,
        &interceptores_globales,
        &interceptores_ruta_vec,
        &manejador,
        peticion_http.clone(),
    );

    match resultado {
        Ok(valor) => respuesta_desde_valor(vm, registro_flujos, &valor),
        Err(fallo) => {
            // Si hay manejador de errores, invocarlo.
            let manejador_errores = instancia.datos.borrow().get("manejador_errores").cloned();
            if let Some(Valor::Funcion(..)) = &manejador_errores {
                let mensaje = match &fallo {
                    Fallo::Excepcion(datos) => datos.mensaje.clone(),
                    Fallo::Error(e) => e.mensaje.clone(),
                };
                let peticion_para_error = peticion_http;
                if let Some(Valor::Funcion(funcion, entorno)) = &manejador_errores {
                    let r = vm.llamar_funcion(
                        funcion,
                        entorno,
                        vec![Valor::texto(mensaje), peticion_para_error],
                        None,
                        None,
                    );
                    if let Ok(valor_error) = r {
                        return respuesta_desde_valor(vm, registro_flujos, &valor_error);
                    }
                }
            }
            respuesta_desde_error(fallo)
        }
    }
}

// =====================================================================
// PeticionHttp: métodos de solo lectura
// =====================================================================

fn receptor_peticion(
    funcion: &str,
    argumentos: &[Valor],
) -> Result<Rc<DatosInstanciaNativa>, Fallo> {
    match argumentos.first() {
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == TIPO_PETICION => {
            Ok(Rc::clone(instancia))
        }
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba un PeticionHttp, pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
        None => Err(error("E0210", format!("'{funcion}' necesita el receptor"))),
    }
}

fn campo_peticion(funcion: &str, argumentos: &[Valor], campo: &str) -> Result<Valor, Fallo> {
    let instancia = receptor_peticion(funcion, argumentos)?;
    Ok(instancia
        .datos
        .borrow()
        .get(campo)
        .cloned()
        .unwrap_or(Valor::Nulo))
}

fn metodo_peticion_metodo(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.metodo";
    exigir_aridad(F, &argumentos[1..], 0)?;
    campo_peticion(F, argumentos, "metodo")
}

fn metodo_peticion_ruta(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.ruta";
    exigir_aridad(F, &argumentos[1..], 0)?;
    campo_peticion(F, argumentos, "ruta")
}

fn metodo_peticion_protocolo(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.protocolo";
    exigir_aridad(F, &argumentos[1..], 0)?;
    campo_peticion(F, argumentos, "protocolo")
}

fn metodo_peticion_peso_declarado(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.peso_declarado";
    exigir_aridad(F, &argumentos[1..], 0)?;
    campo_peticion(F, argumentos, "peso_declarado")
}

fn metodo_peticion_parametros(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.parametros";
    exigir_aridad(F, &argumentos[1..], 0)?;
    campo_peticion(F, argumentos, "parametros")
}

fn metodo_peticion_consulta(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.consulta";
    exigir_aridad(F, &argumentos[1..], 0)?;
    campo_peticion(F, argumentos, "consulta")
}

fn metodo_peticion_cabeceras(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.cabeceras";
    exigir_aridad(F, &argumentos[1..], 0)?;
    campo_peticion(F, argumentos, "cabeceras")
}

/// `PeticionHttp.cabecera(nombre)`: busca una cabecera sin distinguir
/// mayúsculas/minúsculas (`Content-Type`, `content-type`... son la misma
/// cabecera) y devuelve su valor como `texto`, o `nulo` si no está.
fn metodo_peticion_cabecera(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.cabecera";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let nombre = arg_texto(F, argumentos, 1)?.to_string();
    let cabeceras = campo_peticion(F, argumentos, "cabeceras")?;
    if let Valor::Jsn(mapa) = cabeceras {
        for (clave, valor) in mapa.borrow().iter() {
            if clave.eq_ignore_ascii_case(&nombre) {
                return Ok(valor.clone());
            }
        }
    }
    Ok(Valor::Nulo)
}

fn valores_repetidos_peticion(
    funcion: &str,
    argumentos: &[Valor],
    campo: &str,
    nombre: &str,
    ignorar_mayusculas: bool,
) -> Result<Valor, Fallo> {
    let pares = campo_peticion(funcion, argumentos, campo)?;
    let Valor::Lista(pares) = pares else {
        return Ok(Valor::lista(Vec::new()));
    };
    let mut valores = Vec::new();
    for par in pares.borrow().iter() {
        let Valor::Jsn(par) = par else { continue };
        let par = par.borrow();
        let Some(Valor::Texto(nombre_actual)) = par.get("nombre") else {
            continue;
        };
        let coincide = if ignorar_mayusculas {
            nombre_actual.eq_ignore_ascii_case(nombre)
        } else {
            &**nombre_actual == nombre
        };
        if coincide && let Some(valor) = par.get("valor") {
            valores.push(valor.clone());
        }
    }
    Ok(Valor::lista(valores))
}

fn metodo_peticion_cabeceras_todas(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.cabeceras_todas";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let nombre = arg_texto(F, argumentos, 1)?;
    valores_repetidos_peticion(F, argumentos, "cabeceras_crudas", nombre, true)
}

fn metodo_peticion_consulta_todas(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.consulta_todas";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let nombre = arg_texto(F, argumentos, 1)?;
    valores_repetidos_peticion(F, argumentos, "consulta_cruda", nombre, false)
}

fn metodo_peticion_bits(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.bits";
    exigir_aridad(F, &argumentos[1..], 0)?;
    campo_peticion(F, argumentos, "cuerpo")
}

fn bytes_cuerpo_peticion(funcion: &str, argumentos: &[Valor]) -> Result<Vec<u8>, Fallo> {
    let cuerpo = campo_peticion(funcion, argumentos, "cuerpo")?;
    maquina_virtual::bytes_de_bits(&cuerpo).ok_or_else(|| {
        error(
            "E0406",
            format!("'{funcion}' recibió una PeticionHttp sin cuerpo binario válido"),
        )
    })
}

fn metodo_peticion_texto(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.texto";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let bytes = bytes_cuerpo_peticion(F, argumentos)?;
    String::from_utf8(bytes).map(Valor::texto).map_err(|_| {
        error(
            "E0406",
            format!("'{F}' no pudo decodificar el cuerpo como texto UTF-8 válido"),
        )
    })
}

fn metodo_peticion_jsn(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.jsn";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let bytes = bytes_cuerpo_peticion(F, argumentos)?;
    let texto = String::from_utf8(bytes).map_err(|_| {
        error(
            "E0406",
            format!("'{F}' no pudo decodificar el cuerpo como texto UTF-8 válido"),
        )
    })?;
    let json: serde_json::Value = serde_json::from_str(&texto).map_err(|causa| {
        error(
            "E0406",
            format!("'{F}': el cuerpo no es JSON válido: {causa}"),
        )
    })?;
    Ok(maquina_virtual::valores::json_a_valor(&json))
}

// =====================================================================
// Multipart/form-data y formulario urlencoded
// =====================================================================

const TIPO_PARTE_MULTIPARTE: &str = "ParteMultiparte";

/// Extrae el boundary de una cabecera `Content-Type: multipart/form-data;
/// boundary=...`. Devuelve `None` si no es multipart o no tiene boundary.
fn boundary_de_content_type(tipo: &str) -> Option<String> {
    if !tipo.to_ascii_lowercase().contains("multipart/form-data") {
        return None;
    }
    for parte in tipo.split(';') {
        let parte = parte.trim();
        if let Some(valor) = parte.strip_prefix("boundary=") {
            let valor = valor.trim().trim_matches('"');
            return Some(valor.to_string());
        }
    }
    None
}

/// Una parte de multipart ya parseada: nombre del campo, nombre del archivo
/// (si es un archivo), tipo de contenido y bytes del cuerpo.
struct ParteMultipart {
    nombre: String,
    nombre_archivo: Option<String>,
    tipo_contenido: String,
    datos: Vec<u8>,
}

/// Parsea un cuerpo `multipart/form-data` a partir del boundary y los bytes.
fn parsear_multipart(boundary: &str, cuerpo: &[u8]) -> Result<Vec<ParteMultipart>, Fallo> {
    let separador = format!("--{boundary}");
    let separador_bytes = separador.as_bytes();
    let mut partes = Vec::new();

    // El cuerpo empieza con `--boundary\r\n` y termina con `--boundary--`.
    let mut posicion = 0;
    while posicion < cuerpo.len() {
        // Buscar el siguiente `--boundary`
        let Some(inicio) = cuerpo[posicion..]
            .windows(separador_bytes.len())
            .position(|w| w == separador_bytes)
        else {
            break;
        };
        let inicio_abs = posicion + inicio;
        let despues_separador = inicio_abs + separador_bytes.len();

        // Verificar si es el final (`--boundary--`)
        if despues_separador + 2 <= cuerpo.len()
            && &cuerpo[despues_separador..despues_separador + 2] == b"--"
        {
            break;
        }

        // Saltar `\r\n` después del boundary
        let inicio_parte = if despues_separador + 2 <= cuerpo.len()
            && &cuerpo[despues_separador..despues_separador + 2] == b"\r\n"
        {
            despues_separador + 2
        } else {
            despues_separador
        };

        // Buscar el siguiente `--boundary` (fin de esta parte)
        posicion = inicio_parte;
        let Some(fin_relativo) = cuerpo[posicion..]
            .windows(separador_bytes.len())
            .position(|w| w == separador_bytes)
        else {
            break;
        };
        let fin_parte = posicion + fin_relativo;

        // Quitar `\r\n` finales antes del boundary
        let fin_datos = if fin_parte >= 2 && &cuerpo[fin_parte - 2..fin_parte] == b"\r\n" {
            fin_parte - 2
        } else {
            fin_parte
        };

        let contenido_parte = &cuerpo[inicio_parte..fin_datos];
        let parte = parsear_una_parte(contenido_parte)?;
        partes.push(parte);
        posicion = fin_parte;
    }

    Ok(partes)
}

/// Parsea una parte individual: cabeceras + `\r\n\r\n` + cuerpo.
fn parsear_una_parte(datos: &[u8]) -> Result<ParteMultipart, Fallo> {
    let texto = String::from_utf8_lossy(datos);
    let Some(separador_pos) = texto.find("\r\n\r\n") else {
        return Err(error("E0707", "parte multipart sin separador de cabeceras"));
    };
    let cabeceras_texto = &texto[..separador_pos];
    let cuerpo = &datos[separador_pos + 4..];

    let mut nombre = String::new();
    let mut nombre_archivo = None;
    let mut tipo_contenido = "text/plain".to_string();

    for linea in cabeceras_texto.split("\r\n") {
        if let Some((clave, valor)) = linea.split_once(':') {
            let clave = clave.trim().to_ascii_lowercase();
            let valor = valor.trim();
            if clave == "content-disposition" {
                // form-data; name="campo"; filename="archivo.txt"
                for parte in valor.split(';') {
                    let parte = parte.trim();
                    if let Some(n) = parte.strip_prefix("name=") {
                        nombre = n.trim_matches('"').to_string();
                    } else if let Some(f) = parte.strip_prefix("filename=") {
                        nombre_archivo = Some(f.trim_matches('"').to_string());
                    }
                }
            } else if clave == "content-type" {
                tipo_contenido = valor.to_string();
            }
        }
    }

    Ok(ParteMultipart {
        nombre,
        nombre_archivo,
        tipo_contenido,
        datos: cuerpo.to_vec(),
    })
}

/// Construye una instancia `ParteMultiparte` nativa.
fn nueva_parte_multiparte(parte: ParteMultipart) -> Valor {
    let mut datos = IndexMap::new();
    datos.insert("nombre".to_string(), Valor::texto(parte.nombre));
    datos.insert(
        "nombre_archivo".to_string(),
        match parte.nombre_archivo {
            Some(n) => Valor::texto(n),
            None => Valor::Nulo,
        },
    );
    datos.insert(
        "tipo_contenido".to_string(),
        Valor::texto(parte.tipo_contenido),
    );
    datos.insert("datos".to_string(), crate::bits::valor_bits(&parte.datos));
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_PARTE_MULTIPARTE),
        datos: RefCell::new(datos),
    }))
}

fn receptor_parte_multiparte(
    funcion: &str,
    argumentos: &[Valor],
) -> Result<Rc<DatosInstanciaNativa>, Fallo> {
    match argumentos.first() {
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == TIPO_PARTE_MULTIPARTE => {
            Ok(Rc::clone(instancia))
        }
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba una ParteMultiparte, pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
        None => Err(error("E0210", format!("'{funcion}' necesita el receptor"))),
    }
}

fn metodo_parte_nombre(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ParteMultiparte.nombre";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let instancia = receptor_parte_multiparte(F, argumentos)?;
    Ok(instancia
        .datos
        .borrow()
        .get("nombre")
        .cloned()
        .unwrap_or(Valor::Nulo))
}

fn metodo_parte_nombre_archivo(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ParteMultiparte.nombre_archivo";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let instancia = receptor_parte_multiparte(F, argumentos)?;
    Ok(instancia
        .datos
        .borrow()
        .get("nombre_archivo")
        .cloned()
        .unwrap_or(Valor::Nulo))
}

fn metodo_parte_tipo_contenido(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ParteMultiparte.tipo_contenido";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let instancia = receptor_parte_multiparte(F, argumentos)?;
    Ok(instancia
        .datos
        .borrow()
        .get("tipo_contenido")
        .cloned()
        .unwrap_or(Valor::Nulo))
}

fn metodo_parte_bits(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ParteMultiparte.bits";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let instancia = receptor_parte_multiparte(F, argumentos)?;
    Ok(instancia
        .datos
        .borrow()
        .get("datos")
        .cloned()
        .unwrap_or(Valor::Nulo))
}

fn metodo_parte_texto(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ParteMultiparte.texto";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let instancia = receptor_parte_multiparte(F, argumentos)?;
    let datos = instancia.datos.borrow();
    match datos.get("datos") {
        Some(Valor::InstanciaNativa(bits)) if &*bits.tipo == "Bits" => {
            match bits.datos.borrow().get("datos") {
                Some(Valor::Lista(lista)) => {
                    let bytes: Vec<u8> = lista
                        .borrow()
                        .iter()
                        .map(|v| match v {
                            Valor::Entero(n) if (0..=255).contains(n) => *n as u8,
                            _ => 0,
                        })
                        .collect();
                    String::from_utf8(bytes).map(Valor::texto).map_err(|_| {
                        error(
                            "E0406",
                            format!("'{F}': los datos no son texto UTF-8 válido"),
                        )
                    })
                }
                _ => Err(error("E0406", format!("'{F}': parte sin datos internos"))),
            }
        }
        _ => Err(error(
            "E0406",
            format!("'{F}': la parte no tiene datos binarios"),
        )),
    }
}

/// `PeticionHttp.partes() -> lista<ParteMultiparte>`: parsea el cuerpo
/// `multipart/form-data` y devuelve todas las partes.
fn metodo_peticion_partes(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.partes";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let instancia = receptor_peticion(F, argumentos)?;
    let datos_peticion = instancia.datos.borrow();
    let cabeceras = datos_peticion
        .get("cabeceras")
        .cloned()
        .unwrap_or(Valor::Nulo);
    let cuerpo = datos_peticion.get("cuerpo").cloned().unwrap_or(Valor::Nulo);
    drop(datos_peticion);

    // Buscar Content-Type en las cabeceras
    let tipo_content = buscar_cabecera_peticion(&cabeceras, "content-type");
    let Some(boundary) = tipo_content.as_deref().and_then(boundary_de_content_type) else {
        return Err(error("E0707", "la petición no es multipart/form-data"));
    };

    let bytes = maquina_virtual::bytes_de_bits(&cuerpo).ok_or_else(|| {
        error(
            "E0707",
            "no se pudieron leer los bytes del cuerpo multipart",
        )
    })?;

    let partes = parsear_multipart(&boundary, &bytes)?;
    let lista: Vec<Valor> = partes.into_iter().map(nueva_parte_multiparte).collect();
    Ok(Valor::lista(lista))
}

/// `PeticionHttp.archivos() -> jsn`: parsea multipart y devuelve un jsn
/// con los campos de archivo (los que tienen `filename`). Cada entrada es
/// `{nombre, tipo_contenido, bits}`.
fn metodo_peticion_archivos(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.archivos";
    if !argumentos.is_empty() && argumentos.len() != 1 {
        return Err(error_aridad(F, 0, argumentos.len() - 1));
    }
    let instancia = receptor_peticion(F, argumentos)?;
    let datos_peticion = instancia.datos.borrow();
    let cabeceras = datos_peticion
        .get("cabeceras")
        .cloned()
        .unwrap_or(Valor::Nulo);
    let cuerpo = datos_peticion.get("cuerpo").cloned().unwrap_or(Valor::Nulo);
    drop(datos_peticion);

    let tipo_content = buscar_cabecera_peticion(&cabeceras, "content-type");
    let Some(boundary) = tipo_content.as_deref().and_then(boundary_de_content_type) else {
        return Err(error("E0707", "la petición no es multipart/form-data"));
    };

    let bytes = maquina_virtual::bytes_de_bits(&cuerpo).ok_or_else(|| {
        error(
            "E0707",
            "no se pudieron leer los bytes del cuerpo multipart",
        )
    })?;

    let partes = parsear_multipart(&boundary, &bytes)?;
    let mut resultado = IndexMap::new();
    for parte in partes {
        if parte.nombre_archivo.is_some() {
            let mut entrada = IndexMap::new();
            entrada.insert(
                "nombre".to_string(),
                Valor::texto(parte.nombre_archivo.unwrap_or_default()),
            );
            entrada.insert(
                "tipo_contenido".to_string(),
                Valor::texto(parte.tipo_contenido),
            );
            entrada.insert("bits".to_string(), crate::bits::valor_bits(&parte.datos));
            resultado.insert(parte.nombre, Valor::jsn(entrada));
        }
    }
    Ok(Valor::jsn(resultado))
}

/// `PeticionHttp.formulario() -> jsn`: parsea `application/x-www-form-urlencoded`
/// y devuelve un jsn con los pares campo/valor.
fn metodo_peticion_formulario(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.formulario";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let instancia = receptor_peticion(F, argumentos)?;
    let datos_peticion = instancia.datos.borrow();
    let cuerpo = datos_peticion.get("cuerpo").cloned().unwrap_or(Valor::Nulo);
    drop(datos_peticion);

    let bytes = maquina_virtual::bytes_de_bits(&cuerpo)
        .ok_or_else(|| error("E0406", "no se pudo leer el cuerpo del formulario"))?;
    let texto = String::from_utf8(bytes)
        .map_err(|_| error("E0406", "el cuerpo del formulario no es texto válido"))?;

    let mut resultado = IndexMap::new();
    for par in texto.split('&') {
        if par.is_empty() {
            continue;
        }
        let (clave, valor) = match par.split_once('=') {
            Some((c, v)) => (c, v),
            None => (par, ""),
        };
        let clave_dec = percent_encoding::percent_decode_str(&clave.replace('+', " "))
            .decode_utf8_lossy()
            .into_owned();
        let valor_dec = percent_encoding::percent_decode_str(&valor.replace('+', " "))
            .decode_utf8_lossy()
            .into_owned();
        resultado.insert(clave_dec, Valor::texto(valor_dec));
    }
    Ok(Valor::jsn(resultado))
}

/// Busca una cabecera en el `jsn` de cabeceras de la petición.
fn buscar_cabecera_peticion(cabeceras: &Valor, nombre: &str) -> Option<String> {
    if let Valor::Jsn(mapa) = cabeceras {
        for (clave, valor) in mapa.borrow().iter() {
            if clave.eq_ignore_ascii_case(nombre)
                && let Valor::Texto(t) = valor
            {
                return Some(t.to_string());
            }
        }
    }
    None
}

// =====================================================================
// Galletas (cookies)
// =====================================================================

/// `PeticionHttp.galletas() -> jsn`: parsea la cabecera `Cookie` y devuelve
/// un jsn con los pares nombre/valor.
fn metodo_peticion_galletas(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.galletas";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let cabeceras = campo_peticion(F, argumentos, "cabeceras")?;
    let cookie_header = buscar_cabecera_peticion(&cabeceras, "cookie").unwrap_or_default();
    let mut resultado = IndexMap::new();
    for par in cookie_header.split(';') {
        let par = par.trim();
        if let Some((nombre, valor)) = par.split_once('=') {
            resultado.insert(nombre.trim().to_string(), Valor::texto(valor.trim()));
        }
    }
    Ok(Valor::jsn(resultado))
}

/// `RespuestaServidor.fijar_galleta(nombre, valor, opciones?)`: añade una
/// cabecera `Set-Cookie`. Opciones: `{caducidad, max_edad, ruta, dominio,
/// solo_http, seguro, mismo_sitio}`.
fn metodo_fijar_galleta(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "RespuestaServidor.fijar_galleta";
    if argumentos.len() < 3 || argumentos.len() > 4 {
        return Err(error_aridad(F, 2, argumentos.len() - 1));
    }
    let instancia = receptor_respuesta_servidor(F, argumentos)?;
    let nombre = arg_texto(F, argumentos, 1)?.to_string();
    let valor = arg_texto(F, argumentos, 2)?.to_string();
    let mut galleta = format!("{nombre}={valor}");

    if argumentos.len() == 4 {
        let opciones = arg_jsn(F, argumentos, 3)?;
        let opciones = opciones.borrow();
        if let Some(Valor::Texto(t)) = opciones.get("caducidad") {
            galleta.push_str(&format!("; Expires={t}"));
        }
        if let Some(Valor::Entero(n)) = opciones.get("max_edad") {
            galleta.push_str(&format!("; Max-Age={n}"));
        }
        if let Some(Valor::Texto(t)) = opciones.get("ruta") {
            galleta.push_str(&format!("; Path={t}"));
        }
        if let Some(Valor::Texto(t)) = opciones.get("dominio") {
            galleta.push_str(&format!("; Domain={t}"));
        }
        if let Some(Valor::Log(true)) = opciones.get("solo_http") {
            galleta.push_str("; HttpOnly");
        }
        if let Some(Valor::Log(true)) = opciones.get("seguro") {
            galleta.push_str("; Secure");
        }
        if let Some(Valor::Texto(t)) = opciones.get("mismo_sitio") {
            galleta.push_str(&format!("; SameSite={t}"));
        }
    }

    match instancia.datos.borrow().get("cabeceras") {
        Some(Valor::Jsn(mapa)) => {
            agregar_cabecera(&mut mapa.borrow_mut(), "Set-Cookie".to_string(), galleta);
        }
        _ => {
            return Err(error(
                "E0406",
                format!("'{F}' recibió un RespuestaServidor sin cabeceras internas"),
            ));
        }
    }
    Ok(Valor::Nulo)
}

/// `RespuestaServidor.borrar_galleta(nombre, opciones?)`: envía una
/// `Set-Cookie` con `Max-Age=0` para que el cliente elimine la galleta.
fn metodo_borrar_galleta(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "RespuestaServidor.borrar_galleta";
    if argumentos.len() < 2 || argumentos.len() > 3 {
        return Err(error_aridad(F, 1, argumentos.len() - 1));
    }
    let instancia = receptor_respuesta_servidor(F, argumentos)?;
    let nombre = arg_texto(F, argumentos, 1)?.to_string();
    let mut galleta = format!("{nombre}=; Max-Age=0");
    // Las opciones deben coincidir con las de fijar_galleta (path, domain)
    if argumentos.len() == 3 {
        let opciones = arg_jsn(F, argumentos, 2)?;
        let opciones = opciones.borrow();
        if let Some(Valor::Texto(t)) = opciones.get("ruta") {
            galleta.push_str(&format!("; Path={t}"));
        }
        if let Some(Valor::Texto(t)) = opciones.get("dominio") {
            galleta.push_str(&format!("; Domain={t}"));
        }
    }
    match instancia.datos.borrow().get("cabeceras") {
        Some(Valor::Jsn(mapa)) => {
            agregar_cabecera(&mut mapa.borrow_mut(), "Set-Cookie".to_string(), galleta);
        }
        _ => {
            return Err(error(
                "E0406",
                format!("'{F}' recibió un RespuestaServidor sin cabeceras internas"),
            ));
        }
    }
    Ok(Valor::Nulo)
}

// =====================================================================
// Negociación de contenido
// =====================================================================

/// Divide una cabecera de aceptación (`Accept`, `Accept-Language`, ...) en
/// sus partes y devuelve el primer tipo que coincide con los aceptados.
fn negociar(cabecera: &str, candidatos: &[&str]) -> Option<String> {
    let aceptados: Vec<&str> = cabecera
        .split(',')
        .map(|s| s.trim().split(';').next().unwrap_or("").trim())
        .collect();
    for candidato in candidatos {
        for aceptado in &aceptados {
            if *aceptado == "*" || aceptado == candidato {
                return Some(candidato.to_string());
            }
            // Coincidencia con wildcard (text/* -> text/plain)
            if let Some(prefijo) = aceptado.strip_suffix("/*")
                && candidato.starts_with(prefijo)
            {
                return Some(candidato.to_string());
            }
        }
    }
    None
}

/// `PeticionHttp.acepta(tipo) -> texto|log`: negocia el tipo de contenido
/// basado en la cabecera `Accept`. Devuelve el mejor tipo coincidente o
/// `falso` si ninguno coincide.
fn metodo_peticion_acepta(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.acepta";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let tipo = arg_texto(F, argumentos, 1)?;
    let cabeceras = campo_peticion(F, argumentos, "cabeceras")?;
    let accept =
        buscar_cabecera_peticion(&cabeceras, "accept").unwrap_or_else(|| "*/*".to_string());
    let candidatos: Vec<&str> = tipo.split(',').map(|s| s.trim()).collect();
    match negociar(&accept, &candidatos) {
        Some(t) => Ok(Valor::texto(t)),
        None => Ok(Valor::Log(false)),
    }
}

/// `PeticionHttp.acepta_idioma(idioma) -> texto|log`
fn metodo_peticion_acepta_idioma(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.acepta_idioma";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let idioma = arg_texto(F, argumentos, 1)?;
    let cabeceras = campo_peticion(F, argumentos, "cabeceras")?;
    let accept =
        buscar_cabecera_peticion(&cabeceras, "accept-language").unwrap_or_else(|| "*".to_string());
    let candidatos: Vec<&str> = idioma.split(',').map(|s| s.trim()).collect();
    match negociar(&accept, &candidatos) {
        Some(t) => Ok(Valor::texto(t)),
        None => Ok(Valor::Log(false)),
    }
}

/// `PeticionHttp.acepta_codificacion(cod) -> texto|log`
fn metodo_peticion_acepta_codificacion(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.acepta_codificacion";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let cod = arg_texto(F, argumentos, 1)?;
    let cabeceras = campo_peticion(F, argumentos, "cabeceras")?;
    let accept =
        buscar_cabecera_peticion(&cabeceras, "accept-encoding").unwrap_or_else(|| "*".to_string());
    let candidatos: Vec<&str> = cod.split(',').map(|s| s.trim()).collect();
    match negociar(&accept, &candidatos) {
        Some(t) => Ok(Valor::texto(t)),
        None => Ok(Valor::Log(false)),
    }
}

/// `PeticionHttp.es(tipo) -> texto|log|nulo`: verifica el `Content-Type`
/// de la petición. Devuelve el tipo coincidente, `falso` si no coincide,
/// o `nulo` si no hay cuerpo.
fn metodo_peticion_es(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.es";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let tipo = arg_texto(F, argumentos, 1)?.to_ascii_lowercase();
    let cabeceras = campo_peticion(F, argumentos, "cabeceras")?;
    let content_type = buscar_cabecera_peticion(&cabeceras, "content-type");
    match content_type {
        Some(ct) => {
            let ct_lower = ct.to_ascii_lowercase();
            let ct_base = ct_lower.split(';').next().unwrap_or("").trim();
            let tipo_base = tipo.split(';').next().unwrap_or("").trim();
            if ct_base == tipo_base {
                Ok(Valor::texto(ct))
            } else if tipo.contains('*') {
                //Wildcard: text/* matches text/plain
                let prefijo = tipo_base.strip_suffix("/*").unwrap_or(tipo_base);
                if ct_base.starts_with(prefijo) {
                    Ok(Valor::texto(ct))
                } else {
                    Ok(Valor::Log(false))
                }
            } else {
                Ok(Valor::Log(false))
            }
        }
        None => Ok(Valor::Nulo),
    }
}

// =====================================================================
// Enrutador (grupos de rutas)
// =====================================================================

const TIPO_ENRUTADOR: &str = "Enrutador";

fn nuevo_enrutador() -> Valor {
    let mut datos = IndexMap::new();
    datos.insert("rutas".to_string(), Valor::lista(Vec::new()));
    datos.insert("interceptores".to_string(), Valor::lista(Vec::new()));
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_ENRUTADOR),
        datos: RefCell::new(datos),
    }))
}

fn constructor_enrutador(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    if argumentos.len() > 1 {
        return Err(error_aridad("nuevo Enrutador", 0, argumentos.len()));
    }
    Ok(nuevo_enrutador())
}

fn receptor_enrutador(
    funcion: &str,
    argumentos: &[Valor],
) -> Result<Rc<DatosInstanciaNativa>, Fallo> {
    match argumentos.first() {
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == TIPO_ENRUTADOR => {
            Ok(Rc::clone(instancia))
        }
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba un Enrutador, pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
        None => Err(error("E0210", format!("'{funcion}' necesita el receptor"))),
    }
}

fn agregar_ruta_enrutador(
    instancia: &DatosInstanciaNativa,
    metodo: &str,
    patron: &str,
    manejador: Valor,
) {
    let mut entrada = IndexMap::new();
    entrada.insert("metodo".to_string(), Valor::texto(metodo));
    entrada.insert("patron".to_string(), Valor::texto(patron));
    entrada.insert("manejador".to_string(), manejador);
    entrada.insert("interceptores".to_string(), Valor::lista(Vec::new()));
    if let Some(Valor::Lista(rutas)) = instancia.datos.borrow().get("rutas") {
        rutas.borrow_mut().push(Valor::jsn(entrada));
    }
}

fn metodo_enrutador_fija(
    funcion: &'static str,
    metodo_http: reqwest::Method,
) -> maquina_virtual::FuncionNativa {
    Box::new(move |argumentos| {
        if argumentos.len() < 3 || argumentos.len() > 4 {
            return Err(error_aridad(funcion, 2, argumentos.len() - 1));
        }
        let instancia = receptor_enrutador(funcion, argumentos)?;
        let patron = arg_texto(funcion, argumentos, 1)?.to_string();
        let manejador = exigir_manejador(funcion, &argumentos[argumentos.len() - 1])?;
        agregar_ruta_enrutador(&instancia, metodo_http.as_str(), &patron, manejador);
        Ok(Valor::Nulo)
    })
}

fn metodo_enrutador_usar(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Enrutador.usar";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let instancia = receptor_enrutador(F, argumentos)?;
    let interceptor = exigir_manejador(F, &argumentos[1])?;
    if let Some(Valor::Lista(interceptores)) = instancia.datos.borrow().get("interceptores") {
        interceptores.borrow_mut().push(interceptor);
    }
    Ok(Valor::Nulo)
}

/// `ServidorHttp.montar(prefijo, enrutador)`: copia las rutas del
/// enrutador al servidor, anteponiendo `prefijo` a cada patrón. Los
/// interceptores del enrutador se anteponen a los de cada ruta copiada.
fn metodo_montar(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ServidorHttp.montar";
    exigir_aridad(F, &argumentos[1..], 2)?;
    let servidor = receptor_servidor(F, argumentos)?;
    let prefijo = arg_texto(F, argumentos, 1)?.trim_matches('/').to_string();
    let enrutador = receptor_enrutador(F, &argumentos[2..])?;

    // Copiar interceptores del enrutador
    let interceptores_enrutador: Vec<Valor> = match enrutador.datos.borrow().get("interceptores") {
        Some(Valor::Lista(lista)) => lista.borrow().clone(),
        _ => Vec::new(),
    };

    // Copiar rutas del enrutador al servidor con prefijo
    let datos_enrutador = enrutador.datos.borrow();
    let datos_servidor = servidor.datos.borrow();
    if let (Some(Valor::Lista(rutas_enrutador)), Some(Valor::Lista(rutas_servidor))) =
        (datos_enrutador.get("rutas"), datos_servidor.get("rutas"))
    {
            for entrada in rutas_enrutador.borrow().iter() {
                let Valor::Jsn(mapa) = entrada else { continue };
                let mapa = mapa.borrow();
                let metodo = mapa.get("metodo").cloned().unwrap_or(Valor::Nulo);
                let patron = match mapa.get("patron") {
                    Some(Valor::Texto(t)) => t.to_string(),
                    _ => continue,
                };
                let manejador = mapa.get("manejador").cloned().unwrap_or(Valor::Nulo);
                let interceptores_ruta = mapa
                    .get("interceptores")
                    .cloned()
                    .unwrap_or_else(|| Valor::lista(Vec::new()));

                let patron_completo = if patron.is_empty() || patron == "/" {
                    format!("/{prefijo}")
                } else {
                    format!("/{prefijo}/{patron}")
                };

                // Combinar interceptores del enrutador con los de la ruta
                let mut interceptores_combinados = interceptores_enrutador.clone();
                if let Valor::Lista(lista_ir) = &interceptores_ruta {
                    interceptores_combinados.extend(lista_ir.borrow().iter().cloned());
                }

                let mut nueva_entrada = IndexMap::new();
                nueva_entrada.insert("metodo".to_string(), metodo);
                nueva_entrada.insert("patron".to_string(), Valor::texto(patron_completo));
                nueva_entrada.insert("manejador".to_string(), manejador);
                nueva_entrada.insert(
                    "interceptores".to_string(),
                    Valor::lista(interceptores_combinados),
                );
                rutas_servidor.borrow_mut().push(Valor::jsn(nueva_entrada));
            }
    }
    Ok(Valor::Nulo)
}

// =====================================================================
// Ciclo de vida: al_cerrar, al_listo
// =====================================================================

/// `ServidorHttp.al_cerrar(fun)`: registra un manejador que se invoca
/// cuando se llama `detener()`. Útil para liberar recursos.
fn metodo_al_cerrar(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ServidorHttp.al_cerrar";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let instancia = receptor_servidor(F, argumentos)?;
    let manejador = exigir_manejador(F, &argumentos[1])?;
    instancia
        .datos
        .borrow_mut()
        .insert("al_cerrar".to_string(), manejador);
    Ok(Valor::Nulo)
}

/// `ServidorHttp.al_listo(fun)`: registra un manejador que se invoca
/// cuando el servidor comienza a escuchar.
fn metodo_al_listo(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ServidorHttp.al_listo";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let instancia = receptor_servidor(F, argumentos)?;
    let manejador = exigir_manejador(F, &argumentos[1])?;
    instancia
        .datos
        .borrow_mut()
        .insert("al_listo".to_string(), manejador);
    Ok(Valor::Nulo)
}

/// `PeticionHttp.peso() -> entero`: tamaño del cuerpo en bytes (útil para
/// validar subidas, informar al cliente, o decidir si procesar o rechazar).
fn metodo_peticion_peso(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "PeticionHttp.peso";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let cuerpo = campo_peticion(F, argumentos, "cuerpo")?;
    let peso = match &cuerpo {
        Valor::Nulo => 0i64,
        Valor::InstanciaNativa(instancia) if &*instancia.tipo == "Bits" => {
            match instancia.datos.borrow().get("datos") {
                Some(Valor::Lista(lista)) => lista.borrow().len() as i64,
                _ => 0,
            }
        }
        Valor::Texto(t) => t.len() as i64,
        _ => 0,
    };
    Ok(Valor::Entero(peso))
}

// =====================================================================
// RespuestaServidor: `Respuestas.crear(estado, cuerpo)`
// =====================================================================

fn respuestas_crear(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Respuestas.crear";
    if argumentos.len() < 2 || argumentos.len() > 3 {
        return Err(error_aridad(F, 2, argumentos.len()));
    }
    let estado = arg_entero(F, argumentos, 0)?;
    if !(100..=599).contains(&estado) {
        return Err(error(
            "E0406",
            format!("'{F}' espera un código de estado HTTP entre 100 y 599, pero recibió {estado}"),
        ));
    }
    let mut cabeceras = IndexMap::new();
    let mut datos = IndexMap::new();
    datos.insert("estado".to_string(), Valor::Entero(estado));
    // Procesar opciones ( tercer argumento opcional: jsn con cabeceras, tipo_contenido)
    if argumentos.len() == 3 {
        let opciones = arg_jsn(F, argumentos, 2)?;
        let opciones = opciones.borrow();
        if let Some(Valor::Jsn(cab_opciones)) = opciones.get("cabeceras") {
            for (clave, valor) in cab_opciones.borrow().iter() {
                cabeceras.insert(clave.clone(), valor.clone());
            }
        }
        if let Some(Valor::Texto(tipo)) = opciones.get("tipo_contenido") {
            cabeceras.insert("Content-Type".to_string(), Valor::texto(tipo));
        }
    }
    datos.insert("cabeceras".to_string(), Valor::jsn(cabeceras));
    datos.insert("cuerpo".to_string(), argumentos[1].clone());
    Ok(Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_RESPUESTA_SERVIDOR),
        datos: RefCell::new(datos),
    })))
}

/// Construye una `RespuestaServidor` con estado, cuerpo y tipo de contenido
/// fijos. Utilidad interna compartida por `Respuestas.texto`, `.jsn`, etc.
fn respuesta_simple(estado: i64, cuerpo: Valor, tipo_contenido: &str) -> Valor {
    let mut cabeceras = IndexMap::new();
    cabeceras.insert(
        "Content-Type".to_string(),
        Valor::texto(tipo_contenido),
    );
    let mut datos = IndexMap::new();
    datos.insert("estado".to_string(), Valor::Entero(estado));
    datos.insert("cabeceras".to_string(), Valor::jsn(cabeceras));
    datos.insert("cuerpo".to_string(), cuerpo);
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_RESPUESTA_SERVIDOR),
        datos: RefCell::new(datos),
    }))
}

/// `Respuestas.redirigir(url, opciones?)`: respuesta de redirección con
/// cabecera `Location`. Opciones: `{estado: entero}` (default 302).
fn respuestas_redirigir(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Respuestas.redirigir";
    if argumentos.is_empty() || argumentos.len() > 2 {
        return Err(error_aridad(F, 1, argumentos.len()));
    }
    let url = arg_texto(F, argumentos, 0)?.to_string();
    let estado = if argumentos.len() == 2 {
        let opciones = arg_jsn(F, argumentos, 1)?;
        entero_opcional_de_jsn(F, &opciones.borrow(), "estado")?.unwrap_or(302)
    } else {
        302
    };
    if !(300..=399).contains(&estado) {
        return Err(error(
            "E0406",
            format!(
                "'{F}': el estado de redirección debe estar entre 300 y 399, pero recibió {estado}"
            ),
        ));
    }
    let mut cabeceras = IndexMap::new();
    cabeceras.insert("Location".to_string(), Valor::texto(url));
    let mut datos = IndexMap::new();
    datos.insert("estado".to_string(), Valor::Entero(estado));
    datos.insert("cabeceras".to_string(), Valor::jsn(cabeceras));
    datos.insert("cuerpo".to_string(), Valor::Nulo);
    Ok(Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_RESPUESTA_SERVIDOR),
        datos: RefCell::new(datos),
    })))
}

/// `Respuestas.nada(estado)`: respuesta sin cuerpo (204, 304, ...).
fn respuestas_nada(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Respuestas.nada";
    exigir_aridad(F, argumentos, 1)?;
    let estado = arg_entero(F, argumentos, 0)?;
    if !(100..=599).contains(&estado) {
        return Err(error(
            "E0406",
            format!("'{F}': estado fuera de rango (100-599): {estado}"),
        ));
    }
    let mut datos = IndexMap::new();
    datos.insert("estado".to_string(), Valor::Entero(estado));
    datos.insert("cabeceras".to_string(), Valor::jsn(IndexMap::new()));
    datos.insert("cuerpo".to_string(), Valor::Nulo);
    Ok(Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_RESPUESTA_SERVIDOR),
        datos: RefCell::new(datos),
    })))
}

/// `Respuestas.texto(cuerpo)`: 200 con `Content-Type: text/plain`.
fn respuestas_texto(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Respuestas.texto";
    exigir_aridad(F, argumentos, 1)?;
    Ok(respuesta_simple(
        200,
        argumentos[0].clone(),
        "text/plain; charset=utf-8",
    ))
}

/// `Respuestas.jsn(cuerpo)`: 200 con `Content-Type: application/json`.
fn respuestas_jsn(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Respuestas.jsn";
    exigir_aridad(F, argumentos, 1)?;
    Ok(respuesta_simple(
        200,
        argumentos[0].clone(),
        "application/json",
    ))
}

/// `Respuestas.bits(cuerpo)`: 200 con `Content-Type: application/octet-stream`.
fn respuestas_bits(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Respuestas.bits";
    exigir_aridad(F, argumentos, 1)?;
    Ok(respuesta_simple(
        200,
        argumentos[0].clone(),
        "application/octet-stream",
    ))
}

/// `Respuestas.error(estado, mensaje)`: respuesta de error con cuerpo
/// JSON `{error: mensaje}` y el código de estado indicado.
fn respuestas_error(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Respuestas.error";
    exigir_aridad(F, argumentos, 2)?;
    let estado = arg_entero(F, argumentos, 0)?;
    let mensaje = arg_texto(F, argumentos, 1)?.to_string();
    let mut cuerpo = IndexMap::new();
    cuerpo.insert("error".to_string(), Valor::texto(mensaje));
    Ok(respuesta_simple(
        estado,
        Valor::jsn(cuerpo),
        "application/json",
    ))
}

/// `Respuestas.eventos(generador)`: respuesta de *server-sent events* (SSE).
/// Es un wrapper de `Respuestas.flujo` que fija `Content-Type: text/event-stream`
/// y formatea cada trozo como `data: ...\n\n` automáticamente. El generador
/// debe devolver `texto` (el dato del evento) o `nulo` para terminar.
fn respuestas_eventos(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Respuestas.eventos";
    let (estado, generador) = match argumentos.len() {
        1 => (200i64, argumentos[0].clone()),
        2 => {
            let estado = arg_entero(F, argumentos, 0)?;
            if !(100..=599).contains(&estado) {
                return Err(error(
                    "E0406",
                    format!(
                        "'{F}' espera un código de estado HTTP entre 100 y 599, pero recibió {estado}"
                    ),
                ));
            }
            (estado, argumentos[1].clone())
        }
        recibidos => {
            return Err(error(
                "E0210",
                format!(
                    "'{F}' espera 1 argumento (generador) o 2 (estado, generador), pero recibió {recibidos}"
                ),
            ));
        }
    };
    if !matches!(generador, Valor::Funcion(..)) {
        return Err(error(
            "E0406",
            format!(
                "'{F}' espera una función generadora sin argumentos como manejador, pero recibió '{}'",
                generador.nombre_tipo()
            ),
        ));
    }
    let mut datos = IndexMap::new();
    datos.insert("estado".to_string(), Valor::Entero(estado));
    let mut cabeceras = IndexMap::new();
    cabeceras.insert(
        "Content-Type".to_string(),
        Valor::texto("text/event-stream; charset=utf-8"),
    );
    cabeceras.insert(
        "Cache-Control".to_string(),
        Valor::texto("no-cache"),
    );
    datos.insert("cabeceras".to_string(), Valor::jsn(cabeceras));
    datos.insert("cuerpo".to_string(), Valor::Nulo);
    datos.insert("es_flujo".to_string(), Valor::Log(true));
    datos.insert("generador".to_string(), generador);
    Ok(Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_RESPUESTA_SERVIDOR),
        datos: RefCell::new(datos),
    })))
}

/// `Respuestas.archivo(ruta, opciones?)`: sirve un archivo del disco con
/// streaming de 64 KiB por trozo (nunca lo carga completo en memoria), soporte de
/// `Range` (`206`), caché condicional (`ETag`/`Last-Modified`, `304`) y
/// tipo MIME automático por extensión. Requiere permiso de lectura sobre
/// `ruta` en `quetzal.json` (`sistema_archivos`), verificado aquí mismo
/// (igual que `SistemaArchivos.leer_bits`).
/// Opciones: `{descargar: log, nombre: texto, max_edad: entero}`.
fn respuestas_archivo(guardian: &Rc<GuardianPermisos>) -> maquina_virtual::FuncionNativa {
    const F: &str = "Respuestas.archivo";
    let guardian = Rc::clone(guardian);
    Box::new(move |argumentos| {
        if argumentos.is_empty() || argumentos.len() > 2 {
            return Err(error_aridad(F, 1, argumentos.len()));
        }
        let ruta = arg_ruta(F, argumentos, 0)?;
        guardian
            .verificar_lectura(&ruta)
            .map_err(permiso_denegado)?;
        let (tamano, mtime_ms, es_archivo) = metadatos_archivo(&ruta)
            .map_err(|causa| error_servidor(format!("'{F}': no se pudo leer '{ruta}': {causa}")))?;
        if !es_archivo {
            return Err(error_servidor(format!("'{F}': '{ruta}' no es un archivo")));
        }
        let tamano = tamano as i64;
        let tamano_u64 = tamano as u64;

        // Procesar opciones
        let mut descargar = false;
        let mut nombre_descarga: Option<String> = None;
        if argumentos.len() == 2 {
            let opciones = arg_jsn(F, argumentos, 1)?;
            let opciones = opciones.borrow();
            if let Some(b) = logico_opcional_de_jsn(F, &opciones, "descargar")? {
                descargar = b;
            }
            if let Some(n) = texto_opcional_de_jsn(F, &opciones, "nombre")? {
                nombre_descarga = Some(n);
            }
        }

        let mime = tipo_contenido(&ruta);
        let mut cabeceras_extra = IndexMap::new();

        // Content-Disposition si es descarga
        if descargar {
            let nombre = nombre_descarga.unwrap_or_else(|| {
                std::path::Path::new(&ruta)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("archivo")
                    .to_string()
            });
            cabeceras_extra.insert(
                "Content-Disposition".to_string(),
                Valor::texto(format!("attachment; filename=\"{nombre}\"")),
            );
        }

        let mut datos = IndexMap::new();
        datos.insert("estado".to_string(), Valor::Entero(200));
        datos.insert("cabeceras".to_string(), Valor::jsn(cabeceras_extra));
        datos.insert("cuerpo".to_string(), Valor::Nulo);
        datos.insert("es_archivo".to_string(), Valor::Log(true));
        datos.insert("archivo_ruta".to_string(), Valor::texto(ruta));
        datos.insert("archivo_tamano".to_string(), Valor::Entero(tamano));
        datos.insert("archivo_mtime_ms".to_string(), Valor::Entero(mtime_ms));
        datos.insert(
            "archivo_etag".to_string(),
            Valor::texto(calcular_etag(tamano_u64, mtime_ms)),
        );
        datos.insert("archivo_mime".to_string(), Valor::texto(mime));
        Ok(Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
            tipo: Rc::from(TIPO_RESPUESTA_SERVIDOR),
            datos: RefCell::new(datos),
        })))
    })
}

/// `Respuestas.flujo(generador)` (o `Respuestas.flujo(estado, generador)`):
/// respuesta cuyo cuerpo se produce en trozos, uno por cada llamada a
/// `generador` (una función de Quetzal sin argumentos, pasada por
/// referencia: nunca se copia, solo se clona el `Rc` del `Valor`). Cada
/// llamada debe devolver el siguiente trozo (`texto` o `Bits`) o `nulo`
/// para terminar el flujo. Se envía con `Transfer-Encoding: chunked`, así
/// que sirve tanto para archivos generados sobre la marcha como para
/// *server-sent events* (`Content-Type: text/event-stream` + líneas
/// `data: ...\n\n`).
fn respuestas_flujo(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Respuestas.flujo";
    let (estado, generador) = match argumentos.len() {
        1 => (200i64, argumentos[0].clone()),
        2 => {
            let estado = arg_entero(F, argumentos, 0)?;
            if !(100..=599).contains(&estado) {
                return Err(error(
                    "E0406",
                    format!(
                        "'{F}' espera un código de estado HTTP entre 100 y 599, pero recibió {estado}"
                    ),
                ));
            }
            (estado, argumentos[1].clone())
        }
        recibidos => {
            return Err(error(
                "E0210",
                format!(
                    "'{F}' espera 1 argumento (generador) o 2 (estado, generador), pero recibió {recibidos}"
                ),
            ));
        }
    };
    if !matches!(generador, Valor::Funcion(..)) {
        return Err(error(
            "E0406",
            format!(
                "'{F}' espera una función generadora sin argumentos como manejador, pero recibió '{}'",
                generador.nombre_tipo()
            ),
        ));
    }
    let mut datos = IndexMap::new();
    datos.insert("estado".to_string(), Valor::Entero(estado));
    datos.insert("cabeceras".to_string(), Valor::jsn(IndexMap::new()));
    datos.insert("cuerpo".to_string(), Valor::Nulo);
    datos.insert("es_flujo".to_string(), Valor::Log(true));
    datos.insert("generador".to_string(), generador);
    Ok(Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO_RESPUESTA_SERVIDOR),
        datos: RefCell::new(datos),
    })))
}

fn receptor_respuesta_servidor(
    funcion: &str,
    argumentos: &[Valor],
) -> Result<Rc<DatosInstanciaNativa>, Fallo> {
    match argumentos.first() {
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == TIPO_RESPUESTA_SERVIDOR => {
            Ok(Rc::clone(instancia))
        }
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba un RespuestaServidor, pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
        None => Err(error("E0210", format!("'{funcion}' necesita el receptor"))),
    }
}

fn metodo_respuesta_fijar_cabecera(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "RespuestaServidor.fijar_cabecera";
    exigir_aridad(F, &argumentos[1..], 2)?;
    let instancia = receptor_respuesta_servidor(F, argumentos)?;
    let nombre = arg_texto(F, argumentos, 1)?.to_string();
    let valor = arg_texto(F, argumentos, 2)?.to_string();
    match instancia.datos.borrow().get("cabeceras") {
        Some(Valor::Jsn(mapa)) => {
            mapa.borrow_mut().insert(nombre, Valor::texto(valor));
        }
        _ => {
            return Err(error(
                "E0406",
                format!("'{F}' recibió un RespuestaServidor sin cabeceras internas"),
            ));
        }
    }
    Ok(Valor::Nulo)
}

fn agregar_cabecera(mapa: &mut IndexMap<String, Valor>, nombre: String, valor: String) {
    match mapa.get_mut(&nombre) {
        Some(Valor::Lista(valores)) => valores.borrow_mut().push(Valor::texto(valor)),
        Some(existente) => {
            let anterior = existente.clone();
            *existente = Valor::lista(vec![anterior, Valor::texto(valor)]);
        }
        None => {
            mapa.insert(nombre, Valor::texto(valor));
        }
    }
}

fn metodo_respuesta_agregar_cabecera(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "RespuestaServidor.agregar_cabecera";
    exigir_aridad(F, &argumentos[1..], 2)?;
    let instancia = receptor_respuesta_servidor(F, argumentos)?;
    let nombre = arg_texto(F, argumentos, 1)?.to_string();
    let valor = arg_texto(F, argumentos, 2)?.to_string();
    match instancia.datos.borrow().get("cabeceras") {
        Some(Valor::Jsn(mapa)) => agregar_cabecera(&mut mapa.borrow_mut(), nombre, valor),
        _ => {
            return Err(error(
                "E0406",
                format!("'{F}' recibió un RespuestaServidor sin cabeceras internas"),
            ));
        }
    }
    Ok(Valor::Nulo)
}

// =====================================================================
// Registro
// =====================================================================

pub fn registrar(registro: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    registro.registrar_modulo("servidorhttp");
    registro.registrar_modulo("respuestas");
    registro.registrar_funcion("servidorhttp.constructor", Box::new(constructor));
    registro.registrar_funcion("respuestas.crear", Box::new(respuestas_crear));
    registro.registrar_funcion("respuestas.archivo", respuestas_archivo(guardian));
    registro.registrar_funcion("respuestas.flujo", Box::new(respuestas_flujo));
    registro.registrar_funcion("respuestas.redirigir", Box::new(respuestas_redirigir));
    registro.registrar_funcion("respuestas.nada", Box::new(respuestas_nada));
    registro.registrar_funcion("respuestas.texto", Box::new(respuestas_texto));
    registro.registrar_funcion("respuestas.jsn", Box::new(respuestas_jsn));
    registro.registrar_funcion("respuestas.bits", Box::new(respuestas_bits));
    registro.registrar_funcion("respuestas.error", Box::new(respuestas_error));
    registro.registrar_funcion("respuestas.eventos", Box::new(respuestas_eventos));

    let registro_servidores: RegistroServidores = Rc::new(RefCell::new(HashMap::new()));
    let banderas: RegistroBanderas = Rc::new(RefCell::new(HashMap::new()));
    let registro_flujos: RegistroFlujos = Rc::new(RefCell::new(HashMap::new()));
    let estado_cadena: EstadoCadenaCompartido = Rc::new(RefCell::new(HashMap::new()));

    registro.registrar_funcion_con_vm("__siguiente", crear_siguiente(Rc::clone(&estado_cadena)));

    let rutas_fijas: [(&str, &'static str, reqwest::Method); 7] = [
        ("obtener", "ServidorHttp.obtener", reqwest::Method::GET),
        ("publicar", "ServidorHttp.publicar", reqwest::Method::POST),
        ("poner", "ServidorHttp.poner", reqwest::Method::PUT),
        ("parchar", "ServidorHttp.parchar", reqwest::Method::PATCH),
        ("eliminar", "ServidorHttp.eliminar", reqwest::Method::DELETE),
        ("cabeza", "ServidorHttp.cabeza", reqwest::Method::HEAD),
        (
            "opciones",
            "ServidorHttp.opciones",
            reqwest::Method::OPTIONS,
        ),
    ];
    for (nombre, etiqueta, metodo_http) in rutas_fijas {
        registro.registrar_funcion(
            &format!("{TIPO_SERVIDOR}.{nombre}"),
            metodo_ruta_fija(etiqueta, metodo_http),
        );
    }
    registro.registrar_funcion(
        &format!("{TIPO_SERVIDOR}.ruta"),
        Box::new(metodo_ruta_generica),
    );
    registro.registrar_funcion(&format!("{TIPO_SERVIDOR}.todo"), Box::new(metodo_ruta_todo));
    registro.registrar_funcion(
        &format!("{TIPO_SERVIDOR}.estaticos"),
        Box::new(metodo_estaticos),
    );
    registro.registrar_funcion(&format!("{TIPO_SERVIDOR}.puerto"), Box::new(metodo_puerto));
    registro.registrar_funcion(
        &format!("{TIPO_SERVIDOR}.limite_cuerpo"),
        Box::new(metodo_limite_cuerpo),
    );
    registro.registrar_funcion(&format!("{TIPO_SERVIDOR}.usar"), Box::new(metodo_usar));
    registro.registrar_funcion(
        &format!("{TIPO_SERVIDOR}.manejar_errores"),
        Box::new(metodo_manejar_errores),
    );
    registro.registrar_funcion(
        &format!("{TIPO_SERVIDOR}.esta_escuchando"),
        Box::new(metodo_esta_escuchando),
    );
    registro.registrar_funcion_con_vm(
        &format!("{TIPO_SERVIDOR}.escuchar"),
        metodo_escuchar(
            guardian,
            &registro_servidores,
            &banderas,
            &registro_flujos,
            &estado_cadena,
        ),
    );
    registro.registrar_funcion_con_vm(
        &format!("{TIPO_SERVIDOR}.detener"),
        metodo_detener(&banderas, &registro_servidores),
    );

    type MetodoPeticion = fn(&[Valor]) -> Result<Valor, Fallo>;
    let metodos_peticion: [(&str, MetodoPeticion); 22] = [
        ("metodo", metodo_peticion_metodo),
        ("ruta", metodo_peticion_ruta),
        ("protocolo", metodo_peticion_protocolo),
        ("peso_declarado", metodo_peticion_peso_declarado),
        ("parametros", metodo_peticion_parametros),
        ("consulta", metodo_peticion_consulta),
        ("consulta_todas", metodo_peticion_consulta_todas),
        ("cabeceras", metodo_peticion_cabeceras),
        ("cabecera", metodo_peticion_cabecera),
        ("cabeceras_todas", metodo_peticion_cabeceras_todas),
        ("bits", metodo_peticion_bits),
        ("texto", metodo_peticion_texto),
        ("jsn", metodo_peticion_jsn),
        ("peso", metodo_peticion_peso),
        ("partes", metodo_peticion_partes),
        ("archivos", metodo_peticion_archivos),
        ("formulario", metodo_peticion_formulario),
        ("galletas", metodo_peticion_galletas),
        ("acepta", metodo_peticion_acepta),
        ("acepta_idioma", metodo_peticion_acepta_idioma),
        ("acepta_codificacion", metodo_peticion_acepta_codificacion),
        ("es", metodo_peticion_es),
    ];
    for (nombre, funcion) in metodos_peticion {
        registro.registrar_funcion(&format!("{TIPO_PETICION}.{nombre}"), Box::new(funcion));
    }

    // Métodos de ParteMultiparte
    type MetodoParte = fn(&[Valor]) -> Result<Valor, Fallo>;
    let metodos_parte: [(&str, MetodoParte); 5] = [
        ("nombre", metodo_parte_nombre),
        ("nombre_archivo", metodo_parte_nombre_archivo),
        ("tipo_contenido", metodo_parte_tipo_contenido),
        ("bits", metodo_parte_bits),
        ("texto", metodo_parte_texto),
    ];
    for (nombre, funcion) in metodos_parte {
        registro.registrar_funcion(
            &format!("{TIPO_PARTE_MULTIPARTE}.{nombre}"),
            Box::new(funcion),
        );
    }

    registro.registrar_funcion(
        &format!("{TIPO_RESPUESTA_SERVIDOR}.fijar_cabecera"),
        Box::new(metodo_respuesta_fijar_cabecera),
    );
    registro.registrar_funcion(
        &format!("{TIPO_RESPUESTA_SERVIDOR}.agregar_cabecera"),
        Box::new(metodo_respuesta_agregar_cabecera),
    );
    registro.registrar_funcion(
        &format!("{TIPO_RESPUESTA_SERVIDOR}.fijar_galleta"),
        Box::new(metodo_fijar_galleta),
    );
    registro.registrar_funcion(
        &format!("{TIPO_RESPUESTA_SERVIDOR}.borrar_galleta"),
        Box::new(metodo_borrar_galleta),
    );

    // Enrutador (grupos de rutas)
    registro.registrar_modulo("enrutador");
    registro.registrar_funcion("enrutador.constructor", Box::new(constructor_enrutador));
    let rutas_enrutador: [(&str, &'static str, reqwest::Method); 7] = [
        ("obtener", "Enrutador.obtener", reqwest::Method::GET),
        ("publicar", "Enrutador.publicar", reqwest::Method::POST),
        ("poner", "Enrutador.poner", reqwest::Method::PUT),
        ("parchar", "Enrutador.parchar", reqwest::Method::PATCH),
        ("eliminar", "Enrutador.eliminar", reqwest::Method::DELETE),
        ("cabeza", "Enrutador.cabeza", reqwest::Method::HEAD),
        ("opciones", "Enrutador.opciones", reqwest::Method::OPTIONS),
    ];
    for (nombre, etiqueta, metodo_http) in rutas_enrutador {
        registro.registrar_funcion(
            &format!("{TIPO_ENRUTADOR}.{nombre}"),
            metodo_enrutador_fija(etiqueta, metodo_http),
        );
    }
    registro.registrar_funcion(
        &format!("{TIPO_ENRUTADOR}.usar"),
        Box::new(metodo_enrutador_usar),
    );

    // Montar + ciclo de vida
    registro.registrar_funcion(&format!("{TIPO_SERVIDOR}.montar"), Box::new(metodo_montar));
    registro.registrar_funcion(
        &format!("{TIPO_SERVIDOR}.al_cerrar"),
        Box::new(metodo_al_cerrar),
    );
    registro.registrar_funcion(
        &format!("{TIPO_SERVIDOR}.al_listo"),
        Box::new(metodo_al_listo),
    );
}
