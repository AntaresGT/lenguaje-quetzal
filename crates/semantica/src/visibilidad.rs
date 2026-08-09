//! Reglas de visibilidad de miembros.
//!
//! - `publico:` y `privado:` etiquetan secciones dentro de objetos y prototipos.
//! - Sin etiqueta, los miembros son públicos por defecto.
//! - Un miembro privado solo es accesible desde el propio objeto (incluidos
//!   sus descendientes por herencia).
//!
//! La aplicación de estas reglas vive en `analizador` (error `E0207`).

use ast::Visibilidad;

/// Si un miembro con la visibilidad dada puede accederse desde fuera del objeto.
pub fn accesible_desde_fuera(visibilidad: Visibilidad) -> bool {
    visibilidad == Visibilidad::Publico
}
