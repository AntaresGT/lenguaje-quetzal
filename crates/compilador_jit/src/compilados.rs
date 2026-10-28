use crate::{ErrorJit, SalidaEntera};

#[cfg(feature = "nativo")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FirmaNativa {
    Trivial,
    Entera,
}

#[cfg(feature = "nativo")]
pub struct FuncionNativa {
    pub(crate) _modulo: cranelift_jit::JITModule,
    pub(crate) puntero: *const u8,
    pub(crate) aridad: usize,
    pub(crate) firma: FirmaNativa,
}

#[cfg(not(feature = "nativo"))]
pub struct FuncionNativa;

impl FuncionNativa {
    pub fn ejecutar_trivial(&self, argumentos: &[i64]) -> Result<i64, ErrorJit> {
        #[cfg(feature = "nativo")]
        {
            if self.firma != FirmaNativa::Trivial {
                return Err(ErrorJit::FirmaIncorrecta);
            }
            self.validar_aridad(argumentos)?;
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

    pub fn ejecutar(&self, argumentos: &[i64]) -> Result<SalidaEntera, ErrorJit> {
        #[cfg(feature = "nativo")]
        {
            if self.firma != FirmaNativa::Entera {
                return Err(ErrorJit::FirmaIncorrecta);
            }
            self.validar_aridad(argumentos)?;
            let mut salida = SalidaEntera::nueva();
            // El módulo permanece vivo y los dos buffers son exclusivos y
            // válidos durante toda la llamada nativa.
            unsafe { crate::memoria::llamar_entera(self.puntero, argumentos, &mut salida) };
            Ok(salida)
        }
        #[cfg(not(feature = "nativo"))]
        {
            let _ = argumentos;
            Err(ErrorJit::NoDisponible)
        }
    }

    #[cfg(feature = "nativo")]
    fn validar_aridad(&self, argumentos: &[i64]) -> Result<(), ErrorJit> {
        if argumentos.len() == self.aridad {
            Ok(())
        } else {
            Err(ErrorJit::Aridad {
                esperados: self.aridad,
                recibidos: argumentos.len(),
            })
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
