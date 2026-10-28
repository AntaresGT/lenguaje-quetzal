//! Verificador de bytecode: rechaza módulos corruptos (cache, disco) y
//! calcula `max_pila` por función mediante propagación de flujo.

use std::collections::{HashMap, HashSet, VecDeque};

use nucleo::{CategoriaError, ErrorQuetzal};

use crate::instruccion::{CodigoOp, Instruccion, Trozo};
use crate::modulo::{FuncionCompilada, ModuloCompilado, ObjetoCompilado};

/// Verifica un módulo y rellena `max_pila` de cada trozo.
pub fn verificar_modulo(modulo: &mut ModuloCompilado) -> Result<(), ErrorQuetzal> {
    if modulo.globales_mutables.len() != modulo.nombres_globales.len() {
        return Err(error_verificacion(
            "módulo",
            "la mutabilidad de las globales no coincide con sus nombres",
        ));
    }
    let limites = Limites {
        constantes: modulo.constantes.len(),
        nombres: modulo.nombres.len(),
        globales: modulo.nombres_globales.len(),
    };
    let layouts = layouts_objetos(&modulo.objetos);
    modulo.principal.max_pila = verificar_trozo(
        &modulo.principal,
        limites,
        modulo.ranuras_locales_principal,
        None,
        "principal",
    )?;
    for funcion in &mut modulo.funciones {
        verificar_funcion(funcion, limites, None)?;
    }
    for objeto in &mut modulo.objetos {
        let slots = layouts.get(&objeto.nombre).copied();
        for metodo in &mut objeto.metodos {
            verificar_funcion(&mut metodo.funcion, limites, slots)?;
        }
        for constructor in &mut objeto.constructores {
            verificar_funcion(constructor, limites, slots)?;
        }
        for atributo in &mut objeto.atributos {
            if let Some(inicial) = &mut atributo.inicial {
                inicial.max_pila = verificar_trozo(inicial, limites, 0, slots, &atributo.nombre)?;
            }
        }
    }
    Ok(())
}

/// Verifica una función con las tablas de su módulo sin alterar el bytecode.
pub fn verificar_funcion_inmutable(
    modulo: &ModuloCompilado,
    funcion: &FuncionCompilada,
) -> Result<(), ErrorQuetzal> {
    if modulo.globales_mutables.len() != modulo.nombres_globales.len() {
        return Err(error_verificacion(
            &funcion.nombre,
            "la mutabilidad de las globales no coincide con sus nombres",
        ));
    }
    let limites = Limites {
        constantes: modulo.constantes.len(),
        nombres: modulo.nombres.len(),
        globales: modulo.nombres_globales.len(),
    };
    let mut copia = funcion.clone();
    verificar_funcion(&mut copia, limites, None)
}

fn verificar_funcion(
    funcion: &mut FuncionCompilada,
    limites: Limites,
    slots: Option<usize>,
) -> Result<(), ErrorQuetzal> {
    if (funcion.ranuras_locales as usize) < funcion.parametros.len() {
        return Err(error_verificacion(
            &funcion.nombre,
            "hay menos ranuras locales que parámetros",
        ));
    }
    funcion.trozo.max_pila = verificar_trozo(
        &funcion.trozo,
        limites,
        funcion.ranuras_locales,
        slots,
        &funcion.nombre,
    )?;
    Ok(())
}

#[derive(Clone, Copy, Default)]
struct Limites {
    constantes: usize,
    nombres: usize,
    globales: usize,
}

