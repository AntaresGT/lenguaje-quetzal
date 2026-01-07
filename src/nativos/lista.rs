// Métodos nativos de lista - extensión del tipo Lista
// Se implementará como métodos que se pueden llamar en valores de tipo Lista

use crate::interprete::valores::Valor;
use rust_decimal::Decimal;
use serde_json::Value as JsonValue;

/// Aplica un método de lista a un valor
pub fn aplicar_metodo_lista(
    metodo: &str,
    lista: &mut Vec<Valor>,
    argumentos: Vec<Valor>,
    es_mutable: bool,
) -> Result<(Valor, bool), String> {
    // Retorna (resultado, necesita_actualizar_lista)
    match metodo {
        "longitud" => {
            if !argumentos.is_empty() {
                return Err("método 'longitud()' no acepta argumentos".to_string());
            }
            Ok((Valor::Entero(lista.len() as i64), false))
        }
        "esta_vacia" => {
            if !argumentos.is_empty() {
                return Err("método 'esta_vacia()' no acepta argumentos".to_string());
            }
            Ok((Valor::Logico(lista.is_empty()), false))
        }
        "agregar" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'agregar()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            if !es_mutable {
                return Err("método 'agregar()' requiere una lista mutable".to_string());
            }
            lista.push(argumentos[0].clone());
            Ok((Valor::Vacio, true))
        }
        "insertar" => {
            if argumentos.len() != 2 {
                return Err(format!("método 'insertar()' requiere 2 argumentos, se recibieron {}", argumentos.len()));
            }
            if !es_mutable {
                return Err("método 'insertar()' requiere una lista mutable".to_string());
            }
            let indice = match &argumentos[0] {
                Valor::Entero(n) => *n,
                _ => return Err("método 'insertar()' requiere un índice de tipo entero".to_string()),
            };
            if indice < 0 || indice as usize > lista.len() {
                return Err(format!("índice {} fuera de rango para lista de tamaño {}", indice, lista.len()));
            }
            lista.insert(indice as usize, argumentos[1].clone());
            Ok((Valor::Vacio, true))
        }
        "remover" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'remover()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            if !es_mutable {
                return Err("método 'remover()' requiere una lista mutable".to_string());
            }
            if let Some(pos) = lista.iter().position(|v| v.es_igual(&argumentos[0])) {
                lista.remove(pos);
                Ok((Valor::Vacio, true))
            } else {
                Ok((Valor::Vacio, true))
            }
        }
        "quitar_en" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'quitar_en()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            if !es_mutable {
                return Err("método 'quitar_en()' requiere una lista mutable".to_string());
            }
            let indice = match &argumentos[0] {
                Valor::Entero(n) => *n,
                _ => return Err("método 'quitar_en()' requiere un índice de tipo entero".to_string()),
            };
            let indice_usize = if indice < 0 {
                let len = lista.len() as i64;
                if indice + len < 0 || indice + len >= len {
                    return Err(format!("índice {} fuera de rango", indice));
                }
                (indice + len) as usize
            } else {
                if indice as usize >= lista.len() {
                    return Err(format!("índice {} fuera de rango", indice));
                }
                indice as usize
            };
            lista.remove(indice_usize);
            Ok((Valor::Vacio, true))
        }
        "limpiar" => {
            if !argumentos.is_empty() {
                return Err("método 'limpiar()' no acepta argumentos".to_string());
            }
            if !es_mutable {
                return Err("método 'limpiar()' requiere una lista mutable".to_string());
            }
            lista.clear();
            Ok((Valor::Vacio, true))
        }
        "contiene" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'contiene()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            Ok((Valor::Logico(lista.iter().any(|v| v.es_igual(&argumentos[0]))), false))
        }
        "buscar" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'buscar()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            match lista.iter().position(|v| v.es_igual(&argumentos[0])) {
                Some(pos) => Ok((Valor::Entero(pos as i64), false)),
                None => Ok((Valor::Entero(-1), false)),
            }
        }
        "buscar_ultimo" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'buscar_ultimo()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            match lista.iter().rposition(|v| v.es_igual(&argumentos[0])) {
                Some(pos) => Ok((Valor::Entero(pos as i64), false)),
                None => Ok((Valor::Entero(-1), false)),
            }
        }
        "contar" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'contar()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            let count = lista.iter().filter(|v| v.es_igual(&argumentos[0])).count();
            Ok((Valor::Entero(count as i64), false))
        }
        "ordenar" => {
            if !argumentos.is_empty() {
                return Err("método 'ordenar()' no acepta argumentos".to_string());
            }
            if !es_mutable {
                return Err("método 'ordenar()' requiere una lista mutable".to_string());
            }
            lista.sort_by(|a, b| {
                match (a, b) {
                    (Valor::Entero(ia), Valor::Entero(ib)) => ia.cmp(ib),
                    (Valor::Numero(na), Valor::Numero(nb)) => na.cmp(nb),
                    (Valor::Texto(sa), Valor::Texto(sb)) => sa.cmp(sb),
                    (Valor::Logico(ba), Valor::Logico(bb)) => ba.cmp(bb),
                    _ => std::cmp::Ordering::Equal,
                }
            });
            Ok((Valor::Vacio, true))
        }
        "ordenar_descendente" => {
            if !argumentos.is_empty() {
                return Err("método 'ordenar_descendente()' no acepta argumentos".to_string());
            }
            if !es_mutable {
                return Err("método 'ordenar_descendente()' requiere una lista mutable".to_string());
            }
            lista.sort_by(|a, b| {
                match (a, b) {
                    (Valor::Entero(ia), Valor::Entero(ib)) => ib.cmp(ia),
                    (Valor::Numero(na), Valor::Numero(nb)) => nb.cmp(na),
                    (Valor::Texto(sa), Valor::Texto(sb)) => sb.cmp(sa),
                    (Valor::Logico(ba), Valor::Logico(bb)) => bb.cmp(ba),
                    _ => std::cmp::Ordering::Equal,
                }
            });
            Ok((Valor::Vacio, true))
        }
        "ordenado" => {
            if !argumentos.is_empty() {
                return Err("método 'ordenado()' no acepta argumentos".to_string());
            }
            let mut copia = lista.clone();
            copia.sort_by(|a, b| {
                match (a, b) {
                    (Valor::Entero(ia), Valor::Entero(ib)) => ia.cmp(ib),
                    (Valor::Numero(na), Valor::Numero(nb)) => na.cmp(nb),
                    (Valor::Texto(sa), Valor::Texto(sb)) => sa.cmp(sb),
                    (Valor::Logico(ba), Valor::Logico(bb)) => ba.cmp(bb),
                    _ => std::cmp::Ordering::Equal,
                }
            });
            Ok((Valor::Lista(copia), false))
        }
        "invertir" => {
            if !argumentos.is_empty() {
                return Err("método 'invertir()' no acepta argumentos".to_string());
            }
            if !es_mutable {
                return Err("método 'invertir()' requiere una lista mutable".to_string());
            }
            lista.reverse();
            Ok((Valor::Vacio, true))
        }
        "primero" => {
            if !argumentos.is_empty() {
                return Err("método 'primero()' no acepta argumentos".to_string());
            }
            match lista.first() {
                Some(valor) => Ok((valor.clone(), false)),
                None => Err("lista vacía, no se puede obtener el primer elemento".to_string()),
            }
        }
        "ultimo" => {
            if !argumentos.is_empty() {
                return Err("método 'ultimo()' no acepta argumentos".to_string());
            }
            match lista.last() {
                Some(valor) => Ok((valor.clone(), false)),
                None => Err("lista vacía, no se puede obtener el último elemento".to_string()),
            }
        }
        "tomar" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'tomar()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            let n = match &argumentos[0] {
                Valor::Entero(num) => *num,
                _ => return Err("método 'tomar()' requiere un argumento de tipo entero".to_string()),
            };
            if n < 0 {
                return Err("método 'tomar()' requiere un número no negativo".to_string());
            }
            let n_usize = n as usize;
            let resultado: Vec<Valor> = lista.iter().take(n_usize).cloned().collect();
            Ok((Valor::Lista(resultado), false))
        }
        "saltar" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'saltar()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            let n = match &argumentos[0] {
                Valor::Entero(num) => *num,
                _ => return Err("método 'saltar()' requiere un argumento de tipo entero".to_string()),
            };
            if n < 0 {
                return Err("método 'saltar()' requiere un número no negativo".to_string());
            }
            let n_usize = n as usize;
            let resultado: Vec<Valor> = lista.iter().skip(n_usize).cloned().collect();
            Ok((Valor::Lista(resultado), false))
        }
        "sublista" => {
            if argumentos.len() != 2 {
                return Err(format!("método 'sublista()' requiere 2 argumentos, se recibieron {}", argumentos.len()));
            }
            let inicio = match &argumentos[0] {
                Valor::Entero(n) => *n,
                _ => return Err("método 'sublista()' requiere argumentos de tipo entero".to_string()),
            };
            let fin = match &argumentos[1] {
                Valor::Entero(n) => *n,
                _ => return Err("método 'sublista()' requiere argumentos de tipo entero".to_string()),
            };
            if inicio < 0 || fin < inicio || fin as usize > lista.len() {
                return Err(format!("índices fuera de rango: inicio={}, fin={}, tamaño={}", inicio, fin, lista.len()));
            }
            let resultado: Vec<Valor> = lista[inicio as usize..fin as usize].to_vec();
            Ok((Valor::Lista(resultado), false))
        }
        "sumar" => {
            if !argumentos.is_empty() {
                return Err("método 'sumar()' no acepta argumentos".to_string());
            }
            let mut suma_entero: i64 = 0;
            let mut suma_numero = Decimal::ZERO;
            let mut tiene_enteros = false;
            let mut tiene_numeros = false;
            
            for valor in lista.iter() {
                match valor {
                    Valor::Entero(n) => {
                        suma_entero += n;
                        tiene_enteros = true;
                    }
                    Valor::Numero(n) => {
                        suma_numero += n;
                        tiene_numeros = true;
                    }
                    _ => {
                        return Err("método 'sumar()' solo funciona con listas de números".to_string());
                    }
                }
            }
            
            if tiene_numeros {
                Ok((Valor::Numero(suma_numero + Decimal::from(suma_entero)), false))
            } else if tiene_enteros {
                Ok((Valor::Entero(suma_entero), false))
            } else {
                Ok((Valor::Entero(0), false))
            }
        }
        "promedio" => {
            if !argumentos.is_empty() {
                return Err("método 'promedio()' no acepta argumentos".to_string());
            }
            if lista.is_empty() {
                return Err("no se puede calcular el promedio de una lista vacía".to_string());
            }
            
            let mut suma = Decimal::ZERO;
            let mut count = 0;
            
            for valor in lista.iter() {
                match valor {
                    Valor::Entero(n) => {
                        suma += Decimal::from(*n);
                        count += 1;
                    }
                    Valor::Numero(n) => {
                        suma += *n;
                        count += 1;
                    }
                    _ => {
                        return Err("método 'promedio()' solo funciona con listas de números".to_string());
                    }
                }
            }
            
            Ok((Valor::Numero(suma / Decimal::from(count)), false))
        }
        "maximo" => {
            if !argumentos.is_empty() {
                return Err("método 'maximo()' no acepta argumentos".to_string());
            }
            if lista.is_empty() {
                return Err("no se puede obtener el máximo de una lista vacía".to_string());
            }
            
            let mut max_valor = None;
            
            for valor in lista.iter() {
                match valor {
                    Valor::Entero(n) => {
                        if let Some(Valor::Entero(max)) = max_valor {
                            if *n > max {
                                max_valor = Some(valor.clone());
                            }
                        } else if let Some(Valor::Numero(max)) = max_valor {
                            let n_decimal = Decimal::from(*n);
                            if n_decimal > max {
                                max_valor = Some(valor.clone());
                            }
                        } else {
                            max_valor = Some(valor.clone());
                        }
                    }
                    Valor::Numero(n) => {
                        if let Some(Valor::Entero(max)) = max_valor {
                            let max_decimal = Decimal::from(max);
                            if *n > max_decimal {
                                max_valor = Some(valor.clone());
                            }
                        } else if let Some(Valor::Numero(max)) = max_valor {
                            if *n > max {
                                max_valor = Some(valor.clone());
                            }
                        } else {
                            max_valor = Some(valor.clone());
                        }
                    }
                    _ => {
                        return Err("método 'maximo()' solo funciona con listas de números".to_string());
                    }
                }
            }
            
            Ok((max_valor.unwrap(), false))
        }
        "minimo" => {
            if !argumentos.is_empty() {
                return Err("método 'minimo()' no acepta argumentos".to_string());
            }
            if lista.is_empty() {
                return Err("no se puede obtener el mínimo de una lista vacía".to_string());
            }
            
            let mut min_valor = None;
            
            for valor in lista.iter() {
                match valor {
                    Valor::Entero(n) => {
                        if let Some(Valor::Entero(min)) = min_valor {
                            if *n < min {
                                min_valor = Some(valor.clone());
                            }
                        } else if let Some(Valor::Numero(min)) = min_valor {
                            let n_decimal = Decimal::from(*n);
                            if n_decimal < min {
                                min_valor = Some(valor.clone());
                            }
                        } else {
                            min_valor = Some(valor.clone());
                        }
                    }
                    Valor::Numero(n) => {
                        if let Some(Valor::Entero(min)) = min_valor {
                            let min_decimal = Decimal::from(min);
                            if *n < min_decimal {
                                min_valor = Some(valor.clone());
                            }
                        } else if let Some(Valor::Numero(min)) = min_valor {
                            if *n < min {
                                min_valor = Some(valor.clone());
                            }
                        } else {
                            min_valor = Some(valor.clone());
                        }
                    }
                    _ => {
                        return Err("método 'minimo()' solo funciona con listas de números".to_string());
                    }
                }
            }
            
            Ok((min_valor.unwrap(), false))
        }
        "unir" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'unir()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            let separador = match &argumentos[0] {
                Valor::Texto(s) => s.clone(),
                _ => return Err("método 'unir()' requiere un argumento de tipo texto".to_string()),
            };
            let partes: Vec<String> = lista.iter().map(|v| v.a_texto()).collect();
            Ok((Valor::Texto(partes.join(&separador)), false))
        }
        "concatenar" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'concatenar()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            let otra_lista = match &argumentos[0] {
                Valor::Lista(l) => l.clone(),
                _ => return Err("método 'concatenar()' requiere un argumento de tipo lista".to_string()),
            };
            let mut resultado = lista.clone();
            resultado.extend(otra_lista);
            Ok((Valor::Lista(resultado), false))
        }
        "extender" => {
            if argumentos.len() != 1 {
                return Err(format!("método 'extender()' requiere 1 argumento, se recibieron {}", argumentos.len()));
            }
            if !es_mutable {
                return Err("método 'extender()' requiere una lista mutable".to_string());
            }
            let otra_lista = match &argumentos[0] {
                Valor::Lista(l) => l.clone(),
                _ => return Err("método 'extender()' requiere un argumento de tipo lista".to_string()),
            };
            lista.extend(otra_lista);
            Ok((Valor::Vacio, true))
        }
        "texto" => {
            if !argumentos.is_empty() {
                return Err("método 'texto()' no acepta argumentos".to_string());
            }
            let elementos: Vec<String> = lista.iter().map(|e| e.a_texto()).collect();
            Ok((Valor::Texto(format!("[{}]", elementos.join(", "))), false))
        }
        "json" => {
            if !argumentos.is_empty() {
                return Err("método 'json()' no acepta argumentos".to_string());
            }
            let elementos: Vec<JsonValue> = lista.iter().map(|v| {
                match v {
                    Valor::Vacio => JsonValue::Null,
                    Valor::Entero(n) => JsonValue::Number((*n).into()),
                    Valor::Numero(n) => {
                        let f64_val: f64 = n.to_string().parse().unwrap_or(0.0);
                        JsonValue::Number(serde_json::Number::from_f64(f64_val).unwrap_or(serde_json::Number::from(0)))
                    }
                    Valor::Texto(s) => JsonValue::String(s.clone()),
                    Valor::Logico(b) => JsonValue::Bool(*b),
                    Valor::Lista(l) => {
                        let arr: Vec<JsonValue> = l.iter().map(|val| {
                            match val {
                                Valor::Vacio => JsonValue::Null,
                                Valor::Entero(n) => JsonValue::Number((*n).into()),
                                Valor::Texto(s) => JsonValue::String(s.clone()),
                                Valor::Logico(b) => JsonValue::Bool(*b),
                                _ => JsonValue::String(val.a_texto()),
                            }
                        }).collect();
                        JsonValue::Array(arr)
                    }
                    Valor::Json(j) => j.clone(),
                    _ => JsonValue::String(v.a_texto()),
                }
            }).collect();
            Ok((Valor::Texto(serde_json::to_string(&elementos).unwrap_or_else(|_| "[]".to_string())), false))
        }
        "logico" | "lóg" => {
            if !argumentos.is_empty() {
                return Err("método 'logico()' no acepta argumentos".to_string());
            }
            Ok((Valor::Logico(!lista.is_empty()), false))
        }
        _ => Err(format!("método '{}' no encontrado", metodo)),
    }
}
