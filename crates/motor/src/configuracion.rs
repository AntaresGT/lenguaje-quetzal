//! Configuración del motor de Quetzal.

/// Política de compilación nativa selectiva durante la ejecución.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModoJit {
    Desactivado,
    Automatico,
    Activado,
}

/// Configuración con la que se crea un [`crate::MotorQuetzal`].
#[derive(Debug, Clone)]
pub struct ConfiguracionMotor {
    /// Si los reportes de error usan colores de terminal.
    pub colores: bool,
    /// Especialización y caches en línea. `false` deja el intérprete puro.
    pub optimizar: bool,
    /// Política del JIT en memoria; nunca implica compilación previa del programa.
    pub jit: ModoJit,
}

impl ConfiguracionMotor {
    pub fn por_defecto() -> Self {
        Self {
            colores: true,
            optimizar: true,
            jit: modo_automatico_disponible(),
        }
    }

    pub fn sin_colores() -> Self {
        Self {
            colores: false,
            optimizar: true,
            jit: modo_automatico_disponible(),
        }
    }

    pub fn con_jit(self, habilitado: bool) -> Self {
        self.con_modo_jit(if habilitado {
            ModoJit::Activado
        } else {
            ModoJit::Desactivado
        })
    }

    pub fn con_modo_jit(mut self, modo: ModoJit) -> Self {
        self.jit = modo;
        self
    }

    pub fn sin_optimizar(mut self) -> Self {
        self.optimizar = false;
        self.jit = ModoJit::Desactivado;
        self
    }
}

fn modo_automatico_disponible() -> ModoJit {
    if cfg!(feature = "jit") {
        ModoJit::Automatico
    } else {
        ModoJit::Desactivado
    }
}

impl Default for ConfiguracionMotor {
    fn default() -> Self {
        Self::por_defecto()
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn con_jit_deberia_conservar_el_adaptador_booleano() {
        let activado = ConfiguracionMotor::por_defecto().con_jit(true);
        let desactivado = ConfiguracionMotor::por_defecto().con_jit(false);
        assert_eq!(activado.jit, ModoJit::Activado);
        assert_eq!(desactivado.jit, ModoJit::Desactivado);
    }

    #[test]
    fn sin_optimizar_deberia_desactivar_tambien_el_jit() {
        let configuracion = ConfiguracionMotor::por_defecto()
            .con_jit(true)
            .sin_optimizar();
        assert!(!configuracion.optimizar);
        assert_eq!(configuracion.jit, ModoJit::Desactivado);
    }

    #[cfg(feature = "jit")]
    #[test]
    fn un_binario_con_jit_deberia_elegir_modo_automatico() {
        assert_eq!(ConfiguracionMotor::por_defecto().jit, ModoJit::Automatico);
    }

    #[cfg(not(feature = "jit"))]
    #[test]
    fn un_binario_sin_jit_deberia_usar_el_interprete() {
        assert_eq!(ConfiguracionMotor::por_defecto().jit, ModoJit::Desactivado);
    }
}
