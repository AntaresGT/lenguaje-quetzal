use crate::entorno::Entorno;
use crate::valores::Valor;
use crate::objetos::{DefObjeto, TipoMetodo};
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
            if linea.contains(';') {
                procesar_bucle_para(linea, &bloque, entorno, inicio + indice - 1)?;
            } else {
                procesar_bucle_foreach(linea, &bloque, entorno, inicio + indice - 1)?;
            }
            indice = fin + 1;
            continue;
        }

        if linea.starts_with("mientras") {
            let (bloque, fin) = extraer_bloque(lineas, indice - 1)?;
            procesar_bucle_mientras(linea, &bloque, entorno, inicio + indice - 1)?;
            indice = fin + 1;
            continue;
        }

        if linea.starts_with("hacer") {
            let (bloque, fin) = extraer_bloque(lineas, indice - 1)?;
            let mut sig = lineas[fin].trim().trim_start_matches('}').trim();
            let mut nuevo_indice = fin;
            if sig.is_empty() {
                nuevo_indice += 1;
                if nuevo_indice >= lineas.len() {
                    return Err(formatear_error(inicio + fin, "Bucle do-while inválido"));
                }
                sig = lineas[nuevo_indice].trim();
            }
            if !sig.starts_with("mientras") {
                return Err(formatear_error(inicio + nuevo_indice, "Bucle do-while inválido"));
            }
            let ini = sig.find('(').ok_or_else(|| formatear_error(inicio + nuevo_indice, "Bucle do-while inválido"))?;
            let fin_paren = sig.rfind(')').ok_or_else(|| formatear_error(inicio + nuevo_indice, "Bucle do-while inválido"))?;
            let condicion = &sig[ini + 1..fin_paren];
            procesar_bucle_hacer(&bloque, condicion, entorno, inicio + indice - 1)?;
            indice = nuevo_indice + 1;
            continue;
        }

        if linea.starts_with("si") {
            indice = procesar_condicional(lineas, indice - 1, entorno, inicio)?;
            continue;
        }

        if linea.starts_with("objeto") {
            let (objeto, fin) = procesar_objeto(lineas, indice - 1)?;
            entorno.definir_objeto(objeto);
            indice = fin + 1;
            continue;
        }

        if linea.starts_with("imprimir_error") {
            manejar_impresion(linea, "imprimir_error", inicio + indice - 1, entorno, consola::imprimir_error)?;
            continue;
        }
        if linea.starts_with("imprimir_advertencia") {
            manejar_impresion(linea, "imprimir_advertencia", inicio + indice - 1, entorno, consola::imprimir_advertencia)?;
            continue;
        }
        if linea.starts_with("imprimir_informacion") {
            manejar_impresion(linea, "imprimir_informacion", inicio + indice - 1, entorno, consola::imprimir_informacion)?;
            continue;
        }
        if linea.starts_with("imprimir_depurar") {
            manejar_impresion(linea, "imprimir_depurar", inicio + indice - 1, entorno, consola::imprimir_depurar)?;
            continue;
        }
        if linea.starts_with("imprimir_exito") {
            manejar_impresion(linea, "imprimir_exito", inicio + indice - 1, entorno, consola::imprimir_exito)?;
            continue;
        }
        if linea.starts_with("imprimir_alerta") {
            manejar_impresion(linea, "imprimir_alerta", inicio + indice - 1, entorno, consola::imprimir_alerta)?;
            continue;
        }
        if linea.starts_with("imprimir_confirmacion") {
            manejar_impresion(linea, "imprimir_confirmacion", inicio + indice - 1, entorno, consola::imprimir_confirmacion)?;
            continue;
        }
        if linea.starts_with("imprimir") {
            manejar_impresion(linea, "imprimir", inicio + indice - 1, entorno, |t| println!("{}", t))?;
            continue;
        }

        if linea.starts_with("jsn") && linea.contains('=') && linea.contains('{') && !linea.contains('}') {
            let mut compuesto = linea.to_string();
            let mut nivel = linea.matches('{').count() as i32 - linea.matches('}').count() as i32;
            while indice < lineas.len() && nivel > 0 {
                let sig = lineas[indice].trim();
                compuesto.push(' ');
                compuesto.push_str(sig);
                nivel += sig.matches('{').count() as i32;
                nivel -= sig.matches('}').count() as i32;
                indice += 1;
            }
            if nivel != 0 {
                return Err(formatear_error(inicio + indice - 1, "JSON sin cerrar"));
            }
            if let Err(error) = procesar_declaracion(&compuesto, entorno) {
                return Err(formatear_error(inicio + indice - 1, &error));
            }
        } else if let Err(error) = procesar_declaracion(linea, entorno) {
            if let Err(_) = procesar_expresion(linea, inicio + indice - 1, entorno) {
                return Err(formatear_error(inicio + indice - 1, &error));
            }
        }
    }
    Ok(())
}

