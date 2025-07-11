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
    
    /// Crea un nuevo entorno hijo
    pub fn nuevo_hijo(padre: Rc<RefCell<Entorno>>) -> Self {
        Entorno {
            variables: HashMap::new(),
            funciones: HashMap::new(),
            padre: Some(padre),
        }
    }
    
    /// Define una variable en el entorno
    pub fn definir_variable(&mut self, nombre: String, variable: Variable) -> ResultadoQuetzal<()> {
        // Verificar que no sea una palabra reservada
        if self.es_palabra_reservada(&nombre) {
            return Err(ErrorQuetzal::ErrorEjecucion {
                linea: 0,
                mensaje: format!("'{}' es una palabra reservada y no puede usarse como nombre de variable", nombre),
            });
        }
        
        self.variables.insert(nombre, variable);
        Ok(())
    }
    
    /// Obtiene una variable del entorno
    pub fn obtener_variable(&self, nombre: &str) -> Option<Variable> {
        // Buscar en el entorno actual
        if let Some(variable) = self.variables.get(nombre) {
            return Some(variable.clone());
        }
        
        // Buscar en el entorno padre
        if let Some(padre) = &self.padre {
            return padre.borrow().obtener_variable(nombre);
        }
        
        None
    }
    
    /// Asigna un valor a una variable existente
    pub fn asignar_variable(&mut self, nombre: &str, valor: Valor, linea: usize) -> ResultadoQuetzal<()> {
        // Buscar en el entorno actual
        if let Some(variable) = self.variables.get_mut(nombre) {
            if !variable.es_mutable() {
                return Err(ErrorQuetzal::VariableInmutable {
                    linea,
                    nombre: nombre.to_string(),
                });
            }
            variable.valor = valor;
            return Ok(());
        }
        
        // Buscar en el entorno padre
        if let Some(padre) = &self.padre {
            return padre.borrow_mut().asignar_variable(nombre, valor, linea);
        }
        
        Err(ErrorQuetzal::VariableNoDefinida {
            linea,
            nombre: nombre.to_string(),
        })
    }
    
    /// Define una función en el entorno
    pub fn definir_funcion(&mut self, nombre: String, funcion: FuncionDefinida) {
        self.funciones.insert(nombre, funcion);
    }
    
    /// Obtiene una función del entorno
    pub fn obtener_funcion(&self, nombre: &str) -> Option<FuncionDefinida> {
        // Buscar en el entorno actual
        if let Some(funcion) = self.funciones.get(nombre) {
            return Some(funcion.clone());
        }
        
        // Buscar en el entorno padre
        if let Some(padre) = &self.padre {
            return padre.borrow().obtener_funcion(nombre);
        }
        
        None
    }
    
    /// Verifica si un nombre es una palabra reservada
    fn es_palabra_reservada(&self, nombre: &str) -> bool {
        matches!(nombre,
            "vacio" | "entero" | "cadena" | "bool" | "lista" | "jsn" |
            "mut" | "publico" | "privado" | "libre" |
            "si" | "sino" | "para" | "mientras" | "hacer" | "romper" | "continuar" | "en" |
            "retornar" | "objeto" | "nuevo" | "ambiente" | "asincrono" | "esperar" |
            "intentar" | "atrapar" | "finalmente" | "lanzar" | "excepción" |
            "importar" | "exportar" | "desde" | "como" |
            "verdadero" | "falso" | "y" | "o" | "consola"
        )
    }
}

/// Evaluador que ejecuta el AST
pub struct Evaluador {
    entorno_global: Rc<RefCell<Entorno>>,
}

impl Evaluador {
    /// Crea un nuevo evaluador
    pub fn nuevo() -> Self {
        let mut entorno = Entorno::nuevo();
        
        // Definir funciones y objetos globales predefinidos
        Self::definir_funciones_globales(&mut entorno);
        
        Evaluador {
            entorno_global: Rc::new(RefCell::new(entorno)),
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
                    cuerpo: *cuerpo.clone(),
                    es_asincrona: *es_asincrona,
                };
                
                entorno.borrow_mut().definir_funcion(nombre.clone(), funcion);
                
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            },
            
            Nodo::Literal(valor) => {
                Ok((valor.clone(), ControlFlujo::Ninguno))
            },
            
