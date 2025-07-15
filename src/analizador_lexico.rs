// Analizador léxico para el lenguaje Quetzal
// Convierte el código fuente en tokens para el análisis sintáctico

use crate::errores::{ErrorQuetzal, ResultadoQuetzal};

/// Tipos de tokens del lenguaje Quetzal
#[derive(Debug, Clone, PartialEq)]
pub enum TipoToken {
    // Tipos de datos
    TipoVacio,          // vacio
    TipoEntero,         // entero
    TipoNumero,         // número
    TipoTexto,          // texto
    TipoLog,            // log
    TipoLista,          // lista
    TipoJson,           // jsn
    
    // Modificadores
    Var,                // var
    Publico,            // publico
    Privado,            // privado
    Libre,              // libre
    
    // Palabras clave de control de flujo
    Si,                 // si
    Sino,               // sino
    Para,               // para
    Mientras,           // mientras
    Hacer,              // hacer
    Romper,             // romper
    Continuar,          // continuar
    
    // Palabras clave de funciones
    Retornar,           // retornar
    Objeto,             // objeto
    Nuevo,              // nuevo
    Ambiente,           // ambiente
    Asincrono,          // asincrono
    Esperar,            // esperar
    
    // Palabras clave de excepciones
    Intentar,           // intentar
    Atrapar,            // atrapar
    Finalmente,         // finalmente
    Lanzar,             // lanzar
    Excepcion,          // excepción
    
    // Palabras clave de módulos
    Importar,           // importar
    Exportar,           // exportar
    Desde,              // desde
    Como,               // como
    
    // Valores booleanos
    Verdadero,          // verdadero
    Falso,              // falso
    
    // Operadores lógicos
    Y,                  // y
    O,                  // o
    
    // Operadores
    Suma,               // +
    Resta,              // -
    Multiplicacion,     // *
    Division,           // /
    Modulo,             // %
    Asignacion,         // =
    AsignacionSuma,     // +=
    AsignacionResta,    // -=
    AsignacionMult,     // *=
    AsignacionDiv,      // /=
    AsignacionMod,      // %=
    Incremento,         // ++
    Decremento,         // --
    
    // Comparación
    Igual,              // ==
    Diferente,          // !=
    Mayor,              // >
    Menor,              // <
    MayorIgual,         // >=
    MenorIgual,         // <=
    
    // Lógicos
    AndLogico,          // &&
    OrLogico,           // ||
    Not,                // !
    
    // Delimitadores
    ParentesisAbre,     // (
    ParentesisCierra,   // )
    LlaveAbre,          // {
    LlaveCierra,        // }
    CorcheteAbre,       // [
    CorcheteCierra,     // ]
    Coma,               // ,
    Punto,              // .
    DosPuntos,          // :
    PuntoYComa,         // ;
    
    // Operador ternario
    Pregunta,           // ?
    
    // Propagación
    Propagacion,        // ...
    
    // Concatenación de cadenas
    ConcatenacionVar,   // c"..."
    
    // Literales
    LiteralEntero(i64),
    LiteralNumero(f64),
    LiteralCadena(String),
    Identificador(String),
    
    // Comentarios (generalmente se ignoran)
    ComentarioLinea(String),
    ComentarioBloque(String),
    
    // En
    En,                 // en (obsoleto - usar Cada)
    Cada,               // cada
    
    // Final de archivo
    FinArchivo,
    
    // Nueva línea (para control de líneas)
    NuevaLinea,
}

/// Representación de un token con su posición
#[derive(Debug, Clone)]
pub struct Token {
    pub tipo: TipoToken,
    pub lexema: String,
    pub linea: usize,
    #[allow(dead_code)]
    pub columna: usize,
}

impl Token {
    pub fn nuevo(tipo: TipoToken, lexema: String, linea: usize, columna: usize) -> Self {
        Token {
            tipo,
            lexema,
            linea,
            columna,
        }
    }
}

/// Analizador léxico que convierte el código fuente en tokens
pub struct AnalizadorLexico {
    codigo: Vec<char>,
    posicion: usize,
    linea_actual: usize,
    columna_actual: usize,
}

impl AnalizadorLexico {
    /// Crea un nuevo analizador léxico
    pub fn nuevo(codigo: &str) -> Self {
        AnalizadorLexico {
            codigo: codigo.chars().collect(),
            posicion: 0,
            linea_actual: 1,
            columna_actual: 1,
        }
    }
    
