//! Patrones de ruta del enrutador de `quetzal/red`.
//!
//! Siguen el estilo de Express 5: segmentos literales, parámetros con `:`,
//! segmentos opcionales entre llaves y comodines con `*`.
//!
//! ```text
//! /usuarios/:id           → /usuarios/7          (parámetros: id = "7")
//! /archivos/*ruta         → /archivos/a/b.txt    (parámetros: ruta = "a/b.txt")
//! /libros{/:isbn}         → /libros y /libros/123
//! ```

/// Un segmento del patrón.
#[derive(Debug, Clone)]
enum Segmento {
    Literal(String),
    Parametro { nombre: String, opcional: bool },
    /// Comodín: consume todo lo que queda de la ruta.
    Comodin(String),
    /// Segmento literal opcional (`/libros{/detalle}`).
    LiteralOpcional(String),
}

/// Patrón de ruta ya analizado.
#[derive(Debug, Clone)]
pub(crate) struct Patron {
    segmentos: Vec<Segmento>,
}

/// Resultado de una coincidencia.
pub(crate) struct Coincidencia {
    pub parametros: Vec<(String, String)>,
    /// Cuántos segmentos de la ruta consumió el patrón.
    pub consumidos: usize,
}

impl Patron {
    /// Analiza un camino escrito por el programa.
    pub fn analizar(camino: &str) -> Patron {
        let limpio = camino.trim();
        Patron {
            segmentos: limpio
                .split('/')
                .filter(|parte| !parte.is_empty())
                .map(analizar_segmento)
                .collect(),
        }
    }

    /// Compara el patrón contra los segmentos de la ruta.
    ///
    /// Con `prefijo` en `true` (capas registradas con `usar`) basta con que
    /// el patrón coincida al principio; con `false` la ruta debe consumirse
    /// por completo.
    pub fn coincide(&self, ruta: &[String], prefijo: bool) -> Option<Coincidencia> {
        let mut parametros = Vec::new();
        let consumidos = self.coincidir_desde(0, ruta, 0, prefijo, &mut parametros)?;
        Some(Coincidencia {
            parametros,
            consumidos,
        })
    }

    fn coincidir_desde(
        &self,
        indice_patron: usize,
        ruta: &[String],
        indice_ruta: usize,
        prefijo: bool,
        parametros: &mut Vec<(String, String)>,
    ) -> Option<usize> {
        if indice_patron >= self.segmentos.len() {
            return if prefijo || indice_ruta == ruta.len() {
                Some(indice_ruta)
            } else {
                None
            };
        }

        match &self.segmentos[indice_patron] {
            Segmento::Comodin(nombre) => {
                let resto = ruta[indice_ruta..].join("/");
                parametros.push((nombre.clone(), resto));
                Some(ruta.len())
            }
            Segmento::Literal(texto) => {
                if indice_ruta < ruta.len() && ruta[indice_ruta].eq_ignore_ascii_case(texto) {
                    self.coincidir_desde(
                        indice_patron + 1,
                        ruta,
                        indice_ruta + 1,
                        prefijo,
                        parametros,
                    )
                } else {
                    None
                }
            }
            Segmento::LiteralOpcional(texto) => {
                if indice_ruta < ruta.len() && ruta[indice_ruta].eq_ignore_ascii_case(texto) {
                    let marca = parametros.len();
                    if let Some(consumidos) = self.coincidir_desde(
                        indice_patron + 1,
                        ruta,
                        indice_ruta + 1,
                        prefijo,
                        parametros,
                    ) {
                        return Some(consumidos);
                    }
                    parametros.truncate(marca);
                }
                self.coincidir_desde(indice_patron + 1, ruta, indice_ruta, prefijo, parametros)
            }
            Segmento::Parametro { nombre, opcional } => {
                if indice_ruta < ruta.len() {
                    let marca = parametros.len();
                    parametros.push((nombre.clone(), ruta[indice_ruta].clone()));
                    if let Some(consumidos) = self.coincidir_desde(
                        indice_patron + 1,
                        ruta,
                        indice_ruta + 1,
                        prefijo,
                        parametros,
                    ) {
                        return Some(consumidos);
                    }
                    parametros.truncate(marca);
                }
                if *opcional {
                    return self.coincidir_desde(
                        indice_patron + 1,
                        ruta,
                        indice_ruta,
                        prefijo,
                        parametros,
                    );
                }
                None
            }
        }
    }
}

fn analizar_segmento(parte: &str) -> Segmento {
    // `{...}` marca un segmento opcional; dentro puede ir literal o parámetro.
    if parte.starts_with('{') && parte.ends_with('}') && parte.len() >= 2 {
        let interior = parte[1..parte.len() - 1].trim_start_matches('/');
        return match analizar_segmento(interior) {
            Segmento::Literal(texto) => Segmento::LiteralOpcional(texto),
            Segmento::Parametro { nombre, .. } => Segmento::Parametro {
                nombre,
                opcional: true,
            },
            otro => otro,
        };
    }
    if let Some(nombre) = parte.strip_prefix('*') {
        let nombre = if nombre.is_empty() { "comodin" } else { nombre };
        return Segmento::Comodin(nombre.to_string());
    }
    if let Some(nombre) = parte.strip_prefix(':') {
        let (nombre, opcional) = match nombre.strip_suffix('?') {
            Some(sin_signo) => (sin_signo, true),
            None => (nombre, false),
        };
        return Segmento::Parametro {
            nombre: nombre.to_string(),
            opcional,
        };
    }
    Segmento::Literal(parte.to_string())
}

/// Divide una ruta en segmentos no vacíos.
pub(crate) fn segmentos_de(ruta: &str) -> Vec<String> {
    ruta.split('/')
        .filter(|parte| !parte.is_empty())
        .map(|parte| parte.to_string())
        .collect()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn coincide(patron: &str, ruta: &str) -> Option<Vec<(String, String)>> {
        Patron::analizar(patron)
            .coincide(&segmentos_de(ruta), false)
            .map(|coincidencia| coincidencia.parametros)
    }

    #[test]
    fn ruta_literal_coincide_exacta() {
        assert!(coincide("/usuarios", "/usuarios").is_some());
        assert!(coincide("/usuarios", "/usuarios/7").is_none());
    }

    #[test]
    fn parametro_captura_el_segmento() {
        let parametros = coincide("/usuarios/:id", "/usuarios/7").expect("debe coincidir");
        assert_eq!(parametros, vec![("id".to_string(), "7".to_string())]);
    }

    #[test]
    fn comodin_captura_el_resto() {
        let parametros =
            coincide("/archivos/*ruta", "/archivos/a/b.txt").expect("debe coincidir");
        assert_eq!(parametros, vec![("ruta".to_string(), "a/b.txt".to_string())]);
    }

    #[test]
    fn segmento_opcional_coincide_con_y_sin_el() {
        assert!(coincide("/libros/{:isbn}", "/libros").is_some());
        assert!(coincide("/libros/{:isbn}", "/libros/123").is_some());
    }

    #[test]
    fn prefijo_permite_ruta_mas_larga() {
        let coincidencia = Patron::analizar("/api")
            .coincide(&segmentos_de("/api/usuarios"), true)
            .expect("debe coincidir como prefijo");
        assert_eq!(coincidencia.consumidos, 1);
    }
}
