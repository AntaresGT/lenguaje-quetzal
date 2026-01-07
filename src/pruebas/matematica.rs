// Pruebas unitarias para el modulo de matematicas del lenguaje Quetzal

use super::auxiliares::*;

#[test]
fn prueba_importar_matematica() {
    let codigo = r#"
        importar {
            Matematica
        } desde "quetzal/matematica"
        
        numero resultado = Matematica.sumar(2, 3)
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El codigo deberia ejecutarse correctamente");
    assert!(obtener_valor_variable(&entorno, "resultado").is_some());
}

#[test]
fn prueba_constantes_matematica() {
    let codigo = r#"
        numero pi = Matematica.PI
        numero e = Matematica.E
        numero tau = Matematica.TAU
    "#;
    
    let entorno = ejecutar_codigo(codigo).expect("El codigo deberia ejecutarse correctamente");
    assert!(obtener_valor_variable(&entorno, "pi").is_some());
    assert!(obtener_valor_variable(&entorno, "e").is_some());
    assert!(obtener_valor_variable(&entorno, "tau").is_some());
}

#[test]
fn prueba_operaciones_basicas() {
    let codigo = r#"
        numero suma = Matematica.sumar(10, 5)
        numero resta = Matematica.restar(10, 5)
        numero mult = Matematica.multiplicar(10, 5)
        numero div = Matematica.dividir(10, 5)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_potencia() {
    let codigo = r#"
        numero potencia = Matematica.potencia(2, 10)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_funciones_trigonometricas() {
    let codigo = r#"
        numero radianes = Matematica.grados_a_radianes(90)
        numero seno = Matematica.seno(radianes)
        numero coseno = Matematica.coseno(radianes)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_hipotenusa() {
    let codigo = r#"
        numero hipotenusa = Matematica.hipotenusa(3, 4)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_logaritmos() {
    let codigo = r#"
        numero log_base = Matematica.logaritmo_base(256, 2)
        numero log_natural = Matematica.logaritmo(10)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_exponencial() {
    let codigo = r#"
        numero exp = Matematica.exponencial(1)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_raiz_cuadrada() {
    let codigo = r#"
        numero raiz = Matematica.raiz_cuadrada(25)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_valor_absoluto() {
    let codigo = r#"
        numero abs_positivo = Matematica.absoluto(5)
        numero abs_negativo = Matematica.absoluto(-5)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_redondeo() {
    let codigo = r#"
        numero redondeado = Matematica.redondear(3.141592, 4)
        numero piso = Matematica.piso(3.7)
        numero techo = Matematica.techo(3.2)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_estadisticas_lista() {
    let codigo = r#"
        lista<numero> numeros = [2.5, 5.0, 10.5, 13.0]
        numero promedio = Matematica.promedio(numeros)
        numero maximo = Matematica.maximo(numeros)
        numero minimo = Matematica.minimo(numeros)
        numero suma_total = Matematica.suma_total(numeros)
        numero producto_total = Matematica.producto_total(numeros)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_operaciones_con_constantes() {
    let codigo = r#"
        numero mitad_pi = Matematica.PI / 2
        numero seno_mitad_pi = Matematica.seno(mitad_pi)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_division_por_cero() {
    let codigo = r#"
        numero resultado = Matematica.dividir(10, 0)
    "#;
    
    assert!(verificar_ejecucion_con_error(codigo));
}

#[test]
fn prueba_tangente() {
    let codigo = r#"
        numero radianes = Matematica.grados_a_radianes(45)
        numero tangente = Matematica.tangente(radianes)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_radianes_a_grados() {
    let codigo = r#"
        numero grados = Matematica.radianes_a_grados(Matematica.PI)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_aleatorio() {
    let codigo = r#"
        numero aleatorio = Matematica.aleatorio()
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_aleatorio_rango() {
    let codigo = r#"
        numero aleatorio = Matematica.aleatorio_rango(1, 100)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_logaritmo_valor_negativo() {
    let codigo = r#"
        numero resultado = Matematica.logaritmo(-5)
    "#;
    
    assert!(verificar_ejecucion_con_error(codigo));
}

#[test]
fn prueba_raiz_cuadrada_negativo() {
    let codigo = r#"
        numero resultado = Matematica.raiz_cuadrada(-4)
    "#;
    
    assert!(verificar_ejecucion_con_error(codigo));
}

#[test]
fn prueba_alias_raiz() {
    let codigo = r#"
        numero raiz = Matematica.raiz(16)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_alias_abs() {
    let codigo = r#"
        numero abs = Matematica.abs(-10)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_alias_floor() {
    let codigo = r#"
        numero floor = Matematica.floor(3.9)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_alias_ceil() {
    let codigo = r#"
        numero ceil = Matematica.ceil(3.1)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}

#[test]
fn prueba_alias_logaritmo_natural() {
    let codigo = r#"
        numero resultado_log = Matematica.logaritmo_natural(10)
    "#;
    
    assert!(verificar_ejecucion_exitosa(codigo));
}
