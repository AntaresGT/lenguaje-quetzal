//! Implementación del motor de Quetzal.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use maquina_virtual::RegistroNativos;
use nucleo::{CategoriaError, ErrorQuetzal, Fuente, ResultadoMultiple, ResultadoQuetzal};

use crate::cargador::Cargador;
use crate::configuracion::ConfiguracionMotor;

/// Motor del Lenguaje Quetzal.
///
/// Es la única puerta de entrada para ejecutar o revisar código Quetzal,
/// tanto desde la CLI como desde otro programa Rust que use Quetzal como
/// lenguaje de scripting embebido.
pub struct MotorQuetzal {
    configuracion: ConfiguracionMotor,
    nativos: Rc<RegistroNativos>,
    /// Guardián de permisos: deniega todo hasta cargar un quetzal.json.
    permisos: Rc<runtime::GuardianPermisos>,
    /// Fuentes de la última operación, para reportar errores con contexto.
    fuentes: RefCell<HashMap<String, Fuente>>,
}

impl MotorQuetzal {
    /// Crea un motor nuevo con la configuración indicada.
    pub fn nuevo(configuracion: ConfiguracionMotor) -> ResultadoQuetzal<Self> {
        let permisos = Rc::new(runtime::GuardianPermisos::denegado());
        Ok(Self {
            configuracion,
            nativos: Rc::new(modulos_nativos::crear_registro_con_permisos(&permisos)),
            permisos,
            fuentes: RefCell::new(HashMap::new()),
        })
    }

    pub fn configuracion(&self) -> &ConfiguracionMotor {
        &self.configuracion
    }

    /// Fuente cargada en la última operación, por nombre de archivo. Permite
    /// que quien reporta el error muestre la línea con el problema.
    pub fn fuente(&self, nombre: &str) -> Option<Fuente> {
        self.fuentes.borrow().get(nombre).cloned()
    }

    /// Ejecuta un fragmento de código Quetzal. Los imports relativos se
    /// resuelven desde el directorio actual.
    ///
    /// Tras terminar el código principal, mantiene vivo el bucle de eventos
    /// mientras haya trabajo activo (por ejemplo, un `ServidorHttp` en
    /// escucha que aún no llamó a `detener()`), igual que el bucle de
    /// eventos de Node.js sigue corriendo mientras haya servidores abiertos.
    pub fn ejecutar_texto(&self, codigo: &str) -> ResultadoMultiple<()> {
        let fuente = Fuente::nueva("<entrada>", codigo);
        let mut vm = maquina_virtual::Vm::nueva(Rc::clone(&self.nativos));
        let resultado = {
            let mut cargador = Cargador::nuevo(Some(&mut vm));
            let base = std::env::current_dir().ok();
            let resultado = cargador.cargar_fuente(&fuente, base.as_deref());
            self.fuentes.replace(cargador.fuentes);
            resultado
        };
        if resultado.is_ok() {
            vm.drenar_bucle_eventos();
        }
        resultado
    }

    /// Ejecuta un archivo `.qz` o un proyecto (directorio con `quetzal.json`).
    ///
    /// Tras terminar el código principal, mantiene vivo el bucle de eventos
    /// mientras haya trabajo activo (ver [`Self::ejecutar_texto`]).
    pub fn ejecutar_archivo(&self, ruta: &str) -> ResultadoMultiple<()> {
        let (entrada, raiz) = resolver_entrada(ruta).map_err(|error| vec![error])?;
        let mut vm = maquina_virtual::Vm::nueva(Rc::clone(&self.nativos));
        let resultado = {
            let mut cargador = Cargador::nuevo(Some(&mut vm));
            if let Some(raiz) = &raiz {
                // Los permisos del quetzal.json aplican a toda la ejecución.
                let manifiesto = paquetes::Manifiesto::leer_de_directorio(raiz)
                    .map_err(|error| vec![error])?;
                self.permisos.configurar(manifiesto.permisos, raiz);
                cargador = cargador.con_proyecto(raiz);
            }
            let resultado = cargador.cargar_archivo(&entrada);
            self.fuentes.replace(cargador.fuentes);
            resultado
        };
        if resultado.is_ok() {
            vm.drenar_bucle_eventos();
        }
        resultado
    }

    /// Analiza un archivo o proyecto sin ejecutarlo (léxico, sintaxis,
    /// semántica y generación de bytecode de todo el grafo de imports).
    pub fn revisar_archivo(&self, ruta: &str) -> ResultadoMultiple<()> {
        let (entrada, raiz) = resolver_entrada(ruta).map_err(|error| vec![error])?;
        let mut cargador = Cargador::nuevo(None);
        if let Some(raiz) = &raiz {
            cargador = cargador.con_proyecto(raiz);
        }
        let resultado = cargador.cargar_archivo(&entrada);
        self.fuentes.replace(cargador.fuentes);
        resultado
    }

    /// Registro de nativos compartido (para la sesión interactiva).
    pub(crate) fn nativos(&self) -> Rc<RegistroNativos> {
        Rc::clone(&self.nativos)
    }
}

/// Resuelve la entrada real y la raíz de proyecto (si existe):
/// - un `.qz` directo (con raíz si su directorio tiene `quetzal.json`), o
/// - la `entrada` del `quetzal.json` cuando la ruta es un directorio.
fn resolver_entrada(ruta: &str) -> ResultadoQuetzal<(PathBuf, Option<PathBuf>)> {
    let ruta = Path::new(ruta);
    if ruta.is_dir() {
        if !ruta.join("quetzal.json").is_file() {
            return Err(ErrorQuetzal::nuevo(
                "E0601",
                CategoriaError::Paquetes,
                format!("no se encontró 'quetzal.json' en '{}'", ruta.display()),
            )
            .con_ayuda("ejecuta un archivo directo ('quetzal archivo.qz') o crea un proyecto"));
        }
        let manifiesto = paquetes::Manifiesto::leer_de_directorio(ruta)?;
        return Ok((ruta.join(manifiesto.entrada), Some(ruta.to_path_buf())));
    }

    // Archivo directo: si su directorio tiene quetzal.json se considera parte
    // del proyecto (cache y dependencias habilitadas).
    let raiz = ruta
        .parent()
        .filter(|directorio| directorio.join("quetzal.json").is_file())
        .map(Path::to_path_buf);
    Ok((ruta.to_path_buf(), raiz))
}
