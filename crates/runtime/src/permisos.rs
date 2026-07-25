//! Guardián de permisos del runtime (seguro por defecto).
//!
//! Los módulos nativos peligrosos (`red`, `sistema_archivos`) consultan este
//! guardián antes de cada operación. Sin un `quetzal.json` que habilite el
//! permiso correspondiente, toda operación se rechaza con un mensaje claro.

use std::cell::RefCell;
use std::path::Path;

use paquetes::Permisos;

/// Permisos efectivos del programa en ejecución.
///
/// Se crea siempre denegando todo y se configura después de leer el
/// `quetzal.json` del proyecto (si existe). Usa mutabilidad interior porque
/// el registro de nativos captura el guardián antes de conocer el proyecto.
#[derive(Default)]
pub struct GuardianPermisos {
    estado: RefCell<Estado>,
}

#[derive(Default)]
struct Estado {
    permisos: Permisos,
    /// Raíz del proyecto (reservada para futuros permisos de rutas).
    #[allow(dead_code)]
    raiz: Option<std::path::PathBuf>,
}

impl GuardianPermisos {
    /// Guardián que deniega todo (sin proyecto / sin permisos declarados).
    pub fn denegado() -> Self {
        Self::default()
    }

    /// Aplica los permisos declarados en el `quetzal.json` del proyecto.
    pub fn configurar(&self, permisos: Permisos, raiz: &Path) {
        let mut estado = self.estado.borrow_mut();
        estado.permisos = permisos;
        estado.raiz = Some(raiz.to_path_buf());
    }

    /// Verifica que un ejecutable esté en la lista blanca de `ejecucion`.
    pub fn verificar_ejecucion(&self, programa: &str) -> Result<(), String> {
        let estado = self.estado.borrow();
        let permiso = &estado.permisos.ejecucion;
        if permiso.habilitado
            && permiso
                .ejecutables
                .iter()
                .any(|permitido| permitido == "*" || permitido == programa)
        {
            return Ok(());
        }
        Err(format!(
            "el programa no tiene permiso para ejecutar '{programa}'; agrégalo en quetzal.json: \
             \"permisos\": {{\"ejecucion\": {{\"habilitado\": true, \"ejecutables\": [\"{programa}\"]}}}}"
        ))
    }
}
