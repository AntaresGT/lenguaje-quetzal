// Pruebas unitarias para métodos de listas del lenguaje Quetzal

use super::auxiliares::*;

#[test]
fn prueba_longitud_lista() {
    let codigo = r#"
        lista<texto> mi_lista = ["manzana", "banana", "cereza"]
        entero longitud = mi_lista.longitud()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "longitud", 3));
}

#[test]
fn prueba_esta_vacia() {
    let codigo = r#"
        lista<texto> mi_lista = ["manzana", "banana", "cereza"]
        log esta_vacia = mi_lista.esta_vacia()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_logico(&entorno, "esta_vacia", false));
}

#[test]
fn prueba_agregar_elemento() {
    let codigo = r#"
        lista<texto> var lista_mutable = ["manzana", "banana"]
        lista_mutable.agregar("cereza")
        entero longitud = lista_mutable.longitud()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "longitud", 3));
}

#[test]
fn prueba_insertar_elemento() {
    let codigo = r#"
        lista<texto> var lista_mutable = ["manzana", "banana"]
        lista_mutable.insertar(1, "naranja")
        entero longitud = lista_mutable.longitud()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "longitud", 3));
}

#[test]
fn prueba_contiene_elemento() {
    let codigo = r#"
        lista<texto> mi_lista = ["manzana", "banana", "cereza"]
        log contiene_elemento = mi_lista.contiene("banana")
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_logico(&entorno, "contiene_elemento", true));
}

#[test]
fn prueba_buscar_elemento() {
    let codigo = r#"
        lista<texto> mi_lista = ["manzana", "banana", "cereza"]
        entero posicion_elemento = mi_lista.buscar("cereza")
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "posicion_elemento", 2));
}

#[test]
fn prueba_ordenar() {
    let codigo = r#"
        lista<entero> var numeros = [3, 1, 4, 2]
        numeros.ordenar()
        entero primer_elemento = numeros[0]
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "primer_elemento", 1));
}

#[test]
fn prueba_invertir_lista() {
    let codigo = r#"
        lista<entero> var numeros = [1, 2, 3]
        numeros.invertir()
        entero primer_elemento = numeros[0]
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "primer_elemento", 3));
}

#[test]
fn prueba_primero_ultimo() {
    let codigo = r#"
        lista<texto> mi_lista = ["manzana", "banana", "cereza"]
        texto primer_elemento = mi_lista.primero()
        texto ultimo_elemento = mi_lista.ultimo()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "primer_elemento", "manzana"));
    assert!(verificar_variable_texto(&entorno, "ultimo_elemento", "cereza"));
}

#[test]
fn prueba_tomar_saltar() {
    let codigo = r#"
        lista<texto> mi_lista = ["manzana", "banana", "cereza"]
        lista<texto> primeros_n = mi_lista.tomar(2)
        lista<texto> omitidos_n = mi_lista.saltar(1)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_sumar_lista() {
    let codigo = r#"
        lista<entero> numeros_lista = [1, 2, 3, 4, 5]
        entero suma = numeros_lista.sumar()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "suma", 15));
}

#[test]
fn prueba_promedio() {
    let codigo = r#"
        lista<entero> numeros_lista = [1, 2, 3, 4, 5]
        número promedio = numeros_lista.promedio()
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_maximo_minimo() {
    let codigo = r#"
        lista<entero> numeros_lista = [1, 2, 3, 4, 5]
        entero maximo = numeros_lista.maximo()
        entero minimo = numeros_lista.minimo()
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_entero(&entorno, "maximo", 5));
    assert!(verificar_variable_entero(&entorno, "minimo", 1));
}

#[test]
fn prueba_unir_lista() {
    let codigo = r#"
        lista<texto> palabras = ["Hola", "mundo"]
        texto frase = palabras.unir(" ")
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El código debería ejecutarse correctamente");
    assert!(verificar_variable_texto(&entorno, "frase", "Hola mundo"));
}

#[test]
fn prueba_concatenar_listas() {
    let codigo = r#"
        lista<texto> lista1 = ["A", "B"]
        lista<texto> lista2 = ["C", "D"]
        lista<texto> concatenada = lista1.concatenar(lista2)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}
