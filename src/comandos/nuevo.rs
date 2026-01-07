use crate::errores::{Error, CodigoError, Resultado};
use std::fs;
use std::path::Path;

/// Crea un nuevo proyecto Quetzal
pub fn crear_proyecto(nombre: &str) -> Resultado<()> {
    let ruta_proyecto = Path::new(nombre);
    
    if ruta_proyecto.exists() {
        return Err(Error::sistema(
            CodigoError::ErrorEscrituraArchivo,
            format!("el directorio '{}' ya existe", nombre),
            None,
        ));
    }
    
    // Crear directorio principal
    fs::create_dir_all(ruta_proyecto)
        .map_err(|e| Error::sistema(
            CodigoError::ErrorEscrituraArchivo,
            format!("no se pudo crear el directorio: {}", e),
            None,
        ))?;
    
    // Crear archivo principal
    let contenido_principal = format!("// Archivo principal de {}\n\nentero var contador = 0\n\nconsola.mostrar(\"¡Hola desde Quetzal!\")\n", nombre);
    fs::write(ruta_proyecto.join("principal.qz"), contenido_principal)
        .map_err(|e| Error::sistema(
            CodigoError::ErrorEscrituraArchivo,
            format!("no se pudo crear el archivo principal: {}", e),
            None,
        ))?;
    
    // Crear quetzal.json
    let contenido_config = r#"{
    "versión": "0.1.0",
    "aplicación": "PROYECTO_AQUI",
    "dependencias": {}
}
"#.replace("PROYECTO_AQUI", nombre);
    fs::write(ruta_proyecto.join("quetzal.json"), contenido_config)
        .map_err(|e| Error::sistema(
            CodigoError::ErrorEscrituraArchivo,
            format!("no se pudo crear quetzal.json: {}", e),
            None,
        ))?;
    
    // Crear README.md
    let contenido_readme = format!("# {}\n\nProyecto escrito en Lenguaje Quetzal.\n", nombre);
    fs::write(ruta_proyecto.join("README.md"), contenido_readme)
        .map_err(|e| Error::sistema(
            CodigoError::ErrorEscrituraArchivo,
            format!("no se pudo crear README.md: {}", e),
            None,
        ))?;
    
    Ok(())
}