fn verificar_trozo(
    trozo: &Trozo,
    limites: Limites,
    ranuras: u16,
    slots: Option<usize>,
    nombre: &str,
) -> Result<u16, ErrorQuetzal> {
    let n = trozo.instrucciones.len();
    if trozo.ubicaciones.len() != n {
        return Err(error_verificacion(
            nombre,
            "las instrucciones y ubicaciones tienen longitudes distintas",
        ));
    }
    if n > u32::MAX as usize {
        return Err(error_verificacion(nombre, "el trozo es demasiado grande"));
    }

    for instruccion in trozo.instrucciones.iter().copied() {
        if let Some(destino) = instruccion.destino_salto() {
            let destino = destino as usize;
            if destino > n {
                return Err(error_verificacion(
                    nombre,
                    format!("salto a {destino} fuera del trozo ({n} instrucciones)"),
                ));
            }
            if instruccion.op == CodigoOp::EntrarIntentar && destino == n {
                return Err(error_verificacion(
                    nombre,
                    "el manejador de excepciones no contiene instrucciones",
                ));
            }
        }
        validar_indices(instruccion, limites, ranuras, slots, nombre)?;
    }
    if n == 0 {
        return Ok(0);
    }

    let mut profundidades = vec![None; n];
    let mut trabajo = VecDeque::from([(0usize, 0u32)]);
    let mut maxima = 0u32;
    profundidades[0] = Some(0);

    while let Some((pc, profundidad)) = trabajo.pop_front() {
        let instruccion = trozo.instrucciones[pc];
        maxima = maxima.max(profundidad);
        match instruccion.op {
            CodigoOp::SaltarSiFalsoYDejar | CodigoOp::SaltarSiVerdaderoYDejar => {
                aplicar_efecto(profundidad, 1, 1, pc, nombre)?;
                let siguiente = aplicar_efecto(profundidad, 1, 0, pc, nombre)?;
                encolar(
                    instruccion.destino() as usize,
                    profundidad,
                    pc,
                    n,
                    &mut profundidades,
                    &mut trabajo,
                    &mut maxima,
                    nombre,
                )?;
                encolar(
                    pc + 1,
                    siguiente,
                    pc,
                    n,
                    &mut profundidades,
                    &mut trabajo,
                    &mut maxima,
                    nombre,
                )?;
            }
            CodigoOp::IteradorSiguiente => {
                let siguiente = aplicar_efecto(profundidad, 1, 1, pc, nombre)?;
                let destino = aplicar_efecto(profundidad, 1, 0, pc, nombre)?;
                encolar(
                    pc + 1,
                    siguiente,
                    pc,
                    n,
                    &mut profundidades,
                    &mut trabajo,
                    &mut maxima,
                    nombre,
                )?;
                encolar(
                    instruccion.destino() as usize,
                    destino,
                    pc,
                    n,
                    &mut profundidades,
                    &mut trabajo,
                    &mut maxima,
                    nombre,
                )?;
            }
            CodigoOp::EntrarIntentar => {
                let manejador = profundidad.checked_add(1).ok_or_else(|| {
                    error_verificacion(nombre, "la pila del manejador desborda el contador")
                })?;
                encolar(
                    pc + 1,
                    profundidad,
                    pc,
                    n,
                    &mut profundidades,
                    &mut trabajo,
                    &mut maxima,
                    nombre,
                )?;
                encolar(
                    instruccion.destino() as usize,
                    manejador,
                    pc,
                    n,
                    &mut profundidades,
                    &mut trabajo,
                    &mut maxima,
                    nombre,
                )?;
            }
            _ => {
                let (consume, produce) = efecto_pila(instruccion, nombre)?;
                let salida = aplicar_efecto(profundidad, consume, produce, pc, nombre)?;
                match instruccion.op {
                    CodigoOp::Saltar => encolar(
                        instruccion.destino() as usize,
                        salida,
                        pc,
                        n,
                        &mut profundidades,
                        &mut trabajo,
                        &mut maxima,
                        nombre,
                    )?,
                    CodigoOp::SaltarSiFalso
                    | CodigoOp::SaltarSiVerdadero
                    | CodigoOp::MenorEnteroSaltarSiFalso => {
                        encolar(
                            instruccion.destino() as usize,
                            salida,
                            pc,
                            n,
                            &mut profundidades,
                            &mut trabajo,
                            &mut maxima,
                            nombre,
                        )?;
                        encolar(
                            pc + 1,
                            salida,
                            pc,
                            n,
                            &mut profundidades,
                            &mut trabajo,
                            &mut maxima,
                            nombre,
                        )?;
                    }
                    CodigoOp::Retornar | CodigoOp::RetornarLocal | CodigoOp::Lanzar => {}
                    _ => encolar(
                        pc + 1,
                        salida,
                        pc,
                        n,
                        &mut profundidades,
                        &mut trabajo,
                        &mut maxima,
                        nombre,
                    )?,
                }
            }
        }
    }

    u16::try_from(maxima).map_err(|_| error_verificacion(nombre, "la pila supera 65535 valores"))
}

#[allow(clippy::too_many_arguments)]
fn encolar(
    destino: usize,
    profundidad: u32,
    origen: usize,
    n: usize,
    profundidades: &mut [Option<u32>],
    trabajo: &mut VecDeque<(usize, u32)>,
    maxima: &mut u32,
    nombre: &str,
) -> Result<(), ErrorQuetzal> {
    *maxima = (*maxima).max(profundidad);
    if destino == n {
        return Ok(());
    }
    match profundidades[destino] {
        Some(anterior) if anterior != profundidad => Err(error_verificacion(
            nombre,
            format!(
                "unión incompatible en la instrucción {destino}: profundidades {anterior} y {profundidad} (desde {origen})"
            ),
        )),
        Some(_) => Ok(()),
        None => {
            profundidades[destino] = Some(profundidad);
            trabajo.push_back((destino, profundidad));
            Ok(())
        }
    }
}

