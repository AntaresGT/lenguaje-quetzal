pub const ESTADO_EXITO: u32 = 0;
pub const ESTADO_DIVISION_CERO: u32 = 1;
pub const ESTADO_DESBORDAMIENTO: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct SalidaEntera {
    pub valor: i64,
    pub estado: u32,
    pub pc: u32,
}

impl SalidaEntera {
    pub fn nueva() -> Self {
        Self {
            valor: 0,
            estado: ESTADO_EXITO,
            pc: 0,
        }
    }
}

impl Default for SalidaEntera {
    fn default() -> Self {
        Self::nueva()
    }
}