    /// Analiza el código y devuelve todos los tokens
    pub fn analizar(&mut self) -> ResultadoQuetzal<Vec<Token>> {
        let mut tokens = Vec::new();
        
        while !self.esta_al_final() {
            let token = self.siguiente_token()?;
            
            // Agregar nueva línea como token para control de líneas
            if matches!(token.tipo, TipoToken::NuevaLinea) {
                tokens.push(token);
                continue;
            }
            
            // Ignorar comentarios pero agregar otros tokens
            if !matches!(token.tipo, TipoToken::ComentarioLinea(_) | TipoToken::ComentarioBloque(_)) {
                tokens.push(token);
            }
        }
        
        // Agregar token de fin de archivo
        tokens.push(Token::nuevo(
            TipoToken::FinArchivo,
            String::new(),
            self.linea_actual,
            self.columna_actual,
        ));
        
        Ok(tokens)
    }
    
    /// Obtiene el siguiente token
    fn siguiente_token(&mut self) -> ResultadoQuetzal<Token> {
        self.saltar_espacios_en_blanco();
        
        if self.esta_al_final() {
            return Ok(Token::nuevo(
                TipoToken::FinArchivo,
                String::new(),
                self.linea_actual,
                self.columna_actual,
            ));
        }
        
        let caracter_actual = self.avanzar();
        let linea = self.linea_actual;
        let columna = self.columna_actual - 1;
        
        match caracter_actual {
            // Nueva línea
            '\n' => {
                self.linea_actual += 1;
                self.columna_actual = 1;
                Ok(Token::nuevo(TipoToken::NuevaLinea, "\n".to_string(), linea, columna))
            },
            
            // Operadores de un carácter
            '+' => {
                if self.coincidir('+') {
                    Ok(Token::nuevo(TipoToken::Incremento, "++".to_string(), linea, columna))
                } else if self.coincidir('=') {
                    Ok(Token::nuevo(TipoToken::AsignacionSuma, "+=".to_string(), linea, columna))
                } else {
                    Ok(Token::nuevo(TipoToken::Suma, "+".to_string(), linea, columna))
                }
            },
            '-' => {
                if self.coincidir('-') {
                    Ok(Token::nuevo(TipoToken::Decremento, "--".to_string(), linea, columna))
                } else if self.coincidir('=') {
                    Ok(Token::nuevo(TipoToken::AsignacionResta, "-=".to_string(), linea, columna))
                } else {
                    Ok(Token::nuevo(TipoToken::Resta, "-".to_string(), linea, columna))
                }
            },
            '*' => {
                if self.coincidir('=') {
                    Ok(Token::nuevo(TipoToken::AsignacionMult, "*=".to_string(), linea, columna))
                } else {
                    Ok(Token::nuevo(TipoToken::Multiplicacion, "*".to_string(), linea, columna))
                }
            },
            '/' => {
                if self.coincidir('=') {
                    Ok(Token::nuevo(TipoToken::AsignacionDiv, "/=".to_string(), linea, columna))
                } else if self.coincidir('/') {
                    // Comentario de línea
                    self.comentario_linea()
                } else if self.coincidir('*') {
                    // Comentario de bloque
                    self.comentario_bloque()
                } else {
                    Ok(Token::nuevo(TipoToken::Division, "/".to_string(), linea, columna))
                }
            },
            '%' => {
                if self.coincidir('=') {
                    Ok(Token::nuevo(TipoToken::AsignacionMod, "%=".to_string(), linea, columna))
                } else {
                    Ok(Token::nuevo(TipoToken::Modulo, "%".to_string(), linea, columna))
                }
            },
            '=' => {
                if self.coincidir('=') {
                    Ok(Token::nuevo(TipoToken::Igual, "==".to_string(), linea, columna))
                } else {
                    Ok(Token::nuevo(TipoToken::Asignacion, "=".to_string(), linea, columna))
                }
            },
            '!' => {
                if self.coincidir('=') {
                    Ok(Token::nuevo(TipoToken::Diferente, "!=".to_string(), linea, columna))
                } else {
                    Ok(Token::nuevo(TipoToken::Not, "!".to_string(), linea, columna))
                }
            },
            '>' => {
                if self.coincidir('=') {
                    Ok(Token::nuevo(TipoToken::MayorIgual, ">=".to_string(), linea, columna))
                } else {
                    Ok(Token::nuevo(TipoToken::Mayor, ">".to_string(), linea, columna))
                }
            },
            '<' => {
                if self.coincidir('=') {
                    Ok(Token::nuevo(TipoToken::MenorIgual, "<=".to_string(), linea, columna))
                } else {
                    Ok(Token::nuevo(TipoToken::Menor, "<".to_string(), linea, columna))
                }
            },
            '&' => {
                if self.coincidir('&') {
                    Ok(Token::nuevo(TipoToken::AndLogico, "&&".to_string(), linea, columna))
                } else {
                    Err(ErrorQuetzal::TokenInesperado { 
                        linea: self.linea_actual, 
                        token: "&".to_string() 
                    })
                }
            },
            '|' => {
                if self.coincidir('|') {
                    Ok(Token::nuevo(TipoToken::OrLogico, "||".to_string(), linea, columna))
                } else {
                    Err(ErrorQuetzal::TokenInesperado { 
                        linea: self.linea_actual, 
                        token: "|".to_string() 
                    })
                }
            },
            
            // Delimitadores
            '(' => Ok(Token::nuevo(TipoToken::ParentesisAbre, "(".to_string(), linea, columna)),
            ')' => Ok(Token::nuevo(TipoToken::ParentesisCierra, ")".to_string(), linea, columna)),
            '{' => Ok(Token::nuevo(TipoToken::LlaveAbre, "{".to_string(), linea, columna)),
            '}' => Ok(Token::nuevo(TipoToken::LlaveCierra, "}".to_string(), linea, columna)),
            '[' => Ok(Token::nuevo(TipoToken::CorcheteAbre, "[".to_string(), linea, columna)),
            ']' => Ok(Token::nuevo(TipoToken::CorcheteCierra, "]".to_string(), linea, columna)),
            ',' => Ok(Token::nuevo(TipoToken::Coma, ",".to_string(), linea, columna)),
            ';' => Ok(Token::nuevo(TipoToken::PuntoYComa, ";".to_string(), linea, columna)),
            '?' => Ok(Token::nuevo(TipoToken::Pregunta, "?".to_string(), linea, columna)),
            ':' => Ok(Token::nuevo(TipoToken::DosPuntos, ":".to_string(), linea, columna)),
            '.' => {
                if self.coincidir('.') && self.coincidir('.') {
                    Ok(Token::nuevo(TipoToken::Propagacion, "...".to_string(), linea, columna))
                } else {
                    Ok(Token::nuevo(TipoToken::Punto, ".".to_string(), linea, columna))
                }
            },
            
            // Cadenas de texto
            '"' => self.cadena_texto(),
            
            // Concatenación de cadenas con variables
            'c' if self.mirar() == Some('"') => {
                self.avanzar(); // Saltar la 'c'
                let mut resultado = self.cadena_texto()?;
                resultado.tipo = TipoToken::ConcatenacionVar;
                resultado.lexema = format!("c{}", resultado.lexema);
                Ok(resultado)
            },
            
            // Números
            c if c.is_ascii_digit() => self.numero(),
            
            // Identificadores y palabras clave
            c if c.is_alphabetic() || c == '_' => self.identificador(),
            
            _ => Err(ErrorQuetzal::TokenInesperado { 
                linea: self.linea_actual, 
                token: caracter_actual.to_string() 
            }),
        }
    }
    
