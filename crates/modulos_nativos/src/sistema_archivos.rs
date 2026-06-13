//! Módulo nativo `quetzal/sistema_archivos`: el objeto `SistemaArchivos`
//! (funciones libres) y el objeto `Archivo` (instanciable, con cursor).
//!
//! Cada operación consulta el [`GuardianPermisos`] del runtime; sin permiso
//! declarado en `quetzal.json` la operación falla con `E0701`.
//!
//! `Archivo` no retiene un descriptor del sistema operativo: guarda la ruta,
//! el modo y la posición del cursor, y cada operación abre el archivo, hace
//! seek a la posición, opera y cierra. Así no hay descriptores filtrados y la
//! lectura por lotes (`leer_linea`, `leer_bits(n)`) procesa archivos enormes
//! con memoria acotada.

use std::cell::RefCell;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::rc::Rc;

use indexmap::IndexMap;
use maquina_virtual::{DatosInstanciaNativa, Fallo, RegistroNativos, Valor};
use runtime::GuardianPermisos;

use crate::bits::valor_bits;
use crate::util::{arg_bits, arg_entero, arg_ruta, arg_texto, error, exigir_aridad};

const TIPO: &str = "Archivo";

/// Función nativa que necesita consultar el guardián de permisos.
type NativaConPermisos = fn(&GuardianPermisos, &[Valor]) -> Result<Valor, Fallo>;

/// Convierte una denegación del guardián en una excepción de runtime.
fn permiso_denegado(mensaje: String) -> Fallo {
    error("E0701", mensaje)
}

/// Fallo de E/S con la operación y la ruta en el mensaje.
fn error_es(operacion: &str, ruta: &str, fallo: std::io::Error) -> Fallo {
    error("E0702", format!("no se pudo {operacion} '{ruta}': {fallo}"))
}

/// Divide un contenido en líneas (sin `\n` ni `\r`) como valores de Quetzal.
fn lineas_de_texto(contenido: &str) -> Vec<Valor> {
    contenido.lines().map(Valor::texto).collect()
}

// =====================================================================
// Funciones libres `SistemaArchivos.*`
// =====================================================================

// ----- Texto -----

fn sa_leer_texto(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "SistemaArchivos.leer_texto";
    exigir_aridad(F, argumentos, 1)?;
    let ruta = arg_ruta(F, argumentos, 0)?;
    permisos.verificar_lectura(&ruta).map_err(permiso_denegado)?;
    let contenido =
        std::fs::read_to_string(&ruta).map_err(|fallo| error_es("leer", &ruta, fallo))?;
    Ok(Valor::texto(contenido))
}

fn sa_leer_lineas(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "SistemaArchivos.leer_lineas";
    exigir_aridad(F, argumentos, 1)?;
    let ruta = arg_ruta(F, argumentos, 0)?;
    permisos.verificar_lectura(&ruta).map_err(permiso_denegado)?;
    let contenido =
        std::fs::read_to_string(&ruta).map_err(|fallo| error_es("leer", &ruta, fallo))?;
    Ok(Valor::lista(lineas_de_texto(&contenido)))
}

fn sa_escribir_texto(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "SistemaArchivos.escribir_texto";
    exigir_aridad(F, argumentos, 2)?;
    let ruta = arg_ruta(F, argumentos, 0)?;
    let contenido = arg_texto(F, argumentos, 1)?;
    permisos.verificar_escritura(&ruta).map_err(permiso_denegado)?;
    std::fs::write(&ruta, contenido).map_err(|fallo| error_es("escribir", &ruta, fallo))?;
    Ok(Valor::Nulo)
}

fn sa_agregar_texto(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "SistemaArchivos.agregar_texto";
    exigir_aridad(F, argumentos, 2)?;
    let ruta = arg_ruta(F, argumentos, 0)?;
    let contenido = arg_texto(F, argumentos, 1)?;
    permisos.verificar_escritura(&ruta).map_err(permiso_denegado)?;
    let mut archivo = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&ruta)
        .map_err(|fallo| error_es("abrir", &ruta, fallo))?;
    archivo
        .write_all(contenido.as_bytes())
        .map_err(|fallo| error_es("escribir", &ruta, fallo))?;
    Ok(Valor::Nulo)
}

// ----- Binario -----

fn sa_leer_bits(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "SistemaArchivos.leer_bits";
    exigir_aridad(F, argumentos, 1)?;
    let ruta = arg_ruta(F, argumentos, 0)?;
    permisos.verificar_lectura(&ruta).map_err(permiso_denegado)?;
    let bytes = std::fs::read(&ruta).map_err(|fallo| error_es("leer", &ruta, fallo))?;
    Ok(valor_bits(&bytes))
}

