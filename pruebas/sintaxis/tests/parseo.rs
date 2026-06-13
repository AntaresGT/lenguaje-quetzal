//! Pruebas del parser del Lenguaje Quetzal.

use ast::{Elemento, Modulo, NodoExpresion, NodoSentencia, SegmentoInterpolado, Tipo};
use nucleo::Fuente;
use sintaxis::parsear_modulo;

fn parsear(codigo: &str) -> Modulo {
    let fuente = Fuente::nueva("prueba.qz", codigo);
    match parsear_modulo(&fuente) {
        Ok(modulo) => modulo,
        Err(error) => panic!("no debería fallar el parseo: {error} ({:?})", error),
    }
}

#[test]
fn parsear_deberia_aceptar_declaracion_constante() {
    let modulo = parsear("entero edad = 30");
    let Elemento::Sentencia(sentencia) = &modulo.elementos[0] else {
        panic!("se esperaba una sentencia");
    };
    let NodoSentencia::DeclaracionVariable {
        tipo,
        mutable,
        nombre,
        valor,
    } = &sentencia.nodo
    else {
        panic!("se esperaba una declaración");
    };
    assert_eq!(tipo, &Tipo::Entero);
    assert!(!mutable);
    assert_eq!(nombre, "edad");
    assert!(valor.is_some());
}

