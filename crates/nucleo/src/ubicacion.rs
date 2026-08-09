//! Ubicaciones y posiciones dentro del código fuente.

/// Posición humana dentro de un archivo: línea y columna (ambas inician en 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Posicion {
    pub linea: usize,
    pub columna: usize,
}

/// Rango de bytes dentro de un archivo fuente.
///
/// Se guarda como offsets de bytes para que sea barato de copiar; la
/// conversión a línea/columna la hace [`crate::Fuente`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct Ubicacion {
    /// Offset de byte donde inicia el fragmento (inclusivo).
    pub inicio: usize,
    /// Offset de byte donde termina el fragmento (exclusivo).
    pub fin: usize,
}

impl Ubicacion {
    pub fn nueva(inicio: usize, fin: usize) -> Self {
        Self { inicio, fin }
    }

    /// Combina dos ubicaciones en una que cubre ambas.
    pub fn unir(self, otra: Ubicacion) -> Ubicacion {
        Ubicacion {
            inicio: self.inicio.min(otra.inicio),
            fin: self.fin.max(otra.fin),
        }
    }

    pub fn longitud(&self) -> usize {
        self.fin.saturating_sub(self.inicio)
    }
}
