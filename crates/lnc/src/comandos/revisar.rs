//! Comando `quetzal revisar [ARCHIVO]`.

use motor::{ConfiguracionMotor, MotorQuetzal};

use crate::comandos::ejecutar::reportar_errores;

/// Analiza un archivo o proyecto sin ejecutarlo.
pub fn revisar(archivo: Option<&str>) -> i32 {
    let motor = match MotorQuetzal::nuevo(ConfiguracionMotor::por_defecto()) {
        Ok(motor) => motor,
        Err(error) => return reportar_errores(None, &[error]),
    };

    match motor.revisar_archivo(archivo.unwrap_or(".")) {
        Ok(()) => {
            println!("Revisión completada sin errores.");
            0
        }
        Err(errores) => reportar_errores(Some(&motor), &errores),
    }
}
