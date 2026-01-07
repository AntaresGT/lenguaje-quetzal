use crate::errores::{Error, CodigoError, Resultado};
use std::collections::{HashMap, HashSet};

/// Resuelvedor de dependencias de módulos
pub struct ResuelvedorModulos {
    dependencias: HashMap<String, Vec<String>>,
}

impl ResuelvedorModulos {
    /// Crea un nuevo resuelvedor
    pub fn nuevo() -> Self {
        Self {
            dependencias: HashMap::new(),
        }
    }
    
    /// Resuelve el orden de carga de módulos
    pub fn resolver_orden(&self, modulo_inicial: &str) -> Resultado<Vec<String>> {
        let mut visitados = HashSet::new();
        let mut orden = Vec::new();
        
        self.visitar_modulo(modulo_inicial, &mut visitados, &mut orden)?;
        
        Ok(orden)
    }
    
    fn visitar_modulo(
        &self,
        modulo: &str,
        visitados: &mut HashSet<String>,
        orden: &mut Vec<String>,
    ) -> Resultado<()> {
        if visitados.contains(modulo) {
            if !orden.contains(&modulo.to_string()) {
                return Err(Error::modulo(
                    CodigoError::DependenciaCircular,
                    format!("dependencia circular detectada en módulo '{}'", modulo),
                    Some(modulo.to_string()),
                ));
            }
            return Ok(());
        }
        
        visitados.insert(modulo.to_string());
        
        if let Some(deps) = self.dependencias.get(modulo) {
            for dep in deps {
                self.visitar_modulo(dep, visitados, orden)?;
            }
        }
        
        orden.push(modulo.to_string());
        Ok(())
    }
}
