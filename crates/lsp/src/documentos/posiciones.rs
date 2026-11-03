//! Conversión entre offsets UTF-8 de Quetzal y posiciones UTF-16 de LSP.

use lsp_types::{Position, Range};
use nucleo::Ubicacion;

/// Índice de líneas para convertir posiciones sin modificar el texto.
#[derive(Debug)]
pub struct ConvertidorPosiciones<'a> {
    contenido: &'a str,
    inicios_de_linea: Vec<usize>,
}

impl<'a> ConvertidorPosiciones<'a> {
    /// Indexa los inicios de línea de un fragmento en memoria.
    pub fn nuevo(contenido: &'a str) -> Self {
        let mut inicios_de_linea = vec![0];
        for (indice, byte) in contenido.bytes().enumerate() {
            if byte == b'\n' {
                inicios_de_linea.push(indice + 1);
            }
        }
        Self {
            contenido,
            inicios_de_linea,
        }
    }

    /// Convierte un offset UTF-8 a una posición LSP con unidades UTF-16.
    ///
    /// Un offset fuera del documento se limita al final. Un offset dentro de
    /// un carácter se desplaza a su inicio.
    pub fn offset_a_posicion(&self, offset: usize) -> Position {
        let mut offset = offset.min(self.contenido.len());
        while !self.contenido.is_char_boundary(offset) {
            offset -= 1;
        }

        let linea = match self.inicios_de_linea.binary_search(&offset) {
            Ok(linea) => linea,
            Err(linea) => linea.saturating_sub(1),
        };
        let (inicio, fin) = self.limites_de_linea(linea);
        let offset = offset.min(fin);
        let caracter = self.contenido[inicio..offset].encode_utf16().count();
        Position {
            line: saturar_u32(linea),
            character: saturar_u32(caracter),
        }
    }

    /// Convierte una posición LSP con unidades UTF-16 a un offset UTF-8.
    ///
    /// Devuelve `None` para una línea inexistente. Una columna mayor que la
    /// línea se limita a su final, antes de CRLF. Una posición dentro de un
    /// par sustituto se desplaza al inicio del carácter.
    pub fn posicion_a_offset(&self, posicion: Position) -> Option<usize> {
        let linea = usize::try_from(posicion.line).ok()?;
        let (inicio, fin) = self.limites_de_linea_opcional(linea)?;
        let mut unidades = 0_u32;

        for (offset, caracter) in self.contenido[inicio..fin].char_indices() {
            if unidades == posicion.character {
                return Some(inicio + offset);
            }
            let siguientes = unidades.saturating_add(caracter.len_utf16() as u32);
            if posicion.character < siguientes {
                return Some(inicio + offset);
            }
            unidades = siguientes;
        }
        Some(fin)
    }

    /// Convierte una ubicación de bytes de Quetzal a un rango LSP.
    pub fn ubicacion_a_rango(&self, ubicacion: Ubicacion) -> Range {
        Range {
            start: self.offset_a_posicion(ubicacion.inicio),
            end: self.offset_a_posicion(ubicacion.fin),
        }
    }

    /// Convierte un rango LSP a una ubicación de bytes de Quetzal.
    ///
    /// Devuelve `None` si alguna línea no existe o el rango está invertido.
    pub fn rango_a_ubicacion(&self, rango: Range) -> Option<Ubicacion> {
        let inicio = self.posicion_a_offset(rango.start)?;
        let fin = self.posicion_a_offset(rango.end)?;
        (inicio <= fin).then_some(Ubicacion { inicio, fin })
    }

    fn limites_de_linea(&self, linea: usize) -> (usize, usize) {
        self.limites_de_linea_opcional(linea)
            .unwrap_or((self.contenido.len(), self.contenido.len()))
    }

    fn limites_de_linea_opcional(&self, linea: usize) -> Option<(usize, usize)> {
        let inicio = *self.inicios_de_linea.get(linea)?;
        let mut fin = self
            .inicios_de_linea
            .get(linea + 1)
            .copied()
            .unwrap_or(self.contenido.len());
        if fin > inicio && self.contenido.as_bytes()[fin - 1] == b'\n' {
            fin -= 1;
            if fin > inicio && self.contenido.as_bytes()[fin - 1] == b'\r' {
                fin -= 1;
            }
        }
        Some((inicio, fin))
    }
}

fn saturar_u32(valor: usize) -> u32 {
    u32::try_from(valor).unwrap_or(u32::MAX)
}
