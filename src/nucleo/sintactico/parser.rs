use crate::errores::{Error, CodigoError, Resultado};
use crate::nucleo::lexico::Lexer;
use crate::nucleo::lexico::token::TokenConPosicion;
use crate::nucleo::sintactico::ast::*;

/// Parser para el lenguaje Quetzal
pub struct Parser {
    tokens: Vec<TokenConPosicion>,
    posicion_actual: usize,
}

impl Parser {
    /// Crea un nuevo parser a partir de tokens
    pub fn nuevo(tokens: Vec<TokenConPosicion>) -> Self {
        Self {
            tokens,
            posicion_actual: 0,
        }
    }
    
    /// Parsea el código fuente y genera el AST
    pub fn parsear(fuente: &str) -> Resultado<Vec<NodoAst>> {
        // Primero tokenizar
        let mut lexer = Lexer::nuevo(fuente)?;
        let tokens = lexer.tokenizar()?;
        
        // Luego parsear
        let mut parser = Self::nuevo(tokens);
        parser.parsear_archivo()
    }
    
    /// Parsea un archivo completo
    fn parsear_archivo(&mut self) -> Resultado<Vec<NodoAst>> {
        let mut declaraciones = Vec::new();
        
        while !self.es_fin() {
            // Saltar comentarios y nuevas líneas
            self.saltar_comentarios_y_espacios();
            
            if self.es_fin() {
                break;
            }
            
            if self.es_importacion() {
                declaraciones.push(self.parsear_importacion()?);
            } else {
                declaraciones.push(self.parsear_declaracion()?);
            }
            
            // Saltar comentarios y espacios después de cada declaración
            self.saltar_comentarios_y_espacios();
        }
        
        Ok(declaraciones)
    }
    
    /// Salta comentarios, nuevas líneas y espacios en blanco
    fn saltar_comentarios_y_espacios(&mut self) {
        while let Some(t) = self.token_actual() {
            match t.token {
                crate::nucleo::lexico::token::Token::ComentarioLinea(_)
                | crate::nucleo::lexico::token::Token::ComentarioBloque(_)
                | crate::nucleo::lexico::token::Token::NuevaLinea
                | crate::nucleo::lexico::token::Token::FinDeArchivo => {
                    self.avanzar();
                }
                _ => break,
            }
        }
    }
    
    /// Verifica si estamos al final
    fn es_fin(&self) -> bool {
        self.posicion_actual >= self.tokens.len()
    }
    
    /// Obtiene el token actual
    fn token_actual(&self) -> Option<&TokenConPosicion> {
        self.tokens.get(self.posicion_actual)
    }
    
    /// Avanza al siguiente token
    fn avanzar(&mut self) {
        if !self.es_fin() {
            self.posicion_actual += 1;
        }
    }
    
    /// Verifica si el siguiente token es una importación
    fn es_importacion(&self) -> bool {
        matches!(self.token_actual(), Some(t) if matches!(t.token, crate::nucleo::lexico::token::Token::Importar))
    }
    
    /// Parsea una importación
    fn parsear_importacion(&mut self) -> Resultado<NodoAst> {
        let posicion = self.token_actual().unwrap().posicion;
        
        // importar
        self.avanzar();
        
        // {
        self.expectar_token(crate::nucleo::lexico::token::Token::LlaveIzq)?;
        self.avanzar();
        
        // elementos
        let mut elementos = Vec::new();
        loop {
            if let Some(t) = self.token_actual() {
                if let crate::nucleo::lexico::token::Token::Identificador(ref nombre) = t.token {
                    elementos.push(nombre.clone());
                    self.avanzar();
                    
                    if let Some(t) = self.token_actual() {
                        if matches!(t.token, crate::nucleo::lexico::token::Token::Coma) {
                            self.avanzar();
                            continue;
                        }
                    }
                }
            }
            break;
        }
        
        // }
        self.expectar_token(crate::nucleo::lexico::token::Token::LlaveDer)?;
        self.avanzar();
        
        // desde
        self.expectar_token(crate::nucleo::lexico::token::Token::Desde)?;
        self.avanzar();
        
        // ruta (cadena)
        let ruta = if let Some(t) = self.token_actual() {
            if let crate::nucleo::lexico::token::Token::LiteralTexto(ref texto) = t.token {
                let ruta = texto.clone();
                self.avanzar();
                ruta
            } else {
                return Err(Error::analisis(
                    CodigoError::SintaxisGeneral,
                    "se esperaba una cadena de texto para la ruta del módulo",
                    None,
                    Some(posicion.linea),
                    Some(posicion.columna),
                ));
            }
        } else {
            return Err(Error::analisis(
                CodigoError::SintaxisGeneral,
                "se esperaba una cadena de texto para la ruta del módulo",
                None,
                Some(posicion.linea),
                Some(posicion.columna),
            ));
        };
        
        Ok(NodoAst::Importacion {
            elementos,
            ruta,
            posicion,
        })
    }
    
