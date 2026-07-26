//! Módulo nativo `quetzal/motor`: el objeto `ExpresiónRegular`.
//!
//! Cubre los criterios de aceptación de la especificación: importación y
//! alias, construcción y validación, inmutabilidad, búsquedas con `nulo`,
//! coincidencias ancladas, reemplazo literal, división, grupos capturados,
//! constructores de conveniencia, banderas, casos borde (texto y patrón
//! vacíos) y mensajes en español.

use std::rc::Rc;

use indexmap::IndexMap;
use maquina_virtual::{Valor, Vm};
use nucleo::Fuente;

fn ejecutar(codigo: &str) -> Rc<maquina_virtual::valores::EntornoModulo> {
    let fuente = Fuente::nueva("prueba.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("el código de prueba debe parsear");
    semantica::analizar_modulo(&ast).expect("el código de prueba debe pasar la semántica");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar bytecode");

    // Resolución mínima de imports nativos (el motor hace esto en producción).
    let mut importaciones = IndexMap::new();
    for importacion in &modulo.importaciones {
        if let Some(nombre_modulo) = importacion.origen.strip_prefix("quetzal/") {
            let modulo_normalizado =
                maquina_virtual::normalizar_nombre(nombre_modulo).to_lowercase();
            for (nombre, local) in &importacion.simbolos {
                let simbolo = maquina_virtual::normalizar_nombre(nombre)
                    .to_lowercase()
                    .replace('_', "");
                let destino = modulos_nativos::modulo_de_tipo_exportado(
                    &modulo_normalizado,
                    &simbolo,
                )
                .map(str::to_string)
                .unwrap_or_else(|| modulo_normalizado.clone());
                importaciones.insert(
                    local.clone(),
                    maquina_virtual::Variable {
                        valor: Valor::ModuloNativo(Rc::from(destino.as_str())),
                        mutable: false,
                    },
                );
            }
        }
    }

    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro()));
    let (entorno, _valor) = vm
        .cargar_modulo(Rc::new(modulo), importaciones)
        .expect("el código de prueba debe ejecutar sin errores");
    entorno
}

fn texto_global(entorno: &maquina_virtual::valores::EntornoModulo, nombre: &str) -> String {
    match &entorno
        .globales
        .borrow()
        .get(nombre)
        .unwrap_or_else(|| panic!("debe existir '{nombre}'"))
        .valor
    {
        Valor::Texto(texto) => texto.to_string(),
        otro => maquina_virtual::texto_de_valor(otro),
    }
}

// CA-01, CA-02: importación y alias con y sin tilde.
#[test]
fn deberia_importarse_con_y_sin_tilde() {
    let entorno = ejecutar(
        "importar { ExpresiónRegular } desde \"quetzal/motor\"\n\
         ExpresiónRegular er = nuevo ExpresiónRegular(\"^a$\")\n\
         log coincide = er.coincide_completo(\"a\")\n",
    );
    assert_eq!(texto_global(&entorno, "coincide"), "verdadero");

    let entorno = ejecutar(
        "importar { ExpresionRegular } desde \"quetzal/motor\"\n\
         ExpresionRegular er = nuevo ExpresionRegular(\"^a$\")\n\
         log coincide = er.coincide_completo(\"a\")\n",
    );
    assert_eq!(texto_global(&entorno, "coincide"), "verdadero");
}

// CA-03, CA-04, CA-15: un patrón inválido lanza excepción en español.
#[test]
fn patron_invalido_deberia_lanzar_excepcion_en_espanol() {
    let entorno = ejecutar(
        "importar { ExpresiónRegular } desde \"quetzal/motor\"\n\
         texto var mensaje = \"\"\n\
         intentar {\n\
             ExpresiónRegular rota = nuevo ExpresiónRegular(\"(sin-cerrar\")\n\
         } capturar (excepcion e) {\n\
             mensaje = e.mensaje\n\
         }\n",
    );
    let mensaje = texto_global(&entorno, "mensaje");
    assert!(
        mensaje.contains("expresión regular inválida"),
        "mensaje en español esperado, se obtuvo: {mensaje}"
    );
}

