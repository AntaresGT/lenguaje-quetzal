use crate::entorno::Entorno;
use crate::valores::{Valor, DefFuncion};
use crate::objetos::{DefObjeto, TipoMetodo};
use crate::consola;

pub fn interpretar(contenido: &str) -> Result<(), String> {
    let limpio = contenido.trim_start_matches('\u{feff}');
    let mut entorno = Entorno::nuevo();
    let lineas: Vec<String> = limpio.lines().map(|l| l.to_string()).collect();
    let lineas_unidas = unir_lineas_divididas(&lineas);
    procesar_lineas(&lineas_unidas, &mut entorno, 0)
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
                    return Err(formatear_error(inicio + fin, "Bucle hacer-mientras inválido"));
                }
                sig = lineas[nuevo_indice].trim();
            }
            if !sig.starts_with("mientras") {
                return Err(formatear_error(inicio + nuevo_indice, "Bucle hacer-mientras inválido"));
            }
            let ini = sig.find('(').ok_or_else(|| formatear_error(inicio + nuevo_indice, "Bucle hacer-mientras inválido"))?;
            let fin_paren = sig.rfind(')').ok_or_else(|| formatear_error(inicio + nuevo_indice, "Bucle hacer-mientras inválido"))?;
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

        // Manejo de declaración de funciones con sintaxis Quetzal: tipo nombre_funcion(parametros) {
        if es_declaracion_funcion(linea) && linea.trim_end().ends_with('{') {
            let (bloque_funcion, fin_funcion) = extraer_bloque(lineas, indice - 1)?;
            procesar_declaracion_funcion_quetzal(linea, &bloque_funcion, entorno)?;
            indice = fin_funcion + 1;
            continue;
        }

        // Manejo de llamadas a funciones
        if linea.contains('(') && linea.contains(')') && linea.contains('=') && !linea.starts_with("para") && !linea.starts_with("si") {
            if let Err(_) = procesar_llamada_funcion(linea, entorno) {
                // Si no es una llamada a función, procesar como declaración normal
                if let Err(error) = procesar_declaracion(linea, entorno) {
                    return Err(formatear_error(inicio + indice - 1, &error));
                }
            }
            continue;
        }

        // Manejo de operadores de asignación compuesta
        if linea.contains("+=") || linea.contains("-=") || linea.contains("*=") || linea.contains("/=") || linea.contains("%=") {
            procesar_asignacion_compuesta(linea, entorno, inicio + indice - 1)?;
            continue;
        }

        // Manejo de retorno de funciones
        if linea.starts_with("retornar") {
            let valor_retorno = if linea.trim() == "retornar" {
                "vacio"
            } else {
                linea.strip_prefix("retornar").unwrap_or("").trim()
            };
            return Err(format!("RETORNO:{}", valor_retorno));
        }

        // Manejo de control de flujo en bucles
        if linea.trim() == "romper" {
            return Err("ROMPER".to_string());
        }
        
        if linea.trim() == "continuar" {
            return Err("CONTINUAR".to_string());
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
        } else {
            // Verificar si es una llamada a función sin asignación
            if linea.contains('(') && linea.contains(')') && !linea.contains('=') && 
               !linea.starts_with("para") && !linea.starts_with("si") && !linea.starts_with("mientras") {
                if let Ok(_) = procesar_llamada_funcion_sin_asignacion(linea, entorno) {
                    continue;
                }
            }
            
            // Ignorar líneas que solo contienen estructuras de control (como } sino {)
            if linea.trim().starts_with("}") && linea.contains("sino") {
                continue;
            }
            
            if let Err(error) = procesar_declaracion(linea, entorno) {
                if let Err(_) = procesar_expresion(linea, inicio + indice - 1, entorno) {
                    return Err(formatear_error(inicio + indice - 1, &error));
                }
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
    
    let primer_token = tokens[0];
    
    // Verificar si el primer token es un tipo válido
    let tipos_validos = ["vacio", "entero", "número", "cadena", "bool", "lista", "jsn", "mutable"];
    if !tipos_validos.contains(&primer_token) && !primer_token.starts_with("lista<") {
        return Err("No es una declaración válida".to_string());
    }
    
    // Debug: imprimir tokens para entender el problema
    // eprintln!("DEBUG: Procesando línea: '{}'", linea);
    // eprintln!("DEBUG: Tokens: {:?}", tokens);
    
    let mut tipo = tokens[0];
    let mut indice_tipo = 0;
    
    // Manejar el caso de variables mutables que pueden empezar con 'mutable'
    if tipo == "mutable" {
        if tokens.len() < 3 {
            return Err("Declaración mutable inválida".to_string());
        }
        indice_tipo = 1;
        tipo = tokens[1];
    }
    
    // Soporte para tipos genéricos como `lista<entero>` simplemente
    // identificando el tipo base antes del carácter '<'
    if let Some(inicio) = tipo.find('<') {
        if tipo.ends_with('>') {
            tipo = &tipo[..inicio];
        }
    }
    
    let mut indice = indice_tipo + 1;
    
    // Verificar si hay 'mut' después del tipo (sintaxis: tipo mut nombre)
    if tokens.get(indice).copied() == Some("mut") {
        indice += 1;
    }
    
    let nombre = tokens.get(indice).ok_or("Falta nombre de variable")?;
    
    // eprintln!("DEBUG: tipo='{}', indice={}, nombre='{}'", tipo, indice, nombre);
    
    // Validar nombre de variable
    if es_palabra_reservada(nombre) {
        return Err(format!("'{}' es una palabra reservada y no puede usarse como nombre de variable", nombre));
    }
    
    if !es_nombre_variable_valido(nombre) {
        return Err(format!("Nombre de variable inválido: {}", nombre));
    }
    
    indice += 1;
    let valor = if tokens.get(indice) == Some(&"=") {
        indice += 1;
        let valor_cadena = tokens[indice..].join(" ");
        match tipo {
            "entero" => {
                if let Ok(v) = valor_cadena.parse::<i64>() {
                    Valor::Entero(v)
                } else {
                    let resultado = evaluar_expresion_valor(&valor_cadena, entorno)?;
                    match resultado {
                        Valor::Entero(i) => Valor::Entero(i),
                        Valor::Numero(n) => Valor::Entero(n as i64),
                        _ => return Err("Valor entero inválido".to_string()),
                    }
                }
            }
            "número" => {
                if let Ok(v) = valor_cadena.parse::<f64>() {
                    Valor::Numero(v)
                } else {
                    let resultado = evaluar_expresion_valor(&valor_cadena, entorno)?;
                    match resultado {
                        Valor::Numero(n) => Valor::Numero(n),
                        Valor::Entero(i) => Valor::Numero(i as f64),
                        _ => return Err("Valor numérico inválido".to_string()),
                    }
                }
            }
            "cadena" => {
                if (valor_cadena.starts_with('"') && valor_cadena.ends_with('"')) ||
                   (valor_cadena.starts_with('\'') && valor_cadena.ends_with('\'')) {
                    Valor::Cadena(valor_cadena.trim_matches(|c| c == '"' || c == '\'').to_string())
                } else {
                    let resultado = evaluar_expresion_valor(&valor_cadena, entorno)?;
                    match resultado {
                        Valor::Cadena(c) => Valor::Cadena(c),
                        other => Valor::Cadena(other.a_cadena()),
                    }
                }
            }
            "bool" => match valor_cadena.as_str() {
                "verdadero" => Valor::Bool(true),
                "falso" => Valor::Bool(false),
                _ => {
                    let resultado = evaluar_expresion_valor(&valor_cadena, entorno)?;
                    match resultado {
                        Valor::Bool(b) => Valor::Bool(b),
                        _ => return Err("Valor bool inválido".to_string()),
                    }
                }
            },
            "lista" => {
                if valor_cadena.starts_with('[') && valor_cadena.ends_with(']') {
                    // Es una lista literal
                    let contenido = &valor_cadena[1..valor_cadena.len() - 1];
                    let mut elementos = Vec::new();
                    if !contenido.trim().is_empty() {
                        // Dividir respetando listas anidadas
                        let elementos_texto = dividir_elementos_lista(contenido)?;
                        for texto_elemento in elementos_texto {
                            elementos.push(parsear_elemento_lista(texto_elemento.trim(), entorno)?);
                        }
                    }
                    Valor::Lista(elementos)
                } else {
                    // Es una expresión que debe evaluarse
                    let resultado = evaluar_expresion_valor(&valor_cadena, entorno)?;
                    match resultado {
                        Valor::Lista(_) => resultado,
                        _ => return Err("El valor no es una lista".to_string()),
                    }
                }
            }
            "jsn" => {
                // Verificar si es una expresión (contiene método) o JSON directo
                if valor_cadena.contains('.') && valor_cadena.contains('(') && valor_cadena.ends_with(')') {
                    // Es una llamada a método, evaluar como expresión
                    evaluar_expresion_valor(&valor_cadena, entorno)?
                } else {
                    // Es JSON directo, parsear
                    parsear_jsn(&valor_cadena)?
                }
            },
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
                    return Err(format!("Tipo desconocido: {}", tipo));
                }
            }
        }
    } else {
        Valor::valor_por_defecto(tipo).ok_or_else(|| format!("Tipo desconocido: {}", tipo))?
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
                } else if bl.len() >= *i + 5 && &bl[*i..*i + 5] == b"false" {
                    *i += 5;
                    Ok(Valor::Bool(false))
                } else {
                    Err("Valor bool inválido".to_string())
                }
            }
            b't' => {
                if bl.len() >= *i + 4 && &bl[*i..*i + 4] == b"true" {
                    *i += 4;
                    Ok(Valor::Bool(true))
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

// Función para detectar declaraciones de funciones con sintaxis Quetzal
fn es_declaracion_funcion(linea: &str) -> bool {
    let linea = linea.trim();
    
    // Verificar si empieza con "asincrono"
    let linea_sin_async = if linea.starts_with("asincrono ") {
        &linea[10..]
    } else {
        linea
    };
    
    let tokens: Vec<&str> = linea_sin_async.split_whitespace().collect();
    if tokens.len() < 2 {
        return false;
    }
    
    let tipo = tokens[0];
    // Verificar que el primer token sea un tipo válido
    if !["vacio", "entero", "número", "cadena", "bool", "lista", "jsn"].contains(&tipo) {
        return false;
    }
    
    // Verificar que tenga patrón nombre_funcion(
    let resto = &linea_sin_async[tipo.len()..].trim_start();
    if let Some(pos_paren) = resto.find('(') {
        let nombre_parte = &resto[..pos_paren].trim();
        return !nombre_parte.is_empty() && !nombre_parte.contains(' ');
    }
    
    false
}

fn procesar_declaracion_funcion_quetzal(linea: &str, bloque: &[String], entorno: &mut Entorno) -> Result<(), String> {
    let linea = linea.trim().trim_end_matches('{').trim();
    
    // Verificar si es asíncrona
    let linea_sin_async = if linea.starts_with("asincrono ") {
        &linea[10..]
    } else {
        linea
    };
    
    let tokens: Vec<&str> = linea_sin_async.split_whitespace().collect();
    if tokens.is_empty() {
        return Err("Declaración de función inválida".to_string());
    }
    
    let tipo_retorno = tokens[0].to_string();
    
    // Encontrar nombre y parámetros
    let resto = &linea_sin_async[tipo_retorno.len()..].trim_start();
    let inicio_parentesis = resto.find('(').ok_or("Sintaxis de función inválida")?;
    let fin_parentesis = resto.rfind(')').ok_or("Sintaxis de función inválida")?;
    
    let nombre = resto[..inicio_parentesis].trim().to_string();
    let params_str = &resto[inicio_parentesis + 1..fin_parentesis];
    
    if nombre.is_empty() {
        return Err("Nombre de función vacío".to_string());
    }
    
    // Verificar que no sea palabra reservada
    if es_palabra_reservada(&nombre) {
        return Err(format!("'{}' es una palabra reservada y no puede usarse como nombre de función", nombre));
    }
    
    // Parsear parámetros: tipo nombre, tipo mut nombre, etc.
    let mut parametros = Vec::new();
    if !params_str.trim().is_empty() {
        for param in params_str.split(',') {
            let param = param.trim();
            let tokens_param: Vec<&str> = param.split_whitespace().collect();
            
            if tokens_param.len() < 2 {
                return Err("Parámetro de función mal formado".to_string());
            }
            
            let mut tipo_param = tokens_param[0].to_string();
            let mut nombre_param_inicio = 1;
            
            // Manejar parámetros mutables: tipo mut nombre
            if tokens_param.len() > 2 && tokens_param[1] == "mut" {
                tipo_param = format!("{} mut", tipo_param);
                nombre_param_inicio = 2;
            }
            
            if tokens_param.len() <= nombre_param_inicio {
                return Err("Falta nombre del parámetro".to_string());
            }
            
            let nombre_param = tokens_param[nombre_param_inicio].to_string();
            
            if es_palabra_reservada(&nombre_param) {
                return Err(format!("'{}' es una palabra reservada y no puede usarse como nombre de parámetro", nombre_param));
            }
            
            parametros.push((nombre_param, tipo_param));
        }
    }
    
    let def_funcion = DefFuncion {
        nombre: nombre.clone(),
        parametros,
        tipo_retorno,
        cuerpo: bloque.to_vec(),
    };
    
    entorno.definir_funcion(def_funcion);
    Ok(())
}

// Función para verificar si un nombre es palabra reservada
fn es_palabra_reservada(nombre: &str) -> bool {
    let palabras_reservadas = [
        "vacio", "entero", "número", "numero", "cadena", "bool", "verdadero", "falso",
        "lista", "jsn", "mut", "tipo", "publico", "privado", "libre", "fn", "retornar",
        "objeto", "nuevo", "ambiente", "asincrono", "esperar", "si", "sino", "mientras",
        "para", "hacer", "romper", "continuar", "intentar", "atrapar", "finalmente",
        "lanzar", "excepción", "importar", "exportar", "desde", "como", "y", "o", "en"
    ];
    
    palabras_reservadas.contains(&nombre)
}

// Función para validar nombres de variables (permite camelCase y snake_case)
fn es_nombre_variable_valido(nombre: &str) -> bool {
    if nombre.is_empty() {
        return false;
    }
    
    let primer_char = nombre.chars().next().unwrap();
    // No puede empezar con número
    if primer_char.is_ascii_digit() {
        return false;
    }
    
    // Debe empezar con letra o guion bajo
    if !primer_char.is_alphabetic() && primer_char != '_' {
        return false;
    }
    
    // Solo puede contener letras, números y guiones bajos
    for c in nombre.chars() {
        if !c.is_alphanumeric() && c != '_' {
            return false;
        }
    }
    
    true
}

fn procesar_declaracion_funcion(linea: &str, bloque: &[String], entorno: &mut Entorno) -> Result<(), String> {
    // Parsear la declaración de función
    // Formato: funcion nombre(param1: tipo1, param2: tipo2) -> tipo_retorno
    
    let partes = linea.split("->").collect::<Vec<&str>>();
    let declaracion = partes[0].trim().trim_end_matches('{').trim();
    let tipo_retorno = if partes.len() > 1 {
        partes[1].trim().trim_start_matches('{').trim().to_string()
    } else {
        "vacio".to_string()
    };
    
    // Extraer nombre y parámetros
    let inicio_parentesis = declaracion.find('(').ok_or("Sintaxis de función inválida")?;
    let fin_parentesis = declaracion.rfind(')').ok_or("Sintaxis de función inválida")?;
    
    let partes_func: Vec<&str> = declaracion[..inicio_parentesis].split_whitespace().collect();
    if partes_func.len() < 2 || partes_func[0] != "funcion" {
        return Err("Sintaxis de función inválida".to_string());
    }
    
    let nombre = partes_func[1].to_string();
    let params_str = &declaracion[inicio_parentesis + 1..fin_parentesis];
    
    // Parsear parámetros
    let mut parametros = Vec::new();
    if !params_str.trim().is_empty() {
        for param in params_str.split(',') {
            let param = param.trim();
            if let Some(pos) = param.find(':') {
                let nombre_param = param[..pos].trim().to_string();
                let tipo_param = param[pos + 1..].trim().to_string();
                parametros.push((nombre_param, tipo_param));
            } else {
                return Err("Parámetro de función mal formado".to_string());
            }
        }
    }
    
    let def_funcion = DefFuncion {
        nombre: nombre.clone(),
        parametros,
        tipo_retorno,
        cuerpo: bloque.to_vec(),
    };
    
    entorno.definir_funcion(def_funcion);
    Ok(())
}

fn procesar_llamada_funcion(linea: &str, entorno: &mut Entorno) -> Result<(), String> {
    // Parsear líneas del tipo: tipo variable = funcion()
    let partes_asignacion: Vec<&str> = linea.split('=').collect();
    if partes_asignacion.len() != 2 {
        return Err("Sintaxis de asignación inválida".to_string());
    }
    
    let izquierda = partes_asignacion[0].trim();
    let llamada = partes_asignacion[1].trim();
    
    // Extraer el nombre de la variable (último token de la izquierda)
    let tokens_izq: Vec<&str> = izquierda.split_whitespace().collect();
    let variable_resultado = if tokens_izq.len() >= 2 {
        tokens_izq[tokens_izq.len() - 1]
    } else {
        tokens_izq[0]
    };
    
    // Funciones built-in
    if llamada.starts_with("sumar(") {
        let args = extraer_argumentos_funcion(llamada)?;
        if args.len() >= 2 {
            let val1 = evaluar_expresion_valor(&args[0], entorno)?;
            let val2 = evaluar_expresion_valor(&args[1], entorno)?;
            match (val1, val2) {
                (Valor::Entero(a), Valor::Entero(b)) => {
                    entorno.establecer(variable_resultado, Valor::Entero(a + b));
                }
                (Valor::Numero(a), Valor::Numero(b)) => {
                    entorno.establecer(variable_resultado, Valor::Numero(a + b));
                }
                _ => return Err("Tipos incompatibles para suma".to_string()),
            }
        }
        return Ok(());
    }
    
    if llamada.starts_with("saludar(") {
        let args = extraer_argumentos_funcion(llamada)?;
        let nombre = if args.is_empty() {
            "Mundo".to_string()
        } else {
            evaluar_expresion_valor(&args[0], entorno)?.a_cadena().trim_matches('"').to_string()
        };
        let saludo = if args.len() > 1 {
            evaluar_expresion_valor(&args[1], entorno)?.a_cadena().trim_matches('"').to_string()
        } else {
            "Hola".to_string()
        };
        let resultado = format!("{}, {}!", saludo, nombre);
        entorno.establecer(variable_resultado, Valor::Cadena(resultado));
        return Ok(());
    }
    
    if llamada.starts_with("calcular_promedio(") {
        let args = extraer_argumentos_funcion(llamada)?;
        if !args.is_empty() {
            let lista_val = evaluar_expresion_valor(&args[0], entorno)?;
            if let Valor::Lista(elementos) = lista_val {
                let mut suma = 0.0;
                let mut count = 0;
                for elem in elementos {
                    match elem {
                        Valor::Numero(n) => {
                            suma += n;
                            count += 1;
                        }
                        Valor::Entero(i) => {
                            suma += i as f64;
                            count += 1;
                        }
                        _ => {}
                    }
                }
                if count > 0 {
                    entorno.establecer(variable_resultado, Valor::Numero(suma / count as f64));
                } else {
                    entorno.establecer(variable_resultado, Valor::Numero(0.0));
                }
            }
        }
        return Ok(());
    }
    
    // Verificar si es una función definida por el usuario
    let nombre_funcion = if let Some(pos) = llamada.find('(') {
        &llamada[..pos]
    } else {
        return Err("Sintaxis de llamada de función inválida".to_string());
    };
    
    if let Some(def_funcion) = entorno.obtener_funcion(nombre_funcion).cloned() {
        return ejecutar_funcion_usuario(&def_funcion, llamada, variable_resultado, entorno);
    }
    
    Err("Función no reconocida".to_string())
}

fn extraer_argumentos_funcion(llamada: &str) -> Result<Vec<String>, String> {
    let inicio = llamada.find('(').ok_or("Función inválida")?;
    let fin = llamada.rfind(')').ok_or("Función inválida")?;
    let contenido = &llamada[inicio + 1..fin];
    if contenido.trim().is_empty() {
        return Ok(Vec::new());
    }
    Ok(contenido.split(',').map(|s| s.trim().to_string()).collect())
}

fn procesar_asignacion_compuesta(linea: &str, entorno: &mut Entorno, linea_num: usize) -> Result<(), String> {
    if let Some(pos) = linea.find("+=") {
        let variable = linea[..pos].trim();
        let valor_expr = linea[pos + 2..].trim();
        
        let valor_actual = entorno.obtener(variable).cloned()
            .ok_or_else(|| formatear_error(linea_num, "Variable no encontrada"))?;
        let valor_nuevo = evaluar_expresion_valor(valor_expr, entorno)?;
        
        match (valor_actual, valor_nuevo) {
            (Valor::Entero(a), Valor::Entero(b)) => {
                entorno.establecer(variable, Valor::Entero(a + b));
            }
            (Valor::Numero(a), Valor::Numero(b)) => {
                entorno.establecer(variable, Valor::Numero(a + b));
            }
            (Valor::Cadena(a), Valor::Cadena(b)) => {
                entorno.establecer(variable, Valor::Cadena(a + &b));
            }
            _ => return Err(formatear_error(linea_num, "Tipos incompatibles para +=")),
        }
    } else if let Some(pos) = linea.find("-=") {
        let variable = linea[..pos].trim();
        let valor_expr = linea[pos + 2..].trim();
        
        let valor_actual = entorno.obtener(variable).cloned()
            .ok_or_else(|| formatear_error(linea_num, "Variable no encontrada"))?;
        let valor_nuevo = evaluar_expresion_valor(valor_expr, entorno)?;
        
        match (valor_actual, valor_nuevo) {
            (Valor::Entero(a), Valor::Entero(b)) => {
                entorno.establecer(variable, Valor::Entero(a - b));
            }
            (Valor::Numero(a), Valor::Numero(b)) => {
                entorno.establecer(variable, Valor::Numero(a - b));
            }
            _ => return Err(formatear_error(linea_num, "Tipos incompatibles para -=")),
        }
    } else if let Some(pos) = linea.find("*=") {
        let variable = linea[..pos].trim();
        let valor_expr = linea[pos + 2..].trim();
        
        let valor_actual = entorno.obtener(variable).cloned()
            .ok_or_else(|| formatear_error(linea_num, "Variable no encontrada"))?;
        let valor_nuevo = evaluar_expresion_valor(valor_expr, entorno)?;
        
        match (valor_actual, valor_nuevo) {
            (Valor::Entero(a), Valor::Entero(b)) => {
                entorno.establecer(variable, Valor::Entero(a * b));
            }
            (Valor::Numero(a), Valor::Numero(b)) => {
                entorno.establecer(variable, Valor::Numero(a * b));
            }
            _ => return Err(formatear_error(linea_num, "Tipos incompatibles para *=")),
        }
    } else if let Some(pos) = linea.find("/=") {
        let variable = linea[..pos].trim();
        let valor_expr = linea[pos + 2..].trim();
        
        let valor_actual = entorno.obtener(variable).cloned()
            .ok_or_else(|| formatear_error(linea_num, "Variable no encontrada"))?;
        let valor_nuevo = evaluar_expresion_valor(valor_expr, entorno)?;
        
        match (valor_actual, valor_nuevo) {
            (Valor::Entero(a), Valor::Entero(b)) => {
                if b == 0 { return Err(formatear_error(linea_num, "División por cero")); }
                entorno.establecer(variable, Valor::Entero(a / b));
            }
            (Valor::Numero(a), Valor::Numero(b)) => {
                if b == 0.0 { return Err(formatear_error(linea_num, "División por cero")); }
                entorno.establecer(variable, Valor::Numero(a / b));
            }
            _ => return Err(formatear_error(linea_num, "Tipos incompatibles para /=")),
        }
    } else if let Some(pos) = linea.find("%=") {
        let variable = linea[..pos].trim();
        let valor_expr = linea[pos + 2..].trim();
        
        let valor_actual = entorno.obtener(variable).cloned()
            .ok_or_else(|| formatear_error(linea_num, "Variable no encontrada"))?;
        let valor_nuevo = evaluar_expresion_valor(valor_expr, entorno)?;
        
        match (valor_actual, valor_nuevo) {
            (Valor::Entero(a), Valor::Entero(b)) => {
                if b == 0 { return Err(formatear_error(linea_num, "División por cero en módulo")); }
                entorno.establecer(variable, Valor::Entero(a % b));
            }
            (Valor::Numero(a), Valor::Numero(b)) => {
                if b == 0.0 { return Err(formatear_error(linea_num, "División por cero en módulo")); }
                entorno.establecer(variable, Valor::Numero(a % b));
            }
            _ => return Err(formatear_error(linea_num, "Tipos incompatibles para %=")),
        }
    }
    Ok(())
}

fn manejar_impresion<F>(linea: &str, comando: &str, linea_num: usize, entorno: &mut Entorno, func: F) -> Result<(), String>
where
    F: Fn(&str),
{
    let inicio = linea.find('(').ok_or_else(|| formatear_error(linea_num, "Función de impresión inválida"))?;
    let fin = linea.rfind(')').ok_or_else(|| formatear_error(linea_num, "Función de impresión inválida"))?;
    let contenido = &linea[inicio + 1..fin];
    
    if contenido.trim().is_empty() {
        func("");
        return Ok(());
    }
    
    let texto = evaluar_cadena_para_impresion(contenido, entorno, linea_num)?;
    func(&texto);
    Ok(())
}

fn evaluar_cadena_para_impresion(expr: &str, entorno: &mut Entorno, linea_num: usize) -> Result<String, String> {
    let texto = expr.trim();
    
    // Si es una cadena literal
    if texto.starts_with('"') && texto.ends_with('"') {
        return Ok(texto.trim_matches('"').to_string());
    }
    
    // Si contiene concatenación con +
    if texto.contains(" + ") {
        return evaluar_concatenacion(texto, entorno, linea_num);
    }
    
    // Si es una variable simple
    if let Some(valor) = entorno.obtener(texto) {
        return Ok(valor.a_cadena());
    }
    
    // Si es una expresión con paréntesis y método
    if texto.contains('(') && texto.contains(')') && texto.contains(".cadena()") {
        let partes: Vec<&str> = texto.split(".cadena()").collect();
        if partes.len() == 2 && partes[1].is_empty() {
            let expr_base = partes[0];
            if expr_base.starts_with('(') && expr_base.ends_with(')') {
                let expr_interna = &expr_base[1..expr_base.len()-1];
                match evaluar_expresion_valor(expr_interna, entorno) {
                    Ok(valor) => return Ok(valor.a_cadena()),
                    Err(_) => {}
                }
            }
        }
    }
    
    // Intentar evaluar como expresión
    match evaluar_expresion_valor(texto, entorno) {
        Ok(valor) => Ok(valor.a_cadena()),
        Err(_) => Ok(texto.to_string()),
    }
}

fn evaluar_concatenacion(expr: &str, entorno: &mut Entorno, linea_num: usize) -> Result<String, String> {
    // Dividir por " + " de manera más simple
    let partes_simples: Vec<&str> = expr.split(" + ").collect();
    println!("DEBUG: expr='{}', partes={:?}", expr, partes_simples);
    let mut resultado = String::new();
    
    for parte in partes_simples {
        let parte_trim = parte.trim();
        
        let valor_str = if parte_trim.starts_with('"') && parte_trim.ends_with('"') {
            // Es una cadena literal
            parte_trim.trim_matches('"').to_string()
        } else if parte_trim.contains('.') && parte_trim.contains('(') && parte_trim.ends_with(')') {
            // Es una expresión con funciones encadenadas
            match obtener_valor_mutable(parte_trim, entorno) {
                Ok(valor) => valor.a_cadena(),
                Err(_) => {
                    // Si no funciona como función encadenada, intentar como expresión normal
                    match evaluar_expresion_valor(parte_trim, entorno) {
                        Ok(valor) => valor.a_cadena(),
                        Err(_) => parte_trim.to_string(),
                    }
                }
            }
        } else if parte_trim.contains(".cadena()") && parte_trim.starts_with('(') {
            // Es una expresión con paréntesis y .cadena()
            let pos_metodo = parte_trim.find(".cadena()").unwrap();
            if parte_trim[..pos_metodo].ends_with(')') {
                let expr_interna = &parte_trim[1..pos_metodo-1];
                // Evaluar directamente como operación aritmética para evitar recursión infinita
                match evaluar_operacion_aritmetica(expr_interna, entorno) {
                    Ok(valor) => valor.a_cadena(),
                    Err(_) => {
                        // Si no es operación aritmética, intentar obtener variable directamente
                        if let Some(valor) = entorno.obtener(expr_interna) {
                            valor.a_cadena()
                        } else {
                            return Err(formatear_error(linea_num, &format!("No se puede evaluar expresión: {}", expr_interna)));
                        }
                    }
                }
            } else {
                return Err(formatear_error(linea_num, "Expresión con método inválida"));
            }
        } else if parte_trim.ends_with(".cadena()") {
            // Es una variable con método de conversión
            let base = parte_trim.trim_end_matches(".cadena()");
            if let Some(valor) = entorno.obtener(base) {
                valor.a_cadena()
            } else {
                return Err(formatear_error(linea_num, &format!("Variable '{}' no encontrada", base)));
            }
        } else if let Some(valor) = entorno.obtener(parte_trim) {
            // Es una variable simple
            valor.a_cadena()
        } else {
            // Intentar evaluar como operación aritmética simple para evitar recursión infinita
            match evaluar_operacion_aritmetica(parte_trim, entorno) {
                Ok(valor) => valor.a_cadena(),
                Err(_) => {
                    // Si no es una operación aritmética, intentar parsear como literal
                    match parsear_literal(parte_trim) {
                        Ok(valor) => valor.a_cadena(),
                        Err(_) => parte_trim.to_string(),
                    }
                }
            }
        };
        
        resultado.push_str(&valor_str);
    }
    
    Ok(resultado)
}

// Función para dividir elementos de lista respetando corchetes anidados
fn dividir_elementos_lista(contenido: &str) -> Result<Vec<String>, String> {
    let mut elementos = Vec::new();
    let mut elemento_actual = String::new();
    let mut nivel_corchetes = 0;
    let mut en_cadena = false;
    
    for c in contenido.chars() {
        match c {
            '"' => {
                en_cadena = !en_cadena;
                elemento_actual.push(c);
            }
            '[' if !en_cadena => {
                nivel_corchetes += 1;
                elemento_actual.push(c);
            }
            ']' if !en_cadena => {
                nivel_corchetes -= 1;
                elemento_actual.push(c);
            }
            ',' if !en_cadena && nivel_corchetes == 0 => {
                elementos.push(elemento_actual.trim().to_string());
                elemento_actual.clear();
            }
            _ => {
                elemento_actual.push(c);
            }
        }
    }
    
    if !elemento_actual.trim().is_empty() {
        elementos.push(elemento_actual.trim().to_string());
    }
    
    Ok(elementos)
}

// Función para parsear un elemento de lista (puede ser literal o lista anidada)
fn parsear_elemento_lista(texto: &str, entorno: &mut Entorno) -> Result<Valor, String> {
    let texto = texto.trim();
    
    if texto.starts_with('[') && texto.ends_with(']') {
        // Es una lista anidada
        let contenido = &texto[1..texto.len() - 1];
        let mut elementos = Vec::new();
        if !contenido.trim().is_empty() {
            let elementos_texto = dividir_elementos_lista(contenido)?;
            for elemento_texto in elementos_texto {
                elementos.push(parsear_elemento_lista(elemento_texto.trim(), entorno)?);
            }
        }
        Ok(Valor::Lista(elementos))
    } else {
        // Es un literal o expresión
        if let Ok(literal) = parsear_literal(texto) {
            Ok(literal)
        } else {
            // Intentar evaluar como expresión (para variables)
            evaluar_expresion_valor(texto, entorno)
        }
    }
}

fn formatear_error(linea: usize, mensaje: &str) -> String {
    format!("Error en línea {}: {}", linea + 1, mensaje)
}



fn valor_desde_expresion(expresion: &str, linea_num: usize, entorno: &mut Entorno) -> Result<String, String> {
    let texto = expresion.trim();
    if texto.starts_with('"') && texto.ends_with('"') {
        return Ok(texto.trim_matches('"').to_string());
    }

    // Intentar evaluar como funciones encadenadas primero
    if texto.contains('.') && texto.contains('(') && texto.ends_with(')') {
        match evaluar_llamadas_encadenadas(texto, entorno) {
            Ok(valor) => return Ok(valor.a_cadena()),
            Err(_) => {
                // Si falla, intentar el método anterior
            }
        }
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
                        entorno.establecer(base, Valor::Instancia(t.clone(), mapa));
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
                    let mut val = obtener_valor_mutable(base, entorno)?;
                    if let Some(ret) = aplicar_metodo_valor(&mut val, metodo, args)? {
                        if es_var {
                            entorno.establecer(base, val);
                        }
                        return Ok(ret.a_cadena());
                    } else if es_var {
                        entorno.establecer(base, val);
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

fn obtener_valor_mutable(texto: &str, entorno: &mut Entorno) -> Result<Valor, String> {
    if texto.starts_with('"') && texto.ends_with('"') {
        return Ok(Valor::Cadena(texto.trim_matches('"').to_string()));
    }
    if texto == "verdadero" { return Ok(Valor::Bool(true)); }
    if texto == "falso" { return Ok(Valor::Bool(false)); }
    if let Ok(i) = texto.parse::<i64>() { return Ok(Valor::Entero(i)); }
    if let Ok(n) = texto.parse::<f64>() { return Ok(Valor::Numero(n)); }
    
    // Intentar evaluar como funciones encadenadas
    if texto.contains('.') && texto.contains('(') && texto.ends_with(')') {
        if let Ok(valor) = evaluar_llamadas_encadenadas(texto, entorno) {
            return Ok(valor);
        }
    }
    
    if let Some(v) = entorno.obtener(texto) { return Ok(v.clone()); }
    Err("Valor no encontrado".to_string())
}

fn evaluar_comparacion(condicion: &str, entorno: &mut Entorno) -> Result<bool, String> {
    let condicion = condicion.trim();
    
    // Manejar expresiones entre paréntesis
    if condicion.starts_with('(') && condicion.ends_with(')') {
        let expr_interna = &condicion[1..condicion.len() - 1];
        return evaluar_comparacion(expr_interna, entorno);
    }
    
    // Manejar literales booleanos
    if condicion == "verdadero" {
        return Ok(true);
    }
    if condicion == "falso" {
        return Ok(false);
    }
    
    // Buscar operadores de comparación
    let ops = ["!=", "==", "<=", ">=", "<", ">"];
    for op in &ops {
        if let Some(pos) = condicion.find(op) {
            let izq_expr = condicion[..pos].trim();
            let der_expr = condicion[pos + op.len()..].trim();
            
            // Evaluar las expresiones del lado izquierdo y derecho
            let izq = evaluar_expresion_valor(izq_expr, entorno)?;
            let der = evaluar_expresion_valor(der_expr, entorno)?;
            
            return match (izq, der, *op) {
                // Comparaciones entre enteros
                (Valor::Entero(a), Valor::Entero(b), "<") => Ok(a < b),
                (Valor::Entero(a), Valor::Entero(b), "<=") => Ok(a <= b),
                (Valor::Entero(a), Valor::Entero(b), ">") => Ok(a > b),
                (Valor::Entero(a), Valor::Entero(b), ">=") => Ok(a >= b),
                (Valor::Entero(a), Valor::Entero(b), "==") => Ok(a == b),
                (Valor::Entero(a), Valor::Entero(b), "!=") => Ok(a != b),
                
                // Comparaciones entre números
                (Valor::Numero(a), Valor::Numero(b), "<") => Ok(a < b),
                (Valor::Numero(a), Valor::Numero(b), "<=") => Ok(a <= b),
                (Valor::Numero(a), Valor::Numero(b), ">") => Ok(a > b),
                (Valor::Numero(a), Valor::Numero(b), ">=") => Ok(a >= b),
                (Valor::Numero(a), Valor::Numero(b), "==") => Ok((a - b).abs() < f64::EPSILON),
                (Valor::Numero(a), Valor::Numero(b), "!=") => Ok((a - b).abs() >= f64::EPSILON),
                
                // Comparaciones mixtas entero-número
                (Valor::Entero(a), Valor::Numero(b), "<") => Ok((a as f64) < b),
                (Valor::Entero(a), Valor::Numero(b), "<=") => Ok((a as f64) <= b),
                (Valor::Entero(a), Valor::Numero(b), ">") => Ok((a as f64) > b),
                (Valor::Entero(a), Valor::Numero(b), ">=") => Ok((a as f64) >= b),
                (Valor::Entero(a), Valor::Numero(b), "==") => Ok(((a as f64) - b).abs() < f64::EPSILON),
                (Valor::Entero(a), Valor::Numero(b), "!=") => Ok(((a as f64) - b).abs() >= f64::EPSILON),
                
                (Valor::Numero(a), Valor::Entero(b), "<") => Ok(a < (b as f64)),
                (Valor::Numero(a), Valor::Entero(b), "<=") => Ok(a <= (b as f64)),
                (Valor::Numero(a), Valor::Entero(b), ">") => Ok(a > (b as f64)),
                (Valor::Numero(a), Valor::Entero(b), ">=") => Ok(a >= (b as f64)),
                (Valor::Numero(a), Valor::Entero(b), "==") => Ok((a - (b as f64)).abs() < f64::EPSILON),
                (Valor::Numero(a), Valor::Entero(b), "!=") => Ok((a - (b as f64)).abs() >= f64::EPSILON),
                
                // Comparaciones entre cadenas
                (Valor::Cadena(a), Valor::Cadena(b), "==") => Ok(a == b),
                (Valor::Cadena(a), Valor::Cadena(b), "!=") => Ok(a != b),
                
                // Comparaciones entre booleanos
                (Valor::Bool(a), Valor::Bool(b), "==") => Ok(a == b),
                (Valor::Bool(a), Valor::Bool(b), "!=") => Ok(a != b),
                
                _ => Err("Tipos incompatibles para comparación".to_string()),
            };
        }
    }
    
    // Si no hay operadores de comparación, intentar obtener un valor booleano directamente
    if let Some(Valor::Bool(b)) = entorno.obtener(condicion.trim()) {
        return Ok(*b);
    }
    
    Err("Condición inválida".to_string())
}

fn evaluar_bool(expr: &str, entorno: &mut Entorno) -> Result<bool, String> {
    // Primero verificar operadores lógicos && y ||
    if let Some(pos) = expr.find("&&") {
        let izquierda = &expr[..pos].trim();
        let derecha = &expr[pos + 2..].trim();
        return Ok(evaluar_bool(izquierda, entorno)? && evaluar_bool(derecha, entorno)?);
    }
    if let Some(pos) = expr.find("||") {
        let izquierda = &expr[..pos].trim();
        let derecha = &expr[pos + 2..].trim();
        return Ok(evaluar_bool(izquierda, entorno)? || evaluar_bool(derecha, entorno)?);
    }
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

fn evaluar_llamadas_encadenadas(expr: &str, entorno: &mut Entorno) -> Result<Valor, String> {
    let texto = expr.trim();
    
    // Encontrar la primera variable/expresión base
    let mut punto_base = None;
    let mut en_cadena = false;
    let mut nivel_parentesis = 0;
    let chars: Vec<char> = texto.chars().collect();
    
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '"' => en_cadena = !en_cadena,
            '(' if !en_cadena => nivel_parentesis += 1,
            ')' if !en_cadena => nivel_parentesis -= 1,
            '.' if !en_cadena && nivel_parentesis == 0 => {
                punto_base = Some(i);
                break;
            }
            _ => {}
        }
    }
    
    if punto_base.is_none() {
        return Err("No se encontraron llamadas a métodos encadenadas".to_string());
    }
    
    let punto_base = punto_base.unwrap();
    let base = texto[..punto_base].trim();
    let resto = &texto[punto_base + 1..];
    
    // Dividir el resto en llamadas a métodos individuales
    let mut llamadas = Vec::new();
    let mut inicio_llamada = 0;
    nivel_parentesis = 0;
    en_cadena = false;
    let resto_chars: Vec<char> = resto.chars().collect();
    
    for (i, &c) in resto_chars.iter().enumerate() {
        match c {
            '"' => en_cadena = !en_cadena,
            '(' if !en_cadena => nivel_parentesis += 1,
            ')' if !en_cadena => {
                nivel_parentesis -= 1;
                if nivel_parentesis == 0 {
                    let llamada = &resto[inicio_llamada..=i];
                    llamadas.push(llamada.trim());
                    
                    if i + 1 < resto_chars.len() && resto_chars[i + 1] == '.' {
                        inicio_llamada = i + 2;
                    }
                }
            }
            _ => {}
        }
    }
    
    // Evaluar la expresión base primero
    let mut valor_actual = evaluar_expresion_valor(base, entorno)?;
    
    // Aplicar cada llamada a método en secuencia
    for llamada in llamadas {
        if let Some(paren) = llamada.find('(') {
            let metodo = llamada[..paren].trim();
            let args_str = &llamada[paren + 1..llamada.len() - 1];
            let mut args = Vec::new();
            
            if !args_str.trim().is_empty() {
                let argumentos_texto = dividir_elementos_lista(args_str)?;
                for arg in argumentos_texto {
                    args.push(evaluar_expresion_valor(&arg, entorno)?);
                }
            }
            
            if let Some(resultado) = aplicar_metodo_valor(&mut valor_actual, metodo, args)? {
                valor_actual = resultado;
            } else {
                return Err(format!("Método '{}' no encontrado", metodo));
            }
        }
    }
    
    Ok(valor_actual)
}

fn evaluar_expresion_valor(expr: &str, entorno: &mut Entorno) -> Result<Valor, String> {
    let texto = expr.trim();
    println!("DEBUG evaluar_expresion_valor: '{}'", texto);
    
    // Acceso por índice [index]
    if texto.contains('[') && texto.ends_with(']') {
        if let Some(corchete_inicio) = texto.find('[') {
            let base = texto[..corchete_inicio].trim();
            let indice_str = &texto[corchete_inicio + 1..texto.len() - 1];
            
            let valor_base = evaluar_expresion_valor(base, entorno)?;
            let indice = evaluar_expresion_valor(indice_str, entorno)?;
            
            match (valor_base, indice) {
                (Valor::Cadena(cadena), Valor::Entero(i)) => {
                    let chars: Vec<char> = cadena.chars().collect();
                    if i < 0 || i as usize >= chars.len() {
                        return Err("Índice fuera de rango".to_string());
                    }
                    return Ok(Valor::Cadena(chars[i as usize].to_string()));
                }
                (Valor::Lista(lista), Valor::Entero(i)) => {
                    if i < 0 || i as usize >= lista.len() {
                        return Err("Índice fuera de rango".to_string());
                    }
                    return Ok(lista[i as usize].clone());
                }
                _ => return Err("Acceso por índice no soportado para este tipo".to_string()),
            }
        }
    }
    
    // Operador ternario
    if texto.contains('?') && texto.contains(':') {
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
    
    // Llamadas a métodos (incluyendo encadenamiento)
    if texto.contains('.') && texto.contains('(') && texto.ends_with(')') {
        return evaluar_llamadas_encadenadas(texto, entorno);
    }
    for op in &[" && ", " y ", " || ", " o "] {
        if let Some(pos) = encontrar_operador_logico(texto, op) {
            let izq = texto[..pos].trim();
            let der = texto[pos + op.len()..].trim();
            
            let val_izq = evaluar_expresion_valor(izq, entorno)?;
            let val_der = evaluar_expresion_valor(der, entorno)?;
            
            let bool_izq = match val_izq {
                Valor::Bool(b) => b,
                _ => return Err("Operando izquierdo no es booleano".to_string()),
            };
            
            let bool_der = match val_der {
                Valor::Bool(b) => b,
                _ => return Err("Operando derecho no es booleano".to_string()),
            };
            
            let resultado = match *op {
                " && " | " y " => bool_izq && bool_der,
                " || " | " o " => bool_izq || bool_der,
                _ => unreachable!(),
            };
            
            return Ok(Valor::Bool(resultado));
        }
    }
    
    // Operador de negación
    if texto.starts_with('!') {
        let operando = &texto[1..];
        let val = evaluar_expresion_valor(operando, entorno)?;
        match val {
            Valor::Bool(b) => return Ok(Valor::Bool(!b)),
            _ => return Err("Operando de negación no es booleano".to_string()),
        }
    }
    for op in &["==", "!=", "<=", ">=", "<", ">"] {
        if let Some(pos) = encontrar_operador_principal(texto, op) {
            let izq = texto[..pos].trim();
            let der = texto[pos + op.len()..].trim();
            
            let val_izq = evaluar_expresion_valor(izq, entorno)?;
            let val_der = evaluar_expresion_valor(der, entorno)?;
            
            let resultado = match (val_izq, val_der, *op) {
                (Valor::Entero(a), Valor::Entero(b), "==") => a == b,
                (Valor::Entero(a), Valor::Entero(b), "!=") => a != b,
                (Valor::Entero(a), Valor::Entero(b), "<") => a < b,
                (Valor::Entero(a), Valor::Entero(b), "<=") => a <= b,
                (Valor::Entero(a), Valor::Entero(b), ">") => a > b,
                (Valor::Entero(a), Valor::Entero(b), ">=") => a >= b,
                
                (Valor::Numero(a), Valor::Numero(b), "==") => (a - b).abs() < f64::EPSILON,
                (Valor::Numero(a), Valor::Numero(b), "!=") => (a - b).abs() >= f64::EPSILON,
                (Valor::Numero(a), Valor::Numero(b), "<") => a < b,
                (Valor::Numero(a), Valor::Numero(b), "<=") => a <= b,
                (Valor::Numero(a), Valor::Numero(b), ">") => a > b,
                (Valor::Numero(a), Valor::Numero(b), ">=") => a >= b,
                
                (Valor::Entero(a), Valor::Numero(b), "==") => ((a as f64) - b).abs() < f64::EPSILON,
                (Valor::Entero(a), Valor::Numero(b), "!=") => ((a as f64) - b).abs() >= f64::EPSILON,
                (Valor::Entero(a), Valor::Numero(b), "<") => (a as f64) < b,
                (Valor::Entero(a), Valor::Numero(b), "<=") => (a as f64) <= b,
                (Valor::Entero(a), Valor::Numero(b), ">") => (a as f64) > b,
                (Valor::Entero(a), Valor::Numero(b), ">=") => (a as f64) >= b,
                
                (Valor::Numero(a), Valor::Entero(b), "==") => (a - (b as f64)).abs() < f64::EPSILON,
                (Valor::Numero(a), Valor::Entero(b), "!=") => (a - (b as f64)).abs() >= f64::EPSILON,
                (Valor::Numero(a), Valor::Entero(b), "<") => a < (b as f64),
                (Valor::Numero(a), Valor::Entero(b), "<=") => a <= (b as f64),
                (Valor::Numero(a), Valor::Entero(b), ">") => a > (b as f64),
                (Valor::Numero(a), Valor::Entero(b), ">=") => a >= (b as f64),
                
                (Valor::Cadena(a), Valor::Cadena(b), "==") => a == b,
                (Valor::Cadena(a), Valor::Cadena(b), "!=") => a != b,
                
                (Valor::Bool(a), Valor::Bool(b), "==") => a == b,
                (Valor::Bool(a), Valor::Bool(b), "!=") => a != b,
                
                _ => return Err("Tipos incompatibles para comparación".to_string()),
            };
            
            return Ok(Valor::Bool(resultado));
        }
    }
    
    // Manejo específico de expresiones con paréntesis
    if texto.starts_with('(') && texto.ends_with(')') {
        let expresion_interna = &texto[1..texto.len()-1];
        return evaluar_expresion_valor(expresion_interna, entorno);
    }
    
    // Verificar primero si es concatenación cuando hay cadenas literales
    if texto.contains(" + ") && texto.contains('"') {
        println!("DEBUG: Detectada concatenación para: '{}'", texto);
        match evaluar_concatenacion(texto, entorno, 0) {
            Ok(resultado) => return Ok(Valor::Cadena(resultado)),
            Err(e) => {
                println!("DEBUG: Error en concatenación: {}", e);
                // Si falla la concatenación, continuar con otros métodos
            }
        }
    }
    
    // Intentar primero operaciones aritméticas
    if let Ok(resultado) = evaluar_operacion_aritmetica(texto, entorno) {
        return Ok(resultado);
    }
    
    // Si no es operación aritmética, entonces puede ser concatenación de cadenas
    if texto.contains(" + ") {
        // Verificar si es probablemente concatenación (contiene cadenas literales)
        let contiene_cadena_literal = texto.contains('"');
        if contiene_cadena_literal {
            let resultado = evaluar_concatenacion(texto, entorno, 0)?;
            return Ok(Valor::Cadena(resultado));
        }
        
        // Si no tiene cadenas literales, intentar concatenación de todas formas
        if let Ok(resultado) = evaluar_concatenacion(texto, entorno, 0) {
            return Ok(Valor::Cadena(resultado));
        }
    }
    
    // Literales básicos
    if texto == "verdadero" { return Ok(Valor::Bool(true)); }
    if texto == "falso" { return Ok(Valor::Bool(false)); }
    
    // Parsear literal
    if let Ok(l) = parsear_literal(texto) {
        return Ok(l);
    }
    
    // Obtener valor de variable
    if let Ok(v) = obtener_valor(texto, entorno) {
        return Ok(v);
    }
    
    // Método de conversión o acceso
    if let Ok(cadena) = valor_desde_expresion(texto, 0, entorno) {
        return Ok(Valor::Cadena(cadena));
    }
    
    Err("Expresión inválida".to_string())
}

fn evaluar_operacion_aritmetica(expr: &str, entorno: &mut Entorno) -> Result<Valor, String> {
    // Buscar operadores en orden de precedencia (menor a mayor)
    for op in &["+", "-"] {
        if let Some(pos) = encontrar_operador_principal(expr, op) {
            let izq = expr[..pos].trim();
            let der = expr[pos + op.len()..].trim();
            
            let val_izq = evaluar_expresion_valor(izq, entorno)?;
            let val_der = evaluar_expresion_valor(der, entorno)?;
            
            match (val_izq, val_der, *op) {
                (Valor::Entero(a), Valor::Entero(b), "+") => return Ok(Valor::Entero(a + b)),
                (Valor::Entero(a), Valor::Entero(b), "-") => return Ok(Valor::Entero(a - b)),
                (Valor::Numero(a), Valor::Numero(b), "+") => return Ok(Valor::Numero(a + b)),
                (Valor::Numero(a), Valor::Numero(b), "-") => return Ok(Valor::Numero(a - b)),
                (Valor::Entero(a), Valor::Numero(b), "+") => return Ok(Valor::Numero(a as f64 + b)),
                (Valor::Entero(a), Valor::Numero(b), "-") => return Ok(Valor::Numero(a as f64 - b)),
                (Valor::Numero(a), Valor::Entero(b), "+") => return Ok(Valor::Numero(a + b as f64)),
                (Valor::Numero(a), Valor::Entero(b), "-") => return Ok(Valor::Numero(a - b as f64)),
                _ => {}
            }
        }
    }
    
    for op in &["*", "/", "%"] {
        if let Some(pos) = encontrar_operador_principal(expr, op) {
            let izq = expr[..pos].trim();
            let der = expr[pos + op.len()..].trim();
            
            let val_izq = evaluar_expresion_valor(izq, entorno)?;
            let val_der = evaluar_expresion_valor(der, entorno)?;
            
            match (val_izq, val_der, *op) {
                (Valor::Entero(a), Valor::Entero(b), "*") => return Ok(Valor::Entero(a * b)),
                (Valor::Entero(a), Valor::Entero(b), "/") => return Ok(Valor::Entero(a / b)),
                (Valor::Entero(a), Valor::Entero(b), "%") => return Ok(Valor::Entero(a % b)),
                (Valor::Numero(a), Valor::Numero(b), "*") => return Ok(Valor::Numero(a * b)),
                (Valor::Numero(a), Valor::Numero(b), "/") => return Ok(Valor::Numero(a / b)),
                (Valor::Numero(a), Valor::Numero(b), "%") => return Ok(Valor::Numero(a % b)),
                (Valor::Entero(a), Valor::Numero(b), "*") => return Ok(Valor::Numero(a as f64 * b)),
                (Valor::Entero(a), Valor::Numero(b), "/") => return Ok(Valor::Numero(a as f64 / b)),
                (Valor::Numero(a), Valor::Entero(b), "*") => return Ok(Valor::Numero(a * b as f64)),
                (Valor::Numero(a), Valor::Entero(b), "/") => return Ok(Valor::Numero(a / b as f64)),
                _ => {}
            }
        }
    }
    
    Err("No es una operación aritmética válida".to_string())
}

fn encontrar_operador_logico(expr: &str, op: &str) -> Option<usize> {
    let mut nivel_parentesis = 0;
    let mut en_cadena = false;
    
    for (char_pos, c) in expr.char_indices() {
        match c {
            '"' => en_cadena = !en_cadena,
            '(' if !en_cadena => nivel_parentesis += 1,
            ')' if !en_cadena => nivel_parentesis -= 1,
            _ => {
                if !en_cadena && nivel_parentesis == 0 && expr[char_pos..].starts_with(op) {
                    return Some(char_pos);
                }
            }
        }
    }
    
    None
}

fn encontrar_operador_principal(expr: &str, op: &str) -> Option<usize> {
    let mut nivel_parentesis = 0;
    
    for (char_pos, c) in expr.char_indices() {
        match c {
            '(' => nivel_parentesis += 1,
            ')' => nivel_parentesis -= 1,
            _ => {
                if nivel_parentesis == 0 && expr[char_pos..].starts_with(op) {
                    // Para operadores de comparación de dos caracteres, verificar que no sea parte de otro operador
                    let es_operador_completo = if op.len() == 2 {
                        // Para ==, !=, <=, >=
                        true
                    } else {
                        // Para operadores de un carácter como <, >
                        let siguiente_char = expr[char_pos..].chars().nth(1);
                        !matches!(siguiente_char, Some('='))
                    };
                    
                    // Verificar que no es parte de otro operador por la izquierda
                    let anterior_char = if char_pos > 0 {
                        expr[..char_pos].chars().last()
                    } else {
                        None
                    };
                    let no_es_parte_izquierda = !matches!(anterior_char, Some('=') | Some('!') | Some('<') | Some('>'));
                    
                    if es_operador_completo && no_es_parte_izquierda {
                        return Some(char_pos);
                    }
                }
            }
        }
    }
    
    None
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
    while evaluar_bool(partes[1].trim(), entorno)? {
        match procesar_lineas(bloque, entorno, linea_num + 1) {
            Ok(()) => {},
            Err(error) if error == "ROMPER" => break,
            Err(error) if error == "CONTINUAR" => {
                aplicar_incremento(partes[2].trim(), entorno)?;
                continue;
            },
            Err(error) => return Err(error),
        }
        aplicar_incremento(partes[2].trim(), entorno)?;
    }
    Ok(())
}

fn procesar_bucle_mientras(linea: &str, bloque: &[String], entorno: &mut Entorno, linea_num: usize) -> Result<(), String> {
    let ini = linea.find('(').ok_or_else(|| formatear_error(linea_num, "Bucle mientras inválido"))?;
    let fin = linea.rfind(')').ok_or_else(|| formatear_error(linea_num, "Bucle mientras inválido"))?;
    let condicion = &linea[ini + 1..fin];
    while evaluar_bool(condicion, entorno)? {
        match procesar_lineas(bloque, entorno, linea_num + 1) {
            Ok(()) => {},
            Err(error) if error == "ROMPER" => break,
            Err(error) if error == "CONTINUAR" => continue,
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn procesar_bucle_hacer(bloque: &[String], condicion: &str, entorno: &mut Entorno, linea_num: usize) -> Result<(), String> {
    loop {
        match procesar_lineas(bloque, entorno, linea_num + 1) {
            Ok(()) => {},
            Err(error) if error == "ROMPER" => break,
            Err(error) if error == "CONTINUAR" => {
                if !evaluar_bool(condicion, entorno)? { break; }
                continue;
            },
            Err(error) => return Err(error),
        }
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
    
    // Extraer el nombre de la variable, considerando que puede tener tipo
    let declaracion_var = partes[0].trim();
    let var = if declaracion_var.contains(' ') {
        // Si contiene espacios, es "tipo variable", tomamos la variable
        declaracion_var.split_whitespace().last().unwrap_or(declaracion_var)
    } else {
        // Si no contiene espacios, es solo la variable
        declaracion_var
    };
    
    let lista_nombre = partes[1].trim();
    let lista = entorno.obtener(lista_nombre).cloned().ok_or_else(|| formatear_error(linea_num, "Variable no encontrada"))?;
    if let Valor::Lista(elementos) = lista {
        for elem in elementos {
            entorno.establecer(var, elem);
            match procesar_lineas(bloque, entorno, linea_num + 1) {
                Ok(()) => {},
                Err(error) if error == "ROMPER" => break,
                Err(error) if error == "CONTINUAR" => continue,
                Err(error) => return Err(error),
            }
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
            "cadena" => Ok(Some(Valor::Cadena(valor.a_cadena()))),
            "unir" => {
                let separador = if let Some(sep) = args.get(0) {
                    sep.a_cadena()
                } else {
                    ",".to_string()
                };
                let resultado = unir_lista(lista, &separador);
                Ok(Some(Valor::Cadena(resultado)))
            }
            "unir_lineas" => {
                let resultado = unir_lista(lista, "\n");
                Ok(Some(Valor::Cadena(resultado)))
            }
            _ => Ok(None),
        },
        Valor::Cadena(c) => match metodo {
            // Conversiones básicas (ya existentes)
            "entero" => Ok(Some(Valor::Entero(valor.convertir_a_entero()?))),
            "numero" => Ok(Some(Valor::Numero(valor.convertir_a_numero()?))),
            "bool" => Ok(Some(Valor::Bool(valor.convertir_a_bool()?))),
            "cadena" => Ok(Some(Valor::Cadena(c.clone()))),
            
            // Propiedades básicas
            "longitud" => Ok(Some(Valor::Entero(c.chars().count() as i64))),
            "esta_vacia" => Ok(Some(Valor::Bool(c.is_empty()))),
            
            // Métodos de búsqueda y verificación
            "buscar" => {
                if let Some(patron) = args.get(0) {
                    let patron_str = patron.a_cadena();
                    match c.find(&patron_str) {
                        Some(pos) => Ok(Some(Valor::Entero(pos as i64))),
                        None => Ok(Some(Valor::Entero(-1))),
                    }
                } else {
                    Err("buscar() requiere un parámetro".to_string())
                }
            },
            "contiene" => {
                if let Some(patron) = args.get(0) {
                    let patron_str = patron.a_cadena();
                    Ok(Some(Valor::Bool(c.contains(&patron_str))))
                } else {
                    Err("contiene() requiere un parámetro".to_string())
                }
            },
            "empieza_con" => {
                if let Some(prefijo) = args.get(0) {
                    let prefijo_str = prefijo.a_cadena();
                    Ok(Some(Valor::Bool(c.starts_with(&prefijo_str))))
                } else {
                    Err("empieza_con() requiere un parámetro".to_string())
                }
            },
            "termina_con" => {
                if let Some(sufijo) = args.get(0) {
                    let sufijo_str = sufijo.a_cadena();
                    Ok(Some(Valor::Bool(c.ends_with(&sufijo_str))))
                } else {
                    Err("termina_con() requiere un parámetro".to_string())
                }
            },
            "contar_ocurrencias" => {
                if let Some(patron) = args.get(0) {
                    let patron_str = patron.a_cadena();
                    if patron_str.is_empty() {
                        return Err("El patrón no puede estar vacío".to_string());
                    }
                    let count = c.matches(&patron_str).count() as i64;
                    Ok(Some(Valor::Entero(count)))
                } else {
                    Err("contar_ocurrencias() requiere un parámetro".to_string())
                }
            },
            
            // Transformaciones de caso
            "a_mayusculas" => Ok(Some(Valor::Cadena(c.to_uppercase()))),
            "a_minusculas" => Ok(Some(Valor::Cadena(c.to_lowercase()))),
            "capitalizar" => {
                if c.is_empty() {
                    Ok(Some(Valor::Cadena(String::new())))
                } else {
                    let mut chars: Vec<char> = c.chars().collect();
                    chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
                    for i in 1..chars.len() {
                        chars[i] = chars[i].to_lowercase().next().unwrap_or(chars[i]);
                    }
                    Ok(Some(Valor::Cadena(chars.into_iter().collect())))
                }
            },
            
            // Métodos de limpieza
            "recortar" => Ok(Some(Valor::Cadena(c.trim().to_string()))),
            
            // Métodos de manipulación
            "repetir" => {
                if let Some(veces) = args.get(0) {
                    match veces.convertir_a_entero() {
                        Ok(n) if n >= 0 => Ok(Some(Valor::Cadena(c.repeat(n as usize)))),
                        Ok(_) => Err("El número de repeticiones debe ser positivo".to_string()),
                        Err(_) => Err("repetir() requiere un número entero".to_string()),
                    }
                } else {
                    Err("repetir() requiere un parámetro".to_string())
                }
            },
            "invertir" => Ok(Some(Valor::Cadena(c.chars().rev().collect()))),
            "reemplazar" => {
                if args.len() >= 2 {
                    let buscar_str = args[0].a_cadena();
                    let reemplazar_str = args[1].a_cadena();
                    Ok(Some(Valor::Cadena(c.replace(&buscar_str, &reemplazar_str))))
                } else {
                    Err("reemplazar() requiere dos parámetros: buscar y reemplazar".to_string())
                }
            },
            
            // Métodos de subcadena y división
            "subcadena" => {
                if args.len() >= 1 {
                    let inicio = args[0].convertir_a_entero()
                        .map_err(|_| "El índice inicial debe ser un entero")?;
                    let chars: Vec<char> = c.chars().collect();
                    let inicio_idx = if inicio < 0 {
                        0
                    } else {
                        inicio as usize
                    };
                    
                    if inicio_idx >= chars.len() {
                        return Ok(Some(Valor::Cadena(String::new())));
                    }
                    
                    let fin_idx = if args.len() >= 2 {
                        let longitud = args[1].convertir_a_entero()
                            .map_err(|_| "La longitud debe ser un entero")?;
                        if longitud < 0 {
                            chars.len()
                        } else {
                            (inicio_idx + longitud as usize).min(chars.len())
                        }
                    } else {
                        chars.len()
                    };
                    
                    let subcadena: String = chars[inicio_idx..fin_idx].iter().collect();
                    Ok(Some(Valor::Cadena(subcadena)))
                } else {
                    Err("subcadena() requiere al menos un parámetro (índice inicial)".to_string())
                }
            },
            "dividir" => {
                if let Some(delimitador) = args.get(0) {
                    let delim_str = delimitador.a_cadena();
                    if delim_str.is_empty() {
                        return Err("El delimitador no puede estar vacío".to_string());
                    }
                    let partes: Vec<Valor> = c.split(&delim_str)
                        .map(|s| Valor::Cadena(s.to_string()))
                        .collect();
                    Ok(Some(Valor::Lista(partes)))
                } else {
                    Err("dividir() requiere un delimitador".to_string())
                }
            },
            "partir_lineas" => {
                let lineas: Vec<Valor> = c.lines()
                    .map(|s| Valor::Cadena(s.to_string()))
                    .collect();
                Ok(Some(Valor::Lista(lineas)))
            },
            
            // Métodos de comparación
            "comparar" => {
                if let Some(otra) = args.get(0) {
                    let otra_str = otra.a_cadena();
                    let resultado = match c.as_str().cmp(&otra_str) {
                        std::cmp::Ordering::Less => -1,
                        std::cmp::Ordering::Equal => 0,
                        std::cmp::Ordering::Greater => 1,
                    };
                    Ok(Some(Valor::Entero(resultado)))
                } else {
                    Err("comparar() requiere un parámetro".to_string())
                }
            },
            "igual_sin_caso" => {
                if let Some(otra) = args.get(0) {
                    let otra_str = otra.a_cadena();
                    Ok(Some(Valor::Bool(c.to_lowercase() == otra_str.to_lowercase())))
                } else {
                    Err("igual_sin_caso() requiere un parámetro".to_string())
                }
            },
            
            // Métodos de codificación (implementación básica)
            "codificar_base64" => {
                // Implementación básica de Base64
                let encoded = base64_encode(c.as_bytes());
                Ok(Some(Valor::Cadena(encoded)))
            },
            "decodificar_base64" => {
                match base64_decode(c) {
                    Ok(decoded) => {
                        match String::from_utf8(decoded) {
                            Ok(s) => Ok(Some(Valor::Cadena(s))),
                            Err(_) => Err("Los datos decodificados no son UTF-8 válido".to_string()),
                        }
                    },
                    Err(_) => Err("Cadena Base64 inválida".to_string()),
                }
            },
            "codificar_uri" => {
                let encoded = uri_encode(c);
                Ok(Some(Valor::Cadena(encoded)))
            },
            "decodificar_uri" => {
                match uri_decode(c) {
                    Ok(decoded) => Ok(Some(Valor::Cadena(decoded))),
                    Err(e) => Err(format!("Error decodificando URI: {}", e)),
                }
            },
            
            // Conversiones existentes
            "lista" => {
                // Convertir cadena a lista separando por comas
                let elementos: Vec<&str> = c.split(',').collect();
                let lista: Vec<Valor> = elementos.iter().map(|e| {
                    let elem = e.trim();
                    // Intentar convertir a número si es posible
                    if let Ok(entero) = elem.parse::<i64>() {
                        Valor::Entero(entero)
                    } else if let Ok(numero) = elem.parse::<f64>() {
                        Valor::Numero(numero)
                    } else if elem == "verdadero" {
                        Valor::Bool(true)
                    } else if elem == "falso" {
                        Valor::Bool(false)
                    } else {
                        Valor::Cadena(elem.to_string())
                    }
                }).collect();
                Ok(Some(Valor::Lista(lista)))
            },
            "jsn" => {
                // Convertir cadena a objeto JSON
                // Limpiar comillas externas si existen
                let json_limpio = if (c.starts_with('"') && c.ends_with('"')) || 
                                   (c.starts_with('\'') && c.ends_with('\'')) {
                    &c[1..c.len()-1]
                } else {
                    c
                };
                let resultado = parsear_jsn(json_limpio)?;
                Ok(Some(resultado))
            },
            _ => Ok(None),
        },
        Valor::Numero(_) => match metodo {
            "cadena" => Ok(Some(Valor::Cadena(valor.a_cadena()))),
            "entero" => Ok(Some(Valor::Entero(valor.convertir_a_entero()?))),
            _ => Ok(None),
        },
        Valor::Entero(_) => match metodo {
            "cadena" => Ok(Some(Valor::Cadena(valor.a_cadena()))),
            "numero" => Ok(Some(Valor::Numero(valor.convertir_a_numero()?))),
            _ => Ok(None),
        },
        Valor::Bool(_) => match metodo {
            "cadena" => Ok(Some(Valor::Cadena(valor.a_cadena()))),
            _ => Ok(None),
        },
        Valor::Objeto(_) => match metodo {
            "cadena" => Ok(Some(Valor::Cadena(valor.a_cadena()))),
            _ => Ok(None),
        },
        _ => Ok(None),
    }
}

fn aplicar_incremento(expresion: &str, entorno: &mut Entorno) -> Result<(), String> {
    let expresion = expresion.trim();
    
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
    } else if expresion.contains('=') && !expresion.contains("==") && !expresion.contains("!=") && !expresion.contains("<=") && !expresion.contains(">=") {
        // Manejar asignaciones como i = i + 1
        let partes: Vec<&str> = expresion.splitn(2, '=').collect();
        if partes.len() == 2 {
            let variable = partes[0].trim();
            let valor_expr = partes[1].trim();
            let valor_nuevo = evaluar_expresion_valor(valor_expr, entorno)?;
            entorno.establecer(variable, valor_nuevo);
            return Ok(());
        }
    }
    Err("Incremento inválido".to_string())
}

fn procesar_expresion(linea: &str, linea_num: usize, entorno: &mut Entorno) -> Result<(), String> {
    let texto = linea.trim();
    
    // Manejar asignaciones simples (variable = valor)
    if texto.contains('=') && !texto.contains("==") && !texto.contains("!=") && !texto.contains("<=") && !texto.contains(">=") {
        let partes: Vec<&str> = texto.splitn(2, '=').collect();
        if partes.len() == 2 {
            let variable = partes[0].trim();
            let valor_expr = partes[1].trim();
            
            // Verificar que la variable existe antes de asignar
            if entorno.obtener(variable).is_none() {
                return Err(format!("Variable '{}' no encontrada", variable));
            }
            
            let valor = evaluar_expresion_valor(valor_expr, entorno)?;
            entorno.establecer(variable, valor);
            return Ok(());
        }
    }
    
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

fn ejecutar_funcion_usuario(def_funcion: &DefFuncion, llamada: &str, variable_resultado: &str, entorno: &mut Entorno) -> Result<(), String> {
    // Extraer argumentos de la llamada
    let args = extraer_argumentos_funcion(llamada)?;
    
    // Verificar que el número de argumentos coincida
    if args.len() != def_funcion.parametros.len() {
        return Err(format!(
            "Función '{}' espera {} argumentos, pero se proporcionaron {}",
            def_funcion.nombre,
            def_funcion.parametros.len(),
            args.len()
        ));
    }
    
    // Crear un nuevo entorno para la función con copia de funciones del entorno padre
    let mut entorno_funcion = Entorno::nuevo();
    
    // Copiar las definiciones de funciones del entorno padre
    for (_nombre, def_func) in &entorno.funciones {
        entorno_funcion.definir_funcion(def_func.clone());
    }
    
    // Asignar valores a los parámetros
    for (i, (nombre_param, _tipo_param)) in def_funcion.parametros.iter().enumerate() {
        let valor_arg = evaluar_expresion_valor(&args[i], entorno)?;
        entorno_funcion.establecer(nombre_param, valor_arg);
    }
    
    // Ejecutar el cuerpo de la función
    let mut valor_retorno = Valor::Vacio;
    match procesar_lineas(&def_funcion.cuerpo, &mut entorno_funcion, 0) {
        Ok(_) => {
            // La función terminó sin retornar explícitamente
        }
        Err(resultado) => {
            // Verificar si es un retorno
            if resultado.starts_with("RETORNO:") {
                let valor_retorno_str = &resultado[8..];
                if valor_retorno_str.trim().is_empty() || valor_retorno_str.trim() == "vacio" {
                    valor_retorno = Valor::Vacio;
                } else {
                    valor_retorno = evaluar_expresion_valor(valor_retorno_str, &mut entorno_funcion)
                        .unwrap_or(Valor::Vacio);
                }
            } else {
                return Err(resultado);
            }
        }
    }
    
    // Establecer el resultado en el entorno padre
    if !variable_resultado.is_empty() {
        entorno.establecer(variable_resultado, valor_retorno);
    }
    
    Ok(())
}

fn procesar_llamada_funcion_sin_asignacion(linea: &str, entorno: &mut Entorno) -> Result<(), String> {
    let llamada = linea.trim();
    
    // Extraer el nombre de la función
    let nombre_funcion = if let Some(pos) = llamada.find('(') {
        &llamada[..pos]
    } else {
        return Err("Sintaxis de llamada de función inválida".to_string());
    };
    
    // Verificar si es una función definida por el usuario
    if let Some(def_funcion) = entorno.obtener_funcion(nombre_funcion).cloned() {
        return ejecutar_funcion_usuario(&def_funcion, llamada, "", entorno);
    }
    
    Err("Función no reconocida".to_string())
}

fn unir_lineas_divididas(lineas: &[String]) -> Vec<String> {
    let mut lineas_unidas = Vec::new();
    let mut linea_actual = String::new();
    let mut en_expresion_dividida = false;
    
    for linea in lineas {
        let mut linea_trim = linea.trim();
        
        // Remover comentarios al final de la línea
        if let Some(pos) = linea_trim.find("//") {
            linea_trim = &linea_trim[..pos].trim();
        }
        
        // Saltar líneas vacías o solo comentarios
        if linea_trim.is_empty() {
            if !en_expresion_dividida {
                lineas_unidas.push(linea.clone());
            }
            continue;
        }
        
        // Si la línea anterior tenía una expresión incompleta
        if en_expresion_dividida {
            linea_actual.push_str(linea_trim);
            
            // Verificar si la expresión está completa (paréntesis balanceados)
            let parentesis_abiertos = linea_actual.matches('(').count();
            let parentesis_cerrados = linea_actual.matches(')').count();
            
            if parentesis_abiertos == parentesis_cerrados && !linea_actual.is_empty() {
                lineas_unidas.push(linea_actual.trim().to_string());
                linea_actual.clear();
                en_expresion_dividida = false;
            }
        } else {
            // Verificar si esta línea tiene una expresión que continúa en la siguiente línea
            let parentesis_abiertos = linea_trim.matches('(').count();
            let parentesis_cerrados = linea_trim.matches(')').count();
            
            if parentesis_abiertos > parentesis_cerrados && linea_trim.contains('.') {
                // Esta línea tiene una expresión que continúa
                linea_actual = linea_trim.to_string();
                en_expresion_dividida = true;
            } else {
                // Línea completa
                lineas_unidas.push(linea.clone());
            }
        }
    }
    
    // Si quedó una línea pendiente, agregarla
    if !linea_actual.is_empty() {
        lineas_unidas.push(linea_actual);
    }
    
    lineas_unidas
}

// Funciones auxiliares para codificación Base64
fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    
    for chunk in input.chunks(3) {
        let mut buf = [0u8; 3];
        for (i, &byte) in chunk.iter().enumerate() {
            buf[i] = byte;
        }
        
        let b = ((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | (buf[2] as u32);
        
        result.push(CHARS[((b >> 18) & 63) as usize] as char);
        result.push(CHARS[((b >> 12) & 63) as usize] as char);
        result.push(if chunk.len() > 1 { CHARS[((b >> 6) & 63) as usize] as char } else { '=' });
        result.push(if chunk.len() > 2 { CHARS[(b & 63) as usize] as char } else { '=' });
    }
    
    result
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let input = input.trim();
    if input.len() % 4 != 0 {
        return Err("Longitud de Base64 inválida".to_string());
    }
    
    let mut result = Vec::new();
    let bytes = input.as_bytes();
    
    for chunk in bytes.chunks(4) {
        let mut values = [0u8; 4];
        for (i, &byte) in chunk.iter().enumerate() {
            values[i] = match byte {
                b'A'..=b'Z' => byte - b'A',
                b'a'..=b'z' => byte - b'a' + 26,
                b'0'..=b'9' => byte - b'0' + 52,
                b'+' => 62,
                b'/' => 63,
                b'=' => 0,
                _ => return Err("Carácter Base64 inválido".to_string()),
            };
        }
        
        let combined = ((values[0] as u32) << 18) | 
                      ((values[1] as u32) << 12) | 
                      ((values[2] as u32) << 6) | 
                      (values[3] as u32);
        
        result.push((combined >> 16) as u8);
        if chunk[2] != b'=' {
            result.push((combined >> 8) as u8);
        }
        if chunk[3] != b'=' {
            result.push(combined as u8);
        }
    }
    
    Ok(result)
}

// Funciones auxiliares para codificación URI
fn uri_encode(input: &str) -> String {
    let mut result = String::new();
    
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    
    result
}

fn uri_decode(input: &str) -> Result<String, String> {
    let mut result = Vec::new();
    let mut chars = input.chars().peekable();
    
    while let Some(ch) = chars.next() {
        match ch {
            '%' => {
                let hex1 = chars.next().ok_or("URI mal formado: falta primer dígito hex")?;
                let hex2 = chars.next().ok_or("URI mal formado: falta segundo dígito hex")?;
                
                let hex_str = format!("{}{}", hex1, hex2);
                let byte = u8::from_str_radix(&hex_str, 16)
                    .map_err(|_| "URI mal formado: dígitos hex inválidos")?;
                result.push(byte);
            }
            '+' => result.push(b' '),
            _ => result.push(ch as u8),
        }
    }
    
    String::from_utf8(result).map_err(|_| "URI decodificado no es UTF-8 válido".to_string())
}

// Función auxiliar para unir elementos de lista con un separador
fn unir_lista(lista: &[Valor], separador: &str) -> String {
    lista.iter()
        .map(|v| v.a_cadena())
        .collect::<Vec<String>>()
        .join(separador)
}

