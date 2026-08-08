//! Guardián de permisos del runtime (seguro por defecto).
//!
//! Los módulos nativos peligrosos (`red`, `sistema_archivos`) consultan este
//! guardián antes de cada operación. Sin un `quetzal.json` que habilite el
//! permiso correspondiente, toda operación se rechaza con un mensaje claro.

use std::cell::RefCell;
use std::path::{Component, Path, PathBuf};

use paquetes::{Acceso, Permisos};

/// Nivel de acceso que necesita una operación del sistema de archivos.
///
/// `lectura` cubre listar, leer y consultar metadatos; `escritura` cubre
/// crear, escribir, copiar, mover, renombrar y borrar. El nivel `todo` de
/// `quetzal.json` concede ambos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccesoSolicitado {
    Lectura,
    Escritura,
}

impl AccesoSolicitado {
    /// Si un directorio declarado con `acceso` autoriza esta operación.
    fn autorizado_por(self, acceso: Acceso) -> bool {
        match self {
            AccesoSolicitado::Lectura => matches!(acceso, Acceso::Lectura | Acceso::Todo),
            AccesoSolicitado::Escritura => matches!(acceso, Acceso::Escritura | Acceso::Todo),
        }
    }

    fn nombre(self) -> &'static str {
        match self {
            AccesoSolicitado::Lectura => "lectura",
            AccesoSolicitado::Escritura => "escritura",
        }
    }
}

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
    /// Raíz del proyecto: base de las rutas relativas del programa y de los
    /// directorios declarados en `quetzal.json`.
    raiz: Option<PathBuf>,
}

impl GuardianPermisos {
    /// Guardián que deniega todo (sin proyecto / sin permisos declarados).
    pub fn denegado() -> Self {
        Self::default()
    }

    /// Aplica los permisos declarados en el `quetzal.json` del proyecto.
    ///
    /// La raíz se guarda absoluta: así toda ruta verificada sale absoluta y
    /// volver a verificar una ruta ya resuelta da el mismo resultado (lo
    /// hacen `renombrar` y `mover`, que derivan el destino del origen).
    pub fn configurar(&self, permisos: Permisos, raiz: &Path) {
        let mut estado = self.estado.borrow_mut();
        estado.permisos = permisos;
        estado.raiz = Some(absoluta(raiz));
    }

