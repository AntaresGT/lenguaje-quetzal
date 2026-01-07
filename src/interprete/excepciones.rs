// Sistema de manejo de excepciones para Lenguaje Quetzal

use crate::interprete::valores::Valor;
use crate::errores::Resultado;
use serde_json::Value as JsonValue;

/// Representa una excepción en tiempo de ejecución
#[derive(Debug, Clone)]
pub struct Excepcion {
    pub mensaje: String,
    pub llamadas: Vec<String>, // Pila de llamadas
}

impl Excepcion {
    /// Crea una nueva excepción
    pub fn nueva(mensaje: String) -> Self {
        Self {
            mensaje,
            llamadas: Vec::new(),
        }
    }
    
    /// Crea una excepción desde un Error con pila de llamadas
    pub fn desde_error(error: &crate::errores::Error) -> Self {
        let mensaje = error.mensaje().to_string();
        let llamadas = error.pila_llamadas()
            .map(|p| p.to_vec())
            .unwrap_or_default();
        
        Self { mensaje, llamadas }
    }
    
    /// Convierte la excepción a un objeto Valor (JSON)
    pub fn a_valor(&self) -> Valor {
        use serde_json::Value as JsonValue;
        use serde_json::Map;
        
        let mut obj = Map::new();
        obj.insert("mensaje".to_string(), JsonValue::String(self.mensaje.clone()));
        
        let llamadas_json: Vec<JsonValue> = self.llamadas.iter()
            .map(|l| JsonValue::String(l.clone()))
            .collect();
        obj.insert("llamadas".to_string(), JsonValue::Array(llamadas_json));
        
        Valor::Json(JsonValue::Object(obj))
    }
    
    /// Crea una excepción desde un Valor
    pub fn desde_valor(valor: &Valor) -> Self {
        match valor {
            Valor::Texto(mensaje) => Self::nueva(mensaje.clone()),
            Valor::Json(JsonValue::Object(obj)) => {
                let mensaje = obj.get("mensaje")
                    .and_then(|v| if let JsonValue::String(s) = v { Some(s.clone()) } else { None })
                    .unwrap_or_else(|| "excepción desconocida".to_string());
                
                let llamadas = obj.get("llamadas")
                    .and_then(|v| if let JsonValue::Array(arr) = v {
                        Some(arr.iter().filter_map(|v| {
                            if let JsonValue::String(s) = v {
                                Some(s.clone())
                            } else {
                                None
                            }
                        }).collect())
                    } else {
                        None
                    })
                    .unwrap_or_default();
                
                Self { mensaje, llamadas }
            }
            _ => Self::nueva(valor.a_texto()),
        }
    }
}

/// Tipo de resultado que puede contener una excepción
pub type ResultadoExcepcion<T> = std::result::Result<T, Excepcion>;

/// Convierte un Resultado a ResultadoExcepcion
pub fn convertir_error_a_excepcion(resultado: Resultado<Valor>) -> ResultadoExcepcion<Valor> {
    resultado.map_err(|e| Excepcion::nueva(e.mensaje().to_string()))
}
