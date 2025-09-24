// Módulo nativo de matemáticas para Quetzal
// Proporciona constantes y un objeto utilitario con operaciones matemáticas

use std::collections::HashMap;
use std::f64::consts::{E, PI, TAU};

use crate::datos::tipos_datos::{TipoVariable, Valor, Variable};
use crate::ejecucion::evaluador::{Evaluador, MetodoObjetoNativo, ResultadoMetodoObjetoNativo};
use crate::infraestructura::errores::{ErrorQuetzal, ResultadoQuetzal};
use crate::infraestructura::manejador_modulos::ElementoExportado;

/// Nombre del objeto expuesto por el módulo de matemáticas
const NOMBRE_OBJETO_MATEMATICA: &str = "Matemática";

/// Lista de métodos disponibles en el objeto Matemática
const METODOS_MATEMATICA: &[&str] = &[
    "sumar",
    "restar",
    "multiplicar",
    "dividir",
    "modulo",
    "potencia",
    "raiz",
    "raiz_cuadrada",
    "valor_absoluto",
    "redondear",
    "piso",
    "techo",
    "truncar",
    "signo",
    "seno",
    "coseno",
    "tangente",
    "arcoseno",
    "arcocoseno",
    "arcotangente",
    "hipotenusa",
    "logaritmo",
    "logaritmo_base",
    "logaritmo10",
    "exponencial",
    "maximo",
    "minimo",
    "promedio",
    "suma_total",
    "producto_total",
    "grados_a_radianes",
    "radianes_a_grados",
];

/// Registra todas las exportaciones ofrecidas por el módulo nativo
pub fn registrar_modulo(
    evaluador: &mut Evaluador,
) -> ResultadoQuetzal<HashMap<String, ElementoExportado>> {
    let mut exportaciones = HashMap::new();

    registrar_metodos_matematica(evaluador);

    let objeto_matematica = construir_objeto_matematica(evaluador);

    exportaciones.insert(
        NOMBRE_OBJETO_MATEMATICA.to_string(),
        ElementoExportado::Instancia(objeto_matematica),
    );

    Ok(exportaciones)
}

/// Registra las implementaciones nativas para cada método del objeto Matemática
fn registrar_metodos_matematica(evaluador: &mut Evaluador) {
    for (nombre, implementacion) in METODOS_DISPONIBLES.iter() {
        evaluador.registrar_metodo_objeto_nativo(NOMBRE_OBJETO_MATEMATICA, nombre, *implementacion);
    }
}

/// Listado de métodos disponibles junto con su implementación en Rust
const METODOS_DISPONIBLES: &[(&str, MetodoObjetoNativo)] = &[
    ("sumar", metodo_sumar),
    ("restar", metodo_restar),
    ("multiplicar", metodo_multiplicar),
    ("dividir", metodo_dividir),
    ("modulo", metodo_modulo),
    ("potencia", metodo_potencia),
    ("raiz", metodo_raiz),
    ("raiz_cuadrada", metodo_raiz_cuadrada),
    ("valor_absoluto", metodo_valor_absoluto),
    ("redondear", metodo_redondear),
    ("piso", metodo_piso),
    ("techo", metodo_techo),
    ("truncar", metodo_truncar),
    ("signo", metodo_signo),
    ("seno", metodo_seno),
    ("coseno", metodo_coseno),
    ("tangente", metodo_tangente),
    ("arcoseno", metodo_arcoseno),
    ("arcocoseno", metodo_arcocoseno),
    ("arcotangente", metodo_arcotangente),
    ("hipotenusa", metodo_hipotenusa),
    ("logaritmo", metodo_logaritmo),
    ("logaritmo_base", metodo_logaritmo_base),
    ("logaritmo10", metodo_logaritmo10),
    ("exponencial", metodo_exponencial),
    ("maximo", metodo_maximo),
    ("minimo", metodo_minimo),
    ("promedio", metodo_promedio),
    ("suma_total", metodo_suma_total),
    ("producto_total", metodo_producto_total),
    ("grados_a_radianes", metodo_grados_a_radianes),
    ("radianes_a_grados", metodo_radianes_a_grados),
];

fn metodo_sumar(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 2, linea)?;
    let a = extraer_numero(argumentos, 0, linea, "primer argumento")?;
    let b = extraer_numero(argumentos, 1, linea, "segundo argumento")?;
    Ok(respuesta_numero(evaluador, a + b))
}

