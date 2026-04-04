use crate::errores::{Error, CodigoError, Resultado};
use crate::interprete::entorno::Entorno;
use crate::interprete::expresiones::evaluar_expresion;
use crate::interprete::valores::Valor;
use crate::interprete::excepciones::Excepcion;
use crate::nucleo::sintactico::ast::*;

/// Tipo de control de flujo para romper y continuar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlFlujo {
    Romper,
    Continuar,
}

/// Evalúa una declaración con manejo de control de flujo (romper/continuar)
fn evaluar_declaracion_con_control(nodo: &NodoAst, entorno: &mut Entorno) -> Resultado<Option<ControlFlujo>> {
    match evaluar_declaracion(nodo, entorno) {
        Ok(_) => Ok(None),
        Err(e) => {
            // Verificar si es un error de romper o continuar
            if e.codigo() == CodigoError::RomperFueraDeBucle.codigo() {
                Ok(Some(ControlFlujo::Romper))
            } else if e.codigo() == CodigoError::ContinuarFueraDeBucle.codigo() {
                Ok(Some(ControlFlujo::Continuar))
            } else {
                Err(e)
            }
        }
    }
}

/// Evalúa una declaración
pub fn evaluar_declaracion(nodo: &NodoAst, entorno: &mut Entorno) -> Resultado<Valor> {
    match nodo {
        NodoAst::DeclaracionVariable { tipo: _, mutable, nombre, valor, .. } => {
            let valor_evaluado = evaluar_expresion(valor, entorno)?;
            entorno.definir_variable(nombre.clone(), valor_evaluado.clone(), *mutable)
                .map_err(|e| Error::ejecucion(
                    CodigoError::VariableRedeclarada,
                    e,
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                ))?;
            Ok(valor_evaluado)
        }
        
        NodoAst::Bloque { declaraciones, .. } => {
            entorno.entrar_ambito();
            let mut ultimo_valor = Valor::Vacio;
            
            for declaracion in declaraciones {
                ultimo_valor = evaluar_declaracion(declaracion, entorno)?;
            }
            
            entorno.salir_ambito();
            Ok(ultimo_valor)
        }
        
        NodoAst::Si { condicion, entonces, sino, .. } => {
            let valor_condicion = evaluar_expresion(condicion, entorno)?;
            if valor_condicion.es_verdadero() {
                evaluar_declaracion(entonces, entorno)
            } else if let Some(sino_bloque) = sino {
                evaluar_declaracion(sino_bloque, entorno)
            } else {
                Ok(Valor::Vacio)
            }
        }
        
        NodoAst::Mientras { condicion, cuerpo, .. } => {
            loop {
                let valor_condicion = evaluar_expresion(condicion, entorno)?;
                if !valor_condicion.es_verdadero() {
                    break;
                }
                
                match evaluar_declaracion_con_control(cuerpo, entorno)? {
                    Some(ControlFlujo::Romper) => break,
                    Some(ControlFlujo::Continuar) => continue,
                    _ => {}
                }
            }
            Ok(Valor::Vacio)
        }
        
        NodoAst::Para { inicializacion, condicion, incremento, cuerpo, .. } => {
            // Ejecutar inicialización si existe
            if let Some(init) = inicializacion {
                evaluar_declaracion(init, entorno)?;
            }
            
            // Bucle mientras la condición sea verdadera
            loop {
                // Evaluar condición
                if let Some(cond) = condicion {
                    let valor_condicion = evaluar_expresion(cond, entorno)?;
                    if !valor_condicion.es_verdadero() {
                        break;
                    }
                }
                
                // Ejecutar cuerpo
                match evaluar_declaracion_con_control(cuerpo, entorno)? {
                    Some(ControlFlujo::Romper) => break,
                    Some(ControlFlujo::Continuar) => {
                        // Ejecutar incremento antes de continuar
                        if let Some(inc) = incremento {
                            evaluar_expresion(inc, entorno)?;
                        }
                        continue;
                    }
                    _ => {}
                }
                
                // Ejecutar incremento si existe
                if let Some(inc) = incremento {
                    evaluar_expresion(inc, entorno)?;
                }
            }
            Ok(Valor::Vacio)
        }
        
        NodoAst::ParaEn { variable, tipo: _, mutable: _, coleccion, cuerpo, .. } => {
            // Evaluar la colección
            let valor_coleccion = evaluar_expresion(coleccion, entorno)?;
            
            // Obtener la lista de elementos
            let elementos = match valor_coleccion {
                Valor::Lista(elem) => elem,
                _ => return Err(Error::ejecucion(
                    CodigoError::TiposIncompatibles,
                    format!("se esperaba una lista para el bucle 'para...en', se obtuvo {}", valor_coleccion.tipo()),
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                )),
            };
            
            // Iterar sobre los elementos
            for elemento in elementos {
                entorno.entrar_ambito();
                entorno.definir_variable(variable.clone(), elemento, true)
                    .map_err(|e| Error::ejecucion(
                        CodigoError::VariableRedeclarada,
                        e,
                        None,
                        Some(nodo.posicion().linea),
                        Some(nodo.posicion().columna),
                    ))?;
                
                match evaluar_declaracion_con_control(cuerpo, entorno)? {
                    Some(ControlFlujo::Romper) => {
                        entorno.salir_ambito();
                        break;
                    }
                    Some(ControlFlujo::Continuar) => {
                        entorno.salir_ambito();
                        continue;
                    }
                    _ => {}
                }
                
                entorno.salir_ambito();
            }
            
            Ok(Valor::Vacio)
        }
        
        NodoAst::HacerMientras { cuerpo, condicion, .. } => {
            loop {
                // Ejecutar cuerpo al menos una vez
                match evaluar_declaracion_con_control(cuerpo, entorno)? {
                    Some(ControlFlujo::Romper) => break,
                    Some(ControlFlujo::Continuar) => {
                        // Evaluar condición antes de continuar
                        let valor_condicion = evaluar_expresion(condicion, entorno)?;
                        if !valor_condicion.es_verdadero() {
                            break;
                        }
                        continue;
                    }
                    _ => {}
                }
                
                // Evaluar condición
                let valor_condicion = evaluar_expresion(condicion, entorno)?;
                if !valor_condicion.es_verdadero() {
                    break;
                }
            }
            Ok(Valor::Vacio)
        }
        
        NodoAst::Romper { .. } => {
            // Retornar un error especial que será capturado por el bucle
            Err(Error::ejecucion(
                CodigoError::RomperFueraDeBucle,
                "romper fuera de bucle (esto no debería ocurrir)",
                None,
                Some(nodo.posicion().linea),
                Some(nodo.posicion().columna),
            ))
        }
        
        NodoAst::Continuar { .. } => {
            // Retornar un error especial que será capturado por el bucle
            Err(Error::ejecucion(
                CodigoError::ContinuarFueraDeBucle,
                "continuar fuera de bucle (esto no debería ocurrir)",
                None,
                Some(nodo.posicion().linea),
                Some(nodo.posicion().columna),
            ))
        }
        
        NodoAst::Retornar { valor, .. } => {
            if let Some(valor_expr) = valor {
                evaluar_expresion(valor_expr, entorno)
            } else {
                Ok(Valor::Vacio)
            }
        }
        
        // Declaración de función: registrar la función en el entorno
        NodoAst::DeclaracionFuncion { nombre, parametros, cuerpo, asincrono, .. } => {
            let nombres_parametros: Vec<String> = parametros
                .iter()
                .map(|p| p.nombre.clone())
                .collect();
            
            let mutabilidad_parametros: Vec<bool> = parametros
                .iter()
                .map(|p| p.mutable)
                .collect();
            
            let valor_funcion = Valor::Funcion {
                nombre: nombre.clone(),
                parametros: nombres_parametros,
                parametros_mutables: mutabilidad_parametros,
                cuerpo: cuerpo.clone(),
                asincrono: *asincrono,
            };
            
            entorno.definir_variable(nombre.clone(), valor_funcion, false)
                .map_err(|e| Error::ejecucion(
                    CodigoError::VariableRedeclarada,
                    e,
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                ))?;
            
            Ok(Valor::Vacio)
        }

        // Declaración de prototipo: no-op en runtime (solo se valida en compilación)
        NodoAst::DeclaracionPrototipo { .. } => Ok(Valor::Vacio),
        
        NodoAst::Intentar { bloque, capturar, finalmente, .. } => {
            // Ejecutar el bloque intentar
            let resultado_intentar = evaluar_declaracion(bloque, entorno);
            
            // Si hubo una excepción, manejarla
            let excepcion = match &resultado_intentar {
                Err(e) => {
                    // Convertir Error a Excepcion con pila de llamadas
                    Some(Excepcion::desde_error(e))
                }
                Ok(_) => None,
            };
            
            // Si hay una excepción y hay un bloque capturar, ejecutarlo
            if let Some(exc) = excepcion {
                if let Some(capt) = capturar {
                    // Definir la variable de excepción en el entorno
                    entorno.entrar_ambito();
                    entorno.definir_variable(capt.variable.clone(), exc.a_valor(), false)
                        .map_err(|e| Error::ejecucion(
                            CodigoError::ErrorInternoInterprete,
                            e,
                            None,
                            Some(nodo.posicion().linea),
                            Some(nodo.posicion().columna),
                        ))?;
                    
                    let resultado_capturar = evaluar_declaracion(&capt.bloque, entorno);
                    entorno.salir_ambito();
                    
                    // Si el bloque capturar también lanzó una excepción, propagarla
                    resultado_capturar?;
                } else {
                    // Si no hay bloque capturar, propagar la excepción
                    return resultado_intentar;
                }
            }
            
            // Ejecutar bloque finalmente si existe
            if let Some(bloque_finalmente) = finalmente {
                evaluar_declaracion(bloque_finalmente, entorno)?;
            }
            
            // Retornar el resultado del bloque intentar o vacío si hubo excepción
            resultado_intentar.or(Ok(Valor::Vacio))
        }
        
        NodoAst::Lanzar { expresion, .. } => {
            // Evaluar la expresión (mensaje de la excepción)
            let valor = evaluar_expresion(expresion, entorno)?;
            
            // Convertir el valor a mensaje de excepción
            let mensaje = match valor {
                Valor::Texto(s) => s,
                _ => valor.a_texto(),
            };
            
            // Lanzar la excepción como un error
            Err(Error::ejecucion(
                CodigoError::ExcepcionLanzada,
                mensaje,
                None,
                Some(nodo.posicion().linea),
                Some(nodo.posicion().columna),
            ))
        }
        
        // Declaración de objeto: registra el tipo y los miembros libres
        NodoAst::DeclaracionObjeto { nombre, padres, miembros, .. } => {
            use std::collections::HashMap;
            use crate::interprete::entorno::DefinicionObjeto;
            
            let mut propiedades = HashMap::new();
            propiedades.insert("__tipo__".to_string(), Valor::Texto(format!("tipo:{}", nombre)));
            propiedades.insert("__padres__".to_string(), Valor::Lista(
                padres.iter().map(|p| Valor::Texto(p.clone())).collect()
            ));
            
            // Buscar el constructor (función con el mismo nombre que el objeto)
            let mut constructor = None;
            for miembro in miembros {
                match miembro.declaracion.as_ref() {
                    NodoAst::DeclaracionFuncion { nombre: nombre_fn, .. } if nombre_fn == nombre => {
                        constructor = Some(miembro.declaracion.as_ref().clone());
                    }
                    _ => {}
                }
            }
            
            // Registrar la definición del objeto con información de herencia
            let definicion = DefinicionObjeto {
                nombre: nombre.clone(),
                padres: padres.clone(),
                miembros: miembros.clone(),
                constructor,
            };
            entorno.registrar_definicion_objeto(definicion);
            
            // Procesar miembros libres (estáticos)
            for miembro in miembros {
                if miembro.libre {
                    match miembro.declaracion.as_ref() {
                        NodoAst::DeclaracionFuncion { nombre: nombre_fn, parametros, cuerpo, asincrono, .. } => {
                            let param_nombres: Vec<String> = parametros.iter()
                                .map(|p| p.nombre.clone())
                                .collect();
                            let param_mutables: Vec<bool> = parametros.iter()
                                .map(|p| p.mutable)
                                .collect();
                            let funcion = Valor::Funcion {
                                nombre: nombre_fn.clone(),
                                parametros: param_nombres,
                                parametros_mutables: param_mutables,
                                cuerpo: cuerpo.clone(),
                                asincrono: *asincrono,
                            };
                            propiedades.insert(nombre_fn.clone(), funcion);
                        }
                        NodoAst::DeclaracionVariable { nombre: nombre_var, valor, mutable, .. } => {
                            let valor_evaluado = evaluar_expresion(valor, entorno)?;
                            propiedades.insert(nombre_var.clone(), valor_evaluado);
                            // Si es mutable, marcar como tal
                            if *mutable {
                                propiedades.insert(format!("__mutable_{}__", nombre_var), Valor::Logico(true));
                            }
                        }
                        _ => {}
                    }
                }
            }
            
            let valor_tipo = Valor::Objeto {
                tipo: format!("tipo:{}", nombre),
                propiedades,
            };
            entorno.definir_variable(nombre.clone(), valor_tipo, false)
                .map_err(|e| Error::ejecucion(
                    CodigoError::VariableRedeclarada,
                    e,
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                ))?;
            
            Ok(Valor::Vacio)
        }
        
        // Importación: cargar módulo nativo o externo
        NodoAst::Importacion { elementos, ruta, posicion } => {
            // Primero intentar como módulo nativo
            if crate::nativos::registro::es_modulo_nativo(ruta) {
                crate::nativos::registro::procesar_importacion(elementos, ruta, entorno)?;
            } else {
                // Cargar módulo externo desde archivo .qz
                let mut cargador = crate::modulos::CargadorModulos::nuevo();
                
                // Establecer directorio base si es posible
                if let Some(ref archivo_actual) = entorno.archivo_actual() {
                    if let Some(dir) = std::path::Path::new(archivo_actual).parent() {
                        cargador.establecer_directorio_base(dir.to_path_buf());
                    }
                }
                
                // Cargar el módulo
                let ast_modulo = cargador.cargar_modulo(ruta).map_err(|e| {
                    Error::modulo(
                        CodigoError::ErrorCargarModulo,
                        e.to_string(),
                        Some(ruta.to_string()),
                    )
                })?;

                let prototipos_exportados: std::collections::HashSet<String> = ast_modulo
                    .iter()
                    .filter_map(|nodo| {
                        if let NodoAst::DeclaracionPrototipo { nombre, .. } = nodo {
                            Some(nombre.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                
                // Crear entorno temporal para ejecutar el módulo
                let mut entorno_modulo = Entorno::nuevo();
                crate::nativos::registro::registrar_modulos_nativos(&mut entorno_modulo)?;
                
                // Ejecutar el módulo para obtener sus exportaciones
                for nodo_modulo in &ast_modulo {
                    evaluar_declaracion(nodo_modulo, &mut entorno_modulo)?;
                }
                
                // Importar los elementos solicitados
                if elementos.is_empty() {
                    // Importar todo el módulo con su nombre
                    let nombre_modulo = std::path::Path::new(ruta)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or(ruta);
                    
                    // Crear objeto con las exportaciones del módulo
                    let mut propiedades = std::collections::HashMap::new();
                    for (nombre, valor) in entorno_modulo.obtener_variables_globales() {
                        propiedades.insert(nombre, valor);
                    }
                    
                    let modulo = Valor::Objeto {
                        tipo: format!("modulo:{}", nombre_modulo),
                        propiedades,
                    };
                    
                    entorno.definir_variable(nombre_modulo.to_string(), modulo, false)
                        .map_err(|e| Error::ejecucion(
                            CodigoError::VariableRedeclarada,
                            e,
                            None,
                            Some(posicion.linea),
                            Some(posicion.columna),
                        ))?;
                } else {
                    // Importar elementos específicos
                    for elemento in elementos {
                        // Buscar el elemento por su nombre original en el módulo
                        let nombre_en_modulo = &elemento.nombre;

                        // Los prototipos son contratos de compilación y no generan valores de runtime
                        if prototipos_exportados.contains(nombre_en_modulo) {
                            continue;
                        }
                        
                        // Buscar en variables (incluye funciones que se almacenan como Valor::Funcion)
                        let valor_opt = if let Some(valor_ref) = entorno_modulo.obtener_variable(nombre_en_modulo) {
                            Some(valor_ref.clone())
                        } else if let Some(func) = entorno_modulo.obtener_funcion(nombre_en_modulo) {
                            // Convertir función a Valor si no se encuentra en variables
                            Some(Valor::Funcion {
                                nombre: func.nombre.clone(),
                                parametros: func.parametros.clone(),
                                parametros_mutables: vec![false; func.parametros.len()],
                                cuerpo: func.cuerpo.clone(),
                                asincrono: false,
                            })
                        } else if entorno_modulo.obtener_definicion_objeto(nombre_en_modulo).is_some() {
                            // Convertir definición de objeto a Valor si no se encuentra en variables
                            // Buscar si hay una instancia del objeto en variables
                            if let Some(instancia) = entorno_modulo.obtener_variable(nombre_en_modulo) {
                                Some(instancia.clone())
                            } else {
                                // Si no hay instancia, crear el tipo del objeto
                                Some(Valor::Objeto {
                                    tipo: format!("tipo:{}", nombre_en_modulo),
                                    propiedades: std::collections::HashMap::new(),
                                })
                            }
                        } else {
                            None
                        };
                        
                        if let Some(valor) = valor_opt {
                            // Usar el alias si existe, o el nombre original si no
                            let nombre_importacion = elemento.alias.as_ref().unwrap_or(&elemento.nombre);
                            entorno.definir_variable(nombre_importacion.clone(), valor, false)
                                .map_err(|e| Error::ejecucion(
                                    CodigoError::VariableRedeclarada,
                                    e,
                                    None,
                                    Some(posicion.linea),
                                    Some(posicion.columna),
                                ))?;
                        } else {
                            return Err(Error::modulo(
                                CodigoError::ElementoImportadoNoEncontrado,
                                format!("'{}' no está exportado en el módulo '{}'", nombre_en_modulo, ruta),
                                Some(ruta.to_string()),
                            ));
                        }
                    }
                }
            }
            Ok(Valor::Vacio)
        }
        
        _ => {
            // Si no es una declaración, intenta evaluarlo como expresión
            evaluar_expresion(nodo, entorno)
        }
    }
}
