//! Guardián de permisos del runtime (seguro por defecto).
//!
//! Los módulos nativos peligrosos (`red`, `sistema_archivos`) consultan este
//! guardián antes de cada operación. Sin un `quetzal.json` que habilite el
//! permiso correspondiente, toda operación se rechaza con un mensaje claro.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use paquetes::{Acceso, Permisos};

/// Permisos efectivos del programa en ejecución.
///
/// Se crea siempre denegando todo y se configura después de leer el
/// `quetzal.json` del proyecto (si existe). Usa mutabilidad interior porque
/// el registro de nativos captura el guardián antes de conocer el proyecto.
#[derive(Default)]
pub struct GuardianPermisos {
    estado: RefCell<Estado>,
}

#[derive(Default)]
struct Estado {
    permisos: Permisos,
    /// Raíz del proyecto, para resolver rutas relativas de `directorios`.
    raiz: Option<PathBuf>,
}

impl GuardianPermisos {
    /// Guardián que deniega todo (sin proyecto / sin permisos declarados).
    pub fn denegado() -> Self {
        Self::default()
    }

    /// Aplica los permisos declarados en el `quetzal.json` del proyecto.
    pub fn configurar(&self, permisos: Permisos, raiz: &Path) {
        let mut estado = self.estado.borrow_mut();
        estado.permisos = permisos;
        estado.raiz = Some(raiz.to_path_buf());
    }

    /// Verifica que el programa pueda usar la red en general (compatibilidad
    /// con el módulo original `red.obtener`/`red.enviar`, sin distinguir
    /// cliente de servidor).
    pub fn verificar_red(&self) -> Result<(), String> {
        if self.estado.borrow().permisos.red.habilitado {
            return Ok(());
        }
        Err(
            "el programa no tiene permiso de red; habilítalo en quetzal.json: \
             \"permisos\": {\"red\": {\"habilitado\": true}}"
                .to_string(),
        )
    }

    /// Verifica que el programa pueda conectarse como cliente a `anfitrion`
    /// (dominio o IP) en `puerto`. Sin "anfitriones"/"puertos" declarados en
    /// `quetzal.json`, cualquier anfitrión o puerto es válido.
    pub fn verificar_red_cliente(&self, anfitrion: &str, puerto: u16) -> Result<(), String> {
        let estado = self.estado.borrow();
        let red = &estado.permisos.red;
        if !red.habilitado || !red.cliente {
            return Err(format!(
                "el programa no tiene permiso para conectarse a '{anfitrion}:{puerto}'; \
                 habilítalo en quetzal.json: \"permisos\": {{\"red\": {{\"habilitado\": true, \
                 \"cliente\": true}}}}"
            ));
        }
        if !red.anfitriones.is_empty() && !red.anfitriones.iter().any(|permitido| permitido == "*" || permitido == anfitrion) {
            return Err(format!(
                "el programa no tiene permiso para conectarse al anfitrión '{anfitrion}'; \
                 agrégalo en quetzal.json: \"permisos\": {{\"red\": {{\"habilitado\": true, \
                 \"cliente\": true, \"anfitriones\": [\"{anfitrion}\"]}}}}"
            ));
        }
        if !red.puertos.is_empty() && !red.puertos.contains(&puerto) {
            return Err(format!(
                "el programa no tiene permiso para usar el puerto {puerto}; agrégalo en \
                 quetzal.json: \"permisos\": {{\"red\": {{\"habilitado\": true, \"cliente\": \
                 true, \"puertos\": [{puerto}]}}}}"
            ));
        }
        Ok(())
    }

