//! Módulo nativo `quetzal/sistema_archivos` (vacío por diseño).
//!
//! Las funciones anteriores se retiraron; el módulo sigue registrado para que
//! las importaciones `desde "quetzal/sistema_archivos"` sigan siendo reconocidas.

use std::rc::Rc;

use maquina_virtual::RegistroNativos;
use runtime::GuardianPermisos;

/// Registra el módulo `sistema_archivos` sin exponer ninguna función.
pub fn registrar(_registro: &mut RegistroNativos, _guardian: &Rc<GuardianPermisos>) {}