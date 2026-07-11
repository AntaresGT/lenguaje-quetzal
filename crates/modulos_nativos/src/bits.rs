//! Módulo nativo `quetzal/bits`: el objeto `Bits`.
//!
//! Un `Bits` es un búfer mutable de bytes (enteros 0–255) pensado para
//! trabajar datos binarios sin usar `lista<entero>`: lectura y escritura de
//! archivos binarios, operaciones bit a bit (`y`, `o`, `oexclusivo`, `negar`),
//! desplazamientos y conversiones (texto UTF-8, hexadecimal, lista).
//! No requiere permisos: nunca toca el sistema.

use std::cell::RefCell;
use std::rc::Rc;

use maquina_virtual::{DatosInstanciaNativa, Fallo, RegistroNativos, Valor};

use crate::util::{
    arg_bits, arg_entero, arg_texto, byte_de_valor, error, exigir_aridad, indice_normalizado,
};

const TIPO: &str = "Bits";

/// Función nativa de este módulo, registrable bajo varios nombres.
type Nativa = fn(&[Valor]) -> Result<Valor, Fallo>;

// ----- Instancias -----

/// Crea una instancia de `Bits` a partir de bytes crudos.
///
/// La construcción vive en `maquina_virtual` (no aquí) para que el bucle de
/// eventos pueda producir instancias `Bits` a partir de resultados binarios
/// de tareas nativas (E/S) sin depender de este crate. Esta función es solo
/// un alias conveniente para el resto del módulo.
pub(crate) fn valor_bits(bytes: &[u8]) -> Valor {
    maquina_virtual::instancia_bits_desde(bytes)
}

/// Receptor (`esto`) de un método: la instancia de `Bits`.
fn receptor(funcion: &str, argumentos: &[Valor]) -> Result<Rc<DatosInstanciaNativa>, Fallo> {
    match argumentos.first() {
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == TIPO => {
            Ok(Rc::clone(instancia))
        }
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba una instancia de Bits, pero recibió '{}'",
                otro.nombre_tipo()
            ),
        )),
        None => Err(error(
            "E0210",
            format!("'{funcion}' necesita el receptor"),
        )),
    }
}

/// La lista interna de bytes de la instancia (compartida, mutable).
fn lista_interna(
    funcion: &str,
    instancia: &DatosInstanciaNativa,
) -> Result<Rc<RefCell<Vec<Valor>>>, Fallo> {
    match instancia.datos.borrow().get("datos") {
        Some(Valor::Lista(lista)) => Ok(Rc::clone(lista)),
        _ => Err(error(
            "E0406",
            format!("'{funcion}' recibió un Bits sin datos internos"),
        )),
    }
}

/// Bytes crudos del receptor.
fn bytes_del_receptor(funcion: &str, argumentos: &[Valor]) -> Result<Vec<u8>, Fallo> {
    let instancia = receptor(funcion, argumentos)?;
    let lista = lista_interna(funcion, &instancia)?;
    let bytes = lista
        .borrow()
        .iter()
        .map(|valor| byte_de_valor(funcion, valor))
        .collect::<Result<Vec<u8>, Fallo>>()?;
    Ok(bytes)
}

/// Actualiza el campo `texto` tras una mutación.
fn actualizar_representacion(funcion: &str, instancia: &DatosInstanciaNativa) -> Result<(), Fallo> {
    let lista = lista_interna(funcion, instancia)?;
    let bytes = lista
        .borrow()
        .iter()
        .map(|valor| byte_de_valor(funcion, valor))
        .collect::<Result<Vec<u8>, Fallo>>()?;
    instancia
        .datos
        .borrow_mut()
        .insert(
            "texto".to_string(),
            Valor::texto(maquina_virtual::valores::representacion_bits(&bytes)),
        );
    Ok(())
}

// ----- Constructor: `nuevo Bits(...)` -----

