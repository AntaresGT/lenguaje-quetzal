//! Objeto `Archivo` del módulo nativo `quetzal/sistema_archivos`.
//!
//! `Archivo` es la referencia a un archivo o directorio que devuelven
//! `SistemaArchivos.abrir`, `crear_directorio` y `crear_directorios`. Al
//! estilo de `FileInfo`, además de los metadatos (nombre, extensión, tamaño,
//! fechas) ofrece las operaciones sobre esa misma ruta: leer, escribir,
//! copiar, mover, renombrar y borrar, cada una con su par `_asincrono`.
//!
//! Los metadatos se consultan al disco en el momento de la llamada, así que
//! siempre reflejan el estado actual; `refrescar()` devuelve una referencia
//! nueva a la misma ruta.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use maquina_virtual::{CargaNativa, Fallo, RegistroNativos, Valor};
use runtime::GuardianPermisos;

use crate::bits;
use crate::sistema_archivos::{
    Salida, destino_de_renombrar, ejecutar, error_io, error_permiso, marca_de, op_abrir, op_borrar,
    op_borrar_recursivo, op_copiar, op_escribir, op_escribir_bits, op_leer, op_leer_bits,
    op_listar_archivos, op_listar_directorios, op_mover, tarea,
};
use crate::tiempo;
use crate::util::{arg_texto, error, exigir_aridad};

/// Nombre del tipo visible para el usuario.
pub(crate) const TIPO: &str = "Archivo";

type OpRuta = fn(PathBuf) -> Salida;

// ----- Construcción y extracción -----

/// Crea la instancia de Quetzal que apunta a esta ruta.
///
/// Las operaciones síncronas también la construyen desde [`carga`], porque
/// comparten el mismo camino de conversión que las asincrónicas.
pub(crate) fn carga(ruta: &Path) -> CargaNativa {
    let texto = ruta.to_string_lossy().to_string();
    CargaNativa::Instancia {
        tipo: TIPO.to_string(),
        campos: vec![
            ("ruta".to_string(), CargaNativa::Texto(texto.clone())),
            ("texto".to_string(), CargaNativa::Texto(texto)),
        ],
    }
}

/// Ruta guardada en el receptor del método.
fn ruta_del_receptor(funcion: &str, argumentos: &[Valor]) -> Result<String, Fallo> {
    match argumentos.first() {
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == TIPO => {
            match instancia.datos.borrow().get("ruta") {
                Some(Valor::Texto(ruta)) => Ok(ruta.to_string()),
                _ => Err(error(
                    "E0406",
                    format!("'{funcion}' recibió un Archivo sin ruta válida"),
                )),
            }
        }
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba un Archivo, pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
        None => Err(error("E0210", format!("'{funcion}' necesita un receptor"))),
    }
}

/// Reapunta el receptor a otra ruta (tras `mover` o `renombrar`).
fn reapuntar(argumentos: &[Valor], destino: &Path) {
    if let Some(Valor::InstanciaNativa(instancia)) = argumentos.first() {
        let texto = Valor::texto(destino.to_string_lossy());
        let mut datos = instancia.datos.borrow_mut();
        datos.insert("ruta".to_string(), texto.clone());
        datos.insert("texto".to_string(), texto);
    }
}

fn ruta_lectura(
    guardian: &GuardianPermisos,
    funcion: &str,
    argumentos: &[Valor],
) -> Result<PathBuf, Fallo> {
    let ruta = ruta_del_receptor(funcion, argumentos)?;
    guardian.verificar_lectura(&ruta).map_err(error_permiso)
}

fn ruta_escritura(
    guardian: &GuardianPermisos,
    funcion: &str,
    argumentos: &[Valor],
) -> Result<PathBuf, Fallo> {
    let ruta = ruta_del_receptor(funcion, argumentos)?;
    guardian.verificar_escritura(&ruta).map_err(error_permiso)
}

