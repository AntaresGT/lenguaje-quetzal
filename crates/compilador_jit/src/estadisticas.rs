#[derive(Debug, Default, Clone, Copy)]
pub struct EstadisticasJit {
    pub funciones_compiladas: usize,
    pub bytes_codigo: usize,
}
