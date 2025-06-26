/// Tipos de token que reconoce el analizador léxico
#[derive(Debug, PartialEq)]
pub enum TipoToken {
    PalabraClave(String),
    Identificador(String),
    Numero(String),
    Cadena(String),
    SignoIgual,
    CorcheteAbre,
    CorcheteCierra,
    LlaveAbre,
    LlaveCierra,
    Coma,
}

#[derive(Debug)]
pub struct Token {
    pub tipo: TipoToken,
}

/// Analiza el contenido de una cadena y devuelve una lista de tokens
pub fn analizar_lexico(contenido: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut iterador = contenido.chars().peekable();

    while let Some(&actual) = iterador.peek() {
        match actual {
            ' ' | '\n' | '\t' | '\r' => {
                iterador.next();
            }
            '/' => {
                iterador.next();
                if let Some('/') = iterador.peek() {
                    // Comentario de una línea
                    while let Some(c) = iterador.next() {
                        if c == '\n' { break; }
                    }
                } else if let Some('*') = iterador.peek() {
                    // Comentario multilínea
                    iterador.next();
                    while let Some(c) = iterador.next() {
                        if c == '*' {
                            if let Some('/') = iterador.peek() {
                                iterador.next();
                                break;
                            }
                        }
                    }
                }
            }
            '=' => {
                iterador.next();
                tokens.push(Token { tipo: TipoToken::SignoIgual });
            }
            '[' => { iterador.next(); tokens.push(Token { tipo: TipoToken::CorcheteAbre }); }
            ']' => { iterador.next(); tokens.push(Token { tipo: TipoToken::CorcheteCierra }); }
            '{' => { iterador.next(); tokens.push(Token { tipo: TipoToken::LlaveAbre }); }
            '}' => { iterador.next(); tokens.push(Token { tipo: TipoToken::LlaveCierra }); }
            ',' => { iterador.next(); tokens.push(Token { tipo: TipoToken::Coma }); }
            '"' => {
                iterador.next();
                let mut valor = String::new();
                while let Some(&c) = iterador.peek() {
                    iterador.next();
                    if c == '"' { break; }
                    valor.push(c);
                }
                tokens.push(Token { tipo: TipoToken::Cadena(valor) });
            }
            c if c.is_ascii_digit() => {
                let mut valor = String::new();
                while let Some(&c) = iterador.peek() {
                    if c.is_ascii_digit() || c == '.' { valor.push(c); iterador.next(); } else { break; }
                }
                tokens.push(Token { tipo: TipoToken::Numero(valor) });
            }
            c if c.is_alphabetic() => {
                let mut valor = String::new();
                while let Some(&c) = iterador.peek() {
                    if c.is_alphanumeric() || c == '_' { valor.push(c); iterador.next(); } else { break; }
                }
                tokens.push(Token { tipo: TipoToken::Identificador(valor) });
            }
            _ => { iterador.next(); }
        }
    }
    tokens
}