// ----- Metadatos -----

/// Metadatos que se derivan de la ruta sin tocar el disco.
fn texto_de_ruta(ruta: &Path, parte: Parte) -> Valor {
    let vacio = || Valor::texto("");
    match parte {
        Parte::Nombre => ruta
            .file_name()
            .map(|nombre| Valor::texto(nombre.to_string_lossy()))
            .unwrap_or_else(vacio),
        Parte::NombreSinExtension => ruta
            .file_stem()
            .map(|nombre| Valor::texto(nombre.to_string_lossy()))
            .unwrap_or_else(vacio),
        Parte::Extension => ruta
            .extension()
            .map(|extension| Valor::texto(extension.to_string_lossy()))
            .unwrap_or_else(vacio),
        Parte::DirectorioPadre => ruta
            .parent()
            .map(|padre| Valor::texto(padre.to_string_lossy()))
            .unwrap_or_else(vacio),
        Parte::RutaCompleta => Valor::texto(ruta.to_string_lossy()),
    }
}

#[derive(Clone, Copy)]
enum Parte {
    Nombre,
    NombreSinExtension,
    Extension,
    DirectorioPadre,
    RutaCompleta,
}

/// Tamaño en bytes (0 en directorios, que no tienen un tamaño propio útil).
fn tamano(ruta: &Path) -> Result<Valor, Fallo> {
    if ruta.is_dir() {
        return Ok(Valor::Entero(0));
    }
    let metadatos = ruta.metadata().map_err(|fallo| {
        error_io(format!(
            "no se pudo consultar '{}': {fallo}",
            ruta.display()
        ))
    })?;
    Ok(Valor::Entero(metadatos.len() as i64))
}

/// Un directorio sin entradas o un archivo de 0 bytes.
fn es_vacio(ruta: &Path) -> Result<Valor, Fallo> {
    if ruta.is_dir() {
        let mut entradas = std::fs::read_dir(ruta).map_err(|fallo| {
            error_io(format!("no se pudo listar '{}': {fallo}", ruta.display()))
        })?;
        return Ok(Valor::Log(entradas.next().is_none()));
    }
    match tamano(ruta)? {
        Valor::Entero(bytes) => Ok(Valor::Log(bytes == 0)),
        _ => Ok(Valor::Log(false)),
    }
}

#[derive(Clone, Copy)]
enum Fecha {
    Creacion,
    Modificacion,
    Acceso,
}

/// Fecha del sistema de archivos como instancia de `Tiempo`.
fn fecha(funcion: &str, ruta: &Path, cual: Fecha) -> Result<Valor, Fallo> {
    let metadatos = ruta.metadata().map_err(|fallo| {
        error_io(format!(
            "no se pudo consultar '{}': {fallo}",
            ruta.display()
        ))
    })?;
    let instante = match cual {
        Fecha::Creacion => metadatos.created(),
        Fecha::Modificacion => metadatos.modified(),
        Fecha::Acceso => metadatos.accessed(),
    }
    .map_err(|fallo| {
        error_io(format!(
            "el sistema no reporta esa fecha para '{}': {fallo}",
            ruta.display()
        ))
    })?;
    tiempo::instancia_local(funcion, marca_de(instante))
}

// ----- Registro -----

/// Metadato de solo lectura: `archivo.nombre()`, `archivo.tamaño()`, ...
fn registrar_metadato<F>(
    registro: &mut RegistroNativos,
    guardian: &Rc<GuardianPermisos>,
    nombre: &'static str,
    consulta: F,
) where
    F: Fn(&str, &Path) -> Result<Valor, Fallo> + 'static,
{
    let guardian = Rc::clone(guardian);
    let funcion = format!("{TIPO}.{nombre}");
    registro.registrar_funcion(
        &format!("{TIPO}.{nombre}"),
        Box::new(move |argumentos| {
            exigir_aridad(&funcion, &argumentos[1..], 0)?;
            let ruta = ruta_lectura(&guardian, &funcion, argumentos)?;
            consulta(&funcion, &ruta)
        }),
    );
}

