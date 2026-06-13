//! Métodos nativos del tipo `jsn` según `ejemplos/metodos_json.qz`.

use std::cell::RefCell;
use std::rc::Rc;

use indexmap::IndexMap;
use maquina_virtual::valores::jsn_a_texto;
use maquina_virtual::{Fallo, RegistroNativos, Valor};

use crate::util::{arg_jsn, arg_texto, exigir_aridad};

pub fn registrar(registro: &mut RegistroNativos) {
    metodo(registro, "jsn.contiene_clave", 1, |mapa, argumentos| {
        let clave = arg_texto("jsn.contiene_clave", argumentos, 1)?;
        Ok(Valor::Log(mapa.borrow().contains_key(clave)))
    });
    metodo(registro, "jsn.claves", 0, |mapa, _| {
        Ok(Valor::lista(
            mapa.borrow().keys().map(Valor::texto).collect(),
        ))
    });
    metodo(registro, "jsn.valores", 0, |mapa, _| {
        Ok(Valor::lista(mapa.borrow().values().cloned().collect()))
    });
    metodo(registro, "jsn.establecer", 2, |mapa, argumentos| {
        let clave = arg_texto("jsn.establecer", argumentos, 1)?;
        mapa.borrow_mut()
            .insert(clave.to_string(), argumentos[2].clone());
        Ok(Valor::Nulo)
    });
    metodo(registro, "jsn.eliminar", 1, |mapa, argumentos| {
        let clave = arg_texto("jsn.eliminar", argumentos, 1)?;
        Ok(mapa.borrow_mut().shift_remove(clave).unwrap_or(Valor::Nulo))
    });
    metodo(registro, "jsn.fusionar", 1, |mapa, argumentos| {
        let otro = arg_jsn("jsn.fusionar", argumentos, 1)?;
        let entradas: Vec<(String, Valor)> = otro
            .borrow()
            .iter()
            .map(|(clave, valor)| (clave.clone(), valor.clone()))
            .collect();
        mapa.borrow_mut().extend(entradas);
        Ok(Valor::Nulo)
    });
    metodo(registro, "jsn.texto", 0, |mapa, _| {
        Ok(Valor::texto(jsn_a_texto(&Valor::Jsn(mapa), false)))
    });
    metodo(registro, "jsn.texto_formateado", 0, |mapa, _| {
        Ok(Valor::texto(jsn_a_texto(&Valor::Jsn(mapa), true)))
    });
    metodo(registro, "jsn.longitud", 0, |mapa, _| {
        Ok(Valor::Entero(mapa.borrow().len() as i64))
    });
    metodo(registro, "jsn.logico", 0, |mapa, _| {
        Ok(Valor::Log(!mapa.borrow().is_empty()))
    });
}

/// Registra un método de jsn verificando receptor y aridad.
fn metodo(
    registro: &mut RegistroNativos,
    nombre: &'static str,
    argumentos_extra: usize,
    implementacion: impl Fn(Rc<RefCell<IndexMap<String, Valor>>>, &[Valor]) -> Result<Valor, Fallo>
    + 'static,
) {
    registro.registrar_funcion(
        nombre,
        Box::new(move |argumentos| {
            exigir_aridad(nombre, argumentos, argumentos_extra + 1)?;
            let receptor = arg_jsn(nombre, argumentos, 0)?;
            implementacion(receptor, argumentos)
        }),
    );
}
