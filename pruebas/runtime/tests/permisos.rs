//! Pruebas del modelo de permisos: seguro por defecto, y módulos `red` y
//! `sistema_archivos` controlados por el guardián.

use std::rc::Rc;

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
fn guardian_deberia_denegar_todo_por_defecto() {
    let guardian = GuardianPermisos::denegado();
    assert!(guardian.verificar_red().is_err());
    assert!(guardian.verificar_lectura("datos.txt").is_err());
    assert!(guardian.verificar_escritura("datos.txt").is_err());
    assert!(guardian.verificar_ejecucion("git").is_err());
}

#[test]
fn guardian_deberia_permitir_red_solo_si_esta_habilitada() {
    let raiz = directorio_temporal("red");
    let guardian = GuardianPermisos::denegado();
    guardian.configurar(permisos_de_json(r#"{"red": {"habilitado": true}}"#), &raiz);
    assert!(guardian.verificar_red().is_ok());

    guardian.configurar(permisos_de_json(r#"{"red": {"habilitado": false}}"#), &raiz);
    assert!(guardian.verificar_red().is_err());
    let _ = std::fs::remove_dir_all(&raiz);
}

#[test]
fn guardian_deberia_limitar_archivos_por_directorio_y_acceso() {
    let raiz = directorio_temporal("archivos");
    let guardian = GuardianPermisos::denegado();
    guardian.configurar(
        permisos_de_json(
            r#"{"sistema_archivos": {"habilitado": true, "directorios": [
                {"ruta": "./datos", "permiso": "lectura"}
            ]}}"#,
        ),
        &raiz,
    );

    let dentro = raiz.join("datos").join("archivo.txt");
    let dentro = dentro.to_string_lossy();
    assert!(guardian.verificar_lectura(&dentro).is_ok());
    // Solo tiene permiso de lectura, no de escritura.
    assert!(guardian.verificar_escritura(&dentro).is_err());
    // Fuera del directorio declarado no hay acceso.
    let fuera = raiz.join("secreto.txt");
    assert!(
        guardian
            .verificar_lectura(&fuera.to_string_lossy())
            .is_err()
    );
    // Escapar con `..` tampoco funciona.
    let escapada = raiz.join("datos").join("..").join("secreto.txt");
    assert!(
        guardian
            .verificar_lectura(&escapada.to_string_lossy())
            .is_err()
    );
    let _ = std::fs::remove_dir_all(&raiz);
}

#[test]
fn guardian_deberia_aceptar_acceso_todo() {
    let raiz = directorio_temporal("todo");
    let guardian = GuardianPermisos::denegado();
    guardian.configurar(
        permisos_de_json(
            r#"{"sistema_archivos": {"habilitado": true, "directorios": [
                {"ruta": "./datos", "permiso": "todo"}
            ]}}"#,
        ),
        &raiz,
    );
    let ruta = raiz.join("datos").join("nuevo.txt");
    let ruta = ruta.to_string_lossy();
    assert!(guardian.verificar_lectura(&ruta).is_ok());
    assert!(guardian.verificar_escritura(&ruta).is_ok());
    let _ = std::fs::remove_dir_all(&raiz);
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

#[test]
fn modulo_sistema_archivos_deberia_respetar_permisos() {
    let raiz = directorio_temporal("nativo");
    std::fs::write(raiz.join("datos").join("hola.txt"), "contenido secreto")
        .expect("se puede escribir el archivo de prueba");

    // Sin permisos: leer falla con E0701.
    let registro = modulos_nativos::crear_registro();
    let leer = registro
        .buscar_funcion("sistema_archivos.leer_texto")
        .expect("la función existe");
    let ruta = raiz.join("datos").join("hola.txt");
    let resultado = leer(&[maquina_virtual::Valor::texto(ruta.to_string_lossy())]);
    assert!(resultado.is_err(), "sin permisos la lectura debe fallar");

    // Con permisos de lectura: funciona.
    let guardian = Rc::new(GuardianPermisos::denegado());
    guardian.configurar(
        permisos_de_json(
            r#"{"sistema_archivos": {"habilitado": true, "directorios": [
                {"ruta": "./datos", "permiso": "lectura"}
            ]}}"#,
        ),
        &raiz,
    );
    let registro = modulos_nativos::crear_registro_con_permisos(&guardian);
    let leer = registro
        .buscar_funcion("sistema_archivos.leer_texto")
        .expect("la función existe");
    let valor = leer(&[maquina_virtual::Valor::texto(ruta.to_string_lossy())])
        .expect("con permiso la lectura funciona");
    match valor {
        maquina_virtual::Valor::Texto(texto) => assert_eq!(&*texto, "contenido secreto"),
        otro => panic!("se esperaba texto, llegó {}", otro.nombre_tipo()),
    }

    // Escribir sigue prohibido (solo lectura).
    let escribir = registro
        .buscar_funcion("sistema_archivos.escribir_texto")
        .expect("la función existe");
    let resultado = escribir(&[
        maquina_virtual::Valor::texto(ruta.to_string_lossy()),
        maquina_virtual::Valor::texto("sobrescrito"),
    ]);
    assert!(resultado.is_err(), "solo lectura: escribir debe fallar");
    let _ = std::fs::remove_dir_all(&raiz);
}

