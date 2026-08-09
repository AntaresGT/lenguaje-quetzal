//! Comando `quetzal version`.

use motor::VERSION_QUETZAL;

/// Muestra la versión actual del Lenguaje Quetzal.
pub fn version() -> i32 {
    println!("Quetzal v{VERSION_QUETZAL}");
    0
}
