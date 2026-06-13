//! Pruebas de tokenización del Lenguaje Quetzal.

use lexico::{TipoToken, tokenizar};
use nucleo::Fuente;

fn tokens_de(codigo: &str) -> Vec<TipoToken> {
    let fuente = Fuente::nueva("prueba.qz", codigo);
    tokenizar(&fuente)
        .expect("el código de prueba debería tokenizar sin errores")
        .into_iter()
        .map(|token| token.tipo)
        .collect()
}

#[test]
fn tokenizar_deberia_reconocer_declaracion_de_entero() {
    let tokens = tokens_de("entero edad = 30");
    assert_eq!(
        tokens,
        vec![
            TipoToken::TipoEntero,
            TipoToken::Identificador("edad".to_string()),
            TipoToken::Asignar,
            TipoToken::LiteralEntero(30),
            TipoToken::Fin,
        ]
    );
}

#[test]
fn tokenizar_deberia_aceptar_tipo_numero_con_y_sin_tilde() {
    let con_tilde = tokens_de("número precio = 99.99");
    let sin_tilde = tokens_de("numero precio = 99.99");
    assert_eq!(con_tilde, sin_tilde);
    assert_eq!(con_tilde[0], TipoToken::TipoNumero);
    assert_eq!(con_tilde[3], TipoToken::LiteralNumero("99.99".to_string()));
}

#[test]
fn tokenizar_deberia_soportar_identificadores_unicode() {
    let tokens = tokens_de("entero año = 2026");
    assert_eq!(tokens[1], TipoToken::Identificador("año".to_string()));
}

#[test]
fn tokenizar_deberia_reconocer_asincrono_con_tilde() {
    let tokens = tokens_de("asincróno entero duplicar(entero valor) {}");
    assert_eq!(tokens[0], TipoToken::Asincrono);
}

#[test]
fn tokenizar_deberia_separar_texto_interpolado() {
    let tokens = tokens_de(r#"consola.mostrar(t"Hola {nombre}")"#);
    assert!(
        tokens.contains(&TipoToken::TextoInterpolado("Hola {nombre}".to_string())),
        "tokens: {tokens:?}"
    );
}

#[test]
fn tokenizar_deberia_resolver_escapes_en_textos() {
    let tokens = tokens_de(r#"texto json = "{\"clave\":\"valor\"}""#);
    assert!(tokens.contains(&TipoToken::LiteralTexto(
        "{\"clave\":\"valor\"}".to_string()
    )));
}

#[test]
fn tokenizar_deberia_ignorar_comentarios() {
    let tokens = tokens_de("// comentario de línea\n/* comentario\nde bloque */\nentero a = 1");
    assert_eq!(tokens[0], TipoToken::TipoEntero);
}

#[test]
fn tokenizar_deberia_reconocer_operadores_compuestos() {
    let tokens = tokens_de("a += 1 b-- c == d != e <= f >= g && h || !i");
    for esperado in [
        TipoToken::MasIgual,
        TipoToken::Decremento,
        TipoToken::IgualIgual,
        TipoToken::Diferente,
        TipoToken::MenorOIgual,
        TipoToken::MayorOIgual,
        TipoToken::YLogico,
        TipoToken::OLogico,
        TipoToken::NoLogico,
    ] {
        assert!(tokens.contains(&esperado), "falta {esperado:?}");
    }
}

#[test]
fn tokenizar_deberia_reconocer_palabras_logicas_y_o() {
    let tokens = tokens_de("a y b o c");
    assert_eq!(tokens[1], TipoToken::YLogico);
    assert_eq!(tokens[3], TipoToken::OLogico);
}

#[test]
fn tokenizar_no_deberia_confundir_identificadores_que_empiezan_con_y_u_o() {
    let tokens = tokens_de("yo ya oso orden");
    for esperado in ["yo", "ya", "oso", "orden"] {
        assert!(
            tokens.contains(&TipoToken::Identificador(esperado.to_string())),
            "falta identificador {esperado:?}"
        );
    }
}

#[test]
fn tokenizar_deberia_separar_metodo_de_literal_entero() {
    // 123.texto() debe ser: entero 123, punto, identificador texto, parentesis.
    let tokens = tokens_de("123.texto()");
    assert_eq!(
        tokens[..4],
        [
            TipoToken::LiteralEntero(123),
            TipoToken::Punto,
            TipoToken::TipoTexto,
            TipoToken::ParentesisIzquierdo,
        ]
    );
}

#[test]
fn tokenizar_deberia_fallar_con_texto_sin_cerrar() {
    let fuente = Fuente::nueva("prueba.qz", "texto a = \"sin cerrar");
    let error = tokenizar(&fuente).expect_err("debería fallar");
    assert_eq!(error.codigo, "E0002");
}

#[test]
fn tokenizar_deberia_fallar_con_entero_gigante() {
    let fuente = Fuente::nueva("prueba.qz", "entero a = 99999999999999999999");
    let error = tokenizar(&fuente).expect_err("debería fallar");
    assert_eq!(error.codigo, "E0003");
}

#[test]
fn tokenizar_deberia_procesar_todos_los_ejemplos() {
    let directorio = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ejemplos");
    let entradas = std::fs::read_dir(directorio).expect("debería existir ejemplos/");
    let mut archivos_procesados = 0;

    for entrada in entradas.flatten() {
        let ruta = entrada.path();
        if ruta.extension().and_then(|extension| extension.to_str()) != Some("qz") {
            continue;
        }
        let contenido = std::fs::read_to_string(&ruta).expect("el ejemplo debería poder leerse");
        let nombre = ruta.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        let fuente = Fuente::nueva(nombre, contenido);
        let resultado = tokenizar(&fuente);
        assert!(
            resultado.is_ok(),
            "el ejemplo {nombre} no tokeniza: {:?}",
            resultado.err()
        );
        archivos_procesados += 1;
    }

    assert!(
        archivos_procesados >= 18,
        "se esperaban al menos 18 ejemplos, se procesaron {archivos_procesados}"
    );
}