fn sa_escribir_bits(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "SistemaArchivos.escribir_bits";
    exigir_aridad(F, argumentos, 2)?;
    let ruta = arg_ruta(F, argumentos, 0)?;
    let bytes = arg_bits(F, argumentos, 1)?;
    permisos.verificar_escritura(&ruta).map_err(permiso_denegado)?;
    std::fs::write(&ruta, &bytes).map_err(|fallo| error_es("escribir", &ruta, fallo))?;
    Ok(Valor::Nulo)
}

fn sa_agregar_bits(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "SistemaArchivos.agregar_bits";
    exigir_aridad(F, argumentos, 2)?;
    let ruta = arg_ruta(F, argumentos, 0)?;
    let bytes = arg_bits(F, argumentos, 1)?;
    permisos.verificar_escritura(&ruta).map_err(permiso_denegado)?;
    let mut archivo = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&ruta)
        .map_err(|fallo| error_es("abrir", &ruta, fallo))?;
    archivo
        .write_all(&bytes)
        .map_err(|fallo| error_es("escribir", &ruta, fallo))?;
    Ok(Valor::Nulo)
}

// ----- Consultas -----

fn sa_existe(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "SistemaArchivos.existe";
    exigir_aridad(F, argumentos, 1)?;
    let ruta = arg_ruta(F, argumentos, 0)?;
    permisos.verificar_lectura(&ruta).map_err(permiso_denegado)?;
    Ok(Valor::Log(Path::new(&ruta).exists()))
}

fn sa_es_archivo(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "SistemaArchivos.es_archivo";
    exigir_aridad(F, argumentos, 1)?;
    let ruta = arg_ruta(F, argumentos, 0)?;
    permisos.verificar_lectura(&ruta).map_err(permiso_denegado)?;
    Ok(Valor::Log(Path::new(&ruta).is_file()))
}

fn sa_es_directorio(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "SistemaArchivos.es_directorio";
    exigir_aridad(F, argumentos, 1)?;
    let ruta = arg_ruta(F, argumentos, 0)?;
    permisos.verificar_lectura(&ruta).map_err(permiso_denegado)?;
    Ok(Valor::Log(Path::new(&ruta).is_dir()))
}

fn sa_tamaño(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "SistemaArchivos.tamaño";
    exigir_aridad(F, argumentos, 1)?;
    let ruta = arg_ruta(F, argumentos, 0)?;
    permisos.verificar_lectura(&ruta).map_err(permiso_denegado)?;
    let metadatos =
        std::fs::metadata(&ruta).map_err(|fallo| error_es("consultar", &ruta, fallo))?;
    Ok(Valor::Entero(metadatos.len() as i64))
}

/// Marca Unix en milisegundos de un `SystemTime` (o nulo si no se conoce).
fn marca_ms(tiempo: std::io::Result<std::time::SystemTime>) -> Valor {
    tiempo
        .ok()
        .and_then(|tiempo| tiempo.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duracion| Valor::Entero(duracion.as_millis() as i64))
        .unwrap_or(Valor::Nulo)
}

fn sa_metadatos(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "SistemaArchivos.metadatos";
    exigir_aridad(F, argumentos, 1)?;
    let ruta = arg_ruta(F, argumentos, 0)?;
    permisos.verificar_lectura(&ruta).map_err(permiso_denegado)?;
    let metadatos =
        std::fs::metadata(&ruta).map_err(|fallo| error_es("consultar", &ruta, fallo))?;
    let mut mapa = IndexMap::new();
    mapa.insert("tamaño".to_string(), Valor::Entero(metadatos.len() as i64));
    mapa.insert("es_archivo".to_string(), Valor::Log(metadatos.is_file()));
    mapa.insert("es_directorio".to_string(), Valor::Log(metadatos.is_dir()));
    mapa.insert(
        "solo_lectura".to_string(),
        Valor::Log(metadatos.permissions().readonly()),
    );
    mapa.insert("modificado".to_string(), marca_ms(metadatos.modified()));
    mapa.insert("creado".to_string(), marca_ms(metadatos.created()));
    Ok(Valor::jsn(mapa))
}

// ----- Directorios -----

fn sa_listar(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "SistemaArchivos.listar";
    exigir_aridad(F, argumentos, 1)?;
    let ruta = arg_ruta(F, argumentos, 0)?;
    permisos.verificar_lectura(&ruta).map_err(permiso_denegado)?;
    let entradas = std::fs::read_dir(&ruta)
        .map_err(|fallo| error_es("listar", &ruta, fallo))?
        .filter_map(|entrada| entrada.ok())
        .map(|entrada| Valor::texto(entrada.file_name().to_string_lossy()))
        .collect();
    Ok(Valor::lista(entradas))
}

fn sa_crear_directorio(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "SistemaArchivos.crear_directorio";
    exigir_aridad(F, argumentos, 1)?;
    let ruta = arg_ruta(F, argumentos, 0)?;
    permisos.verificar_escritura(&ruta).map_err(permiso_denegado)?;
    std::fs::create_dir_all(&ruta).map_err(|fallo| error_es("crear el directorio", &ruta, fallo))?;
    Ok(Valor::Nulo)
}

