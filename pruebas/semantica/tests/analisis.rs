//! Pruebas del análisis semántico del Lenguaje Quetzal.

use nucleo::{ErrorQuetzal, Fuente};
use semantica::analizar_modulo;
use sintaxis::parsear_modulo;

fn analizar(codigo: &str) -> Result<(), Vec<ErrorQuetzal>> {
    let fuente = Fuente::nueva("prueba.qz", codigo);
    let modulo = parsear_modulo(&fuente).expect("el código de prueba debería parsear");
    analizar_modulo(&modulo)
}

fn primer_codigo(resultado: Result<(), Vec<ErrorQuetzal>>) -> String {
    let errores = resultado.expect_err("se esperaban errores semánticos");
    errores
        .first()
        .map(|error| error.codigo.clone())
        .unwrap_or_default()
}

#[test]
fn analizar_deberia_aceptar_codigo_valido() {
    let resultado = analizar(
        "entero var contador = 0\nmientras (contador < 10) {\n    contador++\n}\nconsola.mostrar(contador)",
    );
    assert!(resultado.is_ok(), "errores: {:?}", resultado.err());
}

#[test]
fn analizar_deberia_rechazar_variable_no_definida() {
    assert_eq!(
        primer_codigo(analizar("consola.mostrar(inexistente)")),
        "E0201"
    );
}

#[test]
fn analizar_deberia_rechazar_reasignacion_de_constante() {
    assert_eq!(
        primer_codigo(analizar("entero edad = 30\nedad = 31")),
        "E0203"
    );
}

#[test]
fn analizar_deberia_rechazar_duplicados() {
    assert_eq!(
        primer_codigo(analizar("entero a = 1\ntexto a = \"hola\"")),
        "E0202"
    );
}

#[test]
fn analizar_deberia_rechazar_tipos_incompatibles() {
    assert_eq!(primer_codigo(analizar("texto nombre = 123")), "E0204");
}

#[test]
fn analizar_deberia_aceptar_nulo_en_cualquier_tipo() {
    assert!(analizar("entero a = nulo\ntexto b = nulo\njsn c = nulo").is_ok());
}

#[test]
fn analizar_deberia_aceptar_entero_promovido_a_numero() {
    assert!(analizar("número precio = 3").is_ok());
}

#[test]
fn analizar_deberia_rechazar_variable_de_tipo_vacio() {
    assert_eq!(primer_codigo(analizar("vacio x = nulo")), "E0206");
}

#[test]
fn analizar_deberia_rechazar_retorno_invalido() {
    assert_eq!(
        primer_codigo(analizar(
            "entero sumar(entero a) {\n    retornar \"texto\"\n}"
        )),
        "E0205"
    );
}

#[test]
fn analizar_deberia_rechazar_retorno_con_valor_en_vacio() {
    assert_eq!(
        primer_codigo(analizar("vacio saludar() {\n    retornar 5\n}")),
        "E0205"
    );
}

#[test]
fn analizar_deberia_rechazar_argumentos_de_mas() {
    assert_eq!(
        primer_codigo(analizar(
            "entero sumar(entero a, entero b) {\n    retornar a + b\n}\nentero r = sumar(1, 2, 3)"
        )),
        "E0210"
    );
}

#[test]
fn analizar_deberia_rechazar_romper_fuera_de_bucle() {
    assert_eq!(primer_codigo(analizar("romper")), "E0211");
}

#[test]
fn analizar_deberia_rechazar_miembro_privado() {
    let codigo = "objeto Usuario {\n    privado:\n        texto var nombre\n    publico:\n        Usuario(texto nombre) {\n            esto.nombre = nombre\n        }\n}\nUsuario u = nuevo Usuario(\"Ana\")\nconsola.mostrar(u.nombre)";
    assert_eq!(primer_codigo(analizar(codigo)), "E0207");
}

