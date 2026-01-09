use crate::errores::{Error, CodigoError, Resultado};
use crate::interprete::entorno::Entorno;
use crate::interprete::valores::Valor;
use crate::nativos::interfaz::ModuloNativo;
use rust_decimal::Decimal;
use std::f64::consts::{PI, E};

/// Módulo nativo de matemática
pub struct Matematica;

impl Matematica {
    pub fn nuevo() -> Self {
        Self
    }
}

impl ModuloNativo for Matematica {
    fn nombre(&self) -> &str {
        "matemática"
    }
    
    fn ruta(&self) -> &str {
        "quetzal/matemática"
    }

    fn obtener_tipo_constante(&self, nombre: &str) -> Option<crate::nucleo::semantico::tipos::Tipo> {
        use crate::nucleo::semantico::tipos::Tipo;
        match nombre {
            "PI" | "E" | "TAU" => Some(Tipo::Numero),
            _ => None,
        }
    }
    
    fn registrar(&self, entorno: &mut Entorno) -> Resultado<()> {
        // Registrar constantes
        entorno.definir_variable("PI".to_string(), Valor::Numero(Decimal::from_f64_retain(PI).unwrap()), false)
            .map_err(|e| Error::ejecucion(CodigoError::ErrorInternoInterprete, e, None, None, None))?;
        entorno.definir_variable("E".to_string(), Valor::Numero(Decimal::from_f64_retain(E).unwrap()), false)
            .map_err(|e| Error::ejecucion(CodigoError::ErrorInternoInterprete, e, None, None, None))?;
        entorno.definir_variable("TAU".to_string(), Valor::Numero(Decimal::from_f64_retain(2.0 * PI).unwrap()), false)
            .map_err(|e| Error::ejecucion(CodigoError::ErrorInternoInterprete, e, None, None, None))?;
        Ok(())
    }
    
    fn obtener_constante(&self, nombre: &str) -> Option<Valor> {
        match nombre {
            "PI" => Some(Valor::Numero(Decimal::from_f64_retain(PI).unwrap())),
            "E" => Some(Valor::Numero(Decimal::from_f64_retain(E).unwrap())),
            "TAU" => Some(Valor::Numero(Decimal::from_f64_retain(2.0 * PI).unwrap())),
            _ => None,
        }
    }
    
