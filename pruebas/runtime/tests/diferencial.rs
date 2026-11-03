//! Pruebas diferenciales: el intérprete puro y el modo optimizado
//! (especialización + caches) deben producir el mismo estado.

use std::rc::Rc;

use indexmap::IndexMap;
use maquina_virtual::codigo::CacheSitio;
use maquina_virtual::{Valor, Vm};
use nucleo::Fuente;

fn intentar_entorno_con_modo(
    codigo: &str,
    optimizar: bool,
    despacho_rapido: bool,
) -> Result<Rc<maquina_virtual::valores::EntornoModulo>, nucleo::ErrorQuetzal> {
    let fuente = Fuente::nueva("diferencial.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("debe parsear");
    semantica::analizar_modulo(&ast).expect("debe pasar semántica");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar bytecode");
    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro()));
    if !optimizar {
        vm = vm.sin_optimizar();
    }
    if !despacho_rapido {
        vm = vm.sin_despacho_rapido();
    }
    vm.cargar_modulo(Rc::new(modulo), IndexMap::new())
        .map(|(entorno, _)| entorno)
}

fn intentar_entorno(
    codigo: &str,
    optimizar: bool,
) -> Result<Rc<maquina_virtual::valores::EntornoModulo>, nucleo::ErrorQuetzal> {
    intentar_entorno_con_modo(codigo, optimizar, true)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Nivel {
    Referencia,
    OptimizadoSinEspecializacion,
    Optimizado,
    #[cfg(feature = "jit")]
    Jit,
}

fn intentar_entorno_nivel(
    codigo: &str,
    nivel: Nivel,
) -> Result<Rc<maquina_virtual::valores::EntornoModulo>, nucleo::ErrorQuetzal> {
    let fuente = Fuente::nueva("cuatro_niveles.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("debe parsear");
    semantica::analizar_modulo(&ast).expect("debe pasar semántica");
    let modulo = if nivel == Nivel::Referencia {
        let mut modulo = bytecode::generar_modulo_sin_procesar(&ast).expect("debe generar");
        bytecode::verificar_modulo(&mut modulo).expect("el original debe verificar");
        modulo
    } else {
        bytecode::generar_modulo(&ast).expect("debe generar y verificar")
    };
    if nivel == Nivel::Referencia {
        bytecode::verificar_modulo_inmutable(&modulo).expect("la referencia debe ser válida");
    }
    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro()));
    if matches!(
        nivel,
        Nivel::Referencia | Nivel::OptimizadoSinEspecializacion
    ) {
        vm = vm.sin_optimizar();
    }
    if matches!(
        nivel,
        Nivel::Referencia | Nivel::OptimizadoSinEspecializacion
    ) {
        vm = vm.sin_despacho_rapido();
    }
    #[cfg(feature = "jit")]
    if nivel == Nivel::Jit {
        vm = vm.con_jit(true);
    }
    vm.cargar_modulo(Rc::new(modulo), IndexMap::new())
        .map(|(entorno, _)| entorno)
}

fn niveles() -> Vec<Nivel> {
    vec![
        Nivel::OptimizadoSinEspecializacion,
        Nivel::Optimizado,
        #[cfg(feature = "jit")]
        Nivel::Jit,
    ]
}

fn mismos_globales_cuatro_niveles(codigo: &str, nombres: &[&str]) {
    let referencia = intentar_entorno_nivel(codigo, Nivel::Referencia).expect("referencia válida");
    for nivel in niveles() {
        let actual = intentar_entorno_nivel(codigo, nivel).expect("nivel ejecutable");
        for nombre in nombres {
            let esperado = global(&referencia, nombre);
            let valor = global(&actual, nombre);
            assert!(
                valor.es_igual(&esperado),
                "global '{nombre}' difiere en {nivel:?}: actual={valor:?} esperado={esperado:?}\n{codigo}"
            );
        }
    }
}

fn mismos_errores_cuatro_niveles(codigo: &str) {
    let referencia =
        intentar_entorno_nivel(codigo, Nivel::Referencia).expect_err("la referencia debe fallar");
    for nivel in niveles() {
        let actual = intentar_entorno_nivel(codigo, nivel).expect_err("el nivel debe fallar");
        assert_eq!(actual.codigo, referencia.codigo);
        assert_eq!(actual.categoria, referencia.categoria);
        assert_eq!(actual.mensaje, referencia.mensaje);
        assert_eq!(actual.ubicacion, referencia.ubicacion);
    }
}

