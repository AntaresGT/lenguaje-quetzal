// Evaluador del AST para el lenguaje Quetzal
// Ejecuta el Árbol de Sintaxis Abstracta y maneja el entorno de ejecución

use crate::analizador_sintactico::{Nodo, Parametro};
use crate::tipos_datos::{Valor, Variable, TipoVariable};
use crate::errores::{ErrorQuetzal, ResultadoQuetzal};
use crate::consola::CONSOLA_GLOBAL;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

/// Entorno de ejecución que mantiene el estado de variables y funciones
#[derive(Debug, Clone)]
pub struct Entorno {
    /// Variables del entorno actual
    variables: HashMap<String, Variable>,
    /// Funciones definidas
    funciones: HashMap<String, FuncionDefinida>,
    /// Entorno padre (para scoping)
    padre: Option<Rc<RefCell<Entorno>>>,
}

/// Definición de una función
#[derive(Debug, Clone)]
pub struct FuncionDefinida {
    pub parametros: Vec<Parametro>,
    pub tipo_retorno: String,
    pub cuerpo: Nodo,
    #[allow(dead_code)]
    pub es_asincrona: bool,
}

/// Resultado del control de flujo
#[derive(Debug, Clone, PartialEq)]
pub enum ControlFlujo {
    Ninguno,
    Retornar(Valor),
    Romper,
    Continuar,
}

impl Entorno {
    /// Crea un nuevo entorno vacío
    pub fn nuevo() -> Self {
        Entorno {
            variables: HashMap::new(),
            funciones: HashMap::new(),
            padre: None,
        }
    }
    
    /// Crea un nuevo entorno con un padre
    pub fn nuevo_hijo(padre: Rc<RefCell<Entorno>>) -> Self {
        Entorno {
            variables: HashMap::new(),
            funciones: HashMap::new(),
            padre: Some(padre),
        }
    }
    
    /// Crea un nuevo entorno con un padre
    pub fn con_padre(padre: Rc<RefCell<Entorno>>) -> Self {
        Entorno {
            variables: HashMap::new(),
            funciones: HashMap::new(),
            padre: Some(padre),
        }
    }
    
    /// Define una nueva variable
    pub fn definir_variable(&mut self, nombre: String, variable: Variable) -> ResultadoQuetzal<()> {
        if !self.es_nombre_valido(&nombre) {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: 0,
                mensaje: format!("Nombre de variable inválido: {}", nombre),
            });
        }
        
        // Solo verificar duplicados en el entorno actual (shadowing permitido)
        if self.variables.contains_key(&nombre) {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: 0,
                mensaje: format!("La variable '{}' ya está declarada en este ámbito", nombre),
            });
        }
        
        self.variables.insert(nombre, variable);
        Ok(())
    }
    
    /// Obtiene una variable del entorno actual o de los padres
    pub fn obtener_variable(&self, nombre: &str) -> Option<Variable> {
        if let Some(variable) = self.variables.get(nombre) {
            Some(variable.clone())
        } else if let Some(ref padre) = self.padre {
            padre.borrow().obtener_variable(nombre)
        } else {
            None
        }
    }
    
    /// Define una nueva función
    pub fn definir_funcion(&mut self, nombre: String, funcion: FuncionDefinida) -> ResultadoQuetzal<()> {
        if !self.es_nombre_valido(&nombre) {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: 0,
                mensaje: format!("Nombre de función inválido: {}", nombre),
            });
        }
        
        self.funciones.insert(nombre, funcion);
        Ok(())
    }
    
    /// Obtiene una función del entorno actual o de los padres
    pub fn obtener_funcion(&self, nombre: &str) -> Option<FuncionDefinida> {
        if let Some(funcion) = self.funciones.get(nombre) {
            Some(funcion.clone())
        } else if let Some(ref padre) = self.padre {
            padre.borrow().obtener_funcion(nombre)
        } else {
            None
        }
    }
    
    /// Verifica si un nombre de variable es válido
    fn es_nombre_valido(&self, nombre: &str) -> bool {
        if nombre.is_empty() {
            return false;
        }
        
        let palabras_reservadas = [
            "vacio", "entero", "cadena", "bool", "lista", "jsn",
            "si", "sino", "para", "mientras", "hacer", "romper", "continuar",
            "retornar", "objeto", "nuevo", "ambiente", "asincrono", "esperar",
            "intentar", "atrapar", "finalmente", "lanzar", "excepcion",
            "importar", "exportar", "desde", "como", "privado", "publico",
            "verdadero", "falso", "nulo", "y", "o", "en", "de", "es", "no", "mut"
        ];
        
        if palabras_reservadas.contains(&nombre) {
            return false;
        }
        
        let primer_caracter = nombre.chars().next().unwrap();
        if !primer_caracter.is_alphabetic() && primer_caracter != '_' {
            return false;
        }
        
        nombre.chars().all(|c| c.is_alphanumeric() || c == '_')
    }
    
    /// Asigna un valor a una variable existente
    #[allow(dead_code)]
    pub fn asignar_variable(&mut self, nombre: &str, valor: Valor, linea: usize) -> ResultadoQuetzal<()> {
        if let Some(variable) = self.variables.get_mut(nombre) {
            if variable.tipo_variable == TipoVariable::Inmutable {
                return Err(ErrorQuetzal::ErrorEjecucion {
                    linea,
                    mensaje: format!("No se puede reasignar la variable inmutable '{}'", nombre),
                });
            }
            variable.valor = valor;
            Ok(())
        } else if let Some(ref padre) = self.padre {
            padre.borrow_mut().asignar_variable(nombre, valor, linea)
        } else {
            Err(ErrorQuetzal::VariableNoDefinida {
                linea,
                nombre: nombre.to_string(),
            })
        }
    }
}

/// Evaluador principal
pub struct Evaluador {
    entorno_global: Rc<RefCell<Entorno>>,
    profundidad_recursion: usize,
    max_profundidad_recursion: usize,
    dentro_de_funcion: bool,
}

impl Evaluador {
    /// Crea un nuevo evaluador
    pub fn nuevo() -> Self {
        let mut entorno = Entorno::nuevo();
        
        // Definir funciones y objetos globales predefinidos
        Self::definir_funciones_globales(&mut entorno);
        
        Evaluador {
            entorno_global: Rc::new(RefCell::new(entorno)),
            profundidad_recursion: 0,
            max_profundidad_recursion: 1000, // Límite razonable para recursión
            dentro_de_funcion: false,
        }
    }
    
    /// Redondea un número de punto flotante para evitar problemas de precisión
    fn redondear_numero(&self, numero: f64) -> f64 {
        // Redondear a 15 decimales para evitar problemas de precisión de punto flotante
        // pero mantener suficiente precisión para cálculos normales
        let factor = 1e15;
        (numero * factor).round() / factor
    }
    
    /// Define funciones y objetos globales predefinidos
    fn definir_funciones_globales(entorno: &mut Entorno) {
        // Definir el objeto consola global como una variable especial
        let consola_variable = Variable::nueva(
            "consola".to_string(),
            Valor::Json(std::collections::HashMap::new()), // Objeto vacío por ahora
            TipoVariable::Inmutable,
            "consola".to_string(),
        );
        
        // Ignorar el error si la variable ya existe
        let _ = entorno.definir_variable("consola".to_string(), consola_variable);
    }
    
