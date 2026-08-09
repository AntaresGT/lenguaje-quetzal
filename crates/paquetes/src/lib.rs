//! Gestión de proyectos y dependencias del Lenguaje Quetzal.
//!
//! - `quetzal.json` validado y normalizado ([`Manifiesto`]).
//! - `quetzal nuevo NOMBRE` ([`crear_proyecto`]).
//! - `quetzal.bloquear` con versiones y hashes ([`Bloqueo`]).
//! - Instalación local en `.quetzal/bibliotecas/` ([`instalar_dependencias`]).
//! - Cache de bytecode en `.quetzal/cache/` ([`CacheBytecode`]).

pub mod bloqueo;
pub mod cache;
pub mod instalacion;
pub mod manifiesto;
pub mod permisos;
pub mod proyecto;

pub use bloqueo::{Bloqueo, DependenciaBloqueada};
pub use cache::CacheBytecode;
pub use instalacion::instalar_dependencias;
pub use manifiesto::Manifiesto;
pub use permisos::{Acceso, PermisoRed, Permisos};
pub use proyecto::crear_proyecto;
