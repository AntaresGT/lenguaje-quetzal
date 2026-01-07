use crate::errores::Resultado;
use crate::interprete::entorno::Entorno;
use crate::nativos::consola::Consola;
use crate::nativos::matematica::Matematica;
use crate::nativos::interfaz::ModuloNativo;
use std::sync::Arc;
use std::collections::HashMap;

/// Registra todos los módulos nativos en el entorno
pub fn registrar_modulos_nativos(entorno: &mut Entorno) -> Resultado<()> {
    // Registrar consola como objeto global (no necesita importarse)
    let consola = Arc::new(Consola::nuevo());
    entorno.registrar_objeto_nativo("consola".to_string(), consola);
    
    // Registrar funciones globales nativas
    registrar_funciones_globales(entorno)?;
    
    // Registrar los módulos nativos importables
    registrar_modulos_importables(entorno);
    
    Ok(())
}

/// Registra los módulos nativos que pueden ser importados
fn registrar_modulos_importables(entorno: &mut Entorno) {
    // Registrar Matemática como objeto nativo importable
    // Se registra con múltiples variantes de nombre para compatibilidad
    let matematica = Arc::new(Matematica::nuevo());
    entorno.registrar_objeto_nativo("Matemática".to_string(), matematica.clone());
    entorno.registrar_objeto_nativo("Matematica".to_string(), matematica);
}

/// Registra funciones globales nativas como rango()
fn registrar_funciones_globales(_entorno: &mut Entorno) -> Resultado<()> {
    // Función rango(inicio, fin) - crea una lista de números consecutivos
    // Esta función se implementa como una función especial que se detecta en el intérprete
    // La implementación real está en expresiones.rs cuando se llama a una función
    
    Ok(())
}

/// Verifica si una ruta corresponde a un módulo nativo
pub fn es_modulo_nativo(ruta: &str) -> bool {
    let rutas_nativas = [
        "quetzal/matemática",
        "quetzal/matematica",
        "quetzal/consola",
        "quetzal/sistema",
    ];
    rutas_nativas.contains(&ruta)
}

/// Procesa una importación y registra los elementos en el entorno
pub fn procesar_importacion(
    elementos: &[String],
    ruta: &str,
    entorno: &mut Entorno,
) -> Resultado<()> {
    // Mapa de rutas a módulos nativos
    let modulos_nativos: HashMap<&str, fn() -> Arc<dyn ModuloNativo>> = {
        let mut m: HashMap<&str, fn() -> Arc<dyn ModuloNativo>> = HashMap::new();
        m.insert("quetzal/matemática", || Arc::new(Matematica::nuevo()));
        m.insert("quetzal/matematica", || Arc::new(Matematica::nuevo()));
        m
    };
    
    // Buscar el módulo por su ruta
    if let Some(crear_modulo) = modulos_nativos.get(ruta) {
        let modulo = crear_modulo();
        
        // Registrar cada elemento importado
        for elemento in elementos {
            // Registrar el módulo con el nombre importado
            entorno.registrar_objeto_nativo(elemento.clone(), modulo.clone());
        }
    }
    
    Ok(())
}
