//! Ejecución de programas Quetzal en la VM: el estado final de las variables
//! globales del módulo sirve como verificación.

use std::rc::Rc;
use std::str::FromStr;

use indexmap::IndexMap;
use maquina_virtual::{Valor, Vm};
use nucleo::Fuente;
use rust_decimal::Decimal;

/// Ejecuta el código y devuelve la VM y el entorno del módulo.
fn ejecutar(codigo: &str) -> Rc<maquina_virtual::valores::EntornoModulo> {
    intentar_ejecutar(codigo).expect("el código de prueba debe ejecutar sin errores")
}

fn intentar_ejecutar(
    codigo: &str,
) -> Result<Rc<maquina_virtual::valores::EntornoModulo>, nucleo::ErrorQuetzal> {
    let fuente = Fuente::nueva("prueba.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("el código de prueba debe parsear");
    semantica::analizar_modulo(&ast).expect("el código de prueba debe pasar la semántica");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar bytecode");
    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro()));
    vm.cargar_modulo(Rc::new(modulo), IndexMap::new())
        .map(|(entorno, _valor)| entorno)
}

/// Valor final de una variable global del módulo.
fn global(entorno: &maquina_virtual::valores::EntornoModulo, nombre: &str) -> Valor {
    entorno
        .globales
        .borrow()
        .get(nombre)
        .unwrap_or_else(|| panic!("debe existir la global '{nombre}'"))
        .valor
        .clone()
}

fn como_entero(valor: &Valor) -> i64 {
    match valor {
        Valor::Entero(entero) => *entero,
        otro => panic!("se esperaba un entero, se obtuvo {otro:?}"),
    }
}

fn como_texto(valor: &Valor) -> String {
    match valor {
        Valor::Texto(texto) => texto.to_string(),
        otro => panic!("se esperaba un texto, se obtuvo {otro:?}"),
    }
}

#[test]
fn ejecutar_deberia_evaluar_aritmetica_con_precedencia() {
    let entorno = ejecutar("entero resultado = 2 + 3 * 4 - 10 / 2\n");
    assert_eq!(como_entero(&global(&entorno, "resultado")), 9);
}

#[test]
fn ejecutar_deberia_sumar_decimales_exactos() {
    // La razón de usar rust_decimal: 0.1 + 0.2 debe ser exactamente 0.3.
    let entorno = ejecutar("número resultado = 0.1 + 0.2\nlog exacto = resultado == 0.3\n");
    assert!(global(&entorno, "exacto").es_igual(&Valor::Log(true)));
    match global(&entorno, "resultado") {
        Valor::Numero(decimal) => {
            assert_eq!(decimal, Decimal::from_str("0.3").expect("decimal válido"));
        }
        otro => panic!("se esperaba número, se obtuvo {otro:?}"),
    }
}

#[test]
fn ejecutar_deberia_fallar_con_desbordamiento_de_entero() {
    let error =
        intentar_ejecutar("entero grande = 9223372036854775807\nentero malo = grande + 1\n")
            .expect_err("el desbordamiento debe ser un error");
    assert_eq!(error.codigo, "E0402");
}

#[test]
fn ejecutar_deberia_fallar_con_division_por_cero() {
    let error = intentar_ejecutar("entero malo = 10 / 0\n").expect_err("debe fallar");
    assert_eq!(error.codigo, "E0401");
}

#[test]
fn ejecutar_deberia_llamar_funciones_y_recursion() {
    let entorno = ejecutar(
        "entero factorial(entero n) {\n    si (n <= 1) {\n        retornar 1\n    }\n    retornar n * factorial(n - 1)\n}\n\nentero resultado = factorial(6)\n",
    );
    assert_eq!(como_entero(&global(&entorno, "resultado")), 720);
}

#[test]
fn ejecutar_deberia_interpolar_textos() {
    let entorno = ejecutar(
        "texto nombre = \"Quetzal\"\ntexto saludo = t\"Hola {nombre}, 2 + 2 = {2 + 2}\"\n",
    );
    assert_eq!(
        como_texto(&global(&entorno, "saludo")),
        "Hola Quetzal, 2 + 2 = 4"
    );
}

