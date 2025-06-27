use crate::entorno::Entorno;
use crate::valores::Valor;

pub fn interpretar(contenido: &str) -> Result<(), String> {
    let mut entorno = Entorno::nuevo();
    let mut en_comentario = false;

    for (indice, linea_original) in contenido.lines().enumerate() {
        let mut linea = linea_original.trim();
        if en_comentario {
            if let Some(pos) = linea.find("*/") {
                linea = &linea[pos + 2..];
                en_comentario = false;
            } else {
                continue;
            }
        }
        if linea.starts_with("/*") {
            if let Some(pos) = linea.find("*/") {
                linea = &linea[pos + 2..];
            } else {
                en_comentario = true;
                continue;
            }
        }
        if linea.starts_with("//") || linea.is_empty() {
            continue;
        }

        if linea.starts_with("imprimir") {
            let expresion = linea.trim_start_matches("imprimir").trim();
            if expresion.starts_with('"') && expresion.ends_with('"') {
                println!("{}", expresion.trim_matches('"'));
            } else if let Some(valor) = entorno.obtener(expresion) {
                println!("{:?}", valor);
            } else {
                return Err(formatear_error(indice, "Variable no encontrada"));
            }
            continue;
        }

        if let Err(error) = procesar_declaracion(linea, &mut entorno) {
            return Err(formatear_error(indice, &error));
        }
    }
    Ok(())
}

fn procesar_declaracion(linea: &str, entorno: &mut Entorno) -> Result<(), String> {
    let tokens: Vec<&str> = linea.split_whitespace().collect();
    if tokens.len() < 4 {
        return Err("Declaración inválida".to_string());
    }
    let tipo = tokens[0];
    let mut indice = 1;
    if tokens.get(indice).copied() == Some("mut") {
        indice += 1;
    }
    let nombre = tokens.get(indice).ok_or("Falta nombre de variable")?;
    indice += 1;
    if tokens.get(indice) != Some(&"=") {
        return Err("Falta '=' en la declaración".to_string());
    }
    indice += 1;
    let valor_cadena = tokens[indice..].join(" ");
    let valor = match tipo {
        "entero" => Valor::Entero(valor_cadena.parse().map_err(|_| "Valor entero inválido")?),
        "número" => Valor::Numero(valor_cadena.parse().map_err(|_| "Valor númerico inválido")?),
        "cadena" => {
            if valor_cadena.starts_with('"') && valor_cadena.ends_with('"') {
                Valor::Cadena(valor_cadena.trim_matches('"').to_string())
            } else {
                return Err("La cadena debe ir entre comillas".to_string());
            }
        }
        "bool" => match valor_cadena.as_str() {
            "verdadero" => Valor::Bool(true),
            "falso" => Valor::Bool(false),
            _ => return Err("Valor bool inválido".to_string()),
        },
        "lista" => {
            if !valor_cadena.starts_with('[') || !valor_cadena.ends_with(']') {
                return Err("Lista inválida".to_string());
            }
            let contenido = &valor_cadena[1..valor_cadena.len() - 1];
            let mut elementos = Vec::new();
            if !contenido.trim().is_empty() {
                for texto_elemento in contenido.split(',') {
                    elementos.push(parsear_literal(texto_elemento.trim())?);
                }
            }
            Valor::Lista(elementos)
        }
        "jsn" => {
            let valor_json: serde_json::Value =
                serde_json::from_str(&valor_cadena).map_err(|_| "JSON inválido")?;
            convertir_json(&valor_json)
        }
        _ => return Err("Tipo desconocido".to_string()),
    };

    entorno.establecer(nombre, valor);
    Ok(())
}

fn parsear_literal(texto: &str) -> Result<Valor, String> {
    if texto.starts_with('"') && texto.ends_with('"') {
        Ok(Valor::Cadena(texto.trim_matches('"').to_string()))
    } else if texto == "verdadero" {
        Ok(Valor::Bool(true))
    } else if texto == "falso" {
        Ok(Valor::Bool(false))
    } else if let Ok(entero) = texto.parse::<i64>() {
        Ok(Valor::Entero(entero))
    } else if let Ok(numero) = texto.parse::<f64>() {
        Ok(Valor::Numero(numero))
    } else {
        Err("Literal inválido".to_string())
    }
}

fn convertir_json(valor: &serde_json::Value) -> Valor {
    use serde_json::Value;
    match valor {
        Value::Null => Valor::Vacio,
        Value::Bool(b) => Valor::Bool(*b),
        Value::Number(num) => {
            if let Some(i) = num.as_i64() {
                Valor::Entero(i)
            } else if let Some(f) = num.as_f64() {
                Valor::Numero(f)
            } else {
                Valor::Numero(num.as_f64().unwrap())
            }
        }
        Value::String(s) => Valor::Cadena(s.clone()),
        Value::Array(arr) => Valor::Lista(arr.iter().map(convertir_json).collect()),
        Value::Object(obj) => {
            Valor::Objeto(obj.iter().map(|(k, v)| (k.clone(), convertir_json(v))).collect())
        }
    }
}

fn formatear_error(linea: usize, mensaje: &str) -> String {
    format!("Línea {}: {}", linea + 1, mensaje)
}