fn aplicar_efecto(
    profundidad: u32,
    consume: u32,
    produce: u32,
    pc: usize,
    nombre: &str,
) -> Result<u32, ErrorQuetzal> {
    let restante = profundidad.checked_sub(consume).ok_or_else(|| {
        error_verificacion(
            nombre,
            format!("desbordamiento inferior de pila en la instrucción {pc}"),
        )
    })?;
    restante
        .checked_add(produce)
        .ok_or_else(|| error_verificacion(nombre, "desbordamiento del contador de pila"))
}

fn validar_indices(
    instruccion: Instruccion,
    limites: Limites,
    ranuras: u16,
    slots: Option<usize>,
    nombre: &str,
) -> Result<(), ErrorQuetzal> {
    match instruccion.op {
        CodigoOp::CargarConstante if instruccion.indice_u32() as usize >= limites.constantes => {
            Err(error_verificacion(
                nombre,
                "índice de constante fuera de rango",
            ))
        }
        CodigoOp::CargarLocal
        | CodigoOp::GuardarLocal
        | CodigoOp::AjustarLocal
        | CodigoOp::RetornarLocal
            if instruccion.ranura() >= ranuras =>
        {
            Err(error_verificacion(nombre, "ranura local fuera de rango"))
        }
        CodigoOp::CargarLocalLocalSumarEntero | CodigoOp::AjustarLocalConLocal
            if instruccion.ranura() >= ranuras || instruccion.ranura_derecha() >= ranuras =>
        {
            Err(error_verificacion(nombre, "ranura local fuera de rango"))
        }
        CodigoOp::LimpiarLocales if instruccion.ranura() > ranuras => Err(error_verificacion(
            nombre,
            "limpieza de locales fuera de rango",
        )),
        CodigoOp::CargarGlobal | CodigoOp::DeclararGlobal | CodigoOp::GuardarGlobal
            if instruccion.indice_u16() as usize >= limites.globales =>
        {
            Err(error_verificacion(
                nombre,
                "índice de global fuera de rango",
            ))
        }
        CodigoOp::CargarExterno
        | CodigoOp::GuardarExterno
        | CodigoOp::CargarMiembro
        | CodigoOp::GuardarMiembro
        | CodigoOp::LlamarMetodo
            if instruccion.indice_nombre() as usize >= limites.nombres =>
        {
            Err(error_verificacion(
                nombre,
                "índice de nombre fuera de rango",
            ))
        }
        CodigoOp::CargarAtributo | CodigoOp::GuardarAtributo | CodigoOp::CargarEstoAtributo
            if slots.is_some_and(|total| instruccion.slot() as usize >= total) =>
        {
            Err(error_verificacion(
                nombre,
                "slot de atributo fuera de rango",
            ))
        }
        _ => Ok(()),
    }
}