/// Operación sobre la ruta del receptor, en sus formas síncrona y asíncrona.
fn registrar_operacion(
    registro: &mut RegistroNativos,
    guardian: &Rc<GuardianPermisos>,
    nombre: &'static str,
    escritura: bool,
    operacion: OpRuta,
) {
    let funcion = format!("{TIPO}.{nombre}");

    let guardian_sincrono = Rc::clone(guardian);
    let etiqueta = funcion.clone();
    registro.registrar_funcion(
        &format!("{TIPO}.{nombre}"),
        Box::new(move |argumentos| {
            exigir_aridad(&etiqueta, &argumentos[1..], 0)?;
            let ruta =
                ruta_del_receptor_verificada(&guardian_sincrono, &etiqueta, argumentos, escritura)?;
            ejecutar(operacion(ruta))
        }),
    );

    let guardian_asincrono = Rc::clone(guardian);
    registro.registrar_funcion_con_vm(
        &format!("{TIPO}.{nombre}_asincrono"),
        Box::new(move |vm, argumentos| {
            exigir_aridad(&funcion, &argumentos[1..], 0)?;
            let ruta =
                ruta_del_receptor_verificada(&guardian_asincrono, &funcion, argumentos, escritura)?;
            tarea(vm, move || operacion(ruta))
        }),
    );
}

fn ruta_del_receptor_verificada(
    guardian: &GuardianPermisos,
    funcion: &str,
    argumentos: &[Valor],
    escritura: bool,
) -> Result<PathBuf, Fallo> {
    if escritura {
        ruta_escritura(guardian, funcion, argumentos)
    } else {
        ruta_lectura(guardian, funcion, argumentos)
    }
}

/// `archivo.escribir(texto)` y `archivo.escribir_bits(Bits)`.
fn registrar_escrituras(registro: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    const ESCRIBIR: &str = "Archivo.escribir";
    const ESCRIBIR_BITS: &str = "Archivo.escribir_bits";

    let guardian_texto = Rc::clone(guardian);
    registro.registrar_funcion(
        "Archivo.escribir",
        Box::new(move |argumentos| {
            exigir_aridad(ESCRIBIR, &argumentos[1..], 1)?;
            let ruta = ruta_escritura(&guardian_texto, ESCRIBIR, argumentos)?;
            let contenido = arg_texto(ESCRIBIR, argumentos, 1)?.to_string();
            ejecutar(op_escribir(ruta, contenido))
        }),
    );

    let guardian_texto_async = Rc::clone(guardian);
    registro.registrar_funcion_con_vm(
        "Archivo.escribir_asincrono",
        Box::new(move |vm, argumentos| {
            exigir_aridad(ESCRIBIR, &argumentos[1..], 1)?;
            let ruta = ruta_escritura(&guardian_texto_async, ESCRIBIR, argumentos)?;
            let contenido = arg_texto(ESCRIBIR, argumentos, 1)?.to_string();
            tarea(vm, move || op_escribir(ruta, contenido))
        }),
    );

    let guardian_bits = Rc::clone(guardian);
    registro.registrar_funcion(
        "Archivo.escribir_bits",
        Box::new(move |argumentos| {
            exigir_aridad(ESCRIBIR_BITS, &argumentos[1..], 1)?;
            let ruta = ruta_escritura(&guardian_bits, ESCRIBIR_BITS, argumentos)?;
            let bytes = bits::arg_bits(ESCRIBIR_BITS, argumentos, 1)?;
            ejecutar(op_escribir_bits(ruta, bytes))
        }),
    );

    let guardian_bits_async = Rc::clone(guardian);
    registro.registrar_funcion_con_vm(
        "Archivo.escribir_bits_asincrono",
        Box::new(move |vm, argumentos| {
            exigir_aridad(ESCRIBIR_BITS, &argumentos[1..], 1)?;
            let ruta = ruta_escritura(&guardian_bits_async, ESCRIBIR_BITS, argumentos)?;
            let bytes = bits::arg_bits(ESCRIBIR_BITS, argumentos, 1)?;
            tarea(vm, move || op_escribir_bits(ruta, bytes))
        }),
    );
}

