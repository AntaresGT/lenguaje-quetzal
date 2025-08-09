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
        es_variable: bool,
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
    
    // Creación de instancia de objeto
    CreacionObjeto {
        nombre_clase: String,
        argumentos: Vec<Nodo>,
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
    
    // Asignación a propiedad (objeto.propiedad = valor)
    AsignacionPropiedad {
        objeto: Box<Nodo>,
        propiedad: String,
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    BloqueIntentar {
        bloque_intentar: Box<Nodo>,
        bloques_capturar: Vec<BloqueCapturar>,
        bloque_finalmente: Option<Box<Nodo>>,
        linea: usize,
    },
    
    #[allow(dead_code)]
    Lanzar {
        excepcion: Box<Nodo>,
        linea: usize,
    },
    
    // Concatenación de cadenas con variables
    #[allow(dead_code)]
    ConcatenacionVariable {
        plantilla: String,
        variables: Vec<Nodo>,
        linea: usize,
    },
    
    // Módulos
    DeclaracionImportar {
        elementos: Vec<ElementoImportar>,
        ruta: String,
        linea: usize,
    },
    
    DeclaracionExportar {
        elementos: Vec<String>,
        linea: usize,
    },
}

/// Parámetro de función
#[derive(Debug, Clone, PartialEq)]
pub struct Parametro {
    pub nombre: String,
    pub tipo_dato: String,
    pub es_variable: bool,
    pub valor_defecto: Option<Valor>,
}

/// Elemento de importación para módulos
#[derive(Debug, Clone, PartialEq)]
pub struct ElementoImportar {
    pub nombre: String,
    pub alias: Option<String>,
}

/// Bloque de captura de excepciones
#[derive(Debug, Clone, PartialEq)]
pub struct BloqueCapturar {
    pub tipo_excepcion: String,
    pub nombre_variable: String,
    pub bloque: Nodo,
}

/// Analizador sintáctico que convierte tokens en AST
pub struct AnalizadorSintactico {
    tokens: Vec<Token>,
    posicion_actual: usize,
    dentro_de_constructor: bool,
}

impl AnalizadorSintactico {
    /// Crea un nuevo analizador sintáctico
    pub fn nuevo(tokens: Vec<Token>) -> Self {
        AnalizadorSintactico {
            tokens,
            posicion_actual: 0,
            dentro_de_constructor: false,
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
        // Saltar saltos de línea al principio
        while self.coincidir(&TipoToken::NuevaLinea) {
            continue;
        }
        
        // Verificar si es una declaración de importación
        if self.coincidir(&TipoToken::Importar) {
            return self.declaracion_importar();
        }
        
        // Verificar si es una declaración de exportación
        if self.coincidir(&TipoToken::Exportar) {
            return self.declaracion_exportar();
        }
        
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
        
        // Verificar si es lanzar excepción
        if self.coincidir(&TipoToken::Lanzar) {
            return self.declaracion_lanzar();
        }
        
        // Verificar si es una declaración de retorno
        if self.coincidir(&TipoToken::Retornar) {
            return self.declaracion_retorno();
        }
        
        // Verificar si es un bloque intentar
        if self.coincidir(&TipoToken::Intentar) {
            return self.bloque_intentar();
        }
        
        // Verificar si es una declaración de variable
        if self.es_tipo_dato(&self.token_actual().tipo) {
            return self.declaracion_variable();
        }
        
        // Verificar si es una declaración de variable con tipo personalizado
        // Buscar patrón: Identificador Identificador = ...
        if let TipoToken::Identificador(_) = &self.token_actual().tipo {
            if self.posicion_actual + 1 < self.tokens.len() {
                if let TipoToken::Identificador(_) = &self.tokens[self.posicion_actual + 1].tipo {
                    if self.posicion_actual + 2 < self.tokens.len() {
                        if matches!(&self.tokens[self.posicion_actual + 2].tipo, TipoToken::Asignacion) {
                            return self.declaracion_variable();
                        }
                    }
                }
            }
        }
        
        // Verificar si se intenta asignar a una palabra reservada
        match &self.token_actual().tipo {
            TipoToken::Y | TipoToken::O => {
                // Mirar adelante para ver si hay un '=' o asignación compuesta
                if self.posicion_actual + 1 < self.tokens.len() {
                    match &self.tokens[self.posicion_actual + 1].tipo {
                        TipoToken::Asignacion | TipoToken::AsignacionSuma | TipoToken::AsignacionResta | 
                        TipoToken::AsignacionMult | TipoToken::AsignacionDiv | TipoToken::AsignacionMod => {
                            let nombre = if matches!(self.token_actual().tipo, TipoToken::Y) { "y" } else { "o" };
                            return Err(ErrorQuetzal::ErrorSintaxis {
                                linea: self.token_actual().linea,
                                mensaje: format!("no se puede asignar a la palabra reservada `{}`", nombre),
                            });
                        },
                        _ => {}
                    }
                }
            },
            _ => {}
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
                TipoToken::TipoTexto => { self.avanzar(); "texto".to_string() },
                TipoToken::TipoLog => { self.avanzar(); "log".to_string() },
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
    
    /// Analiza una declaración de importación
    fn declaracion_importar(&mut self) -> ResultadoQuetzal<Nodo> {
        let linea = self.token_actual().linea;
        
        // Debe empezar con '{'
        if !self.coincidir(&TipoToken::LlaveAbre) {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba '{' después de 'importar'".to_string(),
            });
        }
        
        let mut elementos = Vec::new();
        
        // Analizar elementos de importación
        if !self.verificar(&TipoToken::LlaveCierra) {
            loop {
                // Nombre del elemento a importar
                let nombre = if let TipoToken::Identificador(nom) = &self.token_actual().tipo {
                    let n = nom.clone();
                    self.avanzar();
                    n
                } else {
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea: self.token_actual().linea,
                        mensaje: "Se esperaba el nombre del elemento a importar".to_string(),
                    });
                };
                
                // Verificar si hay alias (como)
                let alias = if self.coincidir(&TipoToken::Como) {
                    if let TipoToken::Identificador(nom_alias) = &self.token_actual().tipo {
                        let alias = nom_alias.clone();
                        self.avanzar();
                        Some(alias)
                    } else {
                        return Err(ErrorQuetzal::ErrorSintaxis {
                            linea: self.token_actual().linea,
                            mensaje: "Se esperaba el nombre del alias después de 'como'".to_string(),
                        });
                    }
                } else {
                    None
                };
                
                elementos.push(ElementoImportar { nombre, alias });
                
                if !self.coincidir(&TipoToken::Coma) {
                    break;
                }
            }
        }
        
        // Debe terminar con '}'
        if !self.coincidir(&TipoToken::LlaveCierra) {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba '}' después de los elementos de importación".to_string(),
            });
        }
        
        // Palabra clave 'desde'
        if !self.coincidir(&TipoToken::Desde) {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba 'desde' después de los elementos de importación".to_string(),
            });
        }
        
        // Ruta del módulo (cadena literal)
        let ruta = if let TipoToken::LiteralCadena(ruta_str) = &self.token_actual().tipo {
            let r = ruta_str.clone();
            self.avanzar();
            r
        } else {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba la ruta del módulo como cadena literal".to_string(),
            });
        };
        
        Ok(Nodo::DeclaracionImportar {
            elementos,
            ruta,
            linea,
        })
    }
    
    /// Analiza una declaración de exportación
    fn declaracion_exportar(&mut self) -> ResultadoQuetzal<Nodo> {
        let linea = self.token_actual().linea;
        
        // Debe empezar con '{'
        if !self.coincidir(&TipoToken::LlaveAbre) {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba '{' después de 'exportar'".to_string(),
            });
        }
        
        let mut elementos = Vec::new();
        
        // Analizar elementos de exportación
        if !self.verificar(&TipoToken::LlaveCierra) {
            loop {
                // Nombre del elemento a exportar
                let nombre = if let TipoToken::Identificador(nom) = &self.token_actual().tipo {
                    let n = nom.clone();
                    self.avanzar();
                    n
                } else {
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea: self.token_actual().linea,
                        mensaje: "Se esperaba el nombre del elemento a exportar".to_string(),
                    });
                };
                
                elementos.push(nombre);
                
                if !self.coincidir(&TipoToken::Coma) {
                    break;
                }
            }
        }
        
        // Debe terminar con '}'
        if !self.coincidir(&TipoToken::LlaveCierra) {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba '}' después de los elementos de exportación".to_string(),
            });
        }
        
        Ok(Nodo::DeclaracionExportar {
            elementos,
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
                TipoToken::TipoTexto => { self.avanzar(); "texto".to_string() },
                TipoToken::TipoLog => { self.avanzar(); "log".to_string() },
                TipoToken::TipoLista => { 
                    self.avanzar(); 
                    // Verificar si hay un tipo genérico <tipo>
                    if self.coincidir(&TipoToken::Menor) {
                        // Leer el tipo interno
                        let tipo_interno = if self.es_tipo_dato(&self.token_actual().tipo) {
                            match &self.token_actual().tipo {
                                TipoToken::TipoEntero => { self.avanzar(); "entero" },
                                TipoToken::TipoNumero => { self.avanzar(); "número" },
                                TipoToken::TipoTexto => { self.avanzar(); "texto" },
                                TipoToken::TipoLog => { self.avanzar(); "log" },
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
        
        // Verificar si es variable (antes era mutable)
        let es_variable = self.coincidir(&TipoToken::Var);
        
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
            es_variable,
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
            if self.coincidir(&TipoToken::Publico) {
                if !self.coincidir(&TipoToken::DosPuntos) {
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea: self.token_actual().linea,
                        mensaje: "Se esperaba ':' después de 'publico'".to_string(),
                    });
                }
                // Agregar marcador de sección pública
                miembros.push(Nodo::Identificador("__seccion_publica__".to_string()));
                continue;
            }
            
            if self.coincidir(&TipoToken::Privado) {
                if !self.coincidir(&TipoToken::DosPuntos) {
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea: self.token_actual().linea,
                        mensaje: "Se esperaba ':' después de 'privado'".to_string(),
                    });
                }
                // Agregar marcador de sección privada
                miembros.push(Nodo::Identificador("__seccion_privada__".to_string()));
                continue;
            }
            
            // Verificar si es un constructor (nombre de clase seguido de paréntesis o palabra clave "constructor")
            if let TipoToken::Identificador(nombre_posible) = &self.token_actual().tipo {
                if nombre_posible == &nombre && self.verificar_siguiente(&TipoToken::ParentesisAbre) {
                    // Es un constructor con nombre de clase
                    let miembro = self.declaracion_constructor(&nombre)?;
                    miembros.push(miembro);
                    continue;
                }
            }
            
            // Verificar si es la palabra clave "constructor"
            if self.coincidir(&TipoToken::Constructor) {
                let miembro = self.declaracion_constructor_palabra_clave(&nombre)?;
                miembros.push(miembro);
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
    
    /// Analiza una declaración de constructor
    fn declaracion_constructor(&mut self, nombre_clase: &str) -> ResultadoQuetzal<Nodo> {
        let linea = self.token_actual().linea;
        
        // Nombre del constructor (debe coincidir con el nombre de la clase)
        let nombre = if let TipoToken::Identificador(nom) = &self.token_actual().tipo {
            let n = nom.clone();
            self.avanzar();
            n
        } else {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba el nombre del constructor".to_string(),
            });
        };
        
        // Verificar que el nombre del constructor coincida con la clase
        if nombre != nombre_clase {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea,
                mensaje: format!("El constructor debe tener el mismo nombre que la clase '{}'", nombre_clase),
            });
        }
        
        // Parámetros
        if !self.coincidir(&TipoToken::ParentesisAbre) {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba '(' después del nombre del constructor".to_string(),
            });
        }
        
        let mut parametros = Vec::new();
        
        if !self.verificar(&TipoToken::ParentesisCierra) {
            loop {
                let parametro = self.parametro_funcion()?;
                parametros.push(parametro);
                
                if !self.coincidir(&TipoToken::Coma) {
                    break;
                }
            }
        }
        
        if !self.coincidir(&TipoToken::ParentesisCierra) {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba ')' después de los parámetros del constructor".to_string(),
            });
        }
        
        // Marcar que estamos dentro de un constructor
        let anterior_dentro_de_constructor = self.dentro_de_constructor;
        self.dentro_de_constructor = true;
        
        // Cuerpo de la función
        let cuerpo = Box::new(self.bloque_o_expresion()?);
        
        // Restaurar el estado anterior
        self.dentro_de_constructor = anterior_dentro_de_constructor;
        
        Ok(Nodo::DeclaracionFuncion {
            nombre,
            parametros,
            tipo_retorno: "vacio".to_string(), // Los constructores no retornan nada explícitamente
            cuerpo,
            es_asincrona: false,
            linea,
        })
    }
    
    /// Analiza una declaración de constructor usando la palabra clave "constructor"
    fn declaracion_constructor_palabra_clave(&mut self, _nombre_clase: &str) -> ResultadoQuetzal<Nodo> {
        let linea = self.token_actual().linea;
        
        // Parámetros
        if !self.coincidir(&TipoToken::ParentesisAbre) {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba '(' después de 'constructor'".to_string(),
            });
        }
        
        let mut parametros = Vec::new();
        
        if !self.verificar(&TipoToken::ParentesisCierra) {
            loop {
                // Tipo del parámetro
                let tipo_parametro = if self.es_tipo_dato(&self.token_actual().tipo) {
                    match &self.token_actual().tipo {
                        TipoToken::TipoEntero => { self.avanzar(); "entero".to_string() },
                        TipoToken::TipoNumero => { self.avanzar(); "número".to_string() },
                        TipoToken::TipoTexto => { self.avanzar(); "texto".to_string() },
                        TipoToken::TipoLog => { self.avanzar(); "log".to_string() },
                        TipoToken::TipoLista => { self.avanzar(); "lista".to_string() },
                        TipoToken::TipoJson => { self.avanzar(); "jsn".to_string() },
                        _ => return Err(ErrorQuetzal::ErrorSintaxis {
                            linea: self.token_actual().linea,
                            mensaje: "Se esperaba un tipo de dato válido".to_string(),
                        }),
                    }
                } else if let TipoToken::Identificador(tipo_personalizado) = &self.token_actual().tipo {
                    let tipo = tipo_personalizado.clone();
                    self.avanzar();
                    tipo
                } else {
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea: self.token_actual().linea,
                        mensaje: "Se esperaba un tipo de dato".to_string(),
                    });
                };
                
                // Verificar si es variable
                let es_mutable = self.coincidir(&TipoToken::Var);
                
                // Nombre del parámetro
                let nombre_param = if let TipoToken::Identificador(nom) = &self.token_actual().tipo {
                    let n = nom.clone();
                    self.avanzar();
                    n
                } else {
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea: self.token_actual().linea,
                        mensaje: "Se esperaba el nombre del parámetro".to_string(),
                    });
                };
                
                parametros.push(Parametro {
                    nombre: nombre_param,
                    tipo_dato: tipo_parametro,
                    es_variable: es_mutable,
                    valor_defecto: None,
                });
                
                if !self.coincidir(&TipoToken::Coma) {
                    break;
                }
            }
        }
        
        if !self.coincidir(&TipoToken::ParentesisCierra) {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba ')' después de los parámetros del constructor".to_string(),
            });
        }
        
        // Marcar que estamos dentro de un constructor
        let anterior_dentro_de_constructor = self.dentro_de_constructor;
        self.dentro_de_constructor = true;
        
        // Cuerpo de la función
        let cuerpo = Box::new(self.bloque_o_expresion()?);
        
        // Restaurar el estado anterior
        self.dentro_de_constructor = anterior_dentro_de_constructor;
        
        Ok(Nodo::DeclaracionFuncion {
            nombre: "constructor".to_string(),
            parametros,
            tipo_retorno: "vacio".to_string(),
            cuerpo,
            es_asincrona: false,
            linea,
        })
    }
    
    /// Analiza una declaración de variable
    fn declaracion_variable(&mut self) -> ResultadoQuetzal<Nodo> {
        let linea = self.token_actual().linea;
        
        // Tipo de la variable (puede ser tipo primitivo o tipo personalizado)
        let tipo_dato = if self.es_tipo_dato(&self.token_actual().tipo) {
            match &self.token_actual().tipo {
                TipoToken::TipoVacio => { self.avanzar(); "vacio".to_string() },
                TipoToken::TipoEntero => { self.avanzar(); "entero".to_string() },
                TipoToken::TipoNumero => { self.avanzar(); "número".to_string() },
                TipoToken::TipoTexto => { self.avanzar(); "texto".to_string() },
                TipoToken::TipoLog => { self.avanzar(); "log".to_string() },
                TipoToken::TipoLista => { 
                    self.avanzar(); 
                    // Verificar si hay un tipo genérico <tipo>
                    if self.coincidir(&TipoToken::Menor) {
                        // Leer el tipo interno
                        let tipo_interno = if self.es_tipo_dato(&self.token_actual().tipo) {
                            match &self.token_actual().tipo {
                                TipoToken::TipoEntero => { self.avanzar(); "entero" },
                                TipoToken::TipoNumero => { self.avanzar(); "número" },
                                TipoToken::TipoTexto => { self.avanzar(); "texto" },
                                TipoToken::TipoLog => { self.avanzar(); "log" },
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
        } else if let TipoToken::Identificador(tipo_personalizado) = &self.token_actual().tipo {
            // Tipo personalizado (objeto)
            let tipo = tipo_personalizado.clone();
            self.avanzar();
            tipo
        } else {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba un tipo de dato".to_string(),
            });
        };
        
        // Verificar si es variable
        let es_variable = self.coincidir(&TipoToken::Var);
        
        // Nombre de la variable
        let nombre = match &self.token_actual().tipo {
            TipoToken::Identificador(nom) => {
                let n = nom.clone();
                self.avanzar();
                n
            },
            TipoToken::Y => {
                // Es una palabra reservada
                return Err(ErrorQuetzal::ErrorSintaxis {
                    linea: self.token_actual().linea,
                    mensaje: "no se puede usar la palabra reservada `y` como nombre de variable".to_string(),
                });
            },
            TipoToken::O => {
                // Es una palabra reservada
                return Err(ErrorQuetzal::ErrorSintaxis {
                    linea: self.token_actual().linea,
                    mensaje: "no se puede usar la palabra reservada `o` como nombre de variable".to_string(),
                });
            },
            _ => {
                return Err(ErrorQuetzal::ErrorSintaxis {
                    linea: self.token_actual().linea,
                    mensaje: "Se esperaba el nombre de la variable".to_string(),
                });
            }
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
            es_variable,
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
        let expresion = self.operador_ternario()?;
        
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
        
        // Verificar si es una asignación a propiedad (objeto.propiedad = valor)
        if let Nodo::AccesoMiembro { objeto, miembro, linea } = &expresion {
            if self.coincidir(&TipoToken::Asignacion) {
                let valor = Box::new(self.asignacion()?);
                
                return Ok(Nodo::AsignacionPropiedad {
                    objeto: objeto.clone(),
                    propiedad: miembro.clone(),
                    valor,
                    linea: *linea,
                });
            }
        }
        
        // Verificar si es una asignación de variable simple
        if let Nodo::Identificador(nombre) = &expresion {
            if self.coincidir(&TipoToken::Asignacion) {
                // Verificar si el nombre es una palabra reservada
                let palabras_reservadas = [
                    "vacio", "entero", "número", "texto", "log", "lista", "jsn",
                    "si", "sino", "para", "mientras", "hacer", "romper", "continuar",
                    "retornar", "objeto", "nuevo", "ambiente", "asincrono", "esperar",
                    "intentar", "capturar", "finalmente", "lanzar", "excepcion",
                    "importar", "exportar", "desde", "como", "privado", "publico",
                    "verdadero", "falso", "nulo", "y", "o", "en", "de", "es", "no", "var"
                ];
                
                if palabras_reservadas.contains(&nombre.as_str()) {
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea: self.token_anterior().linea,
                        mensaje: format!("no se puede asignar a la palabra reservada `{}`", nombre),
                    });
                }
                
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
                // Verificar si el nombre es una palabra reservada
                let palabras_reservadas = [
                    "vacio", "entero", "número", "texto", "log", "lista", "jsn",
                    "si", "sino", "para", "mientras", "hacer", "romper", "continuar",
                    "retornar", "objeto", "nuevo", "ambiente", "asincrono", "esperar",
                    "intentar", "capturar", "finalmente", "lanzar", "excepcion",
                    "importar", "exportar", "desde", "como", "privado", "publico",
                    "verdadero", "falso", "nulo", "y", "o", "en", "de", "es", "no", "var"
                ];
                
                if palabras_reservadas.contains(&nombre.as_str()) {
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea: self.token_anterior().linea,
                        mensaje: format!("no se puede asignar a la palabra reservada `{}`", nombre),
                    });
                }
                
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
        
        while self.coincidir(&TipoToken::OrLogico) || 
              (self.verificar(&TipoToken::O) && !self.es_asignacion_siguiente()) {
            if self.coincidir(&TipoToken::O) {
                // Ya verificamos que no es asignación, así que es operador
            }
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
        
        while self.coincidir(&TipoToken::AndLogico) || 
              (self.verificar(&TipoToken::Y) && !self.es_asignacion_siguiente()) {
            if self.coincidir(&TipoToken::Y) {
                // Ya verificamos que no es asignación, así que es operador
            }
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
                        TipoToken::TipoTexto => "texto".to_string(),
                        TipoToken::TipoEntero => "entero".to_string(),
                        TipoToken::TipoNumero => "numero".to_string(),
                        TipoToken::TipoLog => "log".to_string(),
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
        // Creación de objetos con 'nuevo'
        if self.coincidir(&TipoToken::Nuevo) {
            let linea = self.token_anterior().linea;
            
            // Nombre de la clase
            let nombre_clase = if let TipoToken::Identificador(nom) = &self.token_actual().tipo {
                let n = nom.clone();
                self.avanzar();
                n
            } else {
                return Err(ErrorQuetzal::ErrorSintaxis {
                    linea: self.token_actual().linea,
                    mensaje: "Se esperaba el nombre de la clase después de 'nuevo'".to_string(),
                });
            };
            
            // Paréntesis y argumentos del constructor
            if !self.coincidir(&TipoToken::ParentesisAbre) {
                return Err(ErrorQuetzal::ErrorSintaxis {
                    linea: self.token_actual().linea,
                    mensaje: "Se esperaba '(' después del nombre de la clase".to_string(),
                });
            }
            
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
                    mensaje: "Se esperaba ')' después de los argumentos del constructor".to_string(),
                });
            }
            
            return Ok(Nodo::CreacionObjeto {
                nombre_clase,
                argumentos,
                linea,
            });
        }
        
        // Literales booleanos
        if self.coincidir(&TipoToken::Verdadero) {
            return Ok(Nodo::Literal(Valor::Log(true)));
        }
        
        if self.coincidir(&TipoToken::Falso) {
            return Ok(Nodo::Literal(Valor::Log(false)));
        }
        
        // Literal nulo
        if self.coincidir(&TipoToken::Nulo) {
            return Ok(Nodo::Literal(Valor::Nulo));
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
                Ok(Nodo::Literal(Valor::Texto(valor)))
            },
            TipoToken::Identificador(nombre) => {
                let nom = nombre.clone();
                self.avanzar();
                Ok(Nodo::Identificador(nom))
            },
            // Manejar 'ambiente' como identificador especial
            TipoToken::Ambiente => {
                let nom = "ambiente".to_string();
                self.avanzar();
                Ok(Nodo::Identificador(nom))
            },
            // Tratar 'y' y 'o' como identificadores en contextos no operadores
            TipoToken::Y => {
                let nom = "y".to_string();
                self.avanzar();
                Ok(Nodo::Identificador(nom))
            },
            TipoToken::O => {
                let nom = "o".to_string();
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
                                    TipoToken::TipoTexto => "texto".to_string(),
                                    TipoToken::TipoLog => "log".to_string(),
                                    TipoToken::TipoLista => "lista".to_string(),
                                    TipoToken::TipoJson => "jsn".to_string(),
                                    _ => return Err(ErrorQuetzal::ErrorSintaxis {
                                        linea: self.token_actual().linea,
                                        mensaje: "Tipo de dato no válido como clave de propiedad".to_string(),
                                    }),
                                };
                                self.avanzar();
                                tipo_como_clave
                            } else if matches!(&self.token_actual().tipo, TipoToken::Verdadero | TipoToken::Falso | TipoToken::Nulo) {
                                // Permitir valores booleanos y nulo como claves
                                let valor_como_clave = match &self.token_actual().tipo {
                                    TipoToken::Verdadero => "verdadero".to_string(),
                                    TipoToken::Falso => "falso".to_string(),
                                    TipoToken::Nulo => "nulo".to_string(),
                                    _ => unreachable!(),
                                };
                                self.avanzar();
                                valor_como_clave
                            } else if matches!(&self.token_actual().tipo, 
                                TipoToken::Objeto | TipoToken::Nuevo | TipoToken::Retornar | 
                                TipoToken::Asincrono | TipoToken::Si | TipoToken::Sino |
                                TipoToken::Para | TipoToken::Mientras | TipoToken::Hacer |
                                TipoToken::Romper | TipoToken::Continuar | TipoToken::En | TipoToken::Cada) {
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
                                    TipoToken::Cada => "cada".to_string(),
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
                Ok(Valor::Texto(valor))
            },
            TipoToken::Verdadero => {
                self.avanzar();
                Ok(Valor::Log(true))
            },
            TipoToken::Falso => {
                self.avanzar();
                Ok(Valor::Log(false))
            },
            TipoToken::Nulo => {
                self.avanzar();
                Ok(Valor::Nulo)
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
            TipoToken::TipoTexto | TipoToken::TipoLog | TipoToken::TipoLista |
            TipoToken::TipoJson
        )
    }
    
    /// Verifica si el siguiente token es una asignación
    fn es_asignacion_siguiente(&self) -> bool {
        if self.posicion_actual + 1 < self.tokens.len() {
            match self.tokens[self.posicion_actual + 1].tipo {
                TipoToken::Asignacion | TipoToken::AsignacionSuma | TipoToken::AsignacionResta |
                TipoToken::AsignacionMult | TipoToken::AsignacionDiv | TipoToken::AsignacionMod => true,
                _ => false,
            }
        } else {
            false
        }
    }

    /// Verifica si el token actual es del tipo especificado
    fn verificar(&self, tipo: &TipoToken) -> bool {
        if self.esta_al_final() {
            return false;
        }
        std::mem::discriminant(&self.token_actual().tipo) == std::mem::discriminant(tipo)
    }
    
    /// Verifica si el siguiente token es del tipo especificado
    fn verificar_siguiente(&self, tipo: &TipoToken) -> bool {
        if self.posicion_actual + 1 >= self.tokens.len() {
            return false;
        }
        std::mem::discriminant(&self.tokens[self.posicion_actual + 1].tipo) == std::mem::discriminant(tipo)
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
            
            // Si hay identificador seguido de 'cada', es foreach
            if let TipoToken::Identificador(_) = &self.token_actual().tipo {
                self.avanzar();
                if self.verificar(&TipoToken::Cada) || self.verificar(&TipoToken::En) {
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
                        TipoToken::TipoTexto => "texto".to_string(),
                        TipoToken::TipoLog => "log".to_string(),
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
                
                // Esperar 'cada' o 'en' (mantener compatibilidad)
                if !self.coincidir(&TipoToken::Cada) && !self.coincidir(&TipoToken::En) {
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea: self.token_actual().linea,
                        mensaje: "Se esperaba 'cada' en bucle foreach".to_string(),
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
            
            // Esperar 'cada' o 'en' (mantener compatibilidad) 
            if !self.coincidir(&TipoToken::Cada) && !self.coincidir(&TipoToken::En) {
                return Err(ErrorQuetzal::ErrorSintaxis {
                    linea: self.token_actual().linea,
                    mensaje: "Se esperaba 'cada' en bucle para".to_string(),
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
        
        // Verificar si estamos dentro de un constructor
        if self.dentro_de_constructor {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea,
                mensaje: "Los constructores no pueden usar la instrucción 'retornar'".to_string(),
            });
        }
        
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
            TipoToken::TipoTexto => { self.avanzar(); "texto".to_string() },
            TipoToken::TipoLog => { self.avanzar(); "log".to_string() },
            TipoToken::TipoLista => { self.avanzar(); "lista".to_string() },
            TipoToken::TipoJson => { self.avanzar(); "jsn".to_string() },
            _ => return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.token_actual().linea,
                mensaje: "Se esperaba un tipo de dato".to_string(),
            }),
        };
        
        // En bucles, las variables son variables por defecto
        let es_variable = true;
        // Consumir 'var' opcional si está presente
        self.coincidir(&TipoToken::Var);
        
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
            es_variable,
            valor,
            linea,
        })
    }

    /// Analiza un bloque intentar-capturar-finalmente
    fn bloque_intentar(&mut self) -> ResultadoQuetzal<Nodo> {
        let linea = self.token_anterior().linea;
        
        // Parsear el bloque intentar
        let bloque_intentar = Box::new(self.bloque_o_declaracion()?);
        
        // Parsear bloques capturar
        let mut bloques_capturar = Vec::new();
        
        while self.coincidir(&TipoToken::Capturar) {
            // Parsear (TipoExcepcion nombre_variable)
            if !self.coincidir(&TipoToken::ParentesisAbre) {
                return Err(ErrorQuetzal::ErrorSintaxis {
                    linea: self.token_actual().linea,
                    mensaje: "Se esperaba '(' después de 'capturar'".to_string(),
                });
            }
            
            // Tipo de excepción
            let tipo_excepcion = if let TipoToken::Identificador(tipo) = &self.token_actual().tipo {
                let t = tipo.clone();
                self.avanzar();
                t
            } else {
                return Err(ErrorQuetzal::ErrorSintaxis {
                    linea: self.token_actual().linea,
                    mensaje: "Se esperaba el tipo de excepción".to_string(),
                });
            };
            
            // Nombre de la variable
            let nombre_variable = if let TipoToken::Identificador(nombre) = &self.token_actual().tipo {
                let n = nombre.clone();
                self.avanzar();
                n
            } else {
                return Err(ErrorQuetzal::ErrorSintaxis {
                    linea: self.token_actual().linea,
                    mensaje: "Se esperaba el nombre de la variable de excepción".to_string(),
                });
            };
            
            if !self.coincidir(&TipoToken::ParentesisCierra) {
                return Err(ErrorQuetzal::ErrorSintaxis {
                    linea: self.token_actual().linea,
                    mensaje: "Se esperaba ')' después de la declaración de excepción".to_string(),
                });
            }
            
            // Bloque de manejo
            let bloque = self.bloque_o_declaracion()?;
            
            bloques_capturar.push(BloqueCapturar {
                tipo_excepcion,
                nombre_variable,
                bloque,
            });
        }
        
        // Verificar que haya al menos un bloque capturar
        if bloques_capturar.is_empty() {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea,
                mensaje: "Se esperaba al menos un bloque 'capturar' después de 'intentar'".to_string(),
            });
        }
        
        // Parsear bloque finalmente (opcional)
        let bloque_finalmente = if self.coincidir(&TipoToken::Finalmente) {
            Some(Box::new(self.bloque_o_declaracion()?))
        } else {
            None
        };
        
        Ok(Nodo::BloqueIntentar {
            bloque_intentar,
            bloques_capturar,
            bloque_finalmente,
            linea,
        })
    }
    
    /// Analiza una declaración de lanzar excepción
    fn declaracion_lanzar(&mut self) -> ResultadoQuetzal<Nodo> {
        let linea = self.token_anterior().linea;
        
        // Parsear la expresión de la excepción
        let excepcion = Box::new(self.expresion()?);
        
        Ok(Nodo::Lanzar {
            excepcion,
            linea,
        })
    }
}