fn constructor(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "nuevo Bits";
    match argumentos.len() {
        // `nuevo Bits()`: búfer vacío.
        0 => Ok(valor_bits(&[])),
        // `nuevo Bits(tamaño)`, `nuevo Bits(texto)` o `nuevo Bits(lista)`.
        1 => match &argumentos[0] {
            Valor::Entero(cantidad) if *cantidad >= 0 => {
                Ok(valor_bits(&vec![0u8; *cantidad as usize]))
            }
            Valor::Entero(cantidad) => Err(error(
                "E0406",
                format!("'{F}' espera un tamaño positivo, pero recibió {cantidad}"),
            )),
            Valor::Texto(texto) => Ok(valor_bits(texto.as_bytes())),
            Valor::Lista(_) => Ok(valor_bits(&arg_bits(F, argumentos, 0)?)),
            otro => Err(error(
                "E0406",
                format!(
                    "'{F}' espera un tamaño (entero), un texto o una lista de bytes, pero recibió '{}'",
                    otro.nombre_tipo()
                ),
            )),
        },
        otros => Err(error(
            "E0210",
            format!("'{F}' acepta 0 o 1 argumentos, pero recibió {otros}"),
        )),
    }
}

// ----- Funciones libres: `Bits.x()` -----

/// `Bits.desde_hex("4f4b0a")`: bytes desde un texto hexadecimal (admite
/// espacios entre pares).
fn libre_desde_hex(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.desde_hex";
    exigir_aridad(F, argumentos, 1)?;
    let texto = arg_texto(F, argumentos, 0)?;
    let limpio: String = texto.chars().filter(|c| !c.is_whitespace()).collect();
    if !limpio.len().is_multiple_of(2) {
        return Err(error(
            "E0406",
            format!("'{F}' espera una cantidad par de dígitos hexadecimales, pero recibió {}", limpio.len()),
        ));
    }
    let bytes = (0..limpio.len())
        .step_by(2)
        .map(|inicio| {
            u8::from_str_radix(&limpio[inicio..inicio + 2], 16).map_err(|_| {
                error(
                    "E0406",
                    format!(
                        "'{F}' no pudo interpretar '{}' como byte hexadecimal",
                        &limpio[inicio..inicio + 2]
                    ),
                )
            })
        })
        .collect::<Result<Vec<u8>, Fallo>>()?;
    Ok(valor_bits(&bytes))
}

// ----- Métodos: consulta -----

fn metodo_longitud(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.longitud";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let bytes = bytes_del_receptor(F, argumentos)?;
    Ok(Valor::Entero(bytes.len() as i64))
}

fn metodo_obtener(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.obtener";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let bytes = bytes_del_receptor(F, argumentos)?;
    let indice = arg_entero(F, argumentos, 1)?;
    let real = indice_normalizado(indice, bytes.len()).ok_or_else(|| {
        error(
            "E0406",
            format!(
                "'{F}' recibió el índice {indice}, pero el búfer tiene {} bytes",
                bytes.len()
            ),
        )
    })?;
    Ok(Valor::Entero(i64::from(bytes[real])))
}

fn metodo_rebanada(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.rebanada";
    exigir_aridad(F, &argumentos[1..], 2)?;
    let bytes = bytes_del_receptor(F, argumentos)?;
    let longitud = bytes.len() as i64;
    let normalizar = |valor: i64| -> i64 {
        let real = if valor < 0 { longitud + valor } else { valor };
        real.clamp(0, longitud)
    };
    let inicio = normalizar(arg_entero(F, argumentos, 1)?);
    let fin = normalizar(arg_entero(F, argumentos, 2)?);
    if inicio > fin {
        return Err(error(
            "E0406",
            format!("'{F}' recibió un rango invertido ({inicio} > {fin})"),
        ));
    }
    Ok(valor_bits(&bytes[inicio as usize..fin as usize]))
}

fn metodo_a_lista(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.a_lista";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let bytes = bytes_del_receptor(F, argumentos)?;
    Ok(Valor::lista(
        bytes
            .iter()
            .map(|byte| Valor::Entero(i64::from(*byte)))
            .collect(),
    ))
}