fn efecto_pila(instruccion: Instruccion, nombre: &str) -> Result<(u32, u32), ErrorQuetzal> {
    let efecto = match instruccion.op {
        CodigoOp::CargarConstante
        | CodigoOp::CargarEntero
        | CodigoOp::CargarLocal
        | CodigoOp::CargarGlobal
        | CodigoOp::CargarExterno
        | CodigoOp::CargarEsto
        | CodigoOp::CargarPadre
        | CodigoOp::CargarEstoAtributo
        | CodigoOp::CargarLocalLocalSumarEntero => (0, 1),
        CodigoOp::Duplicar => (1, 2),
        CodigoOp::Desechar
        | CodigoOp::GuardarLocal
        | CodigoOp::DeclararGlobal
        | CodigoOp::GuardarGlobal
        | CodigoOp::GuardarExterno
        | CodigoOp::SaltarSiFalso
        | CodigoOp::SaltarSiVerdadero
        | CodigoOp::Lanzar => (1, 0),
        CodigoOp::Sumar
        | CodigoOp::Restar
        | CodigoOp::Multiplicar
        | CodigoOp::Dividir
        | CodigoOp::Modulo
        | CodigoOp::Igual
        | CodigoOp::Diferente
        | CodigoOp::Mayor
        | CodigoOp::Menor
        | CodigoOp::MayorOIgual
        | CodigoOp::MenorOIgual
        | CodigoOp::SumarEntero
        | CodigoOp::RestarEntero
        | CodigoOp::MultiplicarEntero
        | CodigoOp::DividirEntero
        | CodigoOp::ModuloEntero
        | CodigoOp::IgualEntero
        | CodigoOp::DiferenteEntero
        | CodigoOp::MayorEntero
        | CodigoOp::MenorEntero
        | CodigoOp::MayorOIgualEntero
        | CodigoOp::MenorOIgualEntero
        | CodigoOp::Indexar => (2, 1),
        CodigoOp::CargarMiembro
        | CodigoOp::CargarAtributo
        | CodigoOp::CrearIterador
        | CodigoOp::Esperar
        | CodigoOp::Negar
        | CodigoOp::NegarEntero
        | CodigoOp::NoLogico => (1, 1),
        CodigoOp::GuardarMiembro
        | CodigoOp::GuardarAtributo
        | CodigoOp::MenorEnteroSaltarSiFalso => (2, 0),
        CodigoOp::GuardarIndice => (3, 0),
        CodigoOp::Llamar | CodigoOp::Instanciar => (
            instruccion.cantidad().checked_add(1).ok_or_else(|| {
                error_verificacion(nombre, "la cantidad de argumentos desborda el contador")
            })?,
            1,
        ),
        CodigoOp::LlamarMetodo => (
            instruccion
                .cantidad_metodo()
                .checked_add(1)
                .ok_or_else(|| {
                    error_verificacion(nombre, "la cantidad de argumentos desborda el contador")
                })?,
            1,
        ),
        CodigoOp::LlamarNativa | CodigoOp::CrearLista | CodigoOp::Interpolar => {
            (instruccion.cantidad(), 1)
        }
        CodigoOp::CrearJsn => (
            instruccion.cantidad().checked_mul(2).ok_or_else(|| {
                error_verificacion(nombre, "la cantidad de entradas desborda el contador")
            })?,
            1,
        ),
        CodigoOp::Retornar => (1, 0),
        CodigoOp::Saltar
        | CodigoOp::LimpiarLocales
        | CodigoOp::AjustarLocal
        | CodigoOp::AjustarLocalConLocal
        | CodigoOp::RetornarLocal
        | CodigoOp::SalirIntentar => (0, 0),
        CodigoOp::SaltarSiFalsoYDejar
        | CodigoOp::SaltarSiVerdaderoYDejar
        | CodigoOp::IteradorSiguiente
        | CodigoOp::EntrarIntentar => {
            return Err(error_verificacion(
                nombre,
                "efecto de pila dependiente del camino fuera de su análisis",
            ));
        }
    };
    Ok(efecto)
}

fn layouts_objetos(objetos: &[ObjetoCompilado]) -> HashMap<String, usize> {
    let mut layouts = HashMap::new();
    for objeto in objetos {
        let mut nombres = HashSet::new();
        let mut visitados = HashSet::new();
        if recolectar_slots(&objeto.nombre, objetos, &mut nombres, &mut visitados) {
            layouts.insert(objeto.nombre.clone(), nombres.len());
        }
    }
    layouts
}

fn recolectar_slots(
    nombre: &str,
    objetos: &[ObjetoCompilado],
    slots: &mut HashSet<String>,
    visitados: &mut HashSet<String>,
) -> bool {
    if !visitados.insert(nombre.to_string()) {
        return false;
    }
    let Some(objeto) = objetos.iter().find(|objeto| objeto.nombre == nombre) else {
        return false;
    };
    for padre in objeto.padres.iter().chain(objeto.extiende_como.iter()) {
        if !recolectar_slots(padre, objetos, slots, visitados) {
            return false;
        }
    }
    for atributo in &objeto.atributos {
        if !atributo.libre {
            slots.insert(atributo.nombre.clone());
        }
    }
    true
}

fn error_verificacion(funcion: &str, mensaje: impl Into<String>) -> ErrorQuetzal {
    ErrorQuetzal::nuevo(
        "E0410",
        CategoriaError::Interno,
        format!("bytecode inválido en '{funcion}': {}", mensaje.into()),
    )
}