    /// Parsea una declaración
    fn parsear_declaracion(&mut self) -> Resultado<NodoAst> {
        // Intentar detectar si es una declaración de variable o función
        let es_asincrono = if let Some(t) = self.token_actual() {
            matches!(t.token, crate::nucleo::lexico::token::Token::Asincrono)
        } else {
            false
        };
        
        if es_asincrono {
            self.avanzar();
            self.saltar_comentarios_y_espacios();
        }
        
        if let Some(t) = self.token_actual() {
            // Si es un identificador que empieza con mayúscula (posible tipo de objeto),
            // verificar si es realmente una declaración (seguido de identificador o var)
            // o si es una expresión (seguido de .)
            if self.es_tipo_objeto(&t.token) {
                // Verificar el siguiente token sin consumir el actual
                if let Some(siguiente) = self.tokens.get(self.posicion_actual + 1) {
                    if matches!(siguiente.token, crate::nucleo::lexico::token::Token::Punto) {
                        // Es una expresión (ej: UsuarioLibre.metodo())
                        return self.parsear_expresion_como_declaracion();
                    }
                }
            }
            
            if self.es_tipo(&t.token) {
                // Guardar posición para poder retroceder si es necesario
                let posicion_backup = self.posicion_actual;
                
                // Parsear tipo
                let tipo = self.parsear_tipo()?;
                
                // Verificar si hay 'var' (solo para variables)
                let _mutable = if let Some(t) = self.token_actual() {
                    if matches!(t.token, crate::nucleo::lexico::token::Token::Var) {
                        self.avanzar();
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                
                // Parsear identificador
                let nombre = if let Some(t) = self.token_actual() {
                    if let crate::nucleo::lexico::token::Token::Identificador(ref nombre) = t.token {
                        let nombre = nombre.clone();
                        self.avanzar();
                        nombre
                    } else {
                        // Retroceder y parsear como variable
                        self.posicion_actual = posicion_backup;
                        return self.parsear_declaracion_variable();
                    }
                } else {
                    // Retroceder y parsear como variable
                    self.posicion_actual = posicion_backup;
                    return self.parsear_declaracion_variable();
                };
                
                // Verificar si es función (tiene '(') o variable (tiene '=')
                if let Some(t) = self.token_actual() {
                    match t.token {
                        crate::nucleo::lexico::token::Token::ParentesisIzq => {
                            // Es una función
                            return self.parsear_declaracion_funcion_desde_tipo(tipo, nombre, es_asincrono);
                        }
                        crate::nucleo::lexico::token::Token::Asignar => {
                            // Es una variable, retroceder y parsear normalmente
                            self.posicion_actual = posicion_backup;
                            return self.parsear_declaracion_variable();
                        }
                        _ => {
                            // Retroceder y parsear como variable
                            self.posicion_actual = posicion_backup;
                            return self.parsear_declaracion_variable();
                        }
                    }
                } else {
                    // Retroceder y parsear como variable
                    self.posicion_actual = posicion_backup;
                    return self.parsear_declaracion_variable();
                }
            }
            
            // Verificar si es una estructura de control de flujo o declaración especial
            match t.token {
                crate::nucleo::lexico::token::Token::Objeto => {
                    return self.parsear_declaracion_objeto();
                }
                crate::nucleo::lexico::token::Token::Mientras => {
                    return self.parsear_mientras();
                }
                crate::nucleo::lexico::token::Token::Para => {
                    return self.parsear_para();
                }
                crate::nucleo::lexico::token::Token::Si => {
                    return self.parsear_si();
                }
                crate::nucleo::lexico::token::Token::Hacer => {
                    return self.parsear_hacer_mientras();
                }
                crate::nucleo::lexico::token::Token::Romper => {
                    return self.parsear_romper();
                }
                crate::nucleo::lexico::token::Token::Continuar => {
                    return self.parsear_continuar();
                }
                crate::nucleo::lexico::token::Token::Retornar => {
                    return self.parsear_retornar();
                }
                crate::nucleo::lexico::token::Token::Intentar => {
                    return self.parsear_intentar();
                }
                crate::nucleo::lexico::token::Token::Lanzar => {
                    return self.parsear_lanzar();
                }
                _ => {}
            }
        }
        
        // Si no es una declaración ni estructura de control, parsear como expresión
        self.parsear_expresion()
    }
    
    /// Verifica si un token es un tipo
    fn es_tipo(&self, token: &crate::nucleo::lexico::token::Token) -> bool {
        matches!(
            token,
            crate::nucleo::lexico::token::Token::Vacio
                | crate::nucleo::lexico::token::Token::VacioAcento
                | crate::nucleo::lexico::token::Token::Entero
                | crate::nucleo::lexico::token::Token::Numero
                | crate::nucleo::lexico::token::Token::NumeroAcento
                | crate::nucleo::lexico::token::Token::Texto
                | crate::nucleo::lexico::token::Token::Log
                | crate::nucleo::lexico::token::Token::LogAcento
                | crate::nucleo::lexico::token::Token::Lista
                | crate::nucleo::lexico::token::Token::Jsn
        ) || self.es_tipo_objeto(token)
    }
    
    /// Verifica si un token es un identificador que podria ser un tipo de objeto
    fn es_tipo_objeto(&self, token: &crate::nucleo::lexico::token::Token) -> bool {
        if let crate::nucleo::lexico::token::Token::Identificador(nombre) = token {
            // Un tipo de objeto empieza con mayúscula
            nombre.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
        } else {
            false
        }
    }
    
    /// Parsea un tipo
    fn parsear_tipo(&mut self) -> Resultado<TipoAst> {
        if let Some(t) = self.token_actual() {
            let tipo = match t.token {
                crate::nucleo::lexico::token::Token::Vacio | crate::nucleo::lexico::token::Token::VacioAcento => TipoAst::Vacio,
                crate::nucleo::lexico::token::Token::Entero => TipoAst::Entero,
                crate::nucleo::lexico::token::Token::Numero | crate::nucleo::lexico::token::Token::NumeroAcento => TipoAst::Numero,
                crate::nucleo::lexico::token::Token::Texto => TipoAst::Texto,
                crate::nucleo::lexico::token::Token::Log | crate::nucleo::lexico::token::Token::LogAcento => TipoAst::Logico,
                crate::nucleo::lexico::token::Token::Lista => {
                    self.avanzar();
                    // Verificar si hay tipo genérico
                    if let Some(t) = self.token_actual() {
                        if matches!(t.token, crate::nucleo::lexico::token::Token::Menor) {
                            self.avanzar();
                            let tipo_interno = self.parsear_tipo()?;
                            self.expectar_token(crate::nucleo::lexico::token::Token::Mayor)?;
                            self.avanzar();
                            return Ok(TipoAst::Lista(Box::new(tipo_interno)));
                        }
                    }
                    return Ok(TipoAst::Lista(Box::new(TipoAst::Vacio)));
                }
                crate::nucleo::lexico::token::Token::Jsn => TipoAst::Json,
                crate::nucleo::lexico::token::Token::Identificador(ref nombre) => {
                    // Verificar si es un tipo de objeto (empieza con mayúscula)
                    if nombre.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                        TipoAst::Objeto(nombre.clone())
                    } else {
                        return Err(Error::analisis(
                            CodigoError::SintaxisGeneral,
                            format!("'{}' no es un tipo válido (los tipos de objeto deben empezar con mayúscula)", nombre),
                            None,
                            Some(t.posicion.linea),
                            Some(t.posicion.columna),
                        ));
                    }
                }
                _ => {
                    return Err(Error::analisis(
                        CodigoError::SintaxisGeneral,
                        "se esperaba un tipo",
                        None,
                        Some(t.posicion.linea),
                        Some(t.posicion.columna),
                    ));
                }
            };
            
            self.avanzar();
            Ok(tipo)
        } else {
            Err(Error::analisis(
                CodigoError::SintaxisGeneral,
                "se esperaba un tipo pero se alcanzó el final del archivo",
                None,
                None,
                None,
            ))
        }
    }
    
    /// Parsea una declaración de variable: tipo [var] identificador = expresion
    fn parsear_declaracion_variable(&mut self) -> Resultado<NodoAst> {
        let posicion_inicial = self.token_actual()
            .map(|t| t.posicion)
            .unwrap_or_else(|| crate::nucleo::lexico::token::Posicion::inicial());
        
        let tipo = self.parsear_tipo()?;
        
        // Verificar si hay 'var' (opcional)
        let mutable = if let Some(t) = self.token_actual() {
            if matches!(t.token, crate::nucleo::lexico::token::Token::Var) {
                self.avanzar();
                true
            } else {
                false
            }
        } else {
            false
        };
        
        // Parsear identificador
        let nombre = if let Some(t) = self.token_actual() {
            if let crate::nucleo::lexico::token::Token::Identificador(ref nombre) = t.token {
                let nombre = nombre.clone();
                self.avanzar();
                nombre
            } else {
                return Err(Error::analisis(
                    CodigoError::SintaxisGeneral,
                    "se esperaba un identificador",
                    None,
                    Some(t.posicion.linea),
                    Some(t.posicion.columna),
                ));
            }
        } else {
            return Err(Error::analisis(
                CodigoError::SintaxisGeneral,
                "se esperaba un identificador pero se alcanzó el final del archivo",
                None,
                None,
                None,
            ));
        };
        
        // Verificar '='
        self.expectar_token(crate::nucleo::lexico::token::Token::Asignar)?;
        self.avanzar();
        
        // Parsear expresión
        let valor = self.parsear_expresion()?;
        
        Ok(NodoAst::DeclaracionVariable {
            tipo,
            mutable,
            nombre,
            valor: Box::new(valor),
            posicion: posicion_inicial,
        })
    }
    
    /// Parsea una declaración de función cuando ya tenemos el tipo y nombre
    /// Formato: [asincrono] tipo nombre (parametros) { cuerpo }
    fn parsear_declaracion_funcion_desde_tipo(&mut self, tipo_retorno: TipoAst, nombre: String, asincrono: bool) -> Resultado<NodoAst> {
        let posicion_inicial = self.token_actual()
            .map(|t| t.posicion)
            .unwrap_or_else(|| crate::nucleo::lexico::token::Posicion::inicial());
        
        // Verificar '('
        self.expectar_token(crate::nucleo::lexico::token::Token::ParentesisIzq)?;
        self.avanzar();
        
        let mut parametros = Vec::new();
        
        // Parsear parámetros si los hay
        if let Some(t) = self.token_actual() {
            if !matches!(t.token, crate::nucleo::lexico::token::Token::ParentesisDer) {
                loop {
                    // Parsear tipo del parámetro
                    let tipo_param = self.parsear_tipo()?;
                    
                    // Verificar si hay 'var' (opcional)
                    let mutable_param = if let Some(t) = self.token_actual() {
                        if matches!(t.token, crate::nucleo::lexico::token::Token::Var) {
                            self.avanzar();
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    
                    // Parsear nombre del parámetro
                    let nombre_param = if let Some(t) = self.token_actual() {
                        if let crate::nucleo::lexico::token::Token::Identificador(ref nombre) = t.token {
                            let nombre = nombre.clone();
                            self.avanzar();
                            nombre
                        } else {
                            return Err(Error::analisis(
                                CodigoError::SintaxisGeneral,
                                "se esperaba un identificador para el parámetro",
                                None,
                                Some(t.posicion.linea),
                                Some(t.posicion.columna),
                            ));
                        }
                    } else {
                        return Err(Error::analisis(
                            CodigoError::SintaxisGeneral,
                            "se esperaba un identificador para el parámetro",
                            None,
                            None,
                            None,
                        ));
                    };
                    
                    let posicion_param = self.token_actual()
                        .map(|t| t.posicion)
                        .unwrap_or_else(|| crate::nucleo::lexico::token::Posicion::inicial());
                    
                    parametros.push(ParametroAst {
                        tipo: tipo_param,
                        mutable: mutable_param,
                        nombre: nombre_param,
                        posicion: posicion_param,
                    });
                    
                    // Verificar si hay más parámetros
                    if let Some(t) = self.token_actual() {
                        if matches!(t.token, crate::nucleo::lexico::token::Token::Coma) {
                            self.avanzar();
                        } else if matches!(t.token, crate::nucleo::lexico::token::Token::ParentesisDer) {
                            break;
                        } else {
                            return Err(Error::analisis(
                                CodigoError::SintaxisGeneral,
                                "se esperaba ',' o ')' después de un parámetro",
                                None,
                                Some(t.posicion.linea),
                                Some(t.posicion.columna),
                            ));
                        }
                    } else {
                        return Err(Error::analisis(
                            CodigoError::SintaxisGeneral,
                            "se esperaba ')' después de los parámetros",
                            None,
                            None,
                            None,
                        ));
                    }
                }
            }
        }
        
        // Verificar ')'
        self.expectar_token(crate::nucleo::lexico::token::Token::ParentesisDer)?;
        self.avanzar();
        
        // Parsear cuerpo (bloque)
        let cuerpo = self.parsear_bloque()?;
        
        Ok(NodoAst::DeclaracionFuncion {
            asincrono,
            tipo_retorno,
            nombre,
            parametros,
            cuerpo: Box::new(cuerpo),
            posicion: posicion_inicial,
        })
    }
    
    /// Parsea una declaración de objeto: objeto nombre { miembros }
    fn parsear_declaracion_objeto(&mut self) -> Resultado<NodoAst> {
        let posicion_inicial = self.token_actual()
            .map(|t| t.posicion)
            .unwrap_or_else(|| crate::nucleo::lexico::token::Posicion::inicial());
        
        // Verificar 'objeto'
        self.expectar_token(crate::nucleo::lexico::token::Token::Objeto)?;
        self.avanzar();
        
        // Parsear nombre del objeto
        let nombre = if let Some(t) = self.token_actual() {
            if let crate::nucleo::lexico::token::Token::Identificador(ref nombre) = t.token {
                let nombre = nombre.clone();
                self.avanzar();
                nombre
            } else {
                return Err(Error::analisis(
                    CodigoError::SintaxisGeneral,
                    "se esperaba un identificador para el nombre del objeto",
                    None,
                    Some(t.posicion.linea),
                    Some(t.posicion.columna),
                ));
            }
        } else {
            return Err(Error::analisis(
                CodigoError::SintaxisGeneral,
                "se esperaba un identificador para el nombre del objeto",
                None,
                None,
                None,
            ));
        };
        
        // Verificar '{'
        self.expectar_token(crate::nucleo::lexico::token::Token::LlaveIzq)?;
        self.avanzar();
        
        let mut miembros = Vec::new();
        let mut modificador_actual = ModificadorAcceso::Privado; // Por defecto privado
        
        // Parsear miembros hasta encontrar '}'
        while let Some(t) = self.token_actual() {
            if matches!(t.token, crate::nucleo::lexico::token::Token::LlaveDer) {
                break;
            }
            
            self.saltar_comentarios_y_espacios();
            
            if let Some(t) = self.token_actual() {
                if matches!(t.token, crate::nucleo::lexico::token::Token::LlaveDer) {
                    break;
                }
                
                // Verificar si es un modificador de acceso
                match t.token {
                    crate::nucleo::lexico::token::Token::Publico | crate::nucleo::lexico::token::Token::PublicoAcento => {
                        self.avanzar();
                        // Verificar ':'
                        self.expectar_token(crate::nucleo::lexico::token::Token::DosPuntos)?;
                        self.avanzar();
                        modificador_actual = ModificadorAcceso::Publico;
                        continue;
                    }
                    crate::nucleo::lexico::token::Token::Privado => {
                        self.avanzar();
                        // Verificar ':'
                        self.expectar_token(crate::nucleo::lexico::token::Token::DosPuntos)?;
                        self.avanzar();
                        modificador_actual = ModificadorAcceso::Privado;
                        continue;
                    }
                    _ => {}
                }
                
                // Verificar si es 'libre' (opcional)
                let libre = if let Some(t) = self.token_actual() {
                    if matches!(t.token, crate::nucleo::lexico::token::Token::Libre) {
                        self.avanzar();
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                
                // Parsear declaración (variable o función)
                let posicion_miembro = self.token_actual()
                    .map(|t| t.posicion)
                    .unwrap_or_else(|| crate::nucleo::lexico::token::Posicion::inicial());
                
                // Verificar si es una declaración de campo (tipo identificador sin =) o declaración normal
                let declaracion = if let Some(t) = self.token_actual() {
                    if self.es_tipo(&t.token) {
                        // Guardar posición para poder retroceder
                        let pos_backup = self.posicion_actual;
                        
                        let tipo = self.parsear_tipo()?;
                        
                        // Verificar si es un constructor: TipoObjeto(parametros) { cuerpo }
                        // Esto ocurre cuando el tipo es un objeto y es seguido por '('
                        if let TipoAst::Objeto(nombre_tipo) = &tipo {
                            if let Some(t) = self.token_actual() {
                                if matches!(t.token, crate::nucleo::lexico::token::Token::ParentesisIzq) {
                                    // Es un constructor
                                    let tipo_retorno = TipoAst::Vacio;
                                    let constructor = self.parsear_declaracion_funcion_desde_tipo(tipo_retorno, nombre_tipo.clone(), false)?;
                                    
                                    miembros.push(MiembroObjetoAst {
                                        modificador_acceso: modificador_actual,
                                        libre,
                                        declaracion: Box::new(constructor),
                                        posicion: posicion_miembro,
                                    });
                                    
                                    self.saltar_comentarios_y_espacios();
                                    continue;
                                }
                            }
                        }
                        
                        // Verificar si hay 'var' (opcional)
                        let mutable = if let Some(t) = self.token_actual() {
                            if matches!(t.token, crate::nucleo::lexico::token::Token::Var) {
                                self.avanzar();
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        };
                        
                        // Parsear identificador
                        let nombre_campo = if let Some(t) = self.token_actual() {
                            if let crate::nucleo::lexico::token::Token::Identificador(ref nombre) = t.token {
                                let nombre = nombre.clone();
                                self.avanzar();
                                Some(nombre)
                            } else {
                                // Retroceder y parsear normalmente
                                self.posicion_actual = pos_backup;
                                return Ok(self.parsear_declaracion()?);
                            }
                        } else {
                            // Retroceder y parsear normalmente
                            self.posicion_actual = pos_backup;
                            return Ok(self.parsear_declaracion()?);
                        };
                        
                        // Verificar si es campo (no tiene '=' ni '(') o declaración normal
                        if let Some(nombre_campo) = nombre_campo {
                            if let Some(t) = self.token_actual() {
                                match t.token {
                                    crate::nucleo::lexico::token::Token::Asignar => {
                                        // Es una variable con valor, retroceder y parsear normalmente
                                        self.posicion_actual = pos_backup;
                                        self.parsear_declaracion()?
                                    }
                                    crate::nucleo::lexico::token::Token::ParentesisIzq => {
                                        // Es una función, parsear directamente desde aquí
                                        // (no retroceder para no perder el contexto de 'libre')
                                        self.parsear_declaracion_funcion_desde_tipo(tipo, nombre_campo, false)?
                                    }
                                    _ => {
                                        // Es un campo de objeto (solo tipo e identificador)
                                        // Crear una declaración de variable con valor por defecto
                                        NodoAst::DeclaracionVariable {
                                            tipo,
                                            mutable,
                                            nombre: nombre_campo,
                                            valor: Box::new(NodoAst::ExpresionLiteral {
                                                valor: LiteralAst::Nulo,
                                                posicion: posicion_miembro,
                                            }),
                                            posicion: posicion_miembro,
                                        }
                                    }
                                }
                            } else {
                                // Es un campo de objeto
                                NodoAst::DeclaracionVariable {
                                    tipo,
                                    mutable,
                                    nombre: nombre_campo,
                                    valor: Box::new(NodoAst::ExpresionLiteral {
                                        valor: LiteralAst::Nulo,
                                        posicion: posicion_miembro,
                                    }),
                                    posicion: posicion_miembro,
                                }
                            }
                        } else {
                            // Retroceder y parsear normalmente
                            self.posicion_actual = pos_backup;
                            self.parsear_declaracion()?
                        }
                    } else {
                        // Verificar si es un constructor (identificador seguido de '(')
                        // Primero verificar si hay un identificador
                        let nombre_constructor_opt = if let Some(t) = self.token_actual() {
                            if let crate::nucleo::lexico::token::Token::Identificador(ref nombre) = t.token {
                                // Clonar el nombre antes de avanzar
                                let nombre_clone = nombre.clone();
                                
                                // Verificar si el siguiente token es '('
                                let pos_backup = self.posicion_actual;
                                self.avanzar();
                                
                                let siguiente_es_parentesis = if let Some(t2) = self.token_actual() {
                                    matches!(t2.token, crate::nucleo::lexico::token::Token::ParentesisIzq)
                                } else {
                                    false
                                };
                                
                                // Restaurar posición
                                self.posicion_actual = pos_backup;
                                
                                if siguiente_es_parentesis {
                                    Some(nombre_clone)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        
                        if let Some(nombre_constructor) = nombre_constructor_opt {
                            // Es un constructor, parsear como función sin tipo de retorno (vacio)
                            // El parser ya está en el identificador, necesitamos avanzar para llegar al '('
                            // Pero parsear_declaracion_funcion_desde_tipo espera estar en el identificador
                            // así que necesitamos crear una función especial o modificar el flujo
                            // Por ahora, simplemente avanzar al '(' y parsear
                            self.avanzar(); // Avanzar desde el identificador al '('
                            self.parsear_declaracion_funcion_desde_tipo(TipoAst::Vacio, nombre_constructor, false)?
                        } else {
                            // No es constructor, parsear normalmente
                            self.parsear_declaracion()?
                        }
                    }
                } else {
                    return Err(Error::analisis(
                        CodigoError::SintaxisGeneral,
                        "se esperaba un miembro del objeto",
                        None,
                        None,
                        None,
                    ));
                };
                
                miembros.push(MiembroObjetoAst {
                    modificador_acceso: modificador_actual,
                    libre,
                    declaracion: Box::new(declaracion),
                    posicion: posicion_miembro,
                });
                
                self.saltar_comentarios_y_espacios();
            } else {
                break;
            }
        }
        
        // Verificar '}'
        self.expectar_token(crate::nucleo::lexico::token::Token::LlaveDer)?;
        self.avanzar();
        
        Ok(NodoAst::DeclaracionObjeto {
            nombre,
            miembros,
            posicion: posicion_inicial,
        })
    }
    
    /// Parsea una expresión como declaración (para expresiones sueltas como llamadas a métodos)
    fn parsear_expresion_como_declaracion(&mut self) -> Resultado<NodoAst> {
        self.parsear_expresion()
    }
    
    /// Parsea una expresión
    fn parsear_expresion(&mut self) -> Resultado<NodoAst> {
        // Las asignaciones tienen la menor precedencia
        self.parsear_expresion_asignacion()
    }
    
    /// Parsea una expresión de asignación: objetivo = valor
    fn parsear_expresion_asignacion(&mut self) -> Resultado<NodoAst> {
        let izquierda = self.parsear_expresion_ternario()?;
        
        if let Some(t) = self.token_actual() {
            let posicion = t.posicion;
            
            // Asignación simple
            if matches!(t.token, crate::nucleo::lexico::token::Token::Asignar) {
                self.avanzar();
                let derecha = self.parsear_expresion_asignacion()?;
                
                return Ok(NodoAst::ExpresionAsignar {
                    objetivo: Box::new(izquierda),
                    valor: Box::new(derecha),
                    posicion,
                });
            }
            
            // Operadores de asignación compuesta: +=, -=, *=, /=, %=
            let operador_compuesto = match t.token {
                crate::nucleo::lexico::token::Token::MasAsignar => Some(OperadorBinario::Suma),
                crate::nucleo::lexico::token::Token::MenosAsignar => Some(OperadorBinario::Resta),
                crate::nucleo::lexico::token::Token::MultiplicarAsignar => Some(OperadorBinario::Multiplicacion),
                crate::nucleo::lexico::token::Token::DividirAsignar => Some(OperadorBinario::Division),
                crate::nucleo::lexico::token::Token::ModuloAsignar => Some(OperadorBinario::Modulo),
                _ => None,
            };
            
            if let Some(operador) = operador_compuesto {
                self.avanzar();
                let derecha = self.parsear_expresion_asignacion()?;
                
                // Convertir a += b en a = a + b
                let expresion_operacion = NodoAst::ExpresionBinaria {
                    operador,
                    izquierda: Box::new(izquierda.clone()),
                    derecha: Box::new(derecha),
                    posicion,
                };
                
                return Ok(NodoAst::ExpresionAsignar {
                    objetivo: Box::new(izquierda),
                    valor: Box::new(expresion_operacion),
                    posicion,
                });
            }
        }
        
        Ok(izquierda)
    }
    
    /// Parsea una expresión ternaria
    fn parsear_expresion_ternario(&mut self) -> Resultado<NodoAst> {
        let condicion = self.parsear_expresion_logica_or()?;
        
        if let Some(t) = self.token_actual() {
            if matches!(t.token, crate::nucleo::lexico::token::Token::Ternario) {
                let posicion = t.posicion;
                self.avanzar();
                
                let verdadero = self.parsear_expresion()?;
                
                self.expectar_token(crate::nucleo::lexico::token::Token::DosPuntos)?;
                self.avanzar();
                
                let falso = self.parsear_expresion()?;
                
                return Ok(NodoAst::ExpresionTernario {
                    condicion: Box::new(condicion),
                    verdadero: Box::new(verdadero),
                    falso: Box::new(falso),
                    posicion,
                });
            }
        }
        
        Ok(condicion)
    }
    
    /// Parsea una expresión lógica OR
    fn parsear_expresion_logica_or(&mut self) -> Resultado<NodoAst> {
        let mut izquierda = self.parsear_expresion_logica_and()?;
        
        while let Some(t) = self.token_actual() {
            if matches!(t.token, crate::nucleo::lexico::token::Token::O | crate::nucleo::lexico::token::Token::OConAcento) {
                let posicion = t.posicion;
                self.avanzar();
                let derecha = self.parsear_expresion_logica_and()?;
                izquierda = NodoAst::ExpresionBinaria {
                    operador: OperadorBinario::O,
                    izquierda: Box::new(izquierda),
                    derecha: Box::new(derecha),
                    posicion,
                };
            } else {
                break;
            }
        }
        
        Ok(izquierda)
    }
    
    /// Parsea una expresión lógica AND
    fn parsear_expresion_logica_and(&mut self) -> Resultado<NodoAst> {
        let mut izquierda = self.parsear_expresion_comparacion()?;
        
        while let Some(t) = self.token_actual() {
            if matches!(t.token, crate::nucleo::lexico::token::Token::Y) {
                let posicion = t.posicion;
                self.avanzar();
                let derecha = self.parsear_expresion_comparacion()?;
                izquierda = NodoAst::ExpresionBinaria {
                    operador: OperadorBinario::Y,
                    izquierda: Box::new(izquierda),
                    derecha: Box::new(derecha),
                    posicion,
                };
            } else {
                break;
            }
        }
        
        Ok(izquierda)
    }
    
    /// Parsea una expresión de comparación
    fn parsear_expresion_comparacion(&mut self) -> Resultado<NodoAst> {
        let izquierda = self.parsear_expresion_aditiva()?;
        
        if let Some(t) = self.token_actual() {
            let operador = match t.token {
                crate::nucleo::lexico::token::Token::Igual => Some(OperadorBinario::Igual),
                crate::nucleo::lexico::token::Token::Diferente => Some(OperadorBinario::Diferente),
                crate::nucleo::lexico::token::Token::Mayor => Some(OperadorBinario::Mayor),
                crate::nucleo::lexico::token::Token::Menor => Some(OperadorBinario::Menor),
                crate::nucleo::lexico::token::Token::MayorIgual => Some(OperadorBinario::MayorIgual),
                crate::nucleo::lexico::token::Token::MenorIgual => Some(OperadorBinario::MenorIgual),
                _ => None,
            };
            
            if let Some(op) = operador {
                let posicion = t.posicion;
                self.avanzar();
                let derecha = self.parsear_expresion_aditiva()?;
                return Ok(NodoAst::ExpresionBinaria {
                    operador: op,
                    izquierda: Box::new(izquierda),
                    derecha: Box::new(derecha),
                    posicion,
                });
            }
        }
        
        Ok(izquierda)
    }
    
    /// Parsea una expresión aditiva
    fn parsear_expresion_aditiva(&mut self) -> Resultado<NodoAst> {
        let mut izquierda = self.parsear_expresion_multiplicativa()?;
        
        while let Some(t) = self.token_actual() {
            let operador = match t.token {
                crate::nucleo::lexico::token::Token::Mas => Some(OperadorBinario::Suma),
                crate::nucleo::lexico::token::Token::Menos => Some(OperadorBinario::Resta),
                _ => None,
            };
            
            if let Some(op) = operador {
                let posicion = t.posicion;
                self.avanzar();
                let derecha = self.parsear_expresion_multiplicativa()?;
                izquierda = NodoAst::ExpresionBinaria {
                    operador: op,
                    izquierda: Box::new(izquierda),
                    derecha: Box::new(derecha),
                    posicion,
                };
            } else {
                break;
            }
        }
        
        Ok(izquierda)
    }
    
    /// Parsea una expresión multiplicativa
    fn parsear_expresion_multiplicativa(&mut self) -> Resultado<NodoAst> {
        let mut izquierda = self.parsear_expresion_unaria()?;
        
        while let Some(t) = self.token_actual() {
            let operador = match t.token {
                crate::nucleo::lexico::token::Token::Multiplicar => Some(OperadorBinario::Multiplicacion),
                crate::nucleo::lexico::token::Token::Dividir => Some(OperadorBinario::Division),
                crate::nucleo::lexico::token::Token::Modulo => Some(OperadorBinario::Modulo),
                _ => None,
            };
            
            if let Some(op) = operador {
                let posicion = t.posicion;
                self.avanzar();
                let derecha = self.parsear_expresion_unaria()?;
                izquierda = NodoAst::ExpresionBinaria {
                    operador: op,
                    izquierda: Box::new(izquierda),
                    derecha: Box::new(derecha),
                    posicion,
                };
            } else {
                break;
            }
        }
        
        Ok(izquierda)
    }
    
    /// Parsea una expresión unaria
    fn parsear_expresion_unaria(&mut self) -> Resultado<NodoAst> {
        if let Some(t) = self.token_actual() {
            let operador = match t.token {
                crate::nucleo::lexico::token::Token::Negacion => Some(OperadorUnario::Negacion),
                crate::nucleo::lexico::token::Token::Menos => Some(OperadorUnario::Negativo),
                crate::nucleo::lexico::token::Token::Mas => Some(OperadorUnario::Positivo),
                _ => None,
            };
            
            if let Some(op) = operador {
                let posicion = t.posicion;
                self.avanzar();
                let expresion = self.parsear_expresion_unaria()?;
                return Ok(NodoAst::ExpresionUnaria {
                    operador: op,
                    expresion: Box::new(expresion),
                    posicion,
                });
            }
        }
        
        self.parsear_expresion_postfija()
    }
    
    /// Parsea una expresión postfija (maneja acceso a miembros, llamadas, índices, etc.)
    fn parsear_expresion_postfija(&mut self) -> Resultado<NodoAst> {
        let mut expresion = self.parsear_expresion_primaria()?;
        
        // Manejar operadores postfijos repetidamente
        loop {
            if let Some(t) = self.token_actual() {
                let posicion = t.posicion;
                
                match t.token {
                    // Acceso a miembro: objeto.miembro
                    crate::nucleo::lexico::token::Token::Punto => {
                        self.avanzar();
                        
                        if let Some(t) = self.token_actual() {
                            // Aceptar identificadores o palabras reservadas de tipo como identificadores después del punto
                            let miembro = match &t.token {
                                crate::nucleo::lexico::token::Token::Identificador(ref nombre) => {
                                    let nombre = nombre.clone();
                                    self.avanzar();
                                    nombre
                                }
                                // Palabras reservadas de tipo que pueden usarse como nombres de método
                                crate::nucleo::lexico::token::Token::Texto => {
                                    self.avanzar();
                                    "texto".to_string()
                                }
                                crate::nucleo::lexico::token::Token::Entero => {
                                    self.avanzar();
                                    "entero".to_string()
                                }
                                crate::nucleo::lexico::token::Token::Numero | crate::nucleo::lexico::token::Token::NumeroAcento => {
                                    self.avanzar();
                                    "numero".to_string()
                                }
                                crate::nucleo::lexico::token::Token::Log | crate::nucleo::lexico::token::Token::LogAcento => {
                                    self.avanzar();
                                    "log".to_string()
                                }
                                crate::nucleo::lexico::token::Token::Lista => {
                                    self.avanzar();
                                    "lista".to_string()
                                }
                                crate::nucleo::lexico::token::Token::Jsn => {
                                    self.avanzar();
                                    "jsn".to_string()
                                }
                                _ => {
                                    return Err(Error::analisis(
                                        CodigoError::SintaxisGeneral,
                                        format!("se esperaba un identificador después del punto, pero se encontró: {:?}", t.token),
                                        None,
                                        Some(t.posicion.linea),
                                        Some(t.posicion.columna),
                                    ));
                                }
                            };
                            
                            expresion = NodoAst::ExpresionAcceso {
                                objeto: Box::new(expresion),
                                miembro,
                                posicion,
                            };
                            continue;
                        } else {
                            return Err(Error::analisis(
                                CodigoError::SintaxisGeneral,
                                "se esperaba un identificador después del punto pero se alcanzó el final del archivo",
                                None,
                                Some(posicion.linea),
                                Some(posicion.columna),
                            ));
                        }
                    }
                    
                    // Llamada a función: funcion(argumentos)
                    crate::nucleo::lexico::token::Token::ParentesisIzq => {
                        self.avanzar();
                        let mut argumentos = Vec::new();
                        
                        // Parsear argumentos si los hay
                        if let Some(t) = self.token_actual() {
                            if !matches!(t.token, crate::nucleo::lexico::token::Token::ParentesisDer) {
                                loop {
                                    argumentos.push(self.parsear_expresion()?);
                                    
                                    if let Some(t) = self.token_actual() {
                                        if matches!(t.token, crate::nucleo::lexico::token::Token::Coma) {
                                            self.avanzar();
                                        } else if matches!(t.token, crate::nucleo::lexico::token::Token::ParentesisDer) {
                                            break;
                                        } else {
                                            return Err(Error::analisis(
                                                CodigoError::SintaxisGeneral,
                                                "se esperaba ',' o ')' en la lista de argumentos",
                                                None,
                                                Some(t.posicion.linea),
                                                Some(t.posicion.columna),
                                            ));
                                        }
                                    } else {
                                        return Err(Error::analisis(
                                            CodigoError::SintaxisGeneral,
                                            "se esperaba ')' para cerrar la llamada a función",
                                            None,
                                            None,
                                            None,
                                        ));
                                    }
                                }
                            }
                        }
                        
                        self.expectar_token(crate::nucleo::lexico::token::Token::ParentesisDer)?;
                        self.avanzar();
                        
                        expresion = NodoAst::ExpresionLlamada {
                            funcion: Box::new(expresion),
                            argumentos,
                            posicion,
                        };
                        continue;
                    }
                    
                    // Acceso por índice: lista[indice]
                    crate::nucleo::lexico::token::Token::CorcheteIzq => {
                        self.avanzar();
                        let indice = self.parsear_expresion()?;
                        self.expectar_token(crate::nucleo::lexico::token::Token::CorcheteDer)?;
                        self.avanzar();
                        
                        expresion = NodoAst::ExpresionIndice {
                            objeto: Box::new(expresion),
                            indice: Box::new(indice),
                            posicion,
                        };
                        continue;
                    }
                    
                    // Incremento/Decremento postfijo
                    crate::nucleo::lexico::token::Token::Incrementar => {
                        self.avanzar();
                        expresion = NodoAst::ExpresionUnaria {
                            operador: OperadorUnario::Incrementar,
                            expresion: Box::new(expresion),
                            posicion,
                        };
                        continue;
                    }
                    
                    crate::nucleo::lexico::token::Token::Decrementar => {
                        self.avanzar();
                        expresion = NodoAst::ExpresionUnaria {
                            operador: OperadorUnario::Decrementar,
                            expresion: Box::new(expresion),
                            posicion,
                        };
                        continue;
                    }
                    
                    _ => break,
                }
            } else {
                break;
            }
        }
        
        Ok(expresion)
    }
    
    /// Parsea una expresión primaria
    fn parsear_expresion_primaria(&mut self) -> Resultado<NodoAst> {
        if let Some(t) = self.token_actual() {
            let posicion = t.posicion;
            let token_clone = t.token.clone();
            
            match token_clone {
                crate::nucleo::lexico::token::Token::LiteralEntero(val) => {
                    self.avanzar();
                    return Ok(NodoAst::ExpresionLiteral {
                        valor: LiteralAst::Entero(val),
                        posicion,
                    });
                }
                crate::nucleo::lexico::token::Token::LiteralNumero(val) => {
                    self.avanzar();
                    return Ok(NodoAst::ExpresionLiteral {
                        valor: LiteralAst::Numero(val),
                        posicion,
                    });
                }
                crate::nucleo::lexico::token::Token::LiteralTexto(val) => {
                    self.avanzar();
                    return Ok(NodoAst::ExpresionLiteral {
                        valor: LiteralAst::Texto(val),
                        posicion,
                    });
                }
                crate::nucleo::lexico::token::Token::InterpolacionTexto(val) => {
                    self.avanzar();
                    return Ok(NodoAst::ExpresionLiteral {
                        valor: LiteralAst::InterpolacionTexto(val),
                        posicion,
                    });
                }
                crate::nucleo::lexico::token::Token::Verdadero => {
                    self.avanzar();
                    return Ok(NodoAst::ExpresionLiteral {
                        valor: LiteralAst::Logico(true),
                        posicion,
                    });
                }
                crate::nucleo::lexico::token::Token::Falso => {
                    self.avanzar();
                    return Ok(NodoAst::ExpresionLiteral {
                        valor: LiteralAst::Logico(false),
                        posicion,
                    });
                }
                crate::nucleo::lexico::token::Token::Esperar => {
                    self.avanzar();
                    // Parsear la expresión que se espera
                    let expresion = self.parsear_expresion()?;
                    return Ok(NodoAst::ExpresionEsperar {
                        expresion: Box::new(expresion),
                        posicion,
                    });
                }
                crate::nucleo::lexico::token::Token::Nulo => {
                    self.avanzar();
                    return Ok(NodoAst::ExpresionLiteral {
                        valor: LiteralAst::Nulo,
                        posicion,
                    });
                }
                // Expresión nuevo: nuevo NombreObjeto(argumentos)
                crate::nucleo::lexico::token::Token::Nuevo => {
                    self.avanzar();
                    
                    // Parsear el nombre del tipo/objeto
                    let tipo = if let Some(t) = self.token_actual() {
                        if let crate::nucleo::lexico::token::Token::Identificador(ref nombre) = t.token {
                            let nombre = nombre.clone();
                            self.avanzar();
                            nombre
                        } else {
                            return Err(Error::analisis(
                                CodigoError::SintaxisGeneral,
                                "se esperaba un identificador después de 'nuevo'",
                                None,
                                Some(t.posicion.linea),
                                Some(t.posicion.columna),
                            ));
                        }
                    } else {
                        return Err(Error::analisis(
                            CodigoError::SintaxisGeneral,
                            "se esperaba un identificador después de 'nuevo'",
                            None,
                            None,
                            None,
                        ));
                    };
                    
                    // Parsear argumentos del constructor
                    self.expectar_token(crate::nucleo::lexico::token::Token::ParentesisIzq)?;
                    self.avanzar();
                    
                    let mut argumentos = Vec::new();
                    
                    // Parsear argumentos si los hay
                    if let Some(t) = self.token_actual() {
                        if !matches!(t.token, crate::nucleo::lexico::token::Token::ParentesisDer) {
                            loop {
                                argumentos.push(self.parsear_expresion()?);
                                
                                if let Some(t) = self.token_actual() {
                                    if matches!(t.token, crate::nucleo::lexico::token::Token::Coma) {
                                        self.avanzar();
                                    } else if matches!(t.token, crate::nucleo::lexico::token::Token::ParentesisDer) {
                                        break;
                                    } else {
                                        return Err(Error::analisis(
                                            CodigoError::SintaxisGeneral,
                                            "se esperaba ',' o ')' en los argumentos del constructor",
                                            None,
                                            Some(t.posicion.linea),
                                            Some(t.posicion.columna),
                                        ));
                                    }
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                    
                    self.expectar_token(crate::nucleo::lexico::token::Token::ParentesisDer)?;
                    self.avanzar();
                    
                    return Ok(NodoAst::ExpresionNuevo {
                        tipo,
                        argumentos,
                        posicion,
                    });
                }
                crate::nucleo::lexico::token::Token::Identificador(nombre) => {
                    self.avanzar();
                    return Ok(NodoAst::ExpresionIdentificador {
                        nombre,
                        posicion,
                    });
                }
                crate::nucleo::lexico::token::Token::Ambiente => {
                    self.avanzar();
                    return Ok(NodoAst::ExpresionIdentificador {
                        nombre: "ambiente".to_string(),
                        posicion,
                    });
                }
                crate::nucleo::lexico::token::Token::ParentesisIzq => {
                    self.avanzar();
                    let expresion = self.parsear_expresion()?;
                    self.expectar_token(crate::nucleo::lexico::token::Token::ParentesisDer)?;
                    self.avanzar();
                    return Ok(expresion);
                }
                // Lista: [elemento1, elemento2, ...]
                crate::nucleo::lexico::token::Token::CorcheteIzq => {
                    self.avanzar();
                    let mut elementos = Vec::new();
                    
                    // Parsear elementos si los hay
                    if let Some(t) = self.token_actual() {
                        if !matches!(t.token, crate::nucleo::lexico::token::Token::CorcheteDer) {
                            loop {
                                elementos.push(self.parsear_expresion()?);
                                
                                if let Some(t) = self.token_actual() {
                                    if matches!(t.token, crate::nucleo::lexico::token::Token::Coma) {
                                        self.avanzar();
                                    } else if matches!(t.token, crate::nucleo::lexico::token::Token::CorcheteDer) {
                                        break;
                                    } else {
                                        return Err(Error::analisis(
                                            CodigoError::SintaxisGeneral,
                                            "se esperaba ',' o ']' en la lista",
                                            None,
                                            Some(t.posicion.linea),
                                            Some(t.posicion.columna),
                                        ));
                                    }
                                } else {
                                    return Err(Error::analisis(
                                        CodigoError::SintaxisGeneral,
                                        "se esperaba ']' para cerrar la lista",
                                        None,
                                        None,
                                        None,
                                    ));
                                }
                            }
                        }
                    }
                    
                    self.expectar_token(crate::nucleo::lexico::token::Token::CorcheteDer)?;
                    self.avanzar();
                    
                    return Ok(NodoAst::ExpresionLista {
                        elementos,
                        posicion,
                    });
                }
                // JSON: {clave: valor, ...}
                crate::nucleo::lexico::token::Token::LlaveIzq => {
                    self.avanzar();
                    let mut propiedades = Vec::new();
                    
                    // Parsear propiedades si las hay
                    if let Some(t) = self.token_actual() {
                        if !matches!(t.token, crate::nucleo::lexico::token::Token::LlaveDer) {
                            loop {
                                // Parsear clave (identificador o texto)
                                let (clave_str, clave_posicion) = if let Some(t) = self.token_actual() {
                                    let clave_posicion = t.posicion;
                                    let clave_str = match &t.token {
                                        crate::nucleo::lexico::token::Token::Identificador(ref nombre) => {
                                            let nombre_clone = nombre.clone();
                                            self.avanzar();
                                            nombre_clone
                                        }
                                        crate::nucleo::lexico::token::Token::LiteralTexto(ref texto) => {
                                            let texto_clone = texto.clone();
                                            self.avanzar();
                                            texto_clone
                                        }
                                        _ => {
                                            return Err(Error::analisis(
                                                CodigoError::SintaxisGeneral,
                                                "se esperaba un identificador o texto para la clave JSON",
                                                None,
                                                Some(clave_posicion.linea),
                                                Some(clave_posicion.columna),
                                            ));
                                        }
                                    };
                                    (clave_str, clave_posicion)
                                } else {
                                    return Err(Error::analisis(
                                        CodigoError::SintaxisGeneral,
                                        "se esperaba una propiedad JSON",
                                        None,
                                        None,
                                        None,
                                    ));
                                };
                                
                                // Verificar ':'
                                self.expectar_token(crate::nucleo::lexico::token::Token::DosPuntos)?;
                                self.avanzar();
                                
                                // Parsear valor
                                let valor = self.parsear_expresion()?;
                                
                                propiedades.push(PropiedadJsonAst {
                                    clave: clave_str,
                                    valor: Box::new(valor),
                                    posicion: clave_posicion,
                                });
                                
                                // Verificar si hay más propiedades
                                // Permitir comas opcionales: si el siguiente token es una cadena o identificador,
                                // asumimos que falta una coma y continuamos
                                if let Some(t) = self.token_actual() {
                                    if matches!(t.token, crate::nucleo::lexico::token::Token::Coma) {
                                        self.avanzar();
                                        continue;
                                    } else if matches!(t.token, crate::nucleo::lexico::token::Token::LlaveDer) {
                                        break;
                                    } else if matches!(t.token, crate::nucleo::lexico::token::Token::LiteralTexto(_) | crate::nucleo::lexico::token::Token::Identificador(_)) {
                                        // Falta una coma, pero continuamos de todos modos (tolerancia a errores)
                                        continue;
                                    } else {
                                        return Err(Error::analisis(
                                            CodigoError::SintaxisGeneral,
                                            "se esperaba ',' o '}' en el objeto JSON",
                                            None,
                                            Some(t.posicion.linea),
                                            Some(t.posicion.columna),
                                        ));
                                    }
                                } else {
                                    return Err(Error::analisis(
                                        CodigoError::SintaxisGeneral,
                                        "se esperaba '}' para cerrar el objeto JSON",
                                        None,
                                        None,
                                        None,
                                    ));
                                }
                            }
                        }
                    }
                    
                    self.expectar_token(crate::nucleo::lexico::token::Token::LlaveDer)?;
                    self.avanzar();
                    
                    return Ok(NodoAst::ExpresionJson {
                        propiedades,
                        posicion,
                    });
                }
                _ => {}
            }
        }
        
        let token_info = if let Some(t) = self.token_actual() {
            format!(" (token: {:?}, línea: {}, columna: {})", t.token, t.posicion.linea, t.posicion.columna)
        } else {
            " (fin de archivo)".to_string()
        };
        Err(Error::analisis(
            CodigoError::SintaxisGeneral,
            format!("expresión inválida{}", token_info),
            None,
            self.token_actual().map(|t| t.posicion.linea),
            self.token_actual().map(|t| t.posicion.columna),
        ))
    }
    
    /// Parsea un bloque: { declaraciones }
    fn parsear_bloque(&mut self) -> Resultado<NodoAst> {
        let posicion_inicial = self.token_actual()
            .map(|t| t.posicion)
            .unwrap_or_else(|| crate::nucleo::lexico::token::Posicion::inicial());
        
        // Verificar '{'
        self.expectar_token(crate::nucleo::lexico::token::Token::LlaveIzq)?;
        self.avanzar();
        
        let mut declaraciones = Vec::new();
        
        // Parsear declaraciones hasta encontrar '}'
        while let Some(t) = self.token_actual() {
            if matches!(t.token, crate::nucleo::lexico::token::Token::LlaveDer) {
                break;
            }
            
            self.saltar_comentarios_y_espacios();
            
            if let Some(t) = self.token_actual() {
                if matches!(t.token, crate::nucleo::lexico::token::Token::LlaveDer) {
                    break;
                }
            }
            
            declaraciones.push(self.parsear_declaracion()?);
            self.saltar_comentarios_y_espacios();
        }
        
        // Verificar '}'
        self.expectar_token(crate::nucleo::lexico::token::Token::LlaveDer)?;
        self.avanzar();
        
        Ok(NodoAst::Bloque {
            declaraciones,
            posicion: posicion_inicial,
        })
    }
    
    /// Parsea un bucle mientras: mientras (condicion) { cuerpo }
    fn parsear_mientras(&mut self) -> Resultado<NodoAst> {
        let posicion_inicial = self.token_actual()
            .map(|t| t.posicion)
            .unwrap_or_else(|| crate::nucleo::lexico::token::Posicion::inicial());
        
        // Verificar 'mientras'
        self.expectar_token(crate::nucleo::lexico::token::Token::Mientras)?;
        self.avanzar();
        
        // Verificar '('
        self.expectar_token(crate::nucleo::lexico::token::Token::ParentesisIzq)?;
        self.avanzar();
        
        // Parsear condición
        let condicion = self.parsear_expresion()?;
        
        // Verificar ')'
        self.expectar_token(crate::nucleo::lexico::token::Token::ParentesisDer)?;
        self.avanzar();
        
        // Parsear cuerpo (bloque)
        let cuerpo = self.parsear_bloque()?;
        
        Ok(NodoAst::Mientras {
            condicion: Box::new(condicion),
            cuerpo: Box::new(cuerpo),
            posicion: posicion_inicial,
        })
    }
    
    /// Parsea un bucle para: para (inicializacion; condicion; incremento) { cuerpo }
    /// o para (tipo var identificador en/cada coleccion) { cuerpo }
    fn parsear_para(&mut self) -> Resultado<NodoAst> {
        let posicion_inicial = self.token_actual()
            .map(|t| t.posicion)
            .unwrap_or_else(|| crate::nucleo::lexico::token::Posicion::inicial());
        
        // Verificar 'para'
        self.expectar_token(crate::nucleo::lexico::token::Token::Para)?;
        self.avanzar();
        
        // Verificar '('
        self.expectar_token(crate::nucleo::lexico::token::Token::ParentesisIzq)?;
        self.avanzar();
        
        // Verificar si es formato "para...en" o "para...cada"
        // Necesitamos mirar adelante para distinguir entre:
        // - para (tipo var id = ...) -> formato tradicional
        // - para (tipo var id en/cada ...) -> formato para...en
        if let Some(t) = self.token_actual() {
            if self.es_tipo(&t.token) {
                // Guardar posición para poder retroceder si es necesario
                let posicion_backup = self.posicion_actual;
                
                // Parsear tipo, var (opcional), e identificador
                let tipo = self.parsear_tipo()?;
                
                let mutable = if let Some(t) = self.token_actual() {
                    if matches!(t.token, crate::nucleo::lexico::token::Token::Var) {
                        self.avanzar();
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                
                // Verificar si hay un identificador
                let nombre_opt = if let Some(t) = self.token_actual() {
                    if let crate::nucleo::lexico::token::Token::Identificador(ref nombre) = t.token {
                        let nombre = nombre.clone();
                        self.avanzar();
                        Some(nombre)
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                // Si tenemos nombre, verificar si es formato para...en
                if let Some(nombre) = nombre_opt {
                    // Verificar si el siguiente token es '=' (formato tradicional) o 'en'/'cada' (formato para...en)
                    if let Some(t) = self.token_actual() {
                        match t.token {
                            crate::nucleo::lexico::token::Token::Asignar => {
                                // Es formato tradicional, retroceder y parsear como tal
                                self.posicion_actual = posicion_backup;
                                // Continuar con formato tradicional más abajo
                            }
                            crate::nucleo::lexico::token::Token::Cada => {
                                // Es formato para...cada
                                self.avanzar();
                                
                                // Parsear colección
                                let coleccion = self.parsear_expresion()?;
                                
                                // Verificar ')'
                                self.expectar_token(crate::nucleo::lexico::token::Token::ParentesisDer)?;
                                self.avanzar();
                                
                                // Parsear cuerpo
                                let cuerpo = self.parsear_bloque()?;
                                
                                return Ok(NodoAst::ParaEn {
                                    variable: nombre,
                                    tipo,
                                    mutable,
                                    coleccion: Box::new(coleccion),
                                    cuerpo: Box::new(cuerpo),
                                    posicion: posicion_inicial,
                                });
                            }
                            crate::nucleo::lexico::token::Token::Identificador(ref id) if id == "en" => {
                                // Es formato para...en
                                self.avanzar();
                                
                                // Parsear colección
                                let coleccion = self.parsear_expresion()?;
                                
                                // Verificar ')'
                                self.expectar_token(crate::nucleo::lexico::token::Token::ParentesisDer)?;
                                self.avanzar();
                                
                                // Parsear cuerpo
                                let cuerpo = self.parsear_bloque()?;
                                
                                return Ok(NodoAst::ParaEn {
                                    variable: nombre,
                                    tipo,
                                    mutable,
                                    coleccion: Box::new(coleccion),
                                    cuerpo: Box::new(cuerpo),
                                    posicion: posicion_inicial,
                                });
                            }
                            _ => {
                                // Retroceder y usar formato tradicional
                                self.posicion_actual = posicion_backup;
                                // Continuar con formato tradicional más abajo
                            }
                        }
                    } else {
                        // Retroceder y usar formato tradicional
                        self.posicion_actual = posicion_backup;
                        // Continuar con formato tradicional más abajo
                    }
                } else {
                    // No hay identificador, usar formato tradicional
                    self.posicion_actual = posicion_backup;
                    // Continuar con formato tradicional más abajo
                }
            }
        }
        
        // Formato tradicional: para (inicializacion; condicion; incremento)
        let inicializacion = if let Some(t) = self.token_actual() {
            if !matches!(t.token, crate::nucleo::lexico::token::Token::PuntoYComa) {
                Some(Box::new(self.parsear_declaracion_variable()?))
            } else {
                None
            }
        } else {
            None
        };
        
        // Verificar ';'
        self.expectar_token(crate::nucleo::lexico::token::Token::PuntoYComa)?;
        self.avanzar();
        
        // Parsear condición
        let condicion = if let Some(t) = self.token_actual() {
            if !matches!(t.token, crate::nucleo::lexico::token::Token::PuntoYComa) {
                Some(Box::new(self.parsear_expresion()?))
            } else {
                None
            }
        } else {
            None
        };
        
        // Verificar ';'
        self.expectar_token(crate::nucleo::lexico::token::Token::PuntoYComa)?;
        self.avanzar();
        
        // Parsear incremento
        let incremento = if let Some(t) = self.token_actual() {
            if !matches!(t.token, crate::nucleo::lexico::token::Token::ParentesisDer) {
                Some(Box::new(self.parsear_expresion()?))
            } else {
                None
            }
        } else {
            None
        };
        
        // Verificar ')'
        self.expectar_token(crate::nucleo::lexico::token::Token::ParentesisDer)?;
        self.avanzar();
        
        // Parsear cuerpo
        let cuerpo = self.parsear_bloque()?;
        
        Ok(NodoAst::Para {
            inicializacion,
            condicion,
            incremento,
            cuerpo: Box::new(cuerpo),
            posicion: posicion_inicial,
        })
    }
    
    /// Parsea un condicional si: si (condicion) { entonces } [sino { sino }]
    fn parsear_si(&mut self) -> Resultado<NodoAst> {
        let posicion_inicial = self.token_actual()
            .map(|t| t.posicion)
            .unwrap_or_else(|| crate::nucleo::lexico::token::Posicion::inicial());
        
        // Verificar 'si'
        self.expectar_token(crate::nucleo::lexico::token::Token::Si)?;
        self.avanzar();
        
        // Verificar '('
        self.expectar_token(crate::nucleo::lexico::token::Token::ParentesisIzq)?;
        self.avanzar();
        
        // Parsear condición
        let condicion = self.parsear_expresion()?;
        
        // Verificar ')'
        self.expectar_token(crate::nucleo::lexico::token::Token::ParentesisDer)?;
        self.avanzar();
        
        // Parsear bloque entonces
        let entonces = self.parsear_bloque()?;
        
        // Verificar si hay 'sino' o 'sino si'
        let sino = if let Some(t) = self.token_actual() {
            if matches!(t.token, crate::nucleo::lexico::token::Token::Sino) {
                self.avanzar();
                
                // Verificar si es 'sino si' o solo 'sino'
                if let Some(t) = self.token_actual() {
                    if matches!(t.token, crate::nucleo::lexico::token::Token::Si) {
                        // Es 'sino si', parsear otro Si y ponerlo en sino
                        Some(Box::new(self.parsear_si()?))
                    } else {
                        // Es solo 'sino', parsear bloque
                        Some(Box::new(self.parsear_bloque()?))
                    }
                } else {
                    Some(Box::new(self.parsear_bloque()?))
                }
            } else {
                None
            }
        } else {
            None
        };
        
        Ok(NodoAst::Si {
            condicion: Box::new(condicion),
            entonces: Box::new(entonces),
            sino,
            posicion: posicion_inicial,
        })
    }
    
    /// Parsea un bucle hacer...mientras: hacer { cuerpo } mientras (condicion)
    fn parsear_hacer_mientras(&mut self) -> Resultado<NodoAst> {
        let posicion_inicial = self.token_actual()
            .map(|t| t.posicion)
            .unwrap_or_else(|| crate::nucleo::lexico::token::Posicion::inicial());
        
        // Verificar 'hacer'
        self.expectar_token(crate::nucleo::lexico::token::Token::Hacer)?;
        self.avanzar();
        
        // Parsear cuerpo
        let cuerpo = self.parsear_bloque()?;
        
        // Verificar 'mientras'
        self.expectar_token(crate::nucleo::lexico::token::Token::Mientras)?;
        self.avanzar();
        
        // Verificar '('
        self.expectar_token(crate::nucleo::lexico::token::Token::ParentesisIzq)?;
        self.avanzar();
        
        // Parsear condición
        let condicion = self.parsear_expresion()?;
        
        // Verificar ')'
        self.expectar_token(crate::nucleo::lexico::token::Token::ParentesisDer)?;
        self.avanzar();
        
        Ok(NodoAst::HacerMientras {
            cuerpo: Box::new(cuerpo),
            condicion: Box::new(condicion),
            posicion: posicion_inicial,
        })
    }
    
    /// Parsea la palabra reservada romper
    fn parsear_romper(&mut self) -> Resultado<NodoAst> {
        let posicion = self.token_actual()
            .map(|t| t.posicion)
            .unwrap_or_else(|| crate::nucleo::lexico::token::Posicion::inicial());
        
        self.expectar_token(crate::nucleo::lexico::token::Token::Romper)?;
        self.avanzar();
        
        Ok(NodoAst::Romper { posicion })
    }
    
    /// Parsea la palabra reservada continuar
    fn parsear_continuar(&mut self) -> Resultado<NodoAst> {
        let posicion = self.token_actual()
            .map(|t| t.posicion)
            .unwrap_or_else(|| crate::nucleo::lexico::token::Posicion::inicial());
        
        self.expectar_token(crate::nucleo::lexico::token::Token::Continuar)?;
        self.avanzar();
        
        Ok(NodoAst::Continuar { posicion })
    }
    
    /// Parsea la palabra reservada retornar
    fn parsear_retornar(&mut self) -> Resultado<NodoAst> {
        let posicion = self.token_actual()
            .map(|t| t.posicion)
            .unwrap_or_else(|| crate::nucleo::lexico::token::Posicion::inicial());
        
        self.expectar_token(crate::nucleo::lexico::token::Token::Retornar)?;
        self.avanzar();
        
        // El valor es opcional
        let valor = if let Some(t) = self.token_actual() {
            // Si el siguiente token no es un delimitador o palabra reservada, es una expresión
            match t.token {
                crate::nucleo::lexico::token::Token::LlaveDer
                | crate::nucleo::lexico::token::Token::PuntoYComa
                | crate::nucleo::lexico::token::Token::ParentesisDer => None,
                _ => Some(Box::new(self.parsear_expresion()?)),
            }
        } else {
            None
        };
        
        Ok(NodoAst::Retornar { valor, posicion })
    }
    
    /// Parsea un bloque intentar...capturar...finalmente
    fn parsear_intentar(&mut self) -> Resultado<NodoAst> {
        let posicion_inicial = self.token_actual()
            .map(|t| t.posicion)
            .unwrap_or_else(|| crate::nucleo::lexico::token::Posicion::inicial());
        
        // Verificar 'intentar'
        self.expectar_token(crate::nucleo::lexico::token::Token::Intentar)?;
        self.avanzar();
        
        // Parsear bloque intentar
        let bloque = self.parsear_bloque()?;
        
        // Verificar si hay 'capturar'
        let capturar = if let Some(t) = self.token_actual() {
            if matches!(t.token, crate::nucleo::lexico::token::Token::Capturar) {
                self.avanzar();
                
                // Verificar '('
                self.expectar_token(crate::nucleo::lexico::token::Token::ParentesisIzq)?;
                self.avanzar();
                
                // Parsear nombre de la variable de excepción
                // Puede tener un tipo opcional antes del identificador (ej: "excepcion e" o solo "e")
                let variable = if let Some(t) = self.token_actual() {
                    // Si el primer token es "excepcion" o "excepción", es el tipo, avanzar y tomar el siguiente
                    if matches!(t.token, crate::nucleo::lexico::token::Token::Excepcion | crate::nucleo::lexico::token::Token::ExcepcionSinAcento) {
                        self.avanzar(); // Saltar el tipo
                        // Ahora esperamos el identificador
                        if let Some(t2) = self.token_actual() {
                            if let crate::nucleo::lexico::token::Token::Identificador(ref nombre) = t2.token {
                                let nombre = nombre.clone();
                                self.avanzar();
                                nombre
                            } else {
                                // Si no hay identificador después del tipo, usar "excepcion" como nombre
                                "excepcion".to_string()
                            }
                        } else {
                            "excepcion".to_string()
                        }
                    } else if let crate::nucleo::lexico::token::Token::Identificador(ref nombre) = t.token {
                        let nombre = nombre.clone();
                        self.avanzar();
                        nombre
                    } else {
                        return Err(Error::analisis(
                            CodigoError::SintaxisGeneral,
                            "se esperaba un identificador para la variable de excepción",
                            None,
                            Some(t.posicion.linea),
                            Some(t.posicion.columna),
                        ));
                    }
                } else {
                    return Err(Error::analisis(
                        CodigoError::SintaxisGeneral,
                        "se esperaba un identificador para la variable de excepción",
                        None,
                        None,
                        None,
                    ));
                };
                
                // Verificar ')'
                self.expectar_token(crate::nucleo::lexico::token::Token::ParentesisDer)?;
                self.avanzar();
                
                // Parsear bloque capturar
                let bloque_capturar = self.parsear_bloque()?;
                
                Some(crate::nucleo::sintactico::ast::CapturarAst {
                    variable,
                    bloque: Box::new(bloque_capturar),
                    posicion: posicion_inicial,
                })
            } else {
                None
            }
        } else {
            None
        };
        
        // Verificar si hay 'finalmente'
        let finalmente = if let Some(t) = self.token_actual() {
            if matches!(t.token, crate::nucleo::lexico::token::Token::Finalmente) {
                self.avanzar();
                Some(Box::new(self.parsear_bloque()?))
            } else {
                None
            }
        } else {
            None
        };
        
        Ok(NodoAst::Intentar {
            bloque: Box::new(bloque),
            capturar,
            finalmente,
            posicion: posicion_inicial,
        })
    }
    
    /// Parsea la palabra reservada lanzar
    fn parsear_lanzar(&mut self) -> Resultado<NodoAst> {
        let posicion = self.token_actual()
            .map(|t| t.posicion)
            .unwrap_or_else(|| crate::nucleo::lexico::token::Posicion::inicial());
        
        // Verificar 'lanzar'
        self.expectar_token(crate::nucleo::lexico::token::Token::Lanzar)?;
        self.avanzar();
        
        // Parsear expresión (mensaje de la excepción)
        let expresion = self.parsear_expresion()?;
        
        Ok(NodoAst::Lanzar {
            expresion: Box::new(expresion),
            posicion,
        })
    }
    
    /// Espera un token específico
    fn expectar_token(&self, esperado: crate::nucleo::lexico::token::Token) -> Resultado<()> {
        if let Some(t) = self.token_actual() {
            if std::mem::discriminant(&t.token) == std::mem::discriminant(&esperado) {
                Ok(())
            } else {
                Err(Error::analisis(
                    CodigoError::SintaxisGeneral,
                    format!("se esperaba {:?}", esperado),
                    None,
                    Some(t.posicion.linea),
                    Some(t.posicion.columna),
                ))
            }
        } else {
            Err(Error::analisis(
                CodigoError::SintaxisGeneral,
                format!("se esperaba {:?} pero se alcanzó el final del archivo", esperado),
                None,
                None,
                None,
            ))
        }
    }
}
