// Analizador sintáctico para el lenguaje Quetzal
// Convierte la secuencia de tokens en un Árbol de Sintaxis Abstracta (AST)

use crate::analizador_lexico::{Token, TipoToken};
use crate::errores::{ErrorQuetzal, ResultadoQuetzal};
use crate::tipos_datos::Valor;
use std::collections::HashMap;

/// Nodo del Árbol de Sintaxis Abstracta
#[derive(Debug, Clone, PartialEq)]
pub enum Nodo {
    // Programa principal
    Programa(Vec<Nodo>),
    
    // Declaraciones
    DeclaracionVariable {
        nombre: String,
        tipo_dato: String,
        es_mutable: bool,
        valor: Option<Box<Nodo>>,
        linea: usize,
    },
    
    DeclaracionFuncion {
        nombre: String,
        parametros: Vec<Parametro>,
        tipo_retorno: String,
        cuerpo: Box<Nodo>,
        es_asincrona: bool,
        linea: usize,
    },
    
    DeclaracionObjeto {
        nombre: String,
        miembros: Vec<Nodo>,
        linea: usize,
    },
    
    // Expresiones
    Literal(Valor),
    Identificador(String),
    
    // Operaciones binarias
    OperacionBinaria {
        izquierdo: Box<Nodo>,
        operador: String,
        derecho: Box<Nodo>,
    },
    
    // Operaciones unarias
    OperacionUnaria {
        operador: String,
        operando: Box<Nodo>,
    },
    
    // Asignación
    Asignacion {
        nombre: String,
        valor: Box<Nodo>,
        linea: usize,
    },
    
    // Asignación compuesta (+=, -=, etc.)
    AsignacionCompuesta {
        nombre: String,
        operador: String,
        valor: Box<Nodo>,
        linea: usize,
    },
    
    // Asignación por índice (lista[indice] = valor)
    AsignacionIndice {
        objeto: Box<Nodo>,
        indice: Box<Nodo>,
        valor: Box<Nodo>,
        linea: usize,
    },
    
    // Llamada a función
    LlamadaFuncion {
        nombre: String,
        argumentos: Vec<Nodo>,
        linea: usize,
    },
    
    // Llamada a método en expresión (objeto.metodo(argumentos))
    LlamadaMetodo {
        objeto: Box<Nodo>,
        metodo: String,
        argumentos: Vec<Nodo>,
        linea: usize,
    },
    
    // Acceso a miembro (objeto.miembro)
    AccesoMiembro {
        objeto: Box<Nodo>,
        miembro: String,
        linea: usize,
    },
    
    // Acceso a índice (lista[indice])
    AccesoIndice {
        objeto: Box<Nodo>,
        indice: Box<Nodo>,
        linea: usize,
    },
    
    // Estructuras de control
    Condicional {
        condicion: Box<Nodo>,
        bloque_si: Box<Nodo>,
        bloque_sino: Option<Box<Nodo>>,
        linea: usize,
    },
    
    BuclePara {
        inicializacion: Option<Box<Nodo>>,
        condicion: Option<Box<Nodo>>,
        incremento: Option<Box<Nodo>>,
        cuerpo: Box<Nodo>,
        linea: usize,
    },
    
    BucleMientras {
        condicion: Box<Nodo>,
        cuerpo: Box<Nodo>,
        linea: usize,
    },
    
    BucleHacerMientras {
        cuerpo: Box<Nodo>,
        condicion: Box<Nodo>,
        linea: usize,
    },
    
    BucleParaCada {
        variable: String,
        iterable: Box<Nodo>,
        cuerpo: Box<Nodo>,
        linea: usize,
    },
    
    // Control de flujo
    Retornar {
        valor: Option<Box<Nodo>>,
        linea: usize,
    },
    
    Romper { linea: usize },
    Continuar { linea: usize },
    
    // Bloque de código
    Bloque(Vec<Nodo>),
    
    // Lista
    Lista(Vec<Nodo>),
    
    // Objeto JSON
    ObjetoJson(HashMap<String, Nodo>),
    
    // Conversión de tipos encadenada
    ConversionTipo {
        expresion: Box<Nodo>,
        tipo_destino: String,
        linea: usize,
    },
    
    // Operador ternario
    OperadorTernario {
        condicion: Box<Nodo>,
        valor_verdadero: Box<Nodo>,
        valor_falso: Box<Nodo>,
    },
    
    // Manejo de excepciones
    BloqueIntentar {
        bloque_intentar: Box<Nodo>,
        bloques_atrapar: Vec<BloqueAtrapar>,
        bloque_finalmente: Option<Box<Nodo>>,
        linea: usize,
    },
    
    Lanzar {
        excepcion: Box<Nodo>,
        linea: usize,
    },
    
    // Concatenación de cadenas con variables
    ConcatenacionVariable {
        plantilla: String,
        variables: Vec<Nodo>,
        linea: usize,
    },
}

/// Parámetro de función
#[derive(Debug, Clone, PartialEq)]
pub struct Parametro {
    pub nombre: String,
    pub tipo_dato: String,
    pub es_mutable: bool,
    pub valor_defecto: Option<Valor>,
}

/// Bloque de captura de excepciones
#[derive(Debug, Clone, PartialEq)]
pub struct BloqueAtrapar {
    pub tipo_excepcion: String,
    pub nombre_variable: String,
    pub bloque: Nodo,
}

/// Analizador sintáctico que convierte tokens en AST
pub struct AnalizadorSintactico {
    tokens: Vec<Token>,
    posicion_actual: usize,
}

impl AnalizadorSintactico {
    /// Crea un nuevo analizador sintáctico
    pub fn nuevo(tokens: Vec<Token>) -> Self {
        AnalizadorSintactico {
            tokens,
            posicion_actual: 0,
        }
    }
    
    /// Analiza los tokens y genera el AST
    pub fn analizar(&mut self) -> ResultadoQuetzal<Nodo> {
        let mut declaraciones = Vec::new();
        
        while !self.esta_al_final() {
            // Saltar nueva línea
            if self.coincidir(&TipoToken::NuevaLinea) {
                continue;
            }
            
            match self.declaracion() {
                Ok(nodo) => declaraciones.push(nodo),
                Err(error) => return Err(error),
            }
        }
        
        Ok(Nodo::Programa(declaraciones))
    }
    