// CA-07, CA-08: coincidencias ancladas y no ancladas.
#[test]
fn coincide_deberia_respetar_anclajes() {
    let entorno = ejecutar(
        "importar { ExpresiónRegular } desde \"quetzal/motor\"\n\
         ExpresiónRegular er = nuevo ExpresiónRegular(\"hola\")\n\
         log cualquier = er.coincide(\"di hola mundo\")\n\
         log inicio_si = er.coincide_desde_inicio(\"hola mundo\")\n\
         log inicio_no = er.coincide_desde_inicio(\"di hola\")\n\
         log completo_si = er.coincide_completo(\"hola\")\n\
         log completo_no = er.coincide_completo(\"hola mundo\")\n",
    );
    assert_eq!(texto_global(&entorno, "cualquier"), "verdadero");
    assert_eq!(texto_global(&entorno, "inicio_si"), "verdadero");
    assert_eq!(texto_global(&entorno, "inicio_no"), "falso");
    assert_eq!(texto_global(&entorno, "completo_si"), "verdadero");
    assert_eq!(texto_global(&entorno, "completo_no"), "falso");
}

// CA-06, D-04: buscar y buscar_posicion devuelven nulo sin match.
#[test]
fn buscar_sin_match_deberia_devolver_nulo() {
    let entorno = ejecutar(
        "importar { ExpresiónRegular } desde \"quetzal/motor\"\n\
         ExpresiónRegular er = nuevo ExpresiónRegular(\"\\\\d+\")\n\
         log b = er.buscar(\"sin dígitos\") == nulo\n\
         log p = er.buscar_posicion(\"sin dígitos\") == nulo\n\
         texto encontrado = er.buscar(\"hay 42 aquí\")\n\
         jsn pos = er.buscar_posicion(\"hay 42 aquí\")\n\
         entero inicio = pos.inicio\n\
         entero fin = pos.fin\n\
         texto coincidencia = pos.coincidencia\n",
    );
    assert_eq!(texto_global(&entorno, "b"), "verdadero");
    assert_eq!(texto_global(&entorno, "p"), "verdadero");
    assert_eq!(texto_global(&entorno, "encontrado"), "42");
    assert_eq!(texto_global(&entorno, "inicio"), "4");
    assert_eq!(texto_global(&entorno, "fin"), "6");
    assert_eq!(texto_global(&entorno, "coincidencia"), "42");
}

// buscar_todo y contar sobre varias coincidencias.
#[test]
fn buscar_todo_y_contar_deberian_listar_todo() {
    let entorno = ejecutar(
        "importar { ExpresiónRegular } desde \"quetzal/motor\"\n\
         ExpresiónRegular er = nuevo ExpresiónRegular(\"\\\\d+\")\n\
         lista<texto> todos = er.buscar_todo(\"a1 b22 c333\")\n\
         texto primero = todos[0]\n\
         texto tercero = todos[2]\n\
         entero largo = todos.longitud()\n\
         entero conteo = er.contar(\"a1 b22 c333\")\n",
    );
    assert_eq!(texto_global(&entorno, "primero"), "1");
    assert_eq!(texto_global(&entorno, "tercero"), "333");
    assert_eq!(texto_global(&entorno, "largo"), "3");
    assert_eq!(texto_global(&entorno, "conteo"), "3");
}

// CA-11, D-02: buscar_grupos devuelve lista<jsn> con grupos numerados,
// nombrados y nulo para grupos que no participaron.
#[test]
fn buscar_grupos_deberia_exponer_grupos() {
    let entorno = ejecutar(
        "importar { ExpresiónRegular } desde \"quetzal/motor\"\n\
         ExpresiónRegular er = nuevo ExpresiónRegular(\"(?P<anio>\\\\d{4})-(\\\\d{2})\")\n\
         lista<jsn> caps = er.buscar_grupos(\"en 2026-07\")\n\
         jsn cap = caps[0]\n\
         texto completa = cap[\"0\"]\n\
         texto anio = cap.anio\n\
         texto mes = cap[\"2\"]\n\
         lista<texto> nombrados = er.grupos_nombrados()\n\
         texto primer_nombre = nombrados[0]\n",
    );
    assert_eq!(texto_global(&entorno, "completa"), "2026-07");
    assert_eq!(texto_global(&entorno, "anio"), "2026");
    assert_eq!(texto_global(&entorno, "mes"), "07");
    assert_eq!(texto_global(&entorno, "primer_nombre"), "anio");
}

