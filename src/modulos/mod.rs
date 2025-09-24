// Registro centralizado de módulos nativos para el lenguaje Quetzal
// Permite ampliar las capacidades del intérprete sin modificar el núcleo

use std::collections::HashMap;

use crate::ejecucion::evaluador::Evaluador;
use crate::infraestructura::errores::ResultadoQuetzal;
use crate::infraestructura::manejador_modulos::ElementoExportado;

mod matematica;

/// Tipo de función responsable de registrar un módulo nativo
pub type FuncionRegistroModulo =
    fn(&mut Evaluador) -> ResultadoQuetzal<HashMap<String, ElementoExportado>>;

/// Descriptor con la información mínima de cada módulo nativo
struct DescriptorModuloNativo {
    nombre: &'static str,
    registrador: FuncionRegistroModulo,
}

/// Lista estática de módulos nativos disponibles
const MODULOS_NATIVOS: &[DescriptorModuloNativo] = &[DescriptorModuloNativo {
    nombre: "quetzal/matemática",
    registrador: matematica::registrar_modulo,
}];

/// Obtiene la función registradora asociada a un módulo nativo específico
pub fn obtener_registrador(nombre: &str) -> Option<FuncionRegistroModulo> {
    let nombre_normalizado = normalizar_nombre(nombre);
    MODULOS_NATIVOS
        .iter()
        .find(|descriptor| normalizar_nombre(descriptor.nombre) == nombre_normalizado)
        .map(|descriptor| descriptor.registrador)
}

/// Devuelve la lista de módulos nativos registrados
pub fn nombres_registrados() -> Vec<String> {
    MODULOS_NATIVOS
        .iter()
        .map(|descriptor| descriptor.nombre.to_string())
        .collect()
}

/// Normaliza el nombre para tratar de forma uniforme rutas con distintas barras
fn normalizar_nombre(nombre: &str) -> String {
    nombre.replace('\\', "/")
}
