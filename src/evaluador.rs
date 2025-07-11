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
        }
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
        if let Nodo::LlamadaFuncion { nombre, .. } = nodo {
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
            
            Nodo::DeclaracionVariable { nombre, tipo_dato, es_mutable, valor, linea: _ } => {
                // Evaluar el valor inicial si existe
                let valor_inicial = if let Some(expr_valor) = valor {
                    let (val, _) = self.evaluar_con_entorno(expr_valor, entorno.clone())?;
                    val
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
                
                let tipo_variable = if *es_mutable {
                    TipoVariable::Mutable
                } else {
                    TipoVariable::Inmutable
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
            
            Nodo::Retornar { valor, linea: _ } => {
                let valor_retorno = if let Some(expr) = valor {
                    let (valor_evaluado, _) = self.evaluar_con_entorno(expr, entorno)?;
                    valor_evaluado
                } else {
                    Valor::Vacio
                };
                
                Ok((valor_retorno.clone(), ControlFlujo::Retornar(valor_retorno)))
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
                
                // Asignar el resultado
                {
                    let mut entorno_ref = entorno.borrow_mut();
                    if let Some(variable) = entorno_ref.variables.get_mut(nombre) {
                        if variable.tipo_variable == TipoVariable::Inmutable {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea: *linea,
                                mensaje: format!("No se puede reasignar la variable inmutable '{}'", nombre),
                            });
                        }
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
            
            Nodo::BucleParaCada { variable, iterable, cuerpo, linea: _ } => {
                let (valor_iterable, _) = self.evaluar_con_entorno(iterable, entorno.clone())?;
                
                match valor_iterable {
                    Valor::Lista(elementos) => {
                        // Crear nuevo entorno para el bucle
                        let entorno_bucle = Rc::new(RefCell::new(Entorno::con_padre(entorno.clone())));
                        
                        for elemento in elementos {
                            // Definir la variable del bucle
                            let variable_bucle = Variable::nueva(
                                variable.clone(),
                                elemento,
                                TipoVariable::Inmutable,
                                "auto".to_string(),
                            );
                            entorno_bucle.borrow_mut().definir_variable(variable.clone(), variable_bucle)?;
                            
                            // Ejecutar el cuerpo del bucle
                            let (_, control) = self.evaluar_con_entorno(cuerpo, entorno_bucle.clone())?;
                            
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
            return self.evaluar_metodo_conversion(nombre, argumentos, entorno);
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
            
            // Verificar número de argumentos
            if valores_argumentos.len() != funcion.parametros.len() {
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
                
                let variable = Variable::nueva(
                    parametro.nombre.clone(),
                    valores_argumentos[i].clone(),
                    tipo_variable,
                    parametro.tipo_dato.clone(),
                );
                
                entorno_funcion.borrow_mut().definir_variable(parametro.nombre.clone(), variable)?;
            }
            
            // Ejecutar cuerpo de la función
            let (valor, control) = self.evaluar_con_entorno(&funcion.cuerpo, entorno_funcion)?;
            
            match control {
                ControlFlujo::Retornar(valor_retorno) => Ok((valor_retorno, ControlFlujo::Ninguno)),
                _ => Ok((valor, ControlFlujo::Ninguno)),
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
        // Evaluar primer argumento (mensaje)
        let mensaje = if !argumentos.is_empty() {
            let (valor, _) = self.evaluar_con_entorno(&argumentos[0], entorno)?;
            valor.a_cadena()
        } else {
            String::new()
        };
        
        match nombre {
            "consola.imprimir" => {
                CONSOLA_GLOBAL.imprimir(&mensaje);
            },
            "consola.imprimir_error" => {
                CONSOLA_GLOBAL.imprimir_error(&mensaje);
            },
            "consola.imprimir_advertencia" => {
                CONSOLA_GLOBAL.imprimir_advertencia(&mensaje);
            },
            "consola.imprimir_informacion" => {
                CONSOLA_GLOBAL.imprimir_informacion(&mensaje);
            },
            "consola.imprimir_depurar" => {
                CONSOLA_GLOBAL.imprimir_depurar(&mensaje);
            },
            "consola.imprimir_exito" => {
                CONSOLA_GLOBAL.imprimir_exito(&mensaje);
            },
            "consola.imprimir_alerta" => {
                CONSOLA_GLOBAL.imprimir_alerta(&mensaje);
            },
            "consola.imprimir_confirmacion" => {
                CONSOLA_GLOBAL.imprimir_confirmacion(&mensaje);
            },
            _ => {
                return Err(ErrorQuetzal::FuncionNoDefinida {
                    linea: 0,
                    nombre: nombre.to_string(),
                });
            }
        }
        
        Ok((Valor::Vacio, ControlFlujo::Ninguno))
    }
    
    /// Evalúa método de conversión en cadena
    fn evaluar_metodo_conversion(&mut self, nombre_completo: &str, _argumentos: &[Nodo], entorno: Rc<RefCell<Entorno>>) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        let partes: Vec<&str> = nombre_completo.split('.').collect();
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
        
        // Aplicar método
        match metodo {
            "cadena" => Ok((Valor::Cadena(valor.a_cadena()), ControlFlujo::Ninguno)),
            "numero" => {
                match valor {
                    Valor::Entero(n) => Ok((Valor::Numero(n as f64), ControlFlujo::Ninguno)),
                    Valor::Numero(n) => Ok((Valor::Numero(n), ControlFlujo::Ninguno)),
                    Valor::Cadena(s) => {
                        match s.trim().parse::<f64>() {
                            Ok(n) => Ok((Valor::Numero(n), ControlFlujo::Ninguno)),
                            Err(_) => Err(ErrorQuetzal::ErrorConversion {
                                linea: 0,
                                mensaje: "No se puede convertir cadena a número".to_string(),
                            }),
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
            _ => Err(ErrorQuetzal::ErrorEjecucion {
                linea: 0,
                mensaje: format!("Método '{}' no está definido", metodo),
            }),
        }
    }
    
    /// Evalúa una operación binaria
    fn evaluar_operacion_binaria(&self, izquierdo: &Valor, operador: &str, derecho: &Valor) -> ResultadoQuetzal<Valor> {
        match operador {
            "+" => {
                match (izquierdo, derecho) {
                    (Valor::Entero(a), Valor::Entero(b)) => Ok(Valor::Entero(a + b)),
                    (Valor::Numero(a), Valor::Numero(b)) => Ok(Valor::Numero(a + b)),
                    (Valor::Entero(a), Valor::Numero(b)) => Ok(Valor::Numero(*a as f64 + b)),
                    (Valor::Numero(a), Valor::Entero(b)) => Ok(Valor::Numero(a + *b as f64)),
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
                    (Valor::Numero(a), Valor::Numero(b)) => Ok(Valor::Numero(a - b)),
                    (Valor::Entero(a), Valor::Numero(b)) => Ok(Valor::Numero(*a as f64 - b)),
                    (Valor::Numero(a), Valor::Entero(b)) => Ok(Valor::Numero(a - *b as f64)),
                    _ => Err(ErrorQuetzal::ErrorTipo {
                        linea: 0,
                        mensaje: format!("No se puede restar {} y {}", izquierdo.tipo_como_cadena(), derecho.tipo_como_cadena()),
                    }),
                }
            },
            "*" => {
                match (izquierdo, derecho) {
                    (Valor::Entero(a), Valor::Entero(b)) => Ok(Valor::Entero(a * b)),
                    (Valor::Numero(a), Valor::Numero(b)) => Ok(Valor::Numero(a * b)),
                    (Valor::Entero(a), Valor::Numero(b)) => Ok(Valor::Numero(*a as f64 * b)),
                    (Valor::Numero(a), Valor::Entero(b)) => Ok(Valor::Numero(a * *b as f64)),
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
                        } else {
                            Ok(Valor::Numero(*a as f64 / *b as f64))
                        }
                    },
                    (Valor::Numero(a), Valor::Numero(b)) => {
                        if *b == 0.0 {
                            Err(ErrorQuetzal::DivisionPorCero { linea: 0 })
                        } else {
                            Ok(Valor::Numero(a / b))
                        }
                    },
                    (Valor::Entero(a), Valor::Numero(b)) => {
                        if *b == 0.0 {
                            Err(ErrorQuetzal::DivisionPorCero { linea: 0 })
                        } else {
                            Ok(Valor::Numero(*a as f64 / b))
                        }
                    },
                    (Valor::Numero(a), Valor::Entero(b)) => {
                        if *b == 0 {
                            Err(ErrorQuetzal::DivisionPorCero { linea: 0 })
                        } else {
                            Ok(Valor::Numero(a / *b as f64))
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
                    // Nuevos métodos de cadena avanzados
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
                        let encoded = urlencoding::encode(cadena);
                        Ok((Valor::Cadena(encoded.to_string()), ControlFlujo::Ninguno))
                    },
                    "decodificar_uri" => {
                        // Implementación básica de decodificación URI
                        match urlencoding::decode(cadena) {
                            Ok(decoded) => Ok((Valor::Cadena(decoded.to_string()), ControlFlujo::Ninguno)),
                            Err(_) => Err(ErrorQuetzal::ErrorConversion {
                                linea,
                                mensaje: "Error al decodificar URI".to_string(),
                            }),
                        }
                    },
                    // Conversiones de tipo
                    "cadena" => Ok((objeto.clone(), ControlFlujo::Ninguno)),
                    "numero" => {
                        match cadena.trim().parse::<f64>() {
                            Ok(n) => Ok((Valor::Numero(n), ControlFlujo::Ninguno)),
                            Err(_) => Err(ErrorQuetzal::ErrorConversion {
                                linea,
                                mensaje: "No se puede convertir cadena a número".to_string(),
                            }),
                        }
                    },
                    "entero" => {
                        match cadena.trim().parse::<i64>() {
                            Ok(n) => Ok((Valor::Entero(n), ControlFlujo::Ninguno)),
                            Err(_) => Err(ErrorQuetzal::ErrorConversion {
                                linea,
                                mensaje: "No se puede convertir cadena a entero".to_string(),
                            }),
                        }
                    },
                    "bool" => {
                        match cadena.to_lowercase().as_str() {
                            "verdadero" | "true" | "1" => Ok((Valor::Bool(true), ControlFlujo::Ninguno)),
                            "falso" | "false" | "0" => Ok((Valor::Bool(false), ControlFlujo::Ninguno)),
                            _ => Err(ErrorQuetzal::ErrorConversion {
                                linea,
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
}