#[test]
fn buscar_grupos_deberia_dar_nulo_a_grupos_sin_participar() {
    let entorno = ejecutar(
        "importar { ExpresiónRegular } desde \"quetzal/motor\"\n\
         ExpresiónRegular er = nuevo ExpresiónRegular(\"(a)|(b)\")\n\
         lista<jsn> caps = er.buscar_grupos(\"a\")\n\
         jsn cap = caps[0]\n\
         texto grupo1 = cap[\"1\"]\n\
         log grupo2_nulo = cap[\"2\"] == nulo\n",
    );
    assert_eq!(texto_global(&entorno, "grupo1"), "a");
    assert_eq!(texto_global(&entorno, "grupo2_nulo"), "verdadero");
}

// CA-09: reemplazar solo la primera, reemplazar_todo todas, literal.
#[test]
fn reemplazar_deberia_ser_literal_y_respetar_cantidad() {
    let entorno = ejecutar(
        "importar { ExpresiónRegular } desde \"quetzal/motor\"\n\
         ExpresiónRegular er = nuevo ExpresiónRegular(\"\\\\d+\")\n\
         texto una = er.reemplazar(\"a1 b2 c3\", \"X\")\n\
         texto todas = er.reemplazar_todo(\"a1 b2 c3\", \"X\")\n\
         texto literal = er.reemplazar(\"a1\", \"\\\\d\")\n\
         texto borrado = er.reemplazar_todo(\"a1 b2\", \"\")\n",
    );
    assert_eq!(texto_global(&entorno, "una"), "aX b2 c3");
    assert_eq!(texto_global(&entorno, "todas"), "aX bX cX");
    // El reemplazo es literal: "\d" se inserta tal cual.
    assert_eq!(texto_global(&entorno, "literal"), "a\\d");
    assert_eq!(texto_global(&entorno, "borrado"), "a b");
}

// CA-10: dividir con y sin coincidencias.
#[test]
fn dividir_deberia_partir_o_devolver_original() {
    let entorno = ejecutar(
        "importar { ExpresiónRegular } desde \"quetzal/motor\"\n\
         ExpresiónRegular er = nuevo ExpresiónRegular(\"[,;]\")\n\
         lista<texto> partes = er.dividir(\"uno,dos;tres\")\n\
         entero largo = partes.longitud()\n\
         texto segunda = partes[1]\n\
         lista<texto> intacta = er.dividir(\"sin separadores\")\n\
         entero largo_intacta = intacta.longitud()\n\
         texto unico = intacta[0]\n",
    );
    assert_eq!(texto_global(&entorno, "largo"), "3");
    assert_eq!(texto_global(&entorno, "segunda"), "dos");
    assert_eq!(texto_global(&entorno, "largo_intacta"), "1");
    assert_eq!(texto_global(&entorno, "unico"), "sin separadores");
}

// CA-12: constructores de conveniencia.
#[test]
fn constructores_de_conveniencia_deberian_funcionar() {
    let entorno = ejecutar(
        "importar { ExpresiónRegular } desde \"quetzal/motor\"\n\
         log dig = ExpresiónRegular.nueva_solo_dígitos().coincide_completo(\"123\")\n\
         log dig_no = ExpresiónRegular.nueva_solo_dígitos().coincide_completo(\"12a\")\n\
         log let_si = ExpresiónRegular.nueva_solo_letras().coincide_completo(\"JoséNuñez\")\n\
         log let_no = ExpresiónRegular.nueva_solo_letras().coincide_completo(\"Ana3\")\n\
         log cor = ExpresiónRegular.nueva_correo().coincide_completo(\"ana@ejemplo.com\")\n\
         log cor_no = ExpresiónRegular.nueva_correo().coincide_completo(\"ana@ejemplo\")\n\
         log url = ExpresiónRegular.nueva_url().coincide_completo(\"https://ejemplo.com/ruta\")\n\
         log ip = ExpresiónRegular.nueva_ipv4().coincide_completo(\"192.168.1.1\")\n\
         log ip_no = ExpresiónRegular.nueva_ipv4().coincide_completo(\"192.168.1\")\n\
         texto esc = ExpresiónRegular.escapar(\"a+b*c\")\n",
    );
    assert_eq!(texto_global(&entorno, "dig"), "verdadero");
    assert_eq!(texto_global(&entorno, "dig_no"), "falso");
    assert_eq!(texto_global(&entorno, "let_si"), "verdadero");
    assert_eq!(texto_global(&entorno, "let_no"), "falso");
    assert_eq!(texto_global(&entorno, "cor"), "verdadero");
    assert_eq!(texto_global(&entorno, "cor_no"), "falso");
    assert_eq!(texto_global(&entorno, "url"), "verdadero");
    assert_eq!(texto_global(&entorno, "ip"), "verdadero");
    assert_eq!(texto_global(&entorno, "ip_no"), "falso");
    assert_eq!(texto_global(&entorno, "esc"), "a\\+b\\*c");
}