fn entorno(codigo: &str, optimizar: bool) -> Rc<maquina_virtual::valores::EntornoModulo> {
    intentar_entorno(codigo, optimizar).expect("debe ejecutar")
}

fn global(entorno: &maquina_virtual::valores::EntornoModulo, nombre: &str) -> Valor {
    entorno
        .globales
        .borrow()
        .get(nombre)
        .unwrap_or_else(|| panic!("debe existir '{nombre}'"))
        .valor
        .clone()
}

fn comprobar_globales(codigo: &str, esperados: &[(&str, Valor)]) {
    for (optimizar, despacho_rapido) in [(true, true), (false, true), (true, false)] {
        let entorno = intentar_entorno_con_modo(codigo, optimizar, despacho_rapido)
            .expect("debe ejecutar en los tres modos");
        for (nombre, esperado) in esperados {
            let actual = global(&entorno, nombre);
            assert!(
                actual.es_igual(esperado),
                "global '{nombre}' difiere con optimizar={optimizar} y rapido={despacho_rapido}: actual={actual:?} esperado={esperado:?}"
            );
        }
    }
}

fn mismos_globales(codigo: &str, nombres: &[&str]) {
    let con = entorno(codigo, true);
    let sin = entorno(codigo, false);
    let generico = intentar_entorno_con_modo(codigo, true, false).expect("debe ejecutar");
    for nombre in nombres {
        let a = global(&con, nombre);
        let b = global(&sin, nombre);
        let esperado = global(&generico, nombre);
        assert!(
            a.es_igual(&esperado) && b.es_igual(&esperado),
            "global '{nombre}' difiere: optimizar={a:?} puro={b:?} generico={esperado:?}"
        );
    }
}

fn mismos_errores(codigo: &str) {
    let con = match intentar_entorno(codigo, true) {
        Ok(_) => panic!("el optimizado debe fallar"),
        Err(error) => error,
    };
    let sin = match intentar_entorno(codigo, false) {
        Ok(_) => panic!("el intérprete puro debe fallar"),
        Err(error) => error,
    };
    let generico = match intentar_entorno_con_modo(codigo, true, false) {
        Ok(_) => panic!("el despachador general debe fallar"),
        Err(error) => error,
    };
    for error in [&con, &sin] {
        assert_eq!(error.codigo, generico.codigo);
        assert_eq!(error.categoria, generico.categoria);
        assert_eq!(error.mensaje, generico.mensaje);
        assert_eq!(error.ubicacion, generico.ubicacion);
    }
}

#[test]
fn diferencial_deberia_coincidir_en_aritmetica_y_bucles() {
    mismos_globales(
        "entero var suma = 0\npara (entero var i = 0; i < 200; i++) {\n    suma += i * 2\n}\n",
        &["suma"],
    );
}

#[test]
fn diferencial_deberia_coincidir_en_fibonacci() {
    mismos_globales(
        "entero fib(entero n) {\n    si (n < 2) { retornar n }\n    retornar fib(n - 1) + fib(n - 2)\n}\nentero resultado = fib(12)\n",
        &["resultado"],
    );
}

#[test]
fn diferencial_deberia_coincidir_en_objetos() {
    mismos_globales(
        "objeto Caja {\n    entero var n = 1\n    entero ver() { retornar esto.n }\n}\nCaja var c = nuevo Caja()\nc.n = 9\nentero resultado = c.ver()\n",
        &["resultado"],
    );
}

#[test]
fn diferencial_deberia_coincidir_en_listas_y_textos() {
    mismos_globales(
        "lista<entero> nums = [1, 2, 3]\nentero largo = nums.longitud()\ntexto t = \"Hola\".mayusculas()\n",
        &["largo", "t"],
    );
}

#[test]
fn diferencial_atributo_deberia_validar_la_clase_tras_especializar() {
    mismos_globales(
        r#"
objeto A {
    publico:
        entero var valor = 11
}
objeto B {
    publico:
        entero var otro = 22
        entero var valor = 33
}
jsn var actual = { objeto: nuevo A() }
entero var resultado = 0
para (entero var i = 0; i < 1002; i++) {
    si (i == 1001) { actual.objeto = nuevo B() }
    resultado = actual.objeto.valor
}
"#,
        &["resultado"],
    );
}

