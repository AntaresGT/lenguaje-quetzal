// Pruebas unitarias para prototipos e implementación de contratos en objetos

use super::auxiliares::ejecutar_codigo;
use crate::errores::CodigoError;
use crate::nucleo::semantico::Verificador;
use crate::nucleo::sintactico::ast::{DeclaracionMiembroPrototipoAst, ModificadorAcceso, NodoAst};
use crate::nucleo::sintactico::Parser;

fn parsear_ok(codigo: &str) -> Vec<NodoAst> {
    Parser::parsear(codigo).expect("el parser debería aceptar el código")
}

fn verificar_semantica_ok(codigo: &str) -> bool {
    let ast = match Parser::parsear(codigo) {
        Ok(ast) => ast,
        Err(_) => return false,
    };

    let mut verificador = Verificador::nuevo();
    verificador.verificar_programa(&ast).is_ok()
}

fn verificar_error_semantico(codigo: &str, codigo_esperado: CodigoError) -> bool {
    let ast = match Parser::parsear(codigo) {
        Ok(ast) => ast,
        Err(_) => return false,
    };

    let mut verificador = Verificador::nuevo();
    match verificador.verificar_programa(&ast) {
        Ok(_) => false,
        Err(e) => e.codigo() == codigo_esperado.codigo(),
    }
}

#[test]
fn prueba_parser_prototipo_con_secciones() {
    let codigo = r#"
        prototipo UsuarioProto {
            privado:
                entero id
            publico:
                texto nombre
                texto obtener_nombre()
        }
    "#;

    let ast = parsear_ok(codigo);
    match &ast[0] {
        NodoAst::DeclaracionPrototipo { miembros, .. } => {
            assert_eq!(miembros.len(), 3);
        }
        _ => panic!("se esperaba una declaración de prototipo"),
    }
}

#[test]
fn prueba_parser_prototipo_sin_secciones_es_publico() {
    let codigo = r#"
        prototipo ProductoProto {
            entero codigo
            texto descripcion
        }
    "#;

    let ast = parsear_ok(codigo);
    match &ast[0] {
        NodoAst::DeclaracionPrototipo { miembros, .. } => {
            assert_eq!(miembros.len(), 2);
            assert!(matches!(miembros[0].modificador_acceso, ModificadorAcceso::Publico));
            assert!(matches!(miembros[1].modificador_acceso, ModificadorAcceso::Publico));
        }
        _ => panic!("se esperaba una declaración de prototipo"),
    }
}

#[test]
fn prueba_parser_error_privado_sin_publico() {
    let codigo = r#"
        prototipo MalDefinido {
            privado:
                entero secreto
        }
    "#;

    let err = Parser::parsear(codigo).expect_err("el parser debería fallar");
    assert_eq!(err.codigo(), CodigoError::SintaxisGeneral.codigo());
}

#[test]
fn prueba_parser_objeto_implementa_unico() {
    let codigo = r#"
        prototipo A {
            entero x
        }
        objeto MiObjeto implementa A {
            entero x = 1
        }
    "#;

    let ast = parsear_ok(codigo);
    match &ast[1] {
        NodoAst::DeclaracionObjeto { prototipos, .. } => {
            assert_eq!(prototipos, &vec!["A".to_string()]);
        }
        _ => panic!("se esperaba una declaración de objeto"),
    }
}

#[test]
fn prueba_parser_objeto_implementa_multiple() {
    let codigo = r#"
        prototipo A { entero x }
        prototipo B { texto nombre }
        objeto MiObjeto implementa A, B {
            entero x = 1
            texto nombre = "ok"
        }
    "#;

    let ast = parsear_ok(codigo);
    match &ast[2] {
        NodoAst::DeclaracionObjeto { prototipos, .. } => {
            assert_eq!(prototipos, &vec!["A".to_string(), "B".to_string()]);
        }
        _ => panic!("se esperaba una declaración de objeto"),
    }
}

#[test]
fn prueba_parser_error_orden_implementa_antes_como() {
    let codigo = r#"
        prototipo A { entero x }
        objeto MiObjeto implementa A como Padre {
            entero x = 1
        }
    "#;

    let err = Parser::parsear(codigo).expect_err("el parser debería fallar por orden inválido");
    assert_eq!(err.codigo(), CodigoError::SintaxisGeneral.codigo());
}

#[test]
fn prueba_semantica_implementacion_correcta() {
    let codigo = r#"
        prototipo Contrato {
            privado:
                entero id
            publico:
                texto nombre
                entero sumar(entero a, entero b)
        }

        objeto Implementador implementa Contrato {
            privado:
                entero id = 1
            publico:
                texto nombre = "obj"
                entero sumar(entero a, entero b) {
                    retornar a + b
                }
        }
    "#;

    assert!(verificar_semantica_ok(codigo));
}