#[test]
fn nueva_desde_comodin_deberia_convertir_globs() {
    let entorno = ejecutar(
        "importar { ExpresiónRegular } desde \"quetzal/motor\"\n\
         ExpresiónRegular er = ExpresiónRegular.nueva_desde_comodín(\"*.qz\")\n\
         log coincide_qz = er.coincide_completo(\"motor.qz\")\n\
         log coincide_rs = er.coincide_completo(\"motor.rs\")\n\
         ExpresiónRegular interrogacion = ExpresiónRegular.nueva_desde_comodín(\"nota?.qz\")\n\
         log corto = interrogacion.coincide_completo(\"nota1.qz\")\n\
         log largo = interrogacion.coincide_completo(\"nota12.qz\")\n",
    );
    assert_eq!(texto_global(&entorno, "coincide_qz"), "verdadero");
    assert_eq!(texto_global(&entorno, "coincide_rs"), "falso");
    assert_eq!(texto_global(&entorno, "corto"), "verdadero");
    assert_eq!(texto_global(&entorno, "largo"), "falso");
}

// CA-13, D-01, D-03: banderas con métodos planos, inmutables y encadenables.
#[test]
fn banderas_deberian_ser_inmutables_y_encadenables() {
    let entorno = ejecutar(
        "importar { ExpresiónRegular } desde \"quetzal/motor\"\n\
         ExpresiónRegular base = nuevo ExpresiónRegular(\"^hola$\")\n\
         ExpresiónRegular con_i = base.con_ignorar_mayúsculas(verdadero)\n\
         log original = base.ignorar_mayúsculas()\n\
         log variante = con_i.ignorar_mayúsculas()\n\
         log sin_i = base.coincide(\"HOLA\")\n\
         log con_i_si = con_i.coincide(\"HOLA\")\n\
         ExpresiónRegular cadena = base.con_multilínea(verdadero).con_punto_total(verdadero)\n\
         log m = cadena.multilínea()\n\
         log s = cadena.punto_total()\n\
         log u = cadena.unicode()\n\
         log apagada = base.con_unicode(falso).unicode()\n",
    );
    assert_eq!(texto_global(&entorno, "original"), "falso");
    assert_eq!(texto_global(&entorno, "variante"), "verdadero");
    assert_eq!(texto_global(&entorno, "sin_i"), "falso");
    assert_eq!(texto_global(&entorno, "con_i_si"), "verdadero");
    assert_eq!(texto_global(&entorno, "m"), "verdadero");
    assert_eq!(texto_global(&entorno, "s"), "verdadero");
    // `unicode` nace activada: el motor trabaja sobre textos UTF-8.
    assert_eq!(texto_global(&entorno, "u"), "verdadero");
    assert_eq!(texto_global(&entorno, "apagada"), "falso");
}

