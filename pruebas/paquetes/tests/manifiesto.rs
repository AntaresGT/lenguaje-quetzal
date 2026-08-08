//! Validación de `quetzal.json`, permisos y cache de bytecode.

use paquetes::{CacheBytecode, Manifiesto};

#[test]
fn manifiesto_deberia_leer_un_proyecto_valido() {
    let manifiesto = Manifiesto::leer_texto(
        r#"{
            "version": "0.1.0",
            "aplicacion": "demo",
            "entrada": "aplicacion/principal.qz",
            "tipo": "aplicacion",
            "quetzal": "0.0.2",
            "dependencias": {"fechas": "1.2.0"},
            "permisos": {}
        }"#,
    )
    .expect("el manifiesto es válido");
    assert_eq!(manifiesto.aplicacion, "demo");
    assert_eq!(manifiesto.entrada, "aplicacion/principal.qz");
    assert_eq!(
        manifiesto.dependencias.get("fechas").map(String::as_str),
        Some("1.2.0")
    );
    assert!(!manifiesto.permisos.red.habilitado);
}

#[test]
fn manifiesto_deberia_normalizar_claves_con_tilde() {
    let manifiesto = Manifiesto::leer_texto(
        r#"{
            "versión": "0.1.0",
            "aplicación": "demo",
            "entrada": "principal.qz",
            "tipo": "aplicación",
            "quetzal": "0.0.2"
        }"#,
    )
    .expect("debe aceptar claves con tilde");
    assert_eq!(manifiesto.version, "0.1.0");
    assert_eq!(manifiesto.tipo, "aplicacion");
}

#[test]
fn manifiesto_deberia_rechazar_campos_faltantes() {
    let error =
        Manifiesto::leer_texto(r#"{"version": "0.1.0"}"#).expect_err("faltan campos obligatorios");
    assert_eq!(error.codigo, "E0601");
}

#[test]
fn manifiesto_deberia_rechazar_version_invalida() {
    let error = Manifiesto::leer_texto(
        r#"{
            "version": "uno.dos",
            "aplicacion": "demo",
            "entrada": "principal.qz",
            "tipo": "aplicacion",
            "quetzal": "0.0.2"
        }"#,
    )
    .expect_err("la versión no es X.Y.Z");
    assert!(error.mensaje.contains("X.Y.Z"));
}

#[test]
fn permisos_deberian_leer_objeto_y_lista() {
    let objeto = Manifiesto::leer_texto(
        r#"{
            "version": "0.1.0", "aplicacion": "a", "entrada": "p.qz",
            "tipo": "aplicacion", "quetzal": "0.0.2",
            "permisos": {
                "red": {"habilitado": true},
                "sistema_archivos": {"habilitado": true, "directorios": [{"ruta": "./datos", "permiso": "lectura"}]}
            }
        }"#,
    )
    .expect("permisos en objeto");
    assert!(objeto.permisos.red.habilitado);
    assert!(objeto.permisos.sistema_archivos.habilitado);
    assert_eq!(objeto.permisos.sistema_archivos.directorios.len(), 1);

    let lista = Manifiesto::leer_texto(
        r#"{
            "version": "0.1.0", "aplicacion": "a", "entrada": "p.qz",
            "tipo": "aplicacion", "quetzal": "0.0.2",
            "permisos": [
                {"tipo": "ejecución", "habilitado": true, "ejecutables": ["*"]},
                {"tipo": "sistema-archivos", "habilitado": false, "directorios": []}
            ]
        }"#,
    )
    .expect("permisos en lista con tildes");
    assert!(lista.permisos.ejecucion.habilitado);
    assert_eq!(lista.permisos.ejecucion.ejecutables, vec!["*".to_string()]);
    assert!(!lista.permisos.sistema_archivos.habilitado);
}

#[test]
fn cache_deberia_guardar_y_recuperar_bytecode() {
    let temporal =
        std::env::temp_dir().join(format!("quetzal_cache_prueba_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temporal);

    let fuente = nucleo::Fuente::nueva("p.qz", "entero x = 1 + 2\n");
    let ast = sintaxis::parsear_modulo(&fuente).expect("parsea");
    let modulo = bytecode::generar_modulo(&ast).expect("genera");

    let cache = CacheBytecode::del_proyecto(&temporal);
    assert!(cache.buscar(&fuente.contenido).is_none());
    cache.guardar(&fuente.contenido, &modulo);
    let recuperado = cache.buscar(&fuente.contenido).expect("debe haber entrada");
    assert_eq!(recuperado, modulo);

    // Otro contenido no debe chocar con la misma entrada.
    assert!(cache.buscar("entero x = 999\n").is_none());

    cache.limpiar().expect("limpia la cache");
    assert!(cache.buscar(&fuente.contenido).is_none());
    let _ = std::fs::remove_dir_all(&temporal);
}
