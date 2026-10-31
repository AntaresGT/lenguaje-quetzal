//! Pruebas de conversión UTF-8 ↔ UTF-16.

use lsp::documentos::ConvertidorPosiciones;
use lsp_types::{Position, Range};
use nucleo::Ubicacion;

fn posicion(line: u32, character: u32) -> Position {
    Position { line, character }
}

#[test]
fn offset_ascii_deberia_convertir_a_posicion() {
    let convertidor = ConvertidorPosiciones::nuevo("abc\nxy");
    assert_eq!(convertidor.offset_a_posicion(5), posicion(1, 1));
}

#[test]
fn posicion_ascii_deberia_convertir_a_offset() {
    let convertidor = ConvertidorPosiciones::nuevo("abc\nxy");
    assert_eq!(convertidor.posicion_a_offset(posicion(1, 1)), Some(5));
}

#[test]
fn tildes_y_enie_deberian_contar_una_unidad_utf16() {
    let contenido = "áñ";
    let convertidor = ConvertidorPosiciones::nuevo(contenido);
    assert_eq!(
        convertidor.offset_a_posicion(contenido.len()),
        posicion(0, 2)
    );
}

#[test]
fn emoji_deberia_contar_par_sustituto() {
    let contenido = "a😀b";
    let convertidor = ConvertidorPosiciones::nuevo(contenido);
    assert_eq!(convertidor.offset_a_posicion("a😀".len()), posicion(0, 3));
}

#[test]
fn posicion_dentro_de_par_sustituto_deberia_iniciar_en_caracter() {
    let convertidor = ConvertidorPosiciones::nuevo("a😀b");
    assert_eq!(convertidor.posicion_a_offset(posicion(0, 2)), Some(1));
}

#[test]
fn caracter_combinado_deberia_conservar_offset_utf8() {
    let contenido = "e\u{301}x";
    let convertidor = ConvertidorPosiciones::nuevo(contenido);
    assert_eq!(convertidor.posicion_a_offset(posicion(0, 2)), Some(3));
}

#[test]
fn crlf_no_deberia_contar_como_caracteres_de_linea() {
    let convertidor = ConvertidorPosiciones::nuevo("uno\r\ndos\r\n");
    assert_eq!(convertidor.offset_a_posicion(3), posicion(0, 3));
    assert_eq!(convertidor.posicion_a_offset(posicion(0, 99)), Some(3));
}

#[test]
fn lineas_vacias_deberian_conservar_sus_offsets() {
    let convertidor = ConvertidorPosiciones::nuevo("\n\n");
    assert_eq!(convertidor.posicion_a_offset(posicion(1, 0)), Some(1));
    assert_eq!(convertidor.posicion_a_offset(posicion(2, 0)), Some(2));
}

#[test]
fn final_de_documento_deberia_convertir_en_ambas_direcciones() {
    let contenido = "añ\n";
    let convertidor = ConvertidorPosiciones::nuevo(contenido);
    assert_eq!(
        convertidor.offset_a_posicion(contenido.len()),
        posicion(1, 0)
    );
    assert_eq!(
        convertidor.posicion_a_offset(posicion(1, 0)),
        Some(contenido.len())
    );
}

#[test]
fn linea_inexistente_deberia_rechazarse() {
    let convertidor = ConvertidorPosiciones::nuevo("una línea");
    assert_eq!(convertidor.posicion_a_offset(posicion(1, 0)), None);
}

#[test]
fn offset_dentro_de_caracter_deberia_desplazarse_a_su_inicio() {
    let convertidor = ConvertidorPosiciones::nuevo("a😀b");
    assert_eq!(convertidor.offset_a_posicion(3), posicion(0, 1));
}

#[test]
fn offset_fuera_de_rango_deberia_limitarse_al_final() {
    let contenido = "ñ";
    let convertidor = ConvertidorPosiciones::nuevo(contenido);
    assert_eq!(convertidor.offset_a_posicion(100), posicion(0, 1));
}

#[test]
fn ubicacion_unicode_deberia_convertir_a_rango_lsp() {
    let contenido = "entero ñ = 😀";
    let inicio = contenido.find('😀').expect("debe existir emoji");
    let convertidor = ConvertidorPosiciones::nuevo(contenido);
    assert_eq!(
        convertidor.ubicacion_a_rango(Ubicacion::nueva(inicio, contenido.len())),
        Range {
            start: posicion(0, 11),
            end: posicion(0, 13),
        }
    );
}

#[test]
fn rango_lsp_deberia_convertir_a_ubicacion_utf8() {
    let contenido = "a😀b";
    let convertidor = ConvertidorPosiciones::nuevo(contenido);
    assert_eq!(
        convertidor.rango_a_ubicacion(Range {
            start: posicion(0, 1),
            end: posicion(0, 3),
        }),
        Some(Ubicacion::nueva(1, 5))
    );
}

#[test]
fn rango_invertido_deberia_rechazarse() {
    let convertidor = ConvertidorPosiciones::nuevo("abc");
    assert_eq!(
        convertidor.rango_a_ubicacion(Range {
            start: posicion(0, 2),
            end: posicion(0, 1),
        }),
        None
    );
}

#[test]
fn fragmento_sin_guardar_deberia_convertirse_sin_disco() {
    let convertidor = ConvertidorPosiciones::nuevo("texto mensaje = \"ño 😀\"");
    assert_eq!(convertidor.posicion_a_offset(posicion(0, 22)), Some(25));
}