#[test]
fn analizar_deberia_permitir_inicializar_constante_en_constructor() {
    let codigo = "objeto Usuario {\n    privado:\n        texto nombre\n    publico:\n        Usuario(texto nombre) {\n            esto.nombre = nombre\n        }\n        texto obtener_nombre() {\n            retornar esto.nombre\n        }\n}\nUsuario u = nuevo Usuario(\"Ana\")";
    let resultado = analizar(codigo);
    assert!(resultado.is_ok(), "errores: {:?}", resultado.err());
}

#[test]
fn analizar_deberia_rechazar_prototipo_incompleto() {
    let codigo = "prototipo Persistible {\n    texto tabla\n    jsn serializar()\n}\nobjeto Usuario implementa Persistible {\n    texto tabla = \"usuarios\"\n}";
    assert_eq!(primer_codigo(analizar(codigo)), "E0208");
}

#[test]
fn analizar_deberia_aceptar_miembro_opcional_sin_implementar() {
    let codigo = "prototipo Contrato {\n    texto nombre\n    log opcional validar(texto dato)\n}\nobjeto Servicio implementa Contrato {\n    texto nombre = \"servicio\"\n}";
    let resultado = analizar(codigo);
    assert!(resultado.is_ok(), "errores: {:?}", resultado.err());
}

#[test]
fn analizar_deberia_rechazar_modulo_nativo_inexistente() {
    assert_eq!(
        primer_codigo(analizar("importar { Algo } desde \"quetzal/inexistente\"")),
        "E0301"
    );
}

#[test]
fn analizar_deberia_aceptar_modulo_nativo_con_tilde() {
    assert!(
        analizar("importar { Matemática } desde \"quetzal/matemática\"\nnúmero pi = Matemática.PI")
            .is_ok()
    );
}

#[test]
fn analizar_deberia_rechazar_exportacion_inexistente() {
    assert_eq!(primer_codigo(analizar("exportar { fantasma }")), "E0302");
}

#[test]
fn analizar_deberia_rechazar_herencia_de_objeto_inexistente() {
    assert_eq!(
        primer_codigo(analizar("objeto Perro hereda Animal {\n}")),
        "E0201"
    );
}

#[test]
fn analizar_deberia_rechazar_mutacion_de_jsn_constante() {
    assert_eq!(
        primer_codigo(analizar(
            "jsn persona = { nombre: \"Ana\" }\npersona.nombre = \"María\""
        )),
        "E0203"
    );
}

#[test]
fn analizar_deberia_aceptar_mutacion_de_jsn_mutable() {
    assert!(analizar("jsn var persona = { nombre: \"Ana\" }\npersona.nombre = \"María\"").is_ok());
}

#[test]
fn analizar_deberia_aceptar_esperar_en_scope_global() {
    // En el scope global (top-level) `esperar` está permitido: es el punto
    // de entrada al bucle de eventos.
    let codigo = "asincrono entero duplicar(entero valor) {\n    retornar valor * 2\n}\nentero resultado = esperar duplicar(21)";
    let resultado = analizar(codigo);
    assert!(resultado.is_ok(), "errores: {:?}", resultado.err());
}

#[test]
fn analizar_deberia_aceptar_esperar_dentro_de_funcion_asincrona() {
    let codigo = "asincrono entero duplicar(entero valor) {\n    retornar valor * 2\n}\nasincrono vacio tarea() {\n    entero resultado = esperar duplicar(10)\n    consola.mostrar(resultado)\n}";
    let resultado = analizar(codigo);
    assert!(resultado.is_ok(), "errores: {:?}", resultado.err());
}

#[test]
fn analizar_deberia_rechazar_esperar_dentro_de_funcion_sincrona() {
    let codigo = "asincrono entero duplicar(entero valor) {\n    retornar valor * 2\n}\nentero calculo() {\n    entero resultado = esperar duplicar(10)\n    retornar resultado\n}";
    assert_eq!(primer_codigo(analizar(codigo)), "E0213");
}

