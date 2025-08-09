// Evaluador del AST para el lenguaje Quetzal
// Ejecuta el Árbol de Sintaxis Abstracta y maneja el entorno de ejecución

use crate::analizador_sintactico::{Nodo, Parametro};
use crate::tipos_datos::{Valor, Variable, TipoVariable};
use crate::errores::{ErrorQuetzal, ResultadoQuetzal};
use crate::consola::CONSOLA_GLOBAL;
use crate::manejador_modulos::ManejadorModulos;
use crate::maquina_virtual::MaquinaVirtualRecursion;
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
    /// Clases definidas
    clases: HashMap<String, ClaseDefinida>,
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

/// Definición de una clase de objeto
#[derive(Debug, Clone)]
pub struct ClaseDefinida {
    pub nombre: String,
    pub miembros_publicos: Vec<Nodo>,
    pub miembros_privados: Vec<Nodo>,
    pub constructor: Option<Nodo>,
    pub propiedades_publicas: Vec<String>, // Lista de nombres de propiedades públicas
    pub propiedades_privadas: Vec<String>, // Lista de nombres de propiedades privadas
    pub metodos_publicos: Vec<String>, // Lista de nombres de métodos públicos
    pub metodos_privados: Vec<String>, // Lista de nombres de métodos privados
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
            clases: HashMap::new(),
            padre: None,
        }
    }
    
    /// Crea un nuevo entorno con un padre
    pub fn nuevo_hijo(padre: Rc<RefCell<Entorno>>) -> Self {
        Entorno {
            variables: HashMap::new(),
            funciones: HashMap::new(),
            clases: HashMap::new(),
            padre: Some(padre),
        }
    }
    
    /// Crea un nuevo entorno con un padre
    pub fn con_padre(padre: Rc<RefCell<Entorno>>) -> Self {
        Entorno {
            variables: HashMap::new(),
            funciones: HashMap::new(),
            clases: HashMap::new(),
            padre: Some(padre),
        }
    }
    
    /// Define una nueva variable
    pub fn definir_variable(&mut self, nombre: String, variable: Variable) -> ResultadoQuetzal<()> {
        // Permitir 'ambiente' como variable especial en objetos
        if !self.es_nombre_valido(&nombre) && nombre != "ambiente" {
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
    
    /// Actualiza una variable existente en el entorno donde está definida
    pub fn actualizar_variable(&mut self, nombre: &str, nueva_variable: Variable) -> bool {
        if self.variables.contains_key(nombre) {
            self.variables.insert(nombre.to_string(), nueva_variable);
            true
        } else if let Some(ref padre) = self.padre {
            padre.borrow_mut().actualizar_variable(nombre, nueva_variable)
        } else {
            false
        }
    }
    
    /// Obtiene una referencia mutable a una variable en el entorno donde está definida
    pub fn obtener_variable_mut(&mut self, nombre: &str) -> Option<&mut Variable> {
        if self.variables.contains_key(nombre) {
            self.variables.get_mut(nombre)
        } else {
            // No podemos devolver una referencia mutable de un padre debido a borrow checker
            // En su lugar, debemos usar actualizar_variable para casos de mutación
            None
        }
    }
    
    /// Obtiene los nombres de todas las variables definidas en el entorno actual
    pub fn obtener_todas_las_variables(&self) -> Vec<String> {
        let mut variables: Vec<String> = self.variables.keys().cloned().collect();
        
        // También incluir variables de entornos padre
        if let Some(ref padre) = self.padre {
            let variables_padre = padre.borrow().obtener_todas_las_variables();
            for var in variables_padre {
                if !variables.contains(&var) {
                    variables.push(var);
                }
            }
        }
        
        variables.sort();
        variables
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
    
    /// Define una nueva clase
    pub fn definir_clase(&mut self, nombre: String, clase: ClaseDefinida) -> ResultadoQuetzal<()> {
        if !self.es_nombre_valido(&nombre) {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: 0,
                mensaje: format!("Nombre de clase inválido: {}", nombre),
            });
        }
        
        self.clases.insert(nombre, clase);
        Ok(())
    }
    
    /// Obtiene una clase del entorno actual o de los padres
    pub fn obtener_clase(&self, nombre: &str) -> Option<ClaseDefinida> {
        if let Some(clase) = self.clases.get(nombre) {
            Some(clase.clone())
        } else if let Some(ref padre) = self.padre {
            padre.borrow().obtener_clase(nombre)
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
            "vacio", "entero", "texto", "log", "lista", "jsn",
            "si", "sino", "para", "mientras", "hacer", "romper", "continuar",
            "retornar", "objeto", "nuevo", "ambiente", "asincrono", "esperar",
            "intentar", "capturar", "finalmente", "lanzar", "excepcion",
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
    profundidad_recursion: usize, // Solo para estadísticas/debugging
    dentro_de_funcion: bool,
    dentro_de_constructor: bool, // Indica si estamos ejecutando un constructor
    dentro_de_metodo_clase: bool, // Indica si estamos ejecutando un método de clase
    manejador_modulos: Option<ManejadorModulos>,
    ruta_archivo_actual: Option<String>, // Rastrea el archivo que se está evaluando actualmente
    vm_recursion: MaquinaVirtualRecursion,
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
            dentro_de_funcion: false,
            dentro_de_constructor: false,
            dentro_de_metodo_clase: false,
            manejador_modulos: None,
            ruta_archivo_actual: None,
            vm_recursion: MaquinaVirtualRecursion::nueva(),
        }
    }
    
    /// Crea un nuevo evaluador con manejador de módulos
    pub fn nuevo_con_modulos(ruta_principal: &str) -> ResultadoQuetzal<Self> {
        let mut evaluador = Self::nuevo();
        let manejador = ManejadorModulos::nuevo(ruta_principal, evaluador.entorno_global.clone())?;
        evaluador.manejador_modulos = Some(manejador);
        evaluador.ruta_archivo_actual = Some(ruta_principal.to_string());
        Ok(evaluador)
    }
    
    /// Establece la ruta del archivo que se está evaluando actualmente
    pub fn establecer_ruta_archivo_actual(&mut self, ruta: &str) {
        self.ruta_archivo_actual = Some(ruta.to_string());
    }
    
    /// Obtiene la ruta del archivo que se está evaluando actualmente
    pub fn obtener_ruta_archivo_actual(&self) -> Option<&str> {
        self.ruta_archivo_actual.as_deref()
    }
    
    /// Establece temporalmente el manejador de módulos
    pub fn establecer_manejador_modulos(&mut self, manejador: Option<ManejadorModulos>) {
        self.manejador_modulos = manejador;
    }
    
    /// Toma el manejador de módulos temporalmente
    pub fn tomar_manejador_modulos(&mut self) -> Option<ManejadorModulos> {
        self.manejador_modulos.take()
    }
    
    /// Compara si dos valores son iguales (para detectar cambios en objetos)
    fn objeto_fue_modificado(&self, original: &Valor, actual: &Valor) -> bool {
        match (original, actual) {
            (Valor::Objeto { propiedades: p1, .. }, Valor::Objeto { propiedades: p2, .. }) => {
                // Comparación rápida: si el tamaño es diferente, fue modificado
                if p1.len() != p2.len() {
                    return true;
                }
                
                // Comparación de las claves - si las claves son diferentes, fue modificado
                let claves1: std::collections::HashSet<_> = p1.keys().collect();
                let claves2: std::collections::HashSet<_> = p2.keys().collect();
                if claves1 != claves2 {
                    return true;
                }
                
                // Comparación superficial de valores - solo tipos básicos para evitar recursión
                for (clave, valor1) in p1.iter() {
                    if let Some(valor2) = p2.get(clave) {
                        if self.valores_diferentes_superficial(valor1, valor2) {
                            return true;
                        }
                    }
                }
                
                false
            },
            _ => !self.valores_son_iguales(original, actual),
        }
    }
    
    fn valores_diferentes_superficial(&self, a: &Valor, b: &Valor) -> bool {
        match (a, b) {
            (Valor::Entero(a), Valor::Entero(b)) => a != b,
            (Valor::Numero(a), Valor::Numero(b)) => (a - b).abs() > f64::EPSILON,
            (Valor::Texto(a), Valor::Texto(b)) => a != b,
            (Valor::Log(a), Valor::Log(b)) => a != b,
            (Valor::Vacio, Valor::Vacio) => false,
            (Valor::Nulo, Valor::Nulo) => false,
            (Valor::Lista(a), Valor::Lista(b)) => a.len() != b.len(), // Solo comparar longitud
            (Valor::Json(a), Valor::Json(b)) => a.len() != b.len(), // Solo comparar longitud
            (Valor::Objeto { clase: c1, .. }, Valor::Objeto { clase: c2, .. }) => c1 != c2, // Solo comparar clase
            _ => true, // Tipos diferentes
        }
    }
    
    fn valores_son_iguales(&self, a: &Valor, b: &Valor) -> bool {
        self.valores_son_iguales_con_profundidad(a, b, 0, 10) // máximo 10 niveles de profundidad
    }
    
    fn valores_son_iguales_con_profundidad(&self, a: &Valor, b: &Valor, profundidad: usize, max_profundidad: usize) -> bool {
        // Evitar recursión infinita
        if profundidad > max_profundidad {
            return false;
        }
        
        match (a, b) {
            (Valor::Objeto { clase: c1, propiedades: p1, .. }, Valor::Objeto { clase: c2, propiedades: p2, .. }) => {
                if c1 != c2 || p1.len() != p2.len() {
                    return false;
                }
                
                // Comparar propiedades con control de profundidad
                for (k, v1) in p1.iter() {
                    if let Some(v2) = p2.get(k) {
                        if !self.valores_son_iguales_con_profundidad(v1, v2, profundidad + 1, max_profundidad) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            },
            (Valor::Entero(a), Valor::Entero(b)) => a == b,
            (Valor::Numero(a), Valor::Numero(b)) => (a - b).abs() < f64::EPSILON,
            (Valor::Texto(a), Valor::Texto(b)) => a == b,
            (Valor::Log(a), Valor::Log(b)) => a == b,
            (Valor::Vacio, Valor::Vacio) => true,
            (Valor::Nulo, Valor::Nulo) => true,
            _ => false,
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
    
    /// Obtiene el valor de una variable del entorno global (para pruebas)
    pub fn obtener_valor_variable(&self, nombre: &str) -> Option<Valor> {
        self.entorno_global.borrow().variables.get(nombre).map(|v| v.valor.clone())
    }
    
    /// Obtiene estadísticas de la VM (memoria, profundidad, etc.)
    pub fn obtener_estadisticas_vm(&self) -> String {
        format!(
            "VM Universal - Límite actual: {} | Memoria: {}KB | Funciones activas: {}",
            self.vm_recursion.obtener_limite_actual(),
            self.vm_recursion.obtener_memoria_actual() / 1024,
            self.vm_recursion.obtener_profundidad_actual()
        )
    }
    
    /// Evalúa un nodo del AST
    pub fn evaluar(&mut self, nodo: &Nodo) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        self.evaluar_con_entorno(nodo, self.entorno_global.clone())
    }
    
    /// Evalúa un nodo con un entorno específico
    fn evaluar_con_entorno(&mut self, nodo: &Nodo, entorno: Rc<RefCell<Entorno>>) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        // Usar stacker para crecer el stack automáticamente cuando sea necesario
        // Valores más grandes para evitar stack overflow
        stacker::maybe_grow(1024 * 1024, 8 * 1024 * 1024, || {
            self.evaluar_nodo_interno(nodo, entorno)
        })
    }

    /// Evaluación interna del nodo
    fn evaluar_nodo_interno(&mut self, nodo: &Nodo, entorno: Rc<RefCell<Entorno>>) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
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
            
            Nodo::DeclaracionVariable { nombre, tipo_dato, es_variable, valor, linea } => {
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
                        "texto" => Valor::Texto(String::new()),
                        "log" => Valor::Log(false),
                        "lista" => Valor::Lista(Vec::new()),
                        "jsn" => Valor::Json(HashMap::new()),
                        _ => Valor::Vacio,
                    }
                };
                
                // En Quetzal, las variables son inmutables por defecto a menos que se especifique explícitamente como variables
                // Excepción: variables JSON y lista temporales dentro de métodos de objeto son mutables por defecto
                let es_variable_especial = *es_variable || 
                    ((tipo_dato == "jsn" || tipo_dato == "lista") && self.dentro_de_funcion && valor.is_some());
                
                let tipo_variable = if es_variable_especial {
                    TipoVariable::Variable
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
            
            Nodo::DeclaracionObjeto { nombre, miembros, linea } => {
                self.evaluar_declaracion_objeto(nombre, miembros, *linea, entorno)
            },
            
            Nodo::DeclaracionImportar { elementos, ruta, linea } => {
                // Manejar importaciones usando métodos auxiliares para evitar problemas de borrowing
                self.manejar_importacion(elementos, ruta, *linea)?;
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            },
            
            Nodo::DeclaracionExportar { elementos: _, linea: _ } => {
                // Las exportaciones ahora se manejan externamente por el manejador de módulos
                // durante la evaluación del módulo. Por ahora, simplemente retornamos éxito.
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
            
            Nodo::CreacionObjeto { nombre_clase, argumentos, linea } => {
                self.evaluar_creacion_objeto(nombre_clase, argumentos, *linea, entorno)
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
                // Usar la VM híbrida para manejar la recursión automáticamente cuando esté configurada
                // Por ahora, usar el método estándar con stacker como respaldo
                if self.profundidad_recursion >= 4096 {
                    return Err(ErrorQuetzal::ErrorEjecucion { linea: *linea, mensaje: "profundidad máxima de llamadas excedida (límite: 4096)".to_string() });
                }
                self.profundidad_recursion += 1;
                let resultado = self.evaluar_llamada_funcion(nombre, argumentos, *linea, entorno);
                self.profundidad_recursion = self.profundidad_recursion.saturating_sub(1);
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
                if metodo == "agregar" || metodo == "quitar" || metodo == "limpiar" || metodo == "insertar" || metodo == "sacar" || metodo == "sacar_ultimo" {
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
                
                // Métodos que no modifican (como longitud, primero, ultimo, etc.) o métodos de objeto
                let (valor_objeto_original, _) = self.evaluar_con_entorno(objeto, entorno.clone())?;
                
                // Verificar si es un método de objeto que puede modificar el objeto
                if let Valor::Objeto { .. } = valor_objeto_original {
                    if let Nodo::Identificador(nombre_var) = objeto.as_ref() {
                        // Es un método de objeto llamado en una variable - usar versión especial que puede actualizar la variable
                        return self.evaluar_metodo_en_valor_con_actualizacion(&valor_objeto_original, metodo, &args_evaluados, *linea, entorno, Some(nombre_var.clone()));
                    }
                }
                
                // Para otros casos (no objetos o no variables), comportamiento normal
                self.evaluar_metodo_en_valor(&valor_objeto_original, metodo, &args_evaluados, *linea)
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
                    
                    // Actualizar variable existente en su entorno original
                    let nueva_variable = Variable::nueva(
                        nombre.clone(),
                        valor_evaluado.clone(),
                        var_actual.tipo_variable,
                        var_actual.tipo_dato.clone(),
                    );
                    
                    if !entorno.borrow_mut().actualizar_variable(nombre, nueva_variable) {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea: *linea,
                            mensaje: format!("Error interno: no se pudo actualizar la variable '{}'", nombre),
                        });
                    }
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
                
                // Función auxiliar para asignar a índice anidado
                self.asignar_indice_recursivo_universal(objeto, valor_indice, nuevo_valor.clone(), entorno, *linea)?;
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
                        
                        // Verificar que la variable sea un objeto JSON o un objeto personalizado
                        match variable.valor.clone() {
                            Valor::Json(mut mapa) => {
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
                            },
                            Valor::Objeto { clase, mut propiedades, propiedades_publicas, metodos_publicos } => {
                                // Asignar la nueva propiedad a objeto personalizado
                                propiedades.insert(propiedad.clone(), nuevo_valor.clone());
                                
                                // Actualizar la variable en el entorno
                                drop(entorno_ref); // Liberar la referencia inmutable
                                let nueva_variable = Variable::nueva(
                                    nombre_objeto.clone(),
                                    Valor::Objeto { clase, propiedades, propiedades_publicas, metodos_publicos },
                                    variable.tipo_variable,
                                    variable.tipo_dato.clone(),
                                );
                                entorno.borrow_mut().variables.insert(nombre_objeto.clone(), nueva_variable);
                                
                                Ok((nuevo_valor, ControlFlujo::Ninguno))
                            },
                            _ => {
                                Err(ErrorQuetzal::ErrorEjecucion {
                                    linea: *linea,
                                    mensaje: format!("No se puede asignar propiedades a una variable de tipo '{}', debe ser un objeto JSON o un objeto personalizado", 
                                        self.obtener_nombre_tipo(&variable.valor)),
                                })
                            }
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
                // Crear nuevo entorno para el bucle (para la variable de inicialización)
                let entorno_bucle = Rc::new(RefCell::new(Entorno::con_padre(entorno.clone())));
                
                // Ejecutar inicialización si existe (en el entorno del bucle)
                if let Some(init) = inicializacion {
                    self.evaluar_con_entorno(init, entorno_bucle.clone())?;
                }
                
                loop {
                    // Evaluar condición si existe (en el entorno del bucle)
                    if let Some(cond) = condicion {
                        let (valor_condicion, _) = self.evaluar_con_entorno(cond, entorno_bucle.clone())?;
                        if !valor_condicion.a_bool() {
                            break;
                        }
                    }
                    
                    // Crear nuevo entorno para cada iteración (hijo del entorno del bucle)
                    let entorno_iteracion = Rc::new(RefCell::new(Entorno::con_padre(entorno_bucle.clone())));
                    
                    // Ejecutar cuerpo del bucle en el entorno de iteración
                    let (_, control) = self.evaluar_con_entorno(cuerpo, entorno_iteracion)?;
                    
                    match control {
                        ControlFlujo::Romper => break,
                        ControlFlujo::Continuar => {
                            // Ejecutar incremento antes de continuar (en el entorno del bucle)
                            if let Some(inc) = incremento {
                                self.evaluar_con_entorno(inc, entorno_bucle.clone())?;
                            }
                            continue;
                        },
                        ControlFlujo::Retornar(_) => return Ok((Valor::Vacio, control)),
                        _ => {},
                    }
                    
                    // Ejecutar incremento al final de cada iteración (en el entorno del bucle)
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
                    
                    // No crear entorno hijo para bucles - usar el entorno actual para permitir 
                    // modificaciones de variables existentes
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
                    // No crear entorno hijo para bucles - usar el entorno actual
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
                    
                    (Valor::Texto(cadena), Valor::Entero(i)) => {
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
                        
                        Ok((Valor::Texto(chars[indice_usize].to_string()), ControlFlujo::Ninguno))
                    },
                    
                    (Valor::Json(mapa), Valor::Texto(clave)) => {
                        if let Some(valor) = mapa.get(clave) {
                            Ok((valor.clone(), ControlFlujo::Ninguno))
                        } else {
                            // Devolver nulo en lugar de error cuando la propiedad no existe
                            Ok((Valor::Nulo, ControlFlujo::Ninguno))
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
                    Valor::Log(b) => b,
                    Valor::Entero(n) => n != 0,
                    Valor::Numero(n) => n != 0.0,
                    Valor::Texto(s) => !s.is_empty(),
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
            
            Nodo::Lanzar { excepcion, linea } => {
                // Evaluar la expresión de la excepción
                let (valor_excepcion, _) = self.evaluar_con_entorno(excepcion, entorno)?;
                
                // Convertir a texto si no es ya texto
                let mensaje = match valor_excepcion {
                    Valor::Texto(msg) => msg,
                    _ => valor_excepcion.a_cadena(),
                };
                
                // Lanzar el error
                return Err(ErrorQuetzal::ErrorEjecucion {
                    linea: *linea,
                    mensaje,
                });
            },
            
            Nodo::BloqueIntentar { bloque_intentar, bloques_capturar, bloque_finalmente, linea } => {
                // Evaluar el bloque intentar
                let resultado_intentar = self.evaluar_con_entorno(bloque_intentar, entorno.clone());
                
                // Variable para almacenar el resultado final
                let mut resultado_final = Ok((Valor::Vacio, ControlFlujo::Ninguno));
                
                match resultado_intentar {
                    Ok((valor, control)) => {
                        // Si no hubo error, el resultado es el valor del bloque intentar
                        resultado_final = Ok((valor, control));
                    },
                    Err(error) => {
                        // Hubo error, buscar un bloque capturar apropiado
                        let mut error_manejado = false;
                        
                        for bloque_capturar in bloques_capturar {
                            // Por simplicidad, por ahora capturamos cualquier tipo de error con cualquier tipo de excepción
                            // TODO: Implementar verificación específica de tipos de excepción
                            
                            // Crear nuevo entorno para el bloque capturar
                            let entorno_capturar = Rc::new(RefCell::new(Entorno::con_padre(entorno.clone())));
                            
                            // Crear objeto de excepción simple
                            let mensaje_error = match &error {
                                ErrorQuetzal::ErrorSintaxis { mensaje, .. } => mensaje.clone(),
                                ErrorQuetzal::ErrorTipo { mensaje, .. } => mensaje.clone(),
                                ErrorQuetzal::ErrorEjecucion { mensaje, .. } => mensaje.clone(),
                                ErrorQuetzal::VariableNoDefinida { nombre, .. } => format!("Variable no definida: {}", nombre),
                                ErrorQuetzal::FuncionNoDefinida { nombre, .. } => format!("Función no definida: {}", nombre),
                                ErrorQuetzal::DivisionPorCero { .. } => "División por cero".to_string(),
                                _ => "Error desconocido".to_string(),
                            };
                            
                            let mut propiedades_excepcion = HashMap::new();
                            propiedades_excepcion.insert("mensaje".to_string(), Valor::Texto(mensaje_error));
                            propiedades_excepcion.insert("llamadas".to_string(), Valor::Lista(vec![])); // Lista vacía por simplicidad
                            
                            let excepcion = Valor::Json(propiedades_excepcion);
                            
                            // Definir la variable de excepción en el entorno del bloque capturar
                            let variable_excepcion = Variable::nueva(
                                bloque_capturar.nombre_variable.clone(),
                                excepcion,
                                TipoVariable::Inmutable,
                                "excepcion".to_string(),
                            );
                            entorno_capturar.borrow_mut().definir_variable(
                                bloque_capturar.nombre_variable.clone(), 
                                variable_excepcion
                            )?;
                            
                            // Evaluar el bloque capturar
                            resultado_final = self.evaluar_con_entorno(&bloque_capturar.bloque, entorno_capturar);
                            error_manejado = true;
                            break; // Solo ejecutar el primer bloque capturar que coincida
                        }
                        
                        // Si no se manejó el error, propagarlo
                        if !error_manejado {
                            resultado_final = Err(error);
                        }
                    }
                }
                
                // Ejecutar bloque finalmente si existe
                if let Some(bloque_fin) = bloque_finalmente {
                    let _resultado_finalmente = self.evaluar_con_entorno(bloque_fin, entorno);
                    // Ignoramos errores en el bloque finalmente para mantener el error original
                    // En una implementación completa, los errores en finalmente deberían reemplazar el error original
                }
                
                resultado_final
            },
            
            _ => {
                // Otros nodos no implementados aún
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            }
        }
    }
    
    /// Evalúa una llamada a función
    pub fn evaluar_llamada_funcion(&mut self, nombre: &str, argumentos: &[Nodo], linea: usize, entorno: Rc<RefCell<Entorno>>) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        // Detectar métodos de objeto (formato: "variable.metodo")
        if let Some(punto_pos) = nombre.rfind('.') {
            let nombre_variable = &nombre[..punto_pos];
            let nombre_metodo = &nombre[punto_pos + 1..];
            
            // Verificar si la variable es un objeto
            let entorno_ref = entorno.borrow();
            if let Some(variable) = entorno_ref.obtener_variable(nombre_variable) {
                if let Valor::Objeto { .. } = variable.valor {
                    drop(entorno_ref); // Liberar referencia
                    
                    // Evaluar argumentos
                    let mut args_evaluados = Vec::new();
                    for arg in argumentos {
                        let (valor_arg, _) = self.evaluar_con_entorno(arg, entorno.clone())?;
                        args_evaluados.push(valor_arg);
                    }
                    
                    // Usar la función especial para métodos de objeto
                    return self.evaluar_metodo_en_valor_con_actualizacion(&variable.valor, nombre_metodo, &args_evaluados, linea, entorno, Some(nombre_variable.to_string()));
                }
            }
        }
        
        // Verificar funciones de consola
        if nombre.starts_with("consola.") {
            return self.evaluar_funcion_consola(nombre, argumentos, linea, entorno);
        }
        
        // Funciones globales especiales
        match nombre {
            "imprimir" => {
                return self.evaluar_funcion_consola("consola.mostrar", argumentos, linea, entorno);
            },
            "imprimir_error" => {
                return self.evaluar_funcion_consola("consola.mostrar_error", argumentos, linea, entorno);
            },
            "imprimir_advertencia" => {
                return self.evaluar_funcion_consola("consola.mostrar_advertencia", argumentos, linea, entorno);
            },
            "imprimir_informacion" => {
                return self.evaluar_funcion_consola("consola.mostrar_informacion", argumentos, linea, entorno);
            },
            "imprimir_exito" => {
                return self.evaluar_funcion_consola("consola.mostrar_exito", argumentos, linea, entorno);
            },
            "imprimir_depurar" => {
                return self.evaluar_funcion_consola("consola.mostrar_depurar", argumentos, linea, entorno);
            },
            "imprimir_alerta" => {
                return self.evaluar_funcion_consola("consola.mostrar_alerta", argumentos, linea, entorno);
            },
            "imprimir_confirmacion" => {
                return self.evaluar_funcion_consola("consola.mostrar_confirmacion", argumentos, linea, entorno);
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
                if nombre_metodo == "agregar" || nombre_metodo == "quitar" || nombre_metodo == "limpiar" || nombre_metodo == "insertar" || nombre_metodo == "sacar" || nombre_metodo == "sacar_ultimo" {
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
                let tipo_variable = if parametro.es_variable {
                    TipoVariable::Variable
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
            
            // Ejecutar cuerpo de la función con trampolina para evitar stack overflow
            let estado_anterior = self.dentro_de_funcion;
            self.dentro_de_funcion = true;
            let resultado = self.evaluar_con_trampolina(&funcion.cuerpo, entorno_funcion);
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
    
    /// Evalúa un nodo usando stacker para evitar stack overflow en recursión
    fn evaluar_con_trampolina(&mut self, nodo: &Nodo, entorno: Rc<RefCell<Entorno>>) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        // Usar stacker para crecer el stack automáticamente cuando sea necesario
        stacker::maybe_grow(32 * 1024, 1024 * 1024, || {
            self.evaluar_con_entorno(nodo, entorno)
        })
    }
    
    /// Evalúa funciones de consola
    fn evaluar_funcion_consola(&mut self, nombre: &str, argumentos: &[Nodo], linea: usize, entorno: Rc<RefCell<Entorno>>) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        match nombre {
            "consola.imprimir" | "consola.mostrar" => {
                // Evaluar primer argumento (mensaje)
                let mensaje = if !argumentos.is_empty() {
                    let (valor, _) = self.evaluar_con_entorno(&argumentos[0], entorno)?;
                    valor.a_cadena()
                } else {
                    String::new()
                };
                CONSOLA_GLOBAL.mostrar(&mensaje);
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            },
            "consola.imprimir_error" | "consola.mostrar_error" => {
                let mensaje = if !argumentos.is_empty() {
                    let (valor, _) = self.evaluar_con_entorno(&argumentos[0], entorno)?;
                    valor.a_cadena()
                } else {
                    String::new()
                };
                CONSOLA_GLOBAL.mostrar_error(&mensaje);
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            },
            "consola.imprimir_advertencia" | "consola.mostrar_advertencia" => {
                let mensaje = if !argumentos.is_empty() {
                    let (valor, _) = self.evaluar_con_entorno(&argumentos[0], entorno)?;
                    valor.a_cadena()
                } else {
                    String::new()
                };
                CONSOLA_GLOBAL.mostrar_advertencia(&mensaje);
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            },
            "consola.imprimir_informacion" | "consola.mostrar_informacion" => {
                let mensaje = if !argumentos.is_empty() {
                    let (valor, _) = self.evaluar_con_entorno(&argumentos[0], entorno)?;
                    valor.a_cadena()
                } else {
                    String::new()
                };
                CONSOLA_GLOBAL.mostrar_informacion(&mensaje);
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            },
            "consola.imprimir_depurar" | "consola.mostrar_depurar" => {
                let mensaje = if !argumentos.is_empty() {
                    let (valor, _) = self.evaluar_con_entorno(&argumentos[0], entorno)?;
                    valor.a_cadena()
                } else {
                    String::new()
                };
                CONSOLA_GLOBAL.mostrar_depurar(&mensaje);
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            },
            "consola.imprimir_exito" | "consola.mostrar_exito" => {
                let mensaje = if !argumentos.is_empty() {
                    let (valor, _) = self.evaluar_con_entorno(&argumentos[0], entorno)?;
                    valor.a_cadena()
                } else {
                    String::new()
                };
                CONSOLA_GLOBAL.mostrar_exito(&mensaje);
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            },
            "consola.imprimir_alerta" | "consola.mostrar_alerta" => {
                let mensaje = if !argumentos.is_empty() {
                    let (valor, _) = self.evaluar_con_entorno(&argumentos[0], entorno)?;
                    valor.a_cadena()
                } else {
                    String::new()
                };
                CONSOLA_GLOBAL.mostrar_alerta(&mensaje);
                Ok((Valor::Vacio, ControlFlujo::Ninguno))
            },
            "consola.imprimir_confirmacion" | "consola.mostrar_confirmacion" => {
                let mensaje = if !argumentos.is_empty() {
                    let (valor, _) = self.evaluar_con_entorno(&argumentos[0], entorno)?;
                    valor.a_cadena()
                } else {
                    String::new()
                };
                CONSOLA_GLOBAL.mostrar_confirmacion(&mensaje);
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
                Ok((Valor::Texto(entrada), ControlFlujo::Ninguno))
            },
            "consola.pedir_secreto" => {
                let mensaje = if !argumentos.is_empty() {
                    let (valor, _) = self.evaluar_con_entorno(&argumentos[0], entorno)?;
                    valor.a_cadena()
                } else {
                    String::new()
                };
                let entrada_secreta = CONSOLA_GLOBAL.pedir_secreto(&mensaje);
                Ok((Valor::Texto(entrada_secreta), ControlFlujo::Ninguno))
            },
            _ => Err(ErrorQuetzal::FuncionNoDefinida { linea, nombre: nombre.to_string() })
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
                    Valor::Texto(ref cadena) => {
                        let (resultado, _) = self.evaluar_metodo_cadena(cadena, metodo, &[])?;
                        resultado
                    },
                    _ => {
                        // Aplicar métodos de conversión
                        match metodo {
                            "texto" => Valor::Texto(valor_actual.a_cadena()),
                            "numero" => match valor_actual {
                                Valor::Entero(n) => Valor::Numero(n as f64),
                                Valor::Numero(n) => Valor::Numero(n),
                                Valor::Texto(s) => {
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
                                Valor::Texto(s) => {
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
                            "log" => Valor::Log(valor_actual.a_bool()),
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
                Valor::Texto(ref cadena) => self.evaluar_metodo_cadena(cadena, ultimo_metodo, &args_evaluados),
                _ => {
                    // Aplicar método de conversión final
                    match ultimo_metodo {
                        "texto" => Ok((Valor::Texto(valor_actual.a_cadena()), ControlFlujo::Ninguno)),
                        "numero" => match valor_actual {
                            Valor::Entero(n) => Ok((Valor::Numero(n as f64), ControlFlujo::Ninguno)),
                            Valor::Numero(n) => Ok((Valor::Numero(n), ControlFlujo::Ninguno)),
                            Valor::Texto(s) => {
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
                            Valor::Texto(s) => {
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
                        "log" => Ok((Valor::Log(valor_actual.a_bool()), ControlFlujo::Ninguno)),
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
            "texto" => Ok((Valor::Texto(valor.a_cadena()), ControlFlujo::Ninguno)),
            "numero" => {
                match valor {
                    Valor::Entero(n) => Ok((Valor::Numero(n as f64), ControlFlujo::Ninguno)),
                    Valor::Numero(n) => Ok((Valor::Numero(n), ControlFlujo::Ninguno)),
                    Valor::Texto(s) => {
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
                    Valor::Texto(s) => {
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
            "log" => {
                Ok((Valor::Log(valor.a_bool()), ControlFlujo::Ninguno))
            },
            
            // Métodos de cadenas avanzadas
            _ => {
                match &valor {
                    Valor::Texto(cadena) => self.evaluar_metodo_cadena(cadena, metodo, &args_evaluados),
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
            "texto" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'cadena' no acepta argumentos".to_string(),
                    });
                }
                Ok((Valor::Texto(cadena.to_string()), ControlFlujo::Ninguno))
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
            
            "log" => {
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
                Ok((Valor::Log(valor_bool), ControlFlujo::Ninguno))
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
                Ok((Valor::Log(cadena.is_empty()), ControlFlujo::Ninguno))
            },
            
            "contiene" => {
                if argumentos.len() != 1 {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'contiene' requiere exactamente un argumento".to_string(),
                    });
                }
                let patron = argumentos[0].a_cadena();
                Ok((Valor::Log(cadena.contains(&patron)), ControlFlujo::Ninguno))
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
                Ok((Valor::Log(cadena.starts_with(&prefijo)), ControlFlujo::Ninguno))
            },
            
            "termina_con" => {
                if argumentos.len() != 1 {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'termina_con' requiere exactamente un argumento".to_string(),
                    });
                }
                let sufijo = argumentos[0].a_cadena();
                Ok((Valor::Log(cadena.ends_with(&sufijo)), ControlFlujo::Ninguno))
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
                Ok((Valor::Texto(cadena.replace(&buscar, &reemplazar)), ControlFlujo::Ninguno))
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
                    return Ok((Valor::Texto(String::new()), ControlFlujo::Ninguno));
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
                Ok((Valor::Texto(resultado), ControlFlujo::Ninguno))
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
                    .map(|s| Valor::Texto(s.to_string()))
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
                Ok((Valor::Texto(cadena.repeat(veces)), ControlFlujo::Ninguno))
            },
            
            "a_mayusculas" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'a_mayusculas' no acepta argumentos".to_string(),
                    });
                }
                Ok((Valor::Texto(cadena.to_uppercase()), ControlFlujo::Ninguno))
            },
            
            "mayuscula" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'mayuscula' no acepta argumentos".to_string(),
                    });
                }
                Ok((Valor::Texto(cadena.to_uppercase()), ControlFlujo::Ninguno))
            },
            
            "a_minusculas" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'a_minusculas' no acepta argumentos".to_string(),
                    });
                }
                Ok((Valor::Texto(cadena.to_lowercase()), ControlFlujo::Ninguno))
            },
            
            "minuscula" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'minuscula' no acepta argumentos".to_string(),
                    });
                }
                Ok((Valor::Texto(cadena.to_lowercase()), ControlFlujo::Ninguno))
            },
            
            "capitalizar" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'capitalizar' no acepta argumentos".to_string(),
                    });
                }
                let mut chars: Vec<char> = cadena.chars().collect();
                if !chars.is_empty() {
                    chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
                    for i in 1..chars.len() {
                        chars[i] = chars[i].to_lowercase().next().unwrap_or(chars[i]);
                    }
                }
                Ok((Valor::Texto(chars.into_iter().collect()), ControlFlujo::Ninguno))
            },
            
            "titulo" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'titulo' no acepta argumentos".to_string(),
                    });
                }
                let resultado = cadena.split_whitespace()
                    .map(|palabra| {
                        let mut chars: Vec<char> = palabra.chars().collect();
                        if !chars.is_empty() {
                            chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
                            for i in 1..chars.len() {
                                chars[i] = chars[i].to_lowercase().next().unwrap_or(chars[i]);
                            }
                        }
                        chars.into_iter().collect::<String>()
                    })
                    .collect::<Vec<String>>()
                    .join(" ");
                Ok((Valor::Texto(resultado), ControlFlujo::Ninguno))
            },
            
            "recortar" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'recortar' no acepta argumentos".to_string(),
                    });
                }
                Ok((Valor::Texto(cadena.trim().to_string()), ControlFlujo::Ninguno))
            },
            
            "invertir" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'invertir' no acepta argumentos".to_string(),
                    });
                }
                Ok((Valor::Texto(cadena.chars().rev().collect()), ControlFlujo::Ninguno))
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
                Ok((Valor::Log(cadena.to_lowercase() == otra_cadena.to_lowercase()), ControlFlujo::Ninguno))
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
                Ok((Valor::Texto(resultado), ControlFlujo::Ninguno))
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
                        Ok(resultado) => Ok((Valor::Texto(resultado), ControlFlujo::Ninguno)),
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
                Ok((Valor::Texto(resultado), ControlFlujo::Ninguno))
            },
            
            "decodificar_uri" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: "El método 'decodificar_uri' no acepta argumentos".to_string(),
                    });
                }
                match urlencoding::decode(cadena) {
                    Ok(resultado) => Ok((Valor::Texto(resultado.to_string()), ControlFlujo::Ninguno)),
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
                    .map(|linea| Valor::Texto(linea.to_string()))
                    .collect();
                Ok((Valor::Lista(lineas), ControlFlujo::Ninguno))
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
            // Nulo es compatible con cualquier tipo
            (Valor::Nulo, _) => true,
            (Valor::Vacio, "vacio") => true,
            (Valor::Entero(_), "entero") => true,
            (Valor::Numero(_), "número") => true,
            (Valor::Texto(_), "texto") => true,
            (Valor::Log(_), "log") => true,
            (Valor::Lista(_), "lista") => true,
            (Valor::Json(_), "jsn") => true,
            // Objetos personalizados
            (Valor::Objeto { clase, .. }, tipo_esperado) => clase == tipo_esperado,
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
            // Nulo se mantiene como nulo independientemente del tipo destino
            (val @ Valor::Nulo, _) => Ok(val),
            // Sin conversión necesaria
            (val @ Valor::Vacio, "vacio") => Ok(val),
            (val @ Valor::Entero(_), "entero") => Ok(val),
            (val @ Valor::Numero(_), "número") => Ok(val),
            (val @ Valor::Texto(_), "texto") => Ok(val),
            (val @ Valor::Log(_), "log") => Ok(val),
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
    fn obtener_nombre_tipo(&self, valor: &Valor) -> String {
        match valor {
            Valor::Vacio => "vacio".to_string(),
            Valor::Entero(_) => "entero".to_string(),
            Valor::Numero(_) => "número".to_string(),
            Valor::Texto(_) => "texto".to_string(),
            Valor::Log(_) => "log".to_string(),
            Valor::Lista(_) => "lista".to_string(),
            Valor::Json(_) => "jsn".to_string(),
            Valor::Nulo => "nulo".to_string(),
            Valor::Objeto { clase, .. } => clase.clone(),
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
                    (Valor::Texto(a), Valor::Texto(b)) => Ok(Valor::Texto(format!("{}{}", a, b))),
                    (Valor::Texto(a), b) => Ok(Valor::Texto(format!("{}{}", a, b.a_cadena()))),
                    (a, Valor::Texto(b)) => Ok(Valor::Texto(format!("{}{}", a.a_cadena(), b))),
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
            "==" => Ok(Valor::Log(self.valores_iguales(izquierdo, derecho))),
            "!=" => Ok(Valor::Log(!self.valores_iguales(izquierdo, derecho))),
            ">" => self.comparar_valores(izquierdo, derecho, |a, b| a > b),
            "<" => self.comparar_valores(izquierdo, derecho, |a, b| a < b),
            ">=" => self.comparar_valores(izquierdo, derecho, |a, b| a >= b),
            "<=" => self.comparar_valores(izquierdo, derecho, |a, b| a <= b),
            "&&" | "y" => Ok(Valor::Log(izquierdo.a_bool() && derecho.a_bool())),
            "||" | "o" => Ok(Valor::Log(izquierdo.a_bool() || derecho.a_bool())),
            _ => Err(ErrorQuetzal::ErrorEjecucion {
                linea: 0,
                mensaje: format!("Operador binario no soportado: {}", operador),
            }),
        }
    }
    
    /// Evalúa una operación unaria
    fn evaluar_operacion_unaria(&self, operador: &str, operando: &Valor) -> ResultadoQuetzal<Valor> {
        match operador {
            "!" => Ok(Valor::Log(!operando.a_bool())),
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
            (Valor::Nulo, Valor::Nulo) => true,
            (Valor::Entero(a), Valor::Entero(b)) => a == b,
            (Valor::Numero(a), Valor::Numero(b)) => (a - b).abs() < f64::EPSILON,
            (Valor::Entero(a), Valor::Numero(b)) => (*a as f64 - b).abs() < f64::EPSILON,
            (Valor::Numero(a), Valor::Entero(b)) => (a - *b as f64).abs() < f64::EPSILON,
            (Valor::Texto(a), Valor::Texto(b)) => a == b,
            (Valor::Log(a), Valor::Log(b)) => a == b,
            _ => false,
        }
    }
    
    /// Evalúa acceso a miembro de objeto o método
    fn evaluar_acceso_miembro(&self, objeto: &Valor, miembro: &str, linea: usize) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        match objeto {
            Valor::Objeto { propiedades, propiedades_publicas, metodos_publicos, .. } => {
                // Verificar si el miembro es una propiedad pública
                if propiedades_publicas.contains(&miembro.to_string()) {
                    if let Some(valor) = propiedades.get(miembro) {
                        Ok((valor.clone(), ControlFlujo::Ninguno))
                    } else {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: format!("La propiedad pública '{}' no existe en el objeto", miembro),
                        })
                    }
                }
                // Si estamos dentro de un constructor o método de clase, permitir acceso a propiedades privadas también
                else if self.dentro_de_constructor || self.dentro_de_metodo_clase {
                    if let Some(valor) = propiedades.get(miembro) {
                        Ok((valor.clone(), ControlFlujo::Ninguno))
                    } else {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: format!("La propiedad '{}' no existe en el objeto", miembro),
                        })
                    }
                }
                // Verificar si el miembro es un método público (para futuras implementaciones)
                else if metodos_publicos.contains(&miembro.to_string()) {
                    // Por ahora, no se pueden acceder a métodos directamente
                    // Esta funcionalidad se puede implementar más tarde
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El acceso directo a métodos no está implementado aún: '{}'", miembro),
                    })
                }
                // El miembro no es público o no existe
                else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("La propiedad '{}' no existe en el objeto o es privada", miembro),
                    })
                }
            },
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
            Valor::Texto(cadena) => {
                match miembro {
                    "longitud" => Ok((Valor::Entero(cadena.len() as i64), ControlFlujo::Ninguno)),
                    "esta_vacia" => Ok((Valor::Log(cadena.is_empty()), ControlFlujo::Ninguno)),
                    "a_mayusculas" => Ok((Valor::Texto(cadena.to_uppercase()), ControlFlujo::Ninguno)),
                    "a_minusculas" => Ok((Valor::Texto(cadena.to_lowercase()), ControlFlujo::Ninguno)),
                    "recortar" => Ok((Valor::Texto(cadena.trim().to_string()), ControlFlujo::Ninguno)),
                    "invertir" => Ok((Valor::Texto(cadena.chars().rev().collect()), ControlFlujo::Ninguno)),
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
                        Ok((Valor::Texto(encoded), ControlFlujo::Ninguno))
                    },
                    "decodificar_base64" => {
                        // Implementación básica de base64
                        use base64::Engine;
                        match base64::engine::general_purpose::STANDARD.decode(cadena) {
                            Ok(decoded) => {
                                match String::from_utf8(decoded) {
                                    Ok(s) => Ok((Valor::Texto(s), ControlFlujo::Ninguno)),
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
                        Ok((Valor::Texto(encoded), ControlFlujo::Ninguno))
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
                    "log" => {
                        match cadena.to_lowercase().as_str() {
                            "verdadero" | "true" | "1" => Ok((Valor::Log(true), ControlFlujo::Ninguno)),
                            "falso" | "false" | "0" => Ok((Valor::Log(false), ControlFlujo::Ninguno)),
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
                    "texto" => Ok((Valor::Texto(numero.to_string()), ControlFlujo::Ninguno)),
                    "numero" => Ok((Valor::Numero(*numero as f64), ControlFlujo::Ninguno)),
                    "entero" => Ok((objeto.clone(), ControlFlujo::Ninguno)),
                    "log" => Ok((Valor::Log(*numero != 0), ControlFlujo::Ninguno)),
                    _ => Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("La función '{}' no está definida para enteros", miembro),
                    }),
                }
            },

            Valor::Numero(numero) => {
                match miembro {
                    "texto" => Ok((Valor::Texto(numero.to_string()), ControlFlujo::Ninguno)),
                    "numero" => Ok((objeto.clone(), ControlFlujo::Ninguno)),
                    "entero" => Ok((Valor::Entero(*numero as i64), ControlFlujo::Ninguno)),
                    "log" => Ok((Valor::Log(*numero != 0.0), ControlFlujo::Ninguno)),
                    _ => Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("La función '{}' no está definida para números", miembro),
                    }),
                }
            },
            Valor::Log(booleano) => {
                match miembro {
                    "texto" => {
                        let texto = if *booleano { "verdadero" } else { "falso" };
                        Ok((Valor::Texto(texto.to_string()), ControlFlujo::Ninguno))
                    },
                    "numero" => Ok((Valor::Numero(if *booleano { 1.0 } else { 0.0 }), ControlFlujo::Ninguno)),
                    "entero" => Ok((Valor::Entero(if *booleano { 1 } else { 0 }), ControlFlujo::Ninguno)),
                    "log" => Ok((objeto.clone(), ControlFlujo::Ninguno)),
                    _ => Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("La función '{}' no está definida para booleanos", miembro),
                    }),
                }
            },
            Valor::Lista(lista) => {
                match miembro {
                    "longitud" => Ok((Valor::Entero(lista.len() as i64), ControlFlujo::Ninguno)),
                    "esta_vacia" => Ok((Valor::Log(lista.is_empty()), ControlFlujo::Ninguno)),
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
        
        Ok(Valor::Log(comparador(num_a, num_b)))
    }
    
    /// Evalúa un método en un valor específico
    fn evaluar_metodo_en_valor(&mut self, valor: &Valor, metodo: &str, argumentos: &[Valor], linea: usize) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        // Manejar métodos específicos de objetos Quetzal
        if let Valor::Objeto { clase, .. } = valor {
            // Buscar la clase para obtener los métodos disponibles
            let clase_def = {
                let entorno_ref = self.entorno_global.borrow();
                entorno_ref.obtener_clase(clase)
            };
            
            if let Some(clase_definida) = clase_def {
                // Buscar en métodos públicos
                for miembro in &clase_definida.miembros_publicos {
                    if let Nodo::DeclaracionFuncion { nombre: nombre_metodo, parametros, cuerpo, .. } = miembro {
                        if nombre_metodo == metodo {
                            // Crear entorno para la ejecución del método
                            let entorno_metodo = Rc::new(RefCell::new(Entorno::nuevo()));
                            entorno_metodo.borrow_mut().padre = Some(self.entorno_global.clone());
                            
                            // Verificar número de argumentos
                            if argumentos.len() != parametros.len() {
                                return Err(ErrorQuetzal::ErrorEjecucion {
                                    linea,
                                    mensaje: format!("Método '{}' espera {} argumentos, pero se proporcionaron {}", 
                                        metodo, parametros.len(), argumentos.len()),
                                });
                            }
                            
                            // Definir parámetros
                            for (parametro, valor_arg) in parametros.iter().zip(argumentos.iter()) {
                                let variable = Variable::nueva(
                                    parametro.nombre.clone(),
                                    valor_arg.clone(),
                                    if parametro.es_variable { TipoVariable::Variable } else { TipoVariable::Inmutable },
                                    parametro.tipo_dato.clone(),
                                );
                                entorno_metodo.borrow_mut().definir_variable(parametro.nombre.clone(), variable)?;
                            }
                            
                            // Definir 'ambiente' que referencia al objeto
                            let valor_ambiente = valor.clone(); // Usar directamente el objeto con todas sus propiedades
                            let variable_ambiente = Variable::nueva(
                                "ambiente".to_string(),
                                valor_ambiente,
                                TipoVariable::Variable, // 'ambiente' puede ser modificado en métodos
                                "objeto".to_string(),
                            );
                            entorno_metodo.borrow_mut().definir_variable("ambiente".to_string(), variable_ambiente)?;
                            
                            // Ejecutar el método
                            let anterior_dentro_de_funcion = self.dentro_de_funcion;
                            self.dentro_de_funcion = true;
                            let resultado = self.evaluar_con_entorno(cuerpo, entorno_metodo.clone());
                            self.dentro_de_funcion = anterior_dentro_de_funcion;
                            
                            return resultado;
                        }
                    }
                }
            }
            
            // Si no se encontró el método en la clase, continuar con los métodos estándar
        }
        
        match metodo {
            // Métodos de conversión
            "texto" => {
                let resultado = match valor {
                    Valor::Entero(n) => Valor::Texto(n.to_string()),
                    Valor::Numero(n) => Valor::Texto(n.to_string()),
                    Valor::Log(b) => Valor::Texto(b.to_string()),
                    Valor::Texto(s) => Valor::Texto(s.clone()),
                    Valor::Lista(lista) => {
                        let elementos: Vec<String> = lista.iter()
                            .map(|v| match v {
                                Valor::Texto(s) => s.clone(),
                                _ => v.to_string(),
                            })
                            .collect();
                        Valor::Texto(format!("[{}]", elementos.join(", ")))
                    },
                    _ => Valor::Texto(valor.to_string()),
                };
                Ok((resultado, ControlFlujo::Ninguno))
            },
            
            "entero" => {
                let resultado = match valor {
                    Valor::Entero(n) => Valor::Entero(*n),
                    Valor::Numero(n) => Valor::Entero(*n as i64),
                    Valor::Texto(s) => {
                        match s.trim().parse::<i64>() {
                            Ok(n) => Valor::Entero(n),
                            Err(_) => return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: format!("No se puede convertir '{}' a entero", s),
                            }),
                        }
                    },
                    Valor::Log(true) => Valor::Entero(1),
                    Valor::Log(false) => Valor::Entero(0),
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
                    Valor::Texto(s) => {
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
                    Valor::Log(true) => Valor::Numero(1.0),
                    Valor::Log(false) => Valor::Numero(0.0),
                    _ => return Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("No se puede convertir {} a número", valor.tipo_como_cadena()),
                    }),
                };
                Ok((resultado, ControlFlujo::Ninguno))
            },
            
            "log" => {
                let resultado = match valor {
                    Valor::Log(b) => Valor::Log(*b),
                    Valor::Entero(n) => Valor::Log(*n != 0),
                    Valor::Numero(n) => Valor::Log(*n != 0.0),
                    Valor::Texto(s) => Valor::Log(!s.is_empty()),
                    _ => return Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("No se puede convertir {} a bool", valor.tipo_como_cadena()),
                    }),
                };
                Ok((resultado, ControlFlujo::Ninguno))
            },
            
            "mayuscula" => {
                if let Valor::Texto(s) = valor {
                    Ok((Valor::Texto(s.to_uppercase()), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'mayuscula' solo es válido para cadenas"),
                    })
                }
            },
            
            "minuscula" => {
                if let Valor::Texto(s) = valor {
                    Ok((Valor::Texto(s.to_lowercase()), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'minuscula' solo es válido para cadenas"),
                    })
                }
            },
            
            "a_minusculas" => {
                if let Valor::Texto(s) = valor {
                    Ok((Valor::Texto(s.to_lowercase()), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'a_minusculas' solo es válido para cadenas"),
                    })
                }
            },
            
            "capitalizar" => {
                if let Valor::Texto(s) = valor {
                    let mut chars = s.chars();
                    let resultado = match chars.next() {
                        None => String::new(),
                        Some(primer_char) => primer_char.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
                    };
                    Ok((Valor::Texto(resultado), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'capitalizar' solo es válido para cadenas"),
                    })
                }
            },
            
            "titulo" => {
                if let Valor::Texto(s) = valor {
                    let resultado = s.split_whitespace()
                        .map(|palabra| {
                            let mut chars: Vec<char> = palabra.chars().collect();
                            if !chars.is_empty() {
                                chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
                                for i in 1..chars.len() {
                                    chars[i] = chars[i].to_lowercase().next().unwrap_or(chars[i]);
                                }
                            }
                            chars.into_iter().collect::<String>()
                        })
                        .collect::<Vec<String>>()
                        .join(" ");
                    Ok((Valor::Texto(resultado), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'titulo' solo es válido para cadenas"),
                    })
                }
            },
            
            "recortar" => {
                if let Valor::Texto(s) = valor {
                    Ok((Valor::Texto(s.trim().to_string()), ControlFlujo::Ninguno))
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
                    
                    if let Valor::Texto(separador) = &argumentos[0] {
                        let elementos: Vec<String> = lista.iter()
                            .map(|v| match v {
                                Valor::Texto(s) => s.clone(),
                                _ => v.to_string(),
                            })
                            .collect();
                        Ok((Valor::Texto(elementos.join(separador)), ControlFlujo::Ninguno))
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
                            Valor::Texto(s) => s.clone(),
                            _ => v.to_string(),
                        })
                        .collect();
                    Ok((Valor::Texto(elementos.join("\n")), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'unir_lineas' solo es válido para listas"),
                    })
                }
            },
            
            "buscar" => {
                match valor {
                    Valor::Texto(s) => {
                        if argumentos.is_empty() {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: "El método 'buscar' requiere un patrón como argumento".to_string(),
                            });
                        }
                        
                        if let Valor::Texto(patron) = &argumentos[0] {
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
                    },
                    Valor::Lista(lista) => {
                        if argumentos.is_empty() {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: "El método 'buscar' requiere un elemento como argumento".to_string(),
                            });
                        }
                        
                        let elemento = &argumentos[0];
                        for (i, valor_lista) in lista.iter().enumerate() {
                            if self.valores_son_iguales_simple(valor_lista, elemento) {
                                return Ok((Valor::Entero(i as i64), ControlFlujo::Ninguno));
                            }
                        }
                        Ok((Valor::Entero(-1), ControlFlujo::Ninguno))
                    },
                    _ => {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: format!("El método 'buscar' solo es válido para cadenas y listas"),
                        })
                    }
                }
            },
            
            "contiene" => {
                match valor {
                    Valor::Texto(s) => {
                        if argumentos.is_empty() {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: "El método 'contiene' requiere un patrón como argumento".to_string(),
                            });
                        }
                        
                        if let Valor::Texto(patron) = &argumentos[0] {
                            Ok((Valor::Log(s.contains(patron)), ControlFlujo::Ninguno))
                        } else {
                            Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: "El patrón para 'contiene' debe ser una cadena".to_string(),
                            })
                        }
                    },
                    Valor::Lista(lista) => {
                        if argumentos.is_empty() {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: "El método 'contiene' requiere un elemento como argumento".to_string(),
                            });
                        }
                        
                        let elemento = &argumentos[0];
                        for valor_lista in lista.iter() {
                            if self.valores_son_iguales_simple(valor_lista, elemento) {
                                return Ok((Valor::Log(true), ControlFlujo::Ninguno));
                            }
                        }
                        Ok((Valor::Log(false), ControlFlujo::Ninguno))
                    },
                    _ => {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: format!("El método 'contiene' solo es válido para cadenas y listas"),
                        })
                    }
                }
            },
            
            "empieza_con" => {
                if let Valor::Texto(s) = valor {
                    if argumentos.is_empty() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'empieza_con' requiere un prefijo como argumento".to_string(),
                        });
                    }
                    
                    if let Valor::Texto(prefijo) = &argumentos[0] {
                        Ok((Valor::Log(s.starts_with(prefijo)), ControlFlujo::Ninguno))
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
                if let Valor::Texto(s) = valor {
                    if argumentos.is_empty() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'termina_con' requiere un sufijo como argumento".to_string(),
                        });
                    }
                    
                    if let Valor::Texto(sufijo) = &argumentos[0] {
                        Ok((Valor::Log(s.ends_with(sufijo)), ControlFlujo::Ninguno))
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
                if let Valor::Texto(s) = valor {
                    if argumentos.is_empty() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'contar_ocurrencias' requiere un patrón como argumento".to_string(),
                        });
                    }
                    
                    if let Valor::Texto(patron) = &argumentos[0] {
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
                if let Valor::Texto(s) = valor {
                    Ok((Valor::Texto(s.to_uppercase()), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'a_mayusculas' solo es válido para cadenas"),
                    })
                }
            },
            
            "invertir" => {
                match valor {
                    Valor::Texto(s) => {
                        let invertida = s.chars().rev().collect::<String>();
                        Ok((Valor::Texto(invertida), ControlFlujo::Ninguno))
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
                if let Valor::Texto(s) = valor {
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
                    
                    Ok((Valor::Texto(s.repeat(veces as usize)), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'repetir' solo es válido para cadenas"),
                    })
                }
            },
            
            "reemplazar" => {
                if let Valor::Texto(s) = valor {
                    if argumentos.len() < 2 {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'reemplazar' requiere dos argumentos: buscar y reemplazar".to_string(),
                        });
                    }
                    
                    if let (Valor::Texto(buscar), Valor::Texto(reemplazar)) = (&argumentos[0], &argumentos[1]) {
                        Ok((Valor::Texto(s.replace(buscar, reemplazar)), ControlFlujo::Ninguno))
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
                if let Valor::Texto(s) = valor {
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
                        return Ok((Valor::Texto(String::new()), ControlFlujo::Ninguno));
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
                        return Ok((Valor::Texto(String::new()), ControlFlujo::Ninguno));
                    }
                    
                    let subcadena: String = chars[inicio..fin].iter().collect();
                    Ok((Valor::Texto(subcadena), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'subcadena' solo es válido para cadenas"),
                    })
                }
            },
            
            "dividir" => {
                if let Valor::Texto(s) = valor {
                    if argumentos.is_empty() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'dividir' requiere un delimitador como argumento".to_string(),
                        });
                    }
                    
                    if let Valor::Texto(delimitador) = &argumentos[0] {
                        if delimitador.is_empty() {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: "El delimitador no puede estar vacío".to_string(),
                            });
                        }
                        let partes: Vec<Valor> = s.split(delimitador)
                            .map(|parte| Valor::Texto(parte.to_string()))
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
                if let Valor::Texto(s) = valor {
                    let lineas: Vec<Valor> = s.lines()
                        .map(|linea| Valor::Texto(linea.to_string()))
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
                if let Valor::Texto(s) = valor {
                    if argumentos.is_empty() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'comparar' requiere otra cadena como argumento".to_string(),
                        });
                    }
                    
                    if let Valor::Texto(otra) = &argumentos[0] {
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
                if let Valor::Texto(s) = valor {
                    if argumentos.is_empty() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'igual_sin_caso' requiere otra cadena como argumento".to_string(),
                        });
                    }
                    
                    if let Valor::Texto(otra) = &argumentos[0] {
                        Ok((Valor::Log(s.to_lowercase() == otra.to_lowercase()), ControlFlujo::Ninguno))
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
                if let Valor::Texto(s) = valor {
                    use base64::{Engine as _, engine::general_purpose};
                    let encoded = general_purpose::STANDARD.encode(s.as_bytes());
                    Ok((Valor::Texto(encoded), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'codificar_base64' solo es válido para cadenas"),
                    })
                }
            },
            
            "decodificar_base64" => {
                if let Valor::Texto(s) = valor {
                    use base64::{Engine as _, engine::general_purpose};
                    match general_purpose::STANDARD.decode(s) {
                        Ok(bytes) => match String::from_utf8(bytes) {
                            Ok(decoded) => Ok((Valor::Texto(decoded), ControlFlujo::Ninguno)),
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
                if let Valor::Texto(s) = valor {
                    let encoded = urlencoding::encode(s);
                    Ok((Valor::Texto(encoded.to_string()), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'codificar_uri' solo es válido para cadenas"),
                    })
                }
            },
            
            "decodificar_uri" => {
                if let Valor::Texto(s) = valor {
                    match urlencoding::decode(s) {
                        Ok(decoded) => Ok((Valor::Texto(decoded.to_string()), ControlFlujo::Ninguno)),
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
                            (Valor::Entero(a), Valor::Entero(b)) => a.cmp(b),
                            (Valor::Numero(a), Valor::Numero(b)) => a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
                            (Valor::Texto(a), Valor::Texto(b)) => a.cmp(b),
                            (Valor::Log(a), Valor::Log(b)) => a.cmp(b),
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
            
            "buscar_ultimo" => {
                if let Valor::Lista(lista) = valor {
                    if argumentos.len() != 1 {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'buscar_ultimo' requiere exactamente un argumento".to_string(),
                        });
                    }
                    let elemento_buscar = &argumentos[0];
                    for (indice, elemento) in lista.iter().enumerate().rev() {
                        if self.valores_son_iguales_simple(elemento, elemento_buscar) {
                            return Ok((Valor::Entero(indice as i64), ControlFlujo::Ninguno));
                        }
                    }
                    Ok((Valor::Entero(-1), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'buscar_ultimo' solo es válido para listas"),
                    })
                }
            },
            
            "contar" => {
                if let Valor::Lista(lista) = valor {
                    if argumentos.len() != 1 {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'contar' requiere exactamente un argumento".to_string(),
                        });
                    }
                    let elemento_buscar = &argumentos[0];
                    let mut contador = 0i64;
                    for elemento in lista {
                        if self.valores_son_iguales_simple(elemento, elemento_buscar) {
                            contador += 1;
                        }
                    }
                    Ok((Valor::Entero(contador), ControlFlujo::Ninguno))
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'contar' solo es válido para listas"),
                    })
                }
            },
            
            "longitud" => {
                match valor {
                    Valor::Lista(lista) => Ok((Valor::Entero(lista.len() as i64), ControlFlujo::Ninguno)),
                    Valor::Texto(cadena) => Ok((Valor::Entero(cadena.chars().count() as i64), ControlFlujo::Ninguno)),
                    _ => Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'longitud' solo es válido para cadenas y listas"),
                    })
                }
            },
            
            "esta_vacia" => {
                match valor {
                    Valor::Lista(lista) => Ok((Valor::Log(lista.is_empty()), ControlFlujo::Ninguno)),
                    Valor::Texto(cadena) => Ok((Valor::Log(cadena.is_empty()), ControlFlujo::Ninguno)),
                    _ => Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'esta_vacia' solo es válido para cadenas y listas"),
                    })
                }
            },
            
            "absoluto" => {
                if !argumentos.is_empty() {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: "El método 'absoluto' no acepta argumentos".to_string(),
                    });
                }
                match valor {
                    Valor::Entero(n) => Ok((Valor::Entero(n.abs()), ControlFlujo::Ninguno)),
                    Valor::Numero(n) => Ok((Valor::Numero(n.abs()), ControlFlujo::Ninguno)),
                    _ => Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método 'absoluto' solo es válido para números enteros y decimales"),
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
    
    /// Evalúa un método en un valor específico con capacidad de actualizar la variable original del objeto
    fn evaluar_metodo_en_valor_con_actualizacion(&mut self, valor: &Valor, metodo: &str, argumentos: &[Valor], linea: usize, entorno: Rc<RefCell<Entorno>>, nombre_variable: Option<String>) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        // Manejar métodos específicos de objetos Quetzal
        if let Valor::Objeto { clase, .. } = valor {
            // Buscar la clase para obtener los métodos disponibles
            let clase_def = {
                let entorno_ref = self.entorno_global.borrow();
                entorno_ref.obtener_clase(clase)
            };

            if let Some(clase_definida) = clase_def {
                // Buscar en métodos públicos
                for miembro in &clase_definida.miembros_publicos {
                    if let Nodo::DeclaracionFuncion { nombre: nombre_metodo, parametros, cuerpo, .. } = miembro {
                        if nombre_metodo == metodo {
                            // Crear entorno para la ejecución del método
                            let entorno_metodo = Rc::new(RefCell::new(Entorno::nuevo()));
                            entorno_metodo.borrow_mut().padre = Some(self.entorno_global.clone());
                            
                            // Verificar número de argumentos
                            if argumentos.len() != parametros.len() {
                                return Err(ErrorQuetzal::ErrorEjecucion {
                                    linea,
                                    mensaje: format!("Método '{}' espera {} argumentos, pero se proporcionaron {}", 
                                        metodo, parametros.len(), argumentos.len()),
                                });
                            }
                            
                            // Definir parámetros
                            for (parametro, valor_arg) in parametros.iter().zip(argumentos.iter()) {
                                let variable = Variable::nueva(
                                    parametro.nombre.clone(),
                                    valor_arg.clone(),
                                    if parametro.es_variable { TipoVariable::Variable } else { TipoVariable::Inmutable },
                                    parametro.tipo_dato.clone(),
                                );
                                entorno_metodo.borrow_mut().definir_variable(parametro.nombre.clone(), variable)?;
                            }
                            
                            // Definir 'ambiente' que referencia al objeto
                            let variable_ambiente = Variable::nueva(
                                "ambiente".to_string(),
                                valor.clone(),
                                TipoVariable::Variable, // 'ambiente' puede ser modificado en métodos
                                "objeto".to_string(),
                            );
                            entorno_metodo.borrow_mut().definir_variable("ambiente".to_string(), variable_ambiente)?;
                            
                            // Ejecutar el método
                            let anterior_dentro_de_funcion = self.dentro_de_funcion;
                            let anterior_dentro_de_metodo_clase = self.dentro_de_metodo_clase;
                            self.dentro_de_funcion = true;
                            self.dentro_de_metodo_clase = true;
                            let resultado = self.evaluar_con_entorno(cuerpo, entorno_metodo.clone());
                            self.dentro_de_funcion = anterior_dentro_de_funcion;
                            self.dentro_de_metodo_clase = anterior_dentro_de_metodo_clase;
                            
                            // Después de ejecutar el método, verificar si necesitamos actualizar la variable original
                            // Solo actualizar para métodos que modifican propiedades del objeto (evitar recursión)
                            if let Some(nombre_var) = nombre_variable {
                                if let Some(variable_ambiente_actualizada) = entorno_metodo.borrow().obtener_variable("ambiente") {
                                    let valor_actual = &variable_ambiente_actualizada.valor;
                                    
                                    // Solo actualizar si los punteros son diferentes (indica modificación)
                                    if !std::ptr::eq(valor, valor_actual) {
                                        // Solo hacer actualización simple sin comparación profunda para evitar stack overflow
                                        let entorno_ref = entorno.borrow();
                                        if let Some(variable_original) = entorno_ref.obtener_variable(&nombre_var) {
                                            let nueva_variable = Variable::nueva(
                                                nombre_var.clone(),
                                                variable_ambiente_actualizada.valor.clone(),
                                                variable_original.tipo_variable,
                                                variable_original.tipo_dato.clone(),
                                            );
                                            drop(entorno_ref);
                                            entorno.borrow_mut().variables.insert(nombre_var.clone(), nueva_variable);
                                        }
                                    }
                                }
                            }
                            
                            return resultado;
                        }
                    }
                }
            }
        }
        
        // Si no es un método de objeto, usar el método estándar
        self.evaluar_metodo_en_valor(valor, metodo, argumentos, linea)
    }
    
    /// Evalúa métodos que modifican la variable original (métodos mutantes)
    fn evaluar_metodo_mutante(&mut self, nombre_var: &str, metodo: &str, argumentos: &[Valor], linea: usize, entorno: Rc<RefCell<Entorno>>) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        // Primero verificar si la variable existe en la cadena de entornos
        let variable_original = {
            let entorno_ref = entorno.borrow();
            entorno_ref.obtener_variable(nombre_var)
        };
        
        if let Some(mut variable) = variable_original {
            if !variable.es_mutable() {
                return Err(ErrorQuetzal::ErrorEjecucion {
                    linea,
                    mensaje: format!("No se puede modificar la variable inmutable '{}'", nombre_var),
                });
            }
            
            let resultado = match (&mut variable.valor, metodo) {
                (Valor::Lista(ref mut lista), "agregar") => {
                    if argumentos.is_empty() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'agregar' requiere un argumento".to_string(),
                        });
                    }
                    
                    let elemento = &argumentos[0];
                    
                    // Validación de tipos para listas tipadas
                    if variable.tipo_dato.starts_with("lista<") && variable.tipo_dato.ends_with(">") {
                        let tipo_elemento = &variable.tipo_dato[6..variable.tipo_dato.len()-1]; // extraer tipo entre < >
                        if !self.validar_tipo_compatible(elemento, tipo_elemento) {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: format!(
                                    "Error: No se puede agregar un valor de tipo '{}' a una lista de tipo '{}'",
                                    self.obtener_nombre_tipo(elemento),
                                    variable.tipo_dato
                                ),
                            });
                        }
                    }
                    
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
                
                (Valor::Lista(ref mut lista), "insertar") => {
                    if argumentos.len() != 2 {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'insertar' requiere exactamente dos argumentos (índice, elemento)".to_string(),
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
                                mensaje: "El primer argumento de 'insertar' debe ser un número entero".to_string(),
                            });
                        }
                    };
                    
                    if indice > lista.len() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: format!("Índice fuera de rango: {} (tamaño: {})", indice, lista.len()),
                        });
                    }
                    
                    let elemento = &argumentos[1];
                    
                    // Validación de tipos para listas tipadas
                    if variable.tipo_dato.starts_with("lista<") && variable.tipo_dato.ends_with(">") {
                        let tipo_elemento = &variable.tipo_dato[6..variable.tipo_dato.len()-1];
                        if !self.validar_tipo_compatible(elemento, tipo_elemento) {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: format!(
                                    "Error: No se puede insertar un valor de tipo '{}' en una lista de tipo '{}'",
                                    self.obtener_nombre_tipo(elemento),
                                    variable.tipo_dato
                                ),
                            });
                        }
                    }
                    
                    if let Valor::Vacio = elemento {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "No se puede insertar un valor vacío en la lista".to_string(),
                        });
                    }
                    
                    lista.insert(indice, elemento.clone());
                    Ok((Valor::Vacio, ControlFlujo::Ninguno))
                },
                
                (Valor::Lista(ref mut lista), "sacar") => {
                    if argumentos.len() != 1 {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'sacar' requiere exactamente un argumento (elemento a remover)".to_string(),
                        });
                    }
                    
                    let elemento_buscar = &argumentos[0];
                    
                    // Buscar el elemento
                    for (indice, elemento) in lista.iter().enumerate() {
                        if self.valores_son_iguales_simple(elemento, elemento_buscar) {
                            let elemento_removido = lista.remove(indice);
                            return Ok((elemento_removido, ControlFlujo::Ninguno));
                        }
                    }
                    
                    // Si no se encuentra el elemento
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: "El elemento no se encontró en la lista".to_string(),
                    })
                },
                
                (Valor::Lista(ref mut lista), "sacar_ultimo") => {
                    if !argumentos.is_empty() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "El método 'sacar_ultimo' no acepta argumentos".to_string(),
                        });
                    }
                    
                    if lista.is_empty() {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "No se puede sacar el último elemento de una lista vacía".to_string(),
                        });
                    }
                    
                    let elemento_removido = lista.pop().unwrap();
                    Ok((elemento_removido, ControlFlujo::Ninguno))
                },
                
                _ => {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("El método '{}' no está disponible para el tipo {}", metodo, variable.valor.tipo_como_cadena()),
                    })
                }
            };
            
            // Actualizar la variable en el entorno correcto después de la modificación
            if resultado.is_ok() {
                let mut entorno_ref = entorno.borrow_mut();
                if !entorno_ref.actualizar_variable(nombre_var, variable) {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: format!("Error interno: no se pudo actualizar la variable '{}'", nombre_var),
                    });
                }
            }
            
            resultado
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

    /// Función auxiliar universal para asignar valores a índices (listas con enteros, JSON con cadenas)
    fn asignar_indice_recursivo_universal(
        &mut self,
        objeto: &Nodo,
        indice: Valor,
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
                            mensaje: format!("No se puede modificar el objeto inmutable '{}'", nombre_var),
                        });
                    }
                    
                    match (&variable.valor, &indice) {
                        // Lista con índice entero
                        (Valor::Lista(lista), Valor::Entero(i)) => {
                            if *i < 0 {
                                return Err(ErrorQuetzal::ErrorEjecucion {
                                    linea,
                                    mensaje: format!("Índice negativo: {}", i),
                                });
                            }
                            let indice_usize = *i as usize;
                            if indice_usize >= lista.len() {
                                return Err(ErrorQuetzal::ErrorEjecucion {
                                    linea,
                                    mensaje: format!("Índice fuera de rango: {} (tamaño: {})", indice_usize, lista.len()),
                                });
                            }
                            
                            if let Valor::Lista(ref mut lista_mut) = variable.valor {
                                lista_mut[indice_usize] = nuevo_valor;
                            }
                            Ok(())
                        },
                        // JSON con índice de cadena
                        (Valor::Json(mapa), Valor::Texto(clave)) => {
                            if let Valor::Json(ref mut mapa_mut) = variable.valor {
                                mapa_mut.insert(clave.clone(), nuevo_valor);
                            }
                            Ok(())
                        },
                        _ => {
                            match indice {
                                Valor::Entero(_) => Err(ErrorQuetzal::ErrorEjecucion {
                                    linea,
                                    mensaje: format!("'{}' no es una lista", nombre_var),
                                }),
                                Valor::Texto(_) => Err(ErrorQuetzal::ErrorEjecucion {
                                    linea,
                                    mensaje: format!("'{}' no es un objeto JSON", nombre_var),
                                }),
                                _ => Err(ErrorQuetzal::ErrorEjecucion {
                                    linea,
                                    mensaje: "El índice debe ser un entero para listas o texto para objetos JSON".to_string(),
                                }),
                            }
                        }
                    }
                } else {
                    Err(ErrorQuetzal::VariableNoDefinida {
                        linea,
                        nombre: nombre_var.clone(),
                    })
                }
            },
            // Caso recursivo: acceso a miembro (como ambiente.configuraciones[clave])
            Nodo::AccesoMiembro { objeto, miembro, linea: _ } => {
                // Verificar que el objeto sea un identificador (variable)
                if let Nodo::Identificador(nombre_objeto) = objeto.as_ref() {
                    let mut entorno_ref = entorno.borrow_mut();
                    if let Some(variable) = entorno_ref.variables.get_mut(nombre_objeto) {
                        match &variable.valor {
                            Valor::Objeto { propiedades, .. } => {
                                if let Some(propiedad_valor) = propiedades.get(miembro) {
                                    // La propiedad debe ser mutable para modificarla
                                    match (propiedad_valor, &indice) {
                                        (Valor::Json(mapa), Valor::Texto(clave)) => {
                                            // Modificar directamente la propiedad del objeto
                                            if let Valor::Objeto { ref mut propiedades, .. } = variable.valor {
                                                if let Some(Valor::Json(ref mut mapa_mut)) = propiedades.get_mut(miembro) {
                                                    mapa_mut.insert(clave.clone(), nuevo_valor);
                                                    return Ok(());
                                                }
                                            }
                                        },
                                        _ => {}
                                    }
                                }
                            },
                            _ => {}
                        }
                    }
                }
                Err(ErrorQuetzal::ErrorEjecucion {
                    linea,
                    mensaje: "No se puede asignar a índice de acceso a miembro".to_string(),
                })
            },
            
            // Caso recursivo: acceso a índice anidado (como matriz[0][1][0])
            Nodo::AccesoIndice { objeto: objeto_padre, indice: indice_padre, linea: _ } => {
                // Evaluar el índice padre
                let (valor_indice_padre, _) = self.evaluar_con_entorno(indice_padre, entorno.clone())?;
                
                match valor_indice_padre {
                    Valor::Entero(indice_padre_int) => {
                        if indice_padre_int < 0 {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: format!("Índice negativo: {}", indice_padre_int),
                            });
                        }
                        
                        // Ahora necesitamos acceder al elemento padre y luego asignar al índice hijo
                        match objeto_padre.as_ref() {
                            Nodo::Identificador(nombre_var) => {
                                let mut entorno_ref = entorno.borrow_mut();
                                if let Some(variable) = entorno_ref.variables.get_mut(nombre_var) {
                                    if !variable.es_mutable() {
                                        return Err(ErrorQuetzal::ErrorEjecucion {
                                            linea,
                                            mensaje: format!("No se puede modificar el objeto inmutable '{}'", nombre_var),
                                        });
                                    }
                                    
                                    if let Valor::Lista(ref mut lista) = variable.valor {
                                        let indice_padre_usize = indice_padre_int as usize;
                                        if indice_padre_usize >= lista.len() {
                                            return Err(ErrorQuetzal::ErrorEjecucion {
                                                linea,
                                                mensaje: format!("Índice fuera de rango: {} (tamaño: {})", indice_padre_usize, lista.len()),
                                            });
                                        }
                                        
                                        // Ahora asignar al elemento hijo
                                        match (&mut lista[indice_padre_usize], &indice) {
                                            (Valor::Lista(ref mut lista_hijo), Valor::Entero(indice_hijo)) => {
                                                if *indice_hijo < 0 {
                                                    return Err(ErrorQuetzal::ErrorEjecucion {
                                                        linea,
                                                        mensaje: format!("Índice negativo: {}", indice_hijo),
                                                    });
                                                }
                                                let indice_hijo_usize = *indice_hijo as usize;
                                                if indice_hijo_usize >= lista_hijo.len() {
                                                    return Err(ErrorQuetzal::ErrorEjecucion {
                                                        linea,
                                                        mensaje: format!("Índice fuera de rango: {} (tamaño: {})", indice_hijo_usize, lista_hijo.len()),
                                                    });
                                                }
                                                lista_hijo[indice_hijo_usize] = nuevo_valor;
                                                Ok(())
                                            },
                                            (Valor::Json(ref mut mapa), Valor::Texto(clave)) => {
                                                mapa.insert(clave.clone(), nuevo_valor);
                                                Ok(())
                                            },
                                            _ => Err(ErrorQuetzal::ErrorEjecucion {
                                                linea,
                                                mensaje: "Tipo de índice incompatible para asignación anidada".to_string(),
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
                            // Caso más profundo: objeto_padre también es un AccesoIndice
                            Nodo::AccesoIndice { .. } => {
                                // Para matrices de 3D o más, necesitamos recursión más profunda
                                // Por ahora, implementar casos específicos
                                self.asignar_matriz_multidimensional(objeto_padre, indice_padre_int as usize, indice, nuevo_valor, entorno, linea)
                            },
                            _ => Err(ErrorQuetzal::ErrorEjecucion {
                                linea,
                                mensaje: "Estructura de acceso no soportada para asignación".to_string(),
                            })
                        }
                    },
                    _ => Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: "El índice debe ser un entero".to_string(),
                    })
                }
            },
            
            _ => Err(ErrorQuetzal::ErrorEjecucion {
                linea,
                mensaje: "Solo se puede asignar a índices de variables".to_string(),
            })
        }
    }

    /// Función auxiliar para manejar asignaciones en matrices multidimensionales
    fn asignar_matriz_multidimensional(
        &mut self,
        objeto_padre: &Nodo,
        indice_padre: usize,
        indice_hijo: Valor,
        nuevo_valor: Valor,
        entorno: Rc<RefCell<Entorno>>,
        linea: usize,
    ) -> ResultadoQuetzal<()> {
        match objeto_padre {
            Nodo::AccesoIndice { objeto: objeto_abuelo, indice: indice_abuelo, linea: _ } => {
                // Evaluar el índice del abuelo
                let (valor_indice_abuelo, _) = self.evaluar_con_entorno(indice_abuelo, entorno.clone())?;
                
                if let Valor::Entero(indice_abuelo_int) = valor_indice_abuelo {
                    if indice_abuelo_int < 0 {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: format!("Índice negativo: {}", indice_abuelo_int),
                        });
                    }
                    
                    match objeto_abuelo.as_ref() {
                        Nodo::Identificador(nombre_var) => {
                            let mut entorno_ref = entorno.borrow_mut();
                            if let Some(variable) = entorno_ref.variables.get_mut(nombre_var) {
                                if !variable.es_mutable() {
                                    return Err(ErrorQuetzal::ErrorEjecucion {
                                        linea,
                                        mensaje: format!("No se puede modificar el objeto inmutable '{}'", nombre_var),
                                    });
                                }
                                
                                if let Valor::Lista(ref mut lista_abuelo) = variable.valor {
                                    let indice_abuelo_usize = indice_abuelo_int as usize;
                                    if indice_abuelo_usize >= lista_abuelo.len() {
                                        return Err(ErrorQuetzal::ErrorEjecucion {
                                            linea,
                                            mensaje: format!("Índice fuera de rango: {} (tamaño: {})", indice_abuelo_usize, lista_abuelo.len()),
                                        });
                                    }
                                    
                                    if let Valor::Lista(ref mut lista_padre) = lista_abuelo[indice_abuelo_usize] {
                                        if indice_padre >= lista_padre.len() {
                                            return Err(ErrorQuetzal::ErrorEjecucion {
                                                linea,
                                                mensaje: format!("Índice fuera de rango: {} (tamaño: {})", indice_padre, lista_padre.len()),
                                            });
                                        }
                                        
                                        match (&mut lista_padre[indice_padre], &indice_hijo) {
                                            (Valor::Lista(ref mut lista_hijo), Valor::Entero(indice_hijo_int)) => {
                                                if *indice_hijo_int < 0 {
                                                    return Err(ErrorQuetzal::ErrorEjecucion {
                                                        linea,
                                                        mensaje: format!("Índice negativo: {}", indice_hijo_int),
                                                    });
                                                }
                                                let indice_hijo_usize = *indice_hijo_int as usize;
                                                if indice_hijo_usize >= lista_hijo.len() {
                                                    return Err(ErrorQuetzal::ErrorEjecucion {
                                                        linea,
                                                        mensaje: format!("Índice fuera de rango: {} (tamaño: {})", indice_hijo_usize, lista_hijo.len()),
                                                    });
                                                }
                                                lista_hijo[indice_hijo_usize] = nuevo_valor;
                                                Ok(())
                                            },
                                            _ => Err(ErrorQuetzal::ErrorEjecucion {
                                                linea,
                                                mensaje: "Tipo de índice incompatible para matriz 3D".to_string(),
                                            })
                                        }
                                    } else {
                                        Err(ErrorQuetzal::ErrorEjecucion {
                                            linea,
                                            mensaje: "El elemento padre no es una lista".to_string(),
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
                        // Para matrices 4D o superiores, agregar más recursión aquí
                        Nodo::AccesoIndice { .. } => {
                            self.asignar_matriz_4d_o_superior(objeto_abuelo, indice_abuelo_int as usize, indice_padre, indice_hijo, nuevo_valor, entorno, linea)
                        },
                        _ => Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "Estructura no soportada para matriz multidimensional".to_string(),
                        })
                    }
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: "El índice debe ser un entero".to_string(),
                    })
                }
            },
            _ => Err(ErrorQuetzal::ErrorEjecucion {
                linea,
                mensaje: "Estructura no soportada para matriz multidimensional".to_string(),
            })
        }
    }

    /// Función auxiliar para manejar asignaciones en matrices 4D o superiores
    fn asignar_matriz_4d_o_superior(
        &mut self,
        objeto_bisabuelo: &Nodo,
        indice_bisabuelo: usize,
        indice_abuelo: usize,
        indice_padre: Valor,
        nuevo_valor: Valor,
        entorno: Rc<RefCell<Entorno>>,
        linea: usize,
    ) -> ResultadoQuetzal<()> {
        match objeto_bisabuelo {
            Nodo::AccesoIndice { objeto: objeto_tatarabuelo, indice: indice_tatarabuelo, linea: _ } => {
                // Evaluar el índice del tatarabuelo (matriz 4D)
                let (valor_indice_tatarabuelo, _) = self.evaluar_con_entorno(indice_tatarabuelo, entorno.clone())?;
                
                if let Valor::Entero(indice_tatarabuelo_int) = valor_indice_tatarabuelo {
                    if indice_tatarabuelo_int < 0 {
                        return Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: format!("Índice negativo: {}", indice_tatarabuelo_int),
                        });
                    }
                    
                    if let Nodo::Identificador(nombre_var) = objeto_tatarabuelo.as_ref() {
                        let mut entorno_ref = entorno.borrow_mut();
                        if let Some(variable) = entorno_ref.variables.get_mut(nombre_var) {
                            if !variable.es_mutable() {
                                return Err(ErrorQuetzal::ErrorEjecucion {
                                    linea,
                                    mensaje: format!("No se puede modificar el objeto inmutable '{}'", nombre_var),
                                });
                            }
                            
                            if let Valor::Lista(ref mut lista_tatarabuelo) = variable.valor {
                                let indice_tatarabuelo_usize = indice_tatarabuelo_int as usize;
                                if indice_tatarabuelo_usize >= lista_tatarabuelo.len() {
                                    return Err(ErrorQuetzal::ErrorEjecucion {
                                        linea,
                                        mensaje: format!("Índice fuera de rango: {} (tamaño: {})", indice_tatarabuelo_usize, lista_tatarabuelo.len()),
                                    });
                                }
                                
                                if let Valor::Lista(ref mut lista_bisabuelo) = lista_tatarabuelo[indice_tatarabuelo_usize] {
                                    if indice_bisabuelo >= lista_bisabuelo.len() {
                                        return Err(ErrorQuetzal::ErrorEjecucion {
                                            linea,
                                            mensaje: format!("Índice fuera de rango: {} (tamaño: {})", indice_bisabuelo, lista_bisabuelo.len()),
                                        });
                                    }
                                    
                                    if let Valor::Lista(ref mut lista_abuelo) = lista_bisabuelo[indice_bisabuelo] {
                                        if indice_abuelo >= lista_abuelo.len() {
                                            return Err(ErrorQuetzal::ErrorEjecucion {
                                                linea,
                                                mensaje: format!("Índice fuera de rango: {} (tamaño: {})", indice_abuelo, lista_abuelo.len()),
                                            });
                                        }
                                        
                                        match (&mut lista_abuelo[indice_abuelo], &indice_padre) {
                                            (Valor::Lista(ref mut lista_padre), Valor::Entero(indice_padre_int)) => {
                                                if *indice_padre_int < 0 {
                                                    return Err(ErrorQuetzal::ErrorEjecucion {
                                                        linea,
                                                        mensaje: format!("Índice negativo: {}", indice_padre_int),
                                                    });
                                                }
                                                let indice_padre_usize = *indice_padre_int as usize;
                                                if indice_padre_usize >= lista_padre.len() {
                                                    return Err(ErrorQuetzal::ErrorEjecucion {
                                                        linea,
                                                        mensaje: format!("Índice fuera de rango: {} (tamaño: {})", indice_padre_usize, lista_padre.len()),
                                                    });
                                                }
                                                lista_padre[indice_padre_usize] = nuevo_valor;
                                                Ok(())
                                            },
                                            _ => Err(ErrorQuetzal::ErrorEjecucion {
                                                linea,
                                                mensaje: "Tipo de índice incompatible para matriz 4D".to_string(),
                                            })
                                        }
                                    } else {
                                        Err(ErrorQuetzal::ErrorEjecucion {
                                            linea,
                                            mensaje: "El elemento abuelo no es una lista".to_string(),
                                        })
                                    }
                                } else {
                                    Err(ErrorQuetzal::ErrorEjecucion {
                                        linea,
                                        mensaje: "El elemento bisabuelo no es una lista".to_string(),
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
                    } else {
                        Err(ErrorQuetzal::ErrorEjecucion {
                            linea,
                            mensaje: "Solo se pueden asignar índices a variables".to_string(),
                        })
                    }
                } else {
                    Err(ErrorQuetzal::ErrorEjecucion {
                        linea,
                        mensaje: "El índice debe ser un entero".to_string(),
                    })
                }
            },
            _ => Err(ErrorQuetzal::ErrorEjecucion {
                linea,
                mensaje: "Estructura no soportada para matriz 4D o superior".to_string(),
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
    
    /// Define una variable en el entorno global
    pub fn definir_variable_global(&mut self, nombre: String, variable: Variable) -> ResultadoQuetzal<()> {
        self.entorno_global.borrow_mut().definir_variable(nombre, variable)
    }
    
    /// Define una función en el entorno global
    pub fn definir_funcion_global(&mut self, nombre: String, funcion: FuncionDefinida) -> ResultadoQuetzal<()> {
        self.entorno_global.borrow_mut().definir_funcion(nombre, funcion)
    }
    
    /// Define una clase en el entorno global
    pub fn definir_clase_global(&mut self, nombre: String, clase: ClaseDefinida) -> ResultadoQuetzal<()> {
        self.entorno_global.borrow_mut().definir_clase(nombre, clase)
    }
    
    /// Obtiene una variable del entorno actual
    pub fn obtener_variable(&self, nombre: &str) -> Option<Variable> {
        self.entorno_global.borrow().obtener_variable(nombre)
    }
    
    /// Obtiene una función del entorno actual
    pub fn obtener_funcion(&self, nombre: &str) -> Option<FuncionDefinida> {
        self.entorno_global.borrow().obtener_funcion(nombre)
    }
    
    /// Obtiene una clase del entorno actual
    pub fn obtener_clase(&self, nombre: &str) -> Option<ClaseDefinida> {
        self.entorno_global.borrow().obtener_clase(nombre)
    }
    
    /// Obtiene los nombres de todas las variables definidas en el entorno actual
    pub fn obtener_todas_las_variables(&self) -> Vec<String> {
        self.entorno_global.borrow().obtener_todas_las_variables()
    }
    
    /// Intercambia el entorno global temporalmente y devuelve el anterior
    pub fn intercambiar_entorno(&mut self, nuevo_entorno: Rc<RefCell<Entorno>>) -> Rc<RefCell<Entorno>> {
        std::mem::replace(&mut self.entorno_global, nuevo_entorno)
    }
    
    /// Maneja una declaración de importación
    fn manejar_importacion(&mut self, elementos: &[crate::analizador_sintactico::ElementoImportar], ruta: &str, linea: usize) -> ResultadoQuetzal<()> {
        // Extraer el manejador temporalmente para evitar problemas de préstamo
        if let Some(mut manejador) = self.manejador_modulos.take() {
            let resultado = manejador.procesar_importacion(elementos, ruta, self, linea);
            self.manejador_modulos = Some(manejador);
            resultado
        } else {
            Err(ErrorQuetzal::ErrorCargaModulo {
                linea,
                ruta: ruta.to_string(),
                detalle: "Sistema de módulos no inicializado. No se pueden importar módulos.".to_string(),
            })
        }
    }
    
    /// Maneja una declaración de exportación
    fn manejar_exportacion(&mut self, elementos: &[String], linea: usize) -> ResultadoQuetzal<()> {
        // Extraer el manejador temporalmente para evitar problemas de préstamo
        if let Some(mut manejador) = self.manejador_modulos.take() {
            // Obtener la ruta actual del archivo que se está evaluando
            let ruta_actual = self.ruta_archivo_actual.as_deref()
                .unwrap_or("archivo_desconocido.qz");
            let resultado = manejador.procesar_exportacion(elementos, self, ruta_actual, linea);
            self.manejador_modulos = Some(manejador);
            resultado
        } else {
            Err(ErrorQuetzal::ErrorExportacion {
                linea,
                elemento: elementos.join(", "),
                sugerencia: "Sistema de módulos no inicializado. No se pueden exportar elementos.".to_string(),
            })
        }
    }
    
    /// Evalúa una declaración de objeto
    fn evaluar_declaracion_objeto(&mut self, nombre: &str, miembros: &[Nodo], _linea: usize, entorno: Rc<RefCell<Entorno>>) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        let mut miembros_publicos = Vec::new();
        let mut miembros_privados = Vec::new();
        let mut constructor = None;
        let mut propiedades_publicas = Vec::new();
        let mut propiedades_privadas = Vec::new();
        let mut metodos_publicos = Vec::new();
        let mut metodos_privados = Vec::new();
        
        let mut es_seccion_publica = true; // Por defecto todo es público
        
        for miembro in miembros {
            match miembro {
                // Detectar marcadores de sección
                Nodo::Identificador(palabra) if palabra == "publico" || palabra == "__seccion_publica__" => {
                    es_seccion_publica = true;
                    continue;
                },
                Nodo::Identificador(palabra) if palabra == "privado" || palabra == "__seccion_privada__" => {
                    es_seccion_publica = false;
                    continue;
                },
                
                // Verificar si es un constructor (función con el mismo nombre de la clase)
                Nodo::DeclaracionFuncion { nombre: nombre_funcion, .. } if nombre_funcion == nombre => {
                    constructor = Some(miembro.clone());
                },
                
                // Recopilar nombres de propiedades
                Nodo::DeclaracionVariable { nombre: nombre_prop, .. } => {
                    if es_seccion_publica {
                        miembros_publicos.push(miembro.clone());
                        propiedades_publicas.push(nombre_prop.clone());
                    } else {
                        miembros_privados.push(miembro.clone());
                        propiedades_privadas.push(nombre_prop.clone());
                    }
                },
                
                // Recopilar nombres de métodos
                Nodo::DeclaracionFuncion { nombre: nombre_metodo, .. } => {
                    if es_seccion_publica {
                        miembros_publicos.push(miembro.clone());
                        metodos_publicos.push(nombre_metodo.clone());
                    } else {
                        miembros_privados.push(miembro.clone());
                        metodos_privados.push(nombre_metodo.clone());
                    }
                },
                
                // Cualquier otro miembro
                _ => {
                    if es_seccion_publica {
                        miembros_publicos.push(miembro.clone());
                    } else {
                        miembros_privados.push(miembro.clone());
                    }
                }
            }
        }
        
        let clase = ClaseDefinida {
            nombre: nombre.to_string(),
            miembros_publicos,
            miembros_privados,
            constructor,
            propiedades_publicas: propiedades_publicas.clone(),
            propiedades_privadas: propiedades_privadas.clone(),
            metodos_publicos,
            metodos_privados,
        };
        
        entorno.borrow_mut().definir_clase(nombre.to_string(), clase)?;
        Ok((Valor::Vacio, ControlFlujo::Ninguno))
    }
    
    /// Evalúa la creación de un objeto (nuevo NombreClase(...))
    fn evaluar_creacion_objeto(&mut self, nombre_clase: &str, argumentos: &[Nodo], linea: usize, entorno: Rc<RefCell<Entorno>>) -> ResultadoQuetzal<(Valor, ControlFlujo)> {
        // Obtener la definición de la clase
        let clase = {
            let entorno_ref = entorno.borrow();
            entorno_ref.obtener_clase(nombre_clase)
        };
        
        let clase = match clase {
            Some(c) => c,
            None => {
                return Err(ErrorQuetzal::ErrorEjecucion {
                    linea,
                    mensaje: format!("Clase '{}' no encontrada", nombre_clase),
                });
            }
        };
        
        // Crear entorno para el objeto
        let entorno_objeto = Rc::new(RefCell::new(Entorno::nuevo()));
        entorno_objeto.borrow_mut().padre = Some(entorno.clone());
        
        // Inicializar propiedades públicas con valores por defecto
        let mut propiedades_publicas = HashMap::new();
        for miembro in &clase.miembros_publicos {
            if let Nodo::DeclaracionVariable { nombre: nombre_prop, tipo_dato, valor, .. } = miembro {
                let valor_inicial = if let Some(expr_valor) = valor {
                    let (val, _) = self.evaluar_con_entorno(expr_valor, entorno_objeto.clone())?;
                    val
                } else {
                    // Valor por defecto según el tipo
                    match tipo_dato.as_str() {
                        "vacio" => Valor::Vacio,
                        "entero" => Valor::Entero(0),
                        "número" => Valor::Numero(0.0),
                        "texto" => Valor::Texto(String::new()),
                        "log" => Valor::Log(false),
                        "lista" => Valor::Lista(Vec::new()),
                        "jsn" => Valor::Json(HashMap::new()),
                        _ => Valor::Vacio,
                    }
                };
                propiedades_publicas.insert(nombre_prop.clone(), valor_inicial);
            }
        }
        
        // Inicializar propiedades privadas
        let mut propiedades_privadas = HashMap::new();
        for miembro in &clase.miembros_privados {
            if let Nodo::DeclaracionVariable { nombre: nombre_prop, tipo_dato, es_variable, valor, .. } = miembro {
                let valor_inicial = if let Some(expr_valor) = valor {
                    let (val, _) = self.evaluar_con_entorno(expr_valor, entorno_objeto.clone())?;
                    val
                } else {
                    // Valor por defecto según el tipo
                    match tipo_dato.as_str() {
                        "vacio" => Valor::Vacio,
                        "entero" => Valor::Entero(0),
                        "número" => Valor::Numero(0.0),
                        "texto" => Valor::Texto(String::new()),
                        "log" => Valor::Log(false),
                        "lista" => Valor::Lista(Vec::new()),
                        "jsn" => Valor::Json(HashMap::new()),
                        _ => Valor::Vacio,
                    }
                };
                
                propiedades_privadas.insert(nombre_prop.clone(), valor_inicial.clone());
                
                let tipo_variable = if *es_variable {
                    TipoVariable::Variable
                } else {
                    TipoVariable::Inmutable
                };
                
                let variable = Variable::nueva(
                    nombre_prop.clone(),
                    valor_inicial,
                    tipo_variable,
                    tipo_dato.clone(),
                );
                
                entorno_objeto.borrow_mut().definir_variable(nombre_prop.clone(), variable)?;
            }
        }
        
        // Crear todas las propiedades del objeto (públicas + privadas) para ambiente
        let mut todas_las_propiedades = propiedades_publicas.clone();
        todas_las_propiedades.extend(propiedades_privadas);
        
        // Crear el objeto final con todas las propiedades para que ambiente funcione
        let objeto_final = Valor::Objeto {
            clase: nombre_clase.to_string(),
            propiedades: todas_las_propiedades,
            propiedades_publicas: clase.propiedades_publicas.clone(),
            metodos_publicos: clase.metodos_publicos.clone(),
        };
        
        Ok((objeto_final, ControlFlujo::Ninguno))
    }
    
    /// Método auxiliar para comparar valores de forma simple
    fn valores_son_iguales_simple(&self, a: &Valor, b: &Valor) -> bool {
        match (a, b) {
            (Valor::Vacio, Valor::Vacio) => true,
            (Valor::Entero(a), Valor::Entero(b)) => a == b,
            (Valor::Numero(a), Valor::Numero(b)) => a == b,
            (Valor::Texto(a), Valor::Texto(b)) => a == b,
            (Valor::Log(a), Valor::Log(b)) => a == b,
            (Valor::Lista(a), Valor::Lista(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                for (elem_a, elem_b) in a.iter().zip(b.iter()) {
                    if !self.valores_son_iguales_simple(elem_a, elem_b) {
                        return false;
                    }
                }
                true
            },
            (Valor::Json(a), Valor::Json(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                for (clave, valor_a) in a {
                    if let Some(valor_b) = b.get(clave) {
                        if !self.valores_son_iguales_simple(valor_a, valor_b) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            },
            // Conversiones automáticas
            (Valor::Entero(a), Valor::Numero(b)) => *a as f64 == *b,
            (Valor::Numero(a), Valor::Entero(b)) => *a == *b as f64,
            _ => false,
        }
    }
}
