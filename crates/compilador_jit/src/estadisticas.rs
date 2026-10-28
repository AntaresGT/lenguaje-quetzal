use std::time::Duration;

#[derive(Debug, Default, Clone, Copy)]
pub struct EstadisticasJit {
    pub funciones_compiladas: usize,
    pub bytes_codigo: usize,
    pub tiempo_compilacion: Duration,
    pub fallbacks: usize,
    pub desoptimizaciones: usize,
}
