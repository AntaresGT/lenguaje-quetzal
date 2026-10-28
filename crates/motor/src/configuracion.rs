//! Configuración del motor de Quetzal.

/// Configuración con la que se crea un [`crate::MotorQuetzal`].
#[derive(Debug, Clone)]
pub struct ConfiguracionMotor {
    /// Si los reportes de error usan colores de terminal.
    pub colores: bool,
    /// Especialización, caches en línea y cierres. `false` deja el intérprete puro.
    pub optimizar: bool,
}

impl ConfiguracionMotor {
    pub fn por_defecto() -> Self {
        Self {
            colores: true,
            optimizar: true,
        }
    }

    pub fn sin_colores() -> Self {
        Self {
            colores: false,
            optimizar: true,
        }
    }

    pub fn sin_optimizar(mut self) -> Self {
        self.optimizar = false;
        self
    }
}

impl Default for ConfiguracionMotor {
    fn default() -> Self {
        Self::por_defecto()
    }
}