/// `archivo.copiar(destino)` y `archivo.mover(destino)`.
fn registrar_traslados(registro: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    registrar_traslado(registro, guardian, "copiar", false);
    registrar_traslado(registro, guardian, "mover", true);
}

fn registrar_traslado(
    registro: &mut RegistroNativos,
    guardian: &Rc<GuardianPermisos>,
    nombre: &'static str,
    mueve: bool,
) {
    let funcion = format!("{TIPO}.{nombre}");

    let guardian_sincrono = Rc::clone(guardian);
    let etiqueta = funcion.clone();
    registro.registrar_funcion(
        &format!("{TIPO}.{nombre}"),
        Box::new(move |argumentos| {
            let (origen, destino) =
                rutas_de_traslado(&guardian_sincrono, &etiqueta, argumentos, mueve)?;
            let salida = if mueve {
                op_mover(origen, destino.clone())
            } else {
                op_copiar(origen, destino.clone())
            };
            let valor = ejecutar(salida)?;
            if mueve {
                reapuntar(argumentos, &destino);
            }
            Ok(valor)
        }),
    );

    let guardian_asincrono = Rc::clone(guardian);
    registro.registrar_funcion_con_vm(
        &format!("{TIPO}.{nombre}_asincrono"),
        Box::new(move |vm, argumentos| {
            let (origen, destino) =
                rutas_de_traslado(&guardian_asincrono, &funcion, argumentos, mueve)?;
            if mueve {
                reapuntar(argumentos, &destino);
            }
            tarea(vm, move || {
                if mueve {
                    op_mover(origen, destino)
                } else {
                    op_copiar(origen, destino)
                }
            })
        }),
    );
}

fn rutas_de_traslado(
    guardian: &GuardianPermisos,
    funcion: &str,
    argumentos: &[Valor],
    mueve: bool,
) -> Result<(PathBuf, PathBuf), Fallo> {
    exigir_aridad(funcion, &argumentos[1..], 1)?;
    let origen = if mueve {
        ruta_escritura(guardian, funcion, argumentos)?
    } else {
        ruta_lectura(guardian, funcion, argumentos)?
    };
    let destino = arg_texto(funcion, argumentos, 1)?;
    let destino = guardian
        .verificar_escritura(destino)
        .map_err(error_permiso)?;
    Ok((origen, destino))
}

/// `archivo.renombrar(nombre)`: el nuevo nombre vive en el mismo directorio.
fn registrar_renombrar(registro: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    const F: &str = "Archivo.renombrar";

    let guardian_sincrono = Rc::clone(guardian);
    registro.registrar_funcion(
        "Archivo.renombrar",
        Box::new(move |argumentos| {
            let (origen, destino) = rutas_de_renombrar(&guardian_sincrono, argumentos)?;
            let valor = ejecutar(op_mover(origen, destino.clone()))?;
            reapuntar(argumentos, &destino);
            Ok(valor)
        }),
    );

    let guardian_asincrono = Rc::clone(guardian);
    registro.registrar_funcion_con_vm(
        "Archivo.renombrar_asincrono",
        Box::new(move |vm, argumentos| {
            let (origen, destino) = rutas_de_renombrar(&guardian_asincrono, argumentos)?;
            reapuntar(argumentos, &destino);
            tarea(vm, move || op_mover(origen, destino))
        }),
    );

    fn rutas_de_renombrar(
        guardian: &GuardianPermisos,
        argumentos: &[Valor],
    ) -> Result<(PathBuf, PathBuf), Fallo> {
        exigir_aridad(F, &argumentos[1..], 1)?;
        let origen = ruta_escritura(guardian, F, argumentos)?;
        let destino = destino_de_renombrar(&origen, arg_texto(F, argumentos, 1)?)?;
        guardian
            .verificar_escritura(&destino.to_string_lossy())
            .map_err(error_permiso)?;
        Ok((origen, destino))
    }
}

