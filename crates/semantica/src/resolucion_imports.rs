//! Resolución de orígenes de importación.
//!
//! Orden de resolución del lenguaje:
//! 1. Módulos nativos: `quetzal/*`
//! 2. Rutas relativas: `./archivo.qz`, `../archivo.qz`, `archivo.qz`
//! 3. Dependencias declaradas en `quetzal.json`
//! 4. Error claro si no existe

use lexico::normalizacion::quitar_tildes;

/// Clase de origen de una importación.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrigenImportacion {
    /// `quetzal/matematica` (normalizado sin tildes).
    Nativo(String),
    /// `./calculadora.qz`
    RutaRelativa(String),
    /// Nombre de dependencia declarada en quetzal.json.
    Dependencia(String),
}

/// Módulos nativos disponibles (nombres normalizados sin tilde).
pub const MODULOS_NATIVOS: &[&str] = &[
    "matematica",
    "texto",
    "listas",
    "jsn",
    "tiempo",
    "red",
    "sistema_archivos",
    "bits",
];

/// Clasifica el texto de un `desde "..."`.
pub fn clasificar_origen(origen: &str) -> Option<OrigenImportacion> {
    if let Some(nombre) = origen.strip_prefix("quetzal/") {
        let normalizado = quitar_tildes(nombre).replace('-', "_");
        if MODULOS_NATIVOS.contains(&normalizado.as_str()) {
            return Some(OrigenImportacion::Nativo(normalizado));
        }
        return None;
    }
    if origen.ends_with(".qz") {
        return Some(OrigenImportacion::RutaRelativa(origen.to_string()));
    }
    Some(OrigenImportacion::Dependencia(origen.to_string()))
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn clasificar_origen_deberia_aceptar_nativo_con_tilde() {
        assert_eq!(
            clasificar_origen("quetzal/matemática"),
            Some(OrigenImportacion::Nativo("matematica".to_string()))
        );
    }

    #[test]
    fn clasificar_origen_deberia_rechazar_nativo_inexistente() {
        assert_eq!(clasificar_origen("quetzal/inexistente"), None);
    }

    #[test]
    fn clasificar_origen_deberia_detectar_rutas_relativas() {
        assert_eq!(
            clasificar_origen("./calculadora.qz"),
            Some(OrigenImportacion::RutaRelativa(
                "./calculadora.qz".to_string()
            ))
        );
    }
}
