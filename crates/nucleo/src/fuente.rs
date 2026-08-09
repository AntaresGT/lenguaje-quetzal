//! Representación de un archivo de código fuente Quetzal.

use crate::ubicacion::{Posicion, Ubicacion};

/// Un archivo (o fragmento) de código fuente Quetzal.
#[derive(Debug, Clone)]
pub struct Fuente {
    /// Nombre que se muestra en diagnósticos, por ejemplo `aplicacion/principal.qz`.
    pub nombre: String,
    /// Contenido completo del archivo.
    pub contenido: String,
    /// Offsets de byte donde inicia cada línea (índice 0 = línea 1).
    inicios_de_linea: Vec<usize>,
}

impl Fuente {
    pub fn nueva(nombre: impl Into<String>, contenido: impl Into<String>) -> Self {
        let contenido = contenido.into();
        let mut inicios_de_linea = vec![0];
        for (indice, byte) in contenido.bytes().enumerate() {
            if byte == b'\n' {
                inicios_de_linea.push(indice + 1);
            }
        }
        Self {
            nombre: nombre.into(),
            contenido,
            inicios_de_linea,
        }
    }

    /// Convierte un offset de byte a línea/columna (ambas inician en 1).
    pub fn posicion(&self, offset: usize) -> Posicion {
        let offset = offset.min(self.contenido.len());
        let linea_indice = match self.inicios_de_linea.binary_search(&offset) {
            Ok(indice) => indice,
            Err(indice) => indice.saturating_sub(1),
        };
        let inicio_linea = self.inicios_de_linea[linea_indice];
        let columna = self.contenido[inicio_linea..offset].chars().count() + 1;
        Posicion {
            linea: linea_indice + 1,
            columna,
        }
    }

    /// Devuelve el texto de una línea (iniciando en 1), sin el salto de línea final.
    pub fn linea(&self, numero: usize) -> Option<&str> {
        let indice = numero.checked_sub(1)?;
        let inicio = *self.inicios_de_linea.get(indice)?;
        let fin = self
            .inicios_de_linea
            .get(indice + 1)
            .map(|fin| fin.saturating_sub(1))
            .unwrap_or(self.contenido.len());
        let linea = self.contenido.get(inicio..fin)?;
        Some(linea.strip_suffix('\r').unwrap_or(linea))
    }

    /// Cantidad total de líneas del archivo.
    pub fn total_lineas(&self) -> usize {
        self.inicios_de_linea.len()
    }

    /// Texto cubierto por una ubicación.
    pub fn fragmento(&self, ubicacion: Ubicacion) -> &str {
        self.contenido
            .get(ubicacion.inicio..ubicacion.fin)
            .unwrap_or("")
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn posicion_deberia_calcular_linea_y_columna() {
        let fuente = Fuente::nueva("prueba.qz", "hola\nmundo");
        let posicion = fuente.posicion(5);
        assert_eq!((posicion.linea, posicion.columna), (2, 1));
    }

    #[test]
    fn linea_deberia_devolver_texto_sin_salto() {
        let fuente = Fuente::nueva("prueba.qz", "uno\ndos\ntres");
        assert_eq!(fuente.linea(2), Some("dos"));
    }

    #[test]
    fn posicion_deberia_contar_columnas_en_caracteres_unicode() {
        let fuente = Fuente::nueva("prueba.qz", "año = 1");
        // "año" ocupa 4 bytes pero 3 caracteres; el espacio es la columna 4.
        let offset_espacio = "año".len();
        let posicion = fuente.posicion(offset_espacio);
        assert_eq!(posicion.columna, 4);
    }
}