fn sa_eliminar_archivo(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "SistemaArchivos.eliminar_archivo";
    exigir_aridad(F, argumentos, 1)?;
    let ruta = arg_ruta(F, argumentos, 0)?;
    permisos.verificar_escritura(&ruta).map_err(permiso_denegado)?;
    std::fs::remove_file(&ruta).map_err(|fallo| error_es("eliminar", &ruta, fallo))?;
    Ok(Valor::Nulo)
}

fn sa_eliminar_directorio(
    permisos: &GuardianPermisos,
    argumentos: &[Valor],
) -> Result<Valor, Fallo> {
    const F: &str = "SistemaArchivos.eliminar_directorio";
    exigir_aridad(F, argumentos, 1)?;
    let ruta = arg_ruta(F, argumentos, 0)?;
    permisos.verificar_escritura(&ruta).map_err(permiso_denegado)?;
    std::fs::remove_dir(&ruta).map_err(|fallo| error_es("eliminar el directorio", &ruta, fallo))?;
    Ok(Valor::Nulo)
}

fn sa_eliminar_todo(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "SistemaArchivos.eliminar_todo";
    exigir_aridad(F, argumentos, 1)?;
    let ruta = arg_ruta(F, argumentos, 0)?;
    permisos.verificar_escritura(&ruta).map_err(permiso_denegado)?;
    std::fs::remove_dir_all(&ruta)
        .map_err(|fallo| error_es("eliminar recursivamente", &ruta, fallo))?;
    Ok(Valor::Nulo)
}

// ----- Copiar / mover -----

fn sa_copiar(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "SistemaArchivos.copiar";
    exigir_aridad(F, argumentos, 2)?;
    let origen = arg_ruta(F, argumentos, 0)?;
    let destino = arg_ruta(F, argumentos, 1)?;
    permisos.verificar_lectura(&origen).map_err(permiso_denegado)?;
    permisos.verificar_escritura(&destino).map_err(permiso_denegado)?;
    std::fs::copy(&origen, &destino).map_err(|fallo| error_es("copiar", &origen, fallo))?;
    Ok(Valor::Nulo)
}

fn sa_mover(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "SistemaArchivos.mover";
    exigir_aridad(F, argumentos, 2)?;
    let origen = arg_ruta(F, argumentos, 0)?;
    let destino = arg_ruta(F, argumentos, 1)?;
    permisos.verificar_escritura(&origen).map_err(permiso_denegado)?;
    permisos.verificar_escritura(&destino).map_err(permiso_denegado)?;
    std::fs::rename(&origen, &destino).map_err(|fallo| error_es("mover", &origen, fallo))?;
    Ok(Valor::Nulo)
}

// =====================================================================
// Objeto `Archivo`
// =====================================================================

/// Modo de apertura de un `Archivo`.
#[derive(Clone, Copy, PartialEq)]
enum Modo {
    Lectura,
    Escritura,
    Agregar,
    LecturaEscritura,
}

impl Modo {
    fn de_texto(funcion: &str, texto: &str) -> Result<Self, Fallo> {
        match texto {
            "lectura" => Ok(Modo::Lectura),
            "escritura" => Ok(Modo::Escritura),
            "agregar" => Ok(Modo::Agregar),
            "lectura_escritura" => Ok(Modo::LecturaEscritura),
            otro => Err(error(
                "E0406",
                format!(
                    "'{funcion}' no reconoce el modo '{otro}'; usa 'lectura', 'escritura', 'agregar' o 'lectura_escritura'"
                ),
            )),
        }
    }

    fn nombre(self) -> &'static str {
        match self {
            Modo::Lectura => "lectura",
            Modo::Escritura => "escritura",
            Modo::Agregar => "agregar",
            Modo::LecturaEscritura => "lectura_escritura",
        }
    }

    fn permite_leer(self) -> bool {
        matches!(self, Modo::Lectura | Modo::LecturaEscritura)
    }

    fn permite_escribir(self) -> bool {
        matches!(self, Modo::Escritura | Modo::Agregar | Modo::LecturaEscritura)
    }
}

/// Estado del receptor de un método de `Archivo`.
struct EstadoArchivo {
    instancia: Rc<DatosInstanciaNativa>,
    ruta: String,
    modo: Modo,
    posicion: u64,
}

impl EstadoArchivo {
    /// Guarda la nueva posición del cursor en la instancia.
    fn fijar_posicion(&self, posicion: u64) {
        self.instancia
            .datos
            .borrow_mut()
            .insert("posicion".to_string(), Valor::Entero(posicion as i64));
    }

    /// Marca la instancia como cerrada.
    fn cerrar(&self) {
        self.instancia
            .datos
            .borrow_mut()
            .insert("abierto".to_string(), Valor::Log(false));
    }

