mod comun;

use std::path::PathBuf;
use std::time::Duration;

use comun::{ejecutar, quetzal};
use serde::Deserialize;

#[derive(Deserialize)]
struct Inventario {
    oracle: String,
    casos: Vec<Caso>,
}

#[derive(Deserialize)]
struct Caso {
    nombre: String,
    entrada: String,
    salida: i32,
    stdout: String,
    stderr: String,
    timeout_ms: u64,
}

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("raiz del workspace")
}

#[test]
fn ejemplos_congelados_deberian_coincidir_con_el_oraculo_en_modo_normal_y_sin_jit() {
    let raiz = raiz();
    let inventario: Inventario = serde_json::from_str(
        &std::fs::read_to_string(raiz.join("pruebas/compatibilidad/casos.json"))
            .expect("leer inventario"),
    )
    .expect("inventario válido");
    let oraculo = raiz.join(inventario.oracle);

    for caso in inventario.casos {
        let esperado = std::fs::read(oraculo.join(&caso.stdout)).expect("stdout del oráculo");
        let error_esperado = std::fs::read(oraculo.join(&caso.stderr)).expect("stderr del oráculo");
        for banderas in [vec![], vec!["--sin-jit"]] {
            let entrada = raiz.join(&caso.entrada);
            let mut comando = quetzal();
            comando
                .current_dir(&raiz)
                .arg("ejecutar")
                .args(banderas)
                .arg(&entrada);
            let salida = ejecutar(&mut comando, Duration::from_millis(caso.timeout_ms));
            assert_eq!(
                salida.status.code(),
                Some(caso.salida),
                "caso {}",
                caso.nombre
            );
            assert_eq!(salida.stdout, esperado, "stdout de {}", caso.nombre);
            assert_eq!(salida.stderr, error_esperado, "stderr de {}", caso.nombre);
        }
    }
}
