use crate::errores::{CodigoError, Error, Resultado};
use crate::interprete::entorno::Entorno;
use crate::nativos::consola::Consola;
use crate::nativos::interfaz::{DescriptorModuloNativo, ModuloNativo};
use crate::nativos::matematica::Matematica;
use crate::nucleo::sintactico::ast::ElementoImportacion;
use std::sync::Arc;

type CreadorModuloNativo = fn() -> Arc<dyn ModuloNativo>;

fn crear_consola() -> Arc<dyn ModuloNativo> {
    Arc::new(Consola::nuevo())
}

fn crear_matematica() -> Arc<dyn ModuloNativo> {
    Arc::new(Matematica::nuevo())
}

fn creadores_modulos_nativos() -> Vec<CreadorModuloNativo> {
    vec![crear_consola, crear_matematica]
}

pub fn descriptores_modulos_nativos() -> Vec<DescriptorModuloNativo> {
    creadores_modulos_nativos()
        .into_iter()
        .map(|crear| crear().descriptor())
        .collect()
}

pub fn descriptor_modulo_nativo_por_ruta(ruta: &str) -> Option<DescriptorModuloNativo> {
    descriptores_modulos_nativos()
        .into_iter()
        .find(|descriptor| descriptor.rutas.iter().any(|ruta_descriptor| ruta_descriptor == ruta))
}

pub fn obtener_modulo_nativo_por_ruta(ruta: &str) -> Option<Arc<dyn ModuloNativo>> {
    creadores_modulos_nativos()
        .into_iter()
        .find_map(|crear| {
            let modulo = crear();
            let descriptor = modulo.descriptor();
            descriptor
                .rutas
                .iter()
                .any(|ruta_descriptor| ruta_descriptor == ruta)
                .then_some(modulo)
        })
}

/// Registra todos los módulos nativos en el entorno
pub fn registrar_modulos_nativos(entorno: &mut Entorno) -> Resultado<()> {
    for crear in creadores_modulos_nativos() {
        let modulo = crear();
        let descriptor = modulo.descriptor();

        if descriptor.global {
            for nombre in &descriptor.exportaciones {
                entorno.registrar_objeto_nativo(nombre.clone(), modulo.clone());
            }
        }
    }

    registrar_funciones_globales(entorno)?;
    registrar_modulos_importables(entorno);

    Ok(())
}

/// Registra los módulos nativos que pueden ser importados
fn registrar_modulos_importables(entorno: &mut Entorno) {
    for crear in creadores_modulos_nativos() {
        let modulo = crear();
        let descriptor = modulo.descriptor();

        if descriptor.global {
            continue;
        }

        for nombre in &descriptor.exportaciones {
            entorno.registrar_objeto_nativo(nombre.clone(), modulo.clone());
        }
    }
}

/// Registra funciones globales nativas como rango()
fn registrar_funciones_globales(_entorno: &mut Entorno) -> Resultado<()> {
    // Función rango(inicio, fin) - crea una lista de números consecutivos
    // Esta función se implementa como una función especial que se detecta en el intérprete
    // La implementación real está en expresiones.rs cuando se llama a una función
    Ok(())
}

/// Verifica si una ruta corresponde a un módulo nativo
pub fn es_modulo_nativo(ruta: &str) -> bool {
    descriptor_modulo_nativo_por_ruta(ruta).is_some()
}

/// Procesa una importación y registra los elementos en el entorno
pub fn procesar_importacion(
    elementos: &[ElementoImportacion],
    ruta: &str,
    entorno: &mut Entorno,
) -> Resultado<()> {
    let modulo = obtener_modulo_nativo_por_ruta(ruta).ok_or_else(|| {
        Error::modulo(
            CodigoError::ModuloNoEncontrado,
            format!("no existe un módulo nativo en '{}'", ruta),
            Some(ruta.to_string()),
        )
    })?;
    let descriptor = modulo.descriptor();

    for elemento in elementos {
        let nombre_importacion = elemento.alias.as_ref().unwrap_or(&elemento.nombre);

        if descriptor
            .exportaciones
            .iter()
            .any(|exportado| exportado == &elemento.nombre)
        {
            entorno.registrar_objeto_nativo(nombre_importacion.clone(), modulo.clone());
            continue;
        }

        if let Some(constante) = modulo.obtener_constante(&elemento.nombre) {
            entorno
                .definir_variable(nombre_importacion.clone(), constante, false)
                .map_err(|e| {
                    Error::ejecucion(
                        CodigoError::VariableRedeclarada,
                        e,
                        None,
                        None,
                        None,
                    )
                })?;
            continue;
        }

        return Err(Error::modulo(
            CodigoError::ElementoImportadoNoEncontrado,
            format!(
                "'{}' no está exportado por el módulo nativo '{}'",
                elemento.nombre, descriptor.nombre
            ),
            Some(ruta.to_string()),
        ));
    }

    Ok(())
}
