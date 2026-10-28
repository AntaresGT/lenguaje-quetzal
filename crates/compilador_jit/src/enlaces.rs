pub const ESTADO_EXITO: u32 = 0;
pub const ESTADO_DIVISION_CERO: u32 = 1;
pub const ESTADO_DESBORDAMIENTO: u32 = 2;

pub const OPERACION_NINGUNA: u32 = 0;
pub const OPERACION_SUMA: u32 = 1;
pub const OPERACION_RESTA: u32 = 2;
pub const OPERACION_MULTIPLICACION: u32 = 3;
pub const OPERACION_DIVISION: u32 = 4;
pub const OPERACION_MODULO: u32 = 5;
pub const OPERACION_AJUSTE: u32 = 6;
pub const OPERACION_NEGACION: u32 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct SalidaEntera {
    pub valor: i64,
    pub izquierda: i64,
    pub derecha: i64,
    pub estado: u32,
    pub pc: u32,
    pub operacion: u32,
}

impl SalidaEntera {
    pub fn nueva() -> Self {
        Self {
            valor: 0,
            izquierda: 0,
            derecha: 0,
            estado: ESTADO_EXITO,
            pc: 0,
            operacion: OPERACION_NINGUNA,
        }
    }
}

impl Default for SalidaEntera {
    fn default() -> Self {
        Self::nueva()
    }
}
