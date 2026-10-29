use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct ConfiguracionJit {
    pub max_funciones: usize,
    pub max_bytes_codigo: usize,
    pub anticipacion: bool,
    pub max_instrucciones_anticipacion: usize,
    pub max_costo_estimado_anticipacion: usize,
    pub max_tiempo_compilacion_anticipacion: Duration,
}

impl Default for ConfiguracionJit {
    fn default() -> Self {
        Self {
            max_funciones: 256,
            max_bytes_codigo: 16 * 1024 * 1024,
            anticipacion: true,
            max_instrucciones_anticipacion: 256,
            max_costo_estimado_anticipacion: 512,
            max_tiempo_compilacion_anticipacion: Duration::from_millis(5),
        }
    }
}
