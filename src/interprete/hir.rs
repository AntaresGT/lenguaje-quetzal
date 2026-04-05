use crate::errores::Resultado;
use crate::interprete::declaraciones::evaluar_declaracion;
use crate::interprete::entorno::Entorno;
use crate::nucleo::hir::{ItemHir, ProgramaHir};

/// Ejecuta un programa HIR reutilizando el intérprete actual de declaraciones.
pub fn ejecutar_programa_hir(programa: &ProgramaHir, entorno: &mut Entorno) -> Resultado<()> {
    for item in &programa.items {
        ejecutar_item_hir(item, entorno)?;
    }

    Ok(())
}

/// Ejecuta un ítem HIR individual.
pub fn ejecutar_item_hir(item: &ItemHir, entorno: &mut Entorno) -> Resultado<()> {
    if let Some(nodo) = item.como_nodo_ast() {
        evaluar_declaracion(nodo, entorno)?;
    }

    Ok(())
}