#[test]
fn analizar_deberia_rechazar_esperar_en_metodo_sincrono_de_objeto() {
    let codigo = "asincrono entero duplicar(entero valor) {\n    retornar valor * 2\n}\nobjeto Util {\n    publico:\n        entero calcular() {\n            entero r = esperar duplicar(5)\n            retornar r\n        }\n}";
    assert_eq!(primer_codigo(analizar(codigo)), "E0213");
}

#[test]
fn analizar_deberia_aceptar_esperar_en_metodo_asincrono_de_objeto() {
    let codigo = "asincrono entero duplicar(entero valor) {\n    retornar valor * 2\n}\nobjeto Util {\n    publico:\n        asincrono entero calcular() {\n            entero r = esperar duplicar(5)\n            retornar r\n        }\n}";
    let resultado = analizar(codigo);
    assert!(resultado.is_ok(), "errores: {:?}", resultado.err());
}

#[test]
fn analizar_deberia_aceptar_esperar_en_metodo_libre_asincrono() {
    let codigo = "asincrono entero duplicar(entero valor) {\n    retornar valor * 2\n}\nobjeto Util {\n    publico:\n        libre asincrono entero calcular() {\n            entero r = esperar duplicar(5)\n            retornar r\n        }\n}";
    let resultado = analizar(codigo);
    assert!(resultado.is_ok(), "errores: {:?}", resultado.err());
}

#[test]
fn analizar_deberia_rechazar_esperar_en_constructor() {
    // Los constructores son síncronos por definición: no pueden usar `esperar`.
    let codigo = "asincrono entero duplicar(entero valor) {\n    retornar valor * 2\n}\nobjeto Util {\n    publico:\n        entero var valor = 0\n        Util(entero inicial) {\n            esto.valor = esperar duplicar(inicial)\n        }\n}";
    assert_eq!(primer_codigo(analizar(codigo)), "E0213");
}

#[test]
fn analizar_deberia_aceptar_funcion_asincrona_con_retorno_vacio() {
    // Las funciones asíncronas pueden retornar `vacio` (no devuelven valor).
    let codigo = "asincrono vacio tarea() {\n    consola.mostrar(\"hola\")\n}";
    let resultado = analizar(codigo);
    assert!(resultado.is_ok(), "errores: {:?}", resultado.err());
}

#[test]
fn analizar_deberia_aceptar_esperar_en_expresiones_complejas() {
    // `esperar` puede usarse en cualquier contexto de expresión, no solo en
    // la asignación a una variable.
    let codigo = "asincrono entero duplicar(entero valor) {\n    retornar valor * 2\n}\nasincrono entero cuadruplicar(entero valor) {\n    retornar esperar duplicar(valor) * 2\n}\nentero resultado = esperar cuadruplicar(5)";
    let resultado = analizar(codigo);
    assert!(resultado.is_ok(), "errores: {:?}", resultado.err());
}

#[test]
fn analizar_deberia_procesar_todos_los_ejemplos() {
    let directorio = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ejemplos");
    let entradas = std::fs::read_dir(directorio).expect("debería existir ejemplos/");

    for entrada in entradas.flatten() {
        let ruta = entrada.path();
        if ruta.extension().and_then(|extension| extension.to_str()) != Some("qz") {
            continue;
        }
        let contenido = std::fs::read_to_string(&ruta).expect("el ejemplo debería poder leerse");
        let nombre = ruta.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        // Los ejemplos de la raíz son archivos sueltos: los imports relativos
        // se resuelven a nivel de proyecto, así que aquí solo validamos que el
        // análisis no produzca falsos positivos.
        let fuente = Fuente::nueva(nombre, contenido);
        let modulo = parsear_modulo(&fuente).expect("el ejemplo debería parsear");
        let resultado = analizar_modulo(&modulo);
        assert!(
            resultado.is_ok(),
            "el ejemplo {nombre} produce errores semánticos: {:?}",
            resultado.err().map(|errores| errores
                .iter()
                .map(|error| format!("{} {}", error.codigo, error.mensaje))
                .collect::<Vec<_>>())
        );
    }
}
