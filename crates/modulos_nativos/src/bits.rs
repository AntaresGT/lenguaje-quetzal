//! Objeto `Bits` del módulo nativo `quetzal/sistema_archivos`.
//!
//! `Bits` es una secuencia inmutable de bytes (valores de 0 a 255): el tipo
//! con el que se leen y escriben archivos binarios. Vive en memoria, así que
//! ninguna de sus operaciones necesita el permiso `sistema-archivos`; el
//! permiso solo interviene al pasar por disco (`SistemaArchivos.leer_bits`,
//! `SistemaArchivos.escribir_bits`).
//!
//! Se sigue registrando bajo el módulo `bits` para que la ruta legada
//! `desde "quetzal/bits"` siga funcionando; la API canónica se importa desde
//! `quetzal/sistema_archivos`.

use std::cell::RefCell;
use std::rc::Rc;

use indexmap::IndexMap;
use maquina_virtual::{CargaNativa, DatosInstanciaNativa, Fallo, RegistroNativos, Valor};

use crate::util::{arg_entero, arg_lista, arg_texto, error, exigir_aridad};

/// Nombre del tipo visible para el usuario.
pub(crate) const TIPO: &str = "Bits";

/// Función nativa de este módulo, registrable bajo varios nombres.
type Nativa = fn(&[Valor]) -> Result<Valor, Fallo>;

// ----- Construcción y extracción -----

/// Crea la instancia de Quetzal que envuelve estos bytes.
pub(crate) fn instancia(bytes: &[u8]) -> Valor {
    let mut datos = IndexMap::new();
    datos.insert(
        "bytes".to_string(),
        Valor::lista(
            bytes
                .iter()
                .map(|byte| Valor::Entero(*byte as i64))
                .collect(),
        ),
    );
    datos.insert("texto".to_string(), Valor::texto(descripcion(bytes.len())));
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO),
        datos: RefCell::new(datos),
    }))
}

/// Misma instancia, pero como carga del bucle de eventos: es lo que devuelve
/// una lectura binaria asincrónica antes de volver al hilo de la VM.
pub(crate) fn carga(bytes: &[u8]) -> CargaNativa {
    CargaNativa::Instancia {
        tipo: TIPO.to_string(),
        campos: vec![
            (
                "bytes".to_string(),
                CargaNativa::Lista(
                    bytes
                        .iter()
                        .map(|byte| CargaNativa::Entero(*byte as i64))
                        .collect(),
                ),
            ),
            (
                "texto".to_string(),
                CargaNativa::Texto(descripcion(bytes.len())),
            ),
        ],
    }
}

fn descripcion(longitud: usize) -> String {
    format!("<Bits: {longitud} bytes>")
}

/// Extrae los bytes del argumento en `indice`, que debe ser un `Bits`.
pub(crate) fn arg_bits(
    funcion: &str,
    argumentos: &[Valor],
    indice: usize,
) -> Result<Vec<u8>, Fallo> {
    match argumentos.get(indice) {
        Some(Valor::InstanciaNativa(instancia)) if &*instancia.tipo == TIPO => {
            let datos = instancia.datos.borrow();
            match datos.get("bytes") {
                Some(Valor::Lista(lista)) => bytes_de_valores(funcion, &lista.borrow()),
                _ => Err(error(
                    "E0406",
                    format!("'{funcion}' recibió un Bits sin contenido válido"),
                )),
            }
        }
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba un Bits en el argumento {}, pero recibió '{}'",
                indice + 1,
                otro.nombre_tipo()
            ),
        )),
        None => Err(error(
            "E0210",
            format!("'{funcion}' necesita al menos {} argumentos", indice + 1),
        )),
    }
}

/// Convierte una lista de enteros en bytes, validando el rango 0–255.
fn bytes_de_valores(funcion: &str, valores: &[Valor]) -> Result<Vec<u8>, Fallo> {
    let mut bytes = Vec::with_capacity(valores.len());
    for valor in valores {
        let Valor::Entero(entero) = valor else {
            return Err(error(
                "E0406",
                format!(
                    "'{funcion}' esperaba enteros de 0 a 255, pero encontró '{}'",
                    valor.nombre_tipo()
                ),
            ));
        };
        bytes.push(byte_valido(funcion, *entero)?);
    }
    Ok(bytes)
}

fn byte_valido(funcion: &str, entero: i64) -> Result<u8, Fallo> {
    u8::try_from(entero).map_err(|_| {
        error(
            "E0406",
            format!("'{funcion}' recibió {entero}, que no es un byte válido (0 a 255)"),
        )
    })
}

