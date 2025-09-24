// Módulo nativo de ejemplo para Quetzal: operaciones matemáticas básicas
// El nombre público del módulo es "quetzal/matemática"

use std::collections::HashMap;
use std::f64::consts::PI;

use crate::analizador_sintactico::{Nodo, Parametro};
use crate::errores::{ErrorQuetzal, ResultadoQuetzal};
use crate::evaluador::{Evaluador, FuncionDefinida};
use crate::manejador_modulos::ElementoExportado;
use crate::tipos_datos::{TipoVariable, Valor, Variable};

/// Registra todas las exportaciones ofrecidas por el módulo nativo
pub fn registrar_modulo(
    evaluador: &mut Evaluador,
) -> ResultadoQuetzal<HashMap<String, ElementoExportado>> {
    let mut exportaciones = HashMap::new();

    // Función nativa: sumar(a, b)
    let parametros_sumar = vec![
        Parametro {
            nombre: "a".to_string(),
            tipo_dato: "número".to_string(),
            es_variable: false,
            valor_defecto: None,
        },
        Parametro {
            nombre: "b".to_string(),
            tipo_dato: "número".to_string(),
            es_variable: false,
            valor_defecto: None,
        },
    ];

    let funcion_sumar = FuncionDefinida {
        parametros: parametros_sumar,
        tipo_retorno: "número".to_string(),
        cuerpo: Nodo::Literal(Valor::Vacio),
        es_asincrona: false,
        implementacion_nativa: Some(funcion_sumar_nativa),
    };

    exportaciones.insert(
        "sumar".to_string(),
        ElementoExportado::Funcion(funcion_sumar),
    );

    // Constante útil: aproximación de PI
    // Ajustamos PI con el mismo redondeo que el resto del intérprete
    let numero_pi = evaluador.redondear_numero(PI);

    let variable_pi = Variable::nueva(
        "pi_aproximado".to_string(),
        Valor::Numero(numero_pi),
        TipoVariable::Inmutable,
        "número".to_string(),
    );

    exportaciones.insert(
        "pi_aproximado".to_string(),
        ElementoExportado::Variable(variable_pi),
    );

    Ok(exportaciones)
}

/// Implementación nativa de la función sumar(a, b)
fn funcion_sumar_nativa(
    evaluador: &mut Evaluador,
    argumentos: &[Valor],
    linea: usize,
) -> ResultadoQuetzal<Valor> {
    if argumentos.len() != 2 {
        return Err(ErrorQuetzal::ArgumentosIncorrectos {
            linea,
            esperados: 2,
            recibidos: argumentos.len(),
        });
    }

    let a = extraer_numero(&argumentos[0], linea, "a")?;
    let b = extraer_numero(&argumentos[1], linea, "b")?;

    // Redondeamos usando la lógica central del intérprete para evitar errores de acarreo
    let resultado = evaluador.redondear_numero(a + b);

    Ok(Valor::Numero(resultado))
}

/// Convierte un valor de Quetzal en un número de punto flotante
fn extraer_numero(valor: &Valor, linea: usize, nombre: &str) -> ResultadoQuetzal<f64> {
    match valor {
        Valor::Numero(numero) => Ok(*numero),
        Valor::Entero(entero) => Ok(*entero as f64),
        _ => Err(ErrorQuetzal::ErrorTipo {
            linea,
            mensaje: format!("El argumento '{}' debe ser un número", nombre),
        }),
    }
}
