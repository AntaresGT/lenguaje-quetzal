//! Pruebas del índice semántico persistente para herramientas.

use ast::Visibilidad;
use nucleo::Fuente;
use semantica::indice::{ClaseAmbito, ClaseSimbolo, ResultadoAnalisis, SimboloAnalizado};
use semantica::{TipoSemantico, analizar_modulo_para_herramientas};
use sintaxis::parsear_modulo;

fn analizar(codigo: &str) -> ResultadoAnalisis {
    let fuente = Fuente::nueva("indice.qz", codigo);
    let modulo = parsear_modulo(&fuente).expect("el código debe ser válido");
    analizar_modulo_para_herramientas(&modulo)
}

fn simbolo<'a>(
    resultado: &'a ResultadoAnalisis,
    nombre: &str,
    clase: ClaseSimbolo,
) -> &'a SimboloAnalizado {
    resultado
        .simbolos
        .iter()
        .find(|simbolo| simbolo.nombre == nombre && simbolo.clase == clase)
        .expect("debe existir el símbolo")
}

#[test]
fn indice_deberia_conservar_tipo_mutabilidad_y_referencia() {
    let resultado = analizar("entero var contador = 0\ncontador += 1");
    let contador = simbolo(&resultado, "contador", ClaseSimbolo::Variable);
    assert_eq!(contador.tipo, TipoSemantico::Entero);
    assert!(contador.mutable);
    assert_eq!(resultado.referencias_de(contador.id).count(), 1);
}

#[test]
fn indice_deberia_distinguir_variables_sombreadas() {
    let resultado = analizar(
        "entero valor = 1\nvacio ejecutar() {\n texto valor = \"local\"\n consola.mostrar(valor)\n}\nconsola.mostrar(valor)",
    );
    let variables: Vec<_> = resultado
        .simbolos
        .iter()
        .filter(|simbolo| simbolo.nombre == "valor")
        .collect();
    assert_eq!(variables.len(), 2);
    assert_ne!(variables[0].id, variables[1].id);
    assert_eq!(resultado.referencias_de(variables[0].id).count(), 1);
    assert_eq!(resultado.referencias_de(variables[1].id).count(), 1);
}

#[test]
fn indice_deberia_conservar_firma_y_parametros() {
    let resultado = analizar("entero sumar(entero a, texto b) { retornar a }");
    let funcion = simbolo(&resultado, "sumar", ClaseSimbolo::Funcion);
    let firma = funcion.firma.as_ref().expect("debe tener firma");
    assert_eq!(firma.parametros.len(), 2);
    assert_eq!(firma.retorno, TipoSemantico::Entero);
    assert_eq!(firma.parametros[0].nombre, "a");
    assert_eq!(
        simbolo(&resultado, "a", ClaseSimbolo::Parametro).clase,
        ClaseSimbolo::Parametro
    );
}

#[test]
fn indice_deberia_conservar_ambitos_despues_de_cerrar_bloques() {
    let resultado = analizar(
        "vacio ejecutar() {\n entero valor = 1\n si (verdadero) {\n  texto interior = \"sí\"\n }\n}",
    );
    let interior = simbolo(&resultado, "interior", ClaseSimbolo::Variable);
    let ambito = &resultado.ambitos[interior.ambito as usize];
    assert_eq!(ambito.clase, ClaseAmbito::Bloque);
    assert!(ambito.padre.is_some());
}

#[test]
fn indice_deberia_registrar_bucles_y_capturas() {
    let resultado = analizar(
        "vacio ejecutar() {\n mientras (falso) {}\n intentar {} capturar (excepcion e) {}\n}",
    );
    assert!(
        resultado
            .ambitos
            .iter()
            .any(|ambito| ambito.clase == ClaseAmbito::Bucle)
    );
    assert!(
        resultado
            .ambitos
            .iter()
            .any(|ambito| ambito.clase == ClaseAmbito::Captura)
    );
}

#[test]
fn indice_deberia_registrar_objetos_herencia_miembros_y_visibilidad() {
    let resultado = analizar(
        "objeto Base {}\nobjeto Usuario hereda Base {\n privado:\n texto nombre\n publico:\n texto saludar(texto quien) { retornar quien }\n}",
    );
    let objeto = resultado
        .objetos
        .iter()
        .find(|objeto| objeto.nombre == "Usuario")
        .expect("debe existir Usuario");
    assert_eq!(objeto.padres, ["Base"]);
    assert_eq!(objeto.miembros.len(), 2);
    let nombre = simbolo(&resultado, "nombre", ClaseSimbolo::Atributo);
    assert_eq!(nombre.visibilidad, Some(Visibilidad::Privado));
    let metodo = simbolo(&resultado, "saludar", ClaseSimbolo::Metodo);
    assert_eq!(metodo.visibilidad, Some(Visibilidad::Publico));
    assert!(metodo.firma.is_some());
}

#[test]
fn indice_deberia_vincular_importaciones_y_exportaciones() {
    let resultado =
        analizar("importar { Matematica como Mat } desde \"quetzal/matematica\"\nexportar { Mat }");
    let importado = simbolo(&resultado, "Mat", ClaseSimbolo::Importacion);
    assert_eq!(resultado.importaciones[0].simbolos, [importado.id]);
    assert_eq!(resultado.exportaciones[0].simbolos, [importado.id]);
}

#[test]
fn indice_deberia_compartir_diagnosticos_del_analizador() {
    let resultado = analizar("entero valor = \"texto\"");
    assert_eq!(resultado.diagnosticos[0].codigo, "E0204");
    assert!(simbolo(&resultado, "valor", ClaseSimbolo::Variable).id > 0);
}

#[test]
fn indice_deberia_resolver_referencia_a_atributo_heredado() {
    let resultado = analizar(
        "objeto Base { publico:\n entero var saldo }\nobjeto Usuario hereda Base {}\nUsuario usuario = nuevo Usuario()\nconsola.mostrar(usuario.saldo)",
    );
    let atributo = simbolo(&resultado, "saldo", ClaseSimbolo::Atributo);
    assert_eq!(resultado.referencias_de(atributo.id).count(), 1);
}
