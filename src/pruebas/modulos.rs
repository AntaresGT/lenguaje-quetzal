// Pruebas unitarias para el sistema de modulos del lenguaje Quetzal

use crate::modulos::{CargadorModulos, ManifiestoPaquete};
use crate::errores::CodigoError;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

// Contador atomico para generar IDs unicos
static CONTADOR: AtomicUsize = AtomicUsize::new(0);

/// Crea un archivo temporal de modulo para pruebas con nombre unico
fn crear_modulo_temporal(nombre_base: &str, contenido: &str) -> PathBuf {
    let id = CONTADOR.fetch_add(1, Ordering::SeqCst);
    let nombre_unico = format!("{}_{}", nombre_base, id);
    
    let dir_temp = std::env::temp_dir().join("quetzal_test");
    let _ = fs::create_dir_all(&dir_temp);
    
    let ruta = dir_temp.join(format!("{}.qz", nombre_unico));
    fs::write(&ruta, contenido).expect("No se pudo escribir archivo temporal");
    ruta
}

// ============================================
// Pruebas de CargadorModulos
// ============================================

#[test]
fn prueba_resolver_ruta_con_extension() {
    let cargador = CargadorModulos::nuevo();
    
    // Crear archivo temporal
    let ruta = crear_modulo_temporal("test_extension", "entero x = 1");
    
    // Resolver ruta con extension
    let resultado = cargador.resolver_ruta(ruta.to_str().unwrap());
    assert!(resultado.is_ok(), "Deberia resolver ruta con extension");
}

#[test]
fn prueba_resolver_ruta_sin_extension() {
    let cargador = CargadorModulos::nuevo();
    
    // Crear archivo temporal
    let ruta = crear_modulo_temporal("test_sin_ext", "entero x = 1");
    let ruta_sin_ext = ruta.with_extension("");
    
    // Resolver ruta sin extension
    let resultado = cargador.resolver_ruta(ruta_sin_ext.to_str().unwrap());
    assert!(resultado.is_ok(), "Deberia agregar .qz automaticamente");
}

#[test]
fn prueba_cargar_modulo_simple() {
    let cargador = CargadorModulos::nuevo();
    
    // Crear archivo temporal
    let contenido = "entero valor = 42";
    let ruta = crear_modulo_temporal("modulo_simple", contenido);
    
    // Cargar modulo
    let resultado = cargador.cargar_modulo(ruta.to_str().unwrap());
    assert!(resultado.is_ok(), "Deberia cargar modulo simple");
    
    let ast = resultado.unwrap();
    assert!(!ast.is_empty(), "AST no deberia estar vacio");
}

#[test]
fn prueba_cache_modulos() {
    let cargador = CargadorModulos::nuevo();
    
    // Crear archivo temporal
    let contenido = "entero x = 1";
    let ruta = crear_modulo_temporal("modulo_cache", contenido);
    let ruta_str = ruta.to_str().unwrap();
    
    // Cargar modulo primera vez
    let resultado1 = cargador.cargar_modulo(ruta_str);
    assert!(resultado1.is_ok());
    
    // Verificar que esta en cache
    assert!(cargador.esta_cargado(ruta_str), "Modulo deberia estar en cache");
    
    // Cargar segunda vez (deberia venir del cache)
    let resultado2 = cargador.cargar_modulo(ruta_str);
    assert!(resultado2.is_ok());
}

#[test]
fn prueba_error_modulo_no_encontrado() {
    let cargador = CargadorModulos::nuevo();
    
    // Intentar resolver ruta inexistente
    let resultado = cargador.resolver_ruta("modulo_inexistente_xyz_99999");
    assert!(resultado.is_err(), "Deberia fallar para modulo inexistente");
    
    if let Err(e) = resultado {
        assert_eq!(e.codigo(), CodigoError::ModuloNoEncontrado.codigo());
    }
}

#[test]
fn prueba_obtener_exportaciones() {
    let cargador = CargadorModulos::nuevo();
    
    // Crear modulo con funcion y variable
    let contenido = "entero sumar(entero a, entero b) {\n    retornar a + b\n}\n\nentero valor_constante = 100";
    let ruta = crear_modulo_temporal("modulo_exports", contenido);
    
    // Obtener exportaciones
    let resultado = cargador.obtener_exportaciones(ruta.to_str().unwrap());
    assert!(resultado.is_ok(), "Deberia obtener exportaciones");
    
    let exports = resultado.unwrap();
    assert!(exports.contains_key("sumar"), "Deberia exportar funcion 'sumar'");
    assert!(exports.contains_key("valor_constante"), "Deberia exportar 'valor_constante'");
}