    /// Analiza una declaración
    fn declaracion(&mut self) -> ResultadoQuetzal<Nodo> {
        // Verificar si es una declaración de función
        if self.verificar_declaracion_funcion() {
            return self.declaracion_funcion();
        }
        
        // Verificar si es una declaración de objeto
        if self.coincidir(&TipoToken::Objeto) {
            return self.declaracion_objeto();
        }
        
        // Verificar si es un condicional
        if self.coincidir(&TipoToken::Si) {
            return self.condicional();
        }
        
        // Verificar si es un bucle mientras
        if self.coincidir(&TipoToken::Mientras) {
            return self.bucle_mientras();
        }
        
        // Verificar si es un bucle para
        if self.coincidir(&TipoToken::Para) {
            return self.bucle_para();
        }
        
        // Verificar si es un bucle hacer-mientras
        if self.coincidir(&TipoToken::Hacer) {
            return self.bucle_hacer_mientras();
        }
        
        // Verificar si es una declaración de retorno
        if self.coincidir(&TipoToken::Retornar) {
            return self.declaracion_retorno();
        }
        
        // Verificar si es romper
        if self.coincidir(&TipoToken::Romper) {
            let linea = self.token_anterior().linea;
            return Ok(Nodo::Romper { linea });
        }
        
        // Verificar si es continuar
        if self.coincidir(&TipoToken::Continuar) {
            let linea = self.token_anterior().linea;
            return Ok(Nodo::Continuar { linea });
        }
        
        // Verificar si es una declaración de retorno
        if self.coincidir(&TipoToken::Retornar) {
            return self.declaracion_retorno();
        }
        
        // Verificar si es una declaración de variable
        if self.es_tipo_dato(&self.token_actual().tipo) {
            return self.declaracion_variable();
        }
        
        // Si no es una declaración, es una expresión
        self.expresion()
    }
    
    /// Verifica si el siguiente token es una declaración de función
    fn verificar_declaracion_funcion(&self) -> bool {
        // Buscar patrones como: tipo identificador( o asincrono tipo identificador(
        let mut pos = self.posicion_actual;
        
        // Saltar 'asincrono' si existe
        if pos < self.tokens.len() && matches!(self.tokens[pos].tipo, TipoToken::Asincrono) {
            pos += 1;
        }
        
        // Debe haber un tipo de dato
        if pos >= self.tokens.len() || !self.es_tipo_dato(&self.tokens[pos].tipo) {
            return false;
        }
        pos += 1;
        
        // Debe haber un identificador
        if pos >= self.tokens.len() || !matches!(self.tokens[pos].tipo, TipoToken::Identificador(_)) {
            return false;
        }
        pos += 1;
        
        // Debe haber un paréntesis de apertura
        pos < self.tokens.len() && matches!(self.tokens[pos].tipo, TipoToken::ParentesisAbre)
    }
    
