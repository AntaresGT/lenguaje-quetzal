//! Runtime del Lenguaje Quetzal: permisos y acceso al sistema.
//!
//! El modelo es seguro por defecto: sin un `quetzal.json` que habilite cada
//! permiso, el programa no puede usar la red, leer o escribir archivos ni
//! ejecutar programas externos.

pub mod permisos;

pub use permisos::{AccesoSolicitado, GuardianPermisos};
