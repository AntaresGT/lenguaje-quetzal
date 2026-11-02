mod comun;

use comun::{Temporal, afirmar_salida, ejecutar_en, quetzal};

const SALIDA: &[u8] = b"salida-42\n";

#[test]
fn cli_deberia_ejecutar_archivo_y_proyecto_sin_banderas() {
    let temporal = Temporal::nuevo("entradas");
    let entrada = temporal.proyecto("consola.mostrar(\"salida-42\")\n");
    let ruta = entrada.to_str().expect("ruta UTF-8");

    for args in [
        vec!["ejecutar", ruta],
        vec![ruta],
        vec!["ejecutar"],
        vec!["ejecutar", "--sin-jit", ruta],
        vec!["ejecutar", "--sin-optimizar", ruta],
    ] {
        let salida = ejecutar_en(&temporal.ruta, &args);
        afirmar_salida(&salida, SALIDA);
    }
}

#[test]
fn cli_deberia_conservar_errores_con_y_sin_jit() {
    let temporal = Temporal::nuevo("errores");
    let entrada = temporal.ruta.join("error.qz");
    std::fs::write(&entrada, "entero resultado = 1 / 0\n").expect("escribir error");
    let ruta = entrada.to_str().expect("ruta UTF-8");
    let normal = ejecutar_en(&temporal.ruta, &["ejecutar", ruta]);
    let sin_jit = ejecutar_en(&temporal.ruta, &["ejecutar", "--sin-jit", ruta]);
    assert_eq!(normal.status.code(), Some(1));
    assert_eq!(normal.stdout, sin_jit.stdout);
    assert_eq!(normal.stderr, sin_jit.stderr);
    assert!(String::from_utf8_lossy(&normal.stderr).contains("división por cero"));
}

#[test]
fn cli_deberia_recompilar_una_cache_corrupta() {
    let temporal = Temporal::nuevo("cache");
    temporal.proyecto("consola.mostrar(\"salida-42\")\n");
    afirmar_salida(&ejecutar_en(&temporal.ruta, &["ejecutar"]), SALIDA);
    let cache = temporal.ruta.join(".quetzal").join("cache");
    let entradas: Vec<_> = std::fs::read_dir(&cache)
        .expect("cache creada")
        .map(|entrada| entrada.expect("entrada de cache").path())
        .collect();
    assert!(!entradas.is_empty());
    for entrada in entradas {
        std::fs::write(entrada, b"corrupta").expect("corromper cache");
    }
    afirmar_salida(&ejecutar_en(&temporal.ruta, &["ejecutar"]), SALIDA);
}

#[test]
fn cli_deberia_rechazar_banderas_jit_contradictorias() {
    let mut comando = quetzal();
    comando.args(["ejecutar", "--jit", "--sin-jit"]);
    let salida = comun::ejecutar(&mut comando, std::time::Duration::from_secs(10));
    assert_eq!(salida.status.code(), Some(2));
}

#[cfg(not(feature = "jit"))]
#[test]
fn cli_sin_soporte_deberia_rechazar_solo_la_activacion_explicita() {
    let temporal = Temporal::nuevo("sin_feature");
    let entrada = temporal.proyecto("consola.mostrar(\"salida-42\")\n");
    let ruta = entrada.to_str().expect("ruta UTF-8");
    let salida = ejecutar_en(&temporal.ruta, &["ejecutar", "--jit", ruta]);
    assert_eq!(salida.status.code(), Some(2));
    afirmar_salida(&ejecutar_en(&temporal.ruta, &["ejecutar", ruta]), SALIDA);
}

#[cfg(feature = "jit")]
#[test]
fn cli_con_soporte_deberia_aceptar_activacion_explicita() {
    let temporal = Temporal::nuevo("con_feature");
    let entrada = temporal.proyecto("consola.mostrar(\"salida-42\")\n");
    let ruta = entrada.to_str().expect("ruta UTF-8");
    afirmar_salida(
        &ejecutar_en(&temporal.ruta, &["ejecutar", "--jit", ruta]),
        SALIDA,
    );
}