#[test]
fn diferencial_binaria_deberia_volver_al_camino_generico() {
    mismos_globales(
        r#"
jsn var valores = { a: 1, b: 2 }
número var resultado = 0
para (entero var i = 0; i < 1002; i++) {
    si (i == 1001) { valores.a = 0.5 }
    resultado = valores.a + valores.b
}
"#,
        &["resultado"],
    );
}

#[test]
fn diferencial_nativa_deberia_validar_el_receptor_tras_especializar() {
    mismos_globales(
        r#"
jsn var actual = { valor: "a" }
entero var resultado = 0
para (entero var i = 0; i < 1002; i++) {
    si (i == 1001) { actual.valor = [1, 2, 3] }
    resultado = actual.valor.longitud()
}
"#,
        &["resultado"],
    );
}

#[test]
fn diferencial_atributo_deberia_soportar_alternancia_polimorfica() {
    mismos_globales(
        r#"
objeto A {
    publico:
        entero var valor = 11
}
objeto B {
    publico:
        entero var otro = 22
        entero var valor = 33
}
jsn var actual = { objeto: nuevo A() }
entero var resultado = 0
para (entero var i = 0; i < 1001; i++) {
    si (i % 2 == 0) {
        actual.objeto = nuevo A()
    } sino {
        actual.objeto = nuevo B()
    }
    resultado = actual.objeto.valor
}
"#,
        &["resultado"],
    );
}

#[test]
fn diferencial_especializacion_deberia_respetar_los_umbrales() {
    for cantidad in [1, 10, 999, 1000, 1001] {
        let codigo = format!(
            r#"
jsn var valores = {{ a: 1, b: 2 }}
número var resultado = 0
para (entero var i = 0; i < {cantidad}; i++) {{
    si (i == {cantidad} - 1) {{ valores.a = 0.5 }}
    resultado = valores.a + valores.b
}}
"#
        );
        mismos_globales(&codigo, &["resultado"]);
    }
}

#[test]
fn diferencial_error_deberia_conservar_la_operacion_generica() {
    mismos_errores(
        r#"
jsn var valores = { a: 1, b: 2 }
entero var resultado = 0
para (entero var i = 0; i < 1001; i++) {
    si (i == 1000) { valores.a = verdadero }
    resultado = valores.a - valores.b
}
"#,
    );
}

#[test]
fn diferencial_atributo_constante_no_deberia_mutarse_por_camino_dinamico() {
    mismos_errores(
        r#"
objeto Caja {
    publico:
        entero valor = 1
}
jsn var actual = { objeto: nuevo Caja() }
actual.objeto.valor = 2
"#,
    );
}

#[test]
fn diferencial_atributo_libre_constante_no_deberia_mutarse() {
    mismos_errores(
        r#"
objeto Configuracion {
    publico:
        libre entero valor = 1
}
jsn var actual = { objeto: Configuracion }
actual.objeto.valor = 2
"#,
    );
}

#[test]
fn diferencial_deberia_coincidir_en_el_camino_rapido() {
    mismos_globales(
        r#"
entero incrementar(entero a, entero b) {
    retornar a + b
}
entero var resultado = 40000
para (entero var i = 0; i < 50; i++) {
    si (i % 2 == 0) {
        resultado += incrementar(i, 1)
    } sino {
        resultado += i
    }
}
si (resultado > 40000) {
    resultado += 1
} sino {
    resultado += 2
}
"#,
        &["resultado"],
    );
}

#[test]
fn diferencial_llamadas_deberian_limpiar_la_pila_en_retornos_y_excepciones() {
    mismos_globales(
        r#"
entero sumar_uno(entero n) {
    retornar n + 1
}
entero fib(entero n) {
    si (n < 2) { retornar n }
    retornar fib(n - 1) + fib(n - 2)
}
entero fallar() {
    lanzar "fallo esperado"
}
entero recuperar(entero n) {
    intentar {
        retornar fallar()
    } capturar (excepcion e) {
        retornar sumar_uno(n)
    }
}
entero normal = sumar_uno(41)
entero recursivo = fib(10)
entero recuperado = recuperar(41)
"#,
        &["normal", "recursivo", "recuperado"],
    );
}