/// Registra el objeto `Archivo` con el guardián de permisos.
pub fn registrar(registro: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    registro.registrar_modulo("archivo");

    // Metadatos derivados de la ruta.
    let partes: [(&'static str, Parte); 6] = [
        ("nombre", Parte::Nombre),
        ("nombre_sin_extension", Parte::NombreSinExtension),
        ("extension", Parte::Extension),
        ("extensión", Parte::Extension),
        ("directorio_padre", Parte::DirectorioPadre),
        ("ruta_completa", Parte::RutaCompleta),
    ];
    for (nombre, parte) in partes {
        registrar_metadato(registro, guardian, nombre, move |_, ruta| {
            Ok(texto_de_ruta(ruta, parte))
        });
    }

    // `ruta()` devuelve la ruta tal como se abrió, sin resolver.
    let guardian_ruta = Rc::clone(guardian);
    registro.registrar_funcion(
        "Archivo.ruta",
        Box::new(move |argumentos| {
            const F: &str = "Archivo.ruta";
            exigir_aridad(F, &argumentos[1..], 0)?;
            ruta_lectura(&guardian_ruta, F, argumentos)?;
            Ok(Valor::texto(ruta_del_receptor(F, argumentos)?))
        }),
    );

    // Metadatos que consultan el disco.
    registrar_metadato(registro, guardian, "tamaño", |_, ruta| tamano(ruta));
    registrar_metadato(registro, guardian, "tamano", |_, ruta| tamano(ruta));
    registrar_metadato(registro, guardian, "es_vacio", |_, ruta| es_vacio(ruta));
    registrar_metadato(registro, guardian, "es_vacío", |_, ruta| es_vacio(ruta));
    registrar_metadato(registro, guardian, "existe", |_, ruta| {
        Ok(Valor::Log(ruta.exists()))
    });
    registrar_metadato(registro, guardian, "es_archivo", |_, ruta| {
        Ok(Valor::Log(ruta.is_file()))
    });
    registrar_metadato(registro, guardian, "es_directorio", |_, ruta| {
        Ok(Valor::Log(ruta.is_dir()))
    });
    registrar_metadato(registro, guardian, "fecha_creacion", |funcion, ruta| {
        fecha(funcion, ruta, Fecha::Creacion)
    });
    registrar_metadato(registro, guardian, "fecha_modificacion", |funcion, ruta| {
        fecha(funcion, ruta, Fecha::Modificacion)
    });
    registrar_metadato(registro, guardian, "fecha_acceso", |funcion, ruta| {
        fecha(funcion, ruta, Fecha::Acceso)
    });

    // Operaciones sobre la ruta del receptor.
    let lecturas: [(&'static str, OpRuta); 5] = [
        ("leer", op_leer),
        ("leer_bits", op_leer_bits),
        ("listar_archivos", op_listar_archivos),
        ("listar_directorios", op_listar_directorios),
        ("refrescar", op_abrir),
    ];
    for (nombre, operacion) in lecturas {
        registrar_operacion(registro, guardian, nombre, false, operacion);
    }

    let escrituras: [(&'static str, OpRuta); 2] = [
        ("borrar", op_borrar),
        ("borrar_recursivo", op_borrar_recursivo),
    ];
    for (nombre, operacion) in escrituras {
        registrar_operacion(registro, guardian, nombre, true, operacion);
    }

    registrar_escrituras(registro, guardian);
    registrar_traslados(registro, guardian);
    registrar_renombrar(registro, guardian);
}