fn metodo_restar(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 2, linea)?;
    let minuendo = extraer_numero(argumentos, 0, linea, "minuendo")?;
    let sustraendo = extraer_numero(argumentos, 1, linea, "sustraendo")?;
    Ok(respuesta_numero(evaluador, minuendo - sustraendo))
}

fn metodo_multiplicar(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 2, linea)?;
    let a = extraer_numero(argumentos, 0, linea, "primer argumento")?;
    let b = extraer_numero(argumentos, 1, linea, "segundo argumento")?;
    Ok(respuesta_numero(evaluador, a * b))
}

fn metodo_dividir(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 2, linea)?;
    let dividendo = extraer_numero(argumentos, 0, linea, "dividendo")?;
    let divisor = extraer_numero(argumentos, 1, linea, "divisor")?;

    if divisor.abs() < f64::EPSILON {
        return Err(ErrorQuetzal::DivisionPorCero { linea });
    }

    Ok(respuesta_numero(evaluador, dividendo / divisor))
}

fn metodo_modulo(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 2, linea)?;
    let dividendo = extraer_numero(argumentos, 0, linea, "dividendo")?;
    let divisor = extraer_numero(argumentos, 1, linea, "divisor")?;

    if divisor.abs() < f64::EPSILON {
        return Err(ErrorQuetzal::DivisionPorCero { linea });
    }

    Ok(respuesta_numero(evaluador, dividendo % divisor))
}

fn metodo_potencia(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 2, linea)?;
    let base = extraer_numero(argumentos, 0, linea, "base")?;
    let exponente = extraer_numero(argumentos, 1, linea, "exponente")?;
    let resultado = base.powf(exponente);

    if !resultado.is_finite() {
        return Err(ErrorQuetzal::ErrorEjecucion {
            linea,
            mensaje: "El resultado de 'potencia' no es un número finito".to_string(),
        });
    }

    Ok(respuesta_numero(evaluador, resultado))
}

fn metodo_raiz(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 2, linea)?;
    let valor = extraer_numero(argumentos, 0, linea, "valor")?;

    if valor < 0.0 {
        return Err(ErrorQuetzal::ErrorEjecucion {
            linea,
            mensaje: "No se puede calcular la raíz de un número negativo".to_string(),
        });
    }

    let indice = extraer_entero(argumentos, 1, linea, "índice de la raíz")?;

    if indice == 0 {
        return Err(ErrorQuetzal::ErrorEjecucion {
            linea,
            mensaje: "El índice de la raíz no puede ser cero".to_string(),
        });
    }

    if valor == 0.0 {
        return Ok(respuesta_numero(evaluador, 0.0));
    }

    if indice < 0 {
        return Err(ErrorQuetzal::ErrorEjecucion {
            linea,
            mensaje: "El índice de la raíz debe ser positivo".to_string(),
        });
    }

    let indice_f64 = indice as f64;
    let resultado = valor.powf(1.0 / indice_f64);

    if resultado.is_nan() || !resultado.is_finite() {
        return Err(ErrorQuetzal::ErrorEjecucion {
            linea,
            mensaje: "El resultado de la raíz no es un número válido".to_string(),
        });
    }

    Ok(respuesta_numero(evaluador, resultado))
}

fn metodo_raiz_cuadrada(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 1, linea)?;
    let valor = extraer_numero(argumentos, 0, linea, "valor")?;

    if valor < 0.0 {
        return Err(ErrorQuetzal::ErrorEjecucion {
            linea,
            mensaje: "No se puede calcular la raíz cuadrada de un número negativo".to_string(),
        });
    }

    Ok(respuesta_numero(evaluador, valor.sqrt()))
}

fn metodo_valor_absoluto(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 1, linea)?;
    let valor = extraer_numero(argumentos, 0, linea, "valor")?;
    Ok(respuesta_numero(evaluador, valor.abs()))
}

