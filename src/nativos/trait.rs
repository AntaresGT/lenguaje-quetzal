// Este archivo se renombró a interfaz.rs
// Mantenido por compatibilidad
    /// Nombre del módulo (ej: "matemática", "consola")
    fn nombre(&self) -> &str;
    
    /// Ruta de importación (ej: "quetzal/matemática")
    fn ruta(&self) -> &str;
    
    /// Registra las funciones y constantes del módulo en el entorno
    fn registrar(&self, entorno: &mut Entorno) -> Resultado<()>;
    
    /// Obtiene una constante del módulo
    fn obtener_constante(&self, nombre: &str) -> Option<Valor>;
    
    /// Llama a una función del módulo
    fn llamar_funcion(
        &self,
        nombre: &str,
        argumentos: Vec<Valor>,
        entorno: &Entorno,
    ) -> Resultado<Valor>;
}
