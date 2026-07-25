//! Módulo nativo `quetzal/bits` (vacío por diseño).
//!
//! Las funciones anteriores se retiraron; el módulo sigue registrado para que
//! las importaciones `desde "quetzal/bits"` sigan siendo reconocidas.

use maquina_virtual::RegistroNativos;

/// Registra el módulo `bits` sin exponer ninguna función.
pub fn registrar(_registro: &mut RegistroNativos) {}