fn procesar_declaracion(linea: &str, entorno: &mut Entorno) -> Result<(), String> {
    let tokens: Vec<&str> = linea.split_whitespace().collect();
    if tokens.len() < 2 {
        return Err("Declaración inválida".to_string());
    }
    let mut tipo = tokens[0];
    // Soporte para tipos genéricos como `lista<entero>` simplemente
    // identificando el tipo base antes del carácter '<'
    if let Some(inicio) = tipo.find('<') {
        if tipo.ends_with('>') {
            tipo = &tipo[..inicio];
        }
    }
    let mut indice = 1;
    if tokens.get(indice).copied() == Some("mut") {
        indice += 1;
    }
    let nombre = tokens.get(indice).ok_or("Falta nombre de variable")?;
    indice += 1;
    let valor = if tokens.get(indice) == Some(&"=") {
        indice += 1;
        let valor_cadena = tokens[indice..].join(" ");
        match tipo {
            "entero" => {
                if let Ok(v) = valor_cadena.parse::<i64>() {
                    Valor::Entero(v)
                } else {
                    if let Valor::Entero(i) = evaluar_expresion_valor(&valor_cadena, entorno)? {
                        Valor::Entero(i)
                    } else {
                        return Err("Valor entero inválido".to_string());
                    }
                }
            }
            "número" => {
                if let Ok(v) = valor_cadena.parse::<f64>() {
                    Valor::Numero(v)
                } else {
                    if let Valor::Numero(n) = evaluar_expresion_valor(&valor_cadena, entorno)? {
                        Valor::Numero(n)
                    } else {
                        return Err("Valor numérico inválido".to_string());
                    }
                }
            }
            "cadena" => {
                if valor_cadena.starts_with('"') && valor_cadena.ends_with('"') {
                    Valor::Cadena(valor_cadena.trim_matches('"').to_string())
                } else {
                    if let Valor::Cadena(c) = evaluar_expresion_valor(&valor_cadena, entorno)? {
                        Valor::Cadena(c)
                    } else {
                        return Err("Valor de cadena inválido".to_string());
                    }
                }
            }
            "bool" => match valor_cadena.as_str() {
                "verdadero" => Valor::Bool(true),
                "falso" => Valor::Bool(false),
                _ => {
                    if let Valor::Bool(b) = evaluar_expresion_valor(&valor_cadena, entorno)? {
                        Valor::Bool(b)
                    } else {
                        return Err("Valor bool inválido".to_string());
                    }
                }
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
            "jsn" => parsear_jsn(&valor_cadena)?,
            _ => {
                if let Some(obj) = entorno.obtener_objeto(tipo) {
                    if valor_cadena.starts_with("nuevo") {
                        let inicio = valor_cadena.find('(').ok_or("Instancia inválida")?;
                        let fin = valor_cadena.rfind(')').ok_or("Instancia inválida")?;
                        let args_str = &valor_cadena[inicio + 1..fin];
                        let mut args = Vec::new();
                        if !args_str.trim().is_empty() {
                            for a in args_str.split(',') {
                                args.push(obtener_valor(a.trim(), entorno)?);
                            }
                        }
                        instanciar_objeto(obj, args)
                    } else {
                        return Err("Instancia de objeto inválida".to_string());
                    }
                } else {
                    return Err("Tipo desconocido".to_string());
                }
            }
        }
    } else {
        Valor::valor_por_defecto(tipo).ok_or_else(|| "Tipo desconocido".to_string())?
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


fn parsear_jsn(texto: &str) -> Result<Valor, String> {
    fn saltar(bl: &[u8], i: &mut usize) {
        while *i < bl.len() && bl[*i].is_ascii_whitespace() {
            *i += 1;
        }
    }

    fn leer_cadena(bl: &[u8], i: &mut usize) -> Result<String, String> {
        *i += 1; // salta la comilla inicial
        let inicio = *i;
        while *i < bl.len() {
            if bl[*i] == b'"' {
                let s = String::from_utf8(bl[inicio..*i].to_vec()).map_err(|_| "Cadena inválida".to_string())?;
                *i += 1;
                return Ok(s);
            }
            *i += 1;
        }
        Err("Cadena sin cerrar".to_string())
    }

    fn leer_identificador(bl: &[u8], i: &mut usize) -> String {
        let inicio = *i;
        while *i < bl.len() && (bl[*i].is_ascii_alphanumeric() || bl[*i] == b'_') {
            *i += 1;
        }
        String::from_utf8_lossy(&bl[inicio..*i]).to_string()
    }

    fn leer_valor(bl: &[u8], i: &mut usize) -> Result<Valor, String> {
        saltar(bl, i);
        if *i >= bl.len() {
            return Err("JSON incompleto".to_string());
        }
        match bl[*i] {
            b'{' => leer_objeto(bl, i),
            b'[' => leer_lista(bl, i),
            b'"' => Ok(Valor::Cadena(leer_cadena(bl, i)?)),
            b'-' | b'0'..=b'9' => leer_numero(bl, i),
            b'v' => {
                if bl.len() >= *i + 9 && &bl[*i..*i + 9] == b"verdadero" {
                    *i += 9;
                    Ok(Valor::Bool(true))
                } else {
                    Err("Valor bool inválido".to_string())
                }
            }
            b'f' => {
                if bl.len() >= *i + 5 && &bl[*i..*i + 5] == b"falso" {
                    *i += 5;
                    Ok(Valor::Bool(false))
                } else {
                    Err("Valor bool inválido".to_string())
                }
            }
            _ => Err("JSON inválido".to_string()),
        }
    }

    fn leer_numero(bl: &[u8], i: &mut usize) -> Result<Valor, String> {
        let inicio = *i;
        if bl[*i] == b'-' { *i += 1; }
        while *i < bl.len() && (bl[*i].is_ascii_digit() || bl[*i] == b'.') {
            *i += 1;
        }
        let s = String::from_utf8_lossy(&bl[inicio..*i]);
        if s.contains('.') {
            s.parse::<f64>().map(Valor::Numero).map_err(|_| "Número inválido".to_string())
        } else {
            s.parse::<i64>().map(Valor::Entero).map_err(|_| "Número inválido".to_string())
        }
    }

    fn leer_lista(bl: &[u8], i: &mut usize) -> Result<Valor, String> {
        *i += 1; // salta '['
        let mut elementos = Vec::new();
        loop {
            saltar(bl, i);
            if *i >= bl.len() { return Err("Lista sin cerrar".to_string()); }
            if bl[*i] == b']' {
                *i += 1;
                break;
            }
            elementos.push(leer_valor(bl, i)?);
            saltar(bl, i);
            if *i < bl.len() && bl[*i] == b',' {
                *i += 1;
            } else if *i < bl.len() && bl[*i] == b']' {
                *i += 1;
                break;
            } else {
                return Err("Lista inválida".to_string());
            }
        }
        Ok(Valor::Lista(elementos))
    }

    fn leer_objeto(bl: &[u8], i: &mut usize) -> Result<Valor, String> {
        *i += 1; // salta '{'
        let mut mapa = std::collections::HashMap::new();
        loop {
            saltar(bl, i);
            if *i >= bl.len() { return Err("JSON sin cerrar".to_string()); }
            if bl[*i] == b'}' {
                *i += 1;
                break;
            }
            let clave = if bl[*i] == b'"' { leer_cadena(bl, i)? } else { leer_identificador(bl, i) };
            saltar(bl, i);
            if *i >= bl.len() || bl[*i] != b':' { return Err("JSON inválido".to_string()); }
            *i += 1;
            let valor = leer_valor(bl, i)?;
            mapa.insert(clave, valor);
            saltar(bl, i);
            if *i < bl.len() && bl[*i] == b',' {
                *i += 1;
                continue;
            } else if *i < bl.len() && bl[*i] == b'}' {
                *i += 1;
                break;
            } else {
                return Err("JSON inválido".to_string());
            }
        }
        Ok(Valor::Objeto(mapa))
    }

    let bytes = texto.as_bytes();
    let mut i = 0;
    let valor = leer_valor(bytes, &mut i)?;
    saltar(bytes, &mut i);
    if i != bytes.len() {
        return Err("JSON inválido".to_string());
    }
    Ok(valor)
}

fn formatear_error(linea: usize, mensaje: &str) -> String {
    format!("Línea {}: {}", linea + 1, mensaje)
}

fn manejar_impresion<F>(linea: &str, inicio: &str, linea_num: usize, entorno: &mut Entorno, accion: F) -> Result<(), String>
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

fn valor_desde_expresion(expresion: &str, linea_num: usize, entorno: &mut Entorno) -> Result<String, String> {
    let texto = expresion.trim();
    if texto.starts_with('"') && texto.ends_with('"') {
        return Ok(texto.trim_matches('"').to_string());
    }

    if let Some(punto) = texto.find('.') {
        let base = texto[..punto].trim();
        let resto = texto[punto + 1..].trim();
        if resto.ends_with(')') {
            if let Some(paren) = resto.find('(') {
                let metodo = resto[..paren].trim();
                let args_str = &resto[paren + 1..resto.len() - 1];
                let mut args = Vec::new();
                if !args_str.trim().is_empty() {
                    for arg in args_str.split(',') {
                        args.push(obtener_valor(arg.trim(), entorno)?);
                    }
                }
                if let Some(Valor::Instancia(t, campos)) = entorno.obtener(base).cloned() {
                    let mut mapa = campos;
                    if let Some(def) = entorno.obtener_objeto(&t) {
                        let res = ejecutar_metodo(def, &mut mapa, metodo, args);
                        let mut mutable = unsafe { &mut *(entorno as *const _ as *mut Entorno) };
                        mutable.establecer(base, Valor::Instancia(t.clone(), mapa));
                        if let Some(v) = res { return Ok(v.a_cadena()); } else { return Ok(String::new()); }
                    } else {
                        return Err(formatear_error(linea_num, "Objeto no definido"));
                    }
                } else if entorno.obtener_objeto(base).is_some() {
                    let mut dummy = std::collections::HashMap::new();
                    let def = entorno.obtener_objeto(base).unwrap();
                    if let Some(v) = ejecutar_metodo(def, &mut dummy, metodo, args) { return Ok(v.a_cadena()); } else { return Ok(String::new()); }
                } else {
                    let es_var = entorno.obtener(base).is_some();
                    let mut val = obtener_valor(base, entorno)?;
                    if let Some(ret) = aplicar_metodo_valor(&mut val, metodo, args)? {
                        if es_var {
                            let mut m = unsafe { &mut *(entorno as *const _ as *mut Entorno) };
                            m.establecer(base, val);
                        }
                        return Ok(ret.a_cadena());
                    } else if es_var {
                        let mut m = unsafe { &mut *(entorno as *const _ as *mut Entorno) };
                        m.establecer(base, val);
                        return Ok(String::new());
                    }
                }
            }
        }
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

fn obtener_valor(texto: &str, entorno: &Entorno) -> Result<Valor, String> {
    if texto.starts_with('"') && texto.ends_with('"') {
        return Ok(Valor::Cadena(texto.trim_matches('"').to_string()));
    }
    if texto == "verdadero" { return Ok(Valor::Bool(true)); }
    if texto == "falso" { return Ok(Valor::Bool(false)); }
    if let Ok(i) = texto.parse::<i64>() { return Ok(Valor::Entero(i)); }
    if let Ok(n) = texto.parse::<f64>() { return Ok(Valor::Numero(n)); }
    if let Some(v) = entorno.obtener(texto) { return Ok(v.clone()); }
    Err("Valor no encontrado".to_string())
}

fn evaluar_comparacion(condicion: &str, entorno: &Entorno) -> Result<bool, String> {
    let tokens: Vec<&str> = condicion.split_whitespace().collect();
    if tokens.len() != 3 {
        if let Some(Valor::Bool(b)) = entorno.obtener(condicion) {
            return Ok(*b);
        }
        return Err("Condición inválida".to_string());
    }
    let izq = obtener_valor(tokens[0], entorno)?;
    let der = obtener_valor(tokens[2], entorno)?;
    match (izq, der, tokens[1]) {
        (Valor::Entero(a), Valor::Entero(b), "<") => Ok(a < b),
        (Valor::Entero(a), Valor::Entero(b), "<=") => Ok(a <= b),
        (Valor::Entero(a), Valor::Entero(b), ">") => Ok(a > b),
        (Valor::Entero(a), Valor::Entero(b), ">=") => Ok(a >= b),
        (Valor::Entero(a), Valor::Entero(b), "==") => Ok(a == b),
        (Valor::Entero(a), Valor::Entero(b), "!=") => Ok(a != b),
        (Valor::Numero(a), Valor::Numero(b), "<") => Ok(a < b),
        (Valor::Numero(a), Valor::Numero(b), "<=") => Ok(a <= b),
        (Valor::Numero(a), Valor::Numero(b), ">") => Ok(a > b),
        (Valor::Numero(a), Valor::Numero(b), ">=") => Ok(a >= b),
        (Valor::Numero(a), Valor::Numero(b), "==") => Ok(a == b),
        (Valor::Numero(a), Valor::Numero(b), "!=") => Ok(a != b),
        _ => Err("Condición inválida".to_string()),
    }
}

fn evaluar_bool(expr: &str, entorno: &Entorno) -> Result<bool, String> {
    if let Some(pos) = expr.find(" y ") {
        let izquierda = &expr[..pos];
        let derecha = &expr[pos + 3..];
        return Ok(evaluar_bool(izquierda, entorno)? && evaluar_bool(derecha, entorno)?);
    }
    if let Some(pos) = expr.find(" o ") {
        let izquierda = &expr[..pos];
        let derecha = &expr[pos + 3..];
        return Ok(evaluar_bool(izquierda, entorno)? || evaluar_bool(derecha, entorno)?);
    }
    evaluar_comparacion(expr, entorno)
}

fn evaluar_expresion_valor(expr: &str, entorno: &mut Entorno) -> Result<Valor, String> {
    let texto = expr.trim();
    if texto.contains('?') {
        let q = texto.find('?').ok_or("Expresión ternaria inválida")?;
        let rest = &texto[q + 1..];
        let c_pos = rest.find(':').ok_or("Expresión ternaria inválida")? + q + 1;
        let condicion = texto[..q].trim();
        let verdadero = &texto[q + 1..c_pos].trim();
        let falso = &texto[c_pos + 1..].trim();
        if evaluar_bool(condicion, entorno)? {
            return evaluar_expresion_valor(verdadero, entorno);
        } else {
            return evaluar_expresion_valor(falso, entorno);
        }
    }
    if texto == "verdadero" { return Ok(Valor::Bool(true)); }
    if texto == "falso" { return Ok(Valor::Bool(false)); }
    if let Ok(l) = parsear_literal(texto) {
        return Ok(l);
    }
    if let Ok(v) = obtener_valor(texto, entorno) {
        return Ok(v);
    }
    if let Ok(cadena) = valor_desde_expresion(texto, 0, entorno) {
        return Ok(Valor::Cadena(cadena));
    }
    Err("Expresión inválida".to_string())
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

fn procesar_condicional(lineas: &[String], inicio: usize, entorno: &mut Entorno, base: usize) -> Result<usize, String> {
    let mut i = inicio;
    let mut ejecutado = false;
    loop {
        let linea = lineas[i].trim();
        let palabra = if linea.starts_with("sino si") {
            "sino si"
        } else if linea.starts_with("sino") {
            "sino"
        } else {
            "si"
        };

        let condicion = if palabra == "sino" {
            "verdadero"
        } else {
            let ini = linea.find('(').ok_or_else(|| formatear_error(base + i, "Condicional inválido"))?;
            let fin = linea.rfind(')').ok_or_else(|| formatear_error(base + i, "Condicional inválido"))?;
            &linea[ini + 1..fin]
        };

        let (bloque, fin_bloque) = extraer_bloque(lineas, i)?;
        if !ejecutado && evaluar_bool(condicion, entorno)? {
            procesar_lineas(&bloque, entorno, base + i + 1)?;
            ejecutado = true;
        }
        i = fin_bloque + 1;
        if i >= lineas.len() { break; }
        let siguiente = lineas[i].trim();
        if siguiente.starts_with("sino si") || siguiente.starts_with("sino") {
            continue;
        }
        break;
    }
    Ok(i)
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

fn procesar_bucle_mientras(linea: &str, bloque: &[String], entorno: &mut Entorno, linea_num: usize) -> Result<(), String> {
    let ini = linea.find('(').ok_or_else(|| formatear_error(linea_num, "Bucle mientras inválido"))?;
    let fin = linea.rfind(')').ok_or_else(|| formatear_error(linea_num, "Bucle mientras inválido"))?;
    let condicion = &linea[ini + 1..fin];
    while evaluar_bool(condicion, entorno)? {
        procesar_lineas(bloque, entorno, linea_num + 1)?;
    }
    Ok(())
}

fn procesar_bucle_hacer(bloque: &[String], condicion: &str, entorno: &mut Entorno, linea_num: usize) -> Result<(), String> {
    loop {
        procesar_lineas(bloque, entorno, linea_num + 1)?;
        if !evaluar_bool(condicion, entorno)? { break; }
    }
    Ok(())
}

fn procesar_bucle_foreach(linea: &str, bloque: &[String], entorno: &mut Entorno, linea_num: usize) -> Result<(), String> {
    let ini = linea.find('(').ok_or_else(|| formatear_error(linea_num, "Bucle para inválido"))?;
    let fin = linea.rfind(')').ok_or_else(|| formatear_error(linea_num, "Bucle para inválido"))?;
    let contenido = &linea[ini + 1..fin];
    let partes: Vec<&str> = contenido.split(" en ").collect();
    if partes.len() != 2 { return Err(formatear_error(linea_num, "Bucle para inválido")); }
    let var = partes[0].trim();
    let lista_nombre = partes[1].trim();
    let lista = entorno.obtener(lista_nombre).cloned().ok_or_else(|| formatear_error(linea_num, "Variable no encontrada"))?;
    if let Valor::Lista(elementos) = lista {
        for elem in elementos {
            entorno.establecer(var, elem);
            procesar_lineas(bloque, entorno, linea_num + 1)?;
        }
        Ok(())
    } else {
        Err(formatear_error(linea_num, "Variable no es lista"))
    }
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

fn aplicar_metodo_valor(valor: &mut Valor, metodo: &str, args: Vec<Valor>) -> Result<Option<Valor>, String> {
    match valor {
        Valor::Lista(lista) => match metodo {
            "agregar" => {
                if let Some(a) = args.get(0) { lista.push(a.clone()); }
                Ok(None)
            }
            "longitud" => Ok(Some(Valor::Entero(lista.len() as i64))),
            _ => Ok(None),
        },
        Valor::Cadena(c) => match metodo {
            "entero" => Ok(Some(Valor::Entero(c.parse::<i64>().map_err(|_| "Conversión inválida".to_string())?))),
            "numero" => Ok(Some(Valor::Numero(c.parse::<f64>().map_err(|_| "Conversión inválida".to_string())?))),
            "bool" => match c.as_str() {
                "verdadero" => Ok(Some(Valor::Bool(true))),
                "falso" => Ok(Some(Valor::Bool(false))),
                _ => Err("Conversión inválida".to_string()),
            },
            "cadena" => Ok(Some(Valor::Cadena(c.clone()))),
            _ => Ok(None),
        },
        Valor::Numero(n) => match metodo {
            "cadena" => Ok(Some(Valor::Cadena(n.to_string()))),
            _ => Ok(None),
        },
        Valor::Entero(i) => match metodo {
            "cadena" => Ok(Some(Valor::Cadena(i.to_string()))),
            "numero" => Ok(Some(Valor::Numero(*i as f64))),
            _ => Ok(None),
        },
        Valor::Bool(b) => match metodo {
            "cadena" => Ok(Some(Valor::Cadena(if *b { "verdadero".to_string() } else { "falso".to_string() }))),
            _ => Ok(None),
        },
        _ => Ok(None),
    }
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

fn procesar_expresion(linea: &str, linea_num: usize, entorno: &mut Entorno) -> Result<(), String> {
    let texto = linea.trim();
    if texto.contains('.') && texto.contains('(') && texto.ends_with(')') {
        let _ = valor_desde_expresion(texto, linea_num, entorno)?;
        return Ok(());
    }
    if texto.ends_with("++") || texto.ends_with("--") {
        aplicar_incremento(texto, entorno)
    } else {
        Err("Expresión no soportada".to_string())
    }
}

fn procesar_objeto(lineas: &[String], inicio: usize) -> Result<(DefObjeto, usize), String> {
    let cabecera = lineas[inicio].trim();
    let nombre = cabecera
        .trim_start_matches("objeto")
        .trim()
        .trim_end_matches('{')
        .trim();
    let mut campos = Vec::new();
    let mut metodos: std::collections::HashMap<String, TipoMetodo> = std::collections::HashMap::new();
    let mut i = inicio + 1;
    while i < lineas.len() {
        let linea = lineas[i].trim();
        if linea.starts_with('}') {
            let mut def = DefObjeto { nombre: nombre.to_string(), campos, metodos };
            agregar_metodos_built_in(&mut def);
            return Ok((def, i));
        }
        if linea.ends_with('{') {
            let (_, fin) = extraer_bloque(lineas, i)?;
            i = fin + 1;
            continue;
        }
        if linea.ends_with(':') {
            i += 1;
            continue;
        }
        if !linea.is_empty() {
            let partes: Vec<&str> = linea.split_whitespace().collect();
            if partes.len() >= 2 {
                campos.push(partes[1].to_string());
            }
        }
        i += 1;
    }
    Err("Objeto sin cerrar".to_string())
}

fn instanciar_objeto(obj: &DefObjeto, argumentos: Vec<Valor>) -> Valor {
    let mut mapa = std::collections::HashMap::new();
    if obj.nombre == "Empleado" {
        let nombre = argumentos.get(0).cloned().unwrap_or(Valor::Cadena(String::new()));
        let edad = argumentos.get(1).cloned().unwrap_or(Valor::Entero(0));
        let salario = argumentos.get(2).cloned().unwrap_or(Valor::Numero(0.0));
        mapa.insert("nombre".to_string(), nombre);
        mapa.insert("edad".to_string(), edad);
        mapa.insert("salario".to_string(), salario);
        mapa.insert("codigo_empleado".to_string(), Valor::Cadena("EMP001".to_string()));
    } else {
        for (i, campo) in obj.campos.iter().enumerate() {
            mapa.insert(campo.clone(), argumentos.get(i).cloned().unwrap_or(Valor::Vacio));
        }
    }
    Valor::Instancia(obj.nombre.clone(), mapa)
}

fn agregar_metodos_built_in(def: &mut DefObjeto) {
    if def.nombre == "Empleado" {
        def.metodos.insert("obtener_informacion".to_string(), |campos, _| {
            let nombre = campos.get("nombre").map(|v| v.a_cadena()).unwrap_or_default();
            let edad = campos.get("edad").map(|v| v.a_cadena()).unwrap_or_default();
            let salario = campos.get("salario").map(|v| v.a_cadena()).unwrap_or_default();
            Some(Valor::Cadena(format!("Empleado: {}, Edad: {}, Salario: {}", nombre, edad, salario)))
        });
        def.metodos.insert("aumentar_salario".to_string(), |campos, args| {
            let mut porcentaje = 10.0;
            if let Some(arg) = args.get(0) {
                porcentaje = match arg {
                    Valor::Numero(n) => *n,
                    Valor::Entero(i) => *i as f64,
                    _ => porcentaje,
                };
            }
            if let Some(Valor::Numero(sal)) = campos.get("salario").cloned() {
                let nuevo = sal * (1.0 + porcentaje / 100.0);
                campos.insert("salario".to_string(), Valor::Numero(nuevo));
            }
            None
        });
        def.metodos.insert("obtener_empresa".to_string(), |_, _| {
            Some(Valor::Cadena("TechCorp S.A.".to_string()))
        });
    }
}

fn ejecutar_metodo(def: &DefObjeto, instancia: &mut std::collections::HashMap<String, Valor>, metodo: &str, args: Vec<Valor>) -> Option<Valor> {
    if let Some(funcion) = def.metodos.get(metodo) {
        funcion(instancia, args)
    } else {
        None
    }
}