#[test]
fn prueba_semantica_error_atributo_requerido_faltante() {
    let codigo = r#"
        prototipo Contrato {
            entero id
        }
        objeto Implementador implementa Contrato {
            texto nombre = "x"
        }
    "#;

    assert!(verificar_error_semantico(
        codigo,
        CodigoError::ImplementacionPrototipoIncompleta
    ));
}

#[test]
fn prueba_semantica_error_funcion_requerida_faltante() {
    let codigo = r#"
        prototipo Contrato {
            entero sumar(entero a, entero b)
        }
        objeto Implementador implementa Contrato {
            entero x = 1
        }
    "#;

    assert!(verificar_error_semantico(
        codigo,
        CodigoError::ImplementacionPrototipoIncompleta
    ));
}

#[test]
fn prueba_semantica_error_tipo_atributo_incompatible() {
    let codigo = r#"
        prototipo Contrato {
            entero id
        }
        objeto Implementador implementa Contrato {
            texto id = "uno"
        }
    "#;

    assert!(verificar_error_semantico(
        codigo,
        CodigoError::FirmaPrototipoIncompatible
    ));
}

#[test]
fn prueba_semantica_error_firma_funcion_incompatible() {
    let codigo = r#"
        prototipo Contrato {
            entero sumar(entero a, entero var b)
        }
        objeto Implementador implementa Contrato {
            entero sumar(entero a, entero b) {
                retornar a + b
            }
        }
    "#;

    assert!(verificar_error_semantico(
        codigo,
        CodigoError::FirmaPrototipoIncompatible
    ));
}

#[test]
fn prueba_semantica_error_acceso_incompatible() {
    let codigo = r#"
        prototipo Contrato {
            privado:
                entero secreto
            publico:
                entero obtener()
        }
        objeto Implementador implementa Contrato {
            publico:
                entero secreto = 1
                entero obtener() {
                    retornar ambiente.secreto
                }
        }
    "#;

    assert!(verificar_error_semantico(
        codigo,
        CodigoError::FirmaPrototipoIncompatible
    ));
}

#[test]
fn prueba_semantica_miembro_opcional_omitido() {
    let codigo = r#"
        prototipo Contrato {
            entero id
            texto opcional alias
        }
        objeto Implementador implementa Contrato {
            entero id = 1
        }
    "#;

    assert!(verificar_semantica_ok(codigo));
}

#[test]
fn prueba_semantica_miembro_opcional_incompatible() {
    let codigo = r#"
        prototipo Contrato {
            texto opcional alias
        }
        objeto Implementador implementa Contrato {
            entero alias = 1
        }
    "#;

    assert!(verificar_error_semantico(
        codigo,
        CodigoError::FirmaPrototipoIncompatible
    ));
}

#[test]
fn prueba_semantica_prototipo_no_encontrado() {
    let codigo = r#"
        objeto Implementador implementa NoExiste {
            entero x = 1
        }
    "#;

    assert!(verificar_error_semantico(
        codigo,
        CodigoError::PrototipoNoEncontrado
    ));
}

#[test]
fn prueba_semantica_herencia_no_cuenta_para_implementa() {
    let codigo = r#"
        prototipo Contrato {
            entero id
        }
        objeto Base {
            entero id = 1
        }
        objeto Hijo como Base implementa Contrato {
        }
    "#;

    assert!(verificar_error_semantico(
        codigo,
        CodigoError::ImplementacionPrototipoIncompleta
    ));
}

#[test]
fn prueba_semantica_multiple_prototipos() {
    let codigo = r#"
        prototipo A {
            entero id
        }
        prototipo B {
            texto nombre
            entero sumar(entero a, entero b)
        }
        objeto Implementador implementa A, B {
            entero id = 1
            texto nombre = "ok"
            entero sumar(entero a, entero b) {
                retornar a + b
            }
        }
    "#;

    assert!(verificar_semantica_ok(codigo));
}

#[test]
fn prueba_integracion_archivo_ejemplo_prototipos() {
    let codigo = std::fs::read_to_string("ejemplos/prototipos.qz")
        .expect("debería existir ejemplos/prototipos.qz");

    let ast = Parser::parsear(&codigo).expect("el ejemplo debería parsear correctamente");
    let mut verificador = Verificador::nuevo();
    verificador
        .verificar_programa(&ast)
        .expect("el ejemplo debería pasar verificación semántica");

    ejecutar_codigo(&codigo).expect("el ejemplo debería ejecutarse sin error");
}

#[test]
fn prueba_parser_detecta_opcional_en_funcion() {
    let codigo = r#"
        prototipo Contrato {
            log opcional verificar(entero z)
        }
    "#;

    let ast = parsear_ok(codigo);
    match &ast[0] {
        NodoAst::DeclaracionPrototipo { miembros, .. } => match &miembros[0].declaracion {
            DeclaracionMiembroPrototipoAst::Funcion { nombre, .. } => {
                assert_eq!(nombre, "verificar");
                assert!(miembros[0].opcional);
            }
            _ => panic!("se esperaba función en el prototipo"),
        },
        _ => panic!("se esperaba una declaración de prototipo"),
    }
}
