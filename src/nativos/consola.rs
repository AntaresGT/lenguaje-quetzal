use crate::errores::{Error, CodigoError, Resultado};
use crate::interprete::entorno::Entorno;
use crate::interprete::valores::Valor;
use crate::nativos::interfaz::ModuloNativo;
use colored::*;

/// Módulo nativo de consola
pub struct Consola;

impl Consola {
    pub fn nuevo() -> Self {
        Self
    }
}

impl ModuloNativo for Consola {
    fn nombre(&self) -> &str {
        "consola"
    }
    
    fn ruta(&self) -> &str {
        "quetzal/consola"
    }
    
    fn registrar(&self, _entorno: &mut Entorno) -> Resultado<()> {
        // Consola se registra como objeto global, no como módulo importable
        // Esto se hace en registrar_modulos_nativos
        Ok(())
    }

    fn obtener_tipo_constante(&self, _nombre: &str) -> Option<crate::nucleo::semantico::tipos::Tipo> {
        None
    }
    
    fn obtener_constante(&self, _nombre: &str) -> Option<Valor> {
        None
    }
    
    fn llamar_funcion(
        &self,
        nombre: &str,
        argumentos: Vec<Valor>,
        _entorno: &Entorno,
    ) -> Resultado<Valor> {
        match nombre {
            "mostrar" => {
                if let Some(Valor::Texto(mensaje)) = argumentos.get(0) {
                    println!("{}", mensaje);
                } else if let Some(valor) = argumentos.get(0) {
                    println!("{}", valor.a_texto());
                }
                Ok(Valor::Vacio)
            }
            "mostrar_error" => {
                if let Some(Valor::Texto(mensaje)) = argumentos.get(0) {
                    eprintln!("{}", mensaje.red());
                }
                Ok(Valor::Vacio)
            }
            "mostrar_advertencia" => {
                if let Some(Valor::Texto(mensaje)) = argumentos.get(0) {
                    eprintln!("{}", mensaje.yellow());
                }
                Ok(Valor::Vacio)
            }
            "mostrar_exito" => {
                if let Some(Valor::Texto(mensaje)) = argumentos.get(0) {
                    println!("{}", mensaje.green());
                }
                Ok(Valor::Vacio)
            }
            "mostrar_informacion" => {
                if let Some(Valor::Texto(mensaje)) = argumentos.get(0) {
                    println!("{}", mensaje.blue());
                }
                Ok(Valor::Vacio)
            }
            "pedir" => {
                let prompt = if let Some(Valor::Texto(mensaje)) = argumentos.get(0) {
                    mensaje.clone()
                } else if let Some(valor) = argumentos.get(0) {
                    valor.a_texto()
                } else {
                    "".to_string()
                };
                
                print!("{}", prompt);
                use std::io::{self, Write};
                io::stdout().flush().unwrap_or(());
                
                let mut entrada = String::new();
                io::stdin().read_line(&mut entrada).unwrap_or(0);
                // Remover el salto de línea final
                entrada = entrada.trim_end().to_string();
                
                Ok(Valor::Texto(entrada))
            }
            "pedir_secreto" => {
                let prompt = if let Some(Valor::Texto(mensaje)) = argumentos.get(0) {
                    mensaje.clone()
                } else if let Some(valor) = argumentos.get(0) {
                    valor.a_texto()
                } else {
                    "".to_string()
                };
                
                use std::io::{self, Write};
                print!("{}", prompt);
                io::stdout().flush().unwrap_or(());
                
                // Usar rpassword para ocultar la entrada
                match rpassword::read_password() {
                    Ok(entrada) => Ok(Valor::Texto(entrada)),
                    Err(_) => {
                        // Fallback a lectura normal si rpassword falla
                        let mut entrada = String::new();
                        io::stdin().read_line(&mut entrada).unwrap_or(0);
                        Ok(Valor::Texto(entrada.trim_end().to_string()))
                    }
                }
            }
            _ => Err(Error::ejecucion(
                CodigoError::FuncionNoDeclarada,
                format!("función '{}' no encontrada en módulo consola", nombre),
                None,
                None,
                None,
            )),
        }
    }
}