fn metodo_redondear(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    if argumentos.is_empty() || argumentos.len() > 2 {
        return Err(ErrorQuetzal::ErrorEjecucion {
            linea,
            mensaje: "El método 'redondear' acepta uno o dos argumentos".to_string(),
        });
    }

    let valor = extraer_numero(argumentos, 0, linea, "valor")?;

    let decimales = if argumentos.len() == 2 {
        extraer_entero(argumentos, 1, linea, "decimales")?
    } else {
        0
    };

    if decimales < 0 {
        return Err(ErrorQuetzal::ErrorEjecucion {
            linea,
            mensaje: "El número de decimales no puede ser negativo".to_string(),
        });
    }

    let factor = 10_f64.powi(decimales as i32);
    let resultado = (valor * factor).round() / factor;
    Ok(respuesta_numero(evaluador, resultado))
}

fn metodo_piso(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 1, linea)?;
    let valor = extraer_numero(argumentos, 0, linea, "valor")?;
    Ok(respuesta_numero(evaluador, valor.floor()))
}

fn metodo_techo(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 1, linea)?;
    let valor = extraer_numero(argumentos, 0, linea, "valor")?;
    Ok(respuesta_numero(evaluador, valor.ceil()))
}

fn metodo_truncar(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 1, linea)?;
    let valor = extraer_numero(argumentos, 0, linea, "valor")?;
    Ok(respuesta_numero(evaluador, valor.trunc()))
}

fn metodo_signo(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 1, linea)?;
    let valor = extraer_numero(argumentos, 0, linea, "valor")?;
    let signo = if valor > 0.0 {
        1.0
    } else if valor < 0.0 {
        -1.0
    } else {
        0.0
    };
    Ok(respuesta_numero(evaluador, signo))
}

fn metodo_seno(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 1, linea)?;
    let radianes = extraer_numero(argumentos, 0, linea, "radianes")?;
    Ok(respuesta_numero(evaluador, radianes.sin()))
}

fn metodo_coseno(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 1, linea)?;
    let radianes = extraer_numero(argumentos, 0, linea, "radianes")?;
    Ok(respuesta_numero(evaluador, radianes.cos()))
}

fn metodo_tangente(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 1, linea)?;
    let radianes = extraer_numero(argumentos, 0, linea, "radianes")?;
    Ok(respuesta_numero(evaluador, radianes.tan()))
}

fn metodo_arcoseno(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 1, linea)?;
    let valor = extraer_numero(argumentos, 0, linea, "valor")?;
    if !(valor >= -1.0 && valor <= 1.0) {
        return Err(ErrorQuetzal::ErrorEjecucion {
            linea,
            mensaje: "El argumento de 'arcoseno' debe estar entre -1 y 1".to_string(),
        });
    }
    Ok(respuesta_numero(evaluador, valor.asin()))
}

fn metodo_arcocoseno(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 1, linea)?;
    let valor = extraer_numero(argumentos, 0, linea, "valor")?;
    if !(valor >= -1.0 && valor <= 1.0) {
        return Err(ErrorQuetzal::ErrorEjecucion {
            linea,
            mensaje: "El argumento de 'arcocoseno' debe estar entre -1 y 1".to_string(),
        });
    }
    Ok(respuesta_numero(evaluador, valor.acos()))
}

fn metodo_arcotangente(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 1, linea)?;
    let valor = extraer_numero(argumentos, 0, linea, "valor")?;
    Ok(respuesta_numero(evaluador, valor.atan()))
}

fn metodo_hipotenusa(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 2, linea)?;
    let cateto_a = extraer_numero(argumentos, 0, linea, "primer cateto")?;
    let cateto_b = extraer_numero(argumentos, 1, linea, "segundo cateto")?;
    Ok(respuesta_numero(evaluador, cateto_a.hypot(cateto_b)))
}

fn metodo_logaritmo(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 1, linea)?;
    let valor = extraer_numero(argumentos, 0, linea, "valor")?;

    if valor <= 0.0 {
        return Err(ErrorQuetzal::ErrorEjecucion {
            linea,
            mensaje: "El argumento de 'logaritmo' debe ser mayor que 0".to_string(),
        });
    }

    Ok(respuesta_numero(evaluador, valor.ln()))
}

fn metodo_logaritmo_base(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 2, linea)?;
    let valor = extraer_numero(argumentos, 0, linea, "valor")?;
    let base = extraer_numero(argumentos, 1, linea, "base")?;

    if valor <= 0.0 {
        return Err(ErrorQuetzal::ErrorEjecucion {
            linea,
            mensaje: "El argumento de 'logaritmo_base' debe ser mayor que 0".to_string(),
        });
    }

    if base <= 0.0 || (base - 1.0).abs() < f64::EPSILON {
        return Err(ErrorQuetzal::ErrorEjecucion {
            linea,
            mensaje: "La base del logaritmo debe ser positiva y distinta de 1".to_string(),
        });
    }

    Ok(respuesta_numero(evaluador, valor.log(base)))
}

