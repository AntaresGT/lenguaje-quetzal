//! Comando `quetzal ejecutar [ARCHIVO]`.

use diagnosticos::colores::Paleta;
use motor::{ConfiguracionMotor, MotorQuetzal};

/// Ejecuta un archivo `.qz` o el proyecto actual (vía `quetzal.json`).
pub fn ejecutar(archivo: Option<&str>, sin_optimizar: bool, jit: bool) -> i32 {
    #[cfg(not(feature = "jit"))]
    if jit {
        eprintln!("este binario se compiló sin soporte JIT; usa la feature 'jit'");
        return 2;
    }
    let mut configuracion = ConfiguracionMotor::por_defecto().con_jit(jit);
    if sin_optimizar {
        configuracion = configuracion.sin_optimizar();
    }
    let motor = match MotorQuetzal::nuevo(configuracion) {
        Ok(motor) => motor,
        Err(error) => return reportar_errores(None, &[error]),
    };

    // Sin archivo: el motor busca quetzal.json en el directorio actual.
    match motor.ejecutar_archivo(archivo.unwrap_or(".")) {
        Ok(()) => 0,
        Err(errores) => reportar_errores(Some(&motor), &errores),
    }
}

/// Reporta errores con su línea de código cuando el motor tiene la fuente.
pub fn reportar_errores(motor: Option<&MotorQuetzal>, errores: &[nucleo::ErrorQuetzal]) -> i32 {
    let paleta = Paleta::default();
    for error in errores {
        let fuente = motor
            .zip(error.archivo.as_deref())
            .and_then(|(motor, archivo)| motor.fuente(archivo));
        eprint!(
            "{}",
            diagnosticos::reportar(error, fuente.as_ref(), &paleta)
        );
    }
    1
}
