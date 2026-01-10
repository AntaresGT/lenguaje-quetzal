use crate::errores::{Error, CodigoError, Resultado};
use crate::nucleo::sintactico::ast::*;
use crate::nucleo::semantico::tabla_simbolos::{TablaSimbolos, ParametroFuncion, MiembroObjeto};
use crate::nucleo::semantico::tipos::Tipo;
use std::collections::HashMap;
use crate::nativos::interfaz::ModuloNativo;
use std::sync::Arc;

/// Verificador semántico
pub struct Verificador {
    tabla_simbolos: TablaSimbolos,
    tipo_retorno_actual: Option<Tipo>,
    dentro_de_bucle: bool,
    dentro_de_funcion_asincrona: bool,
    objetos_nativos: HashMap<String, Arc<dyn ModuloNativo>>,
    archivo_actual: Option<String>,
}

impl Verificador {
    /// Crea un nuevo verificador
    pub fn nuevo() -> Self {
        Self {
            tabla_simbolos: TablaSimbolos::nueva(),
            tipo_retorno_actual: None,
            dentro_de_bucle: false,
            dentro_de_funcion_asincrona: false,
            objetos_nativos: HashMap::new(),
            archivo_actual: None,
        }
    }
    
    /// Establece el archivo actual para resolver rutas relativas de módulos
    pub fn establecer_archivo_actual(&mut self, ruta: String) {
        self.archivo_actual = Some(ruta);
    }

    /// Registra un módulo nativo en el verificador para consulta de tipos
    pub fn registrar_objeto_nativo(&mut self, nombre: String, modulo: Arc<dyn ModuloNativo>) {
        self.objetos_nativos.insert(nombre, modulo);
    }
    
    /// Verifica un programa completo (lista de nodos)
    pub fn verificar_programa(&mut self, nodos: &[NodoAst]) -> Resultado<()> {
        // Primera pasada: registrar todas las funciones y objetos
        for nodo in nodos {
            self.registrar_declaraciones(nodo)?;
        }
        
        // Segunda pasada: verificar todo el código
        for nodo in nodos {
            self.verificar(nodo)?;
        }
        
        Ok(())
    }
    
