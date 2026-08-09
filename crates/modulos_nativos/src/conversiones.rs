//! Conversiones entre tipos según `ejemplos/conversion_tipos.qz`:
//! `.texto()`, `.entero()`, `.numero()`, `.log()`, `.jsn()`, `.lista()`.

use std::str::FromStr;

use maquina_virtual::valores::{json_a_valor, texto_de_valor};
use maquina_virtual::{RegistroNativos, Valor};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::util::{arg_texto, error, exigir_aridad};

pub fn registrar(registro: &mut RegistroNativos) {
    // `.texto()` para los tipos primitivos restantes (lista y jsn tienen el
    // suyo en sus módulos).
    for tipo in ["entero", "numero", "log", "nulo"] {
        let nombre: &'static str = match tipo {
            "entero" => "entero.texto",
            "numero" => "numero.texto",
            "log" => "log.texto",
            _ => "nulo.texto",
        };
        registro.registrar_funcion(
            nombre,
            Box::new(move |argumentos| {
                exigir_aridad(nombre, argumentos, 1)?;
                Ok(Valor::texto(texto_de_valor(&argumentos[0])))
            }),
        );
    }
    registro.registrar_funcion(
        "texto.texto",
        Box::new(|argumentos| {
            exigir_aridad("texto.texto", argumentos, 1)?;
            Ok(argumentos[0].clone())
        }),
    );

    // texto → otros tipos.
    registro.registrar_funcion(
        "texto.entero",
        Box::new(|argumentos| {
            exigir_aridad("texto.entero", argumentos, 1)?;
            let texto = arg_texto("texto.entero", argumentos, 0)?;
            texto
                .trim()
                .parse::<i64>()
                .map(Valor::Entero)
                .map_err(|_| error("E0406", format!("'{texto}' no se puede convertir a entero")))
        }),
    );
    registro.registrar_funcion(
        "texto.numero",
        Box::new(|argumentos| {
            exigir_aridad("texto.numero", argumentos, 1)?;
            let texto = arg_texto("texto.numero", argumentos, 0)?;
            Decimal::from_str(texto.trim())
                .map(Valor::Numero)
                .map_err(|_| error("E0406", format!("'{texto}' no se puede convertir a número")))
        }),
    );
    registro.registrar_funcion(
        "texto.log",
        Box::new(|argumentos| {
            exigir_aridad("texto.log", argumentos, 1)?;
            let texto = arg_texto("texto.log", argumentos, 0)?;
            match texto.trim().to_lowercase().as_str() {
                "verdadero" | "true" => Ok(Valor::Log(true)),
                "falso" | "false" => Ok(Valor::Log(false)),
                otro => Err(error(
                    "E0406",
                    format!("'{otro}' no se puede convertir a valor lógico"),
                )),
            }
        }),
    );
    registro.registrar_funcion(
        "texto.jsn",
        Box::new(|argumentos| {
            exigir_aridad("texto.jsn", argumentos, 1)?;
            let texto = arg_texto("texto.jsn", argumentos, 0)?;
            let json: serde_json::Value = serde_json::from_str(texto)
                .map_err(|causa| error("E0406", format!("el texto no es JSON válido: {causa}")))?;
            Ok(json_a_valor(&json))
        }),
    );
    registro.registrar_funcion(
        "texto.lista",
        Box::new(|argumentos| {
            exigir_aridad("texto.lista", argumentos, 1)?;
            let texto = arg_texto("texto.lista", argumentos, 0)?;
            // "1,2,3" → [1, 2, 3]; los fragmentos no numéricos quedan como texto.
            let elementos = texto
                .split(',')
                .map(|fragmento| {
                    let fragmento = fragmento.trim();
                    if let Ok(entero) = fragmento.parse::<i64>() {
                        Valor::Entero(entero)
                    } else if let Ok(decimal) = Decimal::from_str(fragmento) {
                        Valor::Numero(decimal)
                    } else {
                        Valor::texto(fragmento)
                    }
                })
                .collect();
            Ok(Valor::lista(elementos))
        }),
    );

    // entero ↔ número y lógicos.
    registro.registrar_funcion(
        "entero.numero",
        Box::new(|argumentos| {
            exigir_aridad("entero.numero", argumentos, 1)?;
            match &argumentos[0] {
                Valor::Entero(entero) => Ok(Valor::Numero(Decimal::from(*entero))),
                otro => Err(error(
                    "E0406",
                    format!("se esperaba un entero, no '{}'", otro.nombre_tipo()),
                )),
            }
        }),
    );
    registro.registrar_funcion(
        "entero.log",
        Box::new(|argumentos| {
            exigir_aridad("entero.log", argumentos, 1)?;
            match &argumentos[0] {
                Valor::Entero(entero) => Ok(Valor::Log(*entero != 0)),
                otro => Err(error(
                    "E0406",
                    format!("se esperaba un entero, no '{}'", otro.nombre_tipo()),
                )),
            }
        }),
    );
    registro.registrar_funcion(
        "numero.entero",
        Box::new(|argumentos| {
            exigir_aridad("numero.entero", argumentos, 1)?;
            match &argumentos[0] {
                Valor::Numero(decimal) => decimal
                    .trunc()
                    .to_i64()
                    .map(Valor::Entero)
                    .ok_or_else(|| error("E0406", format!("{decimal} no cabe en un entero"))),
                otro => Err(error(
                    "E0406",
                    format!("se esperaba un número, no '{}'", otro.nombre_tipo()),
                )),
            }
        }),
    );
    registro.registrar_funcion(
        "log.entero",
        Box::new(|argumentos| {
            exigir_aridad("log.entero", argumentos, 1)?;
            match &argumentos[0] {
                Valor::Log(valor) => Ok(Valor::Entero(i64::from(*valor))),
                otro => Err(error(
                    "E0406",
                    format!("se esperaba un lógico, no '{}'", otro.nombre_tipo()),
                )),
            }
        }),
    );
}