    fn llamar_funcion(
        &self,
        nombre: &str,
        argumentos: Vec<Valor>,
        _entorno: &Entorno,
    ) -> Resultado<Valor> {
        match nombre {
            "sumar" => {
                if argumentos.len() != 2 {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "sumar requiere 2 argumentos",
                        None,
                        None,
                        None,
                    ));
                }
                let a = obtener_numero(&argumentos[0])?;
                let b = obtener_numero(&argumentos[1])?;
                Ok(Valor::Numero(a + b))
            }
            "restar" => {
                if argumentos.len() != 2 {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "restar requiere 2 argumentos",
                        None,
                        None,
                        None,
                    ));
                }
                let a = obtener_numero(&argumentos[0])?;
                let b = obtener_numero(&argumentos[1])?;
                Ok(Valor::Numero(a - b))
            }
            "multiplicar" => {
                if argumentos.len() != 2 {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "multiplicar requiere 2 argumentos",
                        None,
                        None,
                        None,
                    ));
                }
                let a = obtener_numero(&argumentos[0])?;
                let b = obtener_numero(&argumentos[1])?;
                Ok(Valor::Numero(a * b))
            }
            "dividir" => {
                if argumentos.len() != 2 {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "dividir requiere 2 argumentos",
                        None,
                        None,
                        None,
                    ));
                }
                let a = obtener_numero(&argumentos[0])?;
                let b = obtener_numero(&argumentos[1])?;
                if b == Decimal::ZERO {
                    return Err(Error::ejecucion(
                        CodigoError::DivisionPorCero,
                        "división por cero",
                        None,
                        None,
                        None,
                    ));
                }
                Ok(Valor::Numero(a / b))
            }
            "potencia" => {
                if argumentos.len() != 2 {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "potencia requiere 2 argumentos",
                        None,
                        None,
                        None,
                    ));
                }
                let base = obtener_numero(&argumentos[0])?;
                let exponente = obtener_numero(&argumentos[1])?;
                // Convertir Decimal a f64 para operaciones matemáticas
                let base_f64: f64 = base.to_string().parse().unwrap_or(0.0);
                let exp_f64: f64 = exponente.to_string().parse().unwrap_or(0.0);
                let resultado = base_f64.powf(exp_f64);
                Ok(Valor::Numero(Decimal::from_f64_retain(resultado).unwrap()))
            }
            "seno" => {
                if argumentos.len() != 1 {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "seno requiere 1 argumento",
                        None,
                        None,
                        None,
                    ));
                }
                let angulo = obtener_numero(&argumentos[0])?;
                // Convertir Decimal a f64 para operaciones matemáticas
                let angulo_f64: f64 = angulo.to_string().parse().unwrap_or(0.0);
                let resultado = angulo_f64.sin();
                Ok(Valor::Numero(Decimal::from_f64_retain(resultado).unwrap()))
            }
            "promedio" => {
                if argumentos.is_empty() {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "promedio requiere al menos 1 argumento",
                        None,
                        None,
                        None,
                    ));
                }
                if let Some(Valor::Lista(lista)) = argumentos.get(0) {
                    let suma: Decimal = lista.iter()
                        .map(|v| obtener_numero(v).unwrap_or(Decimal::ZERO))
                        .sum();
                    let cantidad = Decimal::from(lista.len());
                    Ok(Valor::Numero(suma / cantidad))
                } else {
                    Err(Error::ejecucion(
                        CodigoError::TipoArgumentoIncorrecto,
                        "promedio requiere una lista de números",
                        None,
                        None,
                        None,
                    ))
                }
            }
            "coseno" => {
                if argumentos.len() != 1 {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "coseno requiere 1 argumento",
                        None,
                        None,
                        None,
                    ));
                }
                let angulo = obtener_numero(&argumentos[0])?;
                let angulo_f64: f64 = angulo.to_string().parse().unwrap_or(0.0);
                let resultado = angulo_f64.cos();
                Ok(Valor::Numero(Decimal::from_f64_retain(resultado).unwrap()))
            }
            "tangente" => {
                if argumentos.len() != 1 {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "tangente requiere 1 argumento",
                        None,
                        None,
                        None,
                    ));
                }
                let angulo = obtener_numero(&argumentos[0])?;
                let angulo_f64: f64 = angulo.to_string().parse().unwrap_or(0.0);
                let resultado = angulo_f64.tan();
                Ok(Valor::Numero(Decimal::from_f64_retain(resultado).unwrap()))
            }
            "grados_a_radianes" => {
                if argumentos.len() != 1 {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "grados_a_radianes requiere 1 argumento",
                        None,
                        None,
                        None,
                    ));
                }
                let grados = obtener_numero(&argumentos[0])?;
                let grados_f64: f64 = grados.to_string().parse().unwrap_or(0.0);
                let radianes = grados_f64 * PI / 180.0;
                Ok(Valor::Numero(Decimal::from_f64_retain(radianes).unwrap()))
            }
            "radianes_a_grados" => {
                if argumentos.len() != 1 {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "radianes_a_grados requiere 1 argumento",
                        None,
                        None,
                        None,
                    ));
                }
                let radianes = obtener_numero(&argumentos[0])?;
                let radianes_f64: f64 = radianes.to_string().parse().unwrap_or(0.0);
                let grados = radianes_f64 * 180.0 / PI;
                Ok(Valor::Numero(Decimal::from_f64_retain(grados).unwrap()))
            }
            "hipotenusa" => {
                if argumentos.len() != 2 {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "hipotenusa requiere 2 argumentos",
                        None,
                        None,
                        None,
                    ));
                }
                let a = obtener_numero(&argumentos[0])?;
                let b = obtener_numero(&argumentos[1])?;
                let a_f64: f64 = a.to_string().parse().unwrap_or(0.0);
                let b_f64: f64 = b.to_string().parse().unwrap_or(0.0);
                let resultado = (a_f64 * a_f64 + b_f64 * b_f64).sqrt();
                Ok(Valor::Numero(Decimal::from_f64_retain(resultado).unwrap()))
            }
            "logaritmo" | "logaritmo_natural" => {
                if argumentos.len() != 1 {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "logaritmo requiere 1 argumento",
                        None,
                        None,
                        None,
                    ));
                }
                let valor = obtener_numero(&argumentos[0])?;
                let valor_f64: f64 = valor.to_string().parse().unwrap_or(0.0);
                if valor_f64 <= 0.0 {
                    return Err(Error::ejecucion(
                        CodigoError::OperacionNoSoportada,
                        "logaritmo de un número no positivo",
                        None,
                        None,
                        None,
                    ));
                }
                let resultado = valor_f64.ln();
                Ok(Valor::Numero(Decimal::from_f64_retain(resultado).unwrap()))
            }
            "logaritmo_base" => {
                if argumentos.len() != 2 {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "logaritmo_base requiere 2 argumentos (valor, base)",
                        None,
                        None,
                        None,
                    ));
                }
                let valor = obtener_numero(&argumentos[0])?;
                let base = obtener_numero(&argumentos[1])?;
                let valor_f64: f64 = valor.to_string().parse().unwrap_or(0.0);
                let base_f64: f64 = base.to_string().parse().unwrap_or(0.0);
                if valor_f64 <= 0.0 || base_f64 <= 0.0 || base_f64 == 1.0 {
                    return Err(Error::ejecucion(
                        CodigoError::OperacionNoSoportada,
                        "logaritmo de un número no positivo o base inválida",
                        None,
                        None,
                        None,
                    ));
                }
                let resultado = valor_f64.log(base_f64);
                Ok(Valor::Numero(Decimal::from_f64_retain(resultado).unwrap()))
            }
            "exponencial" => {
                if argumentos.len() != 1 {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "exponencial requiere 1 argumento",
                        None,
                        None,
                        None,
                    ));
                }
                let exponente = obtener_numero(&argumentos[0])?;
                let exp_f64: f64 = exponente.to_string().parse().unwrap_or(0.0);
                let resultado = exp_f64.exp();
                Ok(Valor::Numero(Decimal::from_f64_retain(resultado).unwrap()))
            }
            "raiz_cuadrada" | "raiz" => {
                if argumentos.len() != 1 {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "raiz_cuadrada requiere 1 argumento",
                        None,
                        None,
                        None,
                    ));
                }
                let valor = obtener_numero(&argumentos[0])?;
                let valor_f64: f64 = valor.to_string().parse().unwrap_or(0.0);
                if valor_f64 < 0.0 {
                    return Err(Error::ejecucion(
                        CodigoError::OperacionNoSoportada,
                        "raíz cuadrada de un número negativo",
                        None,
                        None,
                        None,
                    ));
                }
                let resultado = valor_f64.sqrt();
                Ok(Valor::Numero(Decimal::from_f64_retain(resultado).unwrap()))
            }
            "absoluto" | "abs" => {
                if argumentos.len() != 1 {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "absoluto requiere 1 argumento",
                        None,
                        None,
                        None,
                    ));
                }
                let valor = obtener_numero(&argumentos[0])?;
                Ok(Valor::Numero(valor.abs()))
            }
            "redondear" => {
                if argumentos.len() < 1 || argumentos.len() > 2 {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "redondear requiere 1 o 2 argumentos (valor, [decimales])",
                        None,
                        None,
                        None,
                    ));
                }
                let valor = obtener_numero(&argumentos[0])?;
                let decimales = if argumentos.len() == 2 {
                    match &argumentos[1] {
                        Valor::Entero(d) => *d as u32,
                        Valor::Numero(d) => d.to_string().parse::<u32>().unwrap_or(0),
                        _ => 0,
                    }
                } else {
                    0
                };
                let resultado = valor.round_dp(decimales);
                Ok(Valor::Numero(resultado))
            }
            "piso" | "floor" => {
                if argumentos.len() != 1 {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "piso requiere 1 argumento",
                        None,
                        None,
                        None,
                    ));
                }
                let valor = obtener_numero(&argumentos[0])?;
                Ok(Valor::Numero(valor.floor()))
            }
            "techo" | "ceil" => {
                if argumentos.len() != 1 {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "techo requiere 1 argumento",
                        None,
                        None,
                        None,
                    ));
                }
                let valor = obtener_numero(&argumentos[0])?;
                Ok(Valor::Numero(valor.ceil()))
            }
            "maximo" => {
                if argumentos.is_empty() {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "maximo requiere al menos 1 argumento",
                        None,
                        None,
                        None,
                    ));
                }
                if let Some(Valor::Lista(lista)) = argumentos.get(0) {
                    if lista.is_empty() {
                        return Err(Error::ejecucion(
                            CodigoError::OperacionNoSoportada,
                            "maximo de una lista vacía",
                            None,
                            None,
                            None,
                        ));
                    }
                    let maximo = lista.iter()
                        .filter_map(|v| obtener_numero(v).ok())
                        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                        .unwrap_or(Decimal::ZERO);
                    Ok(Valor::Numero(maximo))
                } else {
                    // Si son varios argumentos individuales
                    let maximo = argumentos.iter()
                        .filter_map(|v| obtener_numero(v).ok())
                        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                        .unwrap_or(Decimal::ZERO);
                    Ok(Valor::Numero(maximo))
                }
            }
            "minimo" => {
                if argumentos.is_empty() {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "minimo requiere al menos 1 argumento",
                        None,
                        None,
                        None,
                    ));
                }
                if let Some(Valor::Lista(lista)) = argumentos.get(0) {
                    if lista.is_empty() {
                        return Err(Error::ejecucion(
                            CodigoError::OperacionNoSoportada,
                            "minimo de una lista vacía",
                            None,
                            None,
                            None,
                        ));
                    }
                    let minimo = lista.iter()
                        .filter_map(|v| obtener_numero(v).ok())
                        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                        .unwrap_or(Decimal::ZERO);
                    Ok(Valor::Numero(minimo))
                } else {
                    // Si son varios argumentos individuales
                    let minimo = argumentos.iter()
                        .filter_map(|v| obtener_numero(v).ok())
                        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                        .unwrap_or(Decimal::ZERO);
                    Ok(Valor::Numero(minimo))
                }
            }
            "suma_total" => {
                if argumentos.is_empty() {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "suma_total requiere al menos 1 argumento",
                        None,
                        None,
                        None,
                    ));
                }
                if let Some(Valor::Lista(lista)) = argumentos.get(0) {
                    let suma: Decimal = lista.iter()
                        .filter_map(|v| obtener_numero(v).ok())
                        .sum();
                    Ok(Valor::Numero(suma))
                } else {
                    let suma: Decimal = argumentos.iter()
                        .filter_map(|v| obtener_numero(v).ok())
                        .sum();
                    Ok(Valor::Numero(suma))
                }
            }
            "producto_total" => {
                if argumentos.is_empty() {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "producto_total requiere al menos 1 argumento",
                        None,
                        None,
                        None,
                    ));
                }
                if let Some(Valor::Lista(lista)) = argumentos.get(0) {
                    let producto: Decimal = lista.iter()
                        .filter_map(|v| obtener_numero(v).ok())
                        .fold(Decimal::ONE, |acc, x| acc * x);
                    Ok(Valor::Numero(producto))
                } else {
                    let producto: Decimal = argumentos.iter()
                        .filter_map(|v| obtener_numero(v).ok())
                        .fold(Decimal::ONE, |acc, x| acc * x);
                    Ok(Valor::Numero(producto))
                }
            }
            "aleatorio" => {
                // Genera un número aleatorio entre 0 y 1
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos();
                let aleatorio = (seed as f64 / u32::MAX as f64).abs();
                Ok(Valor::Numero(Decimal::from_f64_retain(aleatorio).unwrap()))
            }
            "aleatorio_rango" => {
                if argumentos.len() != 2 {
                    return Err(Error::ejecucion(
                        CodigoError::NumeroArgumentosIncorrecto,
                        "aleatorio_rango requiere 2 argumentos (min, max)",
                        None,
                        None,
                        None,
                    ));
                }
                let min = obtener_numero(&argumentos[0])?;
                let max = obtener_numero(&argumentos[1])?;
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos();
                let aleatorio = (seed as f64 / u32::MAX as f64).abs();
                let rango = max - min;
                let resultado = min + rango * Decimal::from_f64_retain(aleatorio).unwrap();
                Ok(Valor::Numero(resultado))
            }
            _ => Err(Error::ejecucion(
                CodigoError::FuncionNoDeclarada,
                format!("función '{}' no encontrada en módulo matemática", nombre),
                None,
                None,
                None,
            )),
        }
    }
}

fn obtener_numero(valor: &Valor) -> Result<Decimal, Error> {
    match valor {
        Valor::Entero(val) => Ok(Decimal::from(*val)),
        Valor::Numero(val) => Ok(*val),
        _ => Err(Error::ejecucion(
            CodigoError::TipoArgumentoIncorrecto,
            format!("se esperaba un número, se obtuvo {}", valor.tipo()),
            None,
            None,
            None,
        )),
    }
}
