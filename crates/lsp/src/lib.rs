//! Servidor de lenguaje oficial del Lenguaje Quetzal.
//!
//! Analiza código fuente sin ejecutar programas Quetzal ni depender de la
//! máquina virtual, del JIT o de la generación de ejecutables.
#![forbid(unsafe_code)]

pub mod analisis;
pub mod documentos;
pub mod servicios;
pub mod servidor;
