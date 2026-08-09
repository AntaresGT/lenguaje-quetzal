//! Módulo nativo `quetzal/matematica` según `ejemplos/modulo_matematicas.qz`.

use std::str::FromStr;

use maquina_virtual::{Fallo, RegistroNativos, Valor};
use rand::Rng;
use rust_decimal::Decimal;

use crate::util::{
    arg_decimal, arg_entero, arg_lista, decimal_a_f64, error, exigir_aridad, f64_a_decimal,
};

pub fn registrar(registro: &mut RegistroNativos) {
    registro.registrar_modulo("matematica");

    // Constantes (28 dígitos de precisión decimal).
    let pi = Decimal::from_str("3.1415926535897932384626433833").unwrap_or(Decimal::ZERO);
    registro.registrar_constante("matematica.PI", Valor::Numero(pi));
    registro.registrar_constante(
        "matematica.TAU",
        Valor::Numero(Decimal::from_str("6.2831853071795864769252867666").unwrap_or(Decimal::ZERO)),
    );
    registro.registrar_constante(
        "matematica.E",
        Valor::Numero(Decimal::from_str("2.7182818284590452353602874714").unwrap_or(Decimal::ZERO)),
    );

    // Aritmética exacta con decimales.
    binaria_exacta(registro, "matematica.sumar", |a, b| a.checked_add(b));
    binaria_exacta(registro, "matematica.restar", |a, b| a.checked_sub(b));
    binaria_exacta(registro, "matematica.multiplicar", |a, b| a.checked_mul(b));
    registro.registrar_funcion(
        "matematica.dividir",
        Box::new(|argumentos| {
            exigir_aridad("matematica.dividir", argumentos, 2)?;
            let a = arg_decimal("matematica.dividir", argumentos, 0)?;
            let b = arg_decimal("matematica.dividir", argumentos, 1)?;
            if b.is_zero() {
                return Err(error("E0401", "división por cero"));
            }
            a.checked_div(b)
                .map(Valor::Numero)
                .ok_or_else(|| error("E0402", "desbordamiento al dividir"))
        }),
    );
    registro.registrar_funcion(
        "matematica.potencia",
        Box::new(|argumentos| flotante_binaria("matematica.potencia", argumentos, f64::powf)),
    );

    // Trigonometría (vía f64).
    flotante(registro, "matematica.seno", f64::sin);
    flotante(registro, "matematica.coseno", f64::cos);
    flotante(registro, "matematica.tangente", f64::tan);
    flotante(registro, "matematica.grados_a_radianes", f64::to_radians);
    flotante(registro, "matematica.radianes_a_grados", f64::to_degrees);
    flotante(registro, "matematica.logaritmo", f64::ln);
    flotante(registro, "matematica.exponencial", f64::exp);
    flotante(registro, "matematica.raiz_cuadrada", f64::sqrt);
    flotante(registro, "matematica.raiz", f64::sqrt);
    registro.registrar_funcion(
        "matematica.logaritmo_base",
        Box::new(|argumentos| flotante_binaria("matematica.logaritmo_base", argumentos, f64::log)),
    );
    registro.registrar_funcion(
        "matematica.hipotenusa",
        Box::new(|argumentos| flotante_binaria("matematica.hipotenusa", argumentos, f64::hypot)),
    );

    // Redondeo exacto.
    registro.registrar_funcion(
        "matematica.absoluto",
        Box::new(|argumentos| {
            exigir_aridad("matematica.absoluto", argumentos, 1)?;
            match &argumentos[0] {
                Valor::Entero(entero) => entero
                    .checked_abs()
                    .map(Valor::Entero)
                    .ok_or_else(|| error("E0402", "desbordamiento al calcular el absoluto")),
                otro => Ok(Valor::Numero(
                    arg_decimal("matematica.absoluto", std::slice::from_ref(otro), 0)?.abs(),
                )),
            }
        }),
    );
    registro.registrar_funcion(
        "matematica.redondear",
        Box::new(|argumentos| {
            exigir_aridad("matematica.redondear", argumentos, 2)?;
            let valor = arg_decimal("matematica.redondear", argumentos, 0)?;
            let decimales = arg_entero("matematica.redondear", argumentos, 1)?;
            let decimales = u32::try_from(decimales)
                .map_err(|_| error("E0406", "los decimales de 'redondear' deben ser >= 0"))?;
            Ok(Valor::Numero(valor.round_dp(decimales)))
        }),
    );
    registro.registrar_funcion(
        "matematica.piso",
        Box::new(|argumentos| {
            exigir_aridad("matematica.piso", argumentos, 1)?;
            Ok(Valor::Numero(
                arg_decimal("matematica.piso", argumentos, 0)?.floor(),
            ))
        }),
    );
    registro.registrar_funcion(
        "matematica.techo",
        Box::new(|argumentos| {
            exigir_aridad("matematica.techo", argumentos, 1)?;
            Ok(Valor::Numero(
                arg_decimal("matematica.techo", argumentos, 0)?.ceil(),
            ))
        }),
    );

    // Estadística sobre listas de números.
    registro.registrar_funcion(
        "matematica.maximo",
        Box::new(|argumentos| {
            estadistica("matematica.maximo", argumentos, |valores| {
                valores.iter().copied().reduce(Decimal::max)
            })
        }),
    );
    registro.registrar_funcion(
        "matematica.minimo",
        Box::new(|argumentos| {
            estadistica("matematica.minimo", argumentos, |valores| {
                valores.iter().copied().reduce(Decimal::min)
            })
        }),
    );
    registro.registrar_funcion(
        "matematica.suma_total",
        Box::new(|argumentos| {
            estadistica("matematica.suma_total", argumentos, |valores| {
                valores
                    .iter()
                    .try_fold(Decimal::ZERO, |suma, valor| suma.checked_add(*valor))
            })
        }),
    );
    registro.registrar_funcion(
        "matematica.producto_total",
        Box::new(|argumentos| {
            estadistica("matematica.producto_total", argumentos, |valores| {
                valores
                    .iter()
                    .try_fold(Decimal::ONE, |producto, valor| producto.checked_mul(*valor))
            })
        }),
    );
    registro.registrar_funcion(
        "matematica.promedio",
        Box::new(|argumentos| {
            estadistica("matematica.promedio", argumentos, |valores| {
                if valores.is_empty() {
                    return None;
                }
                let suma = valores
                    .iter()
                    .try_fold(Decimal::ZERO, |suma, valor| suma.checked_add(*valor))?;
                suma.checked_div(Decimal::from(valores.len()))
            })
        }),
    );

    // Aleatorios.
    registro.registrar_funcion(
        "matematica.aleatorio",
        Box::new(|argumentos| {
            exigir_aridad("matematica.aleatorio", argumentos, 0)?;
            f64_a_decimal("matematica.aleatorio", rand::rng().random::<f64>())
        }),
    );
    registro.registrar_funcion(
        "matematica.aleatorio_rango",
        Box::new(|argumentos| {
            exigir_aridad("matematica.aleatorio_rango", argumentos, 2)?;
            let inicio = arg_entero("matematica.aleatorio_rango", argumentos, 0)?;
            let fin = arg_entero("matematica.aleatorio_rango", argumentos, 1)?;
            if inicio > fin {
                return Err(error(
                    "E0406",
                    "el inicio de 'aleatorio_rango' debe ser menor o igual que el fin",
                ));
            }
            Ok(Valor::Entero(rand::rng().random_range(inicio..=fin)))
        }),
    );
}