/// Bytes del receptor de un método (`bits.longitud()`).
fn receptor(funcion: &str, argumentos: &[Valor]) -> Result<Vec<u8>, Fallo> {
    arg_bits(funcion, argumentos, 0)
}

// ----- Funciones libres -----

fn libre_vacio(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.vacío";
    exigir_aridad(F, argumentos, 0)?;
    Ok(instancia(&[]))
}

fn libre_desde_lista(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.desde_lista";
    exigir_aridad(F, argumentos, 1)?;
    let lista = arg_lista(F, argumentos, 0)?;
    let bytes = bytes_de_valores(F, &lista.borrow())?;
    Ok(instancia(&bytes))
}

fn libre_desde_texto(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.desde_texto";
    exigir_aridad(F, argumentos, 1)?;
    let texto = arg_texto(F, argumentos, 0)?;
    Ok(instancia(texto.as_bytes()))
}

fn libre_combinar(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.combinar";
    exigir_aridad(F, argumentos, 1)?;
    let lista = arg_lista(F, argumentos, 0)?;
    let partes = lista.borrow().clone();
    let mut bytes = Vec::new();
    for (indice, _) in partes.iter().enumerate() {
        bytes.extend(arg_bits(F, &partes, indice)?);
    }
    Ok(instancia(&bytes))
}

// ----- Métodos de instancia -----

fn metodo_longitud(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.longitud";
    exigir_aridad(F, &argumentos[1..], 0)?;
    Ok(Valor::Entero(receptor(F, argumentos)?.len() as i64))
}

fn metodo_obtener(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.obtener";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let bytes = receptor(F, argumentos)?;
    let indice = indice_valido(F, arg_entero(F, argumentos, 1)?, bytes.len())?;
    Ok(Valor::Entero(bytes[indice] as i64))
}

fn metodo_establecer(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.establecer";
    exigir_aridad(F, &argumentos[1..], 2)?;
    let mut bytes = receptor(F, argumentos)?;
    let indice = indice_valido(F, arg_entero(F, argumentos, 1)?, bytes.len())?;
    bytes[indice] = byte_valido(F, arg_entero(F, argumentos, 2)?)?;
    Ok(instancia(&bytes))
}

fn metodo_trozo(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.trozo";
    exigir_aridad(F, &argumentos[1..], 2)?;
    let bytes = receptor(F, argumentos)?;
    let inicio = arg_entero(F, argumentos, 1)?;
    let fin = arg_entero(F, argumentos, 2)?;
    if inicio < 0 || fin < inicio || fin > bytes.len() as i64 {
        return Err(error(
            "E0403",
            format!(
                "'{F}' recibió un rango inválido ({inicio}, {fin}) para {} bytes",
                bytes.len()
            ),
        ));
    }
    Ok(instancia(&bytes[inicio as usize..fin as usize]))
}

fn metodo_concatenar(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.concatenar";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let mut bytes = receptor(F, argumentos)?;
    bytes.extend(arg_bits(F, argumentos, 1)?);
    Ok(instancia(&bytes))
}

fn metodo_texto(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.texto";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let bytes = receptor(F, argumentos)?;
    String::from_utf8(bytes).map(Valor::texto).map_err(|_| {
        error(
            "E0406",
            format!("'{F}' recibió bytes que no son texto UTF-8 válido"),
        )
    })
}

fn metodo_lista(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.lista";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let bytes = receptor(F, argumentos)?;
    Ok(Valor::lista(
        bytes
            .iter()
            .map(|byte| Valor::Entero(*byte as i64))
            .collect(),
    ))
}

fn metodo_es_igual(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Bits.es_igual";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let bytes = receptor(F, argumentos)?;
    Ok(Valor::Log(bytes == arg_bits(F, argumentos, 1)?))
}

fn indice_valido(funcion: &str, indice: i64, longitud: usize) -> Result<usize, Fallo> {
    crate::util::indice_normalizado(indice, longitud).ok_or_else(|| {
        error(
            "E0403",
            format!("'{funcion}' recibió el índice {indice}, fuera de rango para {longitud} bytes"),
        )
    })
}

// ----- Registro -----