    fn exigir_lectura(&self, funcion: &str) -> Result<(), Fallo> {
        if self.modo.permite_leer() {
            return Ok(());
        }
        Err(error(
            "E0406",
            format!(
                "'{funcion}' no puede leer un archivo abierto en modo '{}'",
                self.modo.nombre()
            ),
        ))
    }

    fn exigir_escritura(&self, funcion: &str) -> Result<(), Fallo> {
        if self.modo.permite_escribir() {
            return Ok(());
        }
        Err(error(
            "E0406",
            format!(
                "'{funcion}' no puede escribir un archivo abierto en modo '{}'",
                self.modo.nombre()
            ),
        ))
    }

    /// Abre el archivo para leer y posiciona el cursor.
    fn abrir_lectura(&self, funcion: &str) -> Result<std::fs::File, Fallo> {
        self.exigir_lectura(funcion)?;
        let mut archivo = OpenOptions::new()
            .read(true)
            .open(&self.ruta)
            .map_err(|fallo| error_es("abrir", &self.ruta, fallo))?;
        archivo
            .seek(SeekFrom::Start(self.posicion))
            .map_err(|fallo| error_es("posicionar", &self.ruta, fallo))?;
        Ok(archivo)
    }

    /// Abre el archivo para escribir: al final en modo `agregar`, o en la
    /// posición del cursor en los demás modos. Devuelve el archivo y la
    /// posición resultante tras escribir `bytes_a_escribir` bytes.
    fn abrir_escritura(&self, funcion: &str) -> Result<std::fs::File, Fallo> {
        self.exigir_escritura(funcion)?;
        let mut opciones = OpenOptions::new();
        opciones.write(true).create(true).truncate(false);
        if self.modo == Modo::Agregar {
            opciones.append(true);
        }
        let mut archivo = opciones
            .open(&self.ruta)
            .map_err(|fallo| error_es("abrir", &self.ruta, fallo))?;
        if self.modo != Modo::Agregar {
            archivo
                .seek(SeekFrom::Start(self.posicion))
                .map_err(|fallo| error_es("posicionar", &self.ruta, fallo))?;
        }
        Ok(archivo)
    }

    fn tamaño(&self) -> Result<u64, Fallo> {
        std::fs::metadata(&self.ruta)
            .map(|metadatos| metadatos.len())
            .map_err(|fallo| error_es("consultar", &self.ruta, fallo))
    }
}

/// Extrae el receptor de un método de `Archivo` sin verificar permisos ni el
/// estado abierto/cerrado (para métodos informativos). Devuelve el estado y
/// si el archivo sigue abierto.
fn receptor_archivo_info(
    funcion: &str,
    argumentos: &[Valor],
) -> Result<(EstadoArchivo, bool), Fallo> {
    let instancia = match argumentos.first() {
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == TIPO => Rc::clone(instancia),
        Some(otro) => {
            return Err(error(
                "E0406",
                format!(
                    "'{funcion}' esperaba una instancia de Archivo, pero recibió '{}'",
                    otro.nombre_tipo()
                ),
            ));
        }
        None => {
            return Err(error(
                "E0210",
                format!("'{funcion}' necesita el receptor"),
            ));
        }
    };

    let (ruta, modo, posicion, abierto) = {
        let datos = instancia.datos.borrow();
        let ruta = match datos.get("ruta") {
            Some(Valor::Texto(texto)) => texto.to_string(),
            _ => return Err(error("E0406", format!("'{funcion}' recibió un Archivo sin ruta interna"))),
        };
        let modo = match datos.get("modo") {
            Some(Valor::Texto(texto)) => Modo::de_texto(funcion, texto)?,
            _ => Modo::Lectura,
        };
        let posicion = match datos.get("posicion") {
            Some(Valor::Entero(entero)) if *entero >= 0 => *entero as u64,
            _ => 0,
        };
        let abierto = matches!(datos.get("abierto"), Some(Valor::Log(true)));
        (ruta, modo, posicion, abierto)
    };

    Ok((
        EstadoArchivo {
            instancia,
            ruta,
            modo,
            posicion,
        },
        abierto,
    ))
}

/// Extrae y valida el receptor de un método de `Archivo`: debe estar abierto
/// y con permisos vigentes para su modo.
fn receptor_archivo(
    funcion: &str,
    permisos: &GuardianPermisos,
    argumentos: &[Valor],
) -> Result<EstadoArchivo, Fallo> {
    let (estado, abierto) = receptor_archivo_info(funcion, argumentos)?;

    if !abierto {
        return Err(error(
            "E0406",
            format!(
                "'{funcion}' no puede operar sobre un archivo cerrado ('{}')",
                estado.ruta
            ),
        ));
    }

    // Cada operación re-verifica permisos: el guardián puede reconfigurarse.
    if estado.modo.permite_leer() {
        permisos
            .verificar_lectura(&estado.ruta)
            .map_err(permiso_denegado)?;
    }
    if estado.modo.permite_escribir() {
        permisos
            .verificar_escritura(&estado.ruta)
            .map_err(permiso_denegado)?;
    }

    Ok(estado)
}