#[test]
fn diferencial_desbordamiento_del_camino_rapido_deberia_ser_capturable() {
    comprobar_globales(
        r#"
texto var mensaje = "sin capturar"
vacio provocar() {
    entero var contador = 9223372036854775807
    intentar {
        contador++
    } capturar (excepcion e) {
        mensaje = e.mensaje
    }
}
provocar()
"#,
        &[(
            "mensaje",
            Valor::texto("desbordamiento de entero al ajustar 9223372036854775807 en 1"),
        )],
    );
}

#[test]
fn diferencial_error_de_tipo_del_camino_rapido_deberia_ser_capturable() {
    comprobar_globales(
        r#"
entero var capturas = 0
vacio provocar() {
    jsn valores = { condicion: 1 }
    intentar {
        si (valores.condicion) {
            capturas = 10
        }
    } capturar (excepcion e) {
        capturas += 1
    }
}
provocar()
"#,
        &[("capturas", Valor::Entero(1))],
    );
}

#[test]
fn diferencial_division_por_cero_deberia_conservar_la_captura() {
    comprobar_globales(
        r#"
texto var mensaje = ""
vacio provocar() {
    entero divisor = 0
    intentar {
        entero resultado = 10 / divisor
    } capturar (excepcion e) {
        mensaje = e.mensaje
    }
}
provocar()
"#,
        &[("mensaje", Valor::texto("división por cero"))],
    );
}

#[test]
fn diferencial_excepciones_anidadas_deberian_conservar_el_orden() {
    comprobar_globales(
        r#"
texto var mensaje = ""
vacio provocar() {
    entero var contador = 9223372036854775807
    intentar {
        intentar {
            contador++
        } capturar (excepcion e) {
            mensaje = "interna"
            lanzar "externa"
        }
    } capturar (excepcion e) {
        mensaje += ":" + e.mensaje
    }
}
provocar()
"#,
        &[("mensaje", Valor::texto("interna:externa"))],
    );
}

#[test]
fn diferencial_excepcion_en_funcion_llamada_deberia_limpiar_su_marco() {
    comprobar_globales(
        r#"
entero var resultado = 0
vacio fallar() {
    entero var contador = 9223372036854775807
    contador++
}
vacio llamar() {
    intentar {
        fallar()
    } capturar (excepcion e) {
        resultado = 42
    }
}
llamar()
"#,
        &[("resultado", Valor::Entero(42))],
    );
}

#[test]
fn diferencial_excepciones_en_bucle_deberian_permitir_continuar() {
    comprobar_globales(
        r#"
entero var capturas = 0
vacio provocar() {
    entero var contador = 9223372036854775807
    entero delta = 1
    para (entero var i = 0; i < 3; i++) {
        intentar {
            contador += delta
        } capturar (excepcion e) {
            capturas += 1
        }
    }
}
provocar()
"#,
        &[("capturas", Valor::Entero(3))],
    );
}

#[test]
fn diferencial_retorno_despues_de_excepcion_deberia_usar_locales_validos() {
    comprobar_globales(
        r#"
entero recuperar() {
    entero var contador = 9223372036854775807
    entero var resultado = 41
    intentar {
        contador++
    } capturar (excepcion e) {
        resultado += 1
    }
    retornar resultado
}
entero resultado = recuperar()
"#,
        &[("resultado", Valor::Entero(42))],
    );
}

#[test]
fn diferencial_captura_deberia_limpiar_los_locales_de_la_iteracion() {
    comprobar_globales(
        r#"
entero recuperar() {
    entero var resultado = 7
    para (entero var i = 0; i < 1; i++) {
        entero var contador = 9223372036854775807
        intentar {
            contador++
        } capturar (excepcion e) {
            resultado += 1
        }
    }
    entero posterior = 34
    retornar resultado + posterior
}
entero resultado = recuperar()
"#,
        &[("resultado", Valor::Entero(42))],
    );
}

#[test]
fn diferencial_excepcion_deberia_limpiar_los_marcos_recursivos() {
    comprobar_globales(
        r#"
entero descender(entero n) {
    si (n == 0) {
        entero var contador = 9223372036854775807
        contador++
        retornar 0
    }
    retornar descender(n - 1)
}
entero recuperar() {
    intentar {
        retornar descender(8)
    } capturar (excepcion e) {
        retornar 108
    }
}
entero resultado = recuperar()
"#,
        &[("resultado", Valor::Entero(108))],
    );
}

