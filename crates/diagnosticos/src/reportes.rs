//! Construcción de reportes de error con estilo parecido a Rust.

use nucleo::{ErrorQuetzal, Fuente};

use crate::colores::Paleta;

/// Genera el reporte completo de un error sobre su archivo fuente.
///
/// Formato:
///
/// ```text
/// error[E0007]: no puedes reasignar una variable constante
///   --> aplicacion/principal.qz:3:1
///    |
///  3 | edad = 31
///    | ^^^^ esta variable fue declarada como constante
///    |
/// ayuda: declara la variable como mutable usando 'var'
/// ```
pub fn reportar(error: &ErrorQuetzal, fuente: Option<&Fuente>, paleta: &Paleta) -> String {
    let mut salida = String::new();

    let encabezado = format!("error[{}]", error.codigo);
    salida.push_str(&paleta.error(&encabezado));
    salida.push_str(&paleta.enfasis(&format!(": {}", error.mensaje)));
    salida.push('\n');

    if let (Some(fuente), Some(ubicacion)) = (fuente, error.ubicacion) {
        let posicion = fuente.posicion(ubicacion.inicio);
        let nombre_archivo = error.archivo.as_deref().unwrap_or(&fuente.nombre);
        salida.push_str(&format!(
            "  {} {}:{}:{}\n",
            paleta.marco("-->"),
            nombre_archivo,
            posicion.linea,
            posicion.columna
        ));

        if let Some(texto_linea) = fuente.linea(posicion.linea) {
            let numero = posicion.linea.to_string();
            let margen = " ".repeat(numero.len());

            salida.push_str(&format!("{} {}\n", margen, paleta.marco("|")));
            salida.push_str(&format!(
                "{} {} {}\n",
                paleta.marco(&numero),
                paleta.marco("|"),
                texto_linea
            ));

            // Carets bajo el fragmento con problema.
            let fin = ubicacion.fin.min(fuente.contenido.len());
            let posicion_fin = fuente.posicion(fin);
            let ancho = if posicion_fin.linea == posicion.linea {
                (posicion_fin.columna.saturating_sub(posicion.columna)).max(1)
            } else {
                texto_linea
                    .chars()
                    .count()
                    .saturating_sub(posicion.columna - 1)
                    .max(1)
            };
            let relleno = " ".repeat(posicion.columna.saturating_sub(1));
            let carets = "^".repeat(ancho);
            let etiqueta = error.etiqueta.as_deref().unwrap_or("");
            let linea_caret = if etiqueta.is_empty() {
                format!("{relleno}{carets}")
            } else {
                format!("{relleno}{carets} {etiqueta}")
            };
            salida.push_str(&format!(
                "{} {} {}\n",
                margen,
                paleta.marco("|"),
                paleta.error(&linea_caret)
            ));
            salida.push_str(&format!("{} {}\n", margen, paleta.marco("|")));
        }
    } else if let Some(archivo) = &error.archivo {
        salida.push_str(&format!("  {} {}\n", paleta.marco("-->"), archivo));
    }

    if let Some(ayuda) = &error.ayuda {
        salida.push_str(&format!("{}: {}\n", paleta.ayuda("ayuda"), ayuda));
    }

    salida
}

/// Reporta varios errores separados por una línea en blanco.
pub fn reportar_varios(
    errores: &[ErrorQuetzal],
    fuente: Option<&Fuente>,
    paleta: &Paleta,
) -> String {
    errores
        .iter()
        .map(|error| reportar(error, fuente, paleta))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use nucleo::{CategoriaError, Ubicacion};

    #[test]
    fn reportar_deberia_incluir_codigo_archivo_y_ayuda() {
        let fuente = Fuente::nueva("aplicacion/principal.qz", "entero edad = 30\n\nedad = 31\n");
        let inicio = fuente.contenido.find("edad = 31").unwrap_or(0);
        let error = ErrorQuetzal::nuevo(
            "E0203",
            CategoriaError::Semantico,
            "no puedes reasignar una variable constante",
        )
        .con_ubicacion(Ubicacion::nueva(inicio, inicio + 4))
        .con_etiqueta("esta variable fue declarada como constante")
        .con_ayuda("declara la variable como mutable usando 'var'");

        let paleta = Paleta::nueva(false);
        let reporte = reportar(&error, Some(&fuente), &paleta);

        assert!(reporte.contains("error[E0203]"));
        assert!(reporte.contains("aplicacion/principal.qz:3:1"));
        assert!(reporte.contains("^^^^ esta variable fue declarada como constante"));
        assert!(reporte.contains("ayuda: declara la variable como mutable usando 'var'"));
    }
}
