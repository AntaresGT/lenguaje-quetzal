// Pruebas unitarias para el sistema de permisos del lenguaje Quetzal

use crate::configuracion::permisos::{ConfiguracionPermisos, Permiso, SistemaPermisos};
use crate::errores::CodigoError;
use std::path::PathBuf;

// ============================================
// Pruebas de SistemaPermisos
// ============================================

#[test]
fn prueba_permisos_por_defecto_denegados() {
    let permisos = SistemaPermisos::nuevo();
    
    // Por defecto, los permisos estan denegados
    let resultado_archivo = permisos.puede_acceder_archivo(&PathBuf::from("/tmp/test.txt"));
    assert!(resultado_archivo.is_err(), "Acceso a archivos deberia estar denegado por defecto");
    
    let resultado_red = permisos.puede_acceder_red();
    assert!(resultado_red.is_err(), "Acceso a red deberia estar denegado por defecto");
}

#[test]
fn prueba_cargar_permisos_archivos_habilitados() {
    let config = ConfiguracionPermisos {
        permisos: vec![
            Permiso {
                tipo: "sistema-archivos".to_string(),
                habilitado: true,
                alcance: None,
                directorios: None,
            }
        ],
    };
    
    let permisos = SistemaPermisos::cargar_desde_config(&config);
    
    // Con permisos de archivos habilitados, deberia permitir acceso
    let resultado = permisos.puede_acceder_archivo(&PathBuf::from("/cualquier/ruta"));
    assert!(resultado.is_ok(), "Deberia permitir acceso a archivos cuando esta habilitado");
}

#[test]
fn prueba_cargar_permisos_red_habilitados() {
    let config = ConfiguracionPermisos {
        permisos: vec![
            Permiso {
                tipo: "red".to_string(),
                habilitado: true,
                alcance: None,
                directorios: None,
            }
        ],
    };
    
    let permisos = SistemaPermisos::cargar_desde_config(&config);
    
    // Con permisos de red habilitados, deberia permitir acceso
    let resultado = permisos.puede_acceder_red();
    assert!(resultado.is_ok(), "Deberia permitir acceso a red cuando esta habilitado");
}

#[test]
fn prueba_permisos_directorios_restringidos() {
    let config = ConfiguracionPermisos {
        permisos: vec![
            Permiso {
                tipo: "sistema-archivos".to_string(),
                habilitado: true,
                alcance: None,
                directorios: Some(vec![
                    "/permitido".to_string(),
                    "/otro/permitido".to_string(),
                ]),
            }
        ],
    };
    
    let permisos = SistemaPermisos::cargar_desde_config(&config);
    
    // Acceso a directorio permitido
    let resultado_permitido = permisos.puede_acceder_archivo(&PathBuf::from("/permitido/archivo.txt"));
    assert!(resultado_permitido.is_ok(), "Deberia permitir acceso a directorio en lista permitida");
    
    // Acceso a directorio no permitido
    let resultado_denegado = permisos.puede_acceder_archivo(&PathBuf::from("/no_permitido/archivo.txt"));
    assert!(resultado_denegado.is_err(), "Deberia denegar acceso a directorio fuera de lista permitida");
}

#[test]
fn prueba_codigo_error_violacion_seguridad() {
    let permisos = SistemaPermisos::nuevo();
    
    // Verificar que el error es de violacion de seguridad
    let resultado = permisos.puede_acceder_archivo(&PathBuf::from("/test"));
    
    if let Err(e) = resultado {
        assert_eq!(e.codigo(), CodigoError::ViolacionSeguridad.codigo(),
            "El error deberia ser de violacion de seguridad");
    }
}

#[test]
fn prueba_permisos_multiples() {
    let config = ConfiguracionPermisos {
        permisos: vec![
            Permiso {
                tipo: "sistema-archivos".to_string(),
                habilitado: true,
                alcance: None,
                directorios: None,
            },
            Permiso {
                tipo: "red".to_string(),
                habilitado: true,
                alcance: None,
                directorios: None,
            }
        ],
    };
    
    let permisos = SistemaPermisos::cargar_desde_config(&config);
    
    // Ambos permisos deberian estar habilitados
    assert!(permisos.puede_acceder_archivo(&PathBuf::from("/test")).is_ok());
    assert!(permisos.puede_acceder_red().is_ok());
}

#[test]
fn prueba_permisos_deshabilitados_explicitos() {
    let config = ConfiguracionPermisos {
        permisos: vec![
            Permiso {
                tipo: "sistema-archivos".to_string(),
                habilitado: false,
                alcance: None,
                directorios: None,
            }
        ],
    };
    
    let permisos = SistemaPermisos::cargar_desde_config(&config);
    
    // Permiso explicitamente deshabilitado
    let resultado = permisos.puede_acceder_archivo(&PathBuf::from("/test"));
    assert!(resultado.is_err(), "Permiso deshabilitado explicitamente deberia denegar acceso");
}

// ============================================
// Pruebas de integracion con Entorno
// ============================================

#[test]
fn prueba_entorno_sin_permisos() {
    use crate::interprete::entorno::Entorno;
    
    let entorno = Entorno::nuevo();
    
    // Sin permisos configurados, deberia permitir todo por defecto
    let resultado_archivo = entorno.verificar_permiso_archivo(&PathBuf::from("/test"));
    assert!(resultado_archivo.is_ok(), "Sin permisos configurados, deberia permitir acceso");
    
    let resultado_red = entorno.verificar_permiso_red();
    assert!(resultado_red.is_ok(), "Sin permisos configurados, deberia permitir red");
}

#[test]
fn prueba_entorno_con_permisos() {
    use crate::interprete::entorno::Entorno;
    
    let mut entorno = Entorno::nuevo();
    
    // Configurar permisos restrictivos
    let permisos = SistemaPermisos::nuevo();
    entorno.establecer_permisos(permisos);
    
    // Con permisos configurados y restrictivos, deberia denegar
    let resultado = entorno.verificar_permiso_archivo(&PathBuf::from("/test"));
    assert!(resultado.is_err(), "Con permisos restrictivos, deberia denegar acceso");
}

// ============================================
// Pruebas de deserializacion JSON
// ============================================

#[test]
fn prueba_deserializar_configuracion_permisos() {
    let json = r#"{
        "permisos": [
            {
                "tipo": "sistema-archivos",
                "habilitado": true,
                "directorios": ["./", "/tmp"]
            },
            {
                "tipo": "red",
                "habilitado": false
            }
        ]
    }"#;
    
    let config: ConfiguracionPermisos = serde_json::from_str(json).expect("Deberia deserializar");
    
    assert_eq!(config.permisos.len(), 2);
    assert_eq!(config.permisos[0].tipo, "sistema-archivos");
    assert!(config.permisos[0].habilitado);
    assert_eq!(config.permisos[1].tipo, "red");
    assert!(!config.permisos[1].habilitado);
}