#[test]
fn prueba_exportaciones_incluyen_prototipos() {
    let cargador = CargadorModulos::nuevo();

    let contenido = r#"
prototipo Contrato {
    entero id
}
"#;
    let ruta = crear_modulo_temporal("modulo_proto_exports", contenido);

    let resultado = cargador.obtener_exportaciones(ruta.to_str().unwrap());
    assert!(resultado.is_ok(), "Deberia obtener exportaciones del modulo con prototipo");

    let exports = resultado.unwrap();
    assert!(exports.contains_key("Contrato"), "Deberia exportar el prototipo 'Contrato'");
}

#[test]
fn prueba_obtener_elementos_especificos() {
    let cargador = CargadorModulos::nuevo();
    
    // Crear modulo con multiples elementos
    let contenido = "entero a = 1\nentero b = 2\nentero c = 3";
    let ruta = crear_modulo_temporal("modulo_elementos", contenido);
    
    // Obtener solo algunos elementos
    let resultado = cargador.obtener_elementos(
        ruta.to_str().unwrap(),
        &["a".to_string(), "c".to_string()]
    );
    assert!(resultado.is_ok(), "Deberia obtener elementos especificos: {:?}", resultado);
    
    let elementos = resultado.unwrap();
    assert_eq!(elementos.len(), 2, "Deberia tener 2 elementos");
    assert!(elementos.contains_key("a"));
    assert!(elementos.contains_key("c"));
}

#[test]
fn prueba_error_elemento_no_exportado() {
    let cargador = CargadorModulos::nuevo();
    
    // Crear modulo simple
    let contenido = "entero x = 1";
    let ruta = crear_modulo_temporal("modulo_no_export", contenido);
    
    // Intentar obtener elemento que no existe
    let resultado = cargador.obtener_elementos(
        ruta.to_str().unwrap(),
        &["elemento_inexistente".to_string()]
    );
    assert!(resultado.is_err(), "Deberia fallar para elemento inexistente");
    
    if let Err(e) = resultado {
        assert_eq!(e.codigo(), CodigoError::ElementoImportadoNoEncontrado.codigo());
    }
}

#[test]
fn prueba_directorio_base() {
    let id = CONTADOR.fetch_add(1, Ordering::SeqCst);
    let dir_temp = std::env::temp_dir().join(format!("quetzal_test_base_{}", id));
    let _ = fs::create_dir_all(&dir_temp);
    
    // Crear modulo en el directorio base
    let ruta = dir_temp.join("modulo_base.qz");
    fs::write(&ruta, "entero x = 1").expect("No se pudo escribir");
    
    // Crear cargador con directorio base
    let cargador = CargadorModulos::con_directorio_base(dir_temp.clone());
    
    // Cargar solo con nombre
    let resultado = cargador.cargar_modulo("modulo_base");
    assert!(resultado.is_ok(), "Deberia resolver desde directorio base");
    
    let _ = fs::remove_dir_all(&dir_temp);
}

#[test]
fn prueba_resolver_dependencia_local_desde_manifiesto() {
    let id = CONTADOR.fetch_add(1, Ordering::SeqCst);
    let dir_temp = std::env::temp_dir().join(format!("quetzal_test_pkg_{}", id));
    let dir_app = dir_temp.join("app");
    let dir_libreria = dir_temp.join("deps").join("utiles");

    fs::create_dir_all(dir_app.join("src")).expect("No se pudo crear app");
    fs::create_dir_all(dir_libreria.join("src")).expect("No se pudo crear librería");

    fs::write(
        dir_app.join("quetzal.json"),
        r#"{
  "version": "0.1.0",
  "nombre": "app-prueba",
  "dependencias": {
    "utiles": "../deps/utiles"
  }
}"#,
    )
    .expect("No se pudo escribir manifiesto principal");

    fs::write(
        dir_libreria.join("quetzal.json"),
        r#"{
  "version": "0.1.0",
  "nombre": "utiles",
  "tipo": "libreria",
  "biblioteca": "src/lib.qz"
}"#,
    )
    .expect("No se pudo escribir manifiesto de librería");

    fs::write(
        dir_libreria.join("src").join("lib.qz"),
        r#"
entero visible = 7

exportar {
    visible,
}
"#,
    )
    .expect("No se pudo escribir librería");

    let cargador = CargadorModulos::con_directorio_base(dir_app.join("src"));
    let resultado = cargador.obtener_exportaciones("utiles");
    assert!(resultado.is_ok(), "Debe resolver librería local por manifiesto");

    let exportaciones = resultado.unwrap();
    assert!(exportaciones.contains_key("visible"), "Debe exponer el símbolo exportado");

    let _ = fs::remove_dir_all(&dir_temp);
}

