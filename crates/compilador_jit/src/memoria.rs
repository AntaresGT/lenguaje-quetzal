use crate::SalidaEntera;

type FirmaTrivial = unsafe extern "C" fn(*const i64, u64) -> i64;
type FirmaEntera = unsafe extern "C" fn(*const i64, u64, *mut SalidaEntera);

/// Invoca código emitido por Cranelift con la ABI interna de este crate.
///
/// # Safety
///
/// `puntero` debe ser la dirección finalizada de una función con `FirmaTrivial`
/// y su `JITModule` propietario debe seguir vivo durante toda la llamada. El
/// slice debe ser válido para lecturas; el código generado no conserva sus
/// punteros ni permite unwinding a través de la frontera.
pub(crate) unsafe fn llamar_trivial(puntero: *const u8, argumentos: &[i64]) -> i64 {
    // SAFETY: el llamador garantiza la procedencia, signatura y vida del código.
    let funcion: FirmaTrivial = unsafe { std::mem::transmute(puntero) };
    // SAFETY: el slice está vivo y el módulo se mantiene prestado por `self`.
    unsafe { funcion(argumentos.as_ptr(), argumentos.len() as u64) }
}

/// Llama una función entera del Baseline JIT.
///
/// # Safety
///
/// `puntero` debe provenir de una función finalizada con `FirmaEntera` y su
/// módulo debe seguir vivo. Los argumentos deben tener la aridad compilada y
/// `salida` debe ser un puntero exclusivo válido para escritura. El código no
/// conserva direcciones ni propaga excepciones de Rust.
pub(crate) unsafe fn llamar_entera(
    puntero: *const u8,
    argumentos: &[i64],
    salida: &mut SalidaEntera,
) {
    // SAFETY: el llamador garantiza la signatura y la vida del módulo.
    let funcion: FirmaEntera = unsafe { std::mem::transmute(puntero) };
    // SAFETY: ambos buffers permanecen válidos durante la llamada.
    unsafe { funcion(argumentos.as_ptr(), argumentos.len() as u64, salida) }
}