    /// Procesa comentarios de línea
    fn comentario_linea(&mut self) -> ResultadoQuetzal<Token> {
        let mut comentario = String::new();
        let linea = self.linea_actual;
        let columna = self.columna_actual - 2;
        
        while self.mirar() != Some('\n') && !self.esta_al_final() {
            comentario.push(self.avanzar());
        }
        
        Ok(Token::nuevo(TipoToken::ComentarioLinea(comentario.clone()), format!("//{}", comentario), linea, columna))
    }
    
    /// Procesa comentarios de bloque
    fn comentario_bloque(&mut self) -> ResultadoQuetzal<Token> {
        let mut comentario = String::new();
        let linea = self.linea_actual;
        let columna = self.columna_actual - 2;
        let mut cerrado = false;
        let mut nivel_anidacion = 1; // Empezamos con 1 porque ya encontramos el /*
        
        while !self.esta_al_final() && nivel_anidacion > 0 {
            if self.mirar() == Some('/') && self.mirar_siguiente() == Some('*') {
                // Encontramos otro /* - incrementar nivel (comentario anidado)
                comentario.push(self.avanzar()); // /
                comentario.push(self.avanzar()); // *
                nivel_anidacion += 1;
            } else if self.mirar() == Some('*') && self.mirar_siguiente() == Some('/') {
                // Encontramos */ - decrementar nivel
                nivel_anidacion -= 1;
                if nivel_anidacion > 0 {
                    comentario.push(self.avanzar()); // *
                    comentario.push(self.avanzar()); // /
                } else {
                    self.avanzar(); // Saltar *
                    self.avanzar(); // Saltar /
                    cerrado = true;
                }
            } else {
                let caracter = self.avanzar();
                if caracter == '\n' {
                    self.linea_actual += 1;
                    self.columna_actual = 1;
                }
                comentario.push(caracter);
            }
        }
        
        if !cerrado {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea,
                mensaje: "Comentario de bloque sin cerrar: se esperaba '*/'".to_string(),
            });
        }
        
        Ok(Token::nuevo(TipoToken::ComentarioBloque(comentario.clone()), format!("/*{}*/", comentario), linea, columna))
    }
    
    /// Procesa cadenas de texto
    fn cadena_texto(&mut self) -> ResultadoQuetzal<Token> {
        let mut valor = String::new();
        let linea = self.linea_actual;
        let columna = self.columna_actual - 1;
        
        while self.mirar() != Some('"') && !self.esta_al_final() {
            let c = self.avanzar();
            
            if c == '\\' {
                // Procesar secuencia de escape
                if let Some(siguiente) = self.mirar() {
                    match siguiente {
                        '"' => {
                            valor.push('"');
                            self.avanzar(); // Consumir el carácter escapado
                        },
                        '\\' => {
                            valor.push('\\');
                            self.avanzar();
                        },
                        'n' => {
                            valor.push('\n');
                            self.avanzar();
                        },
                        't' => {
                            valor.push('\t');
                            self.avanzar();
                        },
                        'r' => {
                            valor.push('\r');
                            self.avanzar();
                        },
                        '0' => {
                            valor.push('\0');
                            self.avanzar();
                        },
                        _ => {
                            // Secuencia de escape no válida
                            return Err(ErrorQuetzal::ErrorSintaxis {
                                linea: self.linea_actual,
                                mensaje: format!("Secuencia de escape no válida: \\{}", siguiente),
                            });
                        }
                    }
                } else {
                    // Backslash al final de la cadena
                    return Err(ErrorQuetzal::ErrorSintaxis {
                        linea: self.linea_actual,
                        mensaje: "Secuencia de escape incompleta".to_string(),
                    });
                }
            } else {
                if c == '\n' {
                    self.linea_actual += 1;
                    self.columna_actual = 1;
                }
                valor.push(c);
            }
        }
        
        if self.esta_al_final() {
            return Err(ErrorQuetzal::ErrorSintaxis {
                linea: self.linea_actual,
                mensaje: "Cadena de texto sin cerrar".to_string(),
            });
        }
        
        self.avanzar(); // Cerrar comillas
        
        Ok(Token::nuevo(
            TipoToken::LiteralCadena(valor.clone()),
            format!("\"{}\"", valor),
            linea,
            columna,
        ))
    }
    
    /// Procesa números (enteros y decimales)
    fn numero(&mut self) -> ResultadoQuetzal<Token> {
        let mut numero = String::new();
        let linea = self.linea_actual;
        let columna = self.columna_actual - 1;
        
        // Retroceder para incluir el primer dígito
        self.posicion -= 1;
        self.columna_actual -= 1;
        
        // Leer la parte entera
        while let Some(c) = self.mirar() {
            if c.is_ascii_digit() {
                numero.push(self.avanzar());
            } else {
                break;
            }
        }
        
        // Verificar si es decimal
        if self.mirar() == Some('.') && self.mirar_siguiente().is_some_and(|c| c.is_ascii_digit()) {
            numero.push(self.avanzar()); // Agregar el punto
            
            // Leer la parte decimal
            while let Some(c) = self.mirar() {
                if c.is_ascii_digit() {
                    numero.push(self.avanzar());
                } else {
                    break;
                }
            }
            
            // Es un número decimal
            let valor: f64 = numero.parse().map_err(|_| ErrorQuetzal::ErrorSintaxis {
                linea: self.linea_actual,
                mensaje: format!("Número decimal inválido: {}", numero),
            })?;
            
            Ok(Token::nuevo(TipoToken::LiteralNumero(valor), numero, linea, columna))
        } else {
            // Es un número entero
            let valor: i64 = numero.parse().map_err(|_| ErrorQuetzal::ErrorSintaxis {
                linea: self.linea_actual,
                mensaje: format!("Número entero inválido: {}", numero),
            })?;
            
            Ok(Token::nuevo(TipoToken::LiteralEntero(valor), numero, linea, columna))
        }
    }
    
    /// Procesa identificadores y palabras clave
    fn identificador(&mut self) -> ResultadoQuetzal<Token> {
        let mut texto = String::new();
        let linea = self.linea_actual;
        let columna = self.columna_actual - 1;
        
        // Retroceder para incluir el primer carácter
        self.posicion -= 1;
        self.columna_actual -= 1;
        
        while let Some(c) = self.mirar() {
            if c.is_alphanumeric() || c == '_' {
                texto.push(self.avanzar());
            } else {
                break;
            }
        }
        
        // Verificar si es una palabra clave
        let tipo_token = match texto.as_str() {
            // Tipos de datos
            "vacio" => TipoToken::TipoVacio,
            "entero" => TipoToken::TipoEntero,
            "número" => TipoToken::TipoNumero,
            "texto" => TipoToken::TipoTexto,
            "log" => TipoToken::TipoLog,
            "lista" => TipoToken::TipoLista,
            "jsn" => TipoToken::TipoJson,
            
            // Modificadores
            "var" => TipoToken::Var,
            "publico" => TipoToken::Publico,
            "privado" => TipoToken::Privado,
            "libre" => TipoToken::Libre,
            
            // Control de flujo
            "si" => TipoToken::Si,
            "sino" => TipoToken::Sino,
            "para" => TipoToken::Para,
            "mientras" => TipoToken::Mientras,
            "hacer" => TipoToken::Hacer,
            "romper" => TipoToken::Romper,
            "continuar" => TipoToken::Continuar,
            "en" => TipoToken::En,        // Mantener para compatibilidad
            "cada" => TipoToken::Cada,    // Nueva sintaxis preferida
            
            // Funciones y objetos
            "retornar" => TipoToken::Retornar,
            "objeto" => TipoToken::Objeto,
            "nuevo" => TipoToken::Nuevo,
            "ambiente" => TipoToken::Ambiente,
            "asincrono" => TipoToken::Asincrono,
            "esperar" => TipoToken::Esperar,
            
            // Excepciones
            "intentar" => TipoToken::Intentar,
            "atrapar" => TipoToken::Atrapar,
            "finalmente" => TipoToken::Finalmente,
            "lanzar" => TipoToken::Lanzar,
            "excepción" => TipoToken::Excepcion,
            
            // Módulos
            "importar" => TipoToken::Importar,
            "exportar" => TipoToken::Exportar,
            "desde" => TipoToken::Desde,
            "como" => TipoToken::Como,
            
            // Valores booleanos
            "verdadero" => TipoToken::Verdadero,
            "falso" => TipoToken::Falso,
            
            // Operadores lógicos
            "y" => TipoToken::Y,
            "o" => TipoToken::O,
            
            // Si no es palabra clave, es un identificador
            _ => TipoToken::Identificador(texto.clone()),
        };
        
        Ok(Token::nuevo(tipo_token, texto, linea, columna))
    }
    
    /// Avanza al siguiente carácter
    fn avanzar(&mut self) -> char {
        if self.esta_al_final() {
            return '\0';
        }
        
        let caracter = self.codigo[self.posicion];
        self.posicion += 1;
        self.columna_actual += 1;
        caracter
    }
    
    /// Mira el carácter actual sin avanzar
    fn mirar(&self) -> Option<char> {
        if self.esta_al_final() {
            None
        } else {
            Some(self.codigo[self.posicion])
        }
    }
    
    /// Mira el siguiente carácter sin avanzar
    fn mirar_siguiente(&self) -> Option<char> {
        if self.posicion + 1 >= self.codigo.len() {
            None
        } else {
            Some(self.codigo[self.posicion + 1])
        }
    }
    
    /// Verifica si coincide con el carácter esperado y avanza si es así
    fn coincidir(&mut self, esperado: char) -> bool {
        if self.mirar() == Some(esperado) {
            self.avanzar();
            true
        } else {
            false
        }
    }
    
    /// Verifica si estamos al final del código
    fn esta_al_final(&self) -> bool {
        self.posicion >= self.codigo.len()
    }
    
    /// Salta espacios en blanco (excepto nueva línea)
    fn saltar_espacios_en_blanco(&mut self) {
        while let Some(c) = self.mirar() {
            if c == ' ' || c == '\r' || c == '\t' {
                self.avanzar();
            } else {
                break;
            }
        }
    }
}

// === COMPATIBILIDAD HACIA ATRÁS ===
// Palabras reservadas anteriores que mapean a las nuevas
// "cadena" => TipoToken::TipoTexto,  // cadena -> texto
// "bool" => TipoToken::TipoLog,      // bool -> log
// "mut" => TipoToken::Var,           // mut -> var
