use crate::errores::Resultado;
use crate::interprete::entorno::Entorno;
use crate::interprete::declaraciones::evaluar_declaracion;
use crate::nucleo::sintactico::Parser;
use crate::nativos::registro::registrar_modulos_nativos;

/// Modo REPL interactivo
pub struct Repl {
    entorno: Entorno,
}

impl Repl {
    /// Crea un nuevo REPL
    pub fn nuevo() -> Resultado<Self> {
        let mut entorno = Entorno::nuevo();
        registrar_modulos_nativos(&mut entorno)?;
        
        Ok(Self {
            entorno,
        })
    }
    
    /// Evalúa una línea de código
    pub fn evaluar(&mut self, codigo: &str) -> Resultado<String> {
        let ast = Parser::parsear(codigo)?;
        
        if ast.is_empty() {
            return Ok(String::new());
        }
        
        let mut resultados = Vec::new();
        for nodo in &ast {
            let valor = evaluar_declaracion(nodo, &mut self.entorno)?;
            resultados.push(valor.a_texto());
        }
        
        Ok(resultados.join("\n"))
    }
}
