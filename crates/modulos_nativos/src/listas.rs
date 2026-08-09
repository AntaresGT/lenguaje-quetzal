//! Métodos nativos del tipo `lista` según `ejemplos/metodos_listas.qz`,
//! y la función global `rango`.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

use maquina_virtual::valores::{jsn_a_texto, texto_de_valor};
use maquina_virtual::{Fallo, RegistroNativos, Valor};
use rust_decimal::Decimal;

use crate::util::{
    arg_decimal, arg_entero, arg_lista, arg_texto, error, exigir_aridad, indice_normalizado,
};

pub fn registrar(registro: &mut RegistroNativos) {
    metodo(registro, "lista.longitud", 0, |lista, _| {
        Ok(Valor::Entero(lista.borrow().len() as i64))
    });
    metodo(registro, "lista.esta_vacia", 0, |lista, _| {
        Ok(Valor::Log(lista.borrow().is_empty()))
    });
    metodo(registro, "lista.agregar", 1, |lista, argumentos| {
        lista.borrow_mut().push(argumentos[1].clone());
        Ok(Valor::Nulo)
    });
    metodo(registro, "lista.insertar", 2, |lista, argumentos| {
        let indice = arg_entero("lista.insertar", argumentos, 1)?;
        let mut elementos = lista.borrow_mut();
        let posicion = usize::try_from(indice)
            .ok()
            .filter(|posicion| *posicion <= elementos.len())
            .ok_or_else(|| error("E0403", format!("el índice {indice} está fuera de rango")))?;
        elementos.insert(posicion, argumentos[2].clone());
        Ok(Valor::Nulo)
    });
    metodo(registro, "lista.remover", 1, |lista, argumentos| {
        let mut elementos = lista.borrow_mut();
        if let Some(posicion) = elementos
            .iter()
            .position(|elemento| elemento.es_igual(&argumentos[1]))
        {
            elementos.remove(posicion);
        }
        Ok(Valor::Nulo)
    });
    metodo(registro, "lista.quitar_en", 1, |lista, argumentos| {
        let indice = arg_entero("lista.quitar_en", argumentos, 1)?;
        let mut elementos = lista.borrow_mut();
        let posicion = indice_normalizado(indice, elementos.len())
            .ok_or_else(|| error("E0403", format!("el índice {indice} está fuera de rango")))?;
        Ok(elementos.remove(posicion))
    });
    metodo(registro, "lista.limpiar", 0, |lista, _| {
        lista.borrow_mut().clear();
        Ok(Valor::Nulo)
    });
    metodo(registro, "lista.contiene", 1, |lista, argumentos| {
        Ok(Valor::Log(
            lista
                .borrow()
                .iter()
                .any(|elemento| elemento.es_igual(&argumentos[1])),
        ))
    });
    metodo(registro, "lista.buscar", 1, |lista, argumentos| {
        let posicion = lista
            .borrow()
            .iter()
            .position(|elemento| elemento.es_igual(&argumentos[1]));
        Ok(Valor::Entero(
            posicion.map(|valor| valor as i64).unwrap_or(-1),
        ))
    });
    metodo(registro, "lista.buscar_ultimo", 1, |lista, argumentos| {
        let posicion = lista
            .borrow()
            .iter()
            .rposition(|elemento| elemento.es_igual(&argumentos[1]));
        Ok(Valor::Entero(
            posicion.map(|valor| valor as i64).unwrap_or(-1),
        ))
    });
    metodo(registro, "lista.contar", 1, |lista, argumentos| {
        Ok(Valor::Entero(
            lista
                .borrow()
                .iter()
                .filter(|elemento| elemento.es_igual(&argumentos[1]))
                .count() as i64,
        ))
    });
    metodo(registro, "lista.ordenar", 0, |lista, _| {
        ordenar(&mut lista.borrow_mut(), false)?;
        Ok(Valor::Nulo)
    });
    metodo(registro, "lista.ordenar_descendente", 0, |lista, _| {
        ordenar(&mut lista.borrow_mut(), true)?;
        Ok(Valor::Nulo)
    });
    metodo(registro, "lista.ordenado", 0, |lista, _| {
        let mut copia = lista.borrow().clone();
        ordenar(&mut copia, false)?;
        Ok(Valor::lista(copia))
    });
    metodo(registro, "lista.invertir", 0, |lista, _| {
        lista.borrow_mut().reverse();
        Ok(Valor::Nulo)
    });
    metodo(registro, "lista.primero", 0, |lista, _| {
        lista
            .borrow()
            .first()
            .cloned()
            .ok_or_else(|| error("E0403", "la lista está vacía"))
    });
    metodo(registro, "lista.ultimo", 0, |lista, _| {
        lista
            .borrow()
            .last()
            .cloned()
            .ok_or_else(|| error("E0403", "la lista está vacía"))
    });
    metodo(registro, "lista.tomar", 1, |lista, argumentos| {
        let cantidad = arg_entero("lista.tomar", argumentos, 1)?.max(0) as usize;
        Ok(Valor::lista(
            lista.borrow().iter().take(cantidad).cloned().collect(),
        ))
    });
    metodo(registro, "lista.saltar", 1, |lista, argumentos| {
        let cantidad = arg_entero("lista.saltar", argumentos, 1)?.max(0) as usize;
        Ok(Valor::lista(
            lista.borrow().iter().skip(cantidad).cloned().collect(),
        ))
    });
    metodo(registro, "lista.sublista", 2, |lista, argumentos| {
        let inicio = arg_entero("lista.sublista", argumentos, 1)?;
        let fin = arg_entero("lista.sublista", argumentos, 2)?;
        let elementos = lista.borrow();
        let inicio = usize::try_from(inicio.max(0))
            .unwrap_or(0)
            .min(elementos.len());
        let fin = usize::try_from(fin.max(0))
            .unwrap_or(0)
            .min(elementos.len());
        Ok(Valor::lista(elementos[inicio..fin.max(inicio)].to_vec()))
    });
    metodo(registro, "lista.sumar", 0, |lista, _| {
        let elementos = lista.borrow();
        // Si todos son enteros el resultado es entero; si hay decimales, número.
        if elementos
            .iter()
            .all(|elemento| matches!(elemento, Valor::Entero(_)))
        {
            let mut suma: i64 = 0;
            for elemento in elementos.iter() {
                if let Valor::Entero(entero) = elemento {
                    suma = suma
                        .checked_add(*entero)
                        .ok_or_else(|| error("E0402", "desbordamiento al sumar la lista"))?;
                }
            }
            return Ok(Valor::Entero(suma));
        }
        let mut suma = Decimal::ZERO;
        for elemento in elementos.iter() {
            let valor = arg_decimal("lista.sumar", std::slice::from_ref(elemento), 0)?;
            suma = suma
                .checked_add(valor)
                .ok_or_else(|| error("E0402", "desbordamiento al sumar la lista"))?;
        }
        Ok(Valor::Numero(suma))
    });
    metodo(registro, "lista.promedio", 0, |lista, _| {
        let elementos = lista.borrow();
        if elementos.is_empty() {
            return Err(error("E0406", "no se puede promediar una lista vacía"));
        }
        let mut suma = Decimal::ZERO;
        for elemento in elementos.iter() {
            suma = suma
                .checked_add(arg_decimal(
                    "lista.promedio",
                    std::slice::from_ref(elemento),
                    0,
                )?)
                .ok_or_else(|| error("E0402", "desbordamiento al promediar la lista"))?;
        }
        suma.checked_div(Decimal::from(elementos.len()))
            .map(Valor::Numero)
            .ok_or_else(|| error("E0402", "desbordamiento al promediar la lista"))
    });
    metodo(registro, "lista.maximo", 0, |lista, _| {
        extremo(&lista.borrow(), Ordering::Greater)
    });
    metodo(registro, "lista.minimo", 0, |lista, _| {
        extremo(&lista.borrow(), Ordering::Less)
    });
    metodo(registro, "lista.unir", 1, |lista, argumentos| {
        let separador = arg_texto("lista.unir", argumentos, 1)?;
        let unido = lista
            .borrow()
            .iter()
            .map(texto_de_valor)
            .collect::<Vec<_>>()
            .join(separador);
        Ok(Valor::texto(unido))
    });
    metodo(registro, "lista.concatenar", 1, |lista, argumentos| {
        let otra = arg_lista("lista.concatenar", argumentos, 1)?;
        let mut resultado = lista.borrow().clone();
        resultado.extend(otra.borrow().iter().cloned());
        Ok(Valor::lista(resultado))
    });
    metodo(registro, "lista.extender", 1, |lista, argumentos| {
        let otra = arg_lista("lista.extender", argumentos, 1)?;
        let elementos_nuevos: Vec<Valor> = otra.borrow().clone();
        lista.borrow_mut().extend(elementos_nuevos);
        Ok(Valor::Nulo)
    });
    metodo(registro, "lista.texto", 0, |lista, _| {
        Ok(Valor::texto(texto_de_valor(&Valor::Lista(lista))))
    });
    metodo(registro, "lista.json", 0, |lista, _| {
        Ok(Valor::texto(jsn_a_texto(&Valor::Lista(lista), false)))
    });
    metodo(registro, "lista.logico", 0, |lista, _| {
        Ok(Valor::Log(!lista.borrow().is_empty()))
    });

    // Función global `rango(inicio, fin)`: ambos extremos incluidos.
    registro.registrar_funcion(
        "rango",
        Box::new(|argumentos| {
            exigir_aridad("rango", argumentos, 2)?;
            let inicio = arg_entero("rango", argumentos, 0)?;
            let fin = arg_entero("rango", argumentos, 1)?;
            if inicio > fin {
                return Ok(Valor::lista(Vec::new()));
            }
            Ok(Valor::lista((inicio..=fin).map(Valor::Entero).collect()))
        }),
    );
}

