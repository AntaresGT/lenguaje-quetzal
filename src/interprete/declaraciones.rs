use crate::errores::{Error, CodigoError, Resultado};
use crate::interprete::entorno::Entorno;
use crate::interprete::expresiones::evaluar_expresion;
use crate::interprete::hir::ejecutar_programa_hir;
use crate::interprete::valores::Valor;
use crate::interprete::excepciones::Excepcion;
use crate::modulos::ModuloCompilado;
use crate::nucleo::sintactico::ast::*;
use std::collections::HashMap;

/// Tipo de control de flujo para romper y continuar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlFlujo {
    Romper,
    Continuar,
}

fn convertir_funcion_a_valor(funcion: &crate::interprete::entorno::Funcion) -> Valor {
    Valor::Funcion {
        nombre: funcion.nombre.clone(),
        parametros: funcion.parametros.clone(),
        parametros_mutables: vec![false; funcion.parametros.len()],
        cuerpo: funcion.cuerpo.clone(),
        asincrono: false,
    }
}

fn buscar_valor_exportado(entorno: &Entorno, nombre: &str) -> Option<Valor> {
    if let Some(valor) = entorno.obtener_variable(nombre) {
        return Some(valor.clone());
    }

    if let Some(funcion) = entorno.obtener_funcion(nombre) {
        return Some(convertir_funcion_a_valor(funcion));
    }

    if entorno.obtener_definicion_objeto(nombre).is_some() {
        return Some(Valor::Objeto {
            tipo: format!("tipo:{}", nombre),
            propiedades: HashMap::new(),
        });
    }

    None
}

fn construir_exportaciones_runtime(modulo: &ModuloCompilado, entorno: &Entorno) -> HashMap<String, Valor> {
    let mut exportaciones = HashMap::new();

    for (nombre, nodo) in &modulo.exportaciones {
        if matches!(nodo, NodoAst::DeclaracionPrototipo { .. }) {
            continue;
        }

        if let Some(valor) = buscar_valor_exportado(entorno, nombre) {
            exportaciones.insert(nombre.clone(), valor);
        }
    }

    exportaciones
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

        // Exportar: no-op en runtime; ya fue resuelto en HIR/cargador de módulos
        NodoAst::Exportacion { .. } => Ok(Valor::Vacio),
        
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
                let cargador = entorno.cargador_modulos();
                let modulo = cargador.obtener_modulo_compilado(ruta).map_err(|e| {
                    Error::modulo(
                        CodigoError::ErrorCargarModulo,
                        e.to_string(),
                        Some(ruta.to_string()),
                    )
                })?;
                let exportaciones = if let Some(cache) =
                    cargador.obtener_exportaciones_ejecutadas(&modulo.ruta_absoluta)
                {
                    cache
                } else {
                    let mut entorno_modulo = Entorno::con_archivo(
                        modulo.ruta_absoluta.to_string_lossy().to_string(),
                    );
                    entorno_modulo.establecer_cargador_modulos(cargador.clone());

                    if let Some(manifiesto) = &modulo.manifiesto {
                        if let Some(permisos) = manifiesto.construir_permisos() {
                            entorno_modulo.establecer_permisos(permisos);
                        }
                    }

                    crate::nativos::registro::registrar_modulos_nativos(&mut entorno_modulo)?;
                    ejecutar_programa_hir(&modulo.hir, &mut entorno_modulo)?;

                    let exportaciones = construir_exportaciones_runtime(&modulo, &entorno_modulo);
                    cargador.guardar_exportaciones_ejecutadas(
                        modulo.ruta_absoluta.clone(),
                        exportaciones.clone(),
                    );
                    exportaciones
                };
                
                // Importar los elementos solicitados
                if elementos.is_empty() {
                    // Importar todo el módulo con su nombre
                    let nombre_modulo = modulo
                        .manifiesto
                        .as_ref()
                        .map(|manifiesto| manifiesto.nombre.as_str())
                        .or_else(|| {
                            modulo
                                .ruta_absoluta
                                .file_stem()
                                .and_then(|segmento| segmento.to_str())
                        })
                        .or_else(|| std::path::Path::new(ruta).file_stem().and_then(|s| s.to_str()))
                        .unwrap_or(ruta);
                    
                    let modulo = Valor::Objeto {
                        tipo: format!("modulo:{}", nombre_modulo),
                        propiedades: exportaciones.clone(),
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
                        if let Some(valor) = exportaciones.get(&elemento.nombre).cloned() {
                            let nombre_importacion = elemento.alias.as_ref().unwrap_or(&elemento.nombre);
                            entorno.definir_variable(nombre_importacion.clone(), valor, false)
                                .map_err(|e| Error::ejecucion(
                                    CodigoError::VariableRedeclarada,
                                    e,
                                    None,
                                    Some(posicion.linea),
                                    Some(posicion.columna),
                                ))?;
                            continue;
                        }

                        if matches!(
                            modulo.exportaciones.get(&elemento.nombre),
                            Some(NodoAst::DeclaracionPrototipo { .. })
                        ) {
                            continue;
                        }

                        return Err(Error::modulo(
                            CodigoError::ElementoImportadoNoEncontrado,
                            format!("'{}' no está exportado en el módulo '{}'", elemento.nombre, ruta),
                            Some(ruta.to_string()),
                        ));
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