// ----- Constructor: `nuevo Archivo(ruta[, modo])` -----

fn constructor_archivo(
    permisos: &GuardianPermisos,
    argumentos: &[Valor],
) -> Result<Valor, Fallo> {
    const F: &str = "nuevo Archivo";
    if argumentos.is_empty() || argumentos.len() > 2 {
        return Err(error(
            "E0210",
            format!("'{F}' acepta 1 o 2 argumentos, pero recibió {}", argumentos.len()),
        ));
    }
    let ruta = arg_ruta(F, argumentos, 0)?;
    let modo = if argumentos.len() == 2 {
        Modo::de_texto(F, arg_texto(F, argumentos, 1)?)?
    } else {
        Modo::Lectura
    };

    if modo.permite_leer() {
        permisos.verificar_lectura(&ruta).map_err(permiso_denegado)?;
    }
    if modo.permite_escribir() {
        permisos.verificar_escritura(&ruta).map_err(permiso_denegado)?;
    }

    // Efecto inmediato de cada modo: existir (lectura), truncar (escritura)
    // o crear si falta (agregar / lectura_escritura).
    match modo {
        Modo::Lectura => {
            OpenOptions::new()
                .read(true)
                .open(&ruta)
                .map_err(|fallo| error_es("abrir", &ruta, fallo))?;
        }
        Modo::Escritura => {
            OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&ruta)
                .map_err(|fallo| error_es("crear", &ruta, fallo))?;
        }
        Modo::Agregar | Modo::LecturaEscritura => {
            // Crea el archivo si falta, sin tocar el contenido existente.
            OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .open(&ruta)
                .map_err(|fallo| error_es("abrir", &ruta, fallo))?;
        }
    }

    let mut datos = IndexMap::new();
    datos.insert("ruta".to_string(), Valor::texto(ruta.clone()));
    datos.insert("modo".to_string(), Valor::texto(modo.nombre()));
    datos.insert("posicion".to_string(), Valor::Entero(0));
    datos.insert("abierto".to_string(), Valor::Log(true));
    datos.insert(
        "texto".to_string(),
        Valor::texto(format!("<Archivo {ruta} ({})>", modo.nombre())),
    );
    Ok(Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO),
        datos: RefCell::new(datos),
    })))
}

// ----- Métodos: lectura completa -----

/// `a.leer_texto()` o `a.leer_texto(n)`: todo desde la posición, o el
/// siguiente lote de hasta n bytes decodificado en límite UTF-8 válido.
fn metodo_leer_texto(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.leer_texto";
    let estado = receptor_archivo(F, permisos, argumentos)?;
    match argumentos.len() - 1 {
        0 => {
            let mut archivo = estado.abrir_lectura(F)?;
            let mut contenido = String::new();
            archivo
                .read_to_string(&mut contenido)
                .map_err(|fallo| error_es("leer", &estado.ruta, fallo))?;
            estado.fijar_posicion(estado.posicion + contenido.len() as u64);
            Ok(Valor::texto(contenido))
        }
        1 => {
            let cantidad = arg_entero(F, argumentos, 1)?;
            if cantidad < 0 {
                return Err(error(
                    "E0406",
                    format!("'{F}' espera una cantidad positiva de bytes, pero recibió {cantidad}"),
                ));
            }
            let mut archivo = estado.abrir_lectura(F)?;
            let mut lote = vec![0u8; cantidad as usize];
            let leidos = leer_exacto(&mut archivo, &mut lote)
                .map_err(|fallo| error_es("leer", &estado.ruta, fallo))?;
            lote.truncate(leidos);
            // Solo se consume el prefijo UTF-8 válido; el resto queda para el
            // siguiente lote (no se parten caracteres multibyte).
            let validos = match std::str::from_utf8(&lote) {
                Ok(_) => lote.len(),
                Err(fallo) if fallo.error_len().is_none() => fallo.valid_up_to(),
                Err(fallo) => {
                    if fallo.valid_up_to() == 0 {
                        return Err(error(
                            "E0406",
                            format!("'{F}' encontró bytes que no son texto UTF-8 válido en '{}'", estado.ruta),
                        ));
                    }
                    fallo.valid_up_to()
                }
            };
            if validos == 0 && !lote.is_empty() {
                return Err(error(
                    "E0406",
                    format!("'{F}' necesita un lote más grande para completar un carácter UTF-8"),
                ));
            }
            let texto = std::str::from_utf8(&lote[..validos]).expect("prefijo UTF-8 validado");
            estado.fijar_posicion(estado.posicion + validos as u64);
            Ok(Valor::texto(texto))
        }
        otros => Err(error(
            "E0210",
            format!("'{F}' acepta 0 o 1 argumentos, pero recibió {otros}"),
        )),
    }
}