#[test]
fn diferencial_ejecucion_deberia_continuar_despues_de_capturar() {
    comprobar_globales(
        r#"
entero var resultado = 1
vacio provocar() {
    entero var contador = 9223372036854775807
    intentar {
        contador++
    } capturar (excepcion e) {
        resultado += 10
    }
    resultado += 100
}
provocar()
resultado += 1000
"#,
        &[("resultado", Valor::Entero(1111))],
    );
}

#[test]
fn diferencial_suma_de_locales_deberia_conservar_el_desbordamiento() {
    comprobar_globales(
        r#"
texto var mensaje = ""
vacio provocar() {
    entero izquierda = 9223372036854775807
    entero derecha = 1
    intentar {
        entero resultado = izquierda + derecha
    } capturar (excepcion e) {
        mensaje = e.mensaje
    }
}
provocar()
"#,
        &[(
            "mensaje",
            Valor::texto("desbordamiento de entero al sumar 9223372036854775807 y 1"),
        )],
    );
}

#[test]
fn diferencial_atributos_deberian_conservar_dos_entradas_inline() {
    let entorno = entorno(
        r#"
objeto A { publico: entero var valor = 11 }
objeto B { publico: entero var otro = 22 entero var valor = 33 }
A primero = nuevo A()
B segundo = nuevo B()
entero ejecutar() {
    jsn var actual = { objeto: primero }
    entero var suma = 0
    para (entero var i = 0; i < 100; i++) {
        si (i % 2 == 0) { actual.objeto = primero } sino { actual.objeto = segundo }
        suma += actual.objeto.valor
    }
    retornar suma
}
entero resultado = ejecutar()
"#,
        true,
    );
    assert!(global(&entorno, "resultado").es_igual(&Valor::Entero(2200)));
    let runtimes = entorno.runtimes.borrow();
    let codigo = runtimes.get("ejecutar").expect("runtime de ejecutar");
    assert!(
        codigo
            .caches
            .iter()
            .any(|cache| matches!(cache.get(), CacheSitio::AtributosPolimorficos { .. }))
    );
}

#[test]
fn diferencial_atributos_deberian_recuperar_especializacion_tras_tercera_clase() {
    let entorno = entorno(
        r#"
objeto A { publico: entero var valor = 11 }
objeto B { publico: entero var otro = 22 entero var valor = 33 }
objeto C { publico: entero var x = 44 entero var extra = 55 entero var valor = 66 }
A primero = nuevo A()
B segundo = nuevo B()
C tercero = nuevo C()
entero ejecutar() {
    jsn var actual = { objeto: primero }
    entero var resultado = 0
    para (entero var i = 0; i < 90; i++) {
        si (i % 3 == 0) {
            actual.objeto = primero
        } sino si (i % 3 == 1) {
            actual.objeto = segundo
        } sino {
            actual.objeto = tercero
        }
        resultado = actual.objeto.valor
    }
    para (entero var i = 0; i < 64; i++) {
        actual.objeto = primero
        resultado = actual.objeto.valor
    }
    retornar resultado
}
entero resultado = ejecutar()
"#,
        true,
    );
    assert!(global(&entorno, "resultado").es_igual(&Valor::Entero(11)));
    let runtimes = entorno.runtimes.borrow();
    let codigo = runtimes.get("ejecutar").expect("runtime de ejecutar");
    assert!(
        codigo
            .caches
            .iter()
            .any(|cache| matches!(cache.get(), CacheSitio::Atributo { .. }))
    );
}

#[test]
fn diferencial_decimales_deberian_conservar_precision_en_operaciones_especializadas() {
    comprobar_globales(
        r#"
número var valor = 10.0
valor += 0.5
valor *= 2.0
valor -= 1.0
valor /= 4.0
valor %= 2.0
número exacto = 0.1 + 0.2
"#,
        &[
            (
                "valor",
                Valor::numero("1.0".parse().expect("decimal válido")),
            ),
            (
                "exacto",
                Valor::numero("0.3".parse().expect("decimal válido")),
            ),
        ],
    );
}

#[test]
fn diferencial_division_decimal_por_cero_deberia_conservar_el_error() {
    mismos_errores("número resultado = 1.0 / 0.0\n");
}

#[test]
fn diferencial_modulo_decimal_por_cero_deberia_conservar_el_error() {
    mismos_errores("número resultado = 1.0 % 0.0\n");
}

