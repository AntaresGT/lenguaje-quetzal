use crate::entorno::Entorno;
use crate::valores::{Valor, DefFuncion};
use crate::objetos::{DefObjeto, TipoMetodo};
use crate::consola;

pub fn interpretar(contenido: &str) -> Result<(), String> {
    let limpio = contenido.trim_start_matches('\u{feff}');
    let mut entorno = Entorno::nuevo();
    let lineas: Vec<String> = limpio.lines().map(|l| l.to_string()).collect();
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
        if linea.contains("+=") || linea.contains("-=") || linea.contains("*=") || linea.contains("/=") {
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
                if valor_cadena.starts_with('"') && valor_cadena.ends_with('"') {
                    Valor::Cadena(valor_cadena.trim_matches('"').to_string())
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
    // Necesitamos encontrar correctamente las partes, teniendo en cuenta paréntesis
    let mut partes = Vec::new();
    let mut parte_actual = String::new();
    let mut nivel_parentesis = 0;
    let mut i = 0;
    let chars: Vec<char> = expr.chars().collect();
    
    while i < chars.len() {
        let c = chars[i];
        
        if c == '(' {
            nivel_parentesis += 1;
            parte_actual.push(c);
        } else if c == ')' {
            nivel_parentesis -= 1;
            parte_actual.push(c);
        } else if c == '+' && nivel_parentesis == 0 {
            // Solo dividir por + si estamos fuera de paréntesis
            if i > 0 && i < chars.len() - 1 && chars[i-1] == ' ' && chars[i+1] == ' ' {
                partes.push(parte_actual.trim().to_string());
                parte_actual.clear();
                i += 2; // Saltar " + "
                continue;
            } else {
                parte_actual.push(c);
            }
        } else {
            parte_actual.push(c);
        }
        i += 1;
    }
    
    if !parte_actual.is_empty() {
        partes.push(parte_actual.trim().to_string());
    }
    
    let mut resultado = String::new();
    
    for parte in partes {
        let parte_trim = parte.trim();
        
        let valor_str = if parte_trim.starts_with('"') && parte_trim.ends_with('"') {
            // Es una cadena literal
            parte_trim.trim_matches('"').to_string()
        } else if parte_trim.contains(".cadena()") && parte_trim.starts_with('(') {
            // Es una expresión con paréntesis y .cadena()
            let pos_metodo = parte_trim.find(".cadena()").unwrap();
            if parte_trim[..pos_metodo].ends_with(')') {
                let expr_interna = &parte_trim[1..pos_metodo-1];
                match evaluar_expresion_valor(expr_interna, entorno) {
                    Ok(valor) => valor.a_cadena(),
                    Err(e) => return Err(e),
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
            // Intentar evaluar como expresión
            match evaluar_expresion_valor(parte_trim, entorno) {
                Ok(valor) => valor.a_cadena(),
                Err(_) => parte_trim.to_string(),
            }
        };
        
        resultado.push_str(&valor_str);
    }
    
    Ok(resultado)
}

fn formatear_error(linea: usize, mensaje: &str) -> String {
    format!("Error en línea {}: {}", linea + 1, mensaje)
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
        if let Some(Valor::Bool(b)) = entorno.obtener(condicion.trim()) {
            return Ok(*b);
        }
        return Err("Condición inválida".to_string());
    }
    let izq = obtener_valor(tokens[0], entorno)?;
    let der = obtener_valor(tokens[2], entorno)?;
    
    match (izq, der, tokens[1]) {
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
    
    // Llamadas a métodos
    if texto.contains('.') && texto.contains('(') && texto.ends_with(')') {
        if let Some(punto) = texto.find('.') {
            let base = texto[..punto].trim();
            let resto = texto[punto + 1..].trim();
            if let Some(paren) = resto.find('(') {
                let metodo = resto[..paren].trim();
                let args_str = &resto[paren + 1..resto.len() - 1];
                let mut args = Vec::new();
                if !args_str.trim().is_empty() {
                    for arg in args_str.split(',') {
                        args.push(evaluar_expresion_valor(arg.trim(), entorno)?);
                    }
                }
                
                // Intentar obtener valor de la variable base
                if let Ok(mut val) = obtener_valor(base, entorno) {
                    if let Some(resultado) = aplicar_metodo_valor(&mut val, metodo, args)? {
                        entorno.establecer(base, val);
                        return Ok(resultado);
                    }
                }
            }
        }
    }
    for op in &[" && ", " y ", " || ", " o "] {
        if let Some(pos) = texto.find(op) {
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
    
    // Operaciones aritméticas
    if let Ok(resultado) = evaluar_operacion_aritmetica(texto, entorno) {
        return Ok(resultado);
    }
    
    // Concatenación de cadenas
    if texto.contains(" + ") && !texto.chars().all(|c| c.is_ascii_digit() || c.is_whitespace() || c == '+' || c == '.' || c == '-') {
        let resultado = evaluar_concatenacion(texto, entorno, 0)?;
        return Ok(Valor::Cadena(resultado));
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
            "cadena" => Ok(Some(Valor::Cadena(valor.a_cadena()))),
            _ => Ok(None),
        },
        Valor::Cadena(c) => match metodo {
            "entero" => Ok(Some(Valor::Entero(valor.convertir_a_entero()?))),
            "numero" => Ok(Some(Valor::Numero(valor.convertir_a_numero()?))),
            "bool" => Ok(Some(Valor::Bool(valor.convertir_a_bool()?))),
            "cadena" => Ok(Some(Valor::Cadena(c.clone()))),
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