            Nodo::Identificador(nombre) => {
                if let Some(variable) = entorno.borrow().obtener_variable(nombre) {
                    Ok((variable.valor, ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::VariableNoDefinida {
                        linea: 0,
                        nombre: nombre.clone(),
                    })
                }
            },
            
            Nodo::OperacionBinaria { izquierdo, operador, derecho } => {
                let (val_izq, _) = self.evaluar_con_entorno(izquierdo, entorno.clone())?;
                let (val_der, _) = self.evaluar_con_entorno(derecho, entorno.clone())?;
                
                let resultado = self.evaluar_operacion_binaria(&val_izq, operador, &val_der)?;
                Ok((resultado, ControlFlujo::Ninguno))
            },
            
            Nodo::OperacionUnaria { operador, operando } => {
                let (valor, _) = self.evaluar_con_entorno(operando, entorno.clone())?;
                let resultado = self.evaluar_operacion_unaria(operador, &valor)?;
                Ok((resultado, ControlFlujo::Ninguno))
            },
            
            Nodo::LlamadaFuncion { nombre, argumentos, linea } => {
                self.evaluar_llamada_funcion(nombre, argumentos, *linea, entorno)
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
            
            _ => {
                // Otros nodos no implementados aún
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            }
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
            "==" => Ok(Valor::Bool(self.valores_iguales(izquierdo, derecho))),
            "!=" => Ok(Valor::Bool(!self.valores_iguales(izquierdo, derecho))),
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
    
    /// Compara dos valores para igualdad
    fn valores_iguales(&self, a: &Valor, b: &Valor) -> bool {
        match (a, b) {
            (Valor::Vacio, Valor::Vacio) => true,
            (Valor::Entero(a), Valor::Entero(b)) => a == b,
            (Valor::Numero(a), Valor::Numero(b)) => a == b,
            (Valor::Entero(a), Valor::Numero(b)) => *a as f64 == *b,
            (Valor::Numero(a), Valor::Entero(b)) => *a == *b as f64,
            (Valor::Cadena(a), Valor::Cadena(b)) => a == b,
            (Valor::Bool(a), Valor::Bool(b)) => a == b,
            _ => false,
        }
    }
    
    /// Evalúa una llamada a función
    fn evaluar_llamada_funcion(&mut self, nombre: &str, argumentos: &[Nodo], linea: usize, entorno: Rc<RefCell<Entorno>>) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        // Funciones especiales de consola
        if nombre.starts_with("consola.") {
            return self.evaluar_funcion_consola(nombre, argumentos, entorno);
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
    
    /// Evalúa métodos de conversión (como variable.cadena(), variable.numero())
    fn evaluar_metodo_conversion(&mut self, nombre: &str, _argumentos: &[Nodo], entorno: Rc<RefCell<Entorno>>) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        let partes: Vec<&str> = nombre.split('.').collect();
        if partes.len() != 2 {
            return Err(ErrorQuetzal::ErrorEjecucion {
                linea: 0,
                mensaje: format!("Formato de método de conversión inválido: {}", nombre),
            });
        }
        
        let nombre_variable = partes[0];
        let metodo = partes[1];
        
        // Obtener el valor de la variable
        let valor = if let Some(variable) = entorno.borrow().obtener_variable(nombre_variable) {
            variable.valor
        } else {
            return Err(ErrorQuetzal::VariableNoDefinida {
                linea: 0,
                nombre: nombre_variable.to_string(),
            });
        };
        
        // Aplicar el método de conversión
        let resultado = match metodo {
            "cadena" => Valor::Cadena(valor.a_cadena()),
            "entero" => {
                match valor.a_entero() {
                    Ok(n) => Valor::Entero(n),
                    Err(_msg) => return Err(ErrorQuetzal::ConversionInvalida {
                        linea: 0,
                        tipo_origen: valor.tipo_como_cadena().to_string(),
                        tipo_destino: "entero".to_string(),
                    }),
                }
            },
            "numero" => {
                match valor.a_numero() {
                    Ok(n) => Valor::Numero(n),
                    Err(_msg) => return Err(ErrorQuetzal::ConversionInvalida {
                        linea: 0,
                        tipo_origen: valor.tipo_como_cadena().to_string(),
                        tipo_destino: "número".to_string(),
                    }),
                }
            },
            "bool" => Valor::Bool(valor.a_bool()),
            _ => {
                return Err(ErrorQuetzal::FuncionNoDefinida {
                    linea: 0,
                    nombre: formato!("{}.{}", nombre_variable, metodo),
                });
            }
        };
        
        Ok((resultado, ControlFlujo::Ninguno))
    }
    
    /// Evalúa acceso a miembro de objeto
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
            _ => Err(ErrorQuetzal::ErrorTipo {
                linea,
                mensaje: format!("No se puede acceder a miembros de {}", objeto.tipo_como_cadena()),
            }),
        }
    }
}