/// Recorre un módulo ya deserializado (cache) y lo rechaza si está corrupto.
pub fn verificar_modulo_inmutable(modulo: &ModuloCompilado) -> Result<(), ErrorQuetzal> {
    let mut copia = modulo.clone();
    verificar_modulo(&mut copia)
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use nucleo::Ubicacion;

    fn verificar(ops: Vec<Instruccion>) -> Result<u16, ErrorQuetzal> {
        let trozo = Trozo {
            ubicaciones: vec![Ubicacion::nueva(0, 1); ops.len()],
            instrucciones: ops,
            max_pila: 0,
        };
        verificar_trozo(
            &trozo,
            Limites {
                globales: 1,
                ..Limites::default()
            },
            1,
            None,
            "prueba",
        )
    }

    #[test]
    fn verificar_deberia_rechazar_salto_fuera_de_rango() {
        assert!(verificar(vec![Instruccion::Saltar(99)]).is_err());
    }

    #[test]
    fn declarar_global_deberia_consumir_su_valor() {
        assert!(verificar(vec![Instruccion::DeclararGlobal(0)]).is_err());
        let maxima = verificar(vec![
            Instruccion::CargarEntero(1),
            Instruccion::DeclararGlobal(0),
        ])
        .expect("declaración válida");
        assert_eq!(maxima, 1);
    }

    #[test]
    fn verificar_deberia_detectar_underflow_en_una_rama() {
        assert!(
            verificar(vec![
                Instruccion::CargarEntero(1),
                Instruccion::SaltarSiFalso(3),
                Instruccion::Retornar(),
                Instruccion::Desechar(),
            ])
            .is_err()
        );
    }

    #[test]
    fn cortocircuito_deberia_calcular_maxima_por_camino() {
        let maxima = verificar(vec![
            Instruccion::CargarEntero(1),
            Instruccion::SaltarSiFalsoYDejar(3),
            Instruccion::CargarEntero(2),
            Instruccion::Retornar(),
        ])
        .expect("cortocircuito válido");
        assert_eq!(maxima, 1);
    }

    #[test]
    fn verificar_deberia_rechazar_una_union_con_profundidades_distintas() {
        assert!(
            verificar(vec![
                Instruccion::CargarEntero(1),
                Instruccion::SaltarSiFalsoYDejar(3),
                Instruccion::Saltar(4),
                Instruccion::CargarEntero(2),
                Instruccion::Retornar(),
            ])
            .is_err()
        );
    }

    #[test]
    fn iterador_deberia_distinguir_elemento_y_salida() {
        assert!(
            verificar(vec![
                Instruccion::CargarEntero(1),
                Instruccion::IteradorSiguiente(3),
                Instruccion::Retornar(),
                Instruccion::Retornar(),
            ])
            .is_err()
        );
        let maxima = verificar(vec![
            Instruccion::CargarEntero(1),
            Instruccion::IteradorSiguiente(3),
            Instruccion::Retornar(),
            Instruccion::CargarEntero(0),
            Instruccion::Retornar(),
        ])
        .expect("iterador válido");
        assert_eq!(maxima, 1);
    }

    #[test]
    fn manejador_deberia_recibir_la_excepcion() {
        let maxima = verificar(vec![
            Instruccion::EntrarIntentar(3, 0),
            Instruccion::SalirIntentar(),
            Instruccion::Saltar(4),
            Instruccion::GuardarLocal(0),
        ])
        .expect("manejador válido");
        assert_eq!(maxima, 1);
        assert!(
            verificar(vec![
                Instruccion::EntrarIntentar(3, 0),
                Instruccion::SalirIntentar(),
                Instruccion::Saltar(4),
                Instruccion::Desechar(),
                Instruccion::Desechar(),
            ])
            .is_err()
        );
    }

    #[test]
    fn verificar_deberia_rechazar_indices_y_metadatos_invalidos() {
        assert!(verificar(vec![Instruccion::CargarConstante(0)]).is_err());
        assert!(verificar(vec![Instruccion::CargarLocal(1)]).is_err());
        let mut modulo = ModuloCompilado {
            nombres_globales: vec!["x".to_string()],
            globales_mutables: Vec::new(),
            ..Default::default()
        };
        assert!(verificar_modulo(&mut modulo).is_err());
    }

    #[test]
    fn verificar_deberia_rechazar_un_manejador_vacio() {
        assert!(verificar(vec![Instruccion::EntrarIntentar(1, 0)]).is_err());
    }

    #[test]
    fn mutaciones_deterministas_no_deberian_provocar_panic() {
        for semilla in 0..512u32 {
            let resultado = verificar(vec![
                Instruccion::CargarEntero(1),
                Instruccion::SaltarSiFalso(semilla % 7),
                Instruccion::CrearJsn(semilla),
                Instruccion::LlamarNativa(semilla as u16, semilla as u16),
                Instruccion::Retornar(),
            ]);
            let _ = resultado;
        }
    }
}