    /// Verifica que el programa pueda usar la red (conectarse o escuchar).
    ///
    /// `operacion` describe lo que se intentaba hacer para que el mensaje de
    /// error diga exactamente qué quedó bloqueado.
    pub fn verificar_red(&self, operacion: &str) -> Result<(), String> {
        if self.estado.borrow().permisos.red.habilitado {
            return Ok(());
        }
        Err(format!(
            "el programa no tiene permiso de red para {operacion}; habilítalo en quetzal.json: \
             \"permisos\": [{{\"tipo\": \"red\", \"habilitado\": true}}]"
        ))
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

    /// Verifica que se pueda **leer** la ruta y devuelve su forma absoluta.
    pub fn verificar_lectura(&self, ruta: &str) -> Result<PathBuf, String> {
        self.verificar_ruta(ruta, AccesoSolicitado::Lectura)
    }

    /// Verifica que se pueda **escribir** la ruta y devuelve su forma
    /// absoluta. Cubre crear, escribir, copiar, mover, renombrar y borrar.
    pub fn verificar_escritura(&self, ruta: &str) -> Result<PathBuf, String> {
        self.verificar_ruta(ruta, AccesoSolicitado::Escritura)
    }

    /// Comprueba una ruta contra los directorios declarados en
    /// `quetzal.json` y la devuelve resuelta respecto a la raíz del proyecto.
    pub fn verificar_ruta(
        &self,
        ruta: &str,
        solicitado: AccesoSolicitado,
    ) -> Result<PathBuf, String> {
        let estado = self.estado.borrow();
        let permiso = &estado.permisos.sistema_archivos;
        if !permiso.habilitado {
            return Err(format!(
                "el programa no tiene permiso de sistema de archivos para '{ruta}'; habilítalo en \
                 quetzal.json: \"permisos\": [{{\"tipo\": \"sistema-archivos\", \"habilitado\": true, \
                 \"directorios\": [{{\"ruta\": \"./\", \"permiso\": \"{}\"}}]}}]",
                solicitado.nombre()
            ));
        }

        let raiz = estado
            .raiz
            .clone()
            .unwrap_or_else(|| absoluta(Path::new(".")));
        let absoluta = resolver(&raiz, Path::new(ruta));

        let mut existe_directorio = false;
        for directorio in &permiso.directorios {
            let coincide = if directorio.ruta == "*" {
                true
            } else {
                absoluta.starts_with(resolver(&raiz, Path::new(&directorio.ruta)))
            };
            if !coincide {
                continue;
            }
            existe_directorio = true;
            if solicitado.autorizado_por(directorio.acceso) {
                return Ok(absoluta);
            }
        }

        if existe_directorio {
            Err(format!(
                "el programa no tiene permiso de {} sobre '{ruta}'; cambia el permiso del \
                 directorio en quetzal.json a \"{}\" o \"todo\"",
                solicitado.nombre(),
                solicitado.nombre()
            ))
        } else {
            Err(format!(
                "la ruta '{ruta}' está fuera de los directorios permitidos; agrégala en \
                 quetzal.json: \"directorios\": [{{\"ruta\": \"{ruta}\", \"permiso\": \"{}\"}}]",
                solicitado.nombre()
            ))
        }
    }
}

/// Resuelve una ruta contra la raíz del proyecto y la limpia léxicamente
/// (`.` y `..`), sin tocar el disco: la ruta puede no existir todavía (por
/// ejemplo, el archivo que se está por crear).
fn resolver(raiz: &Path, ruta: &Path) -> PathBuf {
    let unida = if ruta.is_absolute() {
        ruta.to_path_buf()
    } else {
        raiz.join(ruta)
    };
    limpiar(&unida)
}

/// Ruta absoluta respecto al directorio actual, limpia de `.` y `..`.
fn absoluta(ruta: &Path) -> PathBuf {
    if ruta.is_absolute() {
        return limpiar(ruta);
    }
    let actual = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    limpiar(&actual.join(ruta))
}

/// Elimina `.` y `..` sin tocar el disco: la ruta puede no existir todavía.
fn limpiar(ruta: &Path) -> PathBuf {
    let mut limpia = PathBuf::new();
    for componente in ruta.components() {
        match componente {
            Component::CurDir => {}
            Component::ParentDir => {
                limpia.pop();
            }
            otro => limpia.push(otro.as_os_str()),
        }
    }
    limpia
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn guardian_con(json: &str, raiz: &Path) -> GuardianPermisos {
        let valor: serde_json::Value = serde_json::from_str(json).expect("JSON de prueba válido");
        let permisos = Permisos::desde_json(&valor).expect("permisos de prueba válidos");
        let guardian = GuardianPermisos::denegado();
        guardian.configurar(permisos, raiz);
        guardian
    }

    fn raiz() -> PathBuf {
        if cfg!(windows) {
            PathBuf::from("C:\\proyecto")
        } else {
            PathBuf::from("/proyecto")
        }
    }

    #[test]
    fn sin_permiso_habilitado_se_deniega_todo() {
        let guardian = guardian_con(r#"{"sistema_archivos": {"habilitado": false}}"#, &raiz());
        let error = guardian
            .verificar_lectura("./datos.txt")
            .expect_err("sin permiso no se puede leer");
        assert!(error.contains("no tiene permiso de sistema de archivos"));
    }

    #[test]
    fn lectura_autorizada_devuelve_ruta_absoluta() {
        let guardian = guardian_con(
            r#"{"sistema_archivos": {"habilitado": true,
                "directorios": [{"ruta": "./datos", "permiso": "lectura"}]}}"#,
            &raiz(),
        );
        let ruta = guardian
            .verificar_lectura("./datos/entrada.txt")
            .expect("el directorio declarado permite lectura");
        assert_eq!(ruta, raiz().join("datos").join("entrada.txt"));
    }