/// Registra un método de lista verificando receptor y aridad.
fn metodo(
    registro: &mut RegistroNativos,
    nombre: &'static str,
    argumentos_extra: usize,
    implementacion: impl Fn(Rc<RefCell<Vec<Valor>>>, &[Valor]) -> Result<Valor, Fallo> + 'static,
) {
    registro.registrar_funcion(
        nombre,
        Box::new(move |argumentos| {
            exigir_aridad(nombre, argumentos, argumentos_extra + 1)?;
            let receptor = arg_lista(nombre, argumentos, 0)?;
            implementacion(receptor, argumentos)
        }),
    );
}

/// Ordena la lista in situ; los elementos deben ser comparables entre sí.
fn ordenar(elementos: &mut [Valor], descendente: bool) -> Result<(), Fallo> {
    let mut fallo = None;
    elementos.sort_by(|a, b| match comparar(a, b) {
        Some(orden) => {
            if descendente {
                orden.reverse()
            } else {
                orden
            }
        }
        None => {
            fallo.get_or_insert_with(|| {
                error(
                    "E0406",
                    format!(
                        "no se pueden comparar '{}' y '{}' al ordenar",
                        a.nombre_tipo(),
                        b.nombre_tipo()
                    ),
                )
            });
            Ordering::Equal
        }
    });
    match fallo {
        Some(fallo) => Err(fallo),
        None => Ok(()),
    }
}