#[test]
fn ejecutar_deberia_controlar_flujo_con_si_y_mientras() {
    let entorno = ejecutar(
        "entero var suma = 0\nentero var i = 1\nmientras (i <= 10) {\n    si (i % 2 == 0) {\n        suma += i\n    }\n    i++\n}\n",
    );
    assert_eq!(como_entero(&global(&entorno, "suma")), 30);
}

#[test]
fn ejecutar_deberia_recorrer_listas_con_para() {
    let entorno = ejecutar(
        "lista<entero> numeros = [1, 2, 3, 4]\nentero var total = 0\npara (entero n en numeros) {\n    total += n\n}\n",
    );
    assert_eq!(como_entero(&global(&entorno, "total")), 10);
}

#[test]
fn ejecutar_deberia_indexar_con_negativos() {
    let entorno =
        ejecutar("lista<texto> letras = [\"a\", \"b\", \"c\"]\ntexto ultima = letras[-1]\n");
    assert_eq!(como_texto(&global(&entorno, "ultima")), "c");
}

#[test]
fn ejecutar_deberia_acceder_a_jsn_por_punto_y_corchete() {
    let entorno = ejecutar(
        "jsn persona = {\n    nombre: \"Ana\",\n    edad: 30\n}\ntexto nombre = persona.nombre\nentero edad = persona[\"edad\"]\n",
    );
    assert_eq!(como_texto(&global(&entorno, "nombre")), "Ana");
    assert_eq!(como_entero(&global(&entorno, "edad")), 30);
}

#[test]
fn ejecutar_deberia_capturar_excepciones() {
    let entorno = ejecutar(
        "texto var mensaje = \"\"\nintentar {\n    entero malo = 10 / 0\n} capturar (excepcion e) {\n    mensaje = e.mensaje\n} finalmente {\n    mensaje += \" [fin]\"\n}\n",
    );
    assert_eq!(
        como_texto(&global(&entorno, "mensaje")),
        "división por cero [fin]"
    );
}

#[test]
fn ejecutar_deberia_lanzar_excepciones_propias() {
    let error = intentar_ejecutar("lanzar \"algo salió mal\"\n").expect_err("debe fallar");
    assert_eq!(error.codigo, "E0405");
    assert!(error.mensaje.contains("algo salió mal"));
}

#[test]
fn ejecutar_deberia_instanciar_objetos_con_esto() {
    let entorno = ejecutar(
        "objeto Contador {\n    publico:\n        entero var valor = 0\n\n        Contador(entero inicial) {\n            esto.valor = inicial\n        }\n\n        vacio incrementar() {\n            esto.valor++\n        }\n}\n\nContador contador = nuevo Contador(5)\ncontador.incrementar()\ncontador.incrementar()\nentero final = contador.valor\n",
    );
    assert_eq!(como_entero(&global(&entorno, "final")), 7);
}

#[test]
fn ejecutar_deberia_usar_operador_ternario() {
    let entorno = ejecutar("entero edad = 20\ntexto tipo = edad >= 18 ? \"mayor\" : \"menor\"\n");
    assert_eq!(como_texto(&global(&entorno, "tipo")), "mayor");
}

#[test]
fn ejecutar_deberia_cortocircuitar_logicos() {
    let entorno = ejecutar("log a = falso && (10 / 0 == 0)\nlog b = verdadero || (10 / 0 == 0)\n");
    assert!(global(&entorno, "a").es_igual(&Valor::Log(false)));
    assert!(global(&entorno, "b").es_igual(&Valor::Log(true)));
}

#[test]
fn ejecutar_deberia_cortocircuitar_palabras_logicas_y_o() {
    let entorno = ejecutar("log a = falso y (10 / 0 == 0)\nlog b = verdadero o (10 / 0 == 0)\n");
    assert!(global(&entorno, "a").es_igual(&Valor::Log(false)));
    assert!(global(&entorno, "b").es_igual(&Valor::Log(true)));
}

#[test]
fn ejecutar_deberia_evaluar_palabras_logicas_y_o() {
    let entorno = ejecutar(
        "entero edad = 30\nlog adulto = edad >= 18 y edad <= 65\nlog extremo = edad < 18 o edad > 65\n",
    );
    assert!(global(&entorno, "adulto").es_igual(&Valor::Log(true)));
    assert!(global(&entorno, "extremo").es_igual(&Valor::Log(false)));
}