pub fn registrar(registro: &mut RegistroNativos) {
    registro.registrar_modulo("bits");

    let libres: [(&str, Nativa); 5] = [
        ("vacío", libre_vacio),
        ("vacio", libre_vacio),
        ("desde_lista", libre_desde_lista),
        ("desde_texto", libre_desde_texto),
        ("combinar", libre_combinar),
    ];
    for (nombre, funcion) in libres {
        registro.registrar_funcion(&format!("bits.{nombre}"), Box::new(funcion));
    }

    let metodos: [(&str, Nativa); 8] = [
        ("longitud", metodo_longitud),
        ("obtener", metodo_obtener),
        ("establecer", metodo_establecer),
        ("trozo", metodo_trozo),
        ("concatenar", metodo_concatenar),
        ("texto", metodo_texto),
        ("lista", metodo_lista),
        ("es_igual", metodo_es_igual),
    ];
    for (nombre, funcion) in metodos {
        registro.registrar_funcion(&format!("{TIPO}.{nombre}"), Box::new(funcion));
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn bits(valores: &[u8]) -> Valor {
        instancia(valores)
    }

    #[test]
    fn desde_lista_valida_el_rango_de_cada_byte() {
        let lista = Valor::lista(vec![Valor::Entero(0), Valor::Entero(255)]);
        let creado = libre_desde_lista(&[lista]).expect("0 y 255 son bytes válidos");
        assert_eq!(arg_bits("prueba", &[creado], 0).unwrap(), vec![0, 255]);

        let invalida = Valor::lista(vec![Valor::Entero(256)]);
        assert!(libre_desde_lista(&[invalida]).is_err());
    }

    #[test]
    fn desde_texto_codifica_en_utf8_y_texto_decodifica() {
        let creado = libre_desde_texto(&[Valor::texto("café")]).expect("texto válido");
        let bytes = arg_bits("prueba", std::slice::from_ref(&creado), 0).unwrap();
        assert_eq!(bytes.len(), 5, "la é ocupa dos bytes en UTF-8");

        let decodificado = metodo_texto(&[creado]).expect("los bytes son UTF-8 válido");
        assert_eq!(maquina_virtual::texto_de_valor(&decodificado), "café");
    }

    #[test]
    fn texto_falla_con_bytes_que_no_son_utf8() {
        assert!(metodo_texto(&[bits(&[137, 80, 78, 71])]).is_err());
    }

    #[test]
    fn establecer_y_concatenar_no_modifican_el_original() {
        let original = bits(&[1, 2, 3]);
        let cambiado = metodo_establecer(&[original.clone(), Valor::Entero(0), Valor::Entero(9)])
            .expect("índice y byte válidos");
        assert_eq!(
            arg_bits("prueba", std::slice::from_ref(&original), 0).unwrap(),
            vec![1, 2, 3]
        );
        assert_eq!(arg_bits("prueba", &[cambiado], 0).unwrap(), vec![9, 2, 3]);

        let unido = metodo_concatenar(&[original, bits(&[4])]).expect("concatenar dos Bits");
        assert_eq!(arg_bits("prueba", &[unido], 0).unwrap(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn trozo_respeta_los_limites() {
        let origen = bits(&[1, 2, 3, 4]);
        let medio = metodo_trozo(&[origen.clone(), Valor::Entero(1), Valor::Entero(3)])
            .expect("rango válido");
        assert_eq!(arg_bits("prueba", &[medio], 0).unwrap(), vec![2, 3]);
        assert!(metodo_trozo(&[origen, Valor::Entero(0), Valor::Entero(9)]).is_err());
    }

    #[test]
    fn combinar_une_todas_las_secuencias() {
        let lista = Valor::lista(vec![bits(&[1]), bits(&[2, 3]), bits(&[])]);
        let unido = libre_combinar(&[lista]).expect("todas son secuencias de Bits");
        assert_eq!(arg_bits("prueba", &[unido], 0).unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn es_igual_compara_por_contenido() {
        let iguales = metodo_es_igual(&[bits(&[1, 2]), bits(&[1, 2])]).unwrap();
        let distintos = metodo_es_igual(&[bits(&[1, 2]), bits(&[2, 1])]).unwrap();
        assert!(matches!(iguales, Valor::Log(true)));
        assert!(matches!(distintos, Valor::Log(false)));
    }

    #[test]
    fn obtener_rechaza_indices_fuera_de_rango() {
        assert!(metodo_obtener(&[bits(&[1]), Valor::Entero(5)]).is_err());
        let ultimo = metodo_obtener(&[bits(&[1, 2]), Valor::Entero(-1)]).expect("índice negativo");
        assert!(matches!(ultimo, Valor::Entero(2)));
    }
}