    #[test]
    fn lectura_no_autoriza_escritura() {
        let guardian = guardian_con(
            r#"{"sistema_archivos": {"habilitado": true,
                "directorios": [{"ruta": "./datos", "permiso": "lectura"}]}}"#,
            &raiz(),
        );
        let error = guardian
            .verificar_escritura("./datos/salida.txt")
            .expect_err("lectura no autoriza escribir");
        assert!(error.contains("no tiene permiso de escritura"));
    }

    #[test]
    fn todo_autoriza_lectura_y_escritura() {
        let guardian = guardian_con(
            r#"{"sistema_archivos": {"habilitado": true,
                "directorios": [{"ruta": "./trabajo", "permiso": "todo"}]}}"#,
            &raiz(),
        );
        assert!(guardian.verificar_lectura("./trabajo/a.txt").is_ok());
        assert!(guardian.verificar_escritura("./trabajo/a.txt").is_ok());
    }

    #[test]
    fn ruta_fuera_de_los_directorios_declarados_se_rechaza() {
        let guardian = guardian_con(
            r#"{"sistema_archivos": {"habilitado": true,
                "directorios": [{"ruta": "./datos", "permiso": "todo"}]}}"#,
            &raiz(),
        );
        let error = guardian
            .verificar_lectura("./otros/secreto.txt")
            .expect_err("la ruta no está declarada");
        assert!(error.contains("fuera de los directorios permitidos"));
    }

    #[test]
    fn no_se_puede_escapar_del_directorio_con_dos_puntos() {
        let guardian = guardian_con(
            r#"{"sistema_archivos": {"habilitado": true,
                "directorios": [{"ruta": "./datos", "permiso": "todo"}]}}"#,
            &raiz(),
        );
        let error = guardian
            .verificar_lectura("./datos/../secreto.txt")
            .expect_err("'..' no debe sacar la ruta del directorio permitido");
        assert!(error.contains("fuera de los directorios permitidos"));
    }

    #[test]
    fn verificar_una_ruta_ya_resuelta_da_el_mismo_resultado() {
        // `renombrar` y `mover` derivan el destino de la ruta ya resuelta del
        // origen y la vuelven a verificar: la segunda pasada no debe volver a
        // unirla a la raíz.
        let guardian = guardian_con(
            r#"{"sistema_archivos": {"habilitado": true, "directorios": [
                {"ruta": "./", "permiso": "lectura"},
                {"ruta": "./salida", "permiso": "todo"}]}}"#,
            &raiz(),
        );
        let resuelta = guardian
            .verificar_escritura("./salida/informe.csv")
            .expect("el directorio de salida permite escribir");
        assert!(
            guardian
                .verificar_escritura(&resuelta.to_string_lossy())
                .is_ok()
        );
    }

    #[test]
    fn comodin_permite_cualquier_ruta() {
        let guardian = guardian_con(
            r#"{"sistema_archivos": {"habilitado": true,
                "directorios": [{"ruta": "*", "permiso": "todo"}]}}"#,
            &raiz(),
        );
        assert!(guardian.verificar_escritura("./cualquiera/x.txt").is_ok());
    }

    #[test]
    fn se_toma_el_directorio_mas_permisivo_que_coincida() {
        let guardian = guardian_con(
            r#"{"sistema_archivos": {"habilitado": true, "directorios": [
                {"ruta": "./", "permiso": "lectura"},
                {"ruta": "./salida", "permiso": "escritura"}]}}"#,
            &raiz(),
        );
        assert!(guardian.verificar_lectura("./notas.txt").is_ok());
        assert!(guardian.verificar_escritura("./salida/informe.txt").is_ok());
        assert!(guardian.verificar_escritura("./notas.txt").is_err());
    }
}
