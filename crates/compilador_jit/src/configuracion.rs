#[derive(Debug, Clone, Copy)]
pub struct ConfiguracionJit {
    pub max_funciones: usize,
    pub max_bytes_codigo: usize,
}

impl Default for ConfiguracionJit {
    fn default() -> Self {
        Self {
            max_funciones: 256,
            max_bytes_codigo: 16 * 1024 * 1024,
        }
    }
}
