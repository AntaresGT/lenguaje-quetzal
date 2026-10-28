//! Configuración del motor de Quetzal.

/// Configuración con la que se crea un [`crate::MotorQuetzal`].
#[derive(Debug, Clone)]
pub struct ConfiguracionMotor {
    /// Si los reportes de error usan colores de terminal.
    pub colores: bool,
    /// Especialización y caches en línea. `false` deja el intérprete puro.
    pub optimizar: bool,
    /// Compilación JIT selectiva. Desactivada por defecto.
    pub jit: bool,
}

impl ConfiguracionMotor {
    pub fn por_defecto() -> Self {
        Self {
            colores: true,
            optimizar: true,
            jit: false,
        }
    }

    pub fn sin_colores() -> Self {
        Self {
            colores: false,
            optimizar: true,
            jit: false,
        }
    }

    pub fn con_jit(mut self, habilitado: bool) -> Self {
        self.jit = habilitado;
        self
    }

    pub fn sin_optimizar(mut self) -> Self {
        self.optimizar = false;
        self.jit = false;
        self
    }
}

impl Default for ConfiguracionMotor {
    fn default() -> Self {
        Self::por_defecto()
    }
}
