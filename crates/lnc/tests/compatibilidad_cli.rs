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
            assert_eq!(
                normalizar_saltos(&salida.stdout),
                normalizar_saltos(&esperado),
                "stdout de {}",
                caso.nombre
            );
            assert_eq!(
                normalizar_saltos(&salida.stderr),
                normalizar_saltos(&error_esperado),
                "stderr de {}",
                caso.nombre
            );
        }
    }
}

fn normalizar_saltos(bytes: &[u8]) -> Vec<u8> {
    let mut normalizados = Vec::with_capacity(bytes.len());
    let mut indice = 0;
    while indice < bytes.len() {
        if bytes.get(indice..indice + 2) == Some(b"\r\n") {
            normalizados.push(b'\n');
            indice += 2;
        } else {
            normalizados.push(bytes[indice]);
            indice += 1;
        }
    }
    normalizados
}

#[test]
fn normalizar_saltos_deberia_convertir_crlf_sin_alterar_otros_bytes() {
    assert_eq!(
        normalizar_saltos(b"primero\r\nsegundo\rtercero\n"),
        b"primero\nsegundo\rtercero\n"
    );
}