fn metodo_a_texto(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.a_texto";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let bytes = bytes_del_receptor(F, argumentos)?;
    String::from_utf8(bytes).map(Valor::texto).map_err(|_| {
        error(
            "E0406",
            format!("'{F}' no pudo decodificar el contenido como texto UTF-8 válido"),
        )
    })
}

fn metodo_a_hex(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.a_hex";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let bytes = bytes_del_receptor(F, argumentos)?;
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(Valor::texto(hex))
}

// ----- Métodos: mutación -----

fn metodo_fijar(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.fijar";
    exigir_aridad(F, &argumentos[1..], 2)?;
    let instancia = receptor(F, argumentos)?;
    let lista = lista_interna(F, &instancia)?;
    let longitud = lista.borrow().len();
    let indice = arg_entero(F, argumentos, 1)?;
    let real = indice_normalizado(indice, longitud).ok_or_else(|| {
        error(
            "E0406",
            format!("'{F}' recibió el índice {indice}, pero el búfer tiene {longitud} bytes"),
        )
    })?;
    let byte = byte_de_valor(F, &argumentos[2])?;
    lista.borrow_mut()[real] = Valor::Entero(i64::from(byte));
    actualizar_representacion(F, &instancia)?;
    Ok(Valor::Nulo)
}

fn metodo_agregar(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.agregar";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let instancia = receptor(F, argumentos)?;
    let lista = lista_interna(F, &instancia)?;
    let byte = byte_de_valor(F, &argumentos[1])?;
    lista.borrow_mut().push(Valor::Entero(i64::from(byte)));
    actualizar_representacion(F, &instancia)?;
    Ok(Valor::Nulo)
}

fn metodo_extender(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.extender";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let instancia = receptor(F, argumentos)?;
    let lista = lista_interna(F, &instancia)?;
    let nuevos = arg_bits(F, argumentos, 1)?;
    lista
        .borrow_mut()
        .extend(nuevos.iter().map(|byte| Valor::Entero(i64::from(*byte))));
    actualizar_representacion(F, &instancia)?;
    Ok(Valor::Nulo)
}

fn metodo_limpiar(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.limpiar";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let instancia = receptor(F, argumentos)?;
    let lista = lista_interna(F, &instancia)?;
    lista.borrow_mut().clear();
    actualizar_representacion(F, &instancia)?;
    Ok(Valor::Nulo)
}

// ----- Métodos: operaciones binarias (devuelven Bits nuevo) -----

/// Operación byte a byte entre dos búferes de la misma longitud.
fn operacion_binaria(
    funcion: &str,
    argumentos: &[Valor],
    operar: fn(u8, u8) -> u8,
) -> Result<Valor, Fallo> {
    exigir_aridad(funcion, &argumentos[1..], 1)?;
    let propios = bytes_del_receptor(funcion, argumentos)?;
    let otros = arg_bits(funcion, argumentos, 1)?;
    if propios.len() != otros.len() {
        return Err(error(
            "E0406",
            format!(
                "'{funcion}' necesita búferes de la misma longitud, pero recibió {} y {} bytes",
                propios.len(),
                otros.len()
            ),
        ));
    }
    let resultado: Vec<u8> = propios
        .iter()
        .zip(otros.iter())
        .map(|(a, b)| operar(*a, *b))
        .collect();
    Ok(valor_bits(&resultado))
}

fn metodo_y(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    operacion_binaria("Bits.y", argumentos, |a, b| a & b)
}

fn metodo_o(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    operacion_binaria("Bits.o", argumentos, |a, b| a | b)
}

fn metodo_oexclusivo(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    operacion_binaria("Bits.oexclusivo", argumentos, |a, b| a ^ b)
}

fn metodo_negar(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.negar";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let bytes = bytes_del_receptor(F, argumentos)?;
    let resultado: Vec<u8> = bytes.iter().map(|byte| !byte).collect();
    Ok(valor_bits(&resultado))
}

