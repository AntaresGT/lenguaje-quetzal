//! Comandos de la CLI `quetzal`.

pub mod cache;
pub mod ejecutar;
pub mod instalar;
pub mod nuevo;
pub mod repl;
pub mod revisar;
pub mod version;

/// Muestra la ayuda de la CLI en español.
pub fn ayuda() -> i32 {
    println!(
        "\
Uso: quetzal [COMANDO] [ARCHIVO]

Comandos:
  ejecutar [ARCHIVO]      Ejecuta un archivo o proyecto Quetzal
  revisar [ARCHIVO]       Analiza el código sin ejecutarlo
  nuevo NOMBRE            Crea un nuevo proyecto
  instalar [PAQUETE]      Instala dependencias
  version                 Muestra la versión
  ayuda                   Muestra esta ayuda

Aliases:
  quetzal archivo.qz      Ejecuta archivo.qz
  quetzal                 Inicia REPL si no hay archivo ni quetzal.json
  quetzal --version       Muestra versión
  quetzal --versión       Muestra versión
  quetzal --nuevo NOMBRE  Crea proyecto

Ejemplos:
  quetzal programa.qz
  quetzal nuevo mi-proyecto
  quetzal (modo REPL)
  quetzal instalar [URL|Nombre de la biblioteca]"
    );
    0
}
