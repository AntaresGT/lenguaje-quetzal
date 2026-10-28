use std::collections::VecDeque;

use bytecode::{CodigoOp, Constante, FuncionCompilada, ModuloCompilado};

use crate::ErrorJit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TipoAbstracto {
    Entero,
    Log,
    Nulo,
    Desconocido,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Estado {
    locales: Vec<TipoAbstracto>,
    pila: Vec<TipoAbstracto>,
}

pub(crate) struct Analisis {
    pub profundidades: Vec<Option<u16>>,
    pub max_pila: u16,
}

pub(crate) fn analizar(
    modulo: &ModuloCompilado,
    funcion: &FuncionCompilada,
) -> Result<Analisis, ErrorJit> {
    if funcion.asincrona {
        return Err(ErrorJit::NoElegible(
            "las funciones asincronas quedan en el intérprete".to_string(),
        ));
    }
    let n = funcion.trozo.instrucciones.len();
    if n == 0 {
        return Err(ErrorJit::NoElegible("la función está vacía".to_string()));
    }
    let mut locales = vec![TipoAbstracto::Desconocido; funcion.ranuras_locales as usize];
    for local in locales.iter_mut().take(funcion.parametros.len()) {
        *local = TipoAbstracto::Entero;
    }
    let inicial = Estado {
        locales,
        pila: Vec::new(),
    };
    let mut estados = vec![None; n];
    let mut trabajo = VecDeque::new();
    encolar(0, inicial, n, &mut estados, &mut trabajo)?;
    let mut max_pila = 0usize;
    let mut retornos = 0usize;

    while let Some(pc) = trabajo.pop_front() {
        if pc == n {
            return Err(fallo(pc, "hay un camino que sale sin retornar un entero"));
        }
        let mut estado = estados[pc].clone().expect("estado encolado");
        max_pila = max_pila.max(estado.pila.len());
        let instruccion = funcion.trozo.instrucciones[pc];
        let mut siguientes = Vec::with_capacity(2);
        match instruccion.op {
            CodigoOp::CargarEntero => estado.pila.push(TipoAbstracto::Entero),
            CodigoOp::CargarConstante => {
                let tipo = match modulo.constantes.get(instruccion.indice_u32() as usize) {
                    Some(Constante::Entero(_)) => TipoAbstracto::Entero,
                    Some(Constante::Log(_)) => TipoAbstracto::Log,
                    Some(_) => {
                        return Err(fallo(pc, "la constante no es entera ni lógica"));
                    }
                    None => return Err(fallo(pc, "constante fuera de rango")),
                };
                estado.pila.push(tipo);
            }
            CodigoOp::CargarLocal => {
                let tipo = estado.locales[instruccion.ranura() as usize];
                if matches!(tipo, TipoAbstracto::Nulo | TipoAbstracto::Desconocido) {
                    return Err(fallo(pc, "no se conoce el tipo del local"));
                }
                estado.pila.push(tipo);
            }
            CodigoOp::GuardarLocal => {
                let tipo = sacar(&mut estado, pc)?;
                estado.locales[instruccion.ranura() as usize] = tipo;
            }
            CodigoOp::LimpiarLocales => {
                for local in estado
                    .locales
                    .iter_mut()
                    .skip(instruccion.ranura() as usize)
                {
                    *local = TipoAbstracto::Nulo;
                }
            }
            CodigoOp::AjustarLocal => {
                exigir_local(&estado, instruccion.ranura(), TipoAbstracto::Entero, pc)?;
            }
            CodigoOp::AjustarLocalConLocal => {
                exigir_local(&estado, instruccion.ranura(), TipoAbstracto::Entero, pc)?;
                exigir_local(
                    &estado,
                    instruccion.ranura_derecha(),
                    TipoAbstracto::Entero,
                    pc,
                )?;
            }
            CodigoOp::CargarLocalLocalSumarEntero => {
                exigir_local(&estado, instruccion.ranura(), TipoAbstracto::Entero, pc)?;
                exigir_local(
                    &estado,
                    instruccion.ranura_derecha(),
                    TipoAbstracto::Entero,
                    pc,
                )?;
                estado.pila.push(TipoAbstracto::Entero);
            }
            CodigoOp::Duplicar => {
                let Some(tipo) = estado.pila.last().copied() else {
                    return Err(fallo(pc, "pila vacía al duplicar"));
                };
                estado.pila.push(tipo);
            }
            CodigoOp::Desechar => {
                sacar(&mut estado, pc)?;
            }
            CodigoOp::NegarEntero => {
                exigir(sacar(&mut estado, pc)?, TipoAbstracto::Entero, pc)?;
                estado.pila.push(TipoAbstracto::Entero);
            }
            CodigoOp::NoLogico => {
                exigir(sacar(&mut estado, pc)?, TipoAbstracto::Log, pc)?;
                estado.pila.push(TipoAbstracto::Log);
            }
            CodigoOp::SumarEntero
            | CodigoOp::RestarEntero
            | CodigoOp::MultiplicarEntero
            | CodigoOp::DividirEntero
            | CodigoOp::ModuloEntero
            | CodigoOp::IgualEntero
            | CodigoOp::DiferenteEntero
            | CodigoOp::MayorEntero
            | CodigoOp::MenorEntero
            | CodigoOp::MayorOIgualEntero
            | CodigoOp::MenorOIgualEntero => {
                exigir(sacar(&mut estado, pc)?, TipoAbstracto::Entero, pc)?;
                exigir(sacar(&mut estado, pc)?, TipoAbstracto::Entero, pc)?;
                let tipo = if instruccion.es_binaria_entera()
                    && matches!(
                        instruccion.op,
                        CodigoOp::IgualEntero
                            | CodigoOp::DiferenteEntero
                            | CodigoOp::MayorEntero
                            | CodigoOp::MenorEntero
                            | CodigoOp::MayorOIgualEntero
                            | CodigoOp::MenorOIgualEntero
                    ) {
                    TipoAbstracto::Log
                } else {
                    TipoAbstracto::Entero
                };
                estado.pila.push(tipo);
            }
            CodigoOp::Saltar => siguientes.push(instruccion.destino() as usize),
            CodigoOp::SaltarSiFalso | CodigoOp::SaltarSiVerdadero => {
                exigir(sacar(&mut estado, pc)?, TipoAbstracto::Log, pc)?;
                siguientes.push(instruccion.destino() as usize);
                siguientes.push(pc + 1);
            }
            CodigoOp::MenorEnteroSaltarSiFalso => {
                exigir(sacar(&mut estado, pc)?, TipoAbstracto::Entero, pc)?;
                exigir(sacar(&mut estado, pc)?, TipoAbstracto::Entero, pc)?;
                siguientes.push(instruccion.destino() as usize);
                siguientes.push(pc + 1);
            }
            CodigoOp::Retornar => {
                exigir(sacar(&mut estado, pc)?, TipoAbstracto::Entero, pc)?;
                retornos += 1;
            }
            CodigoOp::RetornarLocal => {
                exigir_local(&estado, instruccion.ranura(), TipoAbstracto::Entero, pc)?;
                retornos += 1;
            }
            _ => {
                return Err(fallo(
                    pc,
                    format!("instrucción no soportada: {:?}", instruccion.op),
                ));
            }
        }
        if siguientes.is_empty()
            && !matches!(instruccion.op, CodigoOp::Retornar | CodigoOp::RetornarLocal)
        {
            siguientes.push(pc + 1);
        }
        max_pila = max_pila.max(estado.pila.len());
        for siguiente in siguientes {
            encolar(siguiente, estado.clone(), n, &mut estados, &mut trabajo)?;
        }
    }
    if retornos == 0 {
        return Err(ErrorJit::NoElegible(
            "no hay un retorno entero alcanzable".to_string(),
        ));
    }
    let profundidades = estados
        .iter()
        .map(|estado| {
            estado
                .as_ref()
                .map(|estado| u16::try_from(estado.pila.len()).expect("profundidad verificada"))
        })
        .collect();
    let max_pila = u16::try_from(max_pila)
        .map_err(|_| ErrorJit::NoElegible("la pila supera 65535 valores".to_string()))?;
    Ok(Analisis {
        profundidades,
        max_pila,
    })
}

fn encolar(
    pc: usize,
    estado: Estado,
    n: usize,
    estados: &mut [Option<Estado>],
    trabajo: &mut VecDeque<usize>,
) -> Result<(), ErrorJit> {
    if pc > n {
        return Err(fallo(pc, "salto fuera de rango"));
    }
    if pc == n {
        trabajo.push_back(pc);
        return Ok(());
    }
    match &estados[pc] {
        Some(anterior) if anterior != &estado => Err(fallo(
            pc,
            "la unión del CFG no conserva tipos y profundidad de pila",
        )),
        Some(_) => Ok(()),
        None => {
            estados[pc] = Some(estado);
            trabajo.push_back(pc);
            Ok(())
        }
    }
}

fn sacar(estado: &mut Estado, pc: usize) -> Result<TipoAbstracto, ErrorJit> {
    estado
        .pila
        .pop()
        .ok_or_else(|| fallo(pc, "desbordamiento inferior de pila"))
}

fn exigir_local(
    estado: &Estado,
    ranura: u16,
    esperado: TipoAbstracto,
    pc: usize,
) -> Result<(), ErrorJit> {
    let tipo = estado
        .locales
        .get(ranura as usize)
        .copied()
        .ok_or_else(|| fallo(pc, "ranura local fuera de rango"))?;
    exigir(tipo, esperado, pc)
}

fn exigir(actual: TipoAbstracto, esperado: TipoAbstracto, pc: usize) -> Result<(), ErrorJit> {
    if actual == esperado {
        Ok(())
    } else {
        Err(fallo(
            pc,
            format!("se esperaba {esperado:?} y se encontró {actual:?}"),
        ))
    }
}

fn fallo(pc: usize, mensaje: impl Into<String>) -> ErrorJit {
    ErrorJit::NoElegible(format!("pc {pc}: {}", mensaje.into()))
}

#[cfg(test)]
mod pruebas {
    use bytecode::{FuncionCompilada, Instruccion, ModuloCompilado, Trozo};
    use nucleo::Ubicacion;

    use super::*;

    fn funcion(instrucciones: Vec<Instruccion>) -> (ModuloCompilado, FuncionCompilada) {
        let ubicaciones = vec![Ubicacion::default(); instrucciones.len()];
        (
            ModuloCompilado::default(),
            FuncionCompilada {
                nombre: "prueba".to_string(),
                parametros: vec![("n".to_string(), false)],
                trozo: Trozo {
                    instrucciones,
                    ubicaciones,
                    max_pila: 0,
                },
                asincrona: false,
                ranuras_locales: 1,
            },
        )
    }

    #[test]
    fn deberia_aceptar_retorno_de_parametro_entero() {
        let (modulo, funcion) = funcion(vec![Instruccion::RetornarLocal(0)]);
        let analisis = analizar(&modulo, &funcion).expect("función elegible");
        assert_eq!(analisis.max_pila, 0);
    }

    #[test]
    fn deberia_rechazar_operacion_generica() {
        let (modulo, funcion) = funcion(vec![
            Instruccion::CargarLocal(0),
            Instruccion::CargarLocal(0),
            Instruccion::Sumar(),
            Instruccion::Retornar(),
        ]);
        assert!(matches!(
            analizar(&modulo, &funcion),
            Err(ErrorJit::NoElegible(_))
        ));
    }
}