/// Lee hasta llenar el búfer (o llegar al final) y devuelve cuántos bytes leyó.
fn leer_exacto(archivo: &mut std::fs::File, destino: &mut [u8]) -> std::io::Result<usize> {
    let mut total = 0;
    while total < destino.len() {
        match archivo.read(&mut destino[total..])? {
            0 => break,
            leidos => total += leidos,
        }
    }
    Ok(total)
}

fn metodo_leer_lineas(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.leer_lineas";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let estado = receptor_archivo(F, permisos, argumentos)?;
    let mut archivo = estado.abrir_lectura(F)?;
    let mut contenido = String::new();
    archivo
        .read_to_string(&mut contenido)
        .map_err(|fallo| error_es("leer", &estado.ruta, fallo))?;
    estado.fijar_posicion(estado.posicion + contenido.len() as u64);
    Ok(Valor::lista(lineas_de_texto(&contenido)))
}

fn metodo_leer_todo_bits(
    permisos: &GuardianPermisos,
    argumentos: &[Valor],
) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.leer_todo_bits";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let estado = receptor_archivo(F, permisos, argumentos)?;
    let mut archivo = estado.abrir_lectura(F)?;
    let mut bytes = Vec::new();
    archivo
        .read_to_end(&mut bytes)
        .map_err(|fallo| error_es("leer", &estado.ruta, fallo))?;
    estado.fijar_posicion(estado.posicion + bytes.len() as u64);
    Ok(valor_bits(&bytes))
}

// ----- Métodos: lectura por lotes / cursor -----

fn metodo_leer_bits(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.leer_bits";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let estado = receptor_archivo(F, permisos, argumentos)?;
    let cantidad = arg_entero(F, argumentos, 1)?;
    if cantidad < 0 {
        return Err(error(
            "E0406",
            format!("'{F}' espera una cantidad positiva de bytes, pero recibió {cantidad}"),
        ));
    }
    let mut archivo = estado.abrir_lectura(F)?;
    let mut lote = vec![0u8; cantidad as usize];
    let leidos = leer_exacto(&mut archivo, &mut lote)
        .map_err(|fallo| error_es("leer", &estado.ruta, fallo))?;
    lote.truncate(leidos);
    estado.fijar_posicion(estado.posicion + leidos as u64);
    Ok(valor_bits(&lote))
}

/// `a.leer_linea()`: la siguiente línea (sin `\n` ni `\r`), o nulo al final.
/// Usa lectura con búfer: nunca carga el resto del archivo.
fn metodo_leer_linea(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.leer_linea";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let estado = receptor_archivo(F, permisos, argumentos)?;
    let archivo = estado.abrir_lectura(F)?;
    let mut lector = BufReader::new(archivo);
    let mut cruda = Vec::new();
    let consumidos = lector
        .read_until(b'\n', &mut cruda)
        .map_err(|fallo| error_es("leer", &estado.ruta, fallo))?;
    if consumidos == 0 {
        return Ok(Valor::Nulo);
    }
    estado.fijar_posicion(estado.posicion + consumidos as u64);
    // Quita el salto de línea final (`\n` o `\r\n`).
    if cruda.last() == Some(&b'\n') {
        cruda.pop();
        if cruda.last() == Some(&b'\r') {
            cruda.pop();
        }
    }
    String::from_utf8(cruda).map(Valor::texto).map_err(|_| {
        error(
            "E0406",
            format!("'{F}' encontró una línea que no es texto UTF-8 válido en '{}'", estado.ruta),
        )
    })
}

fn metodo_al_final(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.al_final";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let estado = receptor_archivo(F, permisos, argumentos)?;
    Ok(Valor::Log(estado.posicion >= estado.tamaño()?))
}

fn metodo_bits_restantes(
    permisos: &GuardianPermisos,
    argumentos: &[Valor],
) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.bits_restantes";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let estado = receptor_archivo(F, permisos, argumentos)?;
    let restantes = estado.tamaño()?.saturating_sub(estado.posicion);
    Ok(Valor::Entero(restantes as i64))
}

// ----- Métodos: escritura -----

fn escribir_bytes(
    funcion: &str,
    estado: &EstadoArchivo,
    bytes: &[u8],
) -> Result<(), Fallo> {
    let mut archivo = estado.abrir_escritura(funcion)?;
    archivo
        .write_all(bytes)
        .map_err(|fallo| error_es("escribir", &estado.ruta, fallo))?;
    // En modo agregar el cursor queda al final real del archivo.
    let nueva_posicion = if estado.modo == Modo::Agregar {
        archivo
            .stream_position()
            .map_err(|fallo| error_es("posicionar", &estado.ruta, fallo))?
    } else {
        estado.posicion + bytes.len() as u64
    };
    estado.fijar_posicion(nueva_posicion);
    Ok(())
}

