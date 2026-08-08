//! Carga de módulos: compila el grafo de imports y lo ejecuta en la VM.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use bytecode::ModuloCompilado;
use indexmap::IndexMap;
use maquina_virtual::valores::{EntornoModulo, Variable};
use maquina_virtual::{Valor, Vm, normalizar_nombre};
use nucleo::{CategoriaError, ErrorQuetzal, Fuente};
use semantica::resolucion_imports::{OrigenImportacion, clasificar_origen};

/// Carga un grafo de módulos: los compila y, si tiene una VM, los ejecuta.
pub(crate) struct Cargador<'vm> {
    /// VM destino; `None` cuando solo se revisa sin ejecutar.
    vm: Option<&'vm mut Vm>,
    /// Módulos ya compilados, por ruta canónica.
    compilados: HashMap<PathBuf, Rc<ModuloCompilado>>,
    /// Entornos ya cargados en la VM, por ruta canónica.
    entornos: HashMap<PathBuf, Rc<EntornoModulo>>,
    /// Rutas en proceso de carga (detección de ciclos).
    cargando: Vec<PathBuf>,
    /// Entorno producido por la última carga (solo al ejecutar).
    ultimo_entorno: Option<Rc<EntornoModulo>>,
    /// Raíz del proyecto (directorio con `quetzal.json`), si existe.
    raiz_proyecto: Option<PathBuf>,
    /// Cache de bytecode del proyecto.
    cache: Option<paquetes::CacheBytecode>,
    /// Fuentes leídas, para reportar diagnósticos con contexto.
    pub fuentes: HashMap<String, Fuente>,
}

impl<'vm> Cargador<'vm> {
    pub fn nuevo(vm: Option<&'vm mut Vm>) -> Self {
        Self {
            vm,
            compilados: HashMap::new(),
            entornos: HashMap::new(),
            cargando: Vec::new(),
            ultimo_entorno: None,
            raiz_proyecto: None,
            cache: None,
            fuentes: HashMap::new(),
        }
    }

    /// Activa la raíz de proyecto: habilita la cache de bytecode y la
    /// resolución de dependencias instaladas en `.quetzal/bibliotecas/`.
    pub fn con_proyecto(mut self, raiz: &Path) -> Self {
        self.cache = Some(paquetes::CacheBytecode::del_proyecto(raiz));
        self.raiz_proyecto = Some(raiz.to_path_buf());
        self
    }

    /// Compila (y ejecuta, si corresponde) el archivo y todo su grafo de imports.
    pub fn cargar_archivo(&mut self, ruta: &Path) -> Result<(), Vec<ErrorQuetzal>> {
        let canonica = ruta.canonicalize().map_err(|_| {
            vec![
                ErrorQuetzal::nuevo(
                    "E0301",
                    CategoriaError::Modulos,
                    format!("no se encontró el archivo '{}'", ruta.display()),
                )
                .con_ayuda("verifica que la ruta exista y termine en '.qz'"),
            ]
        })?;

        if self.compilados.contains_key(&canonica) {
            return Ok(());
        }
        if self.cargando.contains(&canonica) {
            return Err(vec![
                ErrorQuetzal::nuevo(
                    "E0303",
                    CategoriaError::Modulos,
                    format!("importación circular detectada en '{}'", ruta.display()),
                )
                .con_ayuda("rompe el ciclo moviendo el código compartido a un tercer módulo"),
            ]);
        }

        let contenido = std::fs::read_to_string(&canonica).map_err(|error| {
            vec![ErrorQuetzal::nuevo(
                "E0301",
                CategoriaError::Modulos,
                format!("no se pudo leer '{}': {error}", ruta.display()),
            )]
        })?;
        let fuente = Fuente::nueva(nombre_visible(ruta), contenido);

        self.cargando.push(canonica.clone());
        let resultado = self.cargar_fuente_interno(&fuente, canonica.parent());
        self.cargando.pop();

        let modulo = resultado?;
        self.compilados.insert(canonica.clone(), modulo);
        if let Some(entorno) = self.ultimo_entorno.take() {
            self.entornos.insert(canonica, entorno);
        }
        Ok(())
    }

    /// Compila (y ejecuta) un fragmento suelto, por ejemplo del REPL o de
    /// `ejecutar_texto`. Los imports relativos se resuelven desde `base`.
    pub fn cargar_fuente(
        &mut self,
        fuente: &Fuente,
        base: Option<&Path>,
    ) -> Result<(), Vec<ErrorQuetzal>> {
        self.cargar_fuente_interno(fuente, base).map(|_| ())
    }