fn metodo_logaritmo10(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 1, linea)?;
    let valor = extraer_numero(argumentos, 0, linea, "valor")?;

    if valor <= 0.0 {
        return Err(ErrorQuetzal::ErrorEjecucion {
            linea,
            mensaje: "El argumento de 'logaritmo10' debe ser mayor que 0".to_string(),
        });
    }

    Ok(respuesta_numero(evaluador, valor.log10()))
}

fn metodo_exponencial(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 1, linea)?;
    let valor = extraer_numero(argumentos, 0, linea, "valor")?;
    let resultado = valor.exp();

    if !resultado.is_finite() {
        return Err(ErrorQuetzal::ErrorEjecucion {
            linea,
            mensaje: "El resultado de 'exponencial' no es un número finito".to_string(),
        });
    }

    Ok(respuesta_numero(evaluador, resultado))
}

fn metodo_maximo(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 1, linea)?;
    let numeros = extraer_lista_numeros(argumentos, 0, linea, "maximo")?;
    let maximo =
        numeros
            .into_iter()
            .reduce(f64::max)
            .ok_or_else(|| ErrorQuetzal::ErrorEjecucion {
                linea,
                mensaje: "No se puede calcular el máximo de una lista vacía".to_string(),
            })?;
    Ok(respuesta_numero(evaluador, maximo))
}

fn metodo_minimo(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 1, linea)?;
    let numeros = extraer_lista_numeros(argumentos, 0, linea, "minimo")?;
    let minimo =
        numeros
            .into_iter()
            .reduce(f64::min)
            .ok_or_else(|| ErrorQuetzal::ErrorEjecucion {
                linea,
                mensaje: "No se puede calcular el mínimo de una lista vacía".to_string(),
            })?;
    Ok(respuesta_numero(evaluador, minimo))
}

fn metodo_promedio(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 1, linea)?;
    let numeros = extraer_lista_numeros(argumentos, 0, linea, "promedio")?;
    let suma: f64 = numeros.iter().sum();
    Ok(respuesta_numero(evaluador, suma / numeros.len() as f64))
}

fn metodo_suma_total(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 1, linea)?;
    let numeros = extraer_lista_numeros(argumentos, 0, linea, "suma_total")?;
    let suma: f64 = numeros.iter().sum();
    Ok(respuesta_numero(evaluador, suma))
}

fn metodo_producto_total(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 1, linea)?;
    let numeros = extraer_lista_numeros(argumentos, 0, linea, "producto_total")?;
    let producto = numeros.into_iter().product();
    Ok(respuesta_numero(evaluador, producto))
}

fn metodo_grados_a_radianes(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 1, linea)?;
    let grados = extraer_numero(argumentos, 0, linea, "grados")?;
    Ok(respuesta_numero(evaluador, grados * PI / 180.0))
}

