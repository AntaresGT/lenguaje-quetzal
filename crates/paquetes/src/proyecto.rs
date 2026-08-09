//! `quetzal nuevo NOMBRE`: genera la estructura de un proyecto.

use std::path::Path;

use nucleo::{CategoriaError, ErrorQuetzal, ResultadoQuetzal};

/// Crea la estructura de un proyecto nuevo:
///
/// ```text
/// NOMBRE/
///   aplicacion/
///     principal.qz
///   quetzal.json
///   Leeme.md
/// ```
pub fn crear_proyecto(nombre: &str) -> ResultadoQuetzal<()> {
    if nombre.is_empty()
        || !nombre
            .chars()
            .all(|caracter| caracter.is_alphanumeric() || caracter == '-' || caracter == '_')
    {
        return Err(ErrorQuetzal::nuevo(
            "E0601",
            CategoriaError::Paquetes,
            format!("'{nombre}' no es un nombre de proyecto válido"),
        )
        .con_ayuda("usa letras, números, guiones y guiones bajos: quetzal nuevo mi-proyecto"));
    }

    let raiz = Path::new(nombre);
    if raiz.exists() {
        return Err(ErrorQuetzal::nuevo(
            "E0601",
            CategoriaError::Paquetes,
            format!("ya existe un directorio llamado '{nombre}'"),
        )
        .con_ayuda("elige otro nombre o elimina el directorio existente"));
    }

    crear_directorio(&raiz.join("aplicacion"))?;
    escribir(
        &raiz.join("aplicacion").join("principal.qz"),
        &format!("// Archivo principal de {nombre}\n\nconsola.mostrar(\"¡Hola desde Quetzal!\")\n"),
    )?;
    escribir(
        &raiz.join("quetzal.json"),
        &format!(
            "{{\n  \"version\": \"0.1.0\",\n  \"aplicacion\": \"{nombre}\",\n  \"entrada\": \"aplicacion/principal.qz\",\n  \"tipo\": \"aplicacion\",\n  \"quetzal\": \"{}\",\n  \"dependencias\": {{}},\n  \"permisos\": {{}}\n}}\n",
            nucleo::VERSION_QUETZAL
        ),
    )?;
    escribir(
        &raiz.join("Leeme.md"),
        &format!(
            "# {nombre}\n\nProyecto creado con `quetzal nuevo {nombre}`.\n\n## Ejecutar\n\n```bash\nquetzal ejecutar\n```\n"
        ),
    )?;
    Ok(())
}

fn crear_directorio(ruta: &Path) -> ResultadoQuetzal<()> {
    std::fs::create_dir_all(ruta).map_err(|error| {
        ErrorQuetzal::nuevo(
            "E0601",
            CategoriaError::Paquetes,
            format!("no se pudo crear '{}': {error}", ruta.display()),
        )
    })
}

fn escribir(ruta: &Path, contenido: &str) -> ResultadoQuetzal<()> {
    std::fs::write(ruta, contenido).map_err(|error| {
        ErrorQuetzal::nuevo(
            "E0601",
            CategoriaError::Paquetes,
            format!("no se pudo escribir '{}': {error}", ruta.display()),
        )
    })
}