    /// Registra declaraciones de funciones y objetos (primera pasada)
    fn registrar_declaraciones(&mut self, nodo: &NodoAst) -> Resultado<()> {
        match nodo {
            NodoAst::DeclaracionFuncion { nombre, tipo_retorno, parametros, .. } => {
                let tipo_ret = Tipo::desde_ast(tipo_retorno);
                let params: Vec<ParametroFuncion> = parametros.iter().map(|p| {
                    ParametroFuncion {
                        nombre: p.nombre.clone(),
                        tipo: Tipo::desde_ast(&p.tipo),
                        mutable: p.mutable,
                    }
                }).collect();
                
                self.tabla_simbolos.declarar_funcion(nombre.clone(), tipo_ret, params)
                    .map_err(|e| Error::semantico(
                        CodigoError::FuncionRedeclarada,
                        e,
                        None,
                        Some(nodo.posicion().linea),
                        Some(nodo.posicion().columna),
                    ))?;
            }
            
            NodoAst::DeclaracionObjeto { nombre, miembros, .. } => {
                let mut miembros_mapa: HashMap<String, MiembroObjeto> = HashMap::new();
                
                for miembro in miembros {
                    let es_publico = miembro.modificador_acceso == ModificadorAcceso::Publico;
                    let es_libre = miembro.libre;
                    
                    match miembro.declaracion.as_ref() {
                        NodoAst::DeclaracionVariable { tipo, mutable, nombre: nombre_var, .. } => {
                            miembros_mapa.insert(nombre_var.clone(), MiembroObjeto::Variable {
                                tipo: Tipo::desde_ast(tipo),
                                mutable: *mutable,
                                publico: es_publico,
                                libre: es_libre,
                            });
                        }
                        NodoAst::DeclaracionFuncion { nombre: nombre_fun, tipo_retorno, parametros, .. } => {
                            let params: Vec<ParametroFuncion> = parametros.iter().map(|p| {
                                ParametroFuncion {
                                    nombre: p.nombre.clone(),
                                    tipo: Tipo::desde_ast(&p.tipo),
                                    mutable: p.mutable,
                                }
                            }).collect();
                            
                            miembros_mapa.insert(nombre_fun.clone(), MiembroObjeto::Funcion {
                                tipo_retorno: Tipo::desde_ast(tipo_retorno),
                                parametros: params,
                                publico: es_publico,
                                libre: es_libre,
                            });
                        }
                        _ => {}
                    }
                }
                
                self.tabla_simbolos.declarar_objeto(nombre.clone(), miembros_mapa)
                    .map_err(|e| Error::semantico(
                        CodigoError::ObjetoRedeclarado,
                        e,
                        None,
                        Some(nodo.posicion().linea),
                        Some(nodo.posicion().columna),
                    ))?;
            }
            
            NodoAst::Importacion { elementos, ruta, .. } => {
                // Procesar importaciones en la primera pasada para que estén disponibles al verificar
                if crate::nativos::registro::es_modulo_nativo(ruta) {
                    // Para módulos nativos, solo registrar el objeto nativo
                    if let Some(modulo) = obtener_modulo_nativo_por_ruta(ruta) {
                        for elemento in elementos {
                            let nombre_importacion = elemento.alias.as_ref().unwrap_or(&elemento.nombre);
                            self.registrar_objeto_nativo(nombre_importacion.clone(), modulo.clone());
                        }
                    }
                } else {
                    // Procesar módulo externo: cargar y registrar sus exportaciones
                    let mut cargador = crate::modulos::CargadorModulos::nuevo();
                    
                    // Establecer directorio base si tenemos el archivo actual
                    if let Some(ref archivo_actual) = self.archivo_actual {
                        if let Some(dir) = std::path::Path::new(archivo_actual).parent() {
                            cargador.establecer_directorio_base(dir.to_path_buf());
                        }
                    }
                    
                    if let Ok(ast_modulo) = cargador.cargar_modulo(ruta) {
                        // Registrar todas las funciones, variables y objetos exportados del módulo
                        for nodo_modulo in &ast_modulo {
                            if let NodoAst::DeclaracionFuncion { nombre, tipo_retorno, parametros, .. } = nodo_modulo {
                                // Verificar si esta función está en la lista de elementos importados
                                if elementos.iter().any(|e| &e.nombre == nombre) {
                                    let elemento_importado = elementos.iter().find(|e| &e.nombre == nombre).unwrap();
                                    let nombre_importacion = elemento_importado.alias.as_ref().unwrap_or(nombre);
                                    
                                    let params: Vec<ParametroFuncion> = parametros.iter()
                                        .map(|p| ParametroFuncion {
                                            nombre: p.nombre.clone(),
                                            tipo: Tipo::desde_ast(&p.tipo),
                                            mutable: p.mutable,
                                        })
                                        .collect();
                                    
                                    if let Err(e) = self.tabla_simbolos.declarar_funcion(
                                        nombre_importacion.clone(),
                                        Tipo::desde_ast(tipo_retorno),
                                        params,
                                    ) {
                                        return Err(Error::semantico(
                                            CodigoError::FuncionRedeclarada,
                                            e,
                                            None,
                                            Some(nodo_modulo.posicion().linea),
                                            Some(nodo_modulo.posicion().columna),
                                        ));
                                    }
                                }
                            } else if let NodoAst::DeclaracionVariable { nombre, tipo, mutable, .. } = nodo_modulo {
                                // Verificar si esta variable está en la lista de elementos importados
                                if elementos.iter().any(|e| &e.nombre == nombre) {
                                    let elemento_importado = elementos.iter().find(|e| &e.nombre == nombre).unwrap();
                                    let nombre_importacion = elemento_importado.alias.as_ref().unwrap_or(nombre);
                                    
                                    if let Err(e) = self.tabla_simbolos.declarar_variable(
                                        nombre_importacion.clone(),
                                        Tipo::desde_ast(tipo),
                                        *mutable,
                                    ) {
                                        return Err(Error::semantico(
                                            CodigoError::VariableRedeclarada,
                                            e,
                                            None,
                                            Some(nodo_modulo.posicion().linea),
                                            Some(nodo_modulo.posicion().columna),
                                        ));
                                    }
                                }
                            } else if let NodoAst::DeclaracionObjeto { nombre, miembros, .. } = nodo_modulo {
                                // Verificar si este objeto está en la lista de elementos importados
                                if elementos.iter().any(|e| &e.nombre == nombre) {
                                    let elemento_importado = elementos.iter().find(|e| &e.nombre == nombre).unwrap();
                                    let nombre_importacion = elemento_importado.alias.as_ref().unwrap_or(nombre);
                                    
                                    let mut miembros_mapa = HashMap::new();
                                    for miembro_ast in miembros {
                                        match miembro_ast.declaracion.as_ref() {
                                            NodoAst::DeclaracionFuncion { nombre: nombre_fn, tipo_retorno, parametros, .. } => {
                                                let params: Vec<ParametroFuncion> = parametros.iter()
                                                    .map(|p| ParametroFuncion {
                                                        nombre: p.nombre.clone(),
                                                        tipo: Tipo::desde_ast(&p.tipo),
                                                        mutable: p.mutable,
                                                    })
                                                    .collect();
                                                
                                                miembros_mapa.insert(nombre_fn.clone(), MiembroObjeto::Funcion {
                                                    tipo_retorno: Tipo::desde_ast(tipo_retorno),
                                                    parametros: params,
                                                    publico: matches!(miembro_ast.modificador_acceso, crate::nucleo::sintactico::ast::ModificadorAcceso::Publico),
                                                    libre: miembro_ast.libre,
                                                });
                                            }
                                            NodoAst::DeclaracionVariable { nombre: nombre_var, tipo, mutable, .. } => {
                                                miembros_mapa.insert(nombre_var.clone(), MiembroObjeto::Variable {
                                                    tipo: Tipo::desde_ast(tipo),
                                                    mutable: *mutable,
                                                    publico: matches!(miembro_ast.modificador_acceso, crate::nucleo::sintactico::ast::ModificadorAcceso::Publico),
                                                    libre: miembro_ast.libre,
                                                });
                                            }
                                            _ => {}
                                        }
                                    }
                                    
                                    if let Err(e) = self.tabla_simbolos.declarar_objeto(nombre_importacion.clone(), miembros_mapa) {
                                        return Err(Error::semantico(
                                            CodigoError::ObjetoRedeclarado,
                                            e,
                                            None,
                                            Some(nodo_modulo.posicion().linea),
                                            Some(nodo_modulo.posicion().columna),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            _ => {}
        }
        
        Ok(())
    }
    
    /// Verifica un nodo del AST
    pub fn verificar(&mut self, nodo: &NodoAst) -> Resultado<Tipo> {
        match nodo {
            NodoAst::DeclaracionVariable { tipo, mutable, nombre, valor, .. } => {
                let tipo_semantico = Tipo::desde_ast(tipo);
                let tipo_valor = self.verificar(valor)?;
                
                // Permitir asignar a variables de tipo Vacio (inferencia)
                if tipo_semantico != Tipo::Vacio && !tipo_valor.puede_asignarse_a(&tipo_semantico) && tipo_valor != Tipo::Vacio {
                    return Err(Error::semantico(
                        CodigoError::TiposIncompatibles,
                        format!("no se puede asignar {} a variable de tipo {}", tipo_valor.nombre(), tipo_semantico.nombre()),
                        None,
                        Some(nodo.posicion().linea),
                        Some(nodo.posicion().columna),
                    ));
                }
                
                // Registrar la variable en la tabla de símbolos
                let tipo_final = if tipo_semantico == Tipo::Vacio { tipo_valor } else { tipo_semantico };
                self.tabla_simbolos.declarar_variable(nombre.clone(), tipo_final.clone(), *mutable)
                    .map_err(|e| Error::semantico(
                        CodigoError::VariableRedeclarada,
                        e,
                        None,
                        Some(nodo.posicion().linea),
                        Some(nodo.posicion().columna),
                    ))?;
                
                Ok(tipo_final)
            }
            
            NodoAst::DeclaracionFuncion { asincrono, tipo_retorno, parametros, cuerpo, .. } => {
                let tipo_ret = Tipo::desde_ast(tipo_retorno);
                
                // Guardar estado anterior
                let tipo_retorno_anterior = self.tipo_retorno_actual.clone();
                let asincrono_anterior = self.dentro_de_funcion_asincrona;
                
                self.tipo_retorno_actual = Some(tipo_ret.clone());
                self.dentro_de_funcion_asincrona = *asincrono;
                
                // Entrar en nuevo ámbito para los parámetros
                self.tabla_simbolos.entrar_ambito();
                
                // Registrar parámetros
                for param in parametros {
                    let tipo_param = Tipo::desde_ast(&param.tipo);
                    self.tabla_simbolos.declarar_variable(param.nombre.clone(), tipo_param, param.mutable)
                        .map_err(|e| Error::semantico(
                            CodigoError::ParametroRedeclarado,
                            e,
                            None,
                            Some(param.posicion.linea),
                            Some(param.posicion.columna),
                        ))?;
                }
                
                // Verificar cuerpo
                self.verificar(cuerpo)?;
                
                // Salir del ámbito
                self.tabla_simbolos.salir_ambito();
                
                // Restaurar estado
                self.tipo_retorno_actual = tipo_retorno_anterior;
                self.dentro_de_funcion_asincrona = asincrono_anterior;
                
                Ok(Tipo::Vacio)
            }
            
            NodoAst::DeclaracionObjeto { miembros, .. } => {
                // Verificar los cuerpos de los métodos
                for miembro in miembros {
                    if let NodoAst::DeclaracionFuncion { .. } = miembro.declaracion.as_ref() {
                        self.verificar(&miembro.declaracion)?;
                    }
                }
                Ok(Tipo::Vacio)
            }
            
            NodoAst::ExpresionLiteral { valor, .. } => {
                Ok(match valor {
                    LiteralAst::Entero(_) => Tipo::Entero,
                    LiteralAst::Numero(_) => Tipo::Numero,
                    LiteralAst::Texto(_) => Tipo::Texto,
                    LiteralAst::InterpolacionTexto(_) => Tipo::Texto,
                    LiteralAst::Logico(_) => Tipo::Logico,
                    LiteralAst::Nulo => Tipo::Vacio,
                })
            }
            
            NodoAst::ExpresionIdentificador { nombre, .. } => {
                // Manejar identificadores especiales
                if nombre == "ambiente" || nombre == "padre" {
                    // ambiente y padre son identificadores especiales que se resuelven en tiempo de ejecución
                    return Ok(Tipo::Vacio);
                }
                
                // Manejar objetos nativos globales
                if nombre == "consola" || nombre == "Matemática" || nombre == "Matematica" || nombre == "rango" {
                    // Objetos nativos se resuelven en tiempo de ejecución
                    return Ok(Tipo::Vacio);
                }
                
                // Primero buscar como variable
                if let Some(variable) = self.tabla_simbolos.buscar_variable(nombre) {
                    return Ok(variable.tipo.clone());
                }
                
                // Luego buscar como función
                if let Some(funcion) = self.tabla_simbolos.buscar_funcion(nombre) {
                    let params: Vec<Tipo> = funcion.parametros.iter().map(|p| p.tipo.clone()).collect();
                    return Ok(Tipo::Funcion {
                        parametros: params,
                        retorno: Box::new(funcion.tipo_retorno.clone()),
                    });
                }
                
                // Buscar como objeto registrado (clase)
                if self.tabla_simbolos.buscar_objeto(nombre).is_some() {
                    return Ok(Tipo::Objeto(nombre.clone()));
                }
                
                Err(Error::semantico(
                    CodigoError::VariableNoDeclarada,
                    format!("'{}' no está declarado", nombre),
                    None,
                    Some(nodo.posicion().linea),
                    Some(nodo.posicion().columna),
                ))
            }
            
            NodoAst::ExpresionBinaria { operador, izquierda, derecha, .. } => {
                let tipo_izq = self.verificar(izquierda)?;
                let tipo_der = self.verificar(derecha)?;
                
                match operador {
                    OperadorBinario::Suma | OperadorBinario::Resta | 
                    OperadorBinario::Multiplicacion | OperadorBinario::Division | 
                    OperadorBinario::Modulo | OperadorBinario::Potencia => {
                        if tipo_izq == Tipo::Entero && tipo_der == Tipo::Entero {
                            Ok(Tipo::Entero)
                        } else if (tipo_izq == Tipo::Entero || tipo_izq == Tipo::Numero) && 
                                  (tipo_der == Tipo::Entero || tipo_der == Tipo::Numero) {
                            Ok(Tipo::Numero)
                        } else if tipo_izq == Tipo::Texto && tipo_der == Tipo::Texto && *operador == OperadorBinario::Suma {
                            Ok(Tipo::Texto)
                        } else {
                            Err(Error::semantico(
                                CodigoError::TiposIncompatibles,
                                format!("operación no soportada entre {} y {}", tipo_izq.nombre(), tipo_der.nombre()),
                                None,
                                Some(nodo.posicion().linea),
                                Some(nodo.posicion().columna),
                            ))
                        }
                    }
                    
                    OperadorBinario::Igual | OperadorBinario::Diferente | 
                    OperadorBinario::Mayor | OperadorBinario::Menor | 
                    OperadorBinario::MayorIgual | OperadorBinario::MenorIgual => {
                        // Permitir comparaciones entre tipos compatibles
                        if tipo_izq.es_compatible_con(&tipo_der) || tipo_der.es_compatible_con(&tipo_izq) {
                            Ok(Tipo::Logico)
                        } else {
                            Err(Error::semantico(
                                CodigoError::ComparacionTiposIncompatibles,
                                format!("no se pueden comparar {} y {}", tipo_izq.nombre(), tipo_der.nombre()),
                                None,
                                Some(nodo.posicion().linea),
                                Some(nodo.posicion().columna),
                            ))
                        }
                    }
                    
                    OperadorBinario::Y | OperadorBinario::O => {
                        if tipo_izq == Tipo::Logico && tipo_der == Tipo::Logico {
                            Ok(Tipo::Logico)
                        } else {
                            Err(Error::semantico(
                                CodigoError::TiposIncompatibles,
                                "operadores lógicos requieren operandos booleanos".to_string(),
                                None,
                                Some(nodo.posicion().linea),
                                Some(nodo.posicion().columna),
                            ))
                        }
                    }
                    
                    OperadorBinario::Asignar | OperadorBinario::SumaAsignar |
                    OperadorBinario::RestaAsignar | OperadorBinario::MultiplicacionAsignar |
                    OperadorBinario::DivisionAsignar | OperadorBinario::ModuloAsignar => {
                        // Verificar que el lado izquierdo sea asignable
                        if let NodoAst::ExpresionIdentificador { nombre, .. } = izquierda.as_ref() {
                            if let Some(variable) = self.tabla_simbolos.buscar_variable(nombre) {
                                if !variable.mutable {
                                    return Err(Error::semantico(
                                        CodigoError::AsignacionAInmutable,
                                        format!("no se puede asignar a variable inmutable '{}'", nombre),
                                        Some("declara la variable con 'mut' para hacerla mutable".to_string()),
                                        Some(nodo.posicion().linea),
                                        Some(nodo.posicion().columna),
                                    ));
                                }
                            }
                        }
                        Ok(tipo_izq)
                    }
                }
            }
            
            NodoAst::ExpresionUnaria { operador, expresion, .. } => {
                let tipo_expr = self.verificar(expresion)?;
                
                match operador {
                    OperadorUnario::Negacion => {
                        if tipo_expr == Tipo::Logico {
                            Ok(Tipo::Logico)
                        } else {
                            Err(Error::semantico(
                                CodigoError::TiposIncompatibles,
                                "negación lógica requiere operando booleano".to_string(),
                                None,
                                Some(nodo.posicion().linea),
                                Some(nodo.posicion().columna),
                            ))
                        }
                    }
                    
                    OperadorUnario::Negativo | OperadorUnario::Positivo => {
                        if tipo_expr == Tipo::Entero || tipo_expr == Tipo::Numero {
                            Ok(tipo_expr)
                        } else {
                            Err(Error::semantico(
                                CodigoError::TiposIncompatibles,
                                "operador numérico requiere número".to_string(),
                                None,
                                Some(nodo.posicion().linea),
                                Some(nodo.posicion().columna),
                            ))
                        }
                    }
                    
                    OperadorUnario::Incrementar | OperadorUnario::Decrementar => {
                        if tipo_expr == Tipo::Entero || tipo_expr == Tipo::Numero {
                            Ok(tipo_expr)
                        } else {
                            Err(Error::semantico(
                                CodigoError::TiposIncompatibles,
                                "incremento/decremento requiere número".to_string(),
                                None,
                                Some(nodo.posicion().linea),
                                Some(nodo.posicion().columna),
                            ))
                        }
                    }
                }
            }
            
            NodoAst::ExpresionLlamada { funcion, argumentos, .. } => {
                let tipo_funcion = self.verificar(funcion)?;
                
                if let Tipo::Funcion { parametros, retorno } = tipo_funcion {
                    // Verificar número de argumentos
                    if argumentos.len() != parametros.len() {
                        return Err(Error::semantico(
                            CodigoError::NumeroArgumentosIncorrecto,
                            format!("se esperaban {} argumentos, se recibieron {}", parametros.len(), argumentos.len()),
                            None,
                            Some(nodo.posicion().linea),
                            Some(nodo.posicion().columna),
                        ));
                    }
                    
                    // Verificar tipos de argumentos
                    for (i, (arg, param_tipo)) in argumentos.iter().zip(parametros.iter()).enumerate() {
                        let tipo_arg = self.verificar(arg)?;
                        if !tipo_arg.puede_asignarse_a(&param_tipo) && tipo_arg != Tipo::Vacio {
                            return Err(Error::semantico(
                                CodigoError::TipoArgumentoIncorrecto,
                                format!("argumento {} de tipo {} no es compatible con parámetro de tipo {}", 
                                    i + 1, tipo_arg.nombre(), param_tipo.nombre()),
                                None,
                                Some(arg.posicion().linea),
                                Some(arg.posicion().columna),
                            ));
                        }
                    }
                    
                    Ok(*retorno)
                } else {
                    // Si es una llamada a un identificador que podría ser una función nativa
                    // permitimos la llamada sin verificación estricta
                    for arg in argumentos {
                        self.verificar(arg)?;
                    }
                    Ok(Tipo::Vacio)
                }
            }
            
            NodoAst::ExpresionAcceso { objeto, miembro, .. } => {
                let tipo_objeto = self.verificar(objeto)?;
                
                if let Tipo::Objeto(nombre_objeto) = &tipo_objeto {
                    if let Some(obj) = self.tabla_simbolos.buscar_objeto(nombre_objeto) {
                        if let Some(miembro_def) = obj.miembros.get(miembro) {
                            match miembro_def {
                                MiembroObjeto::Variable { tipo, .. } => Ok(tipo.clone()),
                                MiembroObjeto::Funcion { tipo_retorno, parametros, .. } => {
                                    let params: Vec<Tipo> = parametros.iter().map(|p| p.tipo.clone()).collect();
                                    Ok(Tipo::Funcion {
                                        parametros: params,
                                        retorno: Box::new(tipo_retorno.clone()),
                                    })
                                }
                            }
                        } else {
                            // El miembro podría ser heredado de un padre, permitir acceso dinámico
                            Ok(Tipo::Vacio)
                        }
                    } else {
                        // Objeto no registrado, permitir acceso dinámico
                        Ok(Tipo::Vacio)
                    }
                } else if tipo_objeto == Tipo::Vacio {
                    // Si el objeto es Vacio (objeto nativo), verificar constantes conocidas
                    if let NodoAst::ExpresionIdentificador { nombre: nombre_objeto, .. } = objeto.as_ref() {
                        if let Some(modulo_nativo) = self.objetos_nativos.get(nombre_objeto) {
                            // Consultar tipo de constante usando el módulo nativo
                            if let Some(tipo_constante) = modulo_nativo.obtener_tipo_constante(miembro) {
                                return Ok(tipo_constante);
                            }
                        }
                    }
                    // Si no es una constante conocida, continuar con métodos nativos
                    match miembro.as_str() {
                        "texto" => {
                            Ok(Tipo::Funcion {
                                parametros: vec![],
                                retorno: Box::new(Tipo::Texto),
                            })
                        },
                        "entero" => {
                            Ok(Tipo::Funcion {
                                parametros: vec![],
                                retorno: Box::new(Tipo::Entero),
                            })
                        },
                        "numero" | "número" => {
                            Ok(Tipo::Funcion {
                                parametros: vec![],
                                retorno: Box::new(Tipo::Numero),
                            })
                        },
                        "log" | "lóg" => {
                            Ok(Tipo::Funcion {
                                parametros: vec![],
                                retorno: Box::new(Tipo::Logico),
                            })
                        },
                        "jsn" => {
                            Ok(Tipo::Funcion {
                                parametros: vec![],
                                retorno: Box::new(Tipo::Json),
                            })
                        },
                        _ => {
                            Ok(Tipo::Vacio)
                        }
                    }
                } else if tipo_objeto == Tipo::Json {
                    // JSON permite acceso a cualquier propiedad
                    Ok(Tipo::Texto)
                } else {
                    // Para otros tipos, permitir acceso a métodos nativos
                    match miembro.as_str() {
                        "texto" => {
                            Ok(Tipo::Funcion {
                                parametros: vec![],
                                retorno: Box::new(Tipo::Texto),
                            })
                        },
                        "entero" => {
                            Ok(Tipo::Funcion {
                                parametros: vec![],
                                retorno: Box::new(Tipo::Entero),
                            })
                        },
                        "numero" | "número" => {
                            Ok(Tipo::Funcion {
                                parametros: vec![],
                                retorno: Box::new(Tipo::Numero),
                            })
                        },
                        "log" | "lóg" => {
                            Ok(Tipo::Funcion {
                                parametros: vec![],
                                retorno: Box::new(Tipo::Logico),
                            })
                        },
                        "jsn" => {
                            Ok(Tipo::Funcion {
                                parametros: vec![],
                                retorno: Box::new(Tipo::Json),
                            })
                        },
                        _ => {
                            Ok(Tipo::Vacio)
                        }
                    }
                }
            }
            
            NodoAst::ExpresionIndice { objeto, indice, .. } => {
                let tipo_objeto = self.verificar(objeto)?;
                let tipo_indice = self.verificar(indice)?;
                
                match tipo_objeto {
                    Tipo::Lista(tipo_interno) => {
                        if tipo_indice != Tipo::Entero {
                            return Err(Error::semantico(
                                CodigoError::IndiceTipoIncorrecto,
                                format!("índice de lista debe ser entero, se recibió {}", tipo_indice.nombre()),
                                None,
                                Some(nodo.posicion().linea),
                                Some(nodo.posicion().columna),
                            ));
                        }
                        Ok(*tipo_interno)
                    }
                    Tipo::Texto => {
                        if tipo_indice != Tipo::Entero {
                            return Err(Error::semantico(
                                CodigoError::IndiceTipoIncorrecto,
                                format!("índice de texto debe ser entero, se recibió {}", tipo_indice.nombre()),
                                None,
                                Some(nodo.posicion().linea),
                                Some(nodo.posicion().columna),
                            ));
                        }
                        Ok(Tipo::Texto)
                    }
                    Tipo::Json => {
                        // JSON permite cualquier tipo de índice
                        Ok(Tipo::Vacio)
                    }
                    _ => {
                        // Permitir acceso por índice a otros tipos dinámicamente
                        Ok(Tipo::Vacio)
                    }
                }
            }
            
            NodoAst::ExpresionTernario { condicion, verdadero, falso, .. } => {
                let tipo_cond = self.verificar(condicion)?;
                
                if tipo_cond != Tipo::Logico {
                    return Err(Error::semantico(
                        CodigoError::TiposIncompatibles,
                        "la condición del operador ternario debe ser booleana".to_string(),
                        None,
                        Some(nodo.posicion().linea),
                        Some(nodo.posicion().columna),
                    ));
                }
                
                let tipo_verdadero = self.verificar(verdadero)?;
                let tipo_falso = self.verificar(falso)?;
                
                if tipo_verdadero.es_compatible_con(&tipo_falso) {
                    Ok(tipo_verdadero)
                } else if tipo_falso.es_compatible_con(&tipo_verdadero) {
                    Ok(tipo_falso)
                } else {
                    Err(Error::semantico(
                        CodigoError::TiposIncompatibles,
                        format!("ramas del operador ternario tienen tipos incompatibles: {} y {}", 
                            tipo_verdadero.nombre(), tipo_falso.nombre()),
                        None,
                        Some(nodo.posicion().linea),
                        Some(nodo.posicion().columna),
                    ))
                }
            }
            
            NodoAst::ExpresionLista { elementos, .. } => {
                if elementos.is_empty() {
                    return Ok(Tipo::Lista(Box::new(Tipo::Vacio)));
                }
                
                let primer_tipo = self.verificar(&elementos[0])?;
                
                for elemento in elementos.iter().skip(1) {
                    let tipo_elem = self.verificar(elemento)?;
                    if !primer_tipo.es_compatible_con(&tipo_elem) && tipo_elem != Tipo::Vacio {
                        // Permitir listas heterogéneas
                    }
                }
                
                Ok(Tipo::Lista(Box::new(primer_tipo)))
            }
            
            NodoAst::ExpresionJson { propiedades, .. } => {
                for prop in propiedades {
                    self.verificar(&prop.valor)?;
                }
                Ok(Tipo::Json)
            }
            
            NodoAst::ExpresionNuevo { tipo: nombre_tipo, argumentos, .. } => {
                // Verificar argumentos
                for arg in argumentos {
                    self.verificar(arg)?;
                }
                Ok(Tipo::Objeto(nombre_tipo.clone()))
            }
            
            NodoAst::ExpresionAsignar { objetivo, valor, .. } => {
                // Verificar que el objetivo sea asignable
                if let NodoAst::ExpresionIdentificador { nombre, .. } = objetivo.as_ref() {
                    if let Some(variable) = self.tabla_simbolos.buscar_variable(nombre) {
                        if !variable.mutable {
                            return Err(Error::semantico(
                                CodigoError::AsignacionAInmutable,
                                format!("no se puede asignar a variable inmutable '{}'", nombre),
                                Some("declara la variable con 'mut' para hacerla mutable".to_string()),
                                Some(nodo.posicion().linea),
                                Some(nodo.posicion().columna),
                            ));
                        }
                    }
                }
                
                let tipo_objetivo = self.verificar(objetivo)?;
                let tipo_valor = self.verificar(valor)?;
                
                if !tipo_objetivo.es_compatible_con(&tipo_valor) && tipo_valor != Tipo::Vacio && tipo_objetivo != Tipo::Vacio {
                    return Err(Error::semantico(
                        CodigoError::TiposIncompatibles,
                        format!("no se puede asignar {} a {}", tipo_valor.nombre(), tipo_objetivo.nombre()),
                        None,
                        Some(nodo.posicion().linea),
                        Some(nodo.posicion().columna),
                    ));
                }
                
                Ok(tipo_valor)
            }
            
            NodoAst::ExpresionEsperar { expresion, .. } => {
                if !self.dentro_de_funcion_asincrona && self.tipo_retorno_actual.is_some() {
                    return Err(Error::semantico(
                        CodigoError::EsperarFueraDeAsincrona,
                        "'esperar' solo puede usarse dentro de funciones asíncronas".to_string(),
                        None,
                        Some(nodo.posicion().linea),
                        Some(nodo.posicion().columna),
                    ));
                }
                
                self.verificar(expresion)
            }
            
            NodoAst::Bloque { declaraciones, .. } => {
                self.tabla_simbolos.entrar_ambito();
                let mut ultimo_tipo = Tipo::Vacio;
                
                for declaracion in declaraciones {
                    ultimo_tipo = self.verificar(declaracion)?;
                }
                
                self.tabla_simbolos.salir_ambito();
                Ok(ultimo_tipo)
            }
            
            NodoAst::Si { condicion, entonces, sino, .. } => {
                let tipo_cond = self.verificar(condicion)?;
                
                if tipo_cond != Tipo::Logico && tipo_cond != Tipo::Vacio {
                    return Err(Error::semantico(
                        CodigoError::TiposIncompatibles,
                        format!("la condición debe ser booleana, se recibió {}", tipo_cond.nombre()),
                        None,
                        Some(nodo.posicion().linea),
                        Some(nodo.posicion().columna),
                    ));
                }
                
                self.verificar(entonces)?;
                
                if let Some(sino_bloque) = sino {
                    self.verificar(sino_bloque)?;
                }
                
                Ok(Tipo::Vacio)
            }
            
            NodoAst::Mientras { condicion, cuerpo, .. } => {
                let tipo_cond = self.verificar(condicion)?;
                
                if tipo_cond != Tipo::Logico && tipo_cond != Tipo::Vacio {
                    return Err(Error::semantico(
                        CodigoError::TiposIncompatibles,
                        format!("la condición debe ser booleana, se recibió {}", tipo_cond.nombre()),
                        None,
                        Some(nodo.posicion().linea),
                        Some(nodo.posicion().columna),
                    ));
                }
                
                let anterior_bucle = self.dentro_de_bucle;
                self.dentro_de_bucle = true;
                self.verificar(cuerpo)?;
                self.dentro_de_bucle = anterior_bucle;
                
                Ok(Tipo::Vacio)
            }
            
            NodoAst::Para { inicializacion, condicion, incremento, cuerpo, .. } => {
                self.tabla_simbolos.entrar_ambito();
                
                if let Some(init) = inicializacion {
                    self.verificar(init)?;
                }
                
                if let Some(cond) = condicion {
                    let tipo_cond = self.verificar(cond)?;
                    if tipo_cond != Tipo::Logico && tipo_cond != Tipo::Vacio {
                        return Err(Error::semantico(
                            CodigoError::TiposIncompatibles,
                            format!("la condición debe ser booleana, se recibió {}", tipo_cond.nombre()),
                            None,
                            Some(nodo.posicion().linea),
                            Some(nodo.posicion().columna),
                        ));
                    }
                }
                
                if let Some(inc) = incremento {
                    self.verificar(inc)?;
                }
                
                let anterior_bucle = self.dentro_de_bucle;
                self.dentro_de_bucle = true;
                self.verificar(cuerpo)?;
                self.dentro_de_bucle = anterior_bucle;
                
                self.tabla_simbolos.salir_ambito();
                Ok(Tipo::Vacio)
            }
            
            NodoAst::ParaEn { variable, tipo, mutable, coleccion, cuerpo, .. } => {
                let tipo_coleccion = self.verificar(coleccion)?;
                
                // Determinar el tipo del elemento
                let tipo_elemento = match &tipo_coleccion {
                    Tipo::Lista(tipo_interno) => *tipo_interno.clone(),
                    Tipo::Texto => Tipo::Texto,
                    _ => Tipo::desde_ast(tipo),
                };
                
                self.tabla_simbolos.entrar_ambito();
                self.tabla_simbolos.declarar_variable(variable.clone(), tipo_elemento, *mutable)
                    .map_err(|e| Error::semantico(
                        CodigoError::VariableRedeclarada,
                        e,
                        None,
                        Some(nodo.posicion().linea),
                        Some(nodo.posicion().columna),
                    ))?;
                
                let anterior_bucle = self.dentro_de_bucle;
                self.dentro_de_bucle = true;
                self.verificar(cuerpo)?;
                self.dentro_de_bucle = anterior_bucle;
                
                self.tabla_simbolos.salir_ambito();
                Ok(Tipo::Vacio)
            }
            
            NodoAst::HacerMientras { cuerpo, condicion, .. } => {
                let anterior_bucle = self.dentro_de_bucle;
                self.dentro_de_bucle = true;
                self.verificar(cuerpo)?;
                self.dentro_de_bucle = anterior_bucle;
                
                let tipo_cond = self.verificar(condicion)?;
                
                if tipo_cond != Tipo::Logico && tipo_cond != Tipo::Vacio {
                    return Err(Error::semantico(
                        CodigoError::TiposIncompatibles,
                        format!("la condición debe ser booleana, se recibió {}", tipo_cond.nombre()),
                        None,
                        Some(nodo.posicion().linea),
                        Some(nodo.posicion().columna),
                    ));
                }
                
                Ok(Tipo::Vacio)
            }
            
            NodoAst::Retornar { valor, .. } => {
                let tipo_valor = if let Some(val) = valor {
                    self.verificar(val)?
                } else {
                    Tipo::Vacio
                };
                
                if let Some(tipo_esperado) = &self.tipo_retorno_actual {
                    if !tipo_esperado.es_compatible_con(&tipo_valor) && tipo_valor != Tipo::Vacio && *tipo_esperado != Tipo::Vacio {
                        return Err(Error::semantico(
                            CodigoError::TipoRetornoIncorrecto,
                            format!("tipo de retorno incorrecto: se esperaba {}, se recibió {}", 
                                tipo_esperado.nombre(), tipo_valor.nombre()),
                            None,
                            Some(nodo.posicion().linea),
                            Some(nodo.posicion().columna),
                        ));
                    }
                }
                
                Ok(tipo_valor)
            }
            
            NodoAst::Romper { .. } => {
                if !self.dentro_de_bucle {
                    return Err(Error::semantico(
                        CodigoError::RomperFueraDeBucle,
                        "'romper' solo puede usarse dentro de bucles".to_string(),
                        None,
                        Some(nodo.posicion().linea),
                        Some(nodo.posicion().columna),
                    ));
                }
                Ok(Tipo::Vacio)
            }
            
            NodoAst::Continuar { .. } => {
                if !self.dentro_de_bucle {
                    return Err(Error::semantico(
                        CodigoError::ContinuarFueraDeBucle,
                        "'continuar' solo puede usarse dentro de bucles".to_string(),
                        None,
                        Some(nodo.posicion().linea),
                        Some(nodo.posicion().columna),
                    ));
                }
                Ok(Tipo::Vacio)
            }
            
            NodoAst::Intentar { bloque, capturar, finalmente, .. } => {
                self.verificar(bloque)?;
                
                if let Some(cap) = capturar {
                    self.tabla_simbolos.entrar_ambito();
                    // Registrar variable de excepción
                    self.tabla_simbolos.declarar_variable(cap.variable.clone(), Tipo::Json, false)
                        .map_err(|e| Error::semantico(
                            CodigoError::VariableRedeclarada,
                            e,
                            None,
                            Some(cap.posicion.linea),
                            Some(cap.posicion.columna),
                        ))?;
                    self.verificar(&cap.bloque)?;
                    self.tabla_simbolos.salir_ambito();
                }
                
                if let Some(fin) = finalmente {
                    self.verificar(fin)?;
                }
                
                Ok(Tipo::Vacio)
            }
            
            NodoAst::Lanzar { expresion, .. } => {
                self.verificar(expresion)?;
                Ok(Tipo::Vacio)
            }
            
            NodoAst::Importacion { .. } => {
                // Las importaciones ya se procesaron en registrar_declaraciones (primera pasada)
                // Aquí solo verificamos que la sintaxis sea correcta
                Ok(Tipo::Vacio)
            }
        }
    }
}


/// Obtiene un módulo nativo por su ruta (para verificación semántica)
fn obtener_modulo_nativo_por_ruta(ruta: &str) -> Option<Arc<dyn ModuloNativo>> {
    use crate::nativos::matematica::Matematica;
    use std::sync::Arc;
    
    match ruta {
        "quetzal/matemática" | "quetzal/matematica" => {
            Some(Arc::new(Matematica::nuevo()))
        }
        _ => None,
    }
}