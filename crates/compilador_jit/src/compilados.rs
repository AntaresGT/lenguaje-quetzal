use crate::ErrorJit;

#[cfg(feature = "nativo")]
pub struct FuncionNativa {
    pub(crate) _modulo: cranelift_jit::JITModule,
    pub(crate) puntero: *const u8,
    pub(crate) aridad: usize,
}

#[cfg(not(feature = "nativo"))]
pub struct FuncionNativa;

impl FuncionNativa {
    pub fn ejecutar_trivial(&self, argumentos: &[i64]) -> Result<i64, ErrorJit> {
        #[cfg(feature = "nativo")]
        {
            if argumentos.len() != self.aridad {
                return Err(ErrorJit::Aridad {
                    esperados: self.aridad,
                    recibidos: argumentos.len(),
                });
            }
            // El puntero pertenece al módulo vivo contenido en `self` y la
            // signatura coincide con la emitida por `generador`.
            Ok(unsafe { crate::memoria::llamar_trivial(self.puntero, argumentos) })
        }
        #[cfg(not(feature = "nativo"))]
        {
            let _ = argumentos;
            Err(ErrorJit::NoDisponible)
        }
    }
}

#[cfg(feature = "nativo")]
impl std::fmt::Debug for FuncionNativa {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FuncionNativa")
            .field("aridad", &self.aridad)
            .finish_non_exhaustive()
    }
}

#[cfg(not(feature = "nativo"))]
impl std::fmt::Debug for FuncionNativa {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FuncionNativa").finish_non_exhaustive()
    }
}
