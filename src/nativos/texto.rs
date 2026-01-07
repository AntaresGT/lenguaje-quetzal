// Métodos nativos de texto - extensión del tipo Texto
// Se implementará como métodos que se pueden llamar en valores de tipo Texto

use crate::interprete::valores::Valor;
use rust_decimal::Decimal;
use base64::{Engine as _, engine::general_purpose};

/// Aplica un método de texto a un valor
pub fn aplicar_metodo_texto(metodo: &str, texto: &str, argumentos: Vec<Valor>) -> Result<Valor, String> {
    match metodo {
        "longitud" => {
            if !argumentos.is_empty() {
                return Err("método 'longitud()' no acepta argumentos".to_string());
            }
            Ok(Valor::Entero(texto.chars().count() as i64))
        }
        "mayusculas" => {
            if !argumentos.is_empty() {
                return Err("método 'mayusculas()' no acepta argumentos".to_string());
            }
            Ok(Valor::Texto(texto.to_uppercase()))
        }
        "minusculas" => {
            if !argumentos.is_empty() {
                return Err("método 'minusculas()' no acepta argumentos".to_string());
            }
            Ok(Valor::Texto(texto.to_lowercase()))
        }
        "capitalizar" => {
            if !argumentos.is_empty() {
                return Err("método 'capitalizar()' no acepta argumentos".to_string());
            }
            let mut chars = texto.chars();
            match chars.next() {
                None => Ok(Valor::Texto(String::new())),
                Some(first) => Ok(Valor::Texto(first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase())),
            }
        }
        "titulo" => {
            if !argumentos.is_empty() {
                return Err("método 'titulo()' no acepta argumentos".to_string());
            }
            let palabras: Vec<String> = texto.split_whitespace()
                .map(|palabra| {
                    let mut chars = palabra.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
                    }
                })
                .collect();
            Ok(Valor::Texto(palabras.join(" ")))
        }
        "recortar" => {
            if !argumentos.is_empty() {
                return Err("método 'recortar()' no acepta argumentos".to_string());
            }
            Ok(Valor::Texto(texto.trim().to_string()))
        }
        "recortar_inicio" => {
            if !argumentos.is_empty() {
                return Err("método 'recortar_inicio()' no acepta argumentos".to_string());
            }
            Ok(Valor::Texto(texto.trim_start().to_string()))
        }
        "recortar_final" => {
            if !argumentos.is_empty() {
                return Err("método 'recortar_final()' no acepta argumentos".to_string());
            }
            Ok(Valor::Texto(texto.trim_end().to_string()))
        }
        "contiene" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'contiene()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            if let Valor::Texto(subtexto) = &argumentos[0] {
                Ok(Valor::Logico(texto.contains(subtexto)))
            } else {
                Err("método 'contiene()' requiere un argumento de tipo texto".to_string())
            }
        }
        "empieza_con" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'empieza_con()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            if let Valor::Texto(prefijo) = &argumentos[0] {
                Ok(Valor::Logico(texto.starts_with(prefijo)))
            } else {
                Err("método 'empieza_con()' requiere un argumento de tipo texto".to_string())
            }
        }
        "termina_con" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'termina_con()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            if let Valor::Texto(sufijo) = &argumentos[0] {
                Ok(Valor::Logico(texto.ends_with(sufijo)))
            } else {
                Err("método 'termina_con()' requiere un argumento de tipo texto".to_string())
            }
        }
        "encontrar" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'encontrar()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            if let Valor::Texto(subtexto) = &argumentos[0] {
                match texto.find(subtexto) {
                    Some(pos) => Ok(Valor::Entero(pos as i64)),
                    None => Ok(Valor::Entero(-1)),
                }
            } else {
                Err("método 'encontrar()' requiere un argumento de tipo texto".to_string())
            }
        }
        "buscar_ultimo" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'buscar_ultimo()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            if let Valor::Texto(subtexto) = &argumentos[0] {
                match texto.rfind(subtexto) {
                    Some(pos) => Ok(Valor::Entero(pos as i64)),
                    None => Ok(Valor::Entero(-1)),
                }
            } else {
                Err("método 'buscar_ultimo()' requiere un argumento de tipo texto".to_string())
            }
        }
        "reemplazar" => {
            if argumentos.len() != 2 {
                return Err(format!("método 'reemplazar()' requiere 2 argumentos, se recibieron {}", argumentos.len()));
            }
            if let (Valor::Texto(buscar), Valor::Texto(reemplazar_con)) = (&argumentos[0], &argumentos[1]) {
                Ok(Valor::Texto(texto.replace(buscar, reemplazar_con)))
            } else {
                Err("método 'reemplazar()' requiere dos argumentos de tipo texto".to_string())
            }
        }
        "reemplazar_primero" => {
            if argumentos.len() != 2 {
                return Err(format!("método 'reemplazar_primero()' requiere 2 argumentos, se recibieron {}", argumentos.len()));
            }
            if let (Valor::Texto(buscar), Valor::Texto(reemplazar_con)) = (&argumentos[0], &argumentos[1]) {
                Ok(Valor::Texto(texto.replacen(buscar, reemplazar_con, 1)))
            } else {
                Err("método 'reemplazar_primero()' requiere dos argumentos de tipo texto".to_string())
            }
        }
        "dividir" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'dividir()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            if let Valor::Texto(separador) = &argumentos[0] {
                let partes: Vec<Valor> = texto.split(separador)
                    .map(|parte| Valor::Texto(parte.to_string()))
                    .collect();
                Ok(Valor::Lista(partes))
            } else {
                Err("método 'dividir()' requiere un argumento de tipo texto".to_string())
            }
        }
        "partir_lineas" => {
            if !argumentos.is_empty() {
                return Err("método 'partir_lineas()' no acepta argumentos".to_string());
            }
            let lineas: Vec<Valor> = texto.lines()
                .map(|linea| Valor::Texto(linea.to_string()))
                .collect();
            Ok(Valor::Lista(lineas))
        }
        "repetir" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'repetir()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            let veces = match &argumentos[0] {
                Valor::Entero(n) => *n,
                _ => return Err("método 'repetir()' requiere un argumento de tipo entero".to_string()),
            };
            if veces < 0 {
                return Err("método 'repetir()' requiere un número no negativo".to_string());
            }
            Ok(Valor::Texto(texto.repeat(veces as usize)))
        }
        "subtexto" => {
            if argumentos.len() != 2 {
                return Err(format!("método 'subtexto()' requiere 2 argumentos, se recibieron {}", argumentos.len()));
            }
            let inicio = match &argumentos[0] {
                Valor::Entero(n) => *n,
                _ => return Err("método 'subtexto()' requiere argumentos de tipo entero".to_string()),
            };
            let fin = match &argumentos[1] {
                Valor::Entero(n) => *n,
                _ => return Err("método 'subtexto()' requiere argumentos de tipo entero".to_string()),
            };
            let chars: Vec<char> = texto.chars().collect();
            if inicio < 0 || fin < inicio || fin as usize > chars.len() {
                return Err("índices fuera de rango en 'subtexto()'".to_string());
            }
            let resultado: String = chars[inicio as usize..fin as usize].iter().collect();
            Ok(Valor::Texto(resultado))
        }
        "izquierda" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'izquierda()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            let n = match &argumentos[0] {
                Valor::Entero(num) => *num,
                _ => return Err("método 'izquierda()' requiere un argumento de tipo entero".to_string()),
            };
            if n < 0 {
                return Err("método 'izquierda()' requiere un número no negativo".to_string());
            }
            let chars: Vec<char> = texto.chars().collect();
            let n_usize = n as usize;
            if n_usize > chars.len() {
                Ok(Valor::Texto(texto.to_string()))
            } else {
                let resultado: String = chars[..n_usize].iter().collect();
                Ok(Valor::Texto(resultado))
            }
        }
        "derecha" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'derecha()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            let n = match &argumentos[0] {
                Valor::Entero(num) => *num,
                _ => return Err("método 'derecha()' requiere un argumento de tipo entero".to_string()),
            };
            if n < 0 {
                return Err("método 'derecha()' requiere un número no negativo".to_string());
            }
            let chars: Vec<char> = texto.chars().collect();
            let n_usize = n as usize;
            if n_usize > chars.len() {
                Ok(Valor::Texto(texto.to_string()))
            } else {
                let inicio = chars.len() - n_usize;
                let resultado: String = chars[inicio..].iter().collect();
                Ok(Valor::Texto(resultado))
            }
        }
        "es_numero" => {
            if !argumentos.is_empty() {
                return Err("método 'es_numero()' no acepta argumentos".to_string());
            }
            Ok(Valor::Logico(texto.parse::<Decimal>().is_ok()))
        }
        "es_entero" => {
            if !argumentos.is_empty() {
                return Err("método 'es_entero()' no acepta argumentos".to_string());
            }
            Ok(Valor::Logico(texto.parse::<i64>().is_ok()))
        }
        "es_alfanumerico" => {
            if !argumentos.is_empty() {
                return Err("método 'es_alfanumerico()' no acepta argumentos".to_string());
            }
            Ok(Valor::Logico(!texto.is_empty() && texto.chars().all(|c| c.is_alphanumeric())))
        }
        "a_base64" => {
            if !argumentos.is_empty() {
                return Err("método 'a_base64()' no acepta argumentos".to_string());
            }
            let encoded = general_purpose::STANDARD.encode(texto.as_bytes());
            Ok(Valor::Texto(encoded))
        }
        "decodificar_base64" => {
            if !argumentos.is_empty() {
                return Err("método 'decodificar_base64()' no acepta argumentos".to_string());
            }
            // Eliminar espacios y otros caracteres no válidos en base64
            let texto_limpio: String = texto.chars()
                .filter(|c| !c.is_whitespace() && *c != '=')
                .collect();
            // Agregar padding si es necesario
            let padding_necesario = (4 - texto_limpio.len() % 4) % 4;
            let texto_con_padding = texto_limpio + &"=".repeat(padding_necesario);
            match general_purpose::STANDARD.decode(&texto_con_padding) {
                Ok(decoded) => {
                    match String::from_utf8(decoded) {
                        Ok(s) => Ok(Valor::Texto(s)),
                        Err(_) => Err("error al decodificar base64: resultado no es UTF-8 válido".to_string()),
                    }
                }
                Err(e) => Err(format!("error al decodificar base64: {}", e)),
            }
        }
        "a_url" | "a_enlace" => {
            if !argumentos.is_empty() {
                return Err("método 'a_url()' o 'a_enlace()' no acepta argumentos".to_string());
            }
            Ok(Valor::Texto(urlencoding::encode(texto).to_string()))
        }
        "decodificar_url" | "decodificar_enlace" => {
            if !argumentos.is_empty() {
                return Err("método 'decodificar_url()' o 'decodificar_enlace()' no acepta argumentos".to_string());
            }
            match urlencoding::decode(texto) {
                Ok(decoded) => Ok(Valor::Texto(decoded.to_string())),
                Err(e) => Err(format!("error al decodificar URL: {}", e)),
            }
        }
        "igual_sin_caso" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'igual_sin_caso()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            if let Valor::Texto(otro) = &argumentos[0] {
                Ok(Valor::Logico(texto.eq_ignore_ascii_case(otro)))
            } else {
                Err("método 'igual_sin_caso()' requiere un argumento de tipo texto".to_string())
            }
        }
        "contar" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'contar()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            if let Valor::Texto(subtexto) = &argumentos[0] {
                if subtexto.is_empty() {
                    Ok(Valor::Entero(0))
                } else {
                    let count = texto.matches(subtexto).count();
                    Ok(Valor::Entero(count as i64))
                }
            } else {
                Err("método 'contar()' requiere un argumento de tipo texto".to_string())
            }
        }
        "invertir" => {
            if !argumentos.is_empty() {
                return Err("método 'invertir()' no acepta argumentos".to_string());
            }
            let invertido: String = texto.chars().rev().collect();
            Ok(Valor::Texto(invertido))
        }
        // Métodos de conversión de tipos
        "entero" => {
            if !argumentos.is_empty() {
                return Err("método 'entero()' no acepta argumentos".to_string());
            }
            texto.parse::<i64>()
                .map(Valor::Entero)
                .map_err(|_| format!("no se puede convertir '{}' a entero", texto))
        }
        "numero" | "número" => {
            if !argumentos.is_empty() {
                return Err("método 'numero()' no acepta argumentos".to_string());
            }
            use rust_decimal::Decimal;
            texto.parse::<Decimal>()
                .map(Valor::Numero)
                .map_err(|_| format!("no se puede convertir '{}' a número", texto))
        }
        "log" | "lóg" => {
            if !argumentos.is_empty() {
                return Err("método 'log()' no acepta argumentos".to_string());
            }
            let s_lower = texto.to_lowercase();
            Ok(Valor::Logico(s_lower == "verdadero" || s_lower == "true" || s_lower == "1"))
        }
        "jsn" => {
            if !argumentos.is_empty() {
                return Err("método 'jsn()' no acepta argumentos".to_string());
            }
            
            serde_json::from_str(texto)
                .map(Valor::Json)
                .map_err(|e| format!("no se puede convertir texto a JSON: {}", e))
        }
        "lista" => {
            if !argumentos.is_empty() {
                return Err("método 'lista()' no acepta argumentos".to_string());
            }
            // Parsear lista separada por comas
            let elementos: Vec<Valor> = texto.split(',')
                .map(|elem| Valor::Texto(elem.trim().to_string()))
                .collect();
            Ok(Valor::Lista(elementos))
        }
        _ => Err(format!("método '{}' no encontrado", metodo)),
    }
}