#[test]
fn parsear_deberia_aceptar_lista_tipada_mutable() {
    let modulo = parsear(r#"lista<texto> var frutas = ["manzana"]"#);
    let Elemento::Sentencia(sentencia) = &modulo.elementos[0] else {
        panic!("se esperaba una sentencia");
    };
    let NodoSentencia::DeclaracionVariable { tipo, mutable, .. } = &sentencia.nodo else {
        panic!("se esperaba una declaración");
    };
    assert_eq!(tipo, &Tipo::Lista(Some(Box::new(Tipo::Texto))));
    assert!(mutable);
}

#[test]
fn parsear_deberia_aceptar_funcion_con_parametro_mutable() {
    let modulo = parsear(
        "texto agregar(texto var palabra) {\n    palabra += \" agregado\"\n    retornar palabra\n}",
    );
    let Elemento::Funcion(funcion) = &modulo.elementos[0] else {
        panic!("se esperaba una función");
    };
    assert_eq!(funcion.nombre, "agregar");
    assert_eq!(funcion.tipo_retorno, Tipo::Texto);
    assert!(funcion.parametros[0].mutable);
}

#[test]
fn parsear_deberia_aceptar_funcion_asincrona_con_tilde() {
    let modulo = parsear("asincróno entero duplicar(entero valor) {\n    retornar valor * 2\n}");
    let Elemento::Funcion(funcion) = &modulo.elementos[0] else {
        panic!("se esperaba una función");
    };
    assert!(funcion.asincrona);
}

#[test]
fn parsear_deberia_aceptar_sino_si_encadenado() {
    parsear(
        "si (edad > 60) {\n    consola.mostrar(\"a\")\n} sino si (edad > 18) {\n    consola.mostrar(\"b\")\n} sino {\n    consola.mostrar(\"c\")\n}",
    );
}

#[test]
fn parsear_deberia_aceptar_ternario() {
    let modulo = parsear(r#"texto estado = edad >= 18 ? "adulto" : "menor""#);
    assert_eq!(modulo.elementos.len(), 1);
}

#[test]
fn parsear_deberia_aceptar_bucles() {
    parsear("mientras (contador < 10) {\n    contador++\n}");
    parsear("para (entero var i = 0; i < 5; i++) {\n    consola.mostrar(i)\n}");
    parsear("hacer {\n    contador++\n} mientras (contador < 3)");
    parsear("para (entero var valor en lista_numeros) {\n    consola.mostrar(valor)\n}");
    parsear("para (entero var valor cada lista_numeros) {\n    consola.mostrar(valor)\n}");
}

#[test]
fn parsear_deberia_aceptar_excepciones() {
    parsear(
        "intentar {\n    número resultado = dividir(10, 0)\n} capturar (excepcion e) {\n    consola.mostrar(e.mensaje)\n} finalmente {\n    consola.mostrar(\"Fin\")\n}",
    );
}

#[test]
fn parsear_deberia_aceptar_objeto_con_visibilidad() {
    let modulo = parsear(
        "objeto Usuario {\n    privado:\n        texto var nombre\n    publico:\n        Usuario(texto nombre) {\n            esto.nombre = nombre\n        }\n        texto obtener_nombre() {\n            retornar esto.nombre\n        }\n}",
    );
    let Elemento::Objeto(objeto) = &modulo.elementos[0] else {
        panic!("se esperaba un objeto");
    };
    assert_eq!(objeto.nombre, "Usuario");
    assert_eq!(objeto.miembros.len(), 3);
}

#[test]
fn parsear_deberia_aceptar_herencia_multiple() {
    let modulo = parsear("objeto Felino hereda Mamifero, Animal {\n}");
    let Elemento::Objeto(objeto) = &modulo.elementos[0] else {
        panic!("se esperaba un objeto");
    };
    assert_eq!(objeto.padres, vec!["Mamifero", "Animal"]);
}

#[test]
fn parsear_deberia_aceptar_como_e_implementa() {
    let modulo = parsear("objeto CuentaInterna como EntidadAuditable implementa Autenticable {\n}");
    let Elemento::Objeto(objeto) = &modulo.elementos[0] else {
        panic!("se esperaba un objeto");
    };
    assert_eq!(objeto.extiende_como, vec!["EntidadAuditable"]);
    assert_eq!(objeto.prototipos, vec!["Autenticable"]);
}

#[test]
fn parsear_deberia_aceptar_prototipo_con_opcionales() {
    let modulo = parsear(
        "prototipo Contrato {\n    entero atributoA\n    log opcional funcionOpcional(entero z)\n    texto opcional var atributoMutable2\n}",
    );
    let Elemento::Prototipo(prototipo) = &modulo.elementos[0] else {
        panic!("se esperaba un prototipo");
    };
    assert_eq!(prototipo.miembros.len(), 3);
    assert!(prototipo.miembros[1].opcional);
    assert!(prototipo.miembros[2].opcional);
}

#[test]
fn parsear_deberia_aceptar_importacion_con_alias() {
    let modulo =
        parsear("importar {\n    sumar,\n    saludo como texto_saludo\n} desde \"exportar.qz\"");
    let Elemento::Importacion(importacion) = &modulo.elementos[0] else {
        panic!("se esperaba una importación");
    };
    assert_eq!(importacion.origen, "exportar.qz");
    assert_eq!(importacion.simbolos[1].nombre_local(), "texto_saludo");
}

#[test]
fn parsear_deberia_aceptar_texto_interpolado_con_expresiones() {
    let modulo = parsear(r#"consola.mostrar(t"La suma de {a} + {b} es {a + b}")"#);
    let Elemento::Sentencia(sentencia) = &modulo.elementos[0] else {
        panic!("se esperaba una sentencia");
    };
    let NodoSentencia::Expresion(expresion) = &sentencia.nodo else {
        panic!("se esperaba una expresión");
    };
    let NodoExpresion::Llamada { argumentos, .. } = &expresion.nodo else {
        panic!("se esperaba una llamada");
    };
    let NodoExpresion::TextoInterpolado(segmentos) = &argumentos[0].nodo else {
        panic!("se esperaba un texto interpolado");
    };
    let expresiones = segmentos
        .iter()
        .filter(|segmento| matches!(segmento, SegmentoInterpolado::Expresion(_)))
        .count();
    assert_eq!(expresiones, 3);
}

#[test]
fn parsear_deberia_aceptar_jsn_con_claves_con_espacios() {
    parsear("jsn persona = {\n    nombre: \"Ana\",\n    \"clave con espacio\": \"valor\"\n}");
}

#[test]
fn parsear_deberia_aceptar_conversion_sobre_literales() {
    parsear("texto desde_entero = 123.texto()");
    parsear("entero desde_texto = \"123\".entero()");
    parsear("número desde_decimal = 1234.56.número()");
}

#[test]
fn parsear_deberia_aceptar_indices_negativos() {
    parsear("texto ultima = frutas[-1]");
}

#[test]
fn parsear_deberia_rechazar_codigo_invalido() {
    let fuente = Fuente::nueva("prueba.qz", "entero = 5");
    let error = parsear_modulo(&fuente).expect_err("debería fallar");
    assert!(
        error.codigo.starts_with("E01"),
        "se esperaba un error sintáctico, se obtuvo {}",
        error.codigo
    );
}

#[test]
fn parsear_deberia_procesar_todos_los_ejemplos() {
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
        let resultado = parsear_modulo(&fuente);
        assert!(
            resultado.is_ok(),
            "el ejemplo {nombre} no parsea: {}",
            resultado
                .err()
                .map(|error| format!("{error} en {:?}", error.ubicacion))
                .unwrap_or_default()
        );
        archivos_procesados += 1;
    }

    assert!(
        archivos_procesados >= 18,
        "se esperaban al menos 18 ejemplos, se procesaron {archivos_procesados}"
    );
}
