use crate::errores::Resultado;
use crate::interprete::entorno::Entorno;
use crate::interprete::valores::Valor;

#[derive(Debug, Clone)]
pub struct DescriptorModuloNativo {
    pub nombre: String,
    pub rutas: Vec<String>,
    pub exportaciones: Vec<String>,
    pub constantes: Vec<String>,
    pub permisos: Vec<String>,
    pub global: bool,
}

/// Trait que deben implementar todos los módulos nativos
pub trait ModuloNativo {
    /// Nombre del módulo (ej: "matemática", "consola")
    fn nombre(&self) -> &str;
    
    /// Ruta de importación (ej: "quetzal/matemática")
    fn ruta(&self) -> &str;

    /// Descriptor compartido entre runtime y verificación semántica.
    fn descriptor(&self) -> DescriptorModuloNativo;
    
    /// Registra las funciones y constantes del módulo en el entorno
    fn registrar(&self, entorno: &mut Entorno) -> Resultado<()>;
    
    /// Obtiene una constante del módulo
    fn obtener_constante(&self, nombre: &str) -> Option<Valor>;

    /// Obtiene el tipo de una constante del módulo (para verificación semántica)
    fn obtener_tipo_constante(&self, nombre: &str) -> Option<crate::nucleo::semantico::tipos::Tipo>;
    
    /// Llama a una función del módulo
    fn llamar_funcion(
        &self,
        nombre: &str,
        argumentos: Vec<Valor>,
        entorno: &Entorno,
    ) -> Resultado<Valor>;
}