    /// Analiza una declaración de función
    fn declaracion_funcion(&mut self) -> ResultadoQuetzal<Nodo> {
        let linea = self.token_actual().linea;
        
        // Verificar si es asíncrona
        let es_asincrona = self.coincidir(&TipoToken::Asincrono);
        
        // Tipo de retorno
        let tipo_retorno = if self.es_tipo_dato(&self.token_actual().tipo) {
            match &self.token_actual().tipo {
                TipoToken::TipoVacio => { self.avanzar(); "vacio".to_string() },
                TipoToken::TipoEntero => { self.avanzar(); "entero".to_string() },
                TipoToken::TipoNumero => { self.avanzar(); "número".to_string() },
                TipoToken::TipoCadena => { self.avanzar(); "cadena".to_string() },
                TipoToken::TipoBool => { self.avanzar(); "bool".to_string() },
                TipoToken::TipoLista => { self.avanzar(); "lista".to_string() },
                TipoToken::TipoJson => { self.avanzar(); "jsn".to_string() },
                _ => return Err(ErrorQuetzal::ErrorSintaxis {
                    linea: self.token_actual().linea,
                    mensaje: "Se esperaba un tipo de dato para la función".to_string(),
                }),
            }
        } else {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba un tipo de retorno para la función".to_string(),
            });
        };
        
        // Nombre de la función
        let nombre = if let TipoToken::Identificador(nom) = &self.token_actual().tipo {
            let n = nom.clone();
            self.avanzar();
            n
        } else {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba el nombre de la función".to_string(),
            });
        };
        
        // Parámetros
        if !self.coincidir(&TipoToken::ParentesisAbre) {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba '(' después del nombre de la función".to_string(),
            });
        }
        
        let mut parametros = Vec::new();
        let mut nombres_parametros = std::collections::HashSet::new();
        
        if !self.verificar(&TipoToken::ParentesisCierra) {
            loop {
                let parametro = self.parametro_funcion()?;
                
                // Verificar parámetros duplicados
                if nombres_parametros.contains(&parametro.nombre) {
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea: self.token_actual().linea,
                        mensaje: format!("El parámetro '{}' está duplicado", parametro.nombre),
                    });
                }
                nombres_parametros.insert(parametro.nombre.clone());
                
                parametros.push(parametro);
                
                if !self.coincidir(&TipoToken::Coma) {
                    break;
                }
            }
        }
        
        if !self.coincidir(&TipoToken::ParentesisCierra) {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba ')' después de los parámetros".to_string(),
            });
        }
        
        // Cuerpo de la función
        let cuerpo = Box::new(self.bloque_o_expresion()?);
        
        Ok(Nodo::DeclaracionFuncion {
            nombre,
            parametros,
            tipo_retorno,
            cuerpo,
            es_asincrona,
            linea,
        })
    }
    
    /// Analiza un parámetro de función
    fn parametro_funcion(&mut self) -> ResultadoQuetzal<Parametro> {
        // Tipo del parámetro
        let tipo_dato = if self.es_tipo_dato(&self.token_actual().tipo) {
            match &self.token_actual().tipo {
                TipoToken::TipoVacio => { self.avanzar(); "vacio".to_string() },
                TipoToken::TipoEntero => { self.avanzar(); "entero".to_string() },
                TipoToken::TipoNumero => { self.avanzar(); "número".to_string() },
                TipoToken::TipoCadena => { self.avanzar(); "cadena".to_string() },
                TipoToken::TipoBool => { self.avanzar(); "bool".to_string() },
                TipoToken::TipoLista => { 
                    self.avanzar(); 
                    // Verificar si hay un tipo genérico <tipo>
                    if self.coincidir(&TipoToken::Menor) {
                        // Leer el tipo interno
                        let tipo_interno = if self.es_tipo_dato(&self.token_actual().tipo) {
                            match &self.token_actual().tipo {
                                TipoToken::TipoEntero => { self.avanzar(); "entero" },
                                TipoToken::TipoNumero => { self.avanzar(); "número" },
                                TipoToken::TipoCadena => { self.avanzar(); "cadena" },
                                TipoToken::TipoBool => { self.avanzar(); "bool" },
                                _ => return Err(ErrorQuetzal::ErrorSintaxis {
                                    linea: self.token_actual().linea,
                                    mensaje: "Tipo genérico no válido para lista en parámetro".to_string(),
                                }),
                            }
                        } else {
                            return Err(ErrorQuetzal::ErrorSintaxis {
                                linea: self.token_actual().linea,
                                mensaje: "Se esperaba un tipo para la lista en parámetro".to_string(),
                            });
                        };
                        
                        // Esperar el cierre >
                        if !self.coincidir(&TipoToken::Mayor) {
                            return Err(ErrorQuetzal::ErrorSintaxis {
                                linea: self.token_actual().linea,
                                mensaje: "Se esperaba '>' después del tipo de lista en parámetro".to_string(),
                            });
                        }
                        
                        format!("lista<{}>", tipo_interno)
                    } else {
                        "lista".to_string()
                    }
                },
                TipoToken::TipoJson => { self.avanzar(); "jsn".to_string() },
                _ => return Err(ErrorQuetzal::ErrorSintaxis {
                    linea: self.token_actual().linea,
                    mensaje: "Se esperaba un tipo de dato para el parámetro".to_string(),
                }),
            }
        } else {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba un tipo de dato para el parámetro".to_string(),
            });
        };
        
        // Verificar si es mutable
        let es_mutable = self.coincidir(&TipoToken::Mut);
        
        // Nombre del parámetro
        let nombre = if let TipoToken::Identificador(nom) = &self.token_actual().tipo {
            let n = nom.clone();
            self.avanzar();
            n
        } else {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba el nombre del parámetro".to_string(),
            });
        };
        
        // Valor por defecto (opcional)
        let valor_defecto = if self.coincidir(&TipoToken::Asignacion) {
            Some(self.valor_literal()?)
        } else {
            None
        };
        
        Ok(Parametro {
            nombre,
            tipo_dato,
            es_mutable,
            valor_defecto,
        })
    }
    
    /// Analiza una declaración de objeto
    fn declaracion_objeto(&mut self) -> ResultadoQuetzal<Nodo> {
        let linea = self.token_actual().linea;
        
        // Nombre del objeto
        let nombre = if let TipoToken::Identificador(nom) = &self.token_actual().tipo {
            let n = nom.clone();
            self.avanzar();
            n
        } else {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba el nombre del objeto".to_string(),
            });
        };
        
        if !self.coincidir(&TipoToken::LlaveAbre) {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba '{' después del nombre del objeto".to_string(),
            });
        }
        
        let mut miembros = Vec::new();
        
        while !self.verificar(&TipoToken::LlaveCierra) && !self.esta_al_final() {
            // Saltar nueva línea
            if self.coincidir(&TipoToken::NuevaLinea) {
                continue;
            }
            
            // Verificar modificadores de acceso
            if self.coincidir(&TipoToken::Publico) || self.coincidir(&TipoToken::Privado) {
                if !self.coincidir(&TipoToken::DosPuntos) {
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea: self.token_actual().linea,
                        mensaje: "Se esperaba ':' después del modificador de acceso".to_string(),
                    });
                }
                continue;
            }
            
            let miembro = self.declaracion()?;
            miembros.push(miembro);
        }
        
        if !self.coincidir(&TipoToken::LlaveCierra) {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba '}' al final del objeto".to_string(),
            });
        }
        
        Ok(Nodo::DeclaracionObjeto {
            nombre,
            miembros,
            linea,
        })
    }
    
    /// Analiza una declaración de variable
    fn declaracion_variable(&mut self) -> ResultadoQuetzal<Nodo> {
        let linea = self.token_actual().linea;
        
        // Tipo de la variable
        let tipo_dato = if self.es_tipo_dato(&self.token_actual().tipo) {
            match &self.token_actual().tipo {
                TipoToken::TipoVacio => { self.avanzar(); "vacio".to_string() },
                TipoToken::TipoEntero => { self.avanzar(); "entero".to_string() },
                TipoToken::TipoNumero => { self.avanzar(); "número".to_string() },
                TipoToken::TipoCadena => { self.avanzar(); "cadena".to_string() },
                TipoToken::TipoBool => { self.avanzar(); "bool".to_string() },
                TipoToken::TipoLista => { 
                    self.avanzar(); 
                    // Verificar si hay un tipo genérico <tipo>
                    if self.coincidir(&TipoToken::Menor) {
                        // Leer el tipo interno
                        let tipo_interno = if self.es_tipo_dato(&self.token_actual().tipo) {
                            match &self.token_actual().tipo {
                                TipoToken::TipoEntero => { self.avanzar(); "entero" },
                                TipoToken::TipoNumero => { self.avanzar(); "número" },
                                TipoToken::TipoCadena => { self.avanzar(); "cadena" },
                                TipoToken::TipoBool => { self.avanzar(); "bool" },
                                _ => return Err(ErrorQuetzal::ErrorSintaxis {
                                    linea: self.token_actual().linea,
                                    mensaje: "Tipo genérico no válido para lista".to_string(),
                                }),
                            }
                        } else {
                            return Err(ErrorQuetzal::ErrorSintaxis {
                                linea: self.token_actual().linea,
                                mensaje: "Se esperaba un tipo para la lista".to_string(),
                            });
                        };
                        
                        // Esperar el cierre >
                        if !self.coincidir(&TipoToken::Mayor) {
                            return Err(ErrorQuetzal::ErrorSintaxis {
                                linea: self.token_actual().linea,
                                mensaje: "Se esperaba '>' después del tipo de lista".to_string(),
                            });
                        }
                        
                        format!("lista<{}>", tipo_interno)
                    } else {
                        "lista".to_string()
                    }
                },
                TipoToken::TipoJson => { self.avanzar(); "jsn".to_string() },
                _ => return Err(ErrorQuetzal::ErrorSintaxis {
                    linea: self.token_actual().linea,
                    mensaje: "Tipo de dato no válido".to_string(),
                }),
            }
        } else {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba un tipo de dato".to_string(),
            });
        };
        
        // Verificar si es mutable
        let es_mutable = self.coincidir(&TipoToken::Mut);
        
        // Nombre de la variable
        let nombre = if let TipoToken::Identificador(nom) = &self.token_actual().tipo {
            let n = nom.clone();
            self.avanzar();
            n
        } else {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba el nombre de la variable".to_string(),
            });
        };
        
        // Valor inicial (opcional)
        let valor = if self.coincidir(&TipoToken::Asignacion) {
            Some(Box::new(self.expresion()?))
        } else {
            None
        };
        
        Ok(Nodo::DeclaracionVariable {
            nombre,
            tipo_dato,
            es_mutable,
            valor,
            linea,
        })
    }
    
    /// Analiza una expresión
    fn expresion(&mut self) -> ResultadoQuetzal<Nodo> {
        self.asignacion()
    }
    
    /// Analiza asignaciones (simples y compuestas)
    fn asignacion(&mut self) -> ResultadoQuetzal<Nodo> {
        let mut expresion = self.operador_ternario()?;
        
        // Verificar si es una asignación por índice (expresion[indice] = valor)
        if let Nodo::AccesoIndice { objeto, indice, linea } = &expresion {
            if self.coincidir(&TipoToken::Asignacion) {
                let valor = Box::new(self.asignacion()?);
                
                return Ok(Nodo::AsignacionIndice {
                    objeto: objeto.clone(),
                    indice: indice.clone(),
                    valor,
                    linea: *linea,
                });
            }
        }
        
        // Verificar si es una asignación de variable simple
        if let Nodo::Identificador(nombre) = &expresion {
            if self.coincidir(&TipoToken::Asignacion) {
                // Asignación simple (=)
                let valor = Box::new(self.asignacion()?);
                let linea = self.token_anterior().linea;
                
                return Ok(Nodo::Asignacion {
                    nombre: nombre.clone(),
                    valor,
                    linea,
                });
            } else if self.coincidir(&TipoToken::AsignacionSuma) ||
                      self.coincidir(&TipoToken::AsignacionResta) ||
                      self.coincidir(&TipoToken::AsignacionMult) ||
                      self.coincidir(&TipoToken::AsignacionDiv) ||
                      self.coincidir(&TipoToken::AsignacionMod) {
                // Asignación compuesta (+=, -=, etc.)
                let operador = self.token_anterior().lexema.clone();
                let valor = Box::new(self.asignacion()?);
                let linea = self.token_anterior().linea;
                
                return Ok(Nodo::AsignacionCompuesta {
                    nombre: nombre.clone(),
                    operador,
                    valor,
                    linea,
                });
            }
        }
        
        Ok(expresion)
    }
    
    /// Analiza operador ternario
    fn operador_ternario(&mut self) -> ResultadoQuetzal<Nodo> {
        let mut expresion = self.o_logico()?;
        
        if self.coincidir(&TipoToken::Pregunta) {
            let valor_verdadero = Box::new(self.expresion()?);
            
            if !self.coincidir(&TipoToken::DosPuntos) {
                return Err(ErrorQuetzal::ErrorSintaxis {
                    linea: self.token_actual().linea,
                    mensaje: "Se esperaba ':' en operador ternario".to_string(),
                });
            }
            
            let valor_falso = Box::new(self.expresion()?);
            
            expresion = Nodo::OperadorTernario {
                condicion: Box::new(expresion),
                valor_verdadero,
                valor_falso,
            };
        }
        
        Ok(expresion)
    }
    
    /// Analiza OR lógico
    fn o_logico(&mut self) -> ResultadoQuetzal<Nodo> {
        let mut expresion = self.y_logico()?;
        
        while self.coincidir(&TipoToken::OrLogico) || self.coincidir(&TipoToken::O) {
            let operador = self.token_anterior().lexema.clone();
            let derecho = Box::new(self.y_logico()?);
            expresion = Nodo::OperacionBinaria {
                izquierdo: Box::new(expresion),
                operador,
                derecho,
            };
        }
        
        Ok(expresion)
    }
    
    /// Analiza AND lógico
    fn y_logico(&mut self) -> ResultadoQuetzal<Nodo> {
        let mut expresion = self.igualdad()?;
        
        while self.coincidir(&TipoToken::AndLogico) || self.coincidir(&TipoToken::Y) {
            let operador = self.token_anterior().lexema.clone();
            let derecho = Box::new(self.igualdad()?);
            expresion = Nodo::OperacionBinaria {
                izquierdo: Box::new(expresion),
                operador,
                derecho,
            };
        }
        
        Ok(expresion)
    }
    
    /// Analiza igualdad y desigualdad
    fn igualdad(&mut self) -> ResultadoQuetzal<Nodo> {
        let mut expresion = self.comparacion()?;
        
        while self.coincidir(&TipoToken::Igual) || self.coincidir(&TipoToken::Diferente) {
            let operador = self.token_anterior().lexema.clone();
            let derecho = Box::new(self.comparacion()?);
            expresion = Nodo::OperacionBinaria {
                izquierdo: Box::new(expresion),
                operador,
                derecho,
            };
        }
        
        Ok(expresion)
    }
    
    /// Analiza comparaciones
    fn comparacion(&mut self) -> ResultadoQuetzal<Nodo> {
        let mut expresion = self.suma_resta()?;
        
        while self.coincidir(&TipoToken::Mayor) || self.coincidir(&TipoToken::MayorIgual) ||
              self.coincidir(&TipoToken::Menor) || self.coincidir(&TipoToken::MenorIgual) {
            let operador = self.token_anterior().lexema.clone();
            let derecho = Box::new(self.suma_resta()?);
            expresion = Nodo::OperacionBinaria {
                izquierdo: Box::new(expresion),
                operador,
                derecho,
            };
        }
        
        Ok(expresion)
    }
    
    /// Analiza suma y resta
    fn suma_resta(&mut self) -> ResultadoQuetzal<Nodo> {
        let mut expresion = self.multiplicacion_division()?;
        
        while self.coincidir(&TipoToken::Suma) || self.coincidir(&TipoToken::Resta) {
            let operador = self.token_anterior().lexema.clone();
            let derecho = Box::new(self.multiplicacion_division()?);
            expresion = Nodo::OperacionBinaria {
                izquierdo: Box::new(expresion),
                operador,
                derecho,
            };
        }
        
        Ok(expresion)
    }
    
    /// Analiza multiplicación, división y módulo
    fn multiplicacion_division(&mut self) -> ResultadoQuetzal<Nodo> {
        let mut expresion = self.unario()?;
        
        while self.coincidir(&TipoToken::Multiplicacion) || 
              self.coincidir(&TipoToken::Division) || 
              self.coincidir(&TipoToken::Modulo) {
            let operador = self.token_anterior().lexema.clone();
            let derecho = Box::new(self.unario()?);
            expresion = Nodo::OperacionBinaria {
                izquierdo: Box::new(expresion),
                operador,
                derecho,
            };
        }
        
        Ok(expresion)
    }
    
    /// Analiza operaciones unarias
    fn unario(&mut self) -> ResultadoQuetzal<Nodo> {
        if self.coincidir(&TipoToken::Not) || self.coincidir(&TipoToken::Resta) {
            let operador = self.token_anterior().lexema.clone();
            let operando = Box::new(self.unario()?);
            return Ok(Nodo::OperacionUnaria { operador, operando });
        }
        
        self.acceso_miembro()
    }
    
    /// Analiza acceso a miembros y llamadas
    fn acceso_miembro(&mut self) -> ResultadoQuetzal<Nodo> {
        let mut expresion = self.primario()?;
        
        loop {
            if self.coincidir(&TipoToken::Punto) {
                // Acceso a miembro - permitir identificadores o tipos de datos como nombres de miembro
                let miembro = if let TipoToken::Identificador(nombre_miembro) = &self.token_actual().tipo {
                    let m = nombre_miembro.clone();
                    self.avanzar();
                    m
                } else if self.es_tipo_dato(&self.token_actual().tipo) {
                    // Permitir tipos de datos como nombres de método (para conversiones)
                    let m = match &self.token_actual().tipo {
                        TipoToken::TipoCadena => "cadena".to_string(),
                        TipoToken::TipoEntero => "entero".to_string(),
                        TipoToken::TipoNumero => "numero".to_string(),
                        TipoToken::TipoBool => "bool".to_string(),
                        TipoToken::TipoLista => "lista".to_string(),
                        TipoToken::TipoJson => "jsn".to_string(),
                        _ => "desconocido".to_string(),
                    };
                    self.avanzar();
                    m
                } else {
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea: self.token_actual().linea,
                        mensaje: "Se esperaba el nombre del miembro después de '.'".to_string(),
                    });
                };
                
                let linea = self.token_actual().linea;
                
                // Verificar si es una llamada a método
                if self.verificar(&TipoToken::ParentesisAbre) {
                    // Es una llamada a método
                    self.avanzar(); // Consumir '('
                    let mut argumentos = Vec::new();
                    
                    if !self.verificar(&TipoToken::ParentesisCierra) {
                        loop {
                            argumentos.push(self.expresion()?);
                            if !self.coincidir(&TipoToken::Coma) {
                                break;
                            }
                        }
                    }
                    
                    if !self.coincidir(&TipoToken::ParentesisCierra) {
                        return Err(ErrorQuetzal::ErrorSintaxis {
                            linea: self.token_actual().linea,
                            mensaje: "Se esperaba ')' después de los argumentos".to_string(),
                        });
                    }
                    
                    // Crear nodo de llamada a método
                    match &expresion {
                        Nodo::Identificador(var_name) => {
                            // Para variables, usar llamada a función tradicional
                            let nombre_metodo = format!("{}.{}", var_name, miembro);
                            expresion = Nodo::LlamadaFuncion {
                                nombre: nombre_metodo,
                                argumentos,
                                linea,
                            };
                        },
                        _ => {
                            // Para expresiones complejas, usar el nuevo nodo LlamadaMetodo
                            expresion = Nodo::LlamadaMetodo {
                                objeto: Box::new(expresion),
                                metodo: miembro,
                                argumentos,
                                linea,
                            };
                        }
                    }
                } else {
                    // Es acceso simple a miembro
                    expresion = Nodo::AccesoMiembro {
                        objeto: Box::new(expresion),
                        miembro,
                        linea,
                    };
                }
            } else if self.coincidir(&TipoToken::CorcheteAbre) {
                // Acceso a índice
                let indice = Box::new(self.expresion()?);
                let linea = self.token_actual().linea;
                
                if !self.coincidir(&TipoToken::CorcheteCierra) {
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea: self.token_actual().linea,
                        mensaje: "Se esperaba ']' después del índice".to_string(),
                    });
                }
                
                expresion = Nodo::AccesoIndice {
                    objeto: Box::new(expresion),
                    indice,
                    linea,
                };
            } else if self.verificar(&TipoToken::ParentesisAbre) {
                // Llamada a función
                self.avanzar(); // Consumir '('
                let mut argumentos = Vec::new();
                let linea = self.token_actual().linea;
                
                if !self.verificar(&TipoToken::ParentesisCierra) {
                    loop {
                        argumentos.push(self.expresion()?);
                        if !self.coincidir(&TipoToken::Coma) {
                            break;
                        }
                    }
                }
                
                if !self.coincidir(&TipoToken::ParentesisCierra) {
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea: self.token_actual().linea,
                        mensaje: "Se esperaba ')' después de los argumentos".to_string(),
                    });
                }
                
                if let Nodo::Identificador(nombre_funcion) = expresion {
                    expresion = Nodo::LlamadaFuncion {
                        nombre: nombre_funcion,
                        argumentos,
                        linea,
                    };
                } else {
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea,
                        mensaje: "Expresión no válida para llamada a función".to_string(),
                    });
                }
            } else if self.coincidir(&TipoToken::Incremento) {
                // Operador de incremento postfijo i++
                let linea = self.token_anterior().linea;
                if let Nodo::Identificador(ref nombre) = expresion {
                    let nombre_clonado = nombre.clone();
                    expresion = Nodo::Asignacion {
                        nombre: nombre_clonado,
                        valor: Box::new(Nodo::OperacionBinaria {
                            izquierdo: Box::new(expresion.clone()),
                            operador: "+".to_string(),
                            derecho: Box::new(Nodo::Literal(Valor::Entero(1))),
                        }),
                        linea,
                    };
                } else {
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea,
                        mensaje: "Solo se puede aplicar incremento a variables".to_string(),
                    });
                }
            } else {
                break;
            }
        }
        
        Ok(expresion)
    }
    
    /// Analiza expresiones primarias (literales, identificadores, etc.)
    fn primario(&mut self) -> ResultadoQuetzal<Nodo> {
        // Literales booleanos
        if self.coincidir(&TipoToken::Verdadero) {
            return Ok(Nodo::Literal(Valor::Bool(true)));
        }
        
        if self.coincidir(&TipoToken::Falso) {
            return Ok(Nodo::Literal(Valor::Bool(false)));
        }
        
        // Literales numéricos y de cadena
        match &self.token_actual().tipo {
            TipoToken::LiteralEntero(n) => {
                let valor = *n;
                self.avanzar();
                Ok(Nodo::Literal(Valor::Entero(valor)))
            },
            TipoToken::LiteralNumero(n) => {
                let valor = *n;
                self.avanzar();
                Ok(Nodo::Literal(Valor::Numero(valor)))
            },
            TipoToken::LiteralCadena(s) => {
                let valor = s.clone();
                self.avanzar();
                Ok(Nodo::Literal(Valor::Cadena(valor)))
            },
            TipoToken::Identificador(nombre) => {
                let nom = nombre.clone();
                self.avanzar();
                Ok(Nodo::Identificador(nom))
            },
            _ => {
                // Paréntesis para agrupación
                if self.coincidir(&TipoToken::ParentesisAbre) {
                    let expresion = self.expresion()?;
                    if !self.coincidir(&TipoToken::ParentesisCierra) {
                        return Err(ErrorQuetzal::ErrorSintaxis {
                            linea: self.token_actual().linea,
                            mensaje: "Se esperaba ')' después de la expresión".to_string(),
                        });
                    }
                    return Ok(expresion);
                }
                
                // Listas
                if self.coincidir(&TipoToken::CorcheteAbre) {
                    let mut elementos = Vec::new();
                    
                    if !self.verificar(&TipoToken::CorcheteCierra) {
                        loop {
                            elementos.push(self.expresion()?);
                            if !self.coincidir(&TipoToken::Coma) {
                                break;
                            }
                        }
                    }
                    
                    if !self.coincidir(&TipoToken::CorcheteCierra) {
                        return Err(ErrorQuetzal::ErrorSintaxis {
                            linea: self.token_actual().linea,
                            mensaje: "Se esperaba ']' al final de la lista".to_string(),
                        });
                    }
                    
                    return Ok(Nodo::Lista(elementos));
                }
                
                // Objetos JSON
                if self.coincidir(&TipoToken::LlaveAbre) {
                    let mut propiedades = HashMap::new();
                    
                    if !self.verificar(&TipoToken::LlaveCierra) {
                        loop {
                            // Clave (puede ser identificador, cadena, tipo de dato, o palabra reservada)
                            let clave = if let TipoToken::Identificador(nom) = &self.token_actual().tipo {
                                let nombre = nom.clone();
                                self.avanzar();
                                nombre
                            } else if let TipoToken::LiteralCadena(s) = &self.token_actual().tipo {
                                let cadena = s.clone();
                                self.avanzar();
                                cadena
                            } else if self.es_tipo_dato(&self.token_actual().tipo) {
                                // Permitir tipos de datos como claves de propiedades
                                let tipo_como_clave = match &self.token_actual().tipo {
                                    TipoToken::TipoVacio => "vacio".to_string(),
                                    TipoToken::TipoEntero => "entero".to_string(),
                                    TipoToken::TipoNumero => "número".to_string(),
                                    TipoToken::TipoCadena => "cadena".to_string(),
                                    TipoToken::TipoBool => "bool".to_string(),
                                    TipoToken::TipoLista => "lista".to_string(),
                                    TipoToken::TipoJson => "jsn".to_string(),
                                    _ => return Err(ErrorQuetzal::ErrorSintaxis {
                                        linea: self.token_actual().linea,
                                        mensaje: "Tipo de dato no válido como clave de propiedad".to_string(),
                                    }),
                                };
                                self.avanzar();
                                tipo_como_clave
                            } else if matches!(&self.token_actual().tipo, TipoToken::Verdadero | TipoToken::Falso) {
                                // Permitir valores booleanos como claves
                                let bool_como_clave = match &self.token_actual().tipo {
                                    TipoToken::Verdadero => "verdadero".to_string(),
                                    TipoToken::Falso => "falso".to_string(),
                                    _ => unreachable!(),
                                };
                                self.avanzar();
                                bool_como_clave
                            } else if matches!(&self.token_actual().tipo, 
                                TipoToken::Objeto | TipoToken::Nuevo | TipoToken::Retornar | 
                                TipoToken::Asincrono | TipoToken::Si | TipoToken::Sino |
                                TipoToken::Para | TipoToken::Mientras | TipoToken::Hacer |
                                TipoToken::Romper | TipoToken::Continuar | TipoToken::En) {
                                // Permitir palabras reservadas como claves de propiedades
                                let palabra_como_clave = match &self.token_actual().tipo {
                                    TipoToken::Objeto => "objeto".to_string(),
                                    TipoToken::Nuevo => "nuevo".to_string(),
                                    TipoToken::Retornar => "retornar".to_string(),
                                    TipoToken::Asincrono => "asincrono".to_string(),
                                    TipoToken::Si => "si".to_string(),
                                    TipoToken::Sino => "sino".to_string(),
                                    TipoToken::Para => "para".to_string(),
                                    TipoToken::Mientras => "mientras".to_string(),
                                    TipoToken::Hacer => "hacer".to_string(),
                                    TipoToken::Romper => "romper".to_string(),
                                    TipoToken::Continuar => "continuar".to_string(),
                                    TipoToken::En => "en".to_string(),
                                    _ => return Err(ErrorQuetzal::ErrorSintaxis {
                                        linea: self.token_actual().linea,
                                        mensaje: "Palabra reservada no válida como clave de propiedad".to_string(),
                                    }),
                                };
                                self.avanzar();
                                palabra_como_clave
                            } else {
                                return Err(ErrorQuetzal::ErrorSintaxis {
                                    linea: self.token_actual().linea,
                                    mensaje: "Se esperaba el nombre de la propiedad".to_string(),
                                });
                            };
                            
                            if !self.coincidir(&TipoToken::DosPuntos) {
                                return Err(ErrorQuetzal::ErrorSintaxis {
                                    linea: self.token_actual().linea,
                                    mensaje: "Se esperaba ':' después del nombre de la propiedad".to_string(),
                                });
                            }
                            
                            let valor = self.expresion()?;
                            propiedades.insert(clave, valor);
                            
                            if !self.coincidir(&TipoToken::Coma) {
                                break;
                            }
                        }
                    }
                    
                    if !self.coincidir(&TipoToken::LlaveCierra) {
                        return Err(ErrorQuetzal::ErrorSintaxis {
                            linea: self.token_actual().linea,
                            mensaje: "Se esperaba '}' al final del objeto".to_string(),
                        });
                    }
                    
                    return Ok(Nodo::ObjetoJson(propiedades));
                }
                
                Err(ErrorQuetzal::ErrorSintaxis {
                    linea: self.token_actual().linea,
                    mensaje: format!("Token inesperado: {}", self.token_actual().lexema),
                })
            }
        }
    }
    
    /// Analiza un bloque o una expresión única
    fn bloque_o_expresion(&mut self) -> ResultadoQuetzal<Nodo> {
        if self.coincidir(&TipoToken::LlaveAbre) {
            // Es un bloque
            let mut declaraciones = Vec::new();
            
            while !self.verificar(&TipoToken::LlaveCierra) && !self.esta_al_final() {
                // Saltar nueva línea
                if self.coincidir(&TipoToken::NuevaLinea) {
                    continue;
                }
                
                declaraciones.push(self.declaracion()?);
            }
            
            if !self.coincidir(&TipoToken::LlaveCierra) {
                return Err(ErrorQuetzal::ErrorSintaxis {
                    linea: self.token_actual().linea,
                    mensaje: "Se esperaba '}' al final del bloque".to_string(),
                });
            }
            
            Ok(Nodo::Bloque(declaraciones))
        } else {
            // Es una expresión única
            self.expresion()
        }
    }
    
    /// Obtiene un valor literal del token actual
    fn valor_literal(&mut self) -> ResultadoQuetzal<Valor> {
        match &self.token_actual().tipo {
            TipoToken::LiteralEntero(n) => {
                let valor = *n;
                self.avanzar();
                Ok(Valor::Entero(valor))
            },
            TipoToken::LiteralNumero(n) => {
                let valor = *n;
                self.avanzar();
                Ok(Valor::Numero(valor))
            },
            TipoToken::LiteralCadena(s) => {
                let valor = s.clone();
                self.avanzar();
                Ok(Valor::Cadena(valor))
            },
            TipoToken::Verdadero => {
                self.avanzar();
                Ok(Valor::Bool(true))
            },
            TipoToken::Falso => {
                self.avanzar();
                Ok(Valor::Bool(false))
            },
            _ => Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba un valor literal".to_string(),
            }),
        }
    }
    
    /// Verifica si un tipo de token es un tipo de dato
    fn es_tipo_dato(&self, tipo: &TipoToken) -> bool {
        matches!(tipo, 
            TipoToken::TipoVacio | TipoToken::TipoEntero | TipoToken::TipoNumero |
            TipoToken::TipoCadena | TipoToken::TipoBool | TipoToken::TipoLista |
            TipoToken::TipoJson
        )
    }
    
    /// Verifica si el token actual es del tipo especificado
    fn verificar(&self, tipo: &TipoToken) -> bool {
        if self.esta_al_final() {
            return false;
        }
        std::mem::discriminant(&self.token_actual().tipo) == std::mem::discriminant(tipo)
    }
    
    /// Coincide con el tipo de token y avanza si coincide
    fn coincidir(&mut self, tipo: &TipoToken) -> bool {
        if self.verificar(tipo) {
            self.avanzar();
            true
        } else {
            false
        }
    }
    
    /// Avanza al siguiente token
    fn avanzar(&mut self) -> &Token {
        if !self.esta_al_final() {
            self.posicion_actual += 1;
        }
        self.token_anterior()
    }
    
    /// Verifica si estamos al final de los tokens
    fn esta_al_final(&self) -> bool {
        matches!(self.token_actual().tipo, TipoToken::FinArchivo)
    }
    
    /// Obtiene el token actual
    fn token_actual(&self) -> &Token {
        &self.tokens[self.posicion_actual]
    }
    
    /// Obtiene el token anterior
    fn token_anterior(&self) -> &Token {
        &self.tokens[self.posicion_actual - 1]
    }
    
    /// Analiza un condicional si/sino
    fn condicional(&mut self) -> ResultadoQuetzal<Nodo> {
        let linea = self.token_anterior().linea;
        
        // Expresión de condición
        let condicion = Box::new(self.expresion()?);
        
        // Bloque 'si'
        let bloque_si = Box::new(self.bloque_o_declaracion()?);
        
        // Bloque 'sino' opcional
        let bloque_sino = if self.coincidir(&TipoToken::Sino) {
            Some(Box::new(self.bloque_o_declaracion()?))
        } else {
            None
        };
        
        Ok(Nodo::Condicional {
            condicion,
            bloque_si,
            bloque_sino,
            linea,
        })
    }
    
    /// Analiza un bloque o una sola declaración
    fn bloque_o_declaracion(&mut self) -> ResultadoQuetzal<Nodo> {
        if self.coincidir(&TipoToken::LlaveAbre) {
            // Bloque con llaves
            let mut declaraciones = Vec::new();
            
            while !self.verificar(&TipoToken::LlaveCierra) && !self.esta_al_final() {
                declaraciones.push(self.declaracion()?);
            }
            
            if !self.coincidir(&TipoToken::LlaveCierra) {
                return Err(ErrorQuetzal::ErrorSintaxis {
                    linea: self.token_actual().linea,
                    mensaje: "Se esperaba '}' después del bloque".to_string(),
                });
            }
            
            Ok(Nodo::Bloque(declaraciones))
        } else {
            // Una sola declaración
            self.declaracion()
        }
    }
    
    /// Analiza un bucle mientras
    fn bucle_mientras(&mut self) -> ResultadoQuetzal<Nodo> {
        let linea = self.token_anterior().linea;
        
        // Expresión de condición
        let condicion = Box::new(self.expresion()?);
        
        // Cuerpo del bucle
        let cuerpo = Box::new(self.bloque_o_declaracion()?);
        
        Ok(Nodo::BucleMientras {
            condicion,
            cuerpo,
            linea,
        })
    }
    
    /// Analiza un bucle para
    fn bucle_para(&mut self) -> ResultadoQuetzal<Nodo> {
        let linea = self.token_anterior().linea;
        
        // Verificar si tiene paréntesis (bucle tradicional o foreach)
        if self.coincidir(&TipoToken::ParentesisAbre) {
            // Verificar si es foreach: para (variable en iterable) o para (tipo variable en iterable)
            let pos_guardada = self.posicion_actual;
            
            // Intentar leer variable/tipo variable
            let mut es_foreach = false;
            
            // Saltar tipo si existe
            if self.es_tipo_dato(&self.token_actual().tipo) {
                self.avanzar();
            }
            
            // Si hay identificador seguido de 'en', es foreach
            if let TipoToken::Identificador(_) = &self.token_actual().tipo {
                self.avanzar();
                if self.verificar(&TipoToken::En) {
                    es_foreach = true;
                }
            }
            
            // Restaurar posición
            self.posicion_actual = pos_guardada;
            
            if es_foreach {
                // Bucle foreach: para (variable en iterable) o para (tipo variable en iterable)
                
                // Verificar si hay tipo
                let _tipo_variable = if self.es_tipo_dato(&self.token_actual().tipo) {
                    let tipo = match &self.token_actual().tipo {
                        TipoToken::TipoEntero => "entero".to_string(),
                        TipoToken::TipoNumero => "número".to_string(),
                        TipoToken::TipoCadena => "cadena".to_string(),
                        TipoToken::TipoBool => "bool".to_string(),
                        TipoToken::TipoLista => "lista".to_string(),
                        TipoToken::TipoJson => "jsn".to_string(),
                        _ => "desconocido".to_string(),
                    };
                    self.avanzar();
                    Some(tipo)
                } else {
                    None
                };
                
                // Variable del bucle
                let variable = if let TipoToken::Identificador(nom) = &self.token_actual().tipo {
                    let n = nom.clone();
                    self.avanzar();
                    n
                } else {
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea: self.token_actual().linea,
                        mensaje: "Se esperaba el nombre de la variable en bucle foreach".to_string(),
                    });
                };
                
                // Esperar 'en'
                if !self.coincidir(&TipoToken::En) {
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea: self.token_actual().linea,
                        mensaje: "Se esperaba 'en' en bucle foreach".to_string(),
                    });
                }
                
                // Expresión iterable
                let iterable = Box::new(self.expresion()?);
                
                // Cerrar paréntesis
                if !self.coincidir(&TipoToken::ParentesisCierra) {
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea: self.token_actual().linea,
                        mensaje: "Se esperaba ')' después del iterable".to_string(),
                    });
                }
                
                // Cuerpo del bucle
                let cuerpo = Box::new(self.bloque_o_declaracion()?);
                
                Ok(Nodo::BucleParaCada {
                    variable,
                    iterable,
                    cuerpo,
                    linea,
                })
            } else {
                // Bucle tradicional: para (init; condicion; incremento)
                
                // Inicialización (puede ser declaración de variable o asignación)
                let inicializacion = if self.coincidir(&TipoToken::PuntoYComa) {
                    None // Sin inicialización
                } else {
                    Some(Box::new(self.inicializacion_bucle_para()?))
                };
                
                if !self.coincidir(&TipoToken::PuntoYComa) {
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea: self.token_actual().linea,
                        mensaje: "Se esperaba ';' después de la inicialización del bucle para".to_string(),
                    });
                }
                
                // Condición
                let condicion = if self.coincidir(&TipoToken::PuntoYComa) {
                    None // Sin condición (bucle infinito)
                } else {
                    Some(Box::new(self.expresion()?))
                };
                
                if !self.coincidir(&TipoToken::PuntoYComa) {
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea: self.token_actual().linea,
                        mensaje: "Se esperaba ';' después de la condición del bucle para".to_string(),
                    });
                }
                
                // Incremento
                let incremento = if self.coincidir(&TipoToken::ParentesisCierra) {
                    None // Sin incremento
                } else {
                    let inc = Some(Box::new(self.expresion()?));
                    if !self.coincidir(&TipoToken::ParentesisCierra) {
                        return Err(ErrorQuetzal::ErrorSintaxis {
                            linea: self.token_actual().linea,
                            mensaje: "Se esperaba ')' después del incremento del bucle para".to_string(),
                        });
                    }
                    inc
                };
                
                // Cuerpo del bucle
                let cuerpo = Box::new(self.bloque_o_declaracion()?);
                
                Ok(Nodo::BuclePara {
                    inicializacion,
                    condicion,
                    incremento,
                    cuerpo,
                    linea,
                })
            }
        } else {
            // Bucle para cada sin paréntesis: para variable en iterable
            
            // Variable del bucle
            let variable = if let TipoToken::Identificador(nom) = &self.token_actual().tipo {
                let n = nom.clone();
                self.avanzar();
                n
            } else {
                return Err(ErrorQuetzal::ErrorSintaxis {
                    linea: self.token_actual().linea,
                    mensaje: "Se esperaba el nombre de la variable en bucle para".to_string(),
                });
            };
            
            // Esperar 'en' 
            if !self.coincidir(&TipoToken::En) {
                return Err(ErrorQuetzal::ErrorSintaxis {
                    linea: self.token_actual().linea,
                    mensaje: "Se esperaba 'en' en bucle para".to_string(),
                });
            }
            
            // Expresión iterable (rango o lista)
            let iterable = Box::new(self.expresion()?);
            
            // Cuerpo del bucle
            let cuerpo = Box::new(self.bloque_o_declaracion()?);
            
            Ok(Nodo::BucleParaCada {
                variable,
                iterable,
                cuerpo,
                linea,
            })
        }
    }
    
    /// Analiza un bucle hacer-mientras
    fn bucle_hacer_mientras(&mut self) -> ResultadoQuetzal<Nodo> {
        let linea = self.token_anterior().linea;
        
        // Cuerpo del bucle (debe ser un bloque)
        let cuerpo = Box::new(self.bloque_o_declaracion()?);
        
        // Esperar la palabra 'mientras'
        if !self.coincidir(&TipoToken::Mientras) {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba 'mientras' después del bloque en bucle hacer-mientras".to_string(),
            });
        }
        
        // Expresión de condición entre paréntesis
        if !self.coincidir(&TipoToken::ParentesisAbre) {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba '(' después de 'mientras'".to_string(),
            });
        }
        
        let condicion = Box::new(self.expresion()?);
        
        if !self.coincidir(&TipoToken::ParentesisCierra) {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba ')' después de la condición".to_string(),
            });
        }
        
        Ok(Nodo::BucleHacerMientras {
            cuerpo,
            condicion,
            linea,
        })
    }
    
    /// Analiza una declaración de retorno
    fn declaracion_retorno(&mut self) -> ResultadoQuetzal<Nodo> {
        let linea = self.token_anterior().linea;
        
        // El valor a retornar es opcional
        let valor = if self.verificar(&TipoToken::NuevaLinea) || self.esta_al_final() {
            // Retorno sin valor (implícitamente vacio)
            None
        } else {
            // Retorno con valor
            Some(Box::new(self.expresion()?))
        };
        
        Ok(Nodo::Retornar {
            valor,
            linea,
        })
    }
    
    /// Analiza inicialización de bucle para (solo declaraciones de variables y asignaciones)
    fn inicializacion_bucle_para(&mut self) -> ResultadoQuetzal<Nodo> {
        // Verificar si es una declaración de variable con tipo
        if self.es_tipo_dato(&self.token_actual().tipo) {
            return self.declaracion_variable_bucle();
        }
        
        // Si no, debe ser una asignación
        self.asignacion()
    }
    
    /// Analiza una declaración de variable en bucle (por defecto es mutable)
    fn declaracion_variable_bucle(&mut self) -> ResultadoQuetzal<Nodo> {
        let linea = self.token_actual().linea;
        
        // Tipo de la variable
        let tipo_dato = match &self.token_actual().tipo {
            TipoToken::TipoVacio => { self.avanzar(); "vacio".to_string() },
            TipoToken::TipoEntero => { self.avanzar(); "entero".to_string() },
            TipoToken::TipoNumero => { self.avanzar(); "número".to_string() },
            TipoToken::TipoCadena => { self.avanzar(); "cadena".to_string() },
            TipoToken::TipoBool => { self.avanzar(); "bool".to_string() },
            TipoToken::TipoLista => { self.avanzar(); "lista".to_string() },
            TipoToken::TipoJson => { self.avanzar(); "jsn".to_string() },
            _ => return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba un tipo de dato".to_string(),
            }),
        };
        
        // En bucles, las variables son mutables por defecto (a menos que se especifique explícitamente)
        let es_mutable = if self.coincidir(&TipoToken::Mut) {
            true
        } else {
            true // Por defecto mutable en bucles
        };
        
        // Nombre de la variable
        let nombre = if let TipoToken::Identificador(nom) = &self.token_actual().tipo {
            let n = nom.clone();
            self.avanzar();
            n
        } else {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba un nombre de variable".to_string(),
            });
        };
        
        // Valor inicial (opcional)
        let valor = if self.coincidir(&TipoToken::Asignacion) {
            Some(Box::new(self.expresion()?))
        } else {
            None
        };
        
        Ok(Nodo::DeclaracionVariable {
            nombre,
            tipo_dato,
            es_mutable,
            valor,
            linea,
        })
    }
}