#[test]
fn diferencial_nombres_nativos_deberian_conservar_su_valor_y_cache() {
    let codigo = r#"
entero ejecutar() {
    entero var total = 0
    para (entero var i = 0; i < 10; i++) {
        si (rango == rango) {
            total += 100
        }
        total += rango(1, 3).longitud()
    }
    retornar total
}
entero resultado = ejecutar()
"#;
    comprobar_globales(codigo, &[("resultado", Valor::Entero(30))]);
    let entorno = entorno(codigo, true);
    let runtimes = entorno.runtimes.borrow();
    let runtime = runtimes.get("ejecutar").expect("runtime de ejecutar");
    assert!(
        runtime
            .caches
            .iter()
            .any(|cache| matches!(cache.get(), CacheSitio::Externa { .. }))
    );
}

#[test]
fn diferencial_cache_sitio_deberia_permanecer_compacta() {
    assert_eq!(std::mem::size_of::<CacheSitio>(), 24);
}

#[test]
fn cuatro_niveles_deberian_conservar_aritmetica_funciones_y_bucles_calientes() {
    mismos_globales_cuatro_niveles(
        r#"
entero combinar(entero a, entero b, entero c, entero d, entero e) {
    entero var total = a - b + c * d - e
    para (entero var i = 0; i < 2000; i++) {
        si (i % 3 == 0) { total += i } sino { total -= 1 }
    }
    retornar total
}
entero primero = combinar(1, 2, 3, 4, 5)
entero segundo = combinar(2, 3, 4, 5, 6)
entero tercero = combinar(3, 4, 5, 6, 7)
entero cuarto = combinar(4, 5, 6, 7, 8)
entero resultado = combinar(5, 6, 7, 8, 9)
"#,
        &["primero", "segundo", "tercero", "cuarto", "resultado"],
    );
}

#[test]
fn cuatro_niveles_deberian_conservar_objetos_unicode_listas_y_jsn() {
    mismos_globales_cuatro_niveles(
        r#"
objeto Base { publico: entero var valor = 11 }
objeto Derivada hereda Base {
    publico:
        entero doble() { retornar esto.valor * 2 }
}
Derivada instancia = nuevo Derivada()
lista<texto> textos = ["Quetzal🦜", "áéíóú"]
jsn datos = { texto: textos[0], cantidad: textos.longitud(), valor: instancia.doble() }
texto resultado = datos.texto + ":" + datos.cantidad.texto() + ":" + datos.valor.texto()
"#,
        &["resultado"],
    );
}

#[test]
fn cuatro_niveles_deberian_conservar_excepciones_finalmente_y_efectos() {
    mismos_globales_cuatro_niveles(
        r#"
texto var efectos = ""
vacio registrar(texto valor) { efectos += valor }
vacio ejecutar() {
    intentar {
        registrar("inicio;")
        lanzar "esperado"
    } capturar (excepcion e) {
        registrar("captura;")
    } finalmente {
        registrar("fin;")
    }
}
ejecutar()
"#,
        &["efectos"],
    );
}

#[test]
fn cuatro_niveles_deberian_conservar_asincronia() {
    mismos_globales_cuatro_niveles(
        r#"
asincrono entero duplicar(entero valor) { retornar valor * 2 }
asincrono entero cuadruplicar(entero valor) {
    entero doble = esperar duplicar(valor)
    retornar esperar duplicar(doble)
}
entero resultado = esperar cuadruplicar(10)
"#,
        &["resultado"],
    );
}

#[test]
fn cuatro_niveles_deberian_conservar_errores_tras_calentar_jit() {
    mismos_errores_cuatro_niveles(
        r#"
entero dividir(entero a, entero b) { retornar a / b }
entero a = dividir(10, 2)
entero b = dividir(10, 2)
entero c = dividir(10, 2)
entero d = dividir(10, 2)
entero resultado = dividir(1, 0)
"#,
    );
}

#[test]
fn cuatro_niveles_deberian_coincidir_en_programas_generados_con_semillas() {
    for semilla in 0..32i64 {
        let a = semilla % 7 - 3;
        let b = semilla % 5 + 1;
        let limite = semilla % 11 + 1;
        let codigo = format!(
            "entero var total = {a}\npara (entero var i = 0; i < {limite}; i++) {{\n    total = total * 2 + {b} - i\n}}\nentero resultado = total\n"
        );
        mismos_globales_cuatro_niveles(&codigo, &["resultado"]);
    }
}
