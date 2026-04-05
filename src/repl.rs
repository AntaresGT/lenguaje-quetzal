use crate::errores::Resultado;
use crate::interprete::declaraciones::evaluar_declaracion;
use crate::interprete::entorno::Entorno;
use crate::nucleo::sintactico::Parser;
use crate::nucleo::hir::ProgramaHir;
use crate::nativos::registro::registrar_modulos_nativos;

/// Modo REPL interactivo
pub struct Repl {
    entorno: Entorno,
}

impl Repl {
    /// Crea un nuevo REPL
    pub fn nuevo() -> Resultado<Self> {
        let mut entorno = Entorno::nuevo();
        if let Ok(dir_actual) = std::env::current_dir() {
            let mut cargador = entorno.cargador_modulos();
            cargador.establecer_directorio_base(dir_actual);
            entorno.establecer_cargador_modulos(cargador);
        }
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
        
        let programa = ProgramaHir::desde_ast(&ast);
        let mut resultados = Vec::new();

        for item in &programa.items {
            if let Some(nodo) = item.como_nodo_ast() {
                let valor = evaluar_declaracion(nodo, &mut self.entorno)?;
                resultados.push(valor.a_texto());
            }
        }
        
        Ok(resultados.join("\n"))
    }
}
