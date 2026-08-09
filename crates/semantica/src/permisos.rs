//! Validación estática de permisos.
//!
//! **Estado: futuro (fase 10).** Aquí se podrá advertir en análisis cuando un
//! módulo importe `quetzal/red` o `quetzal/sistema_archivos` sin que el
//! `quetzal.json` del proyecto declare los permisos correspondientes. La
//! aplicación real de permisos ocurre en el crate `runtime`.