/// Desplaza los bits de todo el búfer (longitud constante, rellena con ceros).
fn desplazar(bytes: &[u8], cantidad: u64, hacia_izquierda: bool) -> Vec<u8> {
    let longitud = bytes.len();
    if longitud == 0 || cantidad >= (longitud as u64) * 8 {
        return vec![0u8; longitud];
    }
    let saltos_byte = (cantidad / 8) as usize;
    let saltos_bit = (cantidad % 8) as u32;
    let mut resultado = vec![0u8; longitud];
    for (indice, destino) in resultado.iter_mut().enumerate() {
        let valor = if hacia_izquierda {
            let principal = bytes
                .get(indice + saltos_byte)
                .map_or(0, |byte| byte.wrapping_shl(saltos_bit));
            let arrastre = if saltos_bit == 0 {
                0
            } else {
                bytes
                    .get(indice + saltos_byte + 1)
                    .map_or(0, |byte| byte >> (8 - saltos_bit))
            };
            principal | arrastre
        } else {
            let principal = indice
                .checked_sub(saltos_byte)
                .and_then(|origen| bytes.get(origen))
                .map_or(0, |byte| byte >> saltos_bit);
            let arrastre = if saltos_bit == 0 {
                0
            } else {
                indice
                    .checked_sub(saltos_byte + 1)
                    .and_then(|origen| bytes.get(origen))
                    .map_or(0, |byte| byte.wrapping_shl(8 - saltos_bit))
            };
            principal | arrastre
        };
        *destino = valor;
    }
    resultado
}

fn metodo_desplazar(funcion: &str, argumentos: &[Valor], izquierda: bool) -> Result<Valor, Fallo> {
    exigir_aridad(funcion, &argumentos[1..], 1)?;
    let bytes = bytes_del_receptor(funcion, argumentos)?;
    let cantidad = arg_entero(funcion, argumentos, 1)?;
    if cantidad < 0 {
        return Err(error(
            "E0406",
            format!("'{funcion}' espera una cantidad positiva de bits, pero recibió {cantidad}"),
        ));
    }
    Ok(valor_bits(&desplazar(&bytes, cantidad as u64, izquierda)))
}

fn metodo_desplazar_izquierda(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    metodo_desplazar("Bits.desplazar_izquierda", argumentos, true)
}

fn metodo_desplazar_derecha(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    metodo_desplazar("Bits.desplazar_derecha", argumentos, false)
}

// ----- Registro -----

pub fn registrar(registro: &mut RegistroNativos) {
    registro.registrar_modulo("bits");

    // Constructor de `nuevo Bits(...)`.
    registro.registrar_funcion("bits.constructor", Box::new(constructor));

    // Funciones libres: `Bits.desde_hex(...)`.
    registro.registrar_funcion("bits.desde_hex", Box::new(libre_desde_hex));

    // Métodos de instancia: `b.metodo(...)` despacha como `Bits.metodo` con el
    // receptor como primer argumento.
    let metodos: [(&str, Nativa); 14] = [
        ("longitud", metodo_longitud),
        ("obtener", metodo_obtener),
        ("rebanada", metodo_rebanada),
        ("a_lista", metodo_a_lista),
        ("a_texto", metodo_a_texto),
        ("a_hex", metodo_a_hex),
        ("fijar", metodo_fijar),
        ("agregar", metodo_agregar),
        ("extender", metodo_extender),
        ("limpiar", metodo_limpiar),
        ("y", metodo_y),
        ("o", metodo_o),
        ("oexclusivo", metodo_oexclusivo),
        ("negar", metodo_negar),
    ];
    for (nombre, funcion) in metodos {
        registro.registrar_funcion(&format!("{TIPO}.{nombre}"), Box::new(funcion));
    }
    registro.registrar_funcion(
        &format!("{TIPO}.desplazar_izquierda"),
        Box::new(metodo_desplazar_izquierda),
    );
    registro.registrar_funcion(
        &format!("{TIPO}.desplazar_derecha"),
        Box::new(metodo_desplazar_derecha),
    );
}
