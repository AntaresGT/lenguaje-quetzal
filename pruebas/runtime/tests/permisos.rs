//! Pruebas del modelo de permisos: el guardián y el permiso de `ejecucion`.

use paquetes::Permisos;
use runtime::GuardianPermisos;

fn permisos_de_json(json: &str) -> Permisos {
    let valor: serde_json::Value = serde_json::from_str(json).expect("JSON de prueba válido");
    Permisos::desde_json(&valor).expect("permisos de prueba válidos")
}

fn directorio_temporal(nombre: &str) -> std::path::PathBuf {
    let ruta =
        std::env::temp_dir().join(format!("quetzal_permisos_{nombre}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&ruta);
    std::fs::create_dir_all(ruta.join("datos")).expect("se puede crear el directorio temporal");
    ruta
}

#[test]
fn guardian_deberia_aplicar_lista_blanca_de_ejecucion() {
    let raiz = directorio_temporal("ejecucion");
    let guardian = GuardianPermisos::denegado();
    guardian.configurar(
        permisos_de_json(r#"{"ejecucion": {"habilitado": true, "ejecutables": ["git"]}}"#),
        &raiz,
    );
    assert!(guardian.verificar_ejecucion("git").is_ok());
    assert!(guardian.verificar_ejecucion("rm").is_err());

    guardian.configurar(
        permisos_de_json(r#"{"ejecucion": {"habilitado": true, "ejecutables": ["*"]}}"#),
        &raiz,
    );
    assert!(guardian.verificar_ejecucion("cualquiera").is_ok());
    let _ = std::fs::remove_dir_all(&raiz);
}