fn metodo_escribir(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.escribir";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let estado = receptor_archivo(F, permisos, argumentos)?;
    let contenido = arg_texto(F, argumentos, 1)?;
    escribir_bytes(F, &estado, contenido.as_bytes())?;
    Ok(Valor::Nulo)
}

fn metodo_escribir_linea(
    permisos: &GuardianPermisos,
    argumentos: &[Valor],
) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.escribir_linea";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let estado = receptor_archivo(F, permisos, argumentos)?;
    let contenido = arg_texto(F, argumentos, 1)?;
    let mut bytes = contenido.as_bytes().to_vec();
    bytes.push(b'\n');
    escribir_bytes(F, &estado, &bytes)?;
    Ok(Valor::Nulo)
}

fn metodo_escribir_bits(
    permisos: &GuardianPermisos,
    argumentos: &[Valor],
) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.escribir_bits";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let estado = receptor_archivo(F, permisos, argumentos)?;
    let bytes = arg_bits(F, argumentos, 1)?;
    escribir_bytes(F, &estado, &bytes)?;
    Ok(Valor::Nulo)
}

// ----- Métodos: posición -----

fn metodo_posicion(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.posicion";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let estado = receptor_archivo(F, permisos, argumentos)?;
    Ok(Valor::Entero(estado.posicion as i64))
}

fn metodo_ir_a(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.ir_a";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let estado = receptor_archivo(F, permisos, argumentos)?;
    let posicion = arg_entero(F, argumentos, 1)?;
    if posicion < 0 {
        return Err(error(
            "E0406",
            format!("'{F}' espera una posición positiva, pero recibió {posicion}"),
        ));
    }
    estado.fijar_posicion(posicion as u64);
    Ok(Valor::Nulo)
}

fn metodo_ir_al_inicio(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.ir_al_inicio";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let estado = receptor_archivo(F, permisos, argumentos)?;
    estado.fijar_posicion(0);
    Ok(Valor::Nulo)
}

fn metodo_ir_al_final(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.ir_al_final";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let estado = receptor_archivo(F, permisos, argumentos)?;
    let tamaño = estado.tamaño()?;
    estado.fijar_posicion(tamaño);
    Ok(Valor::Nulo)
}

// ----- Métodos: información (funcionan aunque el archivo esté cerrado) -----

fn metodo_ruta(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.ruta";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let (estado, _) = receptor_archivo_info(F, argumentos)?;
    Ok(Valor::texto(estado.ruta))
}

