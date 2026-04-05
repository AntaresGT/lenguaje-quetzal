use crate::errores::{CodigoError, Error, Resultado};
use crate::interprete::valores::Valor;
use crate::modulos::manifiesto::ManifiestoPaquete;
use crate::nucleo::hir::ProgramaHir;
use crate::nucleo::sintactico::ast::NodoAst;
use crate::nucleo::sintactico::Parser;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct ModuloCompilado {
    pub ruta_absoluta: PathBuf,
    pub ast: Vec<NodoAst>,
    pub hir: ProgramaHir,
    pub exportaciones: HashMap<String, NodoAst>,
    pub nombres_exportados: Vec<String>,
    pub manifiesto: Option<ManifiestoPaquete>,
}

#[derive(Debug, Default)]
struct EstadoCargador {
    modulos: HashMap<PathBuf, ModuloCompilado>,
    modulos_ejecutados: HashMap<PathBuf, HashMap<String, Valor>>,
    en_proceso: HashSet<PathBuf>,
}

/// Cargador de módulos con caché compartido y soporte de manifiesto de paquete
#[derive(Clone, Default)]
pub struct CargadorModulos {
    estado: Arc<Mutex<EstadoCargador>>,
    directorio_base: Option<PathBuf>,
    manifiesto_actual: Option<ManifiestoPaquete>,
}

impl CargadorModulos {
    /// Crea un nuevo cargador de módulos
    pub fn nuevo() -> Self {
        Self::default()
    }

    /// Crea un cargador con directorio base específico
    pub fn con_directorio_base(directorio: PathBuf) -> Self {
        let manifiesto_actual = ManifiestoPaquete::buscar_desde_directorio(&directorio);
        Self {
            estado: Arc::new(Mutex::new(EstadoCargador::default())),
            directorio_base: Some(directorio),
            manifiesto_actual,
        }
    }

    /// Crea un cargador a partir del archivo actual
    pub fn desde_archivo_actual(ruta_archivo: &Path) -> Self {
        let directorio_base = ruta_archivo.parent().map(Path::to_path_buf);
        let manifiesto_actual = directorio_base
            .as_deref()
            .and_then(ManifiestoPaquete::buscar_desde_directorio);

        Self {
            estado: Arc::new(Mutex::new(EstadoCargador::default())),
            directorio_base,
            manifiesto_actual,
        }
    }

    /// Establece el directorio base para resolución relativa
    pub fn establecer_directorio_base(&mut self, directorio: PathBuf) {
        self.manifiesto_actual = ManifiestoPaquete::buscar_desde_directorio(&directorio);
        self.directorio_base = Some(directorio);
    }

    /// Establece el manifiesto del paquete actual
    pub fn establecer_manifiesto_actual(&mut self, manifiesto: Option<ManifiestoPaquete>) {
        self.manifiesto_actual = manifiesto;
    }

    fn normalizar_ruta_modulo(ruta: &str) -> String {
        if ruta.ends_with(".qz") {
            ruta.to_string()
        } else {
            format!("{}.qz", ruta)
        }
    }

    fn canonicalizar_si_existe(ruta: PathBuf) -> PathBuf {
        if ruta.exists() {
            ruta.canonicalize().unwrap_or(ruta)
        } else {
            ruta
        }
    }

    fn resolver_dependencia(&self, ruta: &str) -> Resultado<Option<PathBuf>> {
        let Some(manifiesto) = self.manifiesto_actual.as_ref() else {
            return Ok(None);
        };
        let (dependencia, resto) = if let Some((dependencia, resto)) = ruta.split_once('/') {
            (dependencia, Some(resto))
        } else if let Some((dependencia, resto)) = ruta.split_once('\\') {
            (dependencia, Some(resto))
        } else {
            (ruta, None)
        };

        let Some(base) = manifiesto.resolver_dependencia(dependencia)? else {
            return Ok(None);
        };
        if let Some(resto) = resto {
            if base.is_dir() {
                let resto = Self::normalizar_ruta_modulo(resto);
                for candidato in [base.join(&resto), base.join("src").join(&resto)] {
                    if candidato.exists() {
                        return Ok(Some(Self::canonicalizar_si_existe(candidato)));
                    }
                }
            }
            return Ok(None);
        }

        Ok(Some(Self::canonicalizar_si_existe(base)))
    }

    /// Resuelve una ruta de módulo a ruta absoluta
    pub fn resolver_ruta(&self, ruta: &str) -> Resultado<PathBuf> {
        let ruta_con_extension = Self::normalizar_ruta_modulo(ruta);
        let path = Path::new(&ruta_con_extension);

        if path.is_absolute() {
            return Ok(Self::canonicalizar_si_existe(path.to_path_buf()));
        }

        if let Some(ruta_dependencia) = self.resolver_dependencia(ruta)? {
            return Ok(ruta_dependencia);
        }

        if let Some(ref base) = self.directorio_base {
            let ruta_completa = base.join(&ruta_con_extension);
            if ruta_completa.exists() {
                return Ok(Self::canonicalizar_si_existe(ruta_completa));
            }
        }

        let ruta_actual = PathBuf::from(&ruta_con_extension);
        if ruta_actual.exists() {
            return Ok(Self::canonicalizar_si_existe(ruta_actual));
        }

        Err(Error::modulo(
            CodigoError::ModuloNoEncontrado,
            format!("no se encontró el módulo '{}'", ruta),
            Some(ruta.to_string()),
        ))
    }

