//! Interrupción cooperativa de la ejecución (Ctrl+C).
//!
//! Un bucle `mientras (verdadero)` deja al sistema sin respuesta: el
//! proceso consume CPU al máximo y, en Windows, el carácter de interrupción
//! solo mata el proceso en frío, sin mensaje ni limpieza.
//!
//! La solución es una bandera global que un manejador de señales del proceso
//! activa al recibir Ctrl+C. La VM la revisa periódicamente entre
//! instrucciones y detiene la ejecución con un error claro, sin matar el
//! proceso: el REPL queda vivo y los errores se reportan con normalidad.

use std::sync::atomic::{AtomicBool, Ordering};

/// Se activa al recibir Ctrl+C durante la ejecución de código.
static INTERRUMPIDO: AtomicBool = AtomicBool::new(false);

/// Marca la bandera de interrupción. La VM la detecta en su próxima revisión
/// y detiene la ejecución con un error de runtime.
pub fn interrumpir() {
    INTERRUMPIDO.store(true, Ordering::SeqCst);
}

/// Limpia la bandera. Debe llamarse antes de cada evaluación nueva para que
/// un Ctrl+C del pasado no cancele el siguiente código.
pub fn reiniciar() {
    INTERRUMPIDO.store(false, Ordering::SeqCst);
}

/// Indica si hay una interrupción pendente.
pub fn interrumpido() -> bool {
    INTERRUMPIDO.load(Ordering::SeqCst)
}
