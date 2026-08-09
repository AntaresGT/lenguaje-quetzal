//! Comando `quetzal instalar [PAQUETE]`.

use diagnosticos::colores::Paleta;

/// Instala las dependencias declaradas en `quetzal.json`.
pub fn instalar(paquete: Option<&str>) -> i32 {
    if let Some(nombre) = paquete {
        // Instalar un paquete suelto requiere un registro remoto (fase futura).
        eprintln!(
            "Instalar paquetes por nombre ('{nombre}') requiere el registro remoto y todavía no está disponible."
        );
        eprintln!("Declara la dependencia en quetzal.json y ejecuta 'quetzal instalar'.");
        return 1;
    }

    match paquetes::instalar_dependencias(std::path::Path::new(".")) {
        Ok(instaladas) if instaladas.is_empty() => {
            println!("No hay dependencias que instalar.");
            0
        }
        Ok(instaladas) => {
            for nombre in &instaladas {
                println!("Instalada: {nombre}");
            }
            println!("quetzal.bloquear actualizado.");
            0
        }
        Err(error) => {
            let paleta = Paleta::default();
            eprint!("{}", diagnosticos::reportar(&error, None, &paleta));
            1
        }
    }
}