    fn compilar_modulo_desde_ruta(&self, ruta_absoluta: &Path, ruta_original: &str) -> Resultado<ModuloCompilado> {
        let contenido = fs::read_to_string(ruta_absoluta).map_err(|e| {
            Error::modulo(
                CodigoError::ErrorCargarModulo,
                format!("no se pudo leer el archivo '{}': {}", ruta_absoluta.display(), e),
                Some(ruta_original.to_string()),
            )
        })?;

        let ast = Parser::parsear(&contenido)?;
        let hir = ProgramaHir::desde_ast(&ast).con_ruta(ruta_absoluta.to_path_buf());
        let exportaciones = hir.exportaciones_ast();
        let nombres_exportados = hir
            .exportaciones
            .simbolos
            .iter()
            .filter_map(|id| hir.resolver_nombre(*id).map(|nombre| nombre.to_string()))
            .collect();
        let manifiesto = ruta_absoluta
            .parent()
            .map(ManifiestoPaquete::buscar_desde_directorio_resultado)
            .transpose()?
            .flatten();

        Ok(ModuloCompilado {
            ruta_absoluta: ruta_absoluta.to_path_buf(),
            ast,
            hir,
            exportaciones,
            nombres_exportados,
            manifiesto,
        })
    }

    /// Compila un módulo y lo deja en caché
    pub fn obtener_modulo_compilado(&self, ruta: &str) -> Resultado<ModuloCompilado> {
        let ruta_absoluta = self.resolver_ruta(ruta)?;

        {
            let mut estado = self.estado.lock().unwrap();
            if let Some(modulo) = estado.modulos.get(&ruta_absoluta) {
                return Ok(modulo.clone());
            }

            if !estado.en_proceso.insert(ruta_absoluta.clone()) {
                return Err(Error::modulo(
                    CodigoError::DependenciaCircular,
                    format!("dependencia circular detectada al cargar '{}'", ruta),
                    Some(ruta.to_string()),
                ));
            }
        }

        let resultado = self.compilar_modulo_desde_ruta(&ruta_absoluta, ruta);

        let mut estado = self.estado.lock().unwrap();
        estado.en_proceso.remove(&ruta_absoluta);
        if let Ok(modulo) = &resultado {
            estado.modulos.insert(ruta_absoluta.clone(), modulo.clone());
        }

        resultado
    }

    /// Carga el AST de un módulo
    pub fn cargar_modulo(&self, ruta: &str) -> Resultado<Vec<NodoAst>> {
        self.obtener_modulo_compilado(ruta).map(|modulo| modulo.ast)
    }

    /// Carga la HIR de un módulo
    pub fn cargar_hir_modulo(&self, ruta: &str) -> Resultado<ProgramaHir> {
        self.obtener_modulo_compilado(ruta).map(|modulo| modulo.hir)
    }

    /// Obtiene las exportaciones con soporte semántico del módulo
    pub fn obtener_exportaciones(&self, ruta: &str) -> Resultado<HashMap<String, NodoAst>> {
        self.obtener_modulo_compilado(ruta)
            .map(|modulo| modulo.exportaciones)
    }

    /// Obtiene elementos específicos exportados de un módulo
    pub fn obtener_elementos(&self, ruta: &str, elementos: &[String]) -> Resultado<HashMap<String, NodoAst>> {
        let modulo = self.obtener_modulo_compilado(ruta)?;
        let mut resultado = HashMap::new();

        for elemento in elementos {
            if let Some(nodo) = modulo.exportaciones.get(elemento) {
                resultado.insert(elemento.clone(), nodo.clone());
            } else {
                return Err(Error::modulo(
                    CodigoError::ElementoImportadoNoEncontrado,
                    format!("'{}' no está exportado en el módulo '{}'", elemento, ruta),
                    Some(ruta.to_string()),
                ));
            }
        }

        Ok(resultado)
    }

    pub fn nombres_exportados(&self, ruta: &str) -> Resultado<Vec<String>> {
        self.obtener_modulo_compilado(ruta)
            .map(|modulo| modulo.nombres_exportados)
    }

    /// Verifica si un módulo ya está cargado
    pub fn esta_cargado(&self, ruta: &str) -> bool {
        if let Ok(ruta_absoluta) = self.resolver_ruta(ruta) {
            let estado = self.estado.lock().unwrap();
            estado.modulos.contains_key(&ruta_absoluta)
        } else {
            false
        }
    }

    pub fn obtener_exportaciones_ejecutadas(&self, ruta_absoluta: &Path) -> Option<HashMap<String, Valor>> {
        let estado = self.estado.lock().unwrap();
        estado.modulos_ejecutados.get(ruta_absoluta).cloned()
    }

    pub fn guardar_exportaciones_ejecutadas(&self, ruta_absoluta: PathBuf, exportaciones: HashMap<String, Valor>) {
        let mut estado = self.estado.lock().unwrap();
        estado.modulos_ejecutados.insert(ruta_absoluta, exportaciones);
    }
}