fn metodo_radianes_a_grados(
    evaluador: &mut Evaluador,
    _objeto: &Valor,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<ResultadoMetodoObjetoNativo> {
    verificar_argumentos_exactos(argumentos, 1, linea)?;
    let radianes = extraer_numero(argumentos, 0, linea, "radianes")?;
    Ok(respuesta_numero(evaluador, radianes * 180.0 / PI))
}

fn verificar_argumentos_exactos(
    argumentos: &[Valor],
    esperados: usize,
    linea: usize,
) -> ResultadoQuetzal<()> {
    if argumentos.len() != esperados {
        return Err(ErrorQuetzal::ArgumentosIncorrectos {
            linea,
            esperados,
            recibidos: argumentos.len(),
        });
    }
    Ok(())
}

fn extraer_numero(
    argumentos: &[Valor],
    indice: usize,
    linea: usize,
    nombre_argumento: &str,
) -> ResultadoQuetzal<f64> {
    match argumentos.get(indice) {
        Some(Valor::Entero(entero)) => Ok(*entero as f64),
        Some(Valor::Numero(numero)) => Ok(*numero),
        _ => Err(ErrorQuetzal::ErrorTipo {
            linea,
            mensaje: format!(
                "El argumento '{}' debe ser numérico para el módulo Matemática",
                nombre_argumento
            ),
        }),
    }
}

fn extraer_entero(
    argumentos: &[Valor],
    indice: usize,
    linea: usize,
    nombre_argumento: &str,
) -> ResultadoQuetzal<i64> {
    match argumentos.get(indice) {
        Some(Valor::Entero(entero)) => Ok(*entero),
        Some(Valor::Numero(numero)) => {
            let aproximado = redondear_interno(*numero);
            if (aproximado.fract()).abs() < f64::EPSILON {
                Ok(aproximado as i64)
            } else {
                Err(ErrorQuetzal::ErrorTipo {
                    linea,
                    mensaje: format!(
                        "El argumento '{}' debe ser un entero para el módulo Matemática",
                        nombre_argumento
                    ),
                })
            }
        }
        _ => Err(ErrorQuetzal::ErrorTipo {
            linea,
            mensaje: format!(
                "El argumento '{}' debe ser un entero para el módulo Matemática",
                nombre_argumento
            ),
        }),
    }
}

fn extraer_lista_numeros(
    argumentos: &[Valor],
    indice: usize,
    linea: usize,
    metodo: &str,
) -> ResultadoQuetzal<Vec<f64>> {
    match argumentos.get(indice) {
        Some(Valor::Lista(elementos)) => {
            if elementos.is_empty() {
                return Err(ErrorQuetzal::ErrorEjecucion {
                    linea,
                    mensaje: format!(
                        "El método '{}' requiere una lista con al menos un número",
                        metodo
                    ),
                });
            }

            let mut numeros = Vec::with_capacity(elementos.len());
            for (posicion, valor) in elementos.iter().enumerate() {
                match valor {
                    Valor::Entero(entero) => numeros.push(*entero as f64),
                    Valor::Numero(numero) => numeros.push(*numero),
                    _ => {
                        return Err(ErrorQuetzal::ErrorTipo {
                            linea,
                            mensaje: format!(
                                "El argumento 'elemento {}' debe ser numérico para el módulo Matemática",
                                posicion + 1
                            ),
                        });
                    }
                }
            }
            Ok(numeros)
        }
        _ => Err(ErrorQuetzal::ErrorTipo {
            linea,
            mensaje: format!("El método '{}' requiere una lista de números", metodo),
        }),
    }
}

fn respuesta_numero(evaluador: &Evaluador, numero: f64) -> ResultadoMetodoObjetoNativo {
    ResultadoMetodoObjetoNativo::sin_cambios(Valor::Numero(evaluador.redondear_numero(numero)))
}

fn redondear_interno(valor: f64) -> f64 {
    const FACTOR: f64 = 1e15;
    (valor * FACTOR).round() / FACTOR
}

/// Construye el objeto Matemática con sus constantes y metadatos
fn construir_objeto_matematica(evaluador: &Evaluador) -> Variable {
    let mut propiedades = HashMap::new();
    let mut propiedades_publicas = Vec::new();

    // Constantes matemáticas comunes expuestas como propiedades públicas
    let constantes = [
        ("PI", evaluador.redondear_numero(PI)),
        ("TAU", evaluador.redondear_numero(TAU)),
        ("E", evaluador.redondear_numero(E)),
    ];

    for (nombre, valor) in constantes {
        propiedades_publicas.push(nombre.to_string());
        propiedades.insert(nombre.to_string(), Valor::Numero(valor));
    }

    // Marcar internamente el tipo de objeto para facilitar su identificación
    propiedades.insert(
        "__nombre".to_string(),
        Valor::Texto(NOMBRE_OBJETO_MATEMATICA.to_string()),
    );

    let metodos_publicos = METODOS_MATEMATICA
        .iter()
        .map(|nombre| nombre.to_string())
        .collect();

    let valor_objeto = Valor::Objeto {
        clase: NOMBRE_OBJETO_MATEMATICA.to_string(),
        propiedades,
        propiedades_publicas,
        metodos_publicos,
    };

    Variable::nueva(
        NOMBRE_OBJETO_MATEMATICA.to_string(),
        valor_objeto,
        TipoVariable::Inmutable,
        "objeto".to_string(),
    )
}
