use crate::entorno::Entorno;
use crate::valores::Valor;
use crate::consola;

pub fn interpretar(contenido: &str) -> Result<(), String> {
    let mut entorno = Entorno::nuevo();
    let lineas: Vec<String> = contenido.lines().map(|l| l.to_string()).collect();
    procesar_lineas(&lineas, &mut entorno, 0)
}

fn procesar_lineas(lineas: &[String], entorno: &mut Entorno, inicio: usize) -> Result<(), String> {
    let mut en_comentario = false;
    let mut indice = 0;

    while indice < lineas.len() {
        let mut linea = lineas[indice].trim();
        indice += 1;
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

        if linea.starts_with("para") {
            let (bloque, fin) = extraer_bloque(lineas, indice - 1)?;
            procesar_bucle_para(linea, &bloque, entorno, inicio + indice - 1)?;
            indice = fin + 1;
            continue;
        }

        if linea.starts_with("imprimir_error") {
            manejar_impresion(linea, "imprimir_error", inicio + indice - 1, &entorno, consola::imprimir_error)?;
            continue;
        }
        if linea.starts_with("imprimir_advertencia") {
            manejar_impresion(linea, "imprimir_advertencia", inicio + indice - 1, &entorno, consola::imprimir_advertencia)?;
            continue;
        }
        if linea.starts_with("imprimir_informacion") {
            manejar_impresion(linea, "imprimir_informacion", inicio + indice - 1, &entorno, consola::imprimir_informacion)?;
            continue;
        }
        if linea.starts_with("imprimir_depurar") {
            manejar_impresion(linea, "imprimir_depurar", inicio + indice - 1, &entorno, consola::imprimir_depurar)?;
            continue;
        }
        if linea.starts_with("imprimir_exito") {
            manejar_impresion(linea, "imprimir_exito", inicio + indice - 1, &entorno, consola::imprimir_exito)?;
            continue;
        }
        if linea.starts_with("imprimir_alerta") {
            manejar_impresion(linea, "imprimir_alerta", inicio + indice - 1, &entorno, consola::imprimir_alerta)?;
            continue;
        }
        if linea.starts_with("imprimir_confirmacion") {
            manejar_impresion(linea, "imprimir_confirmacion", inicio + indice - 1, &entorno, consola::imprimir_confirmacion)?;
            continue;
        }
        if linea.starts_with("imprimir") {
            manejar_impresion(linea, "imprimir", inicio + indice - 1, &entorno, |t| println!("{}", t))?;
            continue;
        }

        if let Err(error) = procesar_declaracion(linea, entorno) {
            return Err(formatear_error(inicio + indice - 1, &error));
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

fn manejar_impresion<F>(linea: &str, inicio: &str, linea_num: usize, entorno: &Entorno, accion: F) -> Result<(), String>
where
    F: Fn(&str),
{
    let mut expresion = linea.trim_start_matches(inicio).trim();
    if expresion.starts_with('(') && expresion.ends_with(')') {
        expresion = expresion[1..expresion.len() - 1].trim();
    }

    let resultado = if expresion.contains('+') {
        let partes: Vec<&str> = expresion.split('+').collect();
        let mut acumulado = String::new();
        for parte in partes {
            acumulado.push_str(&valor_desde_expresion(parte, linea_num, entorno)?);
        }
        acumulado
    } else {
        valor_desde_expresion(expresion, linea_num, entorno)?
    };

    accion(&resultado);
    Ok(())
}

fn valor_desde_expresion(expresion: &str, linea_num: usize, entorno: &Entorno) -> Result<String, String> {
    let texto = expresion.trim();
    if texto.starts_with('"') && texto.ends_with('"') {
        return Ok(texto.trim_matches('"').to_string());
    }

    if let Some(base) = texto.strip_suffix(".cadena()") {
        if let Some(valor) = entorno.obtener(base.trim()) {
            return Ok(valor.a_cadena());
        } else {
            return Err(formatear_error(linea_num, "Variable no encontrada"));
        }
    }

    if let Some(valor) = entorno.obtener(texto) {
        Ok(valor.a_cadena())
    } else {
        Err(formatear_error(linea_num, "Variable no encontrada"))
    }
}

fn extraer_bloque(lineas: &[String], inicio: usize) -> Result<(Vec<String>, usize), String> {
    let mut bloque = Vec::new();
    let mut nivel = lineas[inicio].matches('{').count() as i32 - lineas[inicio].matches('}').count() as i32;
    let mut i = inicio + 1;
    while i < lineas.len() {
        let linea = &lineas[i];
        nivel += linea.matches('{').count() as i32;
        if linea.contains('}') {
            nivel -= linea.matches('}').count() as i32;
            if nivel == 0 {
                return Ok((bloque, i));
            }
        }
        bloque.push(linea.clone());
        i += 1;
    }
    Err("Bloque sin cerrar".to_string())
}

fn procesar_bucle_para(linea: &str, bloque: &[String], entorno: &mut Entorno, linea_num: usize) -> Result<(), String> {
    let texto = linea.trim();
    let inicio_paren = texto.find('(').ok_or_else(|| formatear_error(linea_num, "Bucle para inválido"))?;
    let fin_paren = texto.rfind(')').ok_or_else(|| formatear_error(linea_num, "Bucle para inválido"))?;
    let contenido = &texto[inicio_paren + 1..fin_paren];
    let partes: Vec<&str> = contenido.split(';').collect();
    if partes.len() != 3 {
        return Err(formatear_error(linea_num, "Bucle para inválido"));
    }
    procesar_declaracion(partes[0].trim(), entorno).map_err(|e| formatear_error(linea_num, &e))?;
    while evaluar_condicion(partes[1].trim(), entorno)? {
        procesar_lineas(bloque, entorno, linea_num + 1)?;
        aplicar_incremento(partes[2].trim(), entorno)?;
    }
    Ok(())
}

fn evaluar_condicion(condicion: &str, entorno: &Entorno) -> Result<bool, String> {
    let tokens: Vec<&str> = condicion.split_whitespace().collect();
    if tokens.len() != 3 {
        return Err("Condición inválida".to_string());
    }
    let izq = obtener_entero(tokens[0], entorno)?;
    let der = obtener_entero(tokens[2], entorno)?;
    match tokens[1] {
        "<" => Ok(izq < der),
        "<=" => Ok(izq <= der),
        ">" => Ok(izq > der),
        ">=" => Ok(izq >= der),
        "==" => Ok(izq == der),
        "!=" => Ok(izq != der),
        _ => Err("Operador de comparación inválido".to_string()),
    }
}

fn obtener_entero(texto: &str, entorno: &Entorno) -> Result<i64, String> {
    if let Ok(i) = texto.parse::<i64>() {
        return Ok(i);
    }
    if let Some(valor) = entorno.obtener(texto) {
        if let Valor::Entero(i) = valor {
            return Ok(*i);
        }
    }
    Err("Valor entero inválido".to_string())
}

fn aplicar_incremento(expresion: &str, entorno: &mut Entorno) -> Result<(), String> {
    if expresion.ends_with("++") {
        let nombre = expresion.trim_end_matches("++").trim();
        if let Some(Valor::Entero(i)) = entorno.obtener(nombre).cloned() {
            entorno.establecer(nombre, Valor::Entero(i + 1));
            return Ok(());
        } else {
            return Err("Variable no encontrada".to_string());
        }
    } else if expresion.ends_with("--") {
        let nombre = expresion.trim_end_matches("--").trim();
        if let Some(Valor::Entero(i)) = entorno.obtener(nombre).cloned() {
            entorno.establecer(nombre, Valor::Entero(i - 1));
            return Ok(());
        } else {
            return Err("Variable no encontrada".to_string());
        }
    }
    Err("Incremento inválido".to_string())
}
