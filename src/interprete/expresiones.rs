use crate::errores::{Error, CodigoError, Resultado};
use crate::interprete::entorno::Entorno;
use crate::interprete::valores::Valor;
use crate::nucleo::sintactico::ast::*;
use crate::nativos::texto::aplicar_metodo_texto;
use crate::nativos::lista::aplicar_metodo_lista;
use crate::nativos::json::aplicar_metodo_json;
use rust_decimal::Decimal;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Evalúa una expresión
pub fn evaluar_expresion(nodo: &NodoAst, entorno: &mut Entorno) -> Resultado<Valor> {
    match nodo {
        NodoAst::ExpresionLiteral { valor, .. } => {
            Ok(match valor {
                LiteralAst::Entero(val) => Valor::Entero(*val),
                LiteralAst::Numero(val) => {
                    Valor::Numero(val.parse().map_err(|_| Error::ejecucion(
                        CodigoError::NumeroMalformado,
                        format!("número inválido: {}", val),
                        None,
                        Some(nodo.posicion().linea),
                        Some(nodo.posicion().columna),
                    ))?)
                }
                LiteralAst::Texto(val) => Valor::Texto(val.clone()),
                LiteralAst::InterpolacionTexto(val) => {
                    procesar_interpolacion_texto(val, entorno, nodo)?
                }
                LiteralAst::Logico(val) => Valor::Logico(*val),
                LiteralAst::Nulo => Valor::Vacio,
            })
        }
        
        NodoAst::ExpresionIdentificador { nombre, .. } => {
            // Verificar si es una función nativa global como rango
            if nombre == "rango" {
                // Crear una función especial que se ejecutará cuando se llame
                return Ok(Valor::Funcion {
                    nombre: "rango".to_string(),
                    parametros: vec!["inicio".to_string(), "fin".to_string()],
                    parametros_mutables: vec![false, false],
                    cuerpo: Box::new(NodoAst::Bloque {
                        declaraciones: vec![],
                        posicion: nodo.posicion(),
                    }),
                    asincrono: false,
                });
            }
            
            // Manejar 'ambiente' como identificador especial (referencia al objeto actual)
            if nombre == "ambiente" {
                // Retornar un objeto especial que representa el ambiente actual
                return Ok(Valor::Objeto {
                    tipo: "ambiente".to_string(),
                    propiedades: HashMap::new(),
                });
            }
            
            // Manejar 'padre' como identificador especial
            if nombre == "padre" {
                // Retornar un objeto especial que representa el padre
                return Ok(Valor::Objeto {
                    tipo: "padre".to_string(),
                    propiedades: HashMap::new(),
                });
            }
            
            // Primero verificar si es un objeto nativo global (como consola)
            if let Some(_modulo_nativo) = entorno.obtener_objeto_nativo(nombre) {
                // Crear un valor objeto especial para objetos nativos
                return Ok(Valor::Objeto {
                    tipo: "nativo".to_string(),
                    propiedades: HashMap::new(),
                });
            }
            
            // Luego verificar variables normales
            entorno.obtener_variable(nombre)
                .cloned()
                .ok_or_else(|| Error::ejecucion(
                    CodigoError::VariableNoDeclarada,
                    format!("variable '{}' no está definida", nombre),
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                ))
        }
        
        NodoAst::ExpresionAcceso { objeto, miembro, .. } => {
            // Si el objeto es un identificador que apunta a un objeto nativo (ej: consola, Matemática)
            if let NodoAst::ExpresionIdentificador { nombre, .. } = objeto.as_ref() {
                // Manejar acceso a ambiente.X (propiedades del objeto actual)
                if nombre == "ambiente" {
                    // Buscar la variable ambiente.X
                    let nombre_completo = format!("ambiente.{}", miembro);
                    if let Some(valor) = entorno.obtener_variable(&nombre_completo) {
                        return Ok(valor.clone());
                    }
                    // Si no existe, retornar nulo
                    return Ok(Valor::Vacio);
                }
                
                // Manejar acceso a padre.X
                if nombre == "padre" {
                    // padre.NombrePadre - crear un valor especial para acceder al padre
                    let mut props = HashMap::new();
                    props.insert("nombre_padre".to_string(), Valor::Texto(miembro.clone()));
                    props.insert("es_acceso_padre".to_string(), Valor::Logico(true));
                    
                    return Ok(Valor::Objeto {
                        tipo: "acceso_padre".to_string(),
                        propiedades: props,
                    });
                }
                
                if let Some(modulo_nativo) = entorno.obtener_objeto_nativo(nombre) {
                    // Primero verificar si es una constante del módulo nativo (ej: Matemática.PI)
                    if let Some(constante) = modulo_nativo.obtener_constante(miembro) {
                        return Ok(constante);
                    }
                    
                    // Si no es una constante, es un acceso a método de objeto nativo (ej: Matemática.sumar)
                    // Retornamos un valor especial que se procesará en ExpresionLlamada cuando se llame
                    return Ok(Valor::Objeto {
                        tipo: format!("nativo:{}", nombre),
                        propiedades: {
                            let mut props = HashMap::new();
                            props.insert("metodo".to_string(), Valor::Texto(miembro.clone()));
                            props.insert("objeto_nativo".to_string(), Valor::Texto(nombre.clone()));
                            props
                        },
                    });
                }
            }
            
            // Evaluar el objeto normalmente
            let valor_objeto = evaluar_expresion(objeto, entorno)?;
            
            // Si el valor es un objeto nativo especial (de una llamada anterior)
            if let Valor::Objeto { tipo, propiedades } = &valor_objeto {
                if tipo.starts_with("nativo:") {
                    // Esto es un acceso encadenado a objeto nativo, no soportado aún
                    return Err(Error::ejecucion(
                        CodigoError::OperacionNoSoportada,
                        "acceso encadenado a objetos nativos no soportado",
                        None,
                        Some(nodo.posicion().linea),
                        Some(nodo.posicion().columna),
                    ));
                }
                
                // Si es un acceso_padre (padre.NombrePadre), manejar acceso a método
                if tipo == "acceso_padre" {
                    if let Some(Valor::Texto(nombre_padre)) = propiedades.get("nombre_padre") {
                        // padre.Mamifero.metodo -> crear valor especial para método de padre
                        let mut props = HashMap::new();
                        props.insert("nombre_padre".to_string(), Valor::Texto(nombre_padre.clone()));
                        props.insert("metodo".to_string(), Valor::Texto(miembro.clone()));
                        props.insert("es_metodo_padre".to_string(), Valor::Logico(true));
                        
                        return Ok(Valor::Objeto {
                            tipo: "metodo_padre".to_string(),
                            propiedades: props,
                        });
                    }
                }
            }
            
            // Si es un JSON, intentar acceder a la propiedad directamente
            if let Valor::Json(JsonValue::Object(obj)) = &valor_objeto {
                if let Some(valor) = obj.get(miembro) {
                    // Convertir JsonValue a Valor
                    return Ok(convertir_json_a_valor(valor));
                }
                // Si no existe la propiedad, puede ser un método, así que continuamos
            }
            
            // Si el miembro es un método que se puede llamar en el valor primitivo
            // Retornamos un valor especial que se procesará en ExpresionLlamada cuando se llame
            match &valor_objeto {
                Valor::Entero(_) | Valor::Numero(_) | Valor::Texto(_) | Valor::Logico(_) | Valor::Lista(_) | Valor::Json(_) => {
                    // Obtener información sobre la variable si el objeto es un identificador
                    let (nombre_variable, es_mutable) = if let NodoAst::ExpresionIdentificador { nombre, .. } = objeto.as_ref() {
                        // Verificar si es una variable mutable
                        if let Some((_, es_mut)) = entorno.obtener_info_variable(nombre) {
                            (Some(nombre.clone()), es_mut)
                        } else {
                            (Some(nombre.clone()), false)
                        }
                    } else {
                        (None, false)
                    };
                    
                    // Crear un valor especial para métodos de valores primitivos
                    let mut props = HashMap::new();
                    props.insert("metodo".to_string(), Valor::Texto(miembro.clone()));
                    props.insert("valor".to_string(), valor_objeto.clone());
                    if let Some(nombre_var) = nombre_variable {
                        props.insert("variable_nombre".to_string(), Valor::Texto(nombre_var));
                        props.insert("variable_mutable".to_string(), Valor::Logico(es_mutable));
                    }
                    
                    Ok(Valor::Objeto {
                        tipo: format!("metodo:{}", valor_objeto.tipo()),
                        propiedades: props,
                    })
                }
                Valor::Objeto { propiedades, tipo } => {
                    if let Some(valor_miembro) = propiedades.get(miembro) {
                        // Si el miembro es una función, crear un método enlazado que incluya las propiedades del objeto
                        if let Valor::Funcion { nombre, parametros, parametros_mutables, cuerpo, asincrono } = valor_miembro {
                            let mut props = HashMap::new();
                            props.insert("__es_metodo_objeto__".to_string(), Valor::Logico(true));
                            props.insert("__tipo_objeto__".to_string(), Valor::Texto(tipo.clone()));
                            props.insert("__nombre_funcion__".to_string(), Valor::Texto(nombre.clone()));
                            props.insert("__funcion__".to_string(), Valor::Funcion {
                                nombre: nombre.clone(),
                                parametros: parametros.clone(),
                                parametros_mutables: parametros_mutables.clone(),
                                cuerpo: cuerpo.clone(),
                                asincrono: *asincrono,
                            });
                            // Copiar las propiedades del objeto para ambiente.X
                            for (prop_nombre, prop_valor) in propiedades {
                                if !prop_nombre.starts_with("__") {
                                    props.insert(format!("__prop_{}__", prop_nombre), prop_valor.clone());
                                }
                            }
                            return Ok(Valor::Objeto {
                                tipo: "metodo_objeto".to_string(),
                                propiedades: props,
                            });
                        }
                        return Ok(valor_miembro.clone());
                    }
                    Err(Error::ejecucion(
                        CodigoError::PropiedadInexistenteJson,
                        format!("propiedad '{}' no existe", miembro),
                        None,
                        Some(nodo.posicion().linea),
                        Some(nodo.posicion().columna),
                    ))
                }
                _ => Err(Error::ejecucion(
                    CodigoError::TiposIncompatibles,
                    format!("no se puede acceder a propiedad '{}' en tipo {}", miembro, valor_objeto.tipo()),
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            }
        }
        
        NodoAst::ExpresionLlamada { funcion, argumentos, .. } => {
            // Evaluar argumentos primero
            let argumentos_evaluados: Resultado<Vec<Valor>> = argumentos.iter()
                .map(|arg| evaluar_expresion(arg, entorno))
                .collect();
            let argumentos_evaluados = argumentos_evaluados?;
            
            // Verificar si es una llamada a método de objeto nativo (ej: consola.mostrar())
            if let NodoAst::ExpresionAcceso { objeto, miembro, .. } = funcion.as_ref() {
                if let NodoAst::ExpresionIdentificador { nombre, .. } = objeto.as_ref() {
                    if let Some(modulo_nativo) = entorno.obtener_objeto_nativo(nombre) {
                        // Llamar al método del módulo nativo
                        return modulo_nativo.llamar_funcion(miembro, argumentos_evaluados, entorno);
                    }
                }
            }
            
            // Evaluar la función (puede ser un objeto nativo especial de ExpresionAcceso)
            let valor_funcion = evaluar_expresion(funcion, entorno)?;
            
            // Si el valor es un objeto nativo especial con método, llamarlo
            if let Valor::Objeto { tipo, propiedades } = &valor_funcion {
                if tipo.starts_with("nativo:") {
                    if let (Some(Valor::Texto(objeto_nombre)), Some(Valor::Texto(metodo))) = 
                        (propiedades.get("objeto_nativo"), propiedades.get("metodo")) {
                        if let Some(modulo_nativo) = entorno.obtener_objeto_nativo(objeto_nombre) {
                            return modulo_nativo.llamar_funcion(metodo, argumentos_evaluados, entorno);
                        }
                    }
                }
                
                // Si es un método de valor primitivo (ej: entero.texto())
                if tipo.starts_with("metodo:") {
                    if let (Some(Valor::Texto(metodo)), Some(valor)) = 
                        (propiedades.get("metodo"), propiedades.get("valor")) {
                        // Obtener información sobre la variable original si existe
                        let nombre_variable = propiedades.get("variable_nombre")
                            .and_then(|v| if let Valor::Texto(s) = v { Some(s.clone()) } else { None });
                        let es_mutable = propiedades.get("variable_mutable")
                            .and_then(|v| if let Valor::Logico(b) = v { Some(*b) } else { None })
                            .unwrap_or(false);
                        
                        return aplicar_metodo_primitivo(metodo, valor, argumentos_evaluados, nodo, entorno, nombre_variable, es_mutable);
                    }
                }
                
                // Si es una llamada a constructor de padre (padre.Mamifero(args))
                if tipo == "acceso_padre" {
                    if let Some(Valor::Texto(nombre_padre)) = propiedades.get("nombre_padre") {
                        // Obtener la definición del padre
                        if let Some(def_padre) = entorno.obtener_definicion_objeto(nombre_padre) {
                            let def_padre = def_padre.clone();
                            
                            // Buscar y ejecutar el constructor del padre
                            if let Some(constructor) = &def_padre.constructor {
                                if let NodoAst::DeclaracionFuncion { parametros, cuerpo, .. } = constructor {
                                    if parametros.len() != argumentos_evaluados.len() {
                                        return Err(Error::ejecucion(
                                            CodigoError::NumeroArgumentosIncorrecto,
                                            format!("constructor de '{}' espera {} argumentos, se recibieron {}", nombre_padre, parametros.len(), argumentos_evaluados.len()),
                                            None,
                                            Some(nodo.posicion().linea),
                                            Some(nodo.posicion().columna),
                                        ));
                                    }
                                    
                                    // Definir los parámetros en el ámbito actual (no crear nuevo ámbito)
                                    // para que las variables ambiente.X se compartan con el constructor hijo
                                    for (param, arg) in parametros.iter().zip(argumentos_evaluados.iter()) {
                                        // Usar una variable temporal para el parámetro
                                        let nombre_param = format!("__param_{}__", param.nombre);
                                        let _ = entorno.definir_variable(nombre_param.clone(), arg.clone(), param.mutable);
                                        // También definir con el nombre normal para acceso en el constructor
                                        let _ = entorno.asignar_variable(&param.nombre, arg.clone())
                                            .or_else(|_| entorno.definir_variable(param.nombre.clone(), arg.clone(), param.mutable));
                                    }
                                    
                                    // Ejecutar el cuerpo del constructor del padre
                                    let resultado = crate::interprete::declaraciones::evaluar_declaracion(&cuerpo, entorno)?;
                                    
                                    return Ok(resultado);
                                }
                            }
                            
                            return Err(Error::ejecucion(
                                CodigoError::MetodoNoEncontrado,
                                format!("constructor no encontrado para '{}'", nombre_padre),
                                None,
                                Some(nodo.posicion().linea),
                                Some(nodo.posicion().columna),
                            ));
                        } else {
                            return Err(Error::ejecucion(
                                CodigoError::ObjetoNoEncontrado,
                                format!("objeto padre '{}' no está definido", nombre_padre),
                                None,
                                Some(nodo.posicion().linea),
                                Some(nodo.posicion().columna),
                            ));
                        }
                    }
                }
                
                // Si es una llamada a método de objeto (objeto.metodo())
                if tipo == "metodo_objeto" {
                    let es_metodo = propiedades.get("__es_metodo_objeto__")
                        .map(|v| matches!(v, Valor::Logico(true)))
                        .unwrap_or(false);
                    
                    if es_metodo {
                        if let Some(funcion) = propiedades.get("__funcion__") {
                            if let Valor::Funcion { nombre, parametros, parametros_mutables, cuerpo, asincrono } = funcion {
                                if parametros.len() != argumentos_evaluados.len() {
                                    return Err(Error::ejecucion(
                                        CodigoError::NumeroArgumentosIncorrecto,
                                        format!("método '{}' espera {} argumentos, se recibieron {}", nombre, parametros.len(), argumentos_evaluados.len()),
                                        None,
                                        Some(nodo.posicion().linea),
                                        Some(nodo.posicion().columna),
                                    ));
                                }
                                
                                // Crear ámbito para el método
                                entorno.entrar_ambito();
                                
                                // Definir los parámetros
                                for (i, (param, arg)) in parametros.iter().zip(argumentos_evaluados.iter()).enumerate() {
                                    let es_mutable = parametros_mutables.get(i).copied().unwrap_or(false);
                                    entorno.definir_variable(param.clone(), arg.clone(), es_mutable)
                                        .map_err(|e| Error::ejecucion(
                                            CodigoError::ErrorInternoInterprete,
                                            e,
                                            None,
                                            Some(nodo.posicion().linea),
                                            Some(nodo.posicion().columna),
                                        ))?;
                                }
                                
                        // Configurar las variables ambiente.X con las propiedades del objeto
                        for (prop_nombre, prop_valor) in propiedades.iter() {
                            if let Some(nombre_con_sufijo) = prop_nombre.strip_prefix("__prop_") {
                                // Quitar también el sufijo "__"
                                let nombre_prop = nombre_con_sufijo.strip_suffix("__").unwrap_or(nombre_con_sufijo);
                                let _ = entorno.definir_variable(format!("ambiente.{}", nombre_prop), prop_valor.clone(), true);
                            }
                        }
                                
                                // Ejecutar el cuerpo del método
                                let resultado = if *asincrono {
                                    ejecutar_funcion_asincrona(
                                        nombre.clone(),
                                        parametros.clone(),
                                        parametros_mutables.clone(),
                                        cuerpo.clone(),
                                        argumentos_evaluados,
                                        entorno,
                                        nodo,
                                    )?
                                } else {
                                    crate::interprete::declaraciones::evaluar_declaracion(cuerpo, entorno)?
                                };
                                
                                entorno.salir_ambito();
                                
                                return Ok(resultado);
                            }
                        }
                    }
                }
                
                // Si es una llamada a método de padre (padre.Mamifero.metodo())
                if tipo == "metodo_padre" {
                    if let (Some(Valor::Texto(nombre_padre)), Some(Valor::Texto(nombre_metodo))) = 
                        (propiedades.get("nombre_padre"), propiedades.get("metodo")) {
                        // Obtener la definición del padre
                        if let Some(def_padre) = entorno.obtener_definicion_objeto(nombre_padre) {
                            let def_padre = def_padre.clone();
                            
                            // Buscar el método en el padre
                            for miembro in &def_padre.miembros {
                                if !miembro.libre {
                                    if let NodoAst::DeclaracionFuncion { nombre: fn_nombre, parametros, cuerpo, asincrono, .. } = miembro.declaracion.as_ref() {
                                        if fn_nombre == nombre_metodo {
                                            if parametros.len() != argumentos_evaluados.len() {
                                                return Err(Error::ejecucion(
                                                    CodigoError::NumeroArgumentosIncorrecto,
                                                    format!("método '{}' de '{}' espera {} argumentos, se recibieron {}", nombre_metodo, nombre_padre, parametros.len(), argumentos_evaluados.len()),
                                                    None,
                                                    Some(nodo.posicion().linea),
                                                    Some(nodo.posicion().columna),
                                                ));
                                            }
                                            
                                            // Crear ámbito para el método
                                            entorno.entrar_ambito();
                                            
                                            // Definir los parámetros
                                            for (param, arg) in parametros.iter().zip(argumentos_evaluados.iter()) {
                                                entorno.definir_variable(param.nombre.clone(), arg.clone(), param.mutable)
                                                    .map_err(|e| Error::ejecucion(
                                                        CodigoError::ErrorInternoInterprete,
                                                        e,
                                                        None,
                                                        Some(nodo.posicion().linea),
                                                        Some(nodo.posicion().columna),
                                                    ))?;
                                            }
                                            
                                            // Si es asíncrono, ejecutar de forma asíncrona
                                            if *asincrono {
                                                let resultado = ejecutar_funcion_asincrona(
                                                    fn_nombre.clone(),
                                                    parametros.iter().map(|p| p.nombre.clone()).collect(),
                                                    parametros.iter().map(|p| p.mutable).collect(),
                                                    cuerpo.clone(),
                                                    argumentos_evaluados,
                                                    entorno,
                                                    nodo,
                                                )?;
                                                entorno.salir_ambito();
                                                return Ok(resultado);
                                            }
                                            
                                            // Ejecutar el cuerpo del método
                                            let resultado = crate::interprete::declaraciones::evaluar_declaracion(&cuerpo, entorno)?;
                                            
                                            entorno.salir_ambito();
                                            
                                            return Ok(resultado);
                                        }
                                    }
                                }
                            }
                            
                            return Err(Error::ejecucion(
                                CodigoError::MetodoNoEncontrado,
                                format!("método '{}' no encontrado en '{}'", nombre_metodo, nombre_padre),
                                None,
                                Some(nodo.posicion().linea),
                                Some(nodo.posicion().columna),
                            ));
                        } else {
                            return Err(Error::ejecucion(
                                CodigoError::ObjetoNoEncontrado,
                                format!("objeto padre '{}' no está definido", nombre_padre),
                                None,
                                Some(nodo.posicion().linea),
                                Some(nodo.posicion().columna),
                            ));
                        }
                    }
                }
            }
            
            // Llamada normal a función
            match valor_funcion {
                Valor::Funcion { nombre, parametros, parametros_mutables, cuerpo, asincrono, .. } => {
                    // Verificar si es una función nativa global como rango
                    if nombre == "rango" {
                        return ejecutar_funcion_rango(argumentos_evaluados, nodo);
                    }
                    
                    if parametros.len() != argumentos_evaluados.len() {
                        return Err(Error::ejecucion(
                            CodigoError::NumeroArgumentosIncorrecto,
                            format!("se esperaban {} argumentos, se recibieron {}", parametros.len(), argumentos_evaluados.len()),
                            None,
                            Some(nodo.posicion().linea),
                            Some(nodo.posicion().columna),
                        ));
                    }
                    
                    // Si es asíncrona, ejecutarla de forma asíncrona
                    if asincrono {
                        return ejecutar_funcion_asincrona(nombre, parametros, parametros_mutables, cuerpo, argumentos_evaluados, entorno, nodo);
                    }
                    
                    // Crear nuevo ámbito para los parámetros
                    entorno.entrar_ambito();
                    for (i, (param, arg)) in parametros.iter().zip(argumentos_evaluados.iter()).enumerate() {
                        // Usar la mutabilidad definida en la declaración de la función
                        let es_mutable = parametros_mutables.get(i).copied().unwrap_or(false);
                        entorno.definir_variable(param.clone(), arg.clone(), es_mutable)
                            .map_err(|e| Error::ejecucion(
                                CodigoError::ErrorInternoInterprete,
                                e,
                                None,
                                Some(nodo.posicion().linea),
                                Some(nodo.posicion().columna),
                            ))?;
                    }
                    
                    let resultado = crate::interprete::declaraciones::evaluar_declaracion(&cuerpo, entorno)?;
                    entorno.salir_ambito();
                    
                    Ok(resultado)
                }
                _ => Err(Error::ejecucion(
                    CodigoError::FuncionNoDeclarada,
                    format!("'{:?}' no es una función", valor_funcion.tipo()),
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            }
        }
        
        NodoAst::ExpresionBinaria { operador, izquierda, derecha, .. } => {
            let valor_izq = evaluar_expresion(izquierda, entorno)?;
            let valor_der = evaluar_expresion(derecha, entorno)?;
            
            evaluar_operador_binario(operador, &valor_izq, &valor_der, nodo)
        }
        
        NodoAst::ExpresionUnaria { operador, expresion, .. } => {
            // Para incremento/decremento postfijo, necesitamos modificar la variable
            if matches!(operador, OperadorUnario::Incrementar | OperadorUnario::Decrementar) {
                // Verificar si la expresión es un identificador (variable)
                if let NodoAst::ExpresionIdentificador { nombre, .. } = expresion.as_ref() {
                    // Obtener el valor actual y clonarlo
                    let valor_actual = entorno.obtener_variable(nombre)
                        .ok_or_else(|| Error::ejecucion(
                            CodigoError::VariableNoDeclarada,
                            format!("variable '{}' no está definida", nombre),
                            None,
                            Some(nodo.posicion().linea),
                            Some(nodo.posicion().columna),
                        ))?
                        .clone();
                    
                    // Calcular el nuevo valor
                    let nuevo_valor = match operador {
                        OperadorUnario::Incrementar => {
                            match &valor_actual {
                                Valor::Entero(val) => Valor::Entero(val + 1),
                                Valor::Numero(val) => Valor::Numero(val + Decimal::ONE),
                                _ => return Err(Error::ejecucion(
                                    CodigoError::TiposIncompatibles,
                                    format!("no se puede incrementar tipo {}", valor_actual.tipo()),
                                    None,
                                    Some(nodo.posicion().linea),
                                    Some(nodo.posicion().columna),
                                )),
                            }
                        }
                        OperadorUnario::Decrementar => {
                            match &valor_actual {
                                Valor::Entero(val) => Valor::Entero(val - 1),
                                Valor::Numero(val) => Valor::Numero(val - Decimal::ONE),
                                _ => return Err(Error::ejecucion(
                                    CodigoError::TiposIncompatibles,
                                    format!("no se puede decrementar tipo {}", valor_actual.tipo()),
                                    None,
                                    Some(nodo.posicion().linea),
                                    Some(nodo.posicion().columna),
                                )),
                            }
                        }
                        _ => unreachable!(),
                    };
                    
                    // Actualizar la variable en el entorno
                    entorno.asignar_variable(nombre, nuevo_valor)
                        .map_err(|e| Error::ejecucion(
                            CodigoError::VariableNoDeclarada,
                            e,
                            None,
                            Some(nodo.posicion().linea),
                            Some(nodo.posicion().columna),
                        ))?;
                    
                    // Retornar el valor original (postfijo)
                    Ok(valor_actual)
                } else {
                    return Err(Error::ejecucion(
                        CodigoError::TiposIncompatibles,
                        "operador de incremento/decremento solo puede aplicarse a variables",
                        None,
                        Some(nodo.posicion().linea),
                        Some(nodo.posicion().columna),
                    ));
                }
            } else {
                let valor = evaluar_expresion(expresion, entorno)?;
                evaluar_operador_unario(operador, &valor, nodo)
            }
        }
        
        NodoAst::ExpresionTernario { condicion, verdadero, falso, .. } => {
            let valor_condicion = evaluar_expresion(condicion, entorno)?;
            if valor_condicion.es_verdadero() {
                evaluar_expresion(verdadero, entorno)
            } else {
                evaluar_expresion(falso, entorno)
            }
        }
        
        NodoAst::ExpresionLista { elementos, .. } => {
            let valores: Resultado<Vec<Valor>> = elementos.iter()
                .map(|elem| evaluar_expresion(elem, entorno))
                .collect();
            Ok(Valor::Lista(valores?))
        }
        
        NodoAst::ExpresionJson { propiedades, .. } => {
            use serde_json::Value as JsonValue;
            let mut json_obj = serde_json::Map::new();
            
            for prop in propiedades {
                let valor = evaluar_expresion(&prop.valor, entorno)?;
                // Convertir Valor a JsonValue
                let json_valor = match valor {
                    Valor::Vacio => JsonValue::Null,
                    Valor::Entero(v) => JsonValue::Number(v.into()),
                    Valor::Numero(v) => {
                        // Convertir Decimal a f64 y luego a JsonValue
                        let f64_val: f64 = v.to_string().parse().unwrap_or(0.0);
                        JsonValue::Number(serde_json::Number::from_f64(f64_val).unwrap_or(serde_json::Number::from(0)))
                    }
                    Valor::Texto(v) => JsonValue::String(v),
                    Valor::Logico(v) => JsonValue::Bool(v),
                    Valor::Lista(v) => {
                        let arr: Vec<JsonValue> = v.iter().map(|val| {
                            match val {
                                Valor::Vacio => JsonValue::Null,
                                Valor::Entero(n) => JsonValue::Number((*n).into()),
                                Valor::Texto(s) => JsonValue::String(s.clone()),
                                Valor::Logico(b) => JsonValue::Bool(*b),
                                Valor::Json(j) => j.clone(),
                                _ => JsonValue::String(val.a_texto()),
                            }
                        }).collect();
                        JsonValue::Array(arr)
                    }
                    Valor::Json(j) => j,
                    _ => JsonValue::String(valor.a_texto()),
                };
                json_obj.insert(prop.clave.clone(), json_valor);
            }
            
            Ok(Valor::Json(JsonValue::Object(json_obj)))
        }
        
        NodoAst::ExpresionAsignar { objetivo, valor, .. } => {
            let valor_asignar = evaluar_expresion(valor, entorno)?;
            
            // Evaluar el objetivo (puede ser un identificador o un acceso a miembro)
            match objetivo.as_ref() {
                NodoAst::ExpresionIdentificador { nombre, .. } => {
                    // Asignación a variable
                    entorno.asignar_variable(nombre, valor_asignar.clone())
                        .map_err(|e| Error::ejecucion(
                            CodigoError::VariableNoDeclarada,
                            e,
                            None,
                            Some(nodo.posicion().linea),
                            Some(nodo.posicion().columna),
                        ))?;
                    Ok(valor_asignar)
                }
                NodoAst::ExpresionAcceso { objeto, miembro, .. } => {
                    // Asignación a miembro de objeto (ej: persona.nombre = valor, ambiente.nombre = valor)
                    if let NodoAst::ExpresionIdentificador { nombre: nombre_objeto, .. } = objeto.as_ref() {
                        if nombre_objeto == "ambiente" {
                            // Asignar a variable con nombre "ambiente.{miembro}"
                            // Las variables ambiente.X son especiales y deben persistir a través de los ámbitos
                            let nombre_completo = format!("ambiente.{}", miembro);
                            
                            // Intentar asignar en cualquier ámbito donde exista, o crear en el ámbito base
                            if entorno.obtener_variable(&nombre_completo).is_some() {
                                entorno.asignar_variable(&nombre_completo, valor_asignar.clone())
                                    .map_err(|e| Error::ejecucion(
                                        CodigoError::ErrorInternoInterprete,
                                        format!("no se puede asignar a {}: {}", nombre_completo, e),
                                        None,
                                        Some(nodo.posicion().linea),
                                        Some(nodo.posicion().columna),
                                    ))?;
                            } else {
                                // Crear la variable en el ámbito base (para que persista)
                                entorno.definir_variable_en_base(nombre_completo.clone(), valor_asignar.clone(), true)
                                    .map_err(|e| Error::ejecucion(
                                        CodigoError::ErrorInternoInterprete,
                                        e,
                                        None,
                                        Some(nodo.posicion().linea),
                                        Some(nodo.posicion().columna),
                                    ))?;
                            }
                            Ok(valor_asignar)
                        } else {
                            // Intentar asignar a propiedad de JSON
                            if let Some((valor_actual, es_mutable)) = entorno.obtener_info_variable(nombre_objeto) {
                                if !es_mutable {
                                    return Err(Error::ejecucion(
                                        CodigoError::VariableNoDeclarada,
                                        format!("variable '{}' no es mutable", nombre_objeto),
                                        None,
                                        Some(nodo.posicion().linea),
                                        Some(nodo.posicion().columna),
                                    ));
                                }
                                let valor_clonado = valor_actual.clone();
                                if let Valor::Json(JsonValue::Object(mut obj)) = valor_clonado {
                                    // Convertir el valor a JsonValue
                                    let json_valor = match valor_asignar.clone() {
                                        Valor::Vacio => JsonValue::Null,
                                        Valor::Entero(v) => JsonValue::Number(v.into()),
                                        Valor::Numero(v) => {
                                            let f64_val: f64 = v.to_string().parse().unwrap_or(0.0);
                                            JsonValue::Number(serde_json::Number::from_f64(f64_val).unwrap_or(serde_json::Number::from(0)))
                                        }
                                        Valor::Texto(v) => JsonValue::String(v),
                                        Valor::Logico(v) => JsonValue::Bool(v),
                                        Valor::Json(j) => j,
                                        _ => JsonValue::String(valor_asignar.a_texto()),
                                    };
                                    obj.insert(miembro.clone(), json_valor);
                                    entorno.asignar_variable(nombre_objeto, Valor::Json(JsonValue::Object(obj)))
                                        .map_err(|e| Error::ejecucion(
                                            CodigoError::ErrorInternoInterprete,
                                            e,
                                            None,
                                            Some(nodo.posicion().linea),
                                            Some(nodo.posicion().columna),
                                        ))?;
                                    Ok(valor_asignar)
                                } else {
                                    Err(Error::ejecucion(
                                        CodigoError::TiposIncompatibles,
                                        format!("no se puede asignar a miembro de '{}' (no es un JSON)", nombre_objeto),
                                        None,
                                        Some(nodo.posicion().linea),
                                        Some(nodo.posicion().columna),
                                    ))
                                }
                            } else {
                                Err(Error::ejecucion(
                                    CodigoError::VariableNoDeclarada,
                                    format!("variable '{}' no está definida", nombre_objeto),
                                    None,
                                    Some(nodo.posicion().linea),
                                    Some(nodo.posicion().columna),
                                ))
                            }
                        }
                    } else {
                        Err(Error::ejecucion(
                            CodigoError::TiposIncompatibles,
                            "solo se puede asignar a identificadores o miembros de objetos",
                            None,
                            Some(nodo.posicion().linea),
                            Some(nodo.posicion().columna),
                        ))
                    }
                }
                _ => Err(Error::ejecucion(
                    CodigoError::TiposIncompatibles,
                    "solo se puede asignar a identificadores o miembros de objetos",
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            }
        }
        
        NodoAst::ExpresionIndice { objeto, indice, .. } => {
            let valor_objeto = evaluar_expresion(objeto, entorno)?;
            let valor_indice = evaluar_expresion(indice, entorno)?;
            
            match (valor_objeto, valor_indice) {
                (Valor::Lista(lista), Valor::Entero(idx)) => {
                    // Soporte para índices negativos (desde el final)
                    let indice_real = if idx < 0 {
                        let len = lista.len() as i64;
                        let indice_neg = idx + len;
                        if indice_neg < 0 || indice_neg >= len {
                            return Err(Error::ejecucion(
                                CodigoError::IndiceFueraDeRango,
                                format!("índice {} fuera de rango para lista de tamaño {}", idx, lista.len()),
                                None,
                                Some(nodo.posicion().linea),
                                Some(nodo.posicion().columna),
                            ));
                        }
                        indice_neg as usize
                    } else {
                        if idx as usize >= lista.len() {
                            return Err(Error::ejecucion(
                                CodigoError::IndiceFueraDeRango,
                                format!("índice {} fuera de rango para lista de tamaño {}", idx, lista.len()),
                                None,
                                Some(nodo.posicion().linea),
                                Some(nodo.posicion().columna),
                            ));
                        }
                        idx as usize
                    };
                    Ok(lista[indice_real].clone())
                }
                (Valor::Json(JsonValue::Object(obj)), Valor::Texto(clave)) => {
                    if let Some(valor) = obj.get(&clave) {
                        // Convertir JsonValue a Valor
                        match valor {
                            JsonValue::Null => Ok(Valor::Vacio),
                            JsonValue::Bool(b) => Ok(Valor::Logico(*b)),
                            JsonValue::Number(n) => {
                                if let Some(i) = n.as_i64() {
                                    Ok(Valor::Entero(i))
                                } else if let Some(f) = n.as_f64() {
                                    Ok(Valor::Numero(Decimal::from_f64_retain(f).unwrap_or(Decimal::ZERO)))
                                } else {
                                    Ok(Valor::Texto(n.to_string()))
                                }
                            }
                            JsonValue::String(s) => Ok(Valor::Texto(s.clone())),
                            JsonValue::Array(arr) => {
                                let valores: Vec<Valor> = arr.iter().map(|v| {
                                    match v {
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
                                        _ => Valor::Texto(v.to_string()),
                                    }
                                }).collect();
                                Ok(Valor::Lista(valores))
                            }
                            JsonValue::Object(_) => Ok(Valor::Json(valor.clone())),
                        }
                    } else {
                        Ok(Valor::Vacio)
                    }
                }
                (Valor::Texto(texto), Valor::Entero(idx)) => {
                    // Soporte para índices negativos en textos también
                    let chars: Vec<char> = texto.chars().collect();
                    let indice_real = if idx < 0 {
                        let len = chars.len() as i64;
                        let indice_neg = idx + len;
                        if indice_neg < 0 || indice_neg >= len {
                            return Err(Error::ejecucion(
                                CodigoError::IndiceFueraDeRango,
                                format!("índice {} fuera de rango para texto de tamaño {}", idx, chars.len()),
                                None,
                                Some(nodo.posicion().linea),
                                Some(nodo.posicion().columna),
                            ));
                        }
                        indice_neg as usize
                    } else {
                        if idx as usize >= chars.len() {
                            return Err(Error::ejecucion(
                                CodigoError::IndiceFueraDeRango,
                                format!("índice {} fuera de rango para texto de tamaño {}", idx, chars.len()),
                                None,
                                Some(nodo.posicion().linea),
                                Some(nodo.posicion().columna),
                            ));
                        }
                        idx as usize
                    };
                    Ok(Valor::Texto(chars[indice_real].to_string()))
                }
                _ => Err(Error::ejecucion(
                    CodigoError::TiposIncompatibles,
                    "acceso por índice solo está disponible para listas, textos y objetos JSON",
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            }
        }
        
        NodoAst::ExpresionEsperar { expresion, .. } => {
            // Evaluar la expresión (puede ser una llamada a función asíncrona)
            // Si es una llamada a función, ya se ejecutó, solo retornamos el resultado
            // Si es una función sin llamar, la ejecutamos
            let valor = evaluar_expresion(expresion, entorno)?;
            
            // Si el valor es una función asíncrona sin argumentos, ejecutarla
            if let Valor::Funcion { nombre: _, parametros, parametros_mutables: _, cuerpo, asincrono, .. } = &valor {
                if *asincrono && parametros.is_empty() {
                    // Ejecutar función asíncrona sin argumentos
                    entorno.entrar_ambito();
                    let resultado = crate::interprete::declaraciones::evaluar_declaracion(&cuerpo, entorno)?;
                    entorno.salir_ambito();
                    return Ok(resultado);
                }
            }
            
            // Si ya es un valor (resultado de una llamada), simplemente retornarlo
            Ok(valor)
        }
        
        // Expresión nuevo: crear instancia de objeto
        NodoAst::ExpresionNuevo { tipo, argumentos, .. } => {
            // Evaluar los argumentos del constructor
            let argumentos_evaluados: Resultado<Vec<Valor>> = argumentos.iter()
                .map(|arg| evaluar_expresion(arg, entorno))
                .collect();
            let argumentos_evaluados = argumentos_evaluados?;
            
            // Crear un objeto con las propiedades iniciales
            let mut propiedades = HashMap::new();
            propiedades.insert("__tipo__".to_string(), Valor::Texto(tipo.clone()));
            
            // Obtener la definición del objeto
            if let Some(definicion) = entorno.obtener_definicion_objeto(tipo) {
                let definicion = definicion.clone();
                
                // Almacenar lista de padres
                propiedades.insert("__padres__".to_string(), Valor::Lista(
                    definicion.padres.iter().map(|p| Valor::Texto(p.clone())).collect()
                ));
                
                // Función auxiliar para recopilar métodos heredados recursivamente
                fn recopilar_metodos_heredados(
                    nombre_objeto: &str,
                    entorno: &crate::interprete::entorno::Entorno,
                    metodos: &mut HashMap<String, Valor>,
                    propiedades: &mut HashMap<String, Valor>,
                    visitados: &mut std::collections::HashSet<String>,
                ) -> Resultado<()> {
                    // Evitar ciclos infinitos
                    if visitados.contains(nombre_objeto) {
                        return Ok(());
                    }
                    visitados.insert(nombre_objeto.to_string());
                    
                    if let Some(def) = entorno.obtener_definicion_objeto(nombre_objeto) {
                        let def = def.clone();
                        
                        // Primero, heredar de los padres (recursivamente)
                        for nombre_padre in &def.padres {
                            recopilar_metodos_heredados(nombre_padre, entorno, metodos, propiedades, visitados)?;
                            // Almacenar referencia al padre
                            propiedades.insert(format!("__padre_{}__", nombre_padre), Valor::Texto(nombre_padre.clone()));
                        }
                        
                        // Luego, agregar métodos y atributos del objeto actual (sobrescriben los heredados)
                        for miembro in &def.miembros {
                            if !miembro.libre {
                                match miembro.declaracion.as_ref() {
                                    NodoAst::DeclaracionFuncion { nombre: nombre_fn, parametros, cuerpo, asincrono, .. } => {
                                        // No incluir el constructor
                                        if nombre_fn != nombre_objeto {
                                            let param_nombres: Vec<String> = parametros.iter().map(|p| p.nombre.clone()).collect();
                                            let param_mutables: Vec<bool> = parametros.iter().map(|p| p.mutable).collect();
                                            let funcion = Valor::Funcion {
                                                nombre: nombre_fn.clone(),
                                                parametros: param_nombres,
                                                parametros_mutables: param_mutables,
                                                cuerpo: cuerpo.clone(),
                                                asincrono: *asincrono,
                                            };
                                            metodos.insert(nombre_fn.clone(), funcion);
                                        }
                                    }
                                    NodoAst::DeclaracionVariable { .. } => {
                                        // Los atributos se inicializan en el constructor
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    Ok(())
                }
                
                // Recopilar métodos y atributos de los padres (herencia en cascada)
                let mut metodos_heredados: HashMap<String, Valor> = HashMap::new();
                let mut visitados = std::collections::HashSet::new();
                for nombre_padre in &definicion.padres {
                    recopilar_metodos_heredados(nombre_padre, entorno, &mut metodos_heredados, &mut propiedades, &mut visitados)?;
                    propiedades.insert(format!("__padre_{}__", nombre_padre), Valor::Texto(nombre_padre.clone()));
                }
                
                // Agregar métodos heredados a propiedades
                for (nombre, valor) in metodos_heredados {
                    propiedades.insert(nombre, valor);
                }
                
                // Agregar métodos y atributos propios del objeto (pueden sobrescribir heredados)
                for miembro in &definicion.miembros {
                    if !miembro.libre {
                        match miembro.declaracion.as_ref() {
                            NodoAst::DeclaracionFuncion { nombre: nombre_fn, parametros, cuerpo, asincrono, .. } => {
                                // Saltar el constructor (se ejecutará después)
                                if nombre_fn != tipo {
                                    let param_nombres: Vec<String> = parametros.iter().map(|p| p.nombre.clone()).collect();
                                    let param_mutables: Vec<bool> = parametros.iter().map(|p| p.mutable).collect();
                                    let funcion = Valor::Funcion {
                                        nombre: nombre_fn.clone(),
                                        parametros: param_nombres,
                                        parametros_mutables: param_mutables,
                                        cuerpo: cuerpo.clone(),
                                        asincrono: *asincrono,
                                    };
                                    propiedades.insert(nombre_fn.clone(), funcion);
                                }
                            }
                            NodoAst::DeclaracionVariable { nombre: nombre_var, valor, mutable, .. } => {
                                let valor_evaluado = evaluar_expresion(valor, entorno)?;
                                propiedades.insert(nombre_var.clone(), valor_evaluado);
                                if *mutable {
                                    propiedades.insert(format!("__mutable_{}__", nombre_var), Valor::Logico(true));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                
                // Crear el objeto
                let objeto = Valor::Objeto {
                    tipo: tipo.clone(),
                    propiedades: propiedades.clone(),
                };
                
                // Ejecutar el constructor si existe
                if let Some(constructor) = &definicion.constructor {
                    if let NodoAst::DeclaracionFuncion { parametros, cuerpo, .. } = constructor {
                        if parametros.len() != argumentos_evaluados.len() {
                            return Err(Error::ejecucion(
                                CodigoError::NumeroArgumentosIncorrecto,
                                format!("constructor de '{}' espera {} argumentos, se recibieron {}", tipo, parametros.len(), argumentos_evaluados.len()),
                                None,
                                Some(nodo.posicion().linea),
                                Some(nodo.posicion().columna),
                            ));
                        }
                        
                        // Crear ámbito para el constructor
                        entorno.entrar_ambito();
                        
                        // Definir los parámetros
                        for (param, arg) in parametros.iter().zip(argumentos_evaluados.iter()) {
                            entorno.definir_variable(param.nombre.clone(), arg.clone(), param.mutable)
                                .map_err(|e| Error::ejecucion(
                                    CodigoError::ErrorInternoInterprete,
                                    e,
                                    None,
                                    Some(nodo.posicion().linea),
                                    Some(nodo.posicion().columna),
                                ))?;
                        }
                        
                        // Definir las variables de ambiente.X como variables accesibles
                        for (nombre_prop, valor_prop) in &propiedades {
                            if !nombre_prop.starts_with("__") {
                                let nombre_amb = format!("ambiente.{}", nombre_prop);
                                let _ = entorno.definir_variable(nombre_amb, valor_prop.clone(), true);
                            }
                        }
                        
                        // Almacenar información del objeto actual para acceso a padre
                        entorno.definir_variable("__objeto_actual__".to_string(), objeto.clone(), false)
                            .map_err(|e| Error::ejecucion(
                                CodigoError::ErrorInternoInterprete,
                                e,
                                None,
                                Some(nodo.posicion().linea),
                                Some(nodo.posicion().columna),
                            ))?;
                        
                        // Ejecutar el cuerpo del constructor
                        crate::interprete::declaraciones::evaluar_declaracion(&cuerpo, entorno)?;
                        
                        // Recopilar TODAS las variables ambiente.X (incluyendo las nuevas creadas en el constructor)
                        let mut propiedades_actualizadas = propiedades.clone();
                        
                        // Obtener todas las variables del ámbito actual que comienzan con "ambiente."
                        let variables_ambiente = entorno.obtener_variables_con_prefijo("ambiente.");
                        for (nombre_completo, valor) in variables_ambiente {
                            // Extraer el nombre de la propiedad sin el prefijo "ambiente."
                            if let Some(nombre_prop) = nombre_completo.strip_prefix("ambiente.") {
                                propiedades_actualizadas.insert(nombre_prop.to_string(), valor);
                            }
                        }
                        
                        entorno.salir_ambito();
                        
                        return Ok(Valor::Objeto {
                            tipo: tipo.clone(),
                            propiedades: propiedades_actualizadas,
                        });
                    }
                }
                
                Ok(objeto)
            } else {
                // Si no hay definición, crear un objeto simple
                for (i, arg) in argumentos_evaluados.iter().enumerate() {
                    propiedades.insert(format!("__arg{}__", i), arg.clone());
                }
                
                Ok(Valor::Objeto {
                    tipo: tipo.clone(),
                    propiedades,
                })
            }
        }
        
        _ => Err(Error::ejecucion(
            CodigoError::OperacionNoSoportada,
            "expresión no implementada aún",
            None,
            Some(nodo.posicion().linea),
            Some(nodo.posicion().columna),
        )),
    }
}

fn evaluar_operador_binario(
    operador: &OperadorBinario,
    izquierda: &Valor,
    derecha: &Valor,
    nodo: &NodoAst,
) -> Resultado<Valor> {
    match operador {
        OperadorBinario::Suma => {
            match (izquierda, derecha) {
                (Valor::Entero(a), Valor::Entero(b)) => Ok(Valor::Entero(a + b)),
                (Valor::Entero(a), Valor::Numero(b)) => Ok(Valor::Numero(Decimal::from(*a) + b)),
                (Valor::Numero(a), Valor::Entero(b)) => Ok(Valor::Numero(a + Decimal::from(*b))),
                (Valor::Numero(a), Valor::Numero(b)) => Ok(Valor::Numero(a + b)),
                (Valor::Texto(a), Valor::Texto(b)) => Ok(Valor::Texto(format!("{}{}", a, b))),
                _ => Err(Error::ejecucion(
                    CodigoError::TiposIncompatibles,
                    format!("no se puede sumar {} y {}", izquierda.tipo(), derecha.tipo()),
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            }
        }
        OperadorBinario::Resta => {
            match (izquierda, derecha) {
                (Valor::Entero(a), Valor::Entero(b)) => Ok(Valor::Entero(a - b)),
                (Valor::Entero(a), Valor::Numero(b)) => Ok(Valor::Numero(Decimal::from(*a) - b)),
                (Valor::Numero(a), Valor::Entero(b)) => Ok(Valor::Numero(a - Decimal::from(*b))),
                (Valor::Numero(a), Valor::Numero(b)) => Ok(Valor::Numero(a - b)),
                _ => Err(Error::ejecucion(
                    CodigoError::TiposIncompatibles,
                    format!("no se puede restar {} y {}", izquierda.tipo(), derecha.tipo()),
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            }
        }
        OperadorBinario::Multiplicacion => {
            match (izquierda, derecha) {
                (Valor::Entero(a), Valor::Entero(b)) => Ok(Valor::Entero(a * b)),
                (Valor::Entero(a), Valor::Numero(b)) => Ok(Valor::Numero(Decimal::from(*a) * b)),
                (Valor::Numero(a), Valor::Entero(b)) => Ok(Valor::Numero(a * Decimal::from(*b))),
                (Valor::Numero(a), Valor::Numero(b)) => Ok(Valor::Numero(a * b)),
                _ => Err(Error::ejecucion(
                    CodigoError::TiposIncompatibles,
                    format!("no se puede multiplicar {} y {}", izquierda.tipo(), derecha.tipo()),
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            }
        }
        OperadorBinario::Division => {
            match (izquierda, derecha) {
                (Valor::Entero(a), Valor::Entero(b)) => {
                    if *b == 0 {
                        return Err(Error::ejecucion(
                            CodigoError::DivisionPorCero,
                            "división por cero",
                            None,
                            Some(nodo.posicion().linea),
                            Some(nodo.posicion().columna),
                        ));
                    }
                    Ok(Valor::Entero(a / b))
                }
                (Valor::Entero(a), Valor::Numero(b)) => {
                    if *b == Decimal::ZERO {
                        return Err(Error::ejecucion(
                            CodigoError::DivisionPorCero,
                            "división por cero",
                            None,
                            Some(nodo.posicion().linea),
                            Some(nodo.posicion().columna),
                        ));
                    }
                    Ok(Valor::Numero(Decimal::from(*a) / b))
                }
                (Valor::Numero(a), Valor::Entero(b)) => {
                    if *b == 0 {
                        return Err(Error::ejecucion(
                            CodigoError::DivisionPorCero,
                            "división por cero",
                            None,
                            Some(nodo.posicion().linea),
                            Some(nodo.posicion().columna),
                        ));
                    }
                    Ok(Valor::Numero(a / Decimal::from(*b)))
                }
                (Valor::Numero(a), Valor::Numero(b)) => {
                    if *b == Decimal::ZERO {
                        return Err(Error::ejecucion(
                            CodigoError::DivisionPorCero,
                            "división por cero",
                            None,
                            Some(nodo.posicion().linea),
                            Some(nodo.posicion().columna),
                        ));
                    }
                    Ok(Valor::Numero(a / b))
                }
                _ => Err(Error::ejecucion(
                    CodigoError::TiposIncompatibles,
                    format!("no se puede dividir {} y {}", izquierda.tipo(), derecha.tipo()),
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            }
        }
        OperadorBinario::Modulo => {
            match (izquierda, derecha) {
                (Valor::Entero(a), Valor::Entero(b)) => {
                    if *b == 0 {
                        return Err(Error::ejecucion(
                            CodigoError::DivisionPorCero,
                            "módulo por cero",
                            None,
                            Some(nodo.posicion().linea),
                            Some(nodo.posicion().columna),
                        ));
                    }
                    Ok(Valor::Entero(a % b))
                }
                (Valor::Entero(a), Valor::Numero(b)) => {
                    if *b == Decimal::ZERO {
                        return Err(Error::ejecucion(
                            CodigoError::DivisionPorCero,
                            "módulo por cero",
                            None,
                            Some(nodo.posicion().linea),
                            Some(nodo.posicion().columna),
                        ));
                    }
                    Ok(Valor::Numero(Decimal::from(*a) % b))
                }
                (Valor::Numero(a), Valor::Entero(b)) => {
                    if *b == 0 {
                        return Err(Error::ejecucion(
                            CodigoError::DivisionPorCero,
                            "módulo por cero",
                            None,
                            Some(nodo.posicion().linea),
                            Some(nodo.posicion().columna),
                        ));
                    }
                    Ok(Valor::Numero(a % Decimal::from(*b)))
                }
                (Valor::Numero(a), Valor::Numero(b)) => {
                    if *b == Decimal::ZERO {
                        return Err(Error::ejecucion(
                            CodigoError::DivisionPorCero,
                            "módulo por cero",
                            None,
                            Some(nodo.posicion().linea),
                            Some(nodo.posicion().columna),
                        ));
                    }
                    Ok(Valor::Numero(a % b))
                }
                _ => Err(Error::ejecucion(
                    CodigoError::TiposIncompatibles,
                    format!("no se puede calcular módulo de {} y {}", izquierda.tipo(), derecha.tipo()),
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            }
        }
        OperadorBinario::Igual => Ok(Valor::Logico(izquierda.es_igual(derecha))),
        OperadorBinario::Diferente => Ok(Valor::Logico(!izquierda.es_igual(derecha))),
        OperadorBinario::Menor => {
            match (izquierda, derecha) {
                (Valor::Entero(a), Valor::Entero(b)) => Ok(Valor::Logico(a < b)),
                (Valor::Entero(a), Valor::Numero(b)) => Ok(Valor::Logico(Decimal::from(*a) < *b)),
                (Valor::Numero(a), Valor::Entero(b)) => Ok(Valor::Logico(*a < Decimal::from(*b))),
                (Valor::Numero(a), Valor::Numero(b)) => Ok(Valor::Logico(a < b)),
                _ => Err(Error::ejecucion(
                    CodigoError::TiposIncompatibles,
                    format!("no se puede comparar {} y {} con '<'", izquierda.tipo(), derecha.tipo()),
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            }
        }
        OperadorBinario::Mayor => {
            match (izquierda, derecha) {
                (Valor::Entero(a), Valor::Entero(b)) => Ok(Valor::Logico(a > b)),
                (Valor::Entero(a), Valor::Numero(b)) => Ok(Valor::Logico(Decimal::from(*a) > *b)),
                (Valor::Numero(a), Valor::Entero(b)) => Ok(Valor::Logico(*a > Decimal::from(*b))),
                (Valor::Numero(a), Valor::Numero(b)) => Ok(Valor::Logico(a > b)),
                _ => Err(Error::ejecucion(
                    CodigoError::TiposIncompatibles,
                    format!("no se puede comparar {} y {} con '>'", izquierda.tipo(), derecha.tipo()),
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            }
        }
        OperadorBinario::MenorIgual => {
            match (izquierda, derecha) {
                (Valor::Entero(a), Valor::Entero(b)) => Ok(Valor::Logico(a <= b)),
                (Valor::Entero(a), Valor::Numero(b)) => Ok(Valor::Logico(Decimal::from(*a) <= *b)),
                (Valor::Numero(a), Valor::Entero(b)) => Ok(Valor::Logico(*a <= Decimal::from(*b))),
                (Valor::Numero(a), Valor::Numero(b)) => Ok(Valor::Logico(a <= b)),
                _ => Err(Error::ejecucion(
                    CodigoError::TiposIncompatibles,
                    format!("no se puede comparar {} y {} con '<='", izquierda.tipo(), derecha.tipo()),
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            }
        }
        OperadorBinario::MayorIgual => {
            match (izquierda, derecha) {
                (Valor::Entero(a), Valor::Entero(b)) => Ok(Valor::Logico(a >= b)),
                (Valor::Entero(a), Valor::Numero(b)) => Ok(Valor::Logico(Decimal::from(*a) >= *b)),
                (Valor::Numero(a), Valor::Entero(b)) => Ok(Valor::Logico(*a >= Decimal::from(*b))),
                (Valor::Numero(a), Valor::Numero(b)) => Ok(Valor::Logico(a >= b)),
                _ => Err(Error::ejecucion(
                    CodigoError::TiposIncompatibles,
                    format!("no se puede comparar {} y {} con '>='", izquierda.tipo(), derecha.tipo()),
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            }
        }
        OperadorBinario::Y => {
            match (izquierda, derecha) {
                (Valor::Logico(a), Valor::Logico(b)) => Ok(Valor::Logico(*a && *b)),
                _ => Err(Error::ejecucion(
                    CodigoError::TiposIncompatibles,
                    "operador 'y' requiere operandos booleanos",
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            }
        }
        OperadorBinario::O => {
            match (izquierda, derecha) {
                (Valor::Logico(a), Valor::Logico(b)) => Ok(Valor::Logico(*a || *b)),
                _ => Err(Error::ejecucion(
                    CodigoError::TiposIncompatibles,
                    "operador 'o' requiere operandos booleanos",
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            }
        }
        _ => Err(Error::ejecucion(
            CodigoError::OperacionNoSoportada,
            format!("operador {:?} no implementado aún", operador),
            None,
            Some(nodo.posicion().linea),
            Some(nodo.posicion().columna),
        )),
    }
}

/// Aplica un método a un valor primitivo
fn aplicar_metodo_primitivo(
    metodo: &str,
    valor: &Valor,
    argumentos: Vec<Valor>,
    nodo: &NodoAst,
    entorno: &mut Entorno,
    nombre_variable: Option<String>,
    es_mutable: bool,
) -> Resultado<Valor> {
    // Si el valor es un texto, usar los métodos de texto
    if let Valor::Texto(texto) = valor {
        match aplicar_metodo_texto(metodo, texto, argumentos) {
            Ok(valor) => Ok(valor),
            Err(mensaje) => Err(Error::ejecucion(
                CodigoError::MetodoNoEncontrado,
                mensaje,
                None,
                Some(nodo.posicion().linea),
                Some(nodo.posicion().columna),
            )),
        }
    } else if let Valor::Lista(lista) = valor {
        // Si el valor es una lista, usar los métodos de lista
        let mut lista_mut = lista.clone();
        match aplicar_metodo_lista(metodo, &mut lista_mut, argumentos, es_mutable) {
            Ok((resultado, necesita_actualizar)) => {
                // Si el método modificó la lista y tenemos el nombre de la variable, actualizarla
                if necesita_actualizar {
                    if let Some(nombre_var) = nombre_variable {
                        entorno.asignar_variable(&nombre_var, Valor::Lista(lista_mut))
                            .map_err(|e| Error::ejecucion(
                                CodigoError::VariableNoDeclarada,
                                e,
                                None,
                                Some(nodo.posicion().linea),
                                Some(nodo.posicion().columna),
                            ))?;
                    }
                }
                Ok(resultado)
            }
            Err(mensaje) => Err(Error::ejecucion(
                CodigoError::MetodoNoEncontrado,
                mensaje,
                None,
                Some(nodo.posicion().linea),
                Some(nodo.posicion().columna),
            )),
        }
    } else if let Valor::Json(json) = valor {
        // Si el valor es un JSON, usar los métodos de JSON
        let mut json_mut = json.clone();
        match aplicar_metodo_json(metodo, &mut json_mut, argumentos, es_mutable) {
            Ok((resultado, necesita_actualizar)) => {
                // Si el método modificó el JSON y tenemos el nombre de la variable, actualizarla
                if necesita_actualizar {
                    if let Some(nombre_var) = nombre_variable {
                        entorno.asignar_variable(&nombre_var, Valor::Json(json_mut))
                            .map_err(|e| Error::ejecucion(
                                CodigoError::VariableNoDeclarada,
                                e,
                                None,
                                Some(nodo.posicion().linea),
                                Some(nodo.posicion().columna),
                            ))?;
                    }
                }
                Ok(resultado)
            }
            Err(mensaje) => Err(Error::ejecucion(
                CodigoError::MetodoNoEncontrado,
                mensaje,
                None,
                Some(nodo.posicion().linea),
                Some(nodo.posicion().columna),
            )),
        }
    } else if !argumentos.is_empty() {
        return Err(Error::ejecucion(
            CodigoError::NumeroArgumentosIncorrecto,
            format!("método '{}()' no acepta argumentos", metodo),
            None,
            Some(nodo.posicion().linea),
            Some(nodo.posicion().columna),
        ));
    } else {
        // Métodos para otros tipos primitivos (sin argumentos)
        match metodo {
        "texto" => Ok(Valor::Texto(valor.a_texto())),
        "entero" => {
            match valor {
                Valor::Texto(s) => {
                    s.parse::<i64>()
                        .map(Valor::Entero)
                        .map_err(|_| Error::ejecucion(
                            CodigoError::TiposIncompatibles,
                            format!("no se puede convertir '{}' a entero", s),
                            None,
                            Some(nodo.posicion().linea),
                            Some(nodo.posicion().columna),
                        ))
                }
                Valor::Entero(i) => Ok(Valor::Entero(*i)),
                Valor::Numero(n) => Ok(Valor::Entero(n.to_string().parse::<i64>().unwrap_or(0))),
                _ => Err(Error::ejecucion(
                    CodigoError::TiposIncompatibles,
                    format!("no se puede convertir {} a entero", valor.tipo()),
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            }
        }
        "numero" | "número" => {
            match valor {
                Valor::Texto(s) => {
                    s.parse::<Decimal>()
                        .map(Valor::Numero)
                        .map_err(|_| Error::ejecucion(
                            CodigoError::TiposIncompatibles,
                            format!("no se puede convertir '{}' a número", s),
                            None,
                            Some(nodo.posicion().linea),
                            Some(nodo.posicion().columna),
                        ))
                }
                Valor::Entero(i) => Ok(Valor::Numero(Decimal::from(*i))),
                Valor::Numero(n) => Ok(Valor::Numero(*n)),
                _ => Err(Error::ejecucion(
                    CodigoError::TiposIncompatibles,
                    format!("no se puede convertir {} a número", valor.tipo()),
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            }
        }
        "log" | "lóg" => {
            match valor {
                Valor::Texto(s) => {
                    let s_lower = s.to_lowercase();
                    Ok(Valor::Logico(s_lower == "verdadero" || s_lower == "true" || s_lower == "1"))
                }
                Valor::Logico(b) => Ok(Valor::Logico(*b)),
                Valor::Entero(i) => Ok(Valor::Logico(*i != 0)),
                Valor::Numero(n) => Ok(Valor::Logico(*n != Decimal::ZERO)),
                _ => Err(Error::ejecucion(
                    CodigoError::TiposIncompatibles,
                    format!("no se puede convertir {} a lógico", valor.tipo()),
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            }
        }
        "jsn" => {
            match valor {
                Valor::Texto(s) => {
                    serde_json::from_str(s)
                        .map(Valor::Json)
                        .map_err(|e| Error::ejecucion(
                            CodigoError::TiposIncompatibles,
                            format!("no se puede convertir texto a JSON: {}", e),
                            None,
                            Some(nodo.posicion().linea),
                            Some(nodo.posicion().columna),
                        ))
                }
                Valor::Json(j) => Ok(Valor::Json(j.clone())),
                _ => Err(Error::ejecucion(
                    CodigoError::TiposIncompatibles,
                    format!("no se puede convertir {} a JSON", valor.tipo()),
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            }
        }
        "lista" => {
            match valor {
                Valor::Texto(s) => {
                    // Parsear lista separada por comas
                    let elementos: Vec<Valor> = s.split(',')
                        .map(|elem| Valor::Texto(elem.trim().to_string()))
                        .collect();
                    Ok(Valor::Lista(elementos))
                }
                Valor::Lista(l) => Ok(Valor::Lista(l.clone())),
                _ => Err(Error::ejecucion(
                    CodigoError::TiposIncompatibles,
                    format!("no se puede convertir {} a lista", valor.tipo()),
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            }
        }
        _ => Err(Error::ejecucion(
            CodigoError::FuncionNoDeclarada,
            format!("método '{}' no está disponible para tipo {}", metodo, valor.tipo()),
            None,
            Some(nodo.posicion().linea),
            Some(nodo.posicion().columna),
        )),
        }
    }
}

fn evaluar_operador_unario(
    operador: &OperadorUnario,
    valor: &Valor,
    nodo: &NodoAst,
) -> Resultado<Valor> {
    match operador {
        OperadorUnario::Negacion => {
            match valor {
                Valor::Logico(val) => Ok(Valor::Logico(!val)),
                _ => Err(Error::ejecucion(
                    CodigoError::TiposIncompatibles,
                    "operador '!' requiere operando booleano",
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            }
        }
        OperadorUnario::Negativo => {
            match valor {
                Valor::Entero(val) => Ok(Valor::Entero(-val)),
                Valor::Numero(val) => Ok(Valor::Numero(-val)),
                _ => Err(Error::ejecucion(
                    CodigoError::TiposIncompatibles,
                    "operador '-' requiere operando numérico",
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            }
        }
        OperadorUnario::Positivo => Ok(valor.clone()),
        _ => Err(Error::ejecucion(
            CodigoError::OperacionNoSoportada,
            format!("operador {:?} no implementado aún", operador),
            None,
            Some(nodo.posicion().linea),
            Some(nodo.posicion().columna),
        )),
    }
}

/// Procesa una interpolación de texto, evaluando las expresiones dentro de { }
fn procesar_interpolacion_texto(
    texto: &str,
    entorno: &mut Entorno,
    nodo: &NodoAst,
) -> Resultado<Valor> {
    let mut resultado = String::new();
    let mut i = 0;
    let chars: Vec<char> = texto.chars().collect();
    
    while i < chars.len() {
        if chars[i] == '{' {
            // Buscar el cierre de la expresión
            let mut profundidad = 1;
            let inicio_expr = i + 1;
            let mut fin_expr = i + 1;
            
            i += 1;
            while i < chars.len() && profundidad > 0 {
                if chars[i] == '{' {
                    profundidad += 1;
                } else if chars[i] == '}' {
                    profundidad -= 1;
                    if profundidad == 0 {
                        fin_expr = i;
                        break;
                    }
                }
                i += 1;
            }
            
            if profundidad == 0 {
                // Extraer y evaluar la expresión
                let expr_str: String = chars[inicio_expr..fin_expr].iter().collect();
                let expr_str = expr_str.trim();
                
                // Parsear la expresión
                match crate::nucleo::sintactico::Parser::parsear(expr_str) {
                    Ok(ast) => {
                        if let Some(expr_nodo) = ast.first() {
                            match evaluar_expresion(expr_nodo, entorno) {
                                Ok(valor) => {
                                    resultado.push_str(&valor.a_texto());
                                }
                                Err(e) => {
                                    return Err(Error::ejecucion(
                                        CodigoError::ErrorInternoInterprete,
                                        format!("error al evaluar expresión en interpolación: {}", e.mensaje()),
                                        None,
                                        Some(nodo.posicion().linea),
                                        Some(nodo.posicion().columna),
                                    ));
                                }
                            }
                        } else {
                            resultado.push_str(&format!("{{{}}}", expr_str));
                        }
                    }
                    Err(_) => {
                        // Si no se puede parsear, tratarlo como texto literal
                        resultado.push_str(&format!("{{{}}}", expr_str));
                    }
                }
            } else {
                // Llave sin cerrar, tratarla como texto literal
                resultado.push('{');
            }
            i += 1;
        } else if chars[i] == '\\' && i + 1 < chars.len() {
            // Procesar escapes
            i += 1;
            match chars[i] {
                'n' => resultado.push('\n'),
                't' => resultado.push('\t'),
                'r' => resultado.push('\r'),
                '\\' => resultado.push('\\'),
                '{' => resultado.push('{'),
                '}' => resultado.push('}'),
                c => {
                    resultado.push('\\');
                    resultado.push(c);
                }
            }
            i += 1;
        } else {
            resultado.push(chars[i]);
            i += 1;
        }
    }
    
    Ok(Valor::Texto(resultado))
}

/// Ejecuta la función nativa rango(inicio, fin)
fn ejecutar_funcion_rango(
    argumentos: Vec<Valor>,
    nodo: &NodoAst,
) -> Resultado<Valor> {
    if argumentos.len() != 2 {
        return Err(Error::ejecucion(
            CodigoError::NumeroArgumentosIncorrecto,
            format!("función 'rango()' requiere 2 argumentos, se recibieron {}", argumentos.len()),
            None,
            Some(nodo.posicion().linea),
            Some(nodo.posicion().columna),
        ));
    }
    
    let inicio = match &argumentos[0] {
        Valor::Entero(n) => *n,
        _ => {
            return Err(Error::ejecucion(
                CodigoError::TiposIncompatibles,
                "función 'rango()' requiere argumentos de tipo entero",
                None,
                Some(nodo.posicion().linea),
                Some(nodo.posicion().columna),
            ));
        }
    };
    
    let fin = match &argumentos[1] {
        Valor::Entero(n) => *n,
        _ => {
            return Err(Error::ejecucion(
                CodigoError::TiposIncompatibles,
                "función 'rango()' requiere argumentos de tipo entero",
                None,
                Some(nodo.posicion().linea),
                Some(nodo.posicion().columna),
            ));
        }
    };
    
    let mut resultado = Vec::new();
    if inicio <= fin {
        for i in inicio..=fin {
            resultado.push(Valor::Entero(i));
        }
    } else {
        for i in (fin..=inicio).rev() {
            resultado.push(Valor::Entero(i));
        }
    }
    
    Ok(Valor::Lista(resultado))
}

/// Ejecuta una función asíncrona
fn ejecutar_funcion_asincrona(
    _nombre: String,
    parametros: Vec<String>,
    parametros_mutables: Vec<bool>,
    cuerpo: Box<NodoAst>,
    argumentos: Vec<Valor>,
    entorno: &mut Entorno,
    nodo: &NodoAst,
) -> Resultado<Valor> {
    // Por ahora, ejecutamos funciones asíncronas de forma síncrona
    // En el futuro se puede mejorar con tokio para verdadera concurrencia
    // Crear nuevo ámbito para los parámetros
    entorno.entrar_ambito();
    for (i, (param, arg)) in parametros.iter().zip(argumentos.iter()).enumerate() {
        let es_mutable = parametros_mutables.get(i).copied().unwrap_or(false);
        entorno.definir_variable(param.clone(), arg.clone(), es_mutable)
            .map_err(|e| Error::ejecucion(
                CodigoError::ErrorInternoInterprete,
                e,
                None,
                Some(nodo.posicion().linea),
                Some(nodo.posicion().columna),
            ))?;
    }
    
    let resultado = crate::interprete::declaraciones::evaluar_declaracion(&cuerpo, entorno)?;
    entorno.salir_ambito();
    
    Ok(resultado)
}

/// Convierte un JsonValue a un Valor del intérprete
fn convertir_json_a_valor(json: &JsonValue) -> Valor {
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
            let valores: Vec<Valor> = arr.iter().map(convertir_json_a_valor).collect();
            Valor::Lista(valores)
        }
        JsonValue::Object(_) => Valor::Json(json.clone()),
    }
}
