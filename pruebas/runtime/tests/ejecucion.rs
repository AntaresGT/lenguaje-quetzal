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
fn funcion_asincrona_deberia_resolverse_con_esperar() {
    let entorno = ejecutar(
        "asincrono entero duplicar(entero valor) {\n\
         \x20   retornar valor * 2\n\
         }\n\
         entero resultado = esperar duplicar(21)\n",
    );
    assert_eq!(como_entero(&global(&entorno, "resultado")), 42);
}

#[test]
fn esperar_sobre_un_valor_normal_lo_devuelve_sin_cambios() {
    // `esperar` sobre algo que no es una tarea simplemente pasa el valor,
    // para que el código pueda tratar funciones síncronas y asincrónicas de
    // forma uniforme.
    let entorno = ejecutar("entero resultado = esperar 7\n");
    assert_eq!(como_entero(&global(&entorno, "resultado")), 7);
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

// ----- Bucle de eventos: funciones `asincrono` y `esperar` -----

#[test]
fn funcion_asincrona_con_tilde_deberia_resolverse_con_esperar() {
    // La palabra reservada acepta tilde: `asíncrono` ≡ `asincrono`.
    let entorno = ejecutar(
        "asíncrono entero duplicar(entero valor) {\n    retornar valor * 2\n}\nentero resultado = esperar duplicar(21)\n",
    );
    assert_eq!(como_entero(&global(&entorno, "resultado")), 42);
}

#[test]
fn funcion_asincrona_vacio_deberia_poder_esperarse() {
    // Una función asíncrona `vacio` puede esperarse: simplemente ejecuta su
    // cuerpo y el `esperar` devuelve `nulo`.
    let entorno = ejecutar(
        "entero var contador = 0\nasincrono vacio incrementar() {\n    contador++\n}\nesperar incrementar()\nesperar incrementar()\nentero final = contador\n",
    );
    assert_eq!(como_entero(&global(&entorno, "final")), 2);
}

#[test]
fn esperar_secuncial_ejecuta_en_orden() {
    // Múltiples `esperar` consecutivos se resuelven en orden de aparición.
    let entorno = ejecutar(
        "asincrono entero sumar(entero a, entero b) {\n    retornar a + b\n}\nentero primero = esperar sumar(1, 2)\nentero segundo = esperar sumar(primero, 10)\nentero tercero = esperar sumar(segundo, 100)\n",
    );
    assert_eq!(como_entero(&global(&entorno, "primero")), 3);
    assert_eq!(como_entero(&global(&entorno, "segundo")), 13);
    assert_eq!(como_entero(&global(&entorno, "tercero")), 113);
}

#[test]
fn funcion_asincrona_puede_esperar_a_otra_funcion_asincrona() {
    let entorno = ejecutar(
        "asincrono entero duplicar(entero valor) {\n    retornar valor * 2\n}\nasincrono entero cuadruplicar(entero valor) {\n    entero doble = esperar duplicar(valor)\n    retornar esperar duplicar(doble)\n}\nentero resultado = esperar cuadruplicar(5)\n",
    );
    assert_eq!(como_entero(&global(&entorno, "resultado")), 20);
}

#[test]
fn esperar_puede_usarse_en_expresiones_complejas() {
    // `esperar` no solo se asigna a variables: puede combinarse con
    // operaciones aritméticas, comparaciones, argumentos, etc.
    let entorno = ejecutar(
        "asincrono entero duplicar(entero valor) {\n    retornar valor * 2\n}\nentero total = esperar duplicar(10) + esperar duplicar(5)\nlog grande = esperar duplicar(30) > 50\n",
    );
    assert_eq!(como_entero(&global(&entorno, "total")), 30);
    assert!(global(&entorno, "grande").es_igual(&Valor::Log(true)));
}

#[test]
fn metodo_asincrono_de_instancia_deberia_resolverse_con_esperar() {
    let entorno = ejecutar(
        "objeto Calculadora {\n    publico:\n        asincrono entero duplicar(entero valor) {\n            retornar valor * 2\n        }\n}\nCalculadora calc = nuevo Calculadora()\nentero resultado = esperar calc.duplicar(21)\n",
    );
    assert_eq!(como_entero(&global(&entorno, "resultado")), 42);
}

#[test]
fn metodo_libre_asincrono_deberia_resolverse_con_esperar() {
    // Un método `libre` asíncrono se llama sin instancia, directamente sobre
    // la clase.
    let entorno = ejecutar(
        "objeto Util {\n    publico:\n        libre asincrono entero triplicar(entero valor) {\n            retornar valor * 3\n        }\n}\nentero resultado = esperar Util.triplicar(7)\n",
    );
    assert_eq!(como_entero(&global(&entorno, "resultado")), 21);
}

#[test]
fn metodo_asincrono_puede_usar_esto() {
    // Un método asíncrono de instancia conserva el receptor `esto`.
    let entorno = ejecutar(
        "objeto Acumulador {\n    publico:\n        entero var total = 0\n        Acumulador(entero inicial) {\n            esto.total = inicial\n        }\n        asincrono vacio agregar(entero cantidad) {\n            esto.total += cantidad\n        }\n}\nAcumulador acumulador = nuevo Acumulador(10)\nesperar acumulador.agregar(5)\nesperar acumulador.agregar(7)\nentero final = acumulador.total\n",
    );
    assert_eq!(como_entero(&global(&entorno, "final")), 22);
}

#[test]
fn llamar_funcion_asincrona_sin_esperar_devuelve_tarea_pendiente() {
    // Llamar una función `asincrono` sin `esperar` devuelve una tarea
    // pendiente (fire-and-forget): la tarea existe, no es el valor final.
    let entorno = ejecutar(
        "asincrono entero duplicar(entero valor) {\n    retornar valor * 2\n}\njsn r = {\n    tarea: duplicar(21),\n    resultado: esperar duplicar(21)\n}\n",
    );
    let tarea = global(&entorno, "r");
    match tarea {
        Valor::Jsn(mapa) => {
            let mapa = mapa.borrow();
            let tarea = mapa.get("tarea").expect("debe existir la clave 'tarea'");
            assert!(
                matches!(tarea, Valor::Tarea(_)),
                "llamar sin `esperar` debe producir una tarea pendiente, no {}",
                tarea.nombre_tipo()
            );
            let resultado = mapa.get("resultado").expect("debe existir la clave 'resultado'");
            assert_eq!(como_entero(resultado), 42);
        }
        otro => panic!("se esperaba jsn, se obtuvo {otro:?}"),
    }
}

#[test]
fn excepcion_en_funcion_asincrona_se_propaga_al_esperar() {
    // Si una función asíncrona lanza, el `esperar` propaga la excepción al
    // contexto llamante, donde puede capturarse.
    let entorno = ejecutar(
        "asincrono entero explotar() {\n    lanzar \"boom\"\n}\ntexto var mensaje = \"\"\nintentar {\n    esperar explotar()\n} capturar (excepcion e) {\n    mensaje = e.mensaje\n}\n",
    );
    assert_eq!(como_texto(&global(&entorno, "mensaje")), "boom");
}

#[test]
fn esperar_en_bucle_se_resuelve_secuncialmente() {
    // Cada iteración del bucle espera a la tarea de esa iteración antes de
    // continuar: el orden de ejecución es predecible.
    let entorno = ejecutar(
        "asincrono entero duplicar(entero valor) {\n    retornar valor * 2\n}\nlista var resultados = []\npara (entero i en [1, 2, 3]) {\n    entero doble = esperar duplicar(i)\n    resultados.agregar(doble)\n}\n",
    );
    let resultados = global(&entorno, "resultados");
    match resultados {
        Valor::Lista(elementos) => {
            let elementos = elementos.borrow();
            let enteros: Vec<i64> = elementos.iter().map(como_entero).collect();
            assert_eq!(enteros, vec![2, 4, 6]);
        }
        otro => panic!("se esperaba lista, se obtuvo {otro:?}"),
    }
}

#[test]
fn esperar_en_condicional_funciona() {
    let entorno = ejecutar(
        "asincrono entero obtener() {\n    retornar 42\n}\nentero var resultado = 0\nsi (verdadero) {\n    resultado = esperar obtener()\n}\n",
    );
    assert_eq!(como_entero(&global(&entorno, "resultado")), 42);
}

#[test]
fn esperar_funciona_desde_el_scope_global_tras_declarar_funciones() {
    // El patrón típico: definir varias funciones asíncronas y usarlas todas
    // desde el scope global, una tras otra.
    let entorno = ejecutar(
        "asincrono entero sumar(entero a, entero b) {\n    retornar a + b\n}\nasincrono entero multiplicar(entero a, entero b) {\n    retornar a * b\n}\nasincrono texto formatear(entero valor) {\n    retornar \"resultado: \" + valor.texto()\n}\nentero suma = esperar sumar(3, 4)\nentero producto = esperar multiplicar(suma, 2)\ntexto mensaje = esperar formatear(producto)\n",
    );
    assert_eq!(como_entero(&global(&entorno, "suma")), 7);
    assert_eq!(como_entero(&global(&entorno, "producto")), 14);
    assert_eq!(como_texto(&global(&entorno, "mensaje")), "resultado: 14");
}
