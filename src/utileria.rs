use std::fs;
use std::io::{self, Read};

/// Lee un archivo y devuelve su contenido en una cadena
pub fn leer_archivo(ruta: &str) -> io::Result<String> {
    let mut archivo = fs::File::open(ruta)?;
    let mut contenido = String::new();
    archivo.read_to_string(&mut contenido)?;
    Ok(contenido)
}