    /// Verifica que el programa pueda escuchar como servidor en `puerto`.
    /// Sin "puertos" declarados en `quetzal.json`, cualquier puerto es
    /// válido.
    pub fn verificar_red_servidor(&self, puerto: u16) -> Result<(), String> {
        let estado = self.estado.borrow();
        let red = &estado.permisos.red;
        if !red.habilitado || !red.servidor {
            return Err(format!(
                "el programa no tiene permiso para crear un servidor en el puerto {puerto}; \
                 habilítalo en quetzal.json: \"permisos\": {{\"red\": {{\"habilitado\": true, \
                 \"servidor\": true, \"puertos\": [{puerto}]}}}}"
            ));
        }
        if !red.puertos.is_empty() && !red.puertos.contains(&puerto) {
            return Err(format!(
                "el programa no tiene permiso para escuchar en el puerto {puerto}; agrégalo en \
                 quetzal.json: \"permisos\": {{\"red\": {{\"habilitado\": true, \"servidor\": \
                 true, \"puertos\": [{puerto}]}}}}"
            ));
        }
        Ok(())
    }

    /// Verifica acceso de lectura a una ruta.
    pub fn verificar_lectura(&self, ruta: &str) -> Result<(), String> {
        self.verificar_archivo(ruta, Acceso::Lectura)
    }

    /// Verifica acceso de escritura a una ruta.
    pub fn verificar_escritura(&self, ruta: &str) -> Result<(), String> {
        self.verificar_archivo(ruta, Acceso::Escritura)
    }

    /// Verifica que un ejecutable esté en la lista blanca de `ejecucion`.
    pub fn verificar_ejecucion(&self, programa: &str) -> Result<(), String> {
        let estado = self.estado.borrow();
        let permiso = &estado.permisos.ejecucion;
        if permiso.habilitado
            && permiso
                .ejecutables
                .iter()
                .any(|permitido| permitido == "*" || permitido == programa)
        {
            return Ok(());
        }
        Err(format!(
            "el programa no tiene permiso para ejecutar '{programa}'; agrégalo en quetzal.json: \
             \"permisos\": {{\"ejecucion\": {{\"habilitado\": true, \"ejecutables\": [\"{programa}\"]}}}}"
        ))
    }

    fn verificar_archivo(&self, ruta: &str, requerido: Acceso) -> Result<(), String> {
        let estado = self.estado.borrow();
        let operacion = match requerido {
            Acceso::Lectura => "leer",
            _ => "escribir",
        };
        let denegado = || {
            format!(
                "el programa no tiene permiso para {operacion} '{ruta}'; declara el directorio \
                 en quetzal.json: \"permisos\": {{\"sistema_archivos\": {{\"habilitado\": true, \
                 \"directorios\": [{{\"ruta\": \"./datos\", \"permiso\": \"todo\"}}]}}}}"
            )
        };

        if !estado.permisos.sistema_archivos.habilitado {
            return Err(denegado());
        }

        let objetivo = normalizar_ruta(estado.raiz.as_deref(), Path::new(ruta));
        for directorio in &estado.permisos.sistema_archivos.directorios {
            let permitido = normalizar_ruta(estado.raiz.as_deref(), Path::new(&directorio.ruta));
            let acceso_suficiente =
                directorio.acceso == Acceso::Todo || directorio.acceso == requerido;
            if acceso_suficiente && objetivo.starts_with(&permitido) {
                return Ok(());
            }
        }
        Err(denegado())
    }
}

/// Convierte una ruta a forma absoluta y sin `.`/`..`, sin exigir que exista
/// (la escritura puede crear archivos nuevos).
fn normalizar_ruta(raiz: Option<&Path>, ruta: &Path) -> PathBuf {
    let absoluta = if ruta.is_absolute() {
        ruta.to_path_buf()
    } else {
        let base = raiz
            .map(Path::to_path_buf)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_default();
        base.join(ruta)
    };

    // canonicalize falla si la ruta no existe; en ese caso se canonicaliza el
    // ancestro existente más cercano y se reagregan los componentes restantes.
    let mut componentes_restantes = Vec::new();
    let mut actual = absoluta.clone();
    loop {
        if let Ok(canonica) = actual.canonicalize() {
            let mut resultado = canonica;
            for componente in componentes_restantes.iter().rev() {
                resultado.push(componente);
            }
            return resultado;
        }
        match (actual.parent(), actual.file_name()) {
            (Some(padre), Some(nombre)) => {
                componentes_restantes.push(nombre.to_os_string());
                actual = padre.to_path_buf();
            }
            _ => return absoluta,
        }
    }
}