fn metodo_nombre(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.nombre";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let (estado, _) = receptor_archivo_info(F, argumentos)?;
    let nombre = Path::new(&estado.ruta)
        .file_name()
        .map(|nombre| nombre.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(Valor::texto(nombre))
}

fn metodo_extension(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.extension";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let (estado, _) = receptor_archivo_info(F, argumentos)?;
    let extension = Path::new(&estado.ruta)
        .extension()
        .map(|ext| ext.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(Valor::texto(extension))
}

fn metodo_modo(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.modo";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let (estado, _) = receptor_archivo_info(F, argumentos)?;
    Ok(Valor::texto(estado.modo.nombre()))
}

fn metodo_tamaño(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.tamaño";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let (estado, _) = receptor_archivo_info(F, argumentos)?;
    permisos
        .verificar_lectura(&estado.ruta)
        .map_err(permiso_denegado)?;
    Ok(Valor::Entero(estado.tamaño()? as i64))
}

fn metodo_existe(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.existe";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let (estado, _) = receptor_archivo_info(F, argumentos)?;
    permisos
        .verificar_lectura(&estado.ruta)
        .map_err(permiso_denegado)?;
    Ok(Valor::Log(Path::new(&estado.ruta).exists()))
}

/// `a.esta_abierto()`: no falla aunque el archivo esté cerrado.
fn metodo_esta_abierto(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.esta_abierto";
    exigir_aridad(F, &argumentos[1..], 0)?;
    match argumentos.first() {
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == TIPO => {
            let abierto = matches!(instancia.datos.borrow().get("abierto"), Some(Valor::Log(true)));
            Ok(Valor::Log(abierto))
        }
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{F}' esperaba una instancia de Archivo, pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
        None => Err(error("E0210", format!("'{F}' necesita el receptor"))),
    }
}

// ----- Métodos: ciclo de vida -----

/// `a.cerrar()`: idempotente, no falla si ya estaba cerrado.
fn metodo_cerrar(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.cerrar";
    exigir_aridad(F, &argumentos[1..], 0)?;
    match argumentos.first() {
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == TIPO => {
            instancia
                .datos
                .borrow_mut()
                .insert("abierto".to_string(), Valor::Log(false));
            Ok(Valor::Nulo)
        }
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{F}' esperaba una instancia de Archivo, pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
        None => Err(error("E0210", format!("'{F}' necesita el receptor"))),
    }
}

fn metodo_eliminar(permisos: &GuardianPermisos, argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Archivo.eliminar";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let estado = receptor_archivo(F, permisos, argumentos)?;
    permisos
        .verificar_escritura(&estado.ruta)
        .map_err(permiso_denegado)?;
    std::fs::remove_file(&estado.ruta).map_err(|fallo| error_es("eliminar", &estado.ruta, fallo))?;
    estado.cerrar();
    Ok(Valor::Nulo)
}

// =====================================================================
// Registro
// =====================================================================

pub fn registrar(registro: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    registro.registrar_modulo("sistema_archivos");
    registro.registrar_modulo("archivo");

    // Funciones libres `SistemaArchivos.*` (clave interna `sistema_archivos.*`).
    let libres: [(&str, NativaConPermisos); 18] = [
        ("leer_texto", sa_leer_texto),
        ("leer_lineas", sa_leer_lineas),
        ("escribir_texto", sa_escribir_texto),
        ("agregar_texto", sa_agregar_texto),
        ("leer_bits", sa_leer_bits),
        ("escribir_bits", sa_escribir_bits),
        ("agregar_bits", sa_agregar_bits),
        ("existe", sa_existe),
        ("es_archivo", sa_es_archivo),
        ("es_directorio", sa_es_directorio),
        ("tamaño", sa_tamaño),
        ("tamano", sa_tamaño),
        ("metadatos", sa_metadatos),
        ("listar", sa_listar),
        ("crear_directorio", sa_crear_directorio),
        ("eliminar_archivo", sa_eliminar_archivo),
        ("eliminar_directorio", sa_eliminar_directorio),
        ("eliminar_todo", sa_eliminar_todo),
    ];
    for (nombre, funcion) in libres {
        let permisos = Rc::clone(guardian);
        registro.registrar_funcion(
            &format!("sistema_archivos.{nombre}"),
            Box::new(move |argumentos| funcion(&permisos, argumentos)),
        );
    }
    for (nombre, funcion) in [("copiar", sa_copiar as NativaConPermisos), ("mover", sa_mover)] {
        let permisos = Rc::clone(guardian);
        registro.registrar_funcion(
            &format!("sistema_archivos.{nombre}"),
            Box::new(move |argumentos| funcion(&permisos, argumentos)),
        );
    }

    // Constructor de `nuevo Archivo(...)`.
    let permisos = Rc::clone(guardian);
    registro.registrar_funcion(
        "archivo.constructor",
        Box::new(move |argumentos| constructor_archivo(&permisos, argumentos)),
    );

    // Métodos de instancia: `a.metodo(...)` despacha como `Archivo.metodo`
    // con el receptor como primer argumento. `tamaño` registra también la
    // variante sin ñ por comodidad.
    let metodos: [(&str, NativaConPermisos); 18] = [
        ("leer_texto", metodo_leer_texto),
        ("leer_lineas", metodo_leer_lineas),
        ("leer_todo_bits", metodo_leer_todo_bits),
        ("leer_bits", metodo_leer_bits),
        ("leer_linea", metodo_leer_linea),
        ("al_final", metodo_al_final),
        ("bits_restantes", metodo_bits_restantes),
        ("escribir", metodo_escribir),
        ("escribir_linea", metodo_escribir_linea),
        ("escribir_bits", metodo_escribir_bits),
        ("posicion", metodo_posicion),
        ("ir_a", metodo_ir_a),
        ("ir_al_inicio", metodo_ir_al_inicio),
        ("ir_al_final", metodo_ir_al_final),
        ("tamaño", metodo_tamaño),
        ("tamano", metodo_tamaño),
        ("existe", metodo_existe),
        ("eliminar", metodo_eliminar),
    ];
    for (nombre, funcion) in metodos {
        let permisos = Rc::clone(guardian);
        registro.registrar_funcion(
            &format!("{TIPO}.{nombre}"),
            Box::new(move |argumentos| funcion(&permisos, argumentos)),
        );
    }

    // Métodos informativos: funcionan aunque el archivo esté cerrado y no
    // tocan el disco (no piden permisos).
    type Nativa = fn(&[Valor]) -> Result<Valor, Fallo>;
    let informativos: [(&str, Nativa); 6] = [
        ("ruta", metodo_ruta),
        ("nombre", metodo_nombre),
        ("extension", metodo_extension),
        ("modo", metodo_modo),
        ("esta_abierto", metodo_esta_abierto),
        ("cerrar", metodo_cerrar),
    ];
    for (nombre, funcion) in informativos {
        registro.registrar_funcion(&format!("{TIPO}.{nombre}"), Box::new(funcion));
    }
}