    fn cargar_fuente_interno(
        &mut self,
        fuente: &Fuente,
        base: Option<&Path>,
    ) -> Result<Rc<ModuloCompilado>, Vec<ErrorQuetzal>> {
        self.fuentes.insert(fuente.nombre.clone(), fuente.clone());

        // La cache evita re-analizar y re-generar bytecode si la fuente, la
        // versión de Quetzal y el formato no cambiaron.
        let en_cache = self
            .cache
            .as_ref()
            .and_then(|cache| cache.buscar(&fuente.contenido))
            // El nombre visible puede cambiar (rutas relativas distintas).
            .map(|mut modulo| {
                modulo.nombre = fuente.nombre.clone();
                modulo
            });
        let modulo = match en_cache {
            Some(modulo) => Rc::new(modulo),
            None => {
                let modulo = Rc::new(compilar(fuente)?);
                if let Some(cache) = &self.cache {
                    cache.guardar(&fuente.contenido, &modulo);
                }
                modulo
            }
        };
        let importaciones = self.resolver_importaciones(&modulo, base, &fuente.nombre)?;

        self.ultimo_entorno = None;
        if let Some(vm) = &mut self.vm {
            let (entorno, _valor) = vm
                .cargar_modulo(Rc::clone(&modulo), importaciones)
                .map_err(|error| vec![con_archivo_si_falta(error, &fuente.nombre)])?;
            self.ultimo_entorno = Some(entorno);
        }
        Ok(modulo)
    }

    /// Resuelve las importaciones de un módulo ya compilado: nativas, rutas
    /// relativas (cargándolas primero) y dependencias.
    pub fn resolver_importaciones(
        &mut self,
        modulo: &ModuloCompilado,
        base: Option<&Path>,
        archivo: &str,
    ) -> Result<IndexMap<String, Variable>, Vec<ErrorQuetzal>> {
        let mut importaciones = IndexMap::new();

        for importacion in &modulo.importaciones {
            match clasificar_origen(&importacion.origen) {
                Some(OrigenImportacion::Nativo(nombre_modulo)) => {
                    for (nombre, local) in &importacion.simbolos {
                        // `SistemaArchivos` y `sistema_archivos` refieren al módulo.
                        let normalizado = normalizar_nombre(nombre).to_lowercase().replace('_', "");
                        let valor = if normalizado == nombre_modulo.replace('_', "") {
                            Valor::ModuloNativo(Rc::from(nombre_modulo.as_str()))
                        } else if let Some(tipo) =
                            // Tipos instanciables exportados por el módulo
                            // (`Archivo` y `Ruta` desde `sistema_archivos`).
                            modulos_nativos::modulo_de_tipo_exportado(
                                nombre_modulo.as_str(),
                                &normalizado,
                            )
                        {
                            Valor::ModuloNativo(Rc::from(tipo))
                        } else {
                            Valor::Nativa(Rc::from(format!("{nombre_modulo}.{nombre}").as_str()))
                        };
                        importaciones.insert(
                            local.clone(),
                            Variable {
                                valor,
                                mutable: false,
                            },
                        );
                    }
                }
                Some(OrigenImportacion::RutaRelativa(relativa)) => {
                    let destino = match base {
                        Some(directorio) => directorio.join(&relativa),
                        None => PathBuf::from(&relativa),
                    };
                    self.cargar_archivo(&destino)?;
                    let canonica = destino.canonicalize().map_err(|_| {
                        vec![
                            ErrorQuetzal::nuevo(
                                "E0301",
                                CategoriaError::Modulos,
                                format!("no se encontró el módulo '{relativa}'"),
                            )
                            .con_archivo(archivo),
                        ]
                    })?;
                    self.tomar_simbolos(&canonica, importacion, &mut importaciones, archivo)?;
                }
                Some(OrigenImportacion::Dependencia(nombre)) => {
                    let entrada = self.entrada_de_dependencia(&nombre, archivo)?;
                    self.cargar_archivo(&entrada)?;
                    let canonica = entrada.canonicalize().map_err(|_| {
                        vec![
                            ErrorQuetzal::nuevo(
                                "E0301",
                                CategoriaError::Modulos,
                                format!("no se encontró la entrada de la dependencia '{nombre}'"),
                            )
                            .con_archivo(archivo),
                        ]
                    })?;
                    self.tomar_simbolos(&canonica, importacion, &mut importaciones, archivo)?;
                }
                None => {
                    return Err(vec![
                        ErrorQuetzal::nuevo(
                            "E0301",
                            CategoriaError::Modulos,
                            format!("no existe el módulo nativo '{}'", importacion.origen),
                        )
                        .con_archivo(archivo)
                        .con_ayuda(
                            "los módulos nativos disponibles son: quetzal/matematica, \
                             quetzal/texto, quetzal/listas, quetzal/jsn, quetzal/tiempo, \
                             quetzal/red, quetzal/sistema_archivos, quetzal/bits y quetzal/motor",
                        ),
                    ]);
                }
            }
        }

        Ok(importaciones)
    }