#[test]
fn prueba_manifiesto_invalido_falla_por_esquema() {
    let id = CONTADOR.fetch_add(1, Ordering::SeqCst);
    let dir_temp = std::env::temp_dir().join(format!("quetzal_test_schema_{}", id));
    let _ = fs::create_dir_all(&dir_temp);
    let ruta = dir_temp.join("quetzal.json");

    fs::write(
        &ruta,
        r#"{
  "nombre": "sin-version",
  "dependencias": {},
  "propiedad_extra": true
}"#,
    )
    .expect("No se pudo escribir manifiesto inválido");

    let resultado = ManifiestoPaquete::cargar_desde_archivo(&ruta);
    assert!(resultado.is_err(), "El manifiesto inválido debe ser rechazado por el esquema");

    let _ = fs::remove_dir_all(&dir_temp);
}

#[test]
fn prueba_manifiesto_legado_valido_pasa_esquema() {
    let id = CONTADOR.fetch_add(1, Ordering::SeqCst);
    let dir_temp = std::env::temp_dir().join(format!("quetzal_test_schema_ok_{}", id));
    let _ = fs::create_dir_all(&dir_temp);
    let ruta = dir_temp.join("quetzal.json");

    fs::write(
        &ruta,
        r#"{
  "versión": "0.2.0",
  "aplicación": "mi-proyecto-quetzal",
  "dependencias": {
    "libreria_ejemplo": "1.0.0"
  }
}"#,
    )
    .expect("No se pudo escribir manifiesto válido");

    let resultado = ManifiestoPaquete::cargar_desde_archivo(&ruta);
    assert!(resultado.is_ok(), "El formato legado debe seguir siendo válido");

    let _ = fs::remove_dir_all(&dir_temp);
}

#[test]
fn prueba_exportacion_explicita_restringe_api_publica() {
    let cargador = CargadorModulos::nuevo();

    let contenido = r#"
entero interno = 1
entero visible = 2

exportar {
    visible,
}
"#;
    let ruta = crear_modulo_temporal("modulo_exportacion_explicita", contenido);

    let resultado = cargador.obtener_exportaciones(ruta.to_str().unwrap());
    assert!(resultado.is_ok(), "Debería obtener exportaciones explícitas");

    let exportaciones = resultado.unwrap();
    assert!(exportaciones.contains_key("visible"), "Debe exportar 'visible'");
    assert!(!exportaciones.contains_key("interno"), "No debe exportar 'interno'");
}

#[test]
fn prueba_importar_elemento_privado_falla_con_exportacion_explicita() {
    let cargador = CargadorModulos::nuevo();

    let contenido = r#"
entero interno = 1
entero visible = 2

exportar {
    visible,
}
"#;
    let ruta = crear_modulo_temporal("modulo_exportacion_privada", contenido);

    let resultado = cargador.obtener_elementos(
        ruta.to_str().unwrap(),
        &["interno".to_string()],
    );
    assert!(resultado.is_err(), "Importar un símbolo privado debe fallar");

    if let Err(e) = resultado {
        assert_eq!(e.codigo(), CodigoError::ElementoImportadoNoEncontrado.codigo());
    }
}

// ============================================
// Pruebas de integracion con importacion
// ============================================

#[test]
fn prueba_importacion_modulo_nativo() {
    use super::auxiliares::verificar_ejecucion_exitosa;
    
    let codigo = r#"importar { Matematica } desde "quetzal/matematica"
entero resultado = Matematica.sumar(2, 3)"#;
    
    assert!(verificar_ejecucion_exitosa(codigo), 
        "Deberia poder importar modulo nativo");
}
