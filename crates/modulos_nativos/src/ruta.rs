//! Módulo nativo `quetzal/sistema_archivos`: el objeto `Ruta` (vacío por diseño).
//!
//! Las funciones anteriores se retiraron; el módulo sigue registrado para que
//! las importaciones sigan siendo reconocidas.

use std::rc::Rc;

use maquina_virtual::RegistroNativos;
use runtime::GuardianPermisos;

/// Registra el módulo `ruta` sin exponer ninguna función.
pub fn registrar(_registro: &mut RegistroNativos, _guardian: &Rc<GuardianPermisos>) {}