/// Registra una operación binaria exacta sobre decimales.
fn binaria_exacta(
    registro: &mut RegistroNativos,
    nombre: &'static str,
    operacion: fn(Decimal, Decimal) -> Option<Decimal>,
) {
    registro.registrar_funcion(
        nombre,
        Box::new(move |argumentos| {
            exigir_aridad(nombre, argumentos, 2)?;
            let a = arg_decimal(nombre, argumentos, 0)?;
            let b = arg_decimal(nombre, argumentos, 1)?;
            operacion(a, b)
                .map(Valor::Numero)
                .ok_or_else(|| error("E0402", format!("desbordamiento en '{nombre}'")))
        }),
    );
}

/// Registra una función unaria que opera en `f64`.
fn flotante(registro: &mut RegistroNativos, nombre: &'static str, operacion: fn(f64) -> f64) {
    registro.registrar_funcion(
        nombre,
        Box::new(move |argumentos| {
            exigir_aridad(nombre, argumentos, 1)?;
            let valor = decimal_a_f64(nombre, arg_decimal(nombre, argumentos, 0)?)?;
            f64_a_decimal(nombre, operacion(valor))
        }),
    );
}

/// Función binaria que opera en `f64`.
fn flotante_binaria(
    nombre: &str,
    argumentos: &[Valor],
    operacion: fn(f64, f64) -> f64,
) -> Result<Valor, Fallo> {
    exigir_aridad(nombre, argumentos, 2)?;
    let a = decimal_a_f64(nombre, arg_decimal(nombre, argumentos, 0)?)?;
    let b = decimal_a_f64(nombre, arg_decimal(nombre, argumentos, 1)?)?;
    f64_a_decimal(nombre, operacion(a, b))
}

/// Aplica una operación estadística sobre una lista de números.
fn estadistica(
    nombre: &str,
    argumentos: &[Valor],
    operacion: impl Fn(&[Decimal]) -> Option<Decimal>,
) -> Result<Valor, Fallo> {
    exigir_aridad(nombre, argumentos, 1)?;
    let lista = arg_lista(nombre, argumentos, 0)?;
    let elementos = lista.borrow();
    let mut valores = Vec::with_capacity(elementos.len());
    for elemento in elementos.iter() {
        valores.push(arg_decimal(nombre, std::slice::from_ref(elemento), 0)?);
    }
    operacion(&valores).map(Valor::Numero).ok_or_else(|| {
        error(
            "E0406",
            format!("'{nombre}' no se pudo calcular sobre la lista"),
        )
    })
}