    /// Evalúa un nodo del AST
    pub fn evaluar(&mut self, nodo: &Nodo) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        self.evaluar_con_entorno(nodo, self.entorno_global.clone())
    }
    
    /// Evalúa un nodo con un entorno específico
    fn evaluar_con_entorno(&mut self, nodo: &Nodo, entorno: Rc<RefCell<Entorno>>) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        // Verificar si estamos en una llamada a función para controlar recursión
        if let Nodo::LlamadaFuncion { .. } = nodo {
            // Verificar recursión infinita para llamadas a funciones
            if self.profundidad_recursion >= self.max_profundidad_recursion {
                return Err(ErrorQuetzal::ErrorEjecucion {
                    linea: 0,
                    mensaje: format!("Recursión infinita detectada: profundidad máxima de {} excedida", self.max_profundidad_recursion),
                });
            }
        }
        
        match nodo {
            Nodo::Programa(declaraciones) => {
                let mut ultimo_valor = Valor::Vacio;
                
                for declaracion in declaraciones {
                    let (valor, control) = self.evaluar_con_entorno(declaracion, entorno.clone())?;
                    ultimo_valor = valor;
                    
                    // Manejar control de flujo a nivel de programa
                    match control {
                        ControlFlujo::Retornar(_) => return Ok((ultimo_valor, control)),
                        _ => {},
                    }
                }
                
                Ok((ultimo_valor, ControlFlujo::Ninguno))
            },
            
            Nodo::DeclaracionVariable { nombre, tipo_dato, es_mutable, valor, linea } => {
                // Evaluar el valor inicial si existe
                let valor_inicial = if let Some(expr_valor) = valor {
                    let (val, _) = self.evaluar_con_entorno(expr_valor, entorno.clone())?;
                    
                    // Validar compatibilidad de tipos
                    if !self.validar_tipo_compatible(&val, tipo_dato) {
                        return Err(ErrorQuetzal::ErrorSintaxis {
                            linea: *linea,
                            mensaje: format!("Tipo incompatible: no se puede asignar {} a variable de tipo {}", 
                                self.obtener_nombre_tipo(&val), tipo_dato),
                        });
                    }
                    
                    // Realizar conversión automática si es necesario
                    self.convertir_tipo_automatico(val, tipo_dato)?
                } else {
                    // Valor por defecto según el tipo
                    match tipo_dato.as_str() {
                        "vacio" => Valor::Vacio,
                        "entero" => Valor::Entero(0),
                        "número" => Valor::Numero(0.0),
                        "cadena" => Valor::Cadena(String::new()),
                        "bool" => Valor::Bool(false),
                        "lista" => Valor::Lista(Vec::new()),
                        "jsn" => Valor::Json(HashMap::new()),
                        _ => Valor::Vacio,
                    }
                };
                
                // En Quetzal, las variables son inmutables por defecto a menos que se especifique explícitamente como mutable
                let tipo_variable = if *es_mutable {
                    TipoVariable::Mutable
                } else {
                    TipoVariable::Inmutable  // Por defecto inmutable
                };
                
                let variable = Variable::nueva(
                    nombre.clone(),
                    valor_inicial.clone(),
                    tipo_variable,
                    tipo_dato.clone(),
                );
                
                entorno.borrow_mut().definir_variable(nombre.clone(), variable)?;
                Ok((valor_inicial, ControlFlujo::Ninguno))
            },
            
            Nodo::DeclaracionFuncion { nombre, parametros, tipo_retorno, cuerpo, es_asincrona, .. } => {
                // Validar que funciones no-vacías tengan cuerpo
                if tipo_retorno != "vacio" {
                    if let Nodo::Bloque(sentencias) = cuerpo.as_ref() {
                        if sentencias.is_empty() {
                            return Err(ErrorQuetzal::ErrorSintaxis {
                                linea: 0,
                                mensaje: format!("La función '{}' de tipo '{}' no puede tener un bloque vacío", nombre, tipo_retorno),
                            });
                        }
                    }
                }
                
                let funcion = FuncionDefinida {
                    parametros: parametros.clone(),
                    tipo_retorno: tipo_retorno.clone(),
                    cuerpo: (**cuerpo).clone(),
                    es_asincrona: *es_asincrona,
                };
                
                entorno.borrow_mut().definir_funcion(nombre.clone(), funcion)?;
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            },
            
            Nodo::Literal(valor) => {
                Ok((valor.clone(), ControlFlujo::Ninguno))
            },
            
            Nodo::Identificador(nombre) => {
                let entorno_ref = entorno.borrow();
                if let Some(variable) = entorno_ref.obtener_variable(nombre) {
                    Ok((variable.valor.clone(), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::VariableNoDefinida {
                        linea: 0,
                        nombre: nombre.clone(),
                    })
                }
            },
            
            Nodo::OperacionBinaria { izquierdo, operador, derecho } => {
                let (valor_izq, _) = self.evaluar_con_entorno(izquierdo, entorno.clone())?;
                let (valor_der, _) = self.evaluar_con_entorno(derecho, entorno)?;
                let resultado = self.evaluar_operacion_binaria(&valor_izq, operador, &valor_der)?;
                Ok((resultado, ControlFlujo::Ninguno))
            },
            
            Nodo::OperacionUnaria { operador, operando } => {
                let (valor, _) = self.evaluar_con_entorno(operando, entorno)?;
                let resultado = self.evaluar_operacion_unaria(operador, &valor)?;
                Ok((resultado, ControlFlujo::Ninguno))
            },
            
            Nodo::Condicional { condicion, bloque_si, bloque_sino, .. } => {
                let (valor_condicion, _) = self.evaluar_con_entorno(condicion, entorno.clone())?;
                
                if valor_condicion.a_bool() {
                    self.evaluar_con_entorno(bloque_si, entorno)
                } else if let Some(bloque_no) = bloque_sino {
                    self.evaluar_con_entorno(bloque_no, entorno)
                } else {
                    Ok((Valor::Vacio, ControlFlujo::Ninguno))
                }
            },
            
            Nodo::Bloque(declaraciones) => {
                let mut ultimo_valor = Valor::Vacio;
                
                for declaracion in declaraciones {
                    let (valor, control) = self.evaluar_con_entorno(declaracion, entorno.clone())?;
                    ultimo_valor = valor;
                    
                    // Propagar control de flujo
                    match control {
                        ControlFlujo::Ninguno => {},
                        _ => return Ok((ultimo_valor, control)),
                    }
                }
                
                Ok((ultimo_valor, ControlFlujo::Ninguno))
            },
            
            Nodo::LlamadaFuncion { nombre, argumentos, linea } => {
                // Incrementar profundidad antes de la llamada
                self.profundidad_recursion += 1;
                let resultado = self.evaluar_llamada_funcion(nombre, argumentos, *linea, entorno);
                self.profundidad_recursion -= 1;
                resultado
            },
            
            Nodo::LlamadaMetodo { objeto, metodo, argumentos, linea } => {
                // Evaluar argumentos primero
                let mut args_evaluados = Vec::new();
                for arg in argumentos {
                    let (valor_arg, _) = self.evaluar_con_entorno(arg, entorno.clone())?;
                    args_evaluados.push(valor_arg);
                }
                
                // Métodos que modifican la variable original (como agregar)
                if metodo == "agregar" || metodo == "quitar" || metodo == "limpiar" {
                    match objeto.as_ref() {
                        Nodo::Identificador(nombre_var) => {
                            return self.evaluar_metodo_mutante(nombre_var, metodo, &args_evaluados, *linea, entorno);
                        },
                        Nodo::AccesoIndice { objeto: objeto_padre, indice, linea: _ } => {
                            return self.evaluar_metodo_mutante_en_indice(objeto_padre, indice, metodo, &args_evaluados, *linea, entorno);
                        },
                        _ => {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea: *linea,
                                mensaje: format!("El método '{}' solo se puede llamar en variables o elementos de listas", metodo),
                            });
                        }
                    }
                }
                
                // Métodos que no modifican (como longitud, primero, ultimo, etc.)
                let (valor_objeto, _) = self.evaluar_con_entorno(objeto, entorno.clone())?;
                self.evaluar_metodo_en_valor(&valor_objeto, metodo, &args_evaluados, *linea)
            },
            
            Nodo::AccesoMiembro { objeto, miembro, linea } => {
                // Verificar si es una llamada a consola
                if let Nodo::Identificador(nombre_objeto) = objeto.as_ref() {
                    if nombre_objeto == "consola" {
                        // Generar nombre de función de consola
                        let nombre_funcion = format!("consola.{}", miembro);
                        return self.evaluar_llamada_funcion(&nombre_funcion, &[], *linea, entorno);
                    }
                }
                
                let (valor_objeto, _) = self.evaluar_con_entorno(objeto, entorno.clone())?;
                self.evaluar_acceso_miembro(&valor_objeto, miembro, *linea)
            },
            
            Nodo::Lista(elementos) => {
                let mut valores = Vec::new();
                for elemento in elementos {
                    let (valor, _) = self.evaluar_con_entorno(elemento, entorno.clone())?;
                    valores.push(valor);
                }
                Ok((Valor::Lista(valores), ControlFlujo::Ninguno))
            },
            
            Nodo::ObjetoJson(propiedades) => {
                let mut objeto = HashMap::new();
                for (clave, valor_nodo) in propiedades {
                    let (valor, _) = self.evaluar_con_entorno(valor_nodo, entorno.clone())?;
                    objeto.insert(clave.clone(), valor);
                }
                Ok((Valor::Json(objeto), ControlFlujo::Ninguno))
            },
            
            Nodo::Retornar { valor, linea } => {
                // Verificar que estemos dentro de una función
                if !self.dentro_de_funcion {
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea: *linea,
                        mensaje: "La declaración 'retornar' solo puede usarse dentro de una función".to_string(),
                    });
                }
                
                let valor_retorno = if let Some(expr) = valor {
                    let (valor_evaluado, _) = self.evaluar_con_entorno(expr, entorno)?;
                    valor_evaluado
                } else {
                    Valor::Vacio
                };
                
                Ok((valor_retorno.clone(), ControlFlujo::Retornar(valor_retorno)))
            },
            
            Nodo::Asignacion { nombre, valor, linea } => {
                // Evaluar el valor a asignar
                let (valor_evaluado, _) = self.evaluar_con_entorno(valor, entorno.clone())?;
                
                // Verificar si la variable existe
                let variable_existente = entorno.borrow().obtener_variable(nombre);
                
                if let Some(var_actual) = variable_existente {
                    // Variable existe - validar mutabilidad
                    if matches!(var_actual.tipo_variable, TipoVariable::Inmutable) {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea: *linea,
                            mensaje: format!("No se puede reasignar la variable inmutable '{}'", nombre),
                        });
                    }
                    
                    // Validar compatibilidad de tipos
                    if !self.validar_tipo_compatible(&valor_evaluado, &var_actual.tipo_dato) {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea: *linea,
                            mensaje: format!("Tipo incompatible: no se puede asignar {} a variable de tipo {}", 
                                self.obtener_nombre_tipo(&valor_evaluado), var_actual.tipo_dato),
                        });
                    }
                    
                    // Actualizar variable existente directamente
                    let nueva_variable = Variable::nueva(
                        nombre.clone(),
                        valor_evaluado.clone(),
                        var_actual.tipo_variable,
                        var_actual.tipo_dato.clone(),
                    );
                    entorno.borrow_mut().variables.insert(nombre.clone(), nueva_variable);
                } else {
                    // Variable no existe - crear nueva (inmutable por defecto)
                    let variable = Variable::nueva(
                        nombre.clone(),
                        valor_evaluado.clone(),
                        TipoVariable::Inmutable,
                        "auto".to_string()
                    );
                    entorno.borrow_mut().definir_variable(nombre.clone(), variable)?;
                }
                
                Ok((valor_evaluado, ControlFlujo::Ninguno))
            },
            
            Nodo::AsignacionCompuesta { nombre, operador, valor, linea } => {
                // Obtener el valor actual de la variable
                let valor_actual = {
                    let entorno_ref = entorno.borrow();
                    if let Some(variable) = entorno_ref.obtener_variable(nombre) {
                        variable.valor.clone()
                    } else {
                        return Err(ErrorQuetzal::VariableNoDefinida {
                            linea: *linea,
                            nombre: nombre.clone(),
                        });
                    }
                };
                
                // Evaluar el valor a asignar
                let (nuevo_valor, _) = self.evaluar_con_entorno(valor, entorno.clone())?;
                
                // Realizar la operación compuesta
                let operador_base = &operador[0..operador.len()-1]; // Quitar '=' del final
                let resultado = self.evaluar_operacion_binaria(&valor_actual, operador_base, &nuevo_valor)?;
                
                // Asignar el resultado (las asignaciones compuestas funcionan en variables inmutables)
                {
                    let mut entorno_ref = entorno.borrow_mut();
                    if let Some(variable) = entorno_ref.variables.get_mut(nombre) {
                        // En Quetzal, las asignaciones compuestas (+=, -=, etc.) funcionan incluso en variables inmutables
                        variable.valor = resultado.clone();
                    } else {
                        return Err(ErrorQuetzal::VariableNoDefinida {
                            linea: *linea,
                            nombre: nombre.clone(),
                        });
                    }
                }
                
                Ok((resultado, ControlFlujo::Ninguno))
            },
            
            Nodo::AsignacionIndice { objeto, indice, valor, linea } => {
                // Evaluar el valor que se va a asignar
                let (nuevo_valor, _) = self.evaluar_con_entorno(valor, entorno.clone())?;
                
                // Evaluar el índice
                let (valor_indice, _) = self.evaluar_con_entorno(indice, entorno.clone())?;
                
                // Verificar que el índice sea un entero
                let indice_usize = match valor_indice {
                    Valor::Entero(i) => {
                        if i < 0 {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea: *linea,
                                mensaje: format!("Índice negativo: {}", i),
                            });
                        }
                        i as usize
                    },
                    _ => {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea: *linea,
                            mensaje: "El índice debe ser un número entero".to_string(),
                        });
                    }
                };
                
                // Función auxiliar para asignar a índice anidado
                self.asignar_indice_recursivo(objeto, indice_usize, nuevo_valor.clone(), entorno, *linea)?;
                Ok((nuevo_valor, ControlFlujo::Ninguno))
            },
            
            Nodo::AsignacionPropiedad { objeto, propiedad, valor, linea } => {
                // Evaluar el valor que se va a asignar
                let (nuevo_valor, _) = self.evaluar_con_entorno(valor, entorno.clone())?;
                
                // Verificar que el objeto sea un identificador (variable)
                if let Nodo::Identificador(nombre_objeto) = objeto.as_ref() {
                    // Obtener la variable del entorno
                    let entorno_ref = entorno.borrow();
                    if let Some(variable) = entorno_ref.obtener_variable(nombre_objeto) {
                        // Verificar que la variable sea mutable
                        if matches!(variable.tipo_variable, TipoVariable::Inmutable) {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea: *linea,
                                mensaje: format!("No se puede modificar propiedades del objeto inmutable '{}'", nombre_objeto),
                            });
                        }
                        
                        // Verificar que la variable sea un objeto JSON
                        if let Valor::Json(mut mapa) = variable.valor.clone() {
                            // Asignar la nueva propiedad
                            mapa.insert(propiedad.clone(), nuevo_valor.clone());
                            
                            // Actualizar la variable en el entorno
                            drop(entorno_ref); // Liberar la referencia inmutable
                            let nueva_variable = Variable::nueva(
                                nombre_objeto.clone(),
                                Valor::Json(mapa),
                                variable.tipo_variable,
                                variable.tipo_dato.clone(),
                            );
                            entorno.borrow_mut().variables.insert(nombre_objeto.clone(), nueva_variable);
                            
                            Ok((nuevo_valor, ControlFlujo::Ninguno))
                        } else {
                            Err(ErrorQuetzal::ErrorEjecucion {
                                linea: *linea,
                                mensaje: format!("No se puede asignar propiedades a una variable de tipo '{}', debe ser un objeto JSON", 
                                    self.obtener_nombre_tipo(&variable.valor)),
                            })
                        }
                    } else {
                        Err(ErrorQuetzal::VariableNoDefinida {
                            linea: *linea,
                            nombre: nombre_objeto.clone(),
                        })
                    }
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea: *linea,
                        mensaje: "Solo se pueden asignar propiedades a variables, no a expresiones complejas".to_string(),
                    })
                }
            },
            
            Nodo::BuclePara { inicializacion, condicion, incremento, cuerpo, linea: _ } => {
                // Crear nuevo entorno para el bucle
                let entorno_bucle = Rc::new(RefCell::new(Entorno::con_padre(entorno.clone())));
                
                // Ejecutar inicialización si existe
                if let Some(init) = inicializacion {
                    self.evaluar_con_entorno(init, entorno_bucle.clone())?;
                }
                
                loop {
                    // Evaluar condición si existe
                    if let Some(cond) = condicion {
                        let (valor_condicion, _) = self.evaluar_con_entorno(cond, entorno_bucle.clone())?;
                        if !valor_condicion.a_bool() {
                            break;
                        }
                    }
                    
                    // Crear nuevo entorno para cada iteración del bucle
                    let entorno_iteracion = Rc::new(RefCell::new(Entorno::con_padre(entorno_bucle.clone())));
                    
                    // Ejecutar cuerpo del bucle en el entorno de iteración
                    let (_, control) = self.evaluar_con_entorno(cuerpo, entorno_iteracion)?;
                    
                    match control {
                        ControlFlujo::Romper => break,
                        ControlFlujo::Continuar => {
                            // Ejecutar incremento antes de continuar
                            if let Some(inc) = incremento {
                                self.evaluar_con_entorno(inc, entorno_bucle.clone())?;
                            }
                            continue;
                        },
                        ControlFlujo::Retornar(_) => return Ok((Valor::Vacio, control)),
                        _ => {},
                    }
                    
                    // Ejecutar incremento al final de cada iteración
                    if let Some(inc) = incremento {
                        self.evaluar_con_entorno(inc, entorno_bucle.clone())?;
                    }
                }
                
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            },
            
            Nodo::BucleParaCada { variable, iterable, cuerpo, linea: _ } => {
                let (valor_iterable, _) = self.evaluar_con_entorno(iterable, entorno.clone())?;
                
                match valor_iterable {
                    Valor::Lista(elementos) => {
                        // Crear nuevo entorno para el bucle
                        let entorno_bucle = Rc::new(RefCell::new(Entorno::con_padre(entorno.clone())));
                        
                        for elemento in elementos {
                            // Crear nuevo entorno para cada iteración del bucle para_cada
                            let entorno_iteracion = Rc::new(RefCell::new(Entorno::con_padre(entorno_bucle.clone())));
                            
                            // Definir la variable del bucle en cada iteración
                            let variable_bucle = Variable::nueva(
                                variable.clone(),
                                elemento,
                                TipoVariable::Inmutable,
                                "auto".to_string(),
                            );
                            entorno_iteracion.borrow_mut().definir_variable(variable.clone(), variable_bucle)?;
                            
                            // Ejecutar el cuerpo del bucle en el entorno de iteración
                            let (_, control) = self.evaluar_con_entorno(cuerpo, entorno_iteracion)?;
                            
                            match control {
                                ControlFlujo::Romper => break,
                                ControlFlujo::Continuar => continue,
                                ControlFlujo::Retornar(_) => return Ok((Valor::Vacio, control)),
                                _ => {},
                            }
                        }
                        
                        Ok((Valor::Vacio, ControlFlujo::Ninguno))
                    },
                    // TODO: Implementar rangos numéricos para bucles para
                    _ => Err(ErrorQuetzal::ErrorTipo {
                        linea: 0,
                        mensaje: "El bucle para solo acepta listas por ahora".to_string(),
                    }),
                }
            },
            
            Nodo::BucleMientras { condicion, cuerpo, linea: _ } => {
                loop {
                    let (valor_condicion, _) = self.evaluar_con_entorno(condicion, entorno.clone())?;
                    
                    if !valor_condicion.a_bool() {
                        break;
                    }
                    
                    let (_, control) = self.evaluar_con_entorno(cuerpo, entorno.clone())?;
                    
                    match control {
                        ControlFlujo::Romper => break,
                        ControlFlujo::Continuar => continue,
                        ControlFlujo::Retornar(_) => return Ok((Valor::Vacio, control)),
                        _ => {},
                    }
                }
                
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            },
            
            Nodo::BucleHacerMientras { cuerpo, condicion, linea: _ } => {
                loop {
                    // Ejecutar el cuerpo al menos una vez
                    let (_, control) = self.evaluar_con_entorno(cuerpo, entorno.clone())?;
                    
                    match control {
                        ControlFlujo::Romper => break,
                        ControlFlujo::Continuar => {
                            // Evaluar condición antes de continuar
                            let (valor_condicion, _) = self.evaluar_con_entorno(condicion, entorno.clone())?;
                            if !valor_condicion.a_bool() {
                                break;
                            }
                            continue;
                        },
                        ControlFlujo::Retornar(_) => return Ok((Valor::Vacio, control)),
                        _ => {},
                    }
                    
                    // Evaluar condición para decidir si continuar
                    let (valor_condicion, _) = self.evaluar_con_entorno(condicion, entorno.clone())?;
                    if !valor_condicion.a_bool() {
                        break;
                    }
                }
                
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            },
            
            Nodo::Romper { linea: _ } => {
                Ok((Valor::Vacio, ControlFlujo::Romper))
            },
            
            Nodo::Continuar { linea: _ } => {
                Ok((Valor::Vacio, ControlFlujo::Continuar))
            },
            
            Nodo::AccesoIndice { objeto, indice, linea } => {
                let (valor_objeto, _) = self.evaluar_con_entorno(objeto, entorno.clone())?;
                let (valor_indice, _) = self.evaluar_con_entorno(indice, entorno)?;
                
                match (&valor_objeto, &valor_indice) {
                    (Valor::Lista(lista), Valor::Entero(i)) => {
                        let indice_usize = if *i < 0 {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea: *linea,
                                mensaje: format!("Índice negativo: {}", i),
                            });
                        } else {
                            *i as usize
                        };
                        
                        if indice_usize >= lista.len() {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea: *linea,
                                mensaje: format!("Índice fuera de rango: {} (tamaño: {})", indice_usize, lista.len()),
                            });
                        }
                        
                        Ok((lista[indice_usize].clone(), ControlFlujo::Ninguno))
                    },
                    
                    (Valor::Cadena(cadena), Valor::Entero(i)) => {
                        let indice_usize = if *i < 0 {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea: *linea,
                                mensaje: format!("Índice negativo: {}", i),
                            });
                        } else {
                            *i as usize
                        };
                        
                        let chars: Vec<char> = cadena.chars().collect();
                        if indice_usize >= chars.len() {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea: *linea,
                                mensaje: format!("Índice fuera de rango: {} (tamaño: {})", indice_usize, chars.len()),
                            });
                        }
                        
                        Ok((Valor::Cadena(chars[indice_usize].to_string()), ControlFlujo::Ninguno))
                    },
                    
                    (Valor::Json(mapa), Valor::Cadena(clave)) => {
                        if let Some(valor) = mapa.get(clave) {
                            Ok((valor.clone(), ControlFlujo::Ninguno))
                        } else {
                            Err(ErrorQuetzal::ErrorEjecucion {
                                linea: *linea,
                                mensaje: format!("La propiedad '{}' no existe en el objeto", clave),
                            })
                        }
                    },
                    
                    _ => Err(ErrorQuetzal::ErrorEjecucion {
                        linea: *linea,
                        mensaje: format!("No se puede acceder por índice a un {} usando {}", 
                                         valor_objeto.tipo_como_cadena(), 
                                         valor_indice.tipo_como_cadena()),
                    }),
                }
            },
            
            Nodo::OperadorTernario { condicion, valor_verdadero, valor_falso } => {
                let (cond_evaluada, _) = self.evaluar_con_entorno(condicion, entorno.clone())?;
                
                // Verificar si la condición es verdadera
                let es_verdadero = match cond_evaluada {
                    Valor::Bool(b) => b,
                    Valor::Entero(n) => n != 0,
                    Valor::Numero(n) => n != 0.0,
                    Valor::Cadena(s) => !s.is_empty(),
                    Valor::Lista(lista) => !lista.is_empty(),
                    Valor::Vacio => false,
                    _ => true,
                };
                
                if es_verdadero {
                    self.evaluar_con_entorno(valor_verdadero, entorno)
                } else {
                    self.evaluar_con_entorno(valor_falso, entorno)
                }
            },
            
            _ => {
                // Otros nodos no implementados aún
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            }
        }
    }
    
    /// Evalúa una llamada a función
    fn evaluar_llamada_funcion(&mut self, nombre: &str, argumentos: &[Nodo], linea: usize, entorno: Rc<RefCell<Entorno>>) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        // Verificar recursión infinita
        if self.profundidad_recursion >= self.max_profundidad_recursion {
            return Err(ErrorQuetzal::ErrorEjecucion {
                linea,
                mensaje: format!("Recursión infinita detectada: profundidad máxima de {} excedida", self.max_profundidad_recursion),
            });
        }
        
        // Verificar funciones de consola
        if nombre.starts_with("consola.") {
            return self.evaluar_funcion_consola(nombre, argumentos, entorno);
        }
        
        // Funciones globales especiales
        match nombre {
            "imprimir" => {
                return self.evaluar_funcion_consola("consola.imprimir", argumentos, entorno);
            },
            "imprimir_error" => {
                return self.evaluar_funcion_consola("consola.imprimir_error", argumentos, entorno);
            },
            "imprimir_advertencia" => {
                return self.evaluar_funcion_consola("consola.imprimir_advertencia", argumentos, entorno);
            },
            "imprimir_informacion" => {
                return self.evaluar_funcion_consola("consola.imprimir_informacion", argumentos, entorno);
            },
            "imprimir_exito" => {
                return self.evaluar_funcion_consola("consola.imprimir_exito", argumentos, entorno);
            },
            "imprimir_depurar" => {
                return self.evaluar_funcion_consola("consola.imprimir_depurar", argumentos, entorno);
            },
            "imprimir_alerta" => {
                return self.evaluar_funcion_consola("consola.imprimir_alerta", argumentos, entorno);
            },
            "imprimir_confirmacion" => {
                return self.evaluar_funcion_consola("consola.imprimir_confirmacion", argumentos, entorno);
            },
            _ => {}
        }
        
        // Verificar si es un método encadenado de conversión (formato variable.metodo)
        if nombre.contains(".") && !nombre.starts_with("consola.") {
            // Separar variable.metodo
            let partes: Vec<&str> = nombre.split('.').collect();
            if partes.len() == 2 {
                let nombre_variable = partes[0];
                let nombre_metodo = partes[1];
                
                // Obtener el valor de la variable
                let valor_variable = {
                    let entorno_ref = entorno.borrow();
                    if let Some(variable) = entorno_ref.obtener_variable(nombre_variable) {
                        variable.valor.clone()
                    } else {
                        return Err(ErrorQuetzal::VariableNoDefinida {
                            linea,
                            nombre: nombre_variable.to_string(),
                        });
                    }
                };
                
                // Evaluar argumentos
                let mut args_evaluados = Vec::new();
                for arg in argumentos {
                    let (valor_arg, _) = self.evaluar_con_entorno(arg, entorno.clone())?;
                    args_evaluados.push(valor_arg);
                }
                
                // Verificar si es un método que modifica la variable (mutante)
                if nombre_metodo == "agregar" || nombre_metodo == "quitar" || nombre_metodo == "limpiar" {
                    return self.evaluar_metodo_mutante(nombre_variable, nombre_metodo, &args_evaluados, linea, entorno);
                }
                
                // Para métodos que no modifican, usar el sistema estándar
                return self.evaluar_metodo_en_valor(&valor_variable, nombre_metodo, &args_evaluados, linea);
            } else {
                // Para cadenas de métodos múltiples, usar el sistema anterior
                return self.evaluar_metodo_conversion(nombre, argumentos, entorno);
            }
        }
        
        // Buscar función definida por el usuario
        let funcion_opt = {
            entorno.borrow().obtener_funcion(nombre)
        };
        
        if let Some(funcion) = funcion_opt {
            // Evaluar argumentos
            let mut valores_argumentos = Vec::new();
            for argumento in argumentos {
                let (valor, _) = self.evaluar_con_entorno(argumento, entorno.clone())?;
                valores_argumentos.push(valor);
            }
            
            // Contar parámetros obligatorios (sin valor por defecto)
            let parametros_obligatorios = funcion.parametros.iter()
                .filter(|p| p.valor_defecto.is_none())
                .count();
            
            // Verificar número de argumentos
            if valores_argumentos.len() < parametros_obligatorios || valores_argumentos.len() > funcion.parametros.len() {
                return Err(ErrorQuetzal::ArgumentosIncorrectos {
                    linea,
                    esperados: funcion.parametros.len(),
                    recibidos: valores_argumentos.len(),
                });
            }
            
            // Crear entorno para la función
            let entorno_funcion = Rc::new(RefCell::new(Entorno::nuevo_hijo(entorno)));
            
            // Asignar parámetros
            for (i, parametro) in funcion.parametros.iter().enumerate() {
                let tipo_variable = if parametro.es_mutable {
                    TipoVariable::Mutable
                } else {
                    TipoVariable::Inmutable
                };
                
                // Determinar el valor a usar
                let valor = if i < valores_argumentos.len() {
                    // Usar argumento proporcionado
                    valores_argumentos[i].clone()
                } else if let Some(valor_defecto) = &parametro.valor_defecto {
                    // Usar valor por defecto
                    valor_defecto.clone()
                } else {
                    return Err(ErrorQuetzal::ArgumentosIncorrectos {
                        linea,
                        esperados: parametros_obligatorios,
                        recibidos: valores_argumentos.len(),
                    });
                };
                
                let variable = Variable::nueva(
                    parametro.nombre.clone(),
                    valor,
                    tipo_variable,
                    parametro.tipo_dato.clone(),
                );
                
                entorno_funcion.borrow_mut().definir_variable(parametro.nombre.clone(), variable)?;
            }
            
            // Ejecutar cuerpo de la función
            let estado_anterior = self.dentro_de_funcion;
            self.dentro_de_funcion = true;
            let resultado = self.evaluar_con_entorno(&funcion.cuerpo, entorno_funcion);
            self.dentro_de_funcion = estado_anterior;
            
            let (valor, control) = resultado?;
            
            match control {
                ControlFlujo::Retornar(valor_retorno) => Ok((valor_retorno, ControlFlujo::Ninguno)),
                _ => {
                    // Si no hay retorno explícito y la función no es de tipo vacio, es un error
                    if funcion.tipo_retorno != "vacio" {
                        return Err(ErrorQuetzal::ErrorSintaxis {
                            linea,
                            mensaje: format!("La función '{}' debe retornar un valor de tipo '{}'", nombre, funcion.tipo_retorno),
                        });
                    }
                    Ok((valor, ControlFlujo::Ninguno))
                }
            }
        } else {
            Err(ErrorQuetzal::FuncionNoDefinida {
                linea,
                nombre: nombre.to_string(),
            })
        }
    }
    
    /// Evalúa funciones de consola
    fn evaluar_funcion_consola(&mut self, nombre: &str, argumentos: &[Nodo], entorno: Rc<RefCell<Entorno>>) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        match nombre {
            "consola.imprimir" => {
                // Evaluar primer argumento (mensaje)
                let mensaje = if !argumentos.is_empty() {
                    let (valor, _) = self.evaluar_con_entorno(&argumentos[0], entorno)?;
                    valor.a_cadena()
                } else {
                    String::new()
                };
                CONSOLA_GLOBAL.imprimir(&mensaje);
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            },
            "consola.imprimir_error" => {
                let mensaje = if !argumentos.is_empty() {
                    let (valor, _) = self.evaluar_con_entorno(&argumentos[0], entorno)?;
                    valor.a_cadena()
                } else {
                    String::new()
                };
                CONSOLA_GLOBAL.imprimir_error(&mensaje);
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            },
            "consola.imprimir_advertencia" => {
                let mensaje = if !argumentos.is_empty() {
                    let (valor, _) = self.evaluar_con_entorno(&argumentos[0], entorno)?;
                    valor.a_cadena()
                } else {
                    String::new()
                };
                CONSOLA_GLOBAL.imprimir_advertencia(&mensaje);
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            },
            "consola.imprimir_informacion" => {
                let mensaje = if !argumentos.is_empty() {
                    let (valor, _) = self.evaluar_con_entorno(&argumentos[0], entorno)?;
                    valor.a_cadena()
                } else {
                    String::new()
                };
                CONSOLA_GLOBAL.imprimir_informacion(&mensaje);
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            },
            "consola.imprimir_depurar" => {
                let mensaje = if !argumentos.is_empty() {
                    let (valor, _) = self.evaluar_con_entorno(&argumentos[0], entorno)?;
                    valor.a_cadena()
                } else {
                    String::new()
                };
                CONSOLA_GLOBAL.imprimir_depurar(&mensaje);
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            },
            "consola.imprimir_exito" => {
                let mensaje = if !argumentos.is_empty() {
                    let (valor, _) = self.evaluar_con_entorno(&argumentos[0], entorno)?;
                    valor.a_cadena()
                } else {
                    String::new()
                };
                CONSOLA_GLOBAL.imprimir_exito(&mensaje);
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            },
            "consola.imprimir_alerta" => {
                let mensaje = if !argumentos.is_empty() {
                    let (valor, _) = self.evaluar_con_entorno(&argumentos[0], entorno)?;
                    valor.a_cadena()
                } else {
                    String::new()
                };
                CONSOLA_GLOBAL.imprimir_alerta(&mensaje);
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            },
            "consola.imprimir_confirmacion" => {
                let mensaje = if !argumentos.is_empty() {
                    let (valor, _) = self.evaluar_con_entorno(&argumentos[0], entorno)?;
                    valor.a_cadena()
                } else {
                    String::new()
                };
                CONSOLA_GLOBAL.imprimir_confirmacion(&mensaje);
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            },
            "consola.pedir" => {
                let mensaje = if !argumentos.is_empty() {
                    let (valor, _) = self.evaluar_con_entorno(&argumentos[0], entorno)?;
                    valor.a_cadena()
                } else {
                    String::new()
                };
                let entrada = CONSOLA_GLOBAL.pedir(&mensaje);
                Ok((Valor::Cadena(entrada), ControlFlujo::Ninguno))
            },
            "consola.pedir_secreto" => {
                let mensaje = if !argumentos.is_empty() {
                    let (valor, _) = self.evaluar_con_entorno(&argumentos[0], entorno)?;
                    valor.a_cadena()
                } else {
                    String::new()
                };
                let entrada_secreta = CONSOLA_GLOBAL.pedir_secreto(&mensaje);
                Ok((Valor::Cadena(entrada_secreta), ControlFlujo::Ninguno))
            },
            _ => {
                return Err(ErrorQuetzal::FuncionNoDefinida {
                    linea: 0,
                    nombre: nombre.to_string(),
                });
            }
        }
    }
    
    /// Evalúa método de conversión en cadena
    fn evaluar_metodo_conversion(&mut self, nombre_completo: &str, argumentos: &[Nodo], entorno: Rc<RefCell<Entorno>>) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        // Manejar métodos encadenados con expresiones temporales
        if nombre_completo.starts_with("expr_temp.") {
            // Para expresiones temporales, necesitamos evaluar de forma diferente
            let metodo = nombre_completo.replace("expr_temp.", "");
            
            // Evaluar argumentos
            let mut args_evaluados = Vec::new();
            for arg in argumentos {
                let (valor_arg, _) = self.evaluar_con_entorno(arg, entorno.clone())?;
                args_evaluados.push(valor_arg);
            }
            
            // HACK: En el contexto actual, no tenemos acceso a la expresión original.
            // Como solución temporal, buscaremos en el entorno una variable temporal especial
            // que almacene el resultado de la expresión recién evaluada.
            
            // Por ahora, retornaremos un error descriptivo, pero en una implementación completa
            // necesitaríamos reestructurar el AST para manejar esto adecuadamente.
            return Err(ErrorQuetzal::ErrorEjecucion {
                linea: 0,
                mensaje: format!("Método '{}' en expresión temporal no soportado aún", metodo),
            });
        }
        
        let partes: Vec<&str> = nombre_completo.split('.').collect();
        
        // Manejar cadenas de métodos múltiples (ej: variable.metodo1.metodo2)
        if partes.len() > 2 {
            // Para métodos encadenados, evaluar paso a paso
            let nombre_variable = partes[0];
            
            // Obtener valor inicial
            let mut valor_actual = {
                let entorno_ref = entorno.borrow();
                if let Some(variable) = entorno_ref.obtener_variable(nombre_variable) {
                    variable.valor.clone()
                } else {
                    return Err(ErrorQuetzal::VariableNoDefinida {
                        linea: 0,
                        nombre: nombre_variable.to_string(),
                    });
                }
            };
            
            // Aplicar cada método en secuencia
            for i in 1..partes.len()-1 {
                let metodo = partes[i];
                valor_actual = match valor_actual {
                    Valor::Cadena(ref cadena) => {
                        let (resultado, _) = self.evaluar_metodo_cadena(cadena, metodo, &[])?;
                        resultado
                    },
                    _ => {
                        // Aplicar métodos de conversión
                        match metodo {
                            "cadena" => Valor::Cadena(valor_actual.a_cadena()),
                            "numero" => match valor_actual {
                                Valor::Entero(n) => Valor::Numero(n as f64),
                                Valor::Numero(n) => Valor::Numero(n),
                                Valor::Cadena(s) => {
                                    let trimmed = s.trim();
                                    
                                    // Intentar directamente como f64 para permitir números decimales largos
                                    if let Ok(f) = trimmed.parse::<f64>() {
                                        Valor::Numero(f)
                                    } else {
                                        return Err(ErrorQuetzal::ErrorConversion {
                                            linea: 0,
                                            mensaje: "No se puede convertir cadena a número".to_string(),
                                        });
                                    }
                                },
                                _ => return Err(ErrorQuetzal::ErrorConversion {
                                    linea: 0,
                                    mensaje: format!("No se puede convertir {} a número", valor_actual.tipo_como_cadena()),
                                }),
                            },
                            "entero" => match valor_actual {
                                Valor::Entero(n) => Valor::Entero(n),
                                Valor::Numero(n) => Valor::Entero(n as i64),
                                Valor::Cadena(s) => {
                                    match s.trim().parse::<i64>() {
                                        Ok(n) => Valor::Entero(n),
                                        Err(_) => return Err(ErrorQuetzal::ErrorConversion {
                                            linea: 0,
                                            mensaje: "No se puede convertir cadena a entero".to_string(),
                                        }),
                                    }
                                },
                                _ => return Err(ErrorQuetzal::ErrorConversion {
                                    linea: 0,
                                    mensaje: format!("No se puede convertir {} a entero", valor_actual.tipo_como_cadena()),
                                }),
                            },
                            "bool" => Valor::Bool(valor_actual.a_bool()),
                            _ => return Err(ErrorQuetzal::ErrorEjecucion {
                                linea: 0,
                                mensaje: format!("Método '{}' no está definido", metodo),
                            }),
                        }
                    }
                };
            }
            
            // Aplicar el último método con argumentos
            let ultimo_metodo = partes[partes.len()-1];
            
            // Evaluar argumentos
            let mut args_evaluados = Vec::new();
            for arg in argumentos {
                let (valor_arg, _) = self.evaluar_con_entorno(arg, entorno.clone())?;
                args_evaluados.push(valor_arg);
            }
            
            return match valor_actual {
                Valor::Cadena(ref cadena) => self.evaluar_metodo_cadena(cadena, ultimo_metodo, &args_evaluados),
                _ => {
                    // Aplicar método de conversión final
                    match ultimo_metodo {
                        "cadena" => Ok((Valor::Cadena(valor_actual.a_cadena()), ControlFlujo::Ninguno)),
                        "numero" => match valor_actual {
                            Valor::Entero(n) => Ok((Valor::Numero(n as f64), ControlFlujo::Ninguno)),
                            Valor::Numero(n) => Ok((Valor::Numero(n), ControlFlujo::Ninguno)),
                            Valor::Cadena(s) => {
                                let trimmed = s.trim();
                                
                                match trimmed.parse::<f64>() {
                                    Ok(n) => {
                                        // Verificar si el número es finito y está en un rango seguro
                                        if n.is_finite() && !n.is_infinite() && !n.is_nan() {
                                            Ok((Valor::Numero(n), ControlFlujo::Ninguno))
                                        } else {
                                            Err(ErrorQuetzal::ErrorConversion {
                                                linea: 0,
                                                mensaje: "Número fuera del rango representable".to_string(),
                                            })
                                        }
                                    },
                                    Err(_) => Err(ErrorQuetzal::ErrorConversion {
                                        linea: 0,
                                        mensaje: "No se puede convertir cadena a número".to_string(),
                                    }),
                                }
                            },
                            _ => Err(ErrorQuetzal::ErrorConversion {
                                linea: 0,
                                mensaje: format!("No se puede convertir {} a número", valor_actual.tipo_como_cadena()),
                            }),
                        },
                        "entero" => match valor_actual {
                            Valor::Entero(n) => Ok((Valor::Entero(n), ControlFlujo::Ninguno)),
                            Valor::Numero(n) => Ok((Valor::Entero(n as i64), ControlFlujo::Ninguno)),
                            Valor::Cadena(s) => {
                                match s.trim().parse::<i64>() {
                                    Ok(n) => Ok((Valor::Entero(n), ControlFlujo::Ninguno)),
                                    Err(_) => Err(ErrorQuetzal::ErrorConversion {
                                        linea: 0,
                                        mensaje: "No se puede convertir cadena a entero".to_string(),
                                    }),
                                }
                            },
                            _ => Err(ErrorQuetzal::ErrorConversion {
                                linea: 0,
                                mensaje: format!("No se puede convertir {} a entero", valor_actual.tipo_como_cadena()),
                            }),
                        },
                        "bool" => Ok((Valor::Bool(valor_actual.a_bool()), ControlFlujo::Ninguno)),
                        _ => Err(ErrorQuetzal::ErrorEjecucion {
                            linea: 0,
                            mensaje: format!("Método '{}' no está definido", ultimo_metodo),
                        }),
                    }
                }
            };
        }
        
        if partes.len() != 2 {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: 0,
                mensaje: "Formato de método inválido".to_string(),
            });
        }
        
        let nombre_variable = partes[0];
        let metodo = partes[1];
        
        // Obtener valor de la variable
        let valor = {
            let entorno_ref = entorno.borrow();
            if let Some(variable) = entorno_ref.obtener_variable(nombre_variable) {
                variable.valor.clone()
            } else {
                return Err(ErrorQuetzal::VariableNoDefinida {
                    linea: 0,
                    nombre: nombre_variable.to_string(),
                });
            }
        };
        
        // Evaluar argumentos
        let mut args_evaluados = Vec::new();
        for arg in argumentos {
            let (valor_arg, _) = self.evaluar_con_entorno(arg, entorno.clone())?;
            args_evaluados.push(valor_arg);
        }
        
        // Aplicar método
        match metodo {
            // Métodos de conversión (sin argumentos)
            "cadena" => Ok((Valor::Cadena(valor.a_cadena()), ControlFlujo::Ninguno)),
            "numero" => {
                match valor {
                    Valor::Entero(n) => Ok((Valor::Numero(n as f64), ControlFlujo::Ninguno)),
                    Valor::Numero(n) => Ok((Valor::Numero(n), ControlFlujo::Ninguno)),
                    Valor::Cadena(s) => {
                        let trimmed = s.trim();
                        
                        // Intentar directamente como f64 para permitir números decimales largos
                        if let Ok(f) = trimmed.parse::<f64>() {
                            if f.is_finite() && !f.is_infinite() && !f.is_nan() {
                                // Verificar que no hayamos perdido precisión significativa
                                // convirtiendo de vuelta a string y comparando
                                let _back_to_string = f.to_string();
                                let original_cleaned = trimmed.trim_start_matches("0").trim_start_matches(".");
                                if original_cleaned.len() > 15 || (f.is_infinite() || f.abs() >= 1e15) {
                                    Err(ErrorQuetzal::ErrorConversion {
                                        linea: 0,
                                        mensaje: "Número demasiado grande para representar con precisión".to_string(),
                                    })
                                } else {
                                    Ok((Valor::Numero(f), ControlFlujo::Ninguno))
                                }
                            } else {
                                Err(ErrorQuetzal::ErrorConversion {
                                    linea: 0,
                                    mensaje: "Número fuera del rango representable".to_string(),
                                })
                            }
                        } else {
                            Err(ErrorQuetzal::ErrorConversion {
                                linea: 0,
                                mensaje: "No se puede convertir cadena a número".to_string(),
                            })
                        }
                    },
                    _ => Err(ErrorQuetzal::ErrorConversion {
                        linea: 0,
                        mensaje: format!("No se puede convertir {} a número", valor.tipo_como_cadena()),
                    }),
                }
            },
            "entero" => {
                match valor {
                    Valor::Entero(n) => Ok((Valor::Entero(n), ControlFlujo::Ninguno)),
                    Valor::Numero(n) => Ok((Valor::Entero(n as i64), ControlFlujo::Ninguno)),
                    Valor::Cadena(s) => {
                        match s.trim().parse::<i64>() {
                            Ok(n) => Ok((Valor::Entero(n), ControlFlujo::Ninguno)),
                            Err(_) => Err(ErrorQuetzal::ErrorConversion {
                                linea: 0,
                                mensaje: "No se puede convertir cadena a entero".to_string(),
                            }),
                        }
                    },
                    _ => Err(ErrorQuetzal::ErrorConversion {
                        linea: 0,
                        mensaje: format!("No se puede convertir {} a entero", valor.tipo_como_cadena()),
                    }),
                }
            },
            "bool" => {
                Ok((Valor::Bool(valor.a_bool()), ControlFlujo::Ninguno))
            },
            
            // Métodos de cadenas avanzadas
            _ => {
                match &valor {
                    Valor::Cadena(cadena) => self.evaluar_metodo_cadena(cadena, metodo, &args_evaluados),
                    _ => Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: format!("El método '{}' solo es válido para cadenas", metodo),
                    }),
                }
            }
        }
    }
    
    /// Evalúa métodos específicos de cadenas
    fn evaluar_metodo_cadena(&self, cadena: &str, metodo: &str, argumentos: &[Valor]) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        match metodo {
            // Métodos de conversión
            "cadena" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'cadena' no acepta argumentos".to_string(),
                    });
                }
                Ok((Valor::Cadena(cadena.to_string()), ControlFlujo::Ninguno))
            },
            
            "numero" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'numero' no acepta argumentos".to_string(),
                    });
                }
                let trimmed = cadena.trim();
                
                match trimmed.parse::<f64>() {
                    Ok(n) => {
                        if n.is_finite() && !n.is_infinite() && !n.is_nan() {
                            Ok((Valor::Numero(n), ControlFlujo::Ninguno))
                        } else {
                            Err(ErrorQuetzal::ErrorConversion {
                                linea: 0,
                                mensaje: "Número fuera del rango representable".to_string(),
                            })
                        }
                    },
                    Err(_) => Err(ErrorQuetzal::ErrorConversion {
                        linea: 0,
                        mensaje: "No se puede convertir cadena a número".to_string(),
                    }),
                }
            },
            
            "entero" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'entero' no acepta argumentos".to_string(),
                    });
                }
                match cadena.trim().parse::<i64>() {
                    Ok(n) => Ok((Valor::Entero(n), ControlFlujo::Ninguno)),
                    Err(_) => Err(ErrorQuetzal::ErrorConversion {
                        linea: 0,
                        mensaje: "No se puede convertir cadena a entero".to_string(),
                    }),
                }
            },
            
            "bool" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'bool' no acepta argumentos".to_string(),
                    });
                }
                let valor_bool = match cadena.to_lowercase().as_str() {
                    "true" | "verdadero" | "1" => true,
                    "false" | "falso" | "0" => false,
                    _ => !cadena.is_empty(),
                };
                Ok((Valor::Bool(valor_bool), ControlFlujo::Ninguno))
            },
            
            // Métodos específicos de cadenas
            "longitud" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'longitud' no acepta argumentos".to_string(),
                    });
                }
                Ok((Valor::Entero(cadena.len() as i64), ControlFlujo::Ninguno))
            },
            
            "esta_vacia" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'esta_vacia' no acepta argumentos".to_string(),
                    });
                }
                Ok((Valor::Bool(cadena.is_empty()), ControlFlujo::Ninguno))
            },
            
            "contiene" => {
                if argumentos.len() != 1 {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'contiene' requiere exactamente un argumento".to_string(),
                    });
                }
                let patron = argumentos[0].a_cadena();
                Ok((Valor::Bool(cadena.contains(&patron)), ControlFlujo::Ninguno))
            },
            
            "buscar" => {
                if argumentos.len() != 1 {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'buscar' requiere exactamente un argumento".to_string(),
                    });
                }
                let patron = argumentos[0].a_cadena();
                match cadena.find(&patron) {
                    Some(pos) => Ok((Valor::Entero(pos as i64), ControlFlujo::Ninguno)),
                    None => Ok((Valor::Entero(-1), ControlFlujo::Ninguno)),
                }
            },
            
            "empieza_con" => {
                if argumentos.len() != 1 {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'empieza_con' requiere exactamente un argumento".to_string(),
                    });
                }
                let prefijo = argumentos[0].a_cadena();
                Ok((Valor::Bool(cadena.starts_with(&prefijo)), ControlFlujo::Ninguno))
            },
            
            "termina_con" => {
                if argumentos.len() != 1 {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'termina_con' requiere exactamente un argumento".to_string(),
                    });
                }
                let sufijo = argumentos[0].a_cadena();
                Ok((Valor::Bool(cadena.ends_with(&sufijo)), ControlFlujo::Ninguno))
            },
            
            "reemplazar" => {
                if argumentos.len() != 2 {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'reemplazar' requiere exactamente dos argumentos".to_string(),
                    });
                }
                let buscar = argumentos[0].a_cadena();
                let reemplazar = argumentos[1].a_cadena();
                Ok((Valor::Cadena(cadena.replace(&buscar, &reemplazar)), ControlFlujo::Ninguno))
            },
            
            "subcadena" => {
                if argumentos.len() < 1 || argumentos.len() > 2 {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'subcadena' requiere 1 o 2 argumentos (inicio [, longitud])".to_string(),
                    });
                }
                
                let inicio = match &argumentos[0] {
                    Valor::Entero(i) => *i as usize,
                    _ => return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El primer argumento de 'subcadena' debe ser un entero".to_string(),
                    }),
                };
                
                if inicio >= cadena.len() {
                    return Ok((Valor::Cadena(String::new()), ControlFlujo::Ninguno));
                }
                
                let fin = if argumentos.len() == 2 {
                    let longitud = match &argumentos[1] {
                        Valor::Entero(l) => *l as usize,
                        _ => return Err(ErrorQuetzal::ErrorEjecucion {
                            linea: 0,
                            mensaje: "El segundo argumento de 'subcadena' debe ser un entero".to_string(),
                        }),
                    };
                    std::cmp::min(inicio + longitud, cadena.len())
                } else {
                    cadena.len()
                };
                
                let resultado = cadena.chars().skip(inicio).take(fin - inicio).collect::<String>();
                Ok((Valor::Cadena(resultado), ControlFlujo::Ninguno))
            },
            
            "dividir" => {
                if argumentos.len() != 1 {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'dividir' requiere exactamente un argumento".to_string(),
                    });
                }
                let delimitador = argumentos[0].a_cadena();
                if delimitador.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El delimitador no puede estar vacío".to_string(),
                    });
                }
                let partes: Vec<Valor> = cadena.split(&delimitador)
                    .map(|s| Valor::Cadena(s.to_string()))
                    .collect();
                Ok((Valor::Lista(partes), ControlFlujo::Ninguno))
            },
            
            "contar_ocurrencias" => {
                if argumentos.len() != 1 {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'contar_ocurrencias' requiere exactamente un argumento".to_string(),
                    });
                }
                let patron = argumentos[0].a_cadena();
                if patron.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El patrón no puede estar vacío".to_string(),
                    });
                }
                let count = cadena.matches(&patron).count() as i64;
                Ok((Valor::Entero(count), ControlFlujo::Ninguno))
            },
            
            "repetir" => {
                if argumentos.len() != 1 {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'repetir' requiere exactamente un argumento".to_string(),
                    });
                }
                let veces = match &argumentos[0] {
                    Valor::Entero(n) => {
                        if *n < 0 {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea: 0,
                                mensaje: "El número de repeticiones no puede ser negativo".to_string(),
                            });
                        }
                        *n as usize
                    },
                    _ => return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El argumento de 'repetir' debe ser un entero".to_string(),
                    }),
                };
                Ok((Valor::Cadena(cadena.repeat(veces)), ControlFlujo::Ninguno))
            },
            
            "a_mayusculas" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'a_mayusculas' no acepta argumentos".to_string(),
                    });
                }
                Ok((Valor::Cadena(cadena.to_uppercase()), ControlFlujo::Ninguno))
            },
            
            "a_minusculas" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'a_minusculas' no acepta argumentos".to_string(),
                    });
                }
                Ok((Valor::Cadena(cadena.to_lowercase()), ControlFlujo::Ninguno))
            },
            
            "recortar" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'recortar' no acepta argumentos".to_string(),
                    });
                }
                Ok((Valor::Cadena(cadena.trim().to_string()), ControlFlujo::Ninguno))
            },
            
            "invertir" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'invertir' no acepta argumentos".to_string(),
                    });
                }
                Ok((Valor::Cadena(cadena.chars().rev().collect()), ControlFlujo::Ninguno))
            },
            
            "comparar" => {
                if argumentos.len() != 1 {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'comparar' requiere exactamente un argumento".to_string(),
                    });
                }
                let otra_cadena = argumentos[0].a_cadena();
                let resultado = match cadena.cmp(&otra_cadena) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                };
                Ok((Valor::Entero(resultado), ControlFlujo::Ninguno))
            },
            
            "igual_sin_caso" => {
                if argumentos.len() != 1 {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'igual_sin_caso' requiere exactamente un argumento".to_string(),
                    });
                }
                let otra_cadena = argumentos[0].a_cadena();
                Ok((Valor::Bool(cadena.to_lowercase() == otra_cadena.to_lowercase()), ControlFlujo::Ninguno))
            },
            
            "codificar_base64" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'codificar_base64' no acepta argumentos".to_string(),
                    });
                }
                use base64::{Engine as _, engine::general_purpose};
                let resultado = general_purpose::STANDARD.encode(cadena.as_bytes());
                Ok((Valor::Cadena(resultado), ControlFlujo::Ninguno))
            },
            
            "decodificar_base64" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'decodificar_base64' no acepta argumentos".to_string(),
                    });
                }
                use base64::{Engine as _, engine::general_purpose};
                match general_purpose::STANDARD.decode(cadena) {
                    Ok(bytes) => match String::from_utf8(bytes) {
                        Ok(resultado) => Ok((Valor::Cadena(resultado), ControlFlujo::Ninguno)),
                        Err(_) => Err(ErrorQuetzal::ErrorEjecucion {
                            linea: 0,
                            mensaje: "Error al decodificar base64: datos no válidos".to_string(),
                        }),
                    },
                    Err(_) => Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "Error al decodificar base64: formato inválido".to_string(),
                    }),
                }
            },
            
            "codificar_uri" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'codificar_uri' no acepta argumentos".to_string(),
                    });
                }
                let resultado = urlencoding::encode(cadena).to_string();
                Ok((Valor::Cadena(resultado), ControlFlujo::Ninguno))
            },
            
            "decodificar_uri" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'decodificar_uri' no acepta argumentos".to_string(),
                    });
                }
                match urlencoding::decode(cadena) {
                    Ok(resultado) => Ok((Valor::Cadena(resultado.to_string()), ControlFlujo::Ninguno)),
                    Err(_) => Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "Error al decodificar URI: formato inválido".to_string(),
                    }),
                }
            },
            
            "partir_lineas" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'partir_lineas' no acepta argumentos".to_string(),
                    });
                }
                let lineas: Vec<Valor> = cadena.lines()
                    .map(|linea| Valor::Cadena(linea.to_string()))
                    .collect();
                Ok((Valor::Lista(lineas), ControlFlujo::Ninguno))
            },
            
            "capitalizar" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'capitalizar' no acepta argumentos".to_string(),
                    });
                }
                let mut chars = cadena.chars();
                let resultado = match chars.next() {
                    None => String::new(),
                    Some(primer_char) => primer_char.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
                };
                Ok((Valor::Cadena(resultado), ControlFlujo::Ninguno))
            },
            
            _ => Err(ErrorQuetzal::ErrorEjecucion {
                linea: 0,
                mensaje: format!("Método '{}' no está definido para cadenas", metodo),
            }),
        }
    }
    
    /// Valida si un valor es compatible con un tipo de dato específico
    fn validar_tipo_compatible(&self, valor: &Valor, tipo_esperado: &str) -> bool {
        match (valor, tipo_esperado) {
            (Valor::Vacio, "vacio") => true,
            (Valor::Entero(_), "entero") => true,
            (Valor::Numero(_), "número") => true,
            (Valor::Cadena(_), "cadena") => true,
            (Valor::Bool(_), "bool") => true,
            (Valor::Lista(_), "lista") => true,
            (Valor::Json(_), "jsn") => true,
            // Permitir conversiones automáticas compatibles
            (Valor::Entero(_), "número") => true, // entero puede ser número
            (Valor::Numero(n), "entero") => {
                // Validar que el número esté en el rango válido para i64
                *n >= i64::MIN as f64 && *n <= i64::MAX as f64 && n.is_finite()
            }, // número puede ser entero si está en rango válido (se truncará la parte decimal)
            // Tipo auto acepta cualquier cosa
            (_, "auto") => true,
            // Manejar listas tipadas
            (Valor::Lista(elementos), tipo) if tipo.starts_with("lista<") && tipo.ends_with(">") => {
                let tipo_elemento = &tipo[6..tipo.len()-1]; // extraer tipo entre < >
                // Validar que todos los elementos de la lista sean del tipo esperado
                elementos.iter().all(|elemento| self.validar_tipo_compatible(elemento, tipo_elemento))
            },
            _ => false,
        }
    }
    
    /// Convierte un valor al tipo especificado automáticamente cuando es compatible
    fn convertir_tipo_automatico(&self, valor: Valor, tipo_destino: &str) -> ResultadoQuetzal<Valor> {
        match (valor, tipo_destino) {
            // Sin conversión necesaria
            (val @ Valor::Vacio, "vacio") => Ok(val),
            (val @ Valor::Entero(_), "entero") => Ok(val),
            (val @ Valor::Numero(_), "número") => Ok(val),
            (val @ Valor::Cadena(_), "cadena") => Ok(val),
            (val @ Valor::Bool(_), "bool") => Ok(val),
            (val @ Valor::Lista(_), "lista") => Ok(val),
            (val @ Valor::Json(_), "jsn") => Ok(val),
            
            // Conversiones automáticas
            (Valor::Entero(n), "número") => Ok(Valor::Numero(n as f64)),
            (Valor::Numero(n), "entero") => {
                if n >= i64::MIN as f64 && n <= i64::MAX as f64 && n.is_finite() {
                    Ok(Valor::Entero(n as i64)) // Truncar la parte decimal
                } else {
                    Err(ErrorQuetzal::ErrorConversion {
                        linea: 0,
                        mensaje: "Número fuera del rango representable como entero".to_string(),
                    })
                }
            },
            
            // Tipo auto acepta cualquier cosa sin conversión
            (val, "auto") => Ok(val),
            
            // No se puede convertir
            (val, _) => Ok(val), // No hacer nada si ya se validó la compatibilidad
        }
    }
    
    /// Obtiene el nombre del tipo de un valor
    fn obtener_nombre_tipo(&self, valor: &Valor) -> &str {
        match valor {
            Valor::Vacio => "vacio",
            Valor::Entero(_) => "entero",
            Valor::Numero(_) => "número",
            Valor::Cadena(_) => "cadena",
            Valor::Bool(_) => "bool",
            Valor::Lista(_) => "lista",
            Valor::Json(_) => "jsn",
        }
    }
    
    /// Evalúa una operación binaria
    fn evaluar_operacion_binaria(&self, izquierdo: &Valor, operador: &str, derecho: &Valor) -> ResultadoQuetzal<Valor> {
        match operador {
            "+" => {
                match (izquierdo, derecho) {
                    (Valor::Entero(a), Valor::Entero(b)) => Ok(Valor::Entero(a + b)),
                    (Valor::Numero(a), Valor::Numero(b)) => {
                        let resultado = a + b;
                        Ok(Valor::Numero(self.redondear_numero(resultado)))
                    },
                    (Valor::Entero(a), Valor::Numero(b)) => {
                        let resultado = *a as f64 + b;
                        Ok(Valor::Numero(self.redondear_numero(resultado)))
                    },
                    (Valor::Numero(a), Valor::Entero(b)) => {
                        let resultado = a + *b as f64;
                        Ok(Valor::Numero(self.redondear_numero(resultado)))
                    },
                    (Valor::Cadena(a), Valor::Cadena(b)) => Ok(Valor::Cadena(format!("{}{}", a, b))),
                    (Valor::Cadena(a), b) => Ok(Valor::Cadena(format!("{}{}", a, b.a_cadena()))),
                    (a, Valor::Cadena(b)) => Ok(Valor::Cadena(format!("{}{}", a.a_cadena(), b))),
                    _ => Err(ErrorQuetzal::ErrorTipo {
                        linea: 0,
                        mensaje: format!("No se puede sumar {} y {}", izquierdo.tipo_como_cadena(), derecho.tipo_como_cadena()),
                    }),
                }
            },
            "-" => {
                match (izquierdo, derecho) {
                    (Valor::Entero(a), Valor::Entero(b)) => Ok(Valor::Entero(a - b)),
                    (Valor::Numero(a), Valor::Numero(b)) => {
                        let resultado = a - b;
                        Ok(Valor::Numero(self.redondear_numero(resultado)))
                    },
                    (Valor::Entero(a), Valor::Numero(b)) => {
                        let resultado = *a as f64 - b;
                        Ok(Valor::Numero(self.redondear_numero(resultado)))
                    },
                    (Valor::Numero(a), Valor::Entero(b)) => {
                        let resultado = a - *b as f64;
                        Ok(Valor::Numero(self.redondear_numero(resultado)))
                    },
                    _ => Err(ErrorQuetzal::ErrorTipo {
                        linea: 0,
                        mensaje: format!("No se puede restar {} y {}", izquierdo.tipo_como_cadena(), derecho.tipo_como_cadena()),
                    }),
                }
            },
            "*" => {
                match (izquierdo, derecho) {
                    (Valor::Entero(a), Valor::Entero(b)) => Ok(Valor::Entero(a * b)),
                    (Valor::Numero(a), Valor::Numero(b)) => {
                        let resultado = a * b;
                        Ok(Valor::Numero(self.redondear_numero(resultado)))
                    },
                    (Valor::Entero(a), Valor::Numero(b)) => {
                        let resultado = *a as f64 * b;
                        Ok(Valor::Numero(self.redondear_numero(resultado)))
                    },
                    (Valor::Numero(a), Valor::Entero(b)) => {
                        let resultado = a * *b as f64;
                        Ok(Valor::Numero(self.redondear_numero(resultado)))
                    },
                    _ => Err(ErrorQuetzal::ErrorTipo {
                        linea: 0,
                        mensaje: format!("No se puede multiplicar {} y {}", izquierdo.tipo_como_cadena(), derecho.tipo_como_cadena()),
                    }),
                }
            },
            "/" => {
                match (izquierdo, derecho) {
                    (Valor::Entero(a), Valor::Entero(b)) => {
                        if *b == 0 {
                            Err(ErrorQuetzal::DivisionPorCero { linea: 0 })
                        } else if a % b == 0 {
                            // Si la división es exacta, mantener como entero
                            Ok(Valor::Entero(a / b))
                        } else {
                            // Si no es exacta, convertir a decimal con redondeo
                            let resultado = *a as f64 / *b as f64;
                            Ok(Valor::Numero(self.redondear_numero(resultado)))
                        }
                    },
                    (Valor::Numero(a), Valor::Numero(b)) => {
                        if *b == 0.0 {
                            Err(ErrorQuetzal::DivisionPorCero { linea: 0 })
                        } else {
                            let resultado = a / b;
                            Ok(Valor::Numero(self.redondear_numero(resultado)))
                        }
                    },
                    (Valor::Entero(a), Valor::Numero(b)) => {
                        if *b == 0.0 {
                            Err(ErrorQuetzal::DivisionPorCero { linea: 0 })
                        } else {
                            let resultado = *a as f64 / b;
                            Ok(Valor::Numero(self.redondear_numero(resultado)))
                        }
                    },
                    (Valor::Numero(a), Valor::Entero(b)) => {
                        if *b == 0 {
                            Err(ErrorQuetzal::DivisionPorCero { linea: 0 })
                        } else {
                            let resultado = a / *b as f64;
                            Ok(Valor::Numero(self.redondear_numero(resultado)))
                        }
                    },
                    _ => Err(ErrorQuetzal::ErrorTipo {
                        linea: 0,
                        mensaje: format!("No se puede dividir {} y {}", izquierdo.tipo_como_cadena(), derecho.tipo_como_cadena()),
                    }),
                }
            },
            "%" => {
                match (izquierdo, derecho) {
                    (Valor::Entero(a), Valor::Entero(b)) => {
                        if *b == 0 {
                            Err(ErrorQuetzal::DivisionPorCero { linea: 0 })
                        } else {
                            Ok(Valor::Entero(a % b))
                        }
                    },
                    _ => Err(ErrorQuetzal::ErrorTipo {
                        linea: 0,
                        mensaje: "El operador módulo solo funciona con enteros".to_string(),
                    }),
                }
            },
            "==" => Ok(Valor::Bool(self.valores_iguales(izquierdo, derecho))),
            "!=" => Ok(Valor::Bool(!self.valores_iguales(izquierdo, derecho))),
            ">" => self.comparar_valores(izquierdo, derecho, |a, b| a > b),
            "<" => self.comparar_valores(izquierdo, derecho, |a, b| a < b),
            ">=" => self.comparar_valores(izquierdo, derecho, |a, b| a >= b),
            "<=" => self.comparar_valores(izquierdo, derecho, |a, b| a <= b),
            "&&" | "y" => Ok(Valor::Bool(izquierdo.a_bool() && derecho.a_bool())),
            "||" | "o" => Ok(Valor::Bool(izquierdo.a_bool() || derecho.a_bool())),
            _ => Err(ErrorQuetzal::ErrorEjecucion {
                linea: 0,
                mensaje: format!("Operador binario no soportado: {}", operador),
            }),
        }
    }
    
    /// Evalúa una operación unaria
    fn evaluar_operacion_unaria(&self, operador: &str, operando: &Valor) -> ResultadoQuetzal<Valor> {
        match operador {
            "!" => Ok(Valor::Bool(!operando.a_bool())),
            "-" => {
                match operando {
                    Valor::Entero(n) => Ok(Valor::Entero(-n)),
                    Valor::Numero(n) => Ok(Valor::Numero(-n)),
                    _ => Err(ErrorQuetzal::ErrorTipo {
                        linea: 0,
                        mensaje: format!("No se puede negar {}", operando.tipo_como_cadena()),
                    }),
                }
            },
            _ => Err(ErrorQuetzal::ErrorEjecucion {
                linea: 0,
                mensaje: format!("Operador unario no soportado: {}", operador),
            }),
        }
    }
    
    /// Verifica si dos valores son iguales
    fn valores_iguales(&self, a: &Valor, b: &Valor) -> bool {
        match (a, b) {
            (Valor::Vacio, Valor::Vacio) => true,
            (Valor::Entero(a), Valor::Entero(b)) => a == b,
            (Valor::Numero(a), Valor::Numero(b)) => (a - b).abs() < f64::EPSILON,
            (Valor::Entero(a), Valor::Numero(b)) => (*a as f64 - b).abs() < f64::EPSILON,
            (Valor::Numero(a), Valor::Entero(b)) => (a - *b as f64).abs() < f64::EPSILON,
            (Valor::Cadena(a), Valor::Cadena(b)) => a == b,
            (Valor::Bool(a), Valor::Bool(b)) => a == b,
            _ => false,
        }
    }
    
    /// Evalúa acceso a miembro de objeto o método
    fn evaluar_acceso_miembro(&self, objeto: &Valor, miembro: &str, linea: usize) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        match objeto {
            Valor::Json(mapa) => {
                if let Some(valor) = mapa.get(miembro) {
                    Ok((valor.clone(), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("La propiedad '{}' no existe en el objeto", miembro),
                    })
                }
            },
            Valor::Cadena(cadena) => {
                match miembro {
                    "longitud" => Ok((Valor::Entero(cadena.len() as i64), ControlFlujo::Ninguno)),
                    "esta_vacia" => Ok((Valor::Bool(cadena.is_empty()), ControlFlujo::Ninguno)),
                    "a_mayusculas" => Ok((Valor::Cadena(cadena.to_uppercase()), ControlFlujo::Ninguno)),
                    "a_minusculas" => Ok((Valor::Cadena(cadena.to_lowercase()), ControlFlujo::Ninguno)),
                    "recortar" => Ok((Valor::Cadena(cadena.trim().to_string()), ControlFlujo::Ninguno)),
                    "invertir" => Ok((Valor::Cadena(cadena.chars().rev().collect()), ControlFlujo::Ninguno)),
                    // Nuevos métodos de cadena avanzadas
                    "contiene" => {
                        // TODO: Necesita parámetro, implementar con argumentos
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'contiene' requiere argumentos".to_string(),
                        })
                    },
                    "buscar" => {
                        // TODO: Necesita parámetro, implementar con argumentos
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'buscar' requiere argumentos".to_string(),
                        })
                    },
                    "empieza_con" => {
                        // TODO: Necesita parámetro, implementar con argumentos
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'empieza_con' requiere argumentos".to_string(),
                        })
                    },
                    "termina_con" => {
                        // TODO: Necesita parámetro, implementar con argumentos
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'termina_con' requiere argumentos".to_string(),
                        })
                    },
                    "contar_ocurrencias" => {
                        // TODO: Necesita parámetro, implementar con argumentos
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'contar_ocurrencias' requiere argumentos".to_string(),
                        })
                    },
                    "reemplazar" => {
                        // TODO: Necesita parámetros, implementar con argumentos
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'reemplazar' requiere argumentos".to_string(),
                        })
                    },
                    "dividir" => {
                        // TODO: Necesita parámetro, implementar con argumentos
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'dividir' requiere argumentos".to_string(),
                        })
                    },
                    "subcadena" => {
                        // TODO: Necesita parámetros, implementar con argumentos
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'subcadena' requiere argumentos".to_string(),
                        })
                    },
                    "repetir" => {
                        // TODO: Necesita parámetro, implementar con argumentos
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'repetir' requiere argumentos".to_string(),
                        })
                    },
                    "comparar" => {
                        // TODO: Necesita parámetro, implementar con argumentos
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'comparar' requiere argumentos".to_string(),
                        })
                    },
                    "igual_sin_caso" => {
                        // TODO: Necesita parámetro, implementar con argumentos
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'igual_sin_caso' requiere argumentos".to_string(),
                        })
                    },
                    "codificar_base64" => {
                        // Implementación básica de base64
                        use base64::Engine;
                        let encoded = base64::engine::general_purpose::STANDARD.encode(cadena.as_bytes());
                        Ok((Valor::Cadena(encoded), ControlFlujo::Ninguno))
                    },
                    "decodificar_base64" => {
                        // Implementación básica de base64
                        use base64::Engine;
                        match base64::engine::general_purpose::STANDARD.decode(cadena) {
                            Ok(decoded) => {
                                match String::from_utf8(decoded) {
                                    Ok(s) => Ok((Valor::Cadena(s), ControlFlujo::Ninguno)),
                                    Err(_) => Err(ErrorQuetzal::ErrorConversion {
                                        linea,
                                        mensaje: "Error al decodificar base64".to_string(),
                                    }),
                                }
                            },
                            Err(_) => Err(ErrorQuetzal::ErrorConversion {
                                linea,
                                mensaje: "Base64 inválido".to_string(),
                            }),
                        }
                    },
                    "codificar_uri" => {
                        // Implementación básica de codificación URI
                        let encoded = urlencoding::encode(cadena).to_string();
                        Ok((Valor::Cadena(encoded), ControlFlujo::Ninguno))
                    },
                    "entero" => {
                        match cadena.trim().parse::<i64>() {
                            Ok(n) => Ok((Valor::Entero(n), ControlFlujo::Ninguno)),
                            Err(_) => Err(ErrorQuetzal::ErrorConversion {
                                linea: 0,
                                mensaje: "No se puede convertir cadena a entero".to_string(),
                            }),
                        }
                    },
                    "bool" => {
                        match cadena.to_lowercase().as_str() {
                            "verdadero" | "true" | "1" => Ok((Valor::Bool(true), ControlFlujo::Ninguno)),
                            "falso" | "false" | "0" => Ok((Valor::Bool(false), ControlFlujo::Ninguno)),
                            _ => Err(ErrorQuetzal::ErrorConversion {
                                linea: 0,
                                mensaje: "No se puede convertir cadena a booleano".to_string(),
                            }),
                        }
                    },
                    _ => Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("La función '{}' no está definida para cadenas", miembro),
                    }),
                }
            },
            Valor::Entero(numero) => {
                match miembro {
                    "cadena" => Ok((Valor::Cadena(numero.to_string()), ControlFlujo::Ninguno)),
                    "numero" => Ok((Valor::Numero(*numero as f64), ControlFlujo::Ninguno)),
                    "entero" => Ok((objeto.clone(), ControlFlujo::Ninguno)),
                    "bool" => Ok((Valor::Bool(*numero != 0), ControlFlujo::Ninguno)),
                    _ => Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("La función '{}' no está definida para enteros", miembro),
                    }),
                }
            },

            Valor::Numero(numero) => {
                match miembro {
                    "cadena" => Ok((Valor::Cadena(numero.to_string()), ControlFlujo::Ninguno)),
                    "numero" => Ok((objeto.clone(), ControlFlujo::Ninguno)),
                    "entero" => Ok((Valor::Entero(*numero as i64), ControlFlujo::Ninguno)),
                    "bool" => Ok((Valor::Bool(*numero != 0.0), ControlFlujo::Ninguno)),
                    _ => Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("La función '{}' no está definida para números", miembro),
                    }),
                }
            },
            Valor::Bool(booleano) => {
                match miembro {
                    "cadena" => {
                        let texto = if *booleano { "verdadero" } else { "falso" };
                        Ok((Valor::Cadena(texto.to_string()), ControlFlujo::Ninguno))
                    },
                    "numero" => Ok((Valor::Numero(if *booleano { 1.0 } else { 0.0 }), ControlFlujo::Ninguno)),
                    "entero" => Ok((Valor::Entero(if *booleano { 1 } else { 0 }), ControlFlujo::Ninguno)),
                    "bool" => Ok((objeto.clone(), ControlFlujo::Ninguno)),
                    _ => Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("La función '{}' no está definida para booleanos", miembro),
                    }),
                }
            },
            Valor::Lista(lista) => {
                match miembro {
                    "longitud" => Ok((Valor::Entero(lista.len() as i64), ControlFlujo::Ninguno)),
                    "esta_vacia" => Ok((Valor::Bool(lista.is_empty()), ControlFlujo::Ninguno)),
                    "ultimo" => {
                        if lista.is_empty() {
                            Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: "No se puede obtener el último elemento de una lista vacía".to_string(),
                            })
                        } else {
                            Ok((lista.last().unwrap().clone(), ControlFlujo::Ninguno))
                        }
                    },
                    "primero" => {
                        if lista.is_empty() {
                            Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: "No se puede obtener el primer elemento de una lista vacía".to_string(),
                            })
                        } else {
                            Ok((lista.first().unwrap().clone(), ControlFlujo::Ninguno))
                        }
                    },
                    _ => Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("La función '{}' no está definida para listas", miembro),
                    }),
                }
            },
            _ => Err(ErrorQuetzal::ErrorTipo {
                linea,
                mensaje: format!("No se puede acceder a miembros de {}", objeto.tipo_como_cadena()),
            }),
        }
    }
    
    /// Compara dos valores numéricamente
    fn comparar_valores<F>(&self, a: &Valor, b: &Valor, comparador: F) -> ResultadoQuetzal<Valor>
    where
        F: Fn(f64, f64) -> bool,
    {
        let num_a = match a {
            Valor::Entero(n) => *n as f64,
            Valor::Numero(n) => *n,
            _ => return Err(ErrorQuetzal::ErrorTipo {
                linea: 0,
                mensaje: format!("No se puede comparar {}", a.tipo_como_cadena()),
            }),
        };
        
        let num_b = match b {
            Valor::Entero(n) => *n as f64,
            Valor::Numero(n) => *n,
            _ => return Err(ErrorQuetzal::ErrorTipo {
                linea: 0,
                mensaje: format!("No se puede comparar {}", b.tipo_como_cadena()),
            }),
        };
        
        Ok(Valor::Bool(comparador(num_a, num_b)))
    }
    
    /// Evalúa un método en un valor específico
    fn evaluar_metodo_en_valor(&mut self, valor: &Valor, metodo: &str, argumentos: &[Valor], linea: usize) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        match metodo {
            // Métodos de conversión
            "cadena" => {
                let resultado = match valor {
                    Valor::Entero(n) => Valor::Cadena(n.to_string()),
                    Valor::Numero(n) => Valor::Cadena(n.to_string()),
                    Valor::Bool(b) => Valor::Cadena(b.to_string()),
                    Valor::Cadena(s) => Valor::Cadena(s.clone()),
                    Valor::Lista(lista) => {
                        let elementos: Vec<String> = lista.iter()
                            .map(|v| match v {
                                Valor::Cadena(s) => s.clone(),
                                _ => v.to_string(),
                            })
                            .collect();
                        Valor::Cadena(format!("[{}]", elementos.join(", ")))
                    },
                    _ => Valor::Cadena(valor.to_string()),
                };
                Ok((resultado, ControlFlujo::Ninguno))
            },
            
            "entero" => {
                let resultado = match valor {
                    Valor::Entero(n) => Valor::Entero(*n),
                    Valor::Numero(n) => Valor::Entero(*n as i64),
                    Valor::Cadena(s) => {
                        match s.trim().parse::<i64>() {
                            Ok(n) => Valor::Entero(n),
                            Err(_) => return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: format!("No se puede convertir '{}' a entero", s),
                            }),
                        }
                    },
                    Valor::Bool(true) => Valor::Entero(1),
                    Valor::Bool(false) => Valor::Entero(0),
                    _ => return Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("No se puede convertir {} a entero", valor.tipo_como_cadena()),
                    }),
                };
                Ok((resultado, ControlFlujo::Ninguno))
            },
            
            "numero" => {
                let resultado = match valor {
                    Valor::Entero(n) => Valor::Numero(*n as f64),
                    Valor::Numero(n) => Valor::Numero(*n),
                    Valor::Cadena(s) => {
                        let trimmed = s.trim();
                        
                        match trimmed.parse::<f64>() {
                            Ok(n) => {
                                if n.is_finite() && !n.is_infinite() && !n.is_nan() {
                                    Valor::Numero(n)
                                } else {
                                    return Err(ErrorQuetzal::ErrorEjecucion {
                                        linea,
                                        mensaje: "Número fuera del rango representable".to_string(),
                                    });
                                }
                            },
                            Err(_) => return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: format!("No se puede convertir '{}' a número", s),
                            }),
                        }
                    },
                    Valor::Bool(true) => Valor::Numero(1.0),
                    Valor::Bool(false) => Valor::Numero(0.0),
                    _ => return Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("No se puede convertir {} a número", valor.tipo_como_cadena()),
                    }),
                };
                Ok((resultado, ControlFlujo::Ninguno))
            },
            
            "bool" => {
                let resultado = match valor {
                    Valor::Bool(b) => Valor::Bool(*b),
                    Valor::Entero(n) => Valor::Bool(*n != 0),
                    Valor::Numero(n) => Valor::Bool(*n != 0.0),
                    Valor::Cadena(s) => Valor::Bool(!s.is_empty()),
                    _ => return Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("No se puede convertir {} a bool", valor.tipo_como_cadena()),
                    }),
                };
                Ok((resultado, ControlFlujo::Ninguno))
            },
            
            "mayuscula" => {
                if let Valor::Cadena(s) = valor {
                    Ok((Valor::Cadena(s.to_uppercase()), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'mayuscula' solo es válido para cadenas"),
                    })
                }
            },
            
            "minuscula" => {
                if let Valor::Cadena(s) = valor {
                    Ok((Valor::Cadena(s.to_lowercase()), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'minuscula' solo es válido para cadenas"),
                    })
                }
            },
            
            "a_minusculas" => {
                if let Valor::Cadena(s) = valor {
                    Ok((Valor::Cadena(s.to_lowercase()), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'a_minusculas' solo es válido para cadenas"),
                    })
                }
            },
            
            "capitalizar" => {
                if let Valor::Cadena(s) = valor {
                    let mut chars = s.chars();
                    let resultado = match chars.next() {
                        None => String::new(),
                        Some(primer_char) => primer_char.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
                    };
                    Ok((Valor::Cadena(resultado), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'capitalizar' solo es válido para cadenas"),
                    })
                }
            },
            
            "recortar" => {
                if let Valor::Cadena(s) = valor {
                    Ok((Valor::Cadena(s.trim().to_string()), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'recortar' solo es válido para cadenas"),
                    })
                }
            },
            
            // Métodos de lista
            "unir" => {
                if let Valor::Lista(lista) = valor {
                    if argumentos.is_empty() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'unir' requiere un separador como argumento".to_string(),
                        });
                    }
                    
                    if let Valor::Cadena(separador) = &argumentos[0] {
                        let elementos: Vec<String> = lista.iter()
                            .map(|v| match v {
                                Valor::Cadena(s) => s.clone(),
                                _ => v.to_string(),
                            })
                            .collect();
                        Ok((Valor::Cadena(elementos.join(separador)), ControlFlujo::Ninguno))
                    } else {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El separador para 'unir' debe ser una cadena".to_string(),
                        })
                    }
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'unir' solo es válido para listas"),
                    })
                }
            },
            
            "unir_lineas" => {
                if let Valor::Lista(lista) = valor {
                    let elementos: Vec<String> = lista.iter()
                        .map(|v| match v {
                            Valor::Cadena(s) => s.clone(),
                            _ => v.to_string(),
                        })
                        .collect();
                    Ok((Valor::Cadena(elementos.join("\n")), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'unir_lineas' solo es válido para listas"),
                    })
                }
            },
            
            "buscar" => {
                if let Valor::Cadena(s) = valor {
                    if argumentos.is_empty() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'buscar' requiere un patrón como argumento".to_string(),
                        });
                    }
                    
                    if let Valor::Cadena(patron) = &argumentos[0] {
                        match s.find(patron) {
                            Some(pos) => Ok((Valor::Entero(pos as i64), ControlFlujo::Ninguno)),
                            None => Ok((Valor::Entero(-1), ControlFlujo::Ninguno)),
                        }
                    } else {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El patrón para 'buscar' debe ser una cadena".to_string(),
                        })
                    }
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'buscar' solo es válido para cadenas"),
                    })
                }
            },
            
            "contiene" => {
                if let Valor::Cadena(s) = valor {
                    if argumentos.is_empty() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'contiene' requiere un patrón como argumento".to_string(),
                        });
                    }
                    
                    if let Valor::Cadena(patron) = &argumentos[0] {
                        Ok((Valor::Bool(s.contains(patron)), ControlFlujo::Ninguno))
                    } else {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El patrón para 'contiene' debe ser una cadena".to_string(),
                        })
                    }
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'contiene' solo es válido para cadenas"),
                    })
                }
            },
            
            "empieza_con" => {
                if let Valor::Cadena(s) = valor {
                    if argumentos.is_empty() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'empieza_con' requiere un prefijo como argumento".to_string(),
                        });
                    }
                    
                    if let Valor::Cadena(prefijo) = &argumentos[0] {
                        Ok((Valor::Bool(s.starts_with(prefijo)), ControlFlujo::Ninguno))
                    } else {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El prefijo para 'empieza_con' debe ser una cadena".to_string(),
                        })
                    }
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'empieza_con' solo es válido para cadenas"),
                    })
                }
            },
            
            "termina_con" => {
                if let Valor::Cadena(s) = valor {
                    if argumentos.is_empty() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'termina_con' requiere un sufijo como argumento".to_string(),
                        });
                    }
                    
                    if let Valor::Cadena(sufijo) = &argumentos[0] {
                        Ok((Valor::Bool(s.ends_with(sufijo)), ControlFlujo::Ninguno))
                    } else {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El sufijo para 'termina_con' debe ser una cadena".to_string(),
                        })
                    }
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'termina_con' solo es válido para cadenas"),
                    })
                }
            },
            
            "contar_ocurrencias" => {
                if let Valor::Cadena(s) = valor {
                    if argumentos.is_empty() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'contar_ocurrencias' requiere un patrón como argumento".to_string(),
                        });
                    }
                    
                    if let Valor::Cadena(patron) = &argumentos[0] {
                        if patron.is_empty() {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: "El patrón no puede estar vacío".to_string(),
                            });
                        }
                        let count = s.matches(patron).count() as i64;
                        Ok((Valor::Entero(count), ControlFlujo::Ninguno))
                    } else {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El patrón para 'contar_ocurrencias' debe ser una cadena".to_string(),
                        })
                    }
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'contar_ocurrencias' solo es válido para cadenas"),
                    })
                }
            },
            
            "a_mayusculas" => {
                if let Valor::Cadena(s) = valor {
                    Ok((Valor::Cadena(s.to_uppercase()), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'a_mayusculas' solo es válido para cadenas"),
                    })
                }
            },
            
            "invertir" => {
                match valor {
                    Valor::Cadena(s) => {
                        let invertida = s.chars().rev().collect::<String>();
                        Ok((Valor::Cadena(invertida), ControlFlujo::Ninguno))
                    },
                    Valor::Lista(lista) => {
                        let mut lista_invertida = lista.clone();
                        lista_invertida.reverse();
                        Ok((Valor::Lista(lista_invertida), ControlFlujo::Ninguno))
                    },
                    _ => {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: format!("El método 'invertir' solo es válido para cadenas y listas"),
                        })
                    }
                }
            },
            
            "repetir" => {
                if let Valor::Cadena(s) = valor {
                    if argumentos.is_empty() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'repetir' requiere un número como argumento".to_string(),
                        });
                    }
                    
                    let veces = match &argumentos[0] {
                        Valor::Entero(n) => *n,
                        Valor::Numero(n) => *n as i64,
                        _ => return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El argumento para 'repetir' debe ser un número".to_string(),
                        }),
                    };
                    
                    if veces < 0 {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El número de repeticiones no puede ser negativo".to_string(),
                        });
                    }
                    
                    Ok((Valor::Cadena(s.repeat(veces as usize)), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'repetir' solo es válido para cadenas"),
                    })
                }
            },
            
            "reemplazar" => {
                if let Valor::Cadena(s) = valor {
                    if argumentos.len() < 2 {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'reemplazar' requiere dos argumentos: buscar y reemplazar".to_string(),
                        });
                    }
                    
                    if let (Valor::Cadena(buscar), Valor::Cadena(reemplazar)) = (&argumentos[0], &argumentos[1]) {
                        Ok((Valor::Cadena(s.replace(buscar, reemplazar)), ControlFlujo::Ninguno))
                    } else {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "Los argumentos para 'reemplazar' deben ser cadenas".to_string(),
                        })
                    }
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'reemplazar' solo es válido para cadenas"),
                    })
                }
            },
            
            "subcadena" => {
                if let Valor::Cadena(s) = valor {
                    if argumentos.is_empty() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'subcadena' requiere al menos un índice".to_string(),
                        });
                    }
                    
                    let inicio = match &argumentos[0] {
                        Valor::Entero(n) => *n as usize,
                        Valor::Numero(n) => *n as usize,
                        _ => return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El índice de inicio debe ser un número".to_string(),
                        }),
                    };
                    
                    let chars: Vec<char> = s.chars().collect();
                    
                    // Si el índice de inicio es igual o mayor que la longitud, devolver cadena vacía
                    if inicio >= chars.len() {
                        return Ok((Valor::Cadena(String::new()), ControlFlujo::Ninguno));
                    }
                    
                    let fin = if argumentos.len() > 1 {
                        match &argumentos[1] {
                            Valor::Entero(n) => (*n as usize).min(chars.len()),
                            Valor::Numero(n) => (*n as usize).min(chars.len()),
                            _ => return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: "El índice de fin debe ser un número".to_string(),
                            }),
                        }
                    } else {
                        chars.len()
                    };
                    
                    if inicio > fin {
                        return Ok((Valor::Cadena(String::new()), ControlFlujo::Ninguno));
                    }
                    
                    let subcadena: String = chars[inicio..fin].iter().collect();
                    Ok((Valor::Cadena(subcadena), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'subcadena' solo es válido para cadenas"),
                    })
                }
            },
            
            "dividir" => {
                if let Valor::Cadena(s) = valor {
                    if argumentos.is_empty() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'dividir' requiere un delimitador como argumento".to_string(),
                        });
                    }
                    
                    if let Valor::Cadena(delimitador) = &argumentos[0] {
                        if delimitador.is_empty() {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: "El delimitador no puede estar vacío".to_string(),
                            });
                        }
                        let partes: Vec<Valor> = s.split(delimitador)
                            .map(|parte| Valor::Cadena(parte.to_string()))
                            .collect();
                        Ok((Valor::Lista(partes), ControlFlujo::Ninguno))
                    } else {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El delimitador para 'dividir' debe ser una cadena".to_string(),
                        })
                    }
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'dividir' solo es válido para cadenas"),
                    })
                }
            },
            
            "partir_lineas" => {
                if let Valor::Cadena(s) = valor {
                    let lineas: Vec<Valor> = s.lines()
                        .map(|linea| Valor::Cadena(linea.to_string()))
                        .collect();
                    Ok((Valor::Lista(lineas), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'partir_lineas' solo es válido para cadenas"),
                    })
                }
            },
            
            "comparar" => {
                if let Valor::Cadena(s) = valor {
                    if argumentos.is_empty() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'comparar' requiere otra cadena como argumento".to_string(),
                        });
                    }
                    
                    if let Valor::Cadena(otra) = &argumentos[0] {
                        let resultado = if s < otra {
                            -1
                        } else if s > otra {
                            1
                        } else {
                            0
                        };
                        Ok((Valor::Entero(resultado), ControlFlujo::Ninguno))
                    } else {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El argumento para 'comparar' debe ser una cadena".to_string(),
                        })
                    }
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'comparar' solo es válido para cadenas"),
                    })
                }
            },
            
            "igual_sin_caso" => {
                if let Valor::Cadena(s) = valor {
                    if argumentos.is_empty() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'igual_sin_caso' requiere otra cadena como argumento".to_string(),
                        });
                    }
                    
                    if let Valor::Cadena(otra) = &argumentos[0] {
                        Ok((Valor::Bool(s.to_lowercase() == otra.to_lowercase()), ControlFlujo::Ninguno))
                    } else {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El argumento para 'igual_sin_caso' debe ser una cadena".to_string(),
                        })
                    }
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'igual_sin_caso' solo es válido para cadenas"),
                    })
                }
            },
            
            "codificar_base64" => {
                if let Valor::Cadena(s) = valor {
                    use base64::{Engine as _, engine::general_purpose};
                    let encoded = general_purpose::STANDARD.encode(s.as_bytes());
                    Ok((Valor::Cadena(encoded), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'codificar_base64' solo es válido para cadenas"),
                    })
                }
            },
            
            "decodificar_base64" => {
                if let Valor::Cadena(s) = valor {
                    use base64::{Engine as _, engine::general_purpose};
                    match general_purpose::STANDARD.decode(s) {
                        Ok(bytes) => match String::from_utf8(bytes) {
                            Ok(decoded) => Ok((Valor::Cadena(decoded), ControlFlujo::Ninguno)),
                            Err(_) => Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: "Los datos decodificados no son texto UTF-8 válido".to_string(),
                            }),
                        },
                        Err(_) => Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "Cadena Base64 inválida".to_string(),
                        }),
                    }
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'decodificar_base64' solo es válido para cadenas"),
                    })
                }
            },
            
            "codificar_uri" => {
                if let Valor::Cadena(s) = valor {
                    let encoded = urlencoding::encode(s);
                    Ok((Valor::Cadena(encoded.to_string()), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'codificar_uri' solo es válido para cadenas"),
                    })
                }
            },
            
            "decodificar_uri" => {
                if let Valor::Cadena(s) = valor {
                    match urlencoding::decode(s) {
                        Ok(decoded) => Ok((Valor::Cadena(decoded.to_string()), ControlFlujo::Ninguno)),
                        Err(_) => Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "URI inválida para decodificar".to_string(),
                        }),
                    }
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'decodificar_uri' solo es válido para cadenas"),
                    })
                }
            },
            
            // Métodos específicos de listas
            "primero" => {
                if let Valor::Lista(lista) = valor {
                    if lista.is_empty() {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "No se puede obtener el primer elemento de una lista vacía".to_string(),
                        })
                    } else {
                        Ok((lista.first().unwrap().clone(), ControlFlujo::Ninguno))
                    }
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'primero' solo es válido para listas"),
                    })
                }
            },
            
            "ultimo" => {
                if let Valor::Lista(lista) = valor {
                    if lista.is_empty() {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "No se puede obtener el último elemento de una lista vacía".to_string(),
                        })
                    } else {
                        Ok((lista.last().unwrap().clone(), ControlFlujo::Ninguno))
                    }
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'ultimo' solo es válido para listas"),
                    })
                }
            },
            
            "ordenar" => {
                if let Valor::Lista(lista) = valor {
                    let mut lista_ordenada = lista.clone();
                    lista_ordenada.sort_by(|a, b| {
                        match (a, b) {
                            (Valor::Entero(x), Valor::Entero(y)) => x.cmp(y),
                            (Valor::Numero(x), Valor::Numero(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                            (Valor::Cadena(x), Valor::Cadena(y)) => x.cmp(y),
                            _ => std::cmp::Ordering::Equal,
                        }
                    });
                    Ok((Valor::Lista(lista_ordenada), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'ordenar' solo es válido para listas"),
                    })
                }
            },
            
            "longitud" => {
                match valor {
                    Valor::Lista(lista) => Ok((Valor::Entero(lista.len() as i64), ControlFlujo::Ninguno)),
                    Valor::Cadena(cadena) => Ok((Valor::Entero(cadena.chars().count() as i64), ControlFlujo::Ninguno)),
                    _ => Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'longitud' solo es válido para cadenas y listas"),
                    })
                }
            },
            
            "esta_vacia" => {
                match valor {
                    Valor::Lista(lista) => Ok((Valor::Bool(lista.is_empty()), ControlFlujo::Ninguno)),
                    Valor::Cadena(cadena) => Ok((Valor::Bool(cadena.is_empty()), ControlFlujo::Ninguno)),
                    _ => Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'esta_vacia' solo es válido para cadenas y listas"),
                    })
                }
            },
            
            _ => {
                Err(ErrorQuetzal::ErrorEjecucion {
                    linea,
                    mensaje: format!("Método '{}' no reconocido para tipo {}", metodo, valor.tipo_como_cadena()),
                })
            }
        }
    }
    
    /// Evalúa métodos que modifican la variable original (métodos mutantes)
    fn evaluar_metodo_mutante(&mut self, nombre_var: &str, metodo: &str, argumentos: &[Valor], linea: usize, entorno: Rc<RefCell<Entorno>>) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        let mut entorno_ref = entorno.borrow_mut();
        
        if let Some(variable) = entorno_ref.variables.get_mut(nombre_var) {
            if !variable.es_mutable() {
                return Err(ErrorQuetzal::ErrorEjecucion {
                    linea,
                    mensaje: format!("No se puede modificar la variable inmutable '{}'", nombre_var),
                });
            }
            
            match (&mut variable.valor, metodo) {
                (Valor::Lista(ref mut lista), "agregar") => {
                    if argumentos.is_empty() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'agregar' requiere un argumento".to_string(),
                        });
                    }
                    
                    let elemento = &argumentos[0];
                    
                    // TODO: Validación de tipos para listas tipadas se puede agregar aquí
                    if let Valor::Vacio = elemento {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "No se puede agregar un valor vacío a la lista".to_string(),
                        });
                    }
                    
                    lista.push(elemento.clone());
                    Ok((Valor::Entero(lista.len() as i64), ControlFlujo::Ninguno))
                },
                
                (Valor::Lista(ref mut lista), "quitar") => {
                    if argumentos.is_empty() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'quitar' requiere un índice".to_string(),
                        });
                    }
                    
                    let indice = match &argumentos[0] {
                        Valor::Entero(i) => {
                            if *i < 0 {
                                return Err(ErrorQuetzal::ErrorEjecucion {
                                    linea,
                                    mensaje: format!("Índice negativo: {}", i),
                                });
                            }
                            *i as usize
                        },
                        _ => {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: "El índice debe ser un número entero".to_string(),
                            });
                        }
                    };
                    
                    if indice >= lista.len() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: format!("Índice fuera de rango: {} (tamaño: {})", indice, lista.len()),
                        });
                    }
                    
                    let elemento_quitado = lista.remove(indice);
                    Ok((elemento_quitado, ControlFlujo::Ninguno))
                },
                
                (Valor::Lista(ref mut lista), "limpiar") => {
                    lista.clear();
                    Ok((Valor::Vacio, ControlFlujo::Ninguno))
                },
                
                _ => {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método '{}' no está disponible para el tipo {}", metodo, variable.valor.tipo_como_cadena()),
                    })
                }
            }
        } else {
            Err(ErrorQuetzal::VariableNoDefinida {
                linea,
                nombre: nombre_var.to_string(),
            })
        }
    }

    /// Función auxiliar para asignar valores a índices anidados (matrices)
    fn asignar_indice_recursivo(
        &mut self,
        objeto: &Nodo,
        indice: usize,
        nuevo_valor: Valor,
        entorno: Rc<RefCell<Entorno>>,
        linea: usize,
    ) -> ResultadoQuetzal<()> {
        match objeto {
            // Caso base: identificador directo (variable)
            Nodo::Identificador(nombre_var) => {
                let mut entorno_ref = entorno.borrow_mut();
                if let Some(variable) = entorno_ref.variables.get_mut(nombre_var) {
                    if !variable.es_mutable() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: format!("No se puede modificar la lista inmutable '{}'", nombre_var),
                        });
                    }
                    
                    if let Valor::Lista(ref mut lista) = variable.valor {
                        if indice >= lista.len() {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: format!("Índice fuera de rango: {} (tamaño: {})", indice, lista.len()),
                            });
                        }
                        
                        lista[indice] = nuevo_valor;
                        Ok(())
                    } else {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: format!("'{}' no es una lista", nombre_var),
                        })
                    }
                } else {
                    Err(ErrorQuetzal::VariableNoDefinida {
                        linea,
                        nombre: nombre_var.clone(),
                    })
                }
            },
            
            // Caso recursivo: acceso a índice anidado
            Nodo::AccesoIndice { objeto: objeto_padre, indice: indice_padre, linea: _ } => {
                // Evaluar el índice del padre
                let (valor_indice_padre, _) = self.evaluar_con_entorno(indice_padre, entorno.clone())?;
                let indice_padre_usize = match valor_indice_padre {
                    Valor::Entero(i) => {
                        if i < 0 {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: format!("Índice negativo: {}", i),
                            });
                        }
                        i as usize
                    },
                    _ => {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El índice debe ser un número entero".to_string(),
                        });
                    }
                };
                
                // Obtener referencia al objeto padre
                self.asignar_indice_anidado_recursivo(objeto_padre, indice_padre_usize, indice, nuevo_valor, entorno, linea)
            },
            
            _ => Err(ErrorQuetzal::ErrorEjecucion {
                linea,
                mensaje: "Solo se puede asignar a índices de variables o accesos a índices".to_string(),
            })
        }
    }

    /// Función auxiliar para la asignación recursiva profunda en matrices
    fn asignar_indice_anidado_recursivo(
        &mut self,
        objeto_padre: &Nodo,
        indice_padre: usize,
        indice_hijo: usize,
        nuevo_valor: Valor,
        entorno: Rc<RefCell<Entorno>>,
        linea: usize,
    ) -> ResultadoQuetzal<()> {
        match objeto_padre {
            Nodo::Identificador(nombre_var) => {
                let mut entorno_ref = entorno.borrow_mut();
                if let Some(variable) = entorno_ref.variables.get_mut(nombre_var) {
                    if !variable.es_mutable() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: format!("No se puede modificar la lista inmutable '{}'", nombre_var),
                        });
                    }
                    
                    if let Valor::Lista(ref mut lista_padre) = variable.valor {
                        if indice_padre >= lista_padre.len() {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: format!("Índice fuera de rango: {} (tamaño: {})", indice_padre, lista_padre.len()),
                            });
                        }
                        
                        if let Valor::Lista(ref mut lista_hija) = lista_padre[indice_padre] {
                            if indice_hijo >= lista_hija.len() {
                                return Err(ErrorQuetzal::ErrorEjecucion {
                                    linea,
                                    mensaje: format!("Índice fuera de rango: {} (tamaño: {})", indice_hijo, lista_hija.len()),
                                });
                            }
                            
                            lista_hija[indice_hijo] = nuevo_valor;
                            Ok(())
                        } else {
                            Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: "El elemento no es una lista".to_string(),
                            })
                        }
                    } else {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: format!("'{}' no es una lista", nombre_var),
                        })
                    }
                } else {
                    Err(ErrorQuetzal::VariableNoDefinida {
                        linea,
                        nombre: nombre_var.clone(),
                    })
                }
            },
            
            // Para casos más anidados, implementar recursivamente
            Nodo::AccesoIndice { objeto: objeto_sub_padre, indice: indice_sub_padre, linea: _ } => {
                // Evaluar el índice del sub-padre
                let (valor_indice_sub_padre, _) = self.evaluar_con_entorno(indice_sub_padre, entorno.clone())?;
                let indice_sub_padre_usize = match valor_indice_sub_padre {
                    Valor::Entero(i) => {
                        if i < 0 {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: format!("Índice negativo: {}", i),
                            });
                        }
                        i as usize
                    },
                    _ => {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El índice debe ser un número entero".to_string(),
                        });
                    }
                };
                
                // Recursión para manejar niveles más profundos
                self.asignar_indice_triple_anidado(objeto_sub_padre, indice_sub_padre_usize, indice_padre, indice_hijo, nuevo_valor, entorno, linea)
            },
            
            _ => Err(ErrorQuetzal::ErrorEjecucion {
                linea,
                mensaje: "Tipo de objeto no soportado para asignación anidada".to_string(),
            })
        }
    }
    
    /// Función auxiliar para manejar asignaciones de 3 o más niveles de profundidad
    fn asignar_indice_triple_anidado(
        &mut self,
        objeto_abuelo: &Nodo,
        indice_abuelo: usize,
        indice_padre: usize,
        indice_hijo: usize,
        nuevo_valor: Valor,
        entorno: Rc<RefCell<Entorno>>,
        linea: usize,
    ) -> ResultadoQuetzal<()> {
        match objeto_abuelo {
            Nodo::Identificador(nombre_var) => {
                let mut entorno_ref = entorno.borrow_mut();
                if let Some(variable) = entorno_ref.variables.get_mut(nombre_var) {
                    if !variable.es_mutable() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: format!("No se puede modificar la matriz inmutable '{}'", nombre_var),
                        });
                    }
                    
                    if let Valor::Lista(ref mut lista_abuelo) = variable.valor {
                        if indice_abuelo >= lista_abuelo.len() {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: format!("Índice fuera de rango en nivel 1: {} (tamaño: {})", indice_abuelo, lista_abuelo.len()),
                            });
                        }
                        
                        if let Valor::Lista(ref mut lista_padre) = lista_abuelo[indice_abuelo] {
                            if indice_padre >= lista_padre.len() {
                                return Err(ErrorQuetzal::ErrorEjecucion {
                                    linea,
                                    mensaje: format!("Índice fuera de rango en nivel 2: {} (tamaño: {})", indice_padre, lista_padre.len()),
                                });
                            }
                            
                            if let Valor::Lista(ref mut lista_hijo) = lista_padre[indice_padre] {
                                if indice_hijo >= lista_hijo.len() {
                                    return Err(ErrorQuetzal::ErrorEjecucion {
                                        linea,
                                        mensaje: format!("Índice fuera de rango en nivel 3: {} (tamaño: {})", indice_hijo, lista_hijo.len()),
                                    });
                                }
                                
                                lista_hijo[indice_hijo] = nuevo_valor;
                                Ok(())
                            } else {
                                Err(ErrorQuetzal::ErrorEjecucion {
                                    linea,
                                    mensaje: "El elemento en nivel 2 no es una lista".to_string(),
                                })
                            }
                        } else {
                            Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: "El elemento en nivel 1 no es una lista".to_string(),
                            })
                        }
                    } else {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: format!("'{}' no es una lista", nombre_var),
                        })
                    }
                } else {
                    Err(ErrorQuetzal::VariableNoDefinida {
                        linea,
                        nombre: nombre_var.clone(),
                    })
                }
            },
            
            // Aquí se puede extender para manejar casos aún más anidados si es necesario
            Nodo::AccesoIndice { objeto: objeto_bis_abuelo, indice: indice_bis_abuelo, linea: _ } => {
                // Para matrices 4D, 5D, etc. - se puede implementar de manera similar
                let (valor_indice_bis_abuelo, _) = self.evaluar_con_entorno(indice_bis_abuelo, entorno.clone())?;
                let indice_bis_abuelo_usize = match valor_indice_bis_abuelo {
                    Valor::Entero(i) => {
                        if i < 0 {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: format!("Índice negativo: {}", i),
                            });
                        }
                        i as usize
                    },
                    _ => {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El índice debe ser un número entero".to_string(),
                        });
                    }
                };
                
                // Implementar recursión para matrices 4D
                self.asignar_indice_cuadruple_anidado(objeto_bis_abuelo, indice_bis_abuelo_usize, indice_abuelo, indice_padre, indice_hijo, nuevo_valor, entorno, linea)
            },
            
            _ => Err(ErrorQuetzal::ErrorEjecucion {
                linea,
                mensaje: "Tipo de objeto no soportado para asignación triple anidada".to_string(),
            })
        }
    }
    
    /// Función auxiliar para manejar asignaciones de 4 niveles de profundidad (matrices 4D)
    fn asignar_indice_cuadruple_anidado(
        &mut self,
        objeto_bis_abuelo: &Nodo,
        indice_bis_abuelo: usize,
        indice_abuelo: usize,
        indice_padre: usize,
        indice_hijo: usize,
        nuevo_valor: Valor,
        entorno: Rc<RefCell<Entorno>>,
        linea: usize,
    ) -> ResultadoQuetzal<()> {
        match objeto_bis_abuelo {
            Nodo::Identificador(nombre_var) => {
                let mut entorno_ref = entorno.borrow_mut();
                if let Some(variable) = entorno_ref.variables.get_mut(nombre_var) {
                    if !variable.es_mutable() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: format!("No se puede modificar la matriz 4D inmutable '{}'", nombre_var),
                        });
                    }
                    
                    if let Valor::Lista(ref mut lista_nivel_0) = variable.valor {
                        if indice_bis_abuelo >= lista_nivel_0.len() {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: format!("Índice fuera de rango en nivel 0: {} (tamaño: {})", indice_bis_abuelo, lista_nivel_0.len()),
                            });
                        }
                        
                        if let Valor::Lista(ref mut lista_nivel_1) = lista_nivel_0[indice_bis_abuelo] {
                            if indice_abuelo >= lista_nivel_1.len() {
                                return Err(ErrorQuetzal::ErrorEjecucion {
                                    linea,
                                    mensaje: format!("Índice fuera de rango en nivel 1: {} (tamaño: {})", indice_abuelo, lista_nivel_1.len()),
                                });
                            }
                            
                            if let Valor::Lista(ref mut lista_nivel_2) = lista_nivel_1[indice_abuelo] {
                                if indice_padre >= lista_nivel_2.len() {
                                    return Err(ErrorQuetzal::ErrorEjecucion {
                                        linea,
                                        mensaje: format!("Índice fuera de rango en nivel 2: {} (tamaño: {})", indice_padre, lista_nivel_2.len()),
                                    });
                                }
                                
                                if let Valor::Lista(ref mut lista_nivel_3) = lista_nivel_2[indice_padre] {
                                    if indice_hijo >= lista_nivel_3.len() {
                                        return Err(ErrorQuetzal::ErrorEjecucion {
                                            linea,
                                            mensaje: format!("Índice fuera de rango en nivel 3: {} (tamaño: {})", indice_hijo, lista_nivel_3.len()),
                                        });
                                    }
                                    
                                    lista_nivel_3[indice_hijo] = nuevo_valor;
                                    Ok(())
                                } else {
                                    Err(ErrorQuetzal::ErrorEjecucion {
                                        linea,
                                        mensaje: "El elemento en nivel 2 no es una lista".to_string(),
                                    })
                                }
                            } else {
                                Err(ErrorQuetzal::ErrorEjecucion {
                                    linea,
                                    mensaje: "El elemento en nivel 1 no es una lista".to_string(),
                                })
                            }
                        } else {
                            Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: "El elemento en nivel 0 no es una lista".to_string(),
                            })
                        }
                    } else {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: format!("'{}' no es una lista", nombre_var),
                        })
                    }
                } else {
                    Err(ErrorQuetzal::VariableNoDefinida {
                        linea,
                        nombre: nombre_var.clone(),
                    })
                }
            },
            
            _ => Err(ErrorQuetzal::ErrorEjecucion {
                linea,
                mensaje: "Matrices de más de 4 dimensiones no están soportadas actualmente".to_string(),
            })
        }
    }

    /// Evalúa métodos mutantes en elementos accedidos por índice (como matriz[i].agregar())
    fn evaluar_metodo_mutante_en_indice(
        &mut self,
        objeto_padre: &Nodo,
        indice: &Nodo,
        metodo: &str,
        argumentos: &[Valor],
        linea: usize,
        entorno: Rc<RefCell<Entorno>>,
    ) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        // Evaluar el índice
        let (valor_indice, _) = self.evaluar_con_entorno(indice, entorno.clone())?;
        let indice_usize = match valor_indice {
            Valor::Entero(i) => {
                if i < 0 {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("Índice negativo: {}", i),
                    });
                }
                i as usize
            },
            _ => {
                return Err(ErrorQuetzal::ErrorEjecucion {
                    linea,
                    mensaje: "El índice debe ser un número entero".to_string(),
                });
            }
        };

        // Manejar solo el caso simple: variable[indice].metodo()
        if let Nodo::Identificador(nombre_var) = objeto_padre {
            // Función recursiva para buscar y modificar la variable en entornos padre
            fn modificar_variable_recursiva(
                entorno: Rc<RefCell<Entorno>>,
                nombre_var: &str,
                indice_usize: usize,
                metodo: &str,
                argumentos: &[Valor],
                linea: usize,
            ) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
                let mut entorno_ref = entorno.borrow_mut();
                
                // Buscar en el entorno actual
                if let Some(variable) = entorno_ref.variables.get_mut(nombre_var) {
                    if !variable.es_mutable() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: format!("No se puede modificar la lista inmutable '{}'", nombre_var),
                        });
                    }
                    
                    if let Valor::Lista(ref mut lista_padre) = variable.valor {
                        if indice_usize >= lista_padre.len() {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: format!("Índice fuera de rango: {} (tamaño: {})", indice_usize, lista_padre.len()),
                            });
                        }
                        
                        // Aplicar el método mutante al elemento de la lista
                        match metodo {
                            "agregar" => {
                                if argumentos.len() != 1 {
                                    return Err(ErrorQuetzal::ErrorEjecucion {
                                        linea,
                                        mensaje: "El método 'agregar' requiere exactamente un argumento".to_string(),
                                    });
                                }
                                
                                if let Valor::Lista(ref mut lista_elemento) = lista_padre[indice_usize] {
                                    lista_elemento.push(argumentos[0].clone());
                                    Ok((Valor::Vacio, ControlFlujo::Ninguno))
                                } else {
                                    Err(ErrorQuetzal::ErrorEjecucion {
                                        linea,
                                        mensaje: "El elemento no es una lista".to_string(),
                                    })
                                }
                            },
                            _ => {
                                Err(ErrorQuetzal::ErrorEjecucion {
                                    linea,
                                    mensaje: format!("Método mutante '{}' no soportado en elementos de lista", metodo),
                                })
                            }
                        }
                    } else {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: format!("'{}' no es una lista", nombre_var),
                        })
                    }
                } else {
                    // Buscar en el entorno padre
                    if let Some(padre) = entorno_ref.padre.clone() {
                        drop(entorno_ref); // Liberar el préstamo antes de la llamada recursiva
                        modificar_variable_recursiva(padre, nombre_var, indice_usize, metodo, argumentos, linea)
                    } else {
                        Err(ErrorQuetzal::VariableNoDefinida {
                            linea,
                            nombre: nombre_var.to_string(),
                        })
                    }
                }
            }
            
            modificar_variable_recursiva(entorno, nombre_var, indice_usize, metodo, argumentos, linea)
        } else {
            Err(ErrorQuetzal::ErrorEjecucion {
                linea,
                mensaje: "Métodos mutantes en accesos anidados complejos no están soportados".to_string(),
            })
        }
    }
}