    /// Localiza el archivo de entrada de una dependencia instalada en
    /// `.quetzal/bibliotecas/<nombre>/<version>/`.
    fn entrada_de_dependencia(
        &self,
        nombre: &str,
        archivo: &str,
    ) -> Result<PathBuf, Vec<ErrorQuetzal>> {
        let error_no_instalada = || {
            vec![
                ErrorQuetzal::nuevo(
                    "E0602",
                    CategoriaError::Paquetes,
                    format!("la dependencia '{nombre}' no está instalada"),
                )
                .con_archivo(archivo)
                .con_ayuda("declárala en quetzal.json y ejecuta 'quetzal instalar'"),
            ]
        };

        let raiz = self.raiz_proyecto.as_ref().ok_or_else(error_no_instalada)?;
        let directorio_dependencia = raiz.join(".quetzal").join("bibliotecas").join(nombre);
        if !directorio_dependencia.is_dir() {
            return Err(error_no_instalada());
        }

        // Se usa la versión instalada (la primera si hubiera varias).
        let version = std::fs::read_dir(&directorio_dependencia)
            .ok()
            .and_then(|mut entradas| entradas.next())
            .and_then(|entrada| entrada.ok())
            .map(|entrada| entrada.path())
            .ok_or_else(error_no_instalada)?;

        let manifiesto =
            paquetes::Manifiesto::leer_de_directorio(&version).map_err(|error| vec![error])?;
        Ok(version.join(manifiesto.entrada))
    }

    fn tomar_simbolos(
        &self,
        canonica: &Path,
        importacion: &bytecode::ImportacionCompilada,
        destino: &mut IndexMap<String, Variable>,
        archivo: &str,
    ) -> Result<(), Vec<ErrorQuetzal>> {
        let Some(compilado) = self.compilados.get(canonica) else {
            return Err(vec![ErrorQuetzal::interno(
                "el módulo importado no quedó registrado tras compilarse",
            )]);
        };

        let mut errores = Vec::new();
        for (nombre, local) in &importacion.simbolos {
            if !compilado.exportaciones.contains(nombre) {
                errores.push(
                    ErrorQuetzal::nuevo(
                        "E0302",
                        CategoriaError::Modulos,
                        format!("el módulo '{}' no exporta '{nombre}'", compilado.nombre),
                    )
                    .con_archivo(archivo)
                    .con_ayuda(format!(
                        "agrega 'exportar {{ {nombre} }}' en '{}'",
                        compilado.nombre
                    )),
                );
                continue;
            }
            // Solo hay valores reales cuando se está ejecutando.
            if let Some(entorno) = self.entornos.get(canonica)
                && let Some(variable) = entorno.globales.borrow().get(nombre)
            {
                destino.insert(local.clone(), variable.clone());
            }
        }

        if errores.is_empty() {
            Ok(())
        } else {
            Err(errores)
        }
    }
}

/// Pipeline de compilación de un archivo: léxico → sintaxis → semántica → bytecode.
fn compilar(fuente: &Fuente) -> Result<ModuloCompilado, Vec<ErrorQuetzal>> {
    let ast = sintaxis::parsear_modulo(fuente)
        .map_err(|error| vec![con_archivo_si_falta(error, &fuente.nombre)])?;

    semantica::analizar_modulo(&ast).map_err(|errores| {
        errores
            .into_iter()
            .map(|error| con_archivo_si_falta(error, &fuente.nombre))
            .collect::<Vec<_>>()
    })?;

    bytecode::generar_modulo(&ast)
        .map_err(|error| vec![con_archivo_si_falta(error, &fuente.nombre)])
}

fn con_archivo_si_falta(mut error: ErrorQuetzal, archivo: &str) -> ErrorQuetzal {
    if error.archivo.is_none() {
        error.archivo = Some(archivo.to_string());
    }
    error
}

/// Nombre del archivo como se muestra en diagnósticos: relativo al directorio
/// actual cuando es posible.
fn nombre_visible(ruta: &Path) -> String {
    let actual = std::env::current_dir().unwrap_or_default();
    ruta.strip_prefix(&actual)
        .unwrap_or(ruta)
        .display()
        .to_string()
        .replace('\\', "/")
}
