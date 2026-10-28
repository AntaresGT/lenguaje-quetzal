//! Verificador de bytecode: rechaza módulos corruptos (cache, disco) y
//! calcula `max_pila` por función.

use nucleo::{CategoriaError, ErrorQuetzal};

use crate::instruccion::{CodigoOp, Instruccion, Trozo};
use crate::modulo::ModuloCompilado;

/// Verifica un módulo y rellena `max_pila` de cada trozo.
pub fn verificar_modulo(modulo: &mut ModuloCompilado) -> Result<(), ErrorQuetzal> {
    let limites = Limites {
        constantes: modulo.constantes.len(),
        nombres: modulo.nombres.len(),
        globales: modulo.nombres_globales.len(),
    };
    modulo.principal.max_pila = verificar_trozo(
        &modulo.principal,
        limites,
        modulo.ranuras_locales_principal,
        "principal",
    )?;
    for funcion in &mut modulo.funciones {
        funcion.trozo.max_pila = verificar_trozo(
            &funcion.trozo,
            limites,
            funcion.ranuras_locales,
            &funcion.nombre,
        )?;
    }
    for objeto in &mut modulo.objetos {
        for metodo in &mut objeto.metodos {
            metodo.funcion.trozo.max_pila = verificar_trozo(
                &metodo.funcion.trozo,
                limites,
                metodo.funcion.ranuras_locales,
                &metodo.funcion.nombre,
            )?;
        }
        for constructor in &mut objeto.constructores {
            constructor.trozo.max_pila = verificar_trozo(
                &constructor.trozo,
                limites,
                constructor.ranuras_locales,
                &constructor.nombre,
            )?;
        }
        for atributo in &mut objeto.atributos {
            if let Some(inicial) = &mut atributo.inicial {
                inicial.max_pila = verificar_trozo(inicial, limites, 0, &atributo.nombre)?;
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Limites {
    constantes: usize,
    nombres: usize,
    globales: usize,
}

fn verificar_trozo(
    trozo: &Trozo,
    limites: Limites,
    ranuras: u16,
    nombre: &str,
) -> Result<u16, ErrorQuetzal> {
    let n = trozo.instrucciones.len();
    if trozo.ubicaciones.len() != n {
        return Err(error_verificacion(
            nombre,
            "el trozo tiene más instrucciones que ubicaciones",
        ));
    }
    let mut profundidad = 0i32;
    let mut maxima = 0i32;
    for (pc, instruccion) in trozo.instrucciones.iter().copied().enumerate() {
        if let Some(destino) = instruccion.destino_salto()
            && destino as usize > n
        {
            return Err(error_verificacion(
                nombre,
                format!("salto a {destino} fuera del trozo ({n} instrucciones)"),
            ));
        }
        validar_indices(instruccion, limites, ranuras, nombre)?;
        profundidad += efecto_pila(instruccion);
        if profundidad < 0 {
            return Err(error_verificacion(
                nombre,
                format!("la pila quedó negativa en la instrucción {pc}"),
            ));
        }
        maxima = maxima.max(profundidad);
    }
    u16::try_from(maxima).map_err(|_| error_verificacion(nombre, "la pila supera 65535 valores"))
}

fn validar_indices(
    instruccion: Instruccion,
    limites: Limites,
    ranuras: u16,
    nombre: &str,
) -> Result<(), ErrorQuetzal> {
    let constantes = limites.constantes;
    let nombres = limites.nombres;
    let globales = limites.globales;
    match instruccion.op {
        CodigoOp::CargarConstante if instruccion.indice_u32() as usize >= constantes => Err(
            error_verificacion(nombre, "índice de constante fuera de rango"),
        ),
        CodigoOp::CargarLocal
        | CodigoOp::GuardarLocal
        | CodigoOp::AjustarLocal
        | CodigoOp::RetornarLocal
            if instruccion.ranura() >= ranuras =>
        {
            Err(error_verificacion(
                nombre,
                "ranura local fuera de rango",
            ))
        }
        CodigoOp::CargarLocalLocalSumarEntero
            if instruccion.ranura() >= ranuras || instruccion.ranura_derecha() >= ranuras =>
        {
            Err(error_verificacion(
                nombre,
                "ranura local fuera de rango",
            ))
        }
        CodigoOp::LimpiarLocales if instruccion.ranura() > ranuras => Err(error_verificacion(
            nombre,
            "Limpieza de locales fuera de rango",
        )),
        CodigoOp::CargarGlobal | CodigoOp::DeclararGlobal | CodigoOp::GuardarGlobal
            if instruccion.indice_u16() as usize >= globales =>
        {
            Err(error_verificacion(nombre, "índice de global fuera de rango"))
        }
        CodigoOp::CargarExterno
        | CodigoOp::GuardarExterno
        | CodigoOp::CargarMiembro
        | CodigoOp::GuardarMiembro
        | CodigoOp::LlamarMetodo
            if instruccion.indice_nombre() as usize >= nombres
                && instruccion.op != CodigoOp::LlamarMetodo
                || (instruccion.op == CodigoOp::LlamarMetodo
                    && instruccion.indice_nombre() as usize >= nombres) =>
        {
            Err(error_verificacion(nombre, "índice de nombre fuera de rango"))
        }
        _ => Ok(()),
    }
}

fn efecto_pila(instruccion: Instruccion) -> i32 {
    match instruccion.op {
        CodigoOp::CargarConstante
        | CodigoOp::CargarEntero
        | CodigoOp::CargarLocal
        | CodigoOp::CargarGlobal
        | CodigoOp::CargarExterno
        | CodigoOp::CargarEsto
        | CodigoOp::CargarPadre
        | CodigoOp::CargarEstoAtributo
        | CodigoOp::Duplicar
        | CodigoOp::CargarLocalLocalSumarEntero => 1,
        CodigoOp::Desechar
        | CodigoOp::GuardarLocal
        | CodigoOp::GuardarGlobal
        | CodigoOp::GuardarExterno
        | CodigoOp::SaltarSiFalso
        | CodigoOp::SaltarSiVerdadero
        | CodigoOp::Lanzar => -1,
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
        | CodigoOp::CargarMiembro
        | CodigoOp::CargarAtributo
        | CodigoOp::Indexar
        | CodigoOp::CrearIterador
        | CodigoOp::Esperar
        | CodigoOp::Negar
        | CodigoOp::NegarEntero
        | CodigoOp::NoLogico => 0,
        CodigoOp::GuardarMiembro | CodigoOp::GuardarAtributo | CodigoOp::MenorEnteroSaltarSiFalso => {
            -2
        }
        CodigoOp::GuardarIndice => -3,
        CodigoOp::Llamar | CodigoOp::Instanciar => -(instruccion.cantidad() as i32),
        CodigoOp::LlamarMetodo => -(instruccion.cantidad_metodo() as i32),
        CodigoOp::LlamarNativa => 1 - (instruccion.cantidad() as i32),
        CodigoOp::CrearLista | CodigoOp::Interpolar => 1 - (instruccion.cantidad() as i32),
        CodigoOp::CrearJsn => 1 - (instruccion.cantidad() as i32 * 2),
        CodigoOp::Saltar
        | CodigoOp::SaltarSiFalsoYDejar
        | CodigoOp::SaltarSiVerdaderoYDejar
        | CodigoOp::LimpiarLocales
        | CodigoOp::AjustarLocal
        | CodigoOp::Retornar
        | CodigoOp::RetornarLocal
        | CodigoOp::EntrarIntentar
        | CodigoOp::SalirIntentar
        | CodigoOp::IteradorSiguiente
        | CodigoOp::DeclararGlobal => 0,
    }
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
    use crate::instruccion::Trozo;
    use nucleo::Ubicacion;

    #[test]
    fn verificar_deberia_rechazar_salto_fuera_de_rango() {
        let mut modulo = ModuloCompilado {
            principal: Trozo {
                instrucciones: vec![Instruccion::Saltar(99)],
                ubicaciones: vec![Ubicacion::nueva(0, 1)],
                max_pila: 0,
            },
            ..Default::default()
        };
        assert!(verificar_modulo(&mut modulo).is_err());
    }
}