#[test]
fn guardian_deberia_permitir_solo_cliente_o_solo_servidor() {
    let raiz = directorio_temporal("red_granular_cliente");
    let guardian = GuardianPermisos::denegado();
    guardian.configurar(
        permisos_de_json(r#"{"red": {"habilitado": true, "cliente": true}}"#),
        &raiz,
    );
    assert!(guardian.verificar_red_cliente("ejemplo.com", 443).is_ok());
    assert!(
        guardian.verificar_red_servidor(8080).is_err(),
        "sin 'servidor' explícito, no se puede escuchar"
    );

    guardian.configurar(
        permisos_de_json(r#"{"red": {"habilitado": true, "servidor": true}}"#),
        &raiz,
    );
    assert!(guardian.verificar_red_servidor(8080).is_ok());
    assert!(
        guardian.verificar_red_cliente("ejemplo.com", 443).is_err(),
        "sin 'cliente' explícito, no se puede conectar"
    );
    let _ = std::fs::remove_dir_all(&raiz);
}

#[test]
fn guardian_deberia_respetar_lista_blanca_de_anfitriones_y_puertos() {
    let raiz = directorio_temporal("red_granular_listas");
    let guardian = GuardianPermisos::denegado();
    guardian.configurar(
        permisos_de_json(
            r#"{"red": {"habilitado": true, "cliente": true, "servidor": true,
                "anfitriones": ["api.ejemplo.com"], "puertos": [8080, 8443]}}"#,
        ),
        &raiz,
    );

    assert!(
        guardian
            .verificar_red_cliente("api.ejemplo.com", 8080)
            .is_ok()
    );
    assert!(
        guardian
            .verificar_red_cliente("otro-dominio.com", 8080)
            .is_err(),
        "el anfitrión no está en la lista blanca"
    );
    assert!(
        guardian
            .verificar_red_cliente("api.ejemplo.com", 9999)
            .is_err(),
        "el puerto no está en la lista blanca"
    );
    assert!(guardian.verificar_red_servidor(8443).is_ok());
    assert!(
        guardian.verificar_red_servidor(3000).is_err(),
        "el puerto 3000 no está en la lista blanca"
    );
    let _ = std::fs::remove_dir_all(&raiz);
}

#[test]
fn guardian_forma_simple_habilita_cliente_y_servidor_sin_restricciones() {
    let raiz = directorio_temporal("red_granular_simple");
    let guardian = GuardianPermisos::denegado();
    guardian.configurar(permisos_de_json(r#"{"red": {"habilitado": true}}"#), &raiz);
    assert!(guardian.verificar_red_cliente("cualquiera.com", 1).is_ok());
    assert!(guardian.verificar_red_servidor(65000).is_ok());
    let _ = std::fs::remove_dir_all(&raiz);
}

#[test]
fn modulo_red_deberia_denegarse_sin_permiso() {
    let registro = modulos_nativos::crear_registro();
    let obtener = registro
        .buscar_funcion("red.obtener")
        .expect("la función existe");
    let resultado = obtener(&[maquina_virtual::Valor::texto("http://ejemplo.com")]);
    assert!(resultado.is_err(), "sin permiso de red debe fallar");
}