fn comparar(a: &Valor, b: &Valor) -> Option<Ordering> {
    match (a, b) {
        (Valor::Entero(a), Valor::Entero(b)) => Some(a.cmp(b)),
        (Valor::Texto(a), Valor::Texto(b)) => Some(a.cmp(b)),
        (Valor::Numero(a), Valor::Numero(b)) => Some(a.cmp(b)),
        (Valor::Entero(a), Valor::Numero(b)) => Some(Decimal::from(*a).cmp(b)),
        (Valor::Numero(a), Valor::Entero(b)) => Some(a.cmp(&Decimal::from(*b))),
        _ => None,
    }
}

/// Máximo o mínimo de una lista de valores comparables.
fn extremo(elementos: &[Valor], objetivo: Ordering) -> Result<Valor, Fallo> {
    let mut mejor: Option<&Valor> = None;
    for elemento in elementos {
        match mejor {
            None => mejor = Some(elemento),
            Some(actual) => {
                let orden = comparar(elemento, actual).ok_or_else(|| {
                    error(
                        "E0406",
                        format!(
                            "no se pueden comparar '{}' y '{}'",
                            elemento.nombre_tipo(),
                            actual.nombre_tipo()
                        ),
                    )
                })?;
                if orden == objetivo {
                    mejor = Some(elemento);
                }
            }
        }
    }
    mejor
        .cloned()
        .ok_or_else(|| error("E0406", "la lista está vacía"))
}
