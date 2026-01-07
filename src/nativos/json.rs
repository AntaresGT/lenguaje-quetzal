// Métodos nativos de JSON - extensión del tipo JSON
// Se implementará como métodos que se pueden llamar en valores de tipo JSON

use crate::interprete::valores::Valor;
use rust_decimal::Decimal;
use serde_json::Value as JsonValue;

/// Aplica un método de JSON a un valor
pub fn aplicar_metodo_json(
    metodo: &str,
    json: &mut JsonValue,
    argumentos: Vec<Valor>,
    es_mutable: bool,
) -> Result<(Valor, bool), String> {
    // Retorna (resultado, necesita_actualizar_json)
    match metodo {
        "contiene_clave" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'contiene_clave()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            if let JsonValue::Object(obj) = json {
                if let Valor::Texto(clave) = &argumentos[0] {
                    Ok((Valor::Logico(obj.contains_key(clave)), false))
                } else {
                    Err("método 'contiene_clave()' requiere un argumento de tipo texto".to_string())
                }
            } else {
                Err("método 'contiene_clave()' solo está disponible para objetos JSON".to_string())
            }
        }
        "claves" => {
            if !argumentos.is_empty() {
                return Err("método 'claves()' no acepta argumentos".to_string());
            }
            if let JsonValue::Object(obj) = json {
                let claves: Vec<Valor> = obj.keys().map(|k| Valor::Texto(k.clone())).collect();
                Ok((Valor::Lista(claves), false))
            } else {
                Err("método 'claves()' solo está disponible para objetos JSON".to_string())
            }
        }
        "valores" => {
            if !argumentos.is_empty() {
                return Err("método 'valores()' no acepta argumentos".to_string());
            }
            if let JsonValue::Object(obj) = json {
                let valores: Vec<Valor> = obj.values().map(|v| json_value_a_valor(v)).collect();
                Ok((Valor::Lista(valores), false))
            } else {
                Err("método 'valores()' solo está disponible para objetos JSON".to_string())
            }
        }
        "establecer" => {
            if argumentos.len() != 2 {
                return Err(format!("método 'establecer()' requiere 2 argumentos, se recibieron {}", argumentos.len()));
            }
            if !es_mutable {
                return Err("método 'establecer()' requiere un JSON mutable".to_string());
            }
            if let JsonValue::Object(obj) = json {
                if let Valor::Texto(clave) = &argumentos[0] {
                    let valor_json = valor_a_json_value(&argumentos[1]);
                    obj.insert(clave.clone(), valor_json);
                    Ok((Valor::Vacio, true))
                } else {
                    Err("método 'establecer()' requiere una clave de tipo texto".to_string())
                }
            } else {
                Err("método 'establecer()' solo está disponible para objetos JSON".to_string())
            }
        }
        "eliminar" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'eliminar()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            if !es_mutable {
                return Err("método 'eliminar()' requiere un JSON mutable".to_string());
            }
            if let JsonValue::Object(obj) = json {
                if let Valor::Texto(clave) = &argumentos[0] {
                    obj.remove(clave);
                    Ok((Valor::Vacio, true))
                } else {
                    Err("método 'eliminar()' requiere una clave de tipo texto".to_string())
                }
            } else {
                Err("método 'eliminar()' solo está disponible para objetos JSON".to_string())
            }
        }
        "fusionar" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'fusionar()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            if !es_mutable {
                return Err("método 'fusionar()' requiere un JSON mutable".to_string());
            }
            if let JsonValue::Object(obj1) = json {
                let json2 = valor_a_json_value(&argumentos[0]);
                if let JsonValue::Object(obj2_map) = json2 {
                    for (k, v) in obj2_map {
                        obj1.insert(k, v);
                    }
                    Ok((Valor::Vacio, true))
                } else {
                    Err("método 'fusionar()' requiere un objeto JSON como argumento".to_string())
                }
            } else {
                Err("método 'fusionar()' solo está disponible para objetos JSON".to_string())
            }
        }
        "texto" => {
            if !argumentos.is_empty() {
                return Err("método 'texto()' no acepta argumentos".to_string());
            }
            Ok((Valor::Texto(json.to_string()), false))
        }
        "texto_formateado" => {
            if !argumentos.is_empty() {
                return Err("método 'texto_formateado()' no acepta argumentos".to_string());
            }
            match serde_json::to_string_pretty(json) {
                Ok(texto) => Ok((Valor::Texto(texto), false)),
                Err(e) => Err(format!("error al formatear JSON: {}", e)),
            }
        }
        _ => Err(format!("método '{}' no encontrado", metodo)),
    }
}

/// Convierte un JsonValue a Valor
fn json_value_a_valor(json: &JsonValue) -> Valor {
    match json {
        JsonValue::Null => Valor::Vacio,
        JsonValue::Bool(b) => Valor::Logico(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Valor::Entero(i)
            } else if let Some(f) = n.as_f64() {
                Valor::Numero(Decimal::from_f64_retain(f).unwrap_or(Decimal::ZERO))
            } else {
                Valor::Texto(n.to_string())
            }
        }
        JsonValue::String(s) => Valor::Texto(s.clone()),
        JsonValue::Array(arr) => {
            let valores: Vec<Valor> = arr.iter().map(json_value_a_valor).collect();
            Valor::Lista(valores)
        }
        JsonValue::Object(_) => Valor::Json(json.clone()),
    }
}

/// Convierte un Valor a JsonValue
fn valor_a_json_value(valor: &Valor) -> JsonValue {
    match valor {
        Valor::Vacio => JsonValue::Null,
        Valor::Entero(n) => JsonValue::Number((*n).into()),
        Valor::Numero(n) => {
            let f64_val: f64 = n.to_string().parse().unwrap_or(0.0);
            JsonValue::Number(serde_json::Number::from_f64(f64_val).unwrap_or(serde_json::Number::from(0)))
        }
        Valor::Texto(s) => JsonValue::String(s.clone()),
        Valor::Logico(b) => JsonValue::Bool(*b),
        Valor::Lista(l) => {
            let arr: Vec<JsonValue> = l.iter().map(valor_a_json_value).collect();
            JsonValue::Array(arr)
        }
        Valor::Json(j) => j.clone(),
        _ => JsonValue::String(valor.a_texto()),
    }
}
