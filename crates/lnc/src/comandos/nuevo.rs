//! Comando `quetzal nuevo NOMBRE`.

use diagnosticos::colores::Paleta;

/// Crea un proyecto Quetzal nuevo.
pub fn nuevo(nombre: &str) -> i32 {
    match paquetes::crear_proyecto(nombre) {
        Ok(()) => {
            println!("Proyecto '{nombre}' creado.");
            println!("  {nombre}/aplicacion/principal.qz");
            println!("  {nombre}/quetzal.json");
            println!("  {nombre}/Leeme.md");
            println!();
            println!("Para ejecutarlo: cd {nombre} && quetzal ejecutar");
            0
        }
        Err(error) => {
            let paleta = Paleta::default();
            eprint!("{}", diagnosticos::reportar(&error, None, &paleta));
            1
        }
    }
}
