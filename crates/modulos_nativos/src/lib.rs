//! Módulos nativos del Lenguaje Quetzal.
//!
//! Llena el [`RegistroNativos`] de la VM con las funciones del lenguaje:
//! `consola` (global), `matematica`, `tiempo`, `motor` (utilería: el objeto
//! `ExpresiónRegular`), los métodos de `texto`, `lista` y `jsn`, las
//! conversiones de tipos y la función global `rango`.

pub mod bits;
pub mod consola;
pub mod conversiones;
pub mod jsn;
pub mod listas;
pub mod matematica;
pub mod motor;
pub mod red;
pub mod ruta;
pub mod sistema_archivos;
pub mod texto;
pub mod tiempo;
mod util;

use std::rc::Rc;

use maquina_virtual::RegistroNativos;
use runtime::GuardianPermisos;

/// Crea el registro con todos los módulos nativos, aplicando el guardián de
/// permisos a los módulos peligrosos (`red`, `sistema_archivos`).
pub fn crear_registro_con_permisos(guardian: &Rc<GuardianPermisos>) -> RegistroNativos {
    let mut registro = RegistroNativos::nuevo();
    consola::registrar(&mut registro);
    matematica::registrar(&mut registro);
    texto::registrar(&mut registro);
    listas::registrar(&mut registro);
    jsn::registrar(&mut registro);
    conversiones::registrar(&mut registro);
    tiempo::registrar(&mut registro);
    bits::registrar(&mut registro);
    motor::registrar(&mut registro);
    red::registrar(&mut registro, guardian);
    sistema_archivos::registrar(&mut registro, guardian);
    ruta::registrar(&mut registro, guardian);
    registro
}

/// Tipos instanciables que un módulo nativo exporta además de sí mismo.
///
/// Al importar `{ ExpresiónRegular }` desde `"quetzal/motor"`, el símbolo
/// debe resolverse al módulo nativo `motor` (cuyo `motor.constructor`
/// atiende `nuevo ExpresiónRegular(...)`). Recibe el módulo de origen y el
/// símbolo ya normalizado (minúsculas, sin tildes ni guiones bajos).
pub fn modulo_de_tipo_exportado(modulo: &str, simbolo_normalizado: &str) -> Option<&'static str> {
    match (modulo, simbolo_normalizado) {
        ("motor", "expresionregular") => Some("motor"),
        _ => None,
    }
}

/// Crea el registro con todos los permisos denegados (seguro por defecto).
pub fn crear_registro() -> RegistroNativos {
    crear_registro_con_permisos(&Rc::new(GuardianPermisos::denegado()))
}
