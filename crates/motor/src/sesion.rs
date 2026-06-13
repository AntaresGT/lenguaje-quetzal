//! Sesión interactiva del motor: estado persistente para el REPL.

use std::path::Path;
use std::rc::Rc;

use bytecode::Instruccion;
use indexmap::IndexMap;
use maquina_virtual::valores::{Variable, texto_de_valor};
use maquina_virtual::{Valor, Vm};
use nucleo::{ErrorQuetzal, Fuente, ResultadoMultiple, ResultadoQuetzal};

use crate::cargador::Cargador;
use crate::configuracion::ConfiguracionMotor;
use crate::motor::MotorQuetzal;

/// Sesión interactiva: evalúa fragmentos sucesivos conservando variables,
/// funciones y objetos entre uno y otro.
pub struct SesionInteractiva {
    vm: Vm,
    /// Variables acumuladas de la sesión.
    globales: IndexMap<String, Variable>,
    /// Número de la entrada actual (para nombrar los fragmentos).
    contador: usize,
    /// Última fuente evaluada, para reportar errores con contexto.
    ultima_fuente: Option<Fuente>,
}

impl SesionInteractiva {
    pub fn nueva(configuracion: ConfiguracionMotor) -> ResultadoQuetzal<Self> {
        let motor = MotorQuetzal::nuevo(configuracion)?;
        Ok(Self {
            vm: Vm::nueva(motor.nativos()),
            globales: IndexMap::new(),
            contador: 0,
            ultima_fuente: None,
        })
    }

    /// Evalúa un fragmento. Si el fragmento termina en una expresión suelta,
    /// devuelve su representación textual para mostrarla.
    ///
    /// En la sesión interactiva no se aplica el análisis semántico completo:
    /// las variables de entradas anteriores no aparecen en el AST del
    /// fragmento actual, así que la validación queda a cargo de la VM.
    pub fn evaluar(&mut self, codigo: &str) -> ResultadoMultiple<Option<String>> {
        self.contador += 1;
        let nombre = format!("<repl:{}>", self.contador);
        let fuente = Fuente::nueva(&nombre, codigo);
        self.ultima_fuente = Some(fuente.clone());

        let ast =
            sintaxis::parsear_modulo(&fuente).map_err(|error| vec![con_archivo(error, &nombre)])?;
        let mut modulo =
            bytecode::generar_modulo(&ast).map_err(|error| vec![con_archivo(error, &nombre)])?;

        // Si el fragmento termina en una expresión suelta, se conserva su
        // valor en la pila para poder mostrarlo.
        let termina_en_expresion = matches!(
            ast.elementos.last(),
            Some(ast::Elemento::Sentencia(sentencia))
                if matches!(sentencia.nodo, ast::NodoSentencia::Expresion(_))
        );
        if termina_en_expresion
            && modulo.principal.instrucciones.last() == Some(&Instruccion::Desechar)
        {
            modulo.principal.instrucciones.pop();
            modulo.principal.ubicaciones.pop();
        }

        let base = std::env::current_dir().ok();
        let importaciones = {
            let mut cargador = Cargador::nuevo(Some(&mut self.vm));
            cargador.resolver_importaciones(&modulo, base.as_deref(), &nombre)?
        };

        let mut globales = self.globales.clone();
        globales.extend(importaciones);

        let (entorno, valor) = self
            .vm
            .cargar_modulo(Rc::new(modulo), globales)
            .map_err(|error| vec![con_archivo(error, &nombre)])?;
        self.globales = entorno.globales.borrow().clone();

        match valor {
            Valor::Nulo => Ok(None),
            otro => Ok(Some(texto_de_valor(&otro))),
        }
    }

    /// Evalúa el contenido de un archivo dentro de la sesión (`:cargar`).
    pub fn cargar_archivo(&mut self, ruta: &str) -> ResultadoMultiple<Option<String>> {
        let ruta = Path::new(ruta);
        let contenido = std::fs::read_to_string(ruta).map_err(|error| {
            vec![ErrorQuetzal::interno(format!(
                "no se pudo leer '{}': {error}",
                ruta.display()
            ))]
        })?;
        self.evaluar(&contenido)
    }

    /// Variables de la sesión: `(nombre, tipo, valor)`.
    pub fn estado(&self) -> Vec<(String, String, String)> {
        self.globales
            .iter()
            .map(|(nombre, variable)| {
                (
                    nombre.clone(),
                    variable.valor.nombre_tipo().to_string(),
                    texto_de_valor(&variable.valor),
                )
            })
            .collect()
    }

    /// Nombres de los módulos cargados en la VM de la sesión.
    pub fn modulos(&self) -> Vec<String> {
        self.vm.modulos.keys().cloned().collect()
    }

    /// Borra todas las variables acumuladas de la sesión.
    pub fn limpiar_estado(&mut self) {
        self.globales.clear();
    }

    /// Fuente de la última evaluación, para reportes con contexto.
    pub fn ultima_fuente(&self) -> Option<&Fuente> {
        self.ultima_fuente.as_ref()
    }
}

fn con_archivo(mut error: ErrorQuetzal, archivo: &str) -> ErrorQuetzal {
    if error.archivo.is_none() {
        error.archivo = Some(archivo.to_string());
    }
    error
}