// es_válida, diagnosticar y patrón (introspección).
#[test]
fn introspeccion_deberia_funcionar() {
    let entorno = ejecutar(
        "importar { ExpresiónRegular } desde \"quetzal/motor\"\n\
         ExpresiónRegular er = nuevo ExpresiónRegular(\"^\\\\d+$\")\n\
         log valida = er.es_válida()\n\
         jsn diag = er.diagnosticar()\n\
         log diag_valida = diag.valida\n\
         log diag_error = diag.error == nulo\n\
         texto patron = er.patrón()\n",
    );
    assert_eq!(texto_global(&entorno, "valida"), "verdadero");
    assert_eq!(texto_global(&entorno, "diag_valida"), "verdadero");
    assert_eq!(texto_global(&entorno, "diag_error"), "verdadero");
    assert_eq!(texto_global(&entorno, "patron"), r"^\d+$");
}

// CA-14: texto vacío.
#[test]
fn texto_vacio_deberia_tener_comportamiento_definido() {
    let entorno = ejecutar(
        "importar { ExpresiónRegular } desde \"quetzal/motor\"\n\
         ExpresiónRegular er = nuevo ExpresiónRegular(\"\\\\d+\")\n\
         log coincide = er.coincide(\"\")\n\
         log inicio = er.coincide_desde_inicio(\"\")\n\
         log completo = er.coincide_completo(\"\")\n\
         log busca = er.buscar(\"\") == nulo\n\
         log pos = er.buscar_posicion(\"\") == nulo\n\
         lista<texto> todos = er.buscar_todo(\"\")\n\
         entero largo_todos = todos.longitud()\n\
         lista<jsn> grupos = er.buscar_grupos(\"\")\n\
         entero largo_grupos = grupos.longitud()\n\
         entero conteo = er.contar(\"\")\n\
         lista<texto> partes = er.dividir(\"\")\n\
         entero largo_partes = partes.longitud()\n\
         texto primera = partes[0]\n",
    );
    assert_eq!(texto_global(&entorno, "coincide"), "falso");
    assert_eq!(texto_global(&entorno, "inicio"), "falso");
    assert_eq!(texto_global(&entorno, "completo"), "falso");
    assert_eq!(texto_global(&entorno, "busca"), "verdadero");
    assert_eq!(texto_global(&entorno, "pos"), "verdadero");
    assert_eq!(texto_global(&entorno, "largo_todos"), "0");
    assert_eq!(texto_global(&entorno, "largo_grupos"), "0");
    assert_eq!(texto_global(&entorno, "conteo"), "0");
    assert_eq!(texto_global(&entorno, "largo_partes"), "1");
    assert_eq!(texto_global(&entorno, "primera"), "");
}

// CA-14: patrón vacío matchea en cada posición entre caracteres.
#[test]
fn patron_vacio_deberia_matchear_entre_caracteres() {
    let entorno = ejecutar(
        "importar { ExpresiónRegular } desde \"quetzal/motor\"\n\
         ExpresiónRegular er = nuevo ExpresiónRegular(\"\")\n\
         log valida = er.es_válida()\n\
         entero conteo = er.contar(\"abc\")\n\
         lista<texto> partes = er.dividir(\"abc\")\n\
         entero largo = partes.longitud()\n\
         texto primera = partes[0]\n\
         texto segunda = partes[1]\n\
         texto ultima = partes[4]\n",
    );
    assert_eq!(texto_global(&entorno, "valida"), "verdadero");
    assert_eq!(texto_global(&entorno, "conteo"), "4");
    assert_eq!(texto_global(&entorno, "largo"), "5");
    assert_eq!(texto_global(&entorno, "primera"), "");
    assert_eq!(texto_global(&entorno, "segunda"), "a");
    assert_eq!(texto_global(&entorno, "ultima"), "");
}

// buscar_posicion usa índices en caracteres (no bytes) con texto Unicode.
#[test]
fn buscar_posicion_deberia_usar_indices_de_caracteres() {
    let entorno = ejecutar(
        "importar { ExpresiónRegular } desde \"quetzal/motor\"\n\
         ExpresiónRegular er = nuevo ExpresiónRegular(\"b\")\n\
         jsn pos = er.buscar_posicion(\"áéb\")\n\
         entero inicio = pos.inicio\n\
         entero fin = pos.fin\n",
    );
    // "áéb": la 'b' está en el carácter 2, no en el byte 4.
    assert_eq!(texto_global(&entorno, "inicio"), "2");
    assert_eq!(texto_global(&entorno, "fin"), "3");
}
