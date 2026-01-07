// Pruebas unitarias para funcionalidad asincrona del lenguaje Quetzal

use crate::interprete::asincrono::*;
use crate::interprete::valores::Valor;
use crate::errores::CodigoError;

// ============================================
// Pruebas de ejecutar_asincrono
// ============================================

#[tokio::test]
async fn prueba_ejecutar_asincrono_simple() {
    let resultado = ejecutar_asincrono(async {
        Ok(Valor::Entero(42))
    }).await;
    
    assert!(resultado.is_ok());
    if let Ok(Valor::Entero(n)) = resultado {
        assert_eq!(n, 42);
    } else {
        panic!("Se esperaba Valor::Entero(42)");
    }
}

#[tokio::test]
async fn prueba_ejecutar_asincrono_con_error() {
    let resultado = ejecutar_asincrono(async {
        Err(crate::errores::Error::ejecucion(
            CodigoError::ErrorInternoInterprete,
            "error de prueba",
            None,
            None,
            None,
        ))
    }).await;
    
    assert!(resultado.is_err());
}

// ============================================
// Pruebas de esperar_ms
// ============================================

#[tokio::test]
async fn prueba_esperar_ms() {
    let inicio = std::time::Instant::now();
    let resultado = esperar_ms(50).await;
    let duracion = inicio.elapsed();
    
    assert!(resultado.is_ok());
    assert!(duracion.as_millis() >= 40, "Deberia esperar al menos 40ms");
}

// ============================================
// Pruebas de EjecutorAsincrono
// ============================================

#[tokio::test]
async fn prueba_ejecutor_crear() {
    let ejecutor = EjecutorAsincrono::nuevo();
    
    // Crear una tarea y esperar su resultado
    let tarea = ejecutor.crear_tarea(async {
        Ok(Valor::Texto("test".to_string()))
    });
    
    let resultado = tarea.esperar_async().await;
    assert!(resultado.is_ok());
}

#[tokio::test]
async fn prueba_crear_tarea() {
    let ejecutor = EjecutorAsincrono::nuevo();
    
    let tarea = ejecutor.crear_tarea(async {
        Ok(Valor::Entero(100))
    });
    
    // Esperar a que complete usando await
    let resultado = tarea.esperar_async().await;
    
    assert!(resultado.is_ok());
    if let Ok(Valor::Entero(n)) = resultado {
        assert_eq!(n, 100);
    }
}

#[tokio::test]
async fn prueba_tarea_esta_completada() {
    let ejecutor = EjecutorAsincrono::nuevo();
    
    let tarea = ejecutor.crear_tarea(async {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        Ok(Valor::Vacio)
    });
    
    // Inmediatamente despues de crear, puede que no este completada
    // Esperar un poco para que complete
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    
    assert!(tarea.esta_completada(), "Tarea deberia haber completado");
}

// ============================================
// Pruebas de con_timeout
// ============================================

#[tokio::test]
async fn prueba_timeout_exitoso() {
    let resultado = con_timeout(
        async { Ok(Valor::Entero(42)) },
        1000, // 1 segundo
    ).await;
    
    assert!(resultado.is_ok());
}

#[tokio::test]
async fn prueba_timeout_excedido() {
    let resultado = con_timeout(
        async {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            Ok(Valor::Vacio)
        },
        50, // 50ms timeout
    ).await;
    
    assert!(resultado.is_err());
    if let Err(e) = resultado {
        assert_eq!(e.codigo(), CodigoError::TiempoEjecucionExcedido.codigo());
    }
}

// ============================================
// Pruebas de promesas
// ============================================

#[tokio::test]
async fn prueba_es_promesa() {
    use std::collections::HashMap;
    
    let promesa = Valor::Objeto {
        tipo: "Promesa".to_string(),
        propiedades: HashMap::new(),
    };
    
    assert!(es_promesa(&promesa), "Deberia ser una promesa");
    
    let no_promesa = Valor::Entero(42);
    assert!(!es_promesa(&no_promesa), "No deberia ser una promesa");
}

#[tokio::test]
async fn prueba_crear_promesa() {
    let ejecutor = EjecutorAsincrono::nuevo();
    
    let tarea = ejecutor.crear_tarea(async {
        Ok(Valor::Texto("resultado".to_string()))
    });
    
    let promesa = crear_promesa(&tarea);
    
    assert!(es_promesa(&promesa), "crear_promesa deberia retornar una promesa");
    
    // Consumir la tarea para limpiar
    let _ = tarea.esperar_async().await;
}

// ============================================
// Pruebas de ContextoAsincrono
// ============================================

#[tokio::test]
async fn prueba_contexto_asincrono() {
    use crate::interprete::entorno::Entorno;
    
    let entorno = Entorno::nuevo();
    let contexto = ContextoAsincrono::nuevo(entorno);
    
    // Verificar que se puede acceder al entorno
    let entorno_guard = contexto.entorno.lock().unwrap();
    assert!(entorno_guard.obtener_variable("inexistente").is_none());
}

// ============================================
// Pruebas de ejecutar_paralelo
// ============================================

#[tokio::test]
async fn prueba_ejecutar_paralelo() {
    let ejecutor = EjecutorAsincrono::nuevo();
    
    let tarea1 = ejecutor.crear_tarea(async {
        Ok(Valor::Entero(1))
    });
    
    let tarea2 = ejecutor.crear_tarea(async {
        Ok(Valor::Entero(2))
    });
    
    let tareas = vec![tarea1, tarea2];
    let resultados = ejecutar_paralelo(tareas).await;
    
    assert_eq!(resultados.len(), 2, "Deberia haber 2 resultados");
    assert!(resultados[0].is_ok());
    assert!(resultados[1].is_ok());
}

// ============================================
// Pruebas de integracion
// ============================================

#[tokio::test]
async fn prueba_integracion_asincrona() {
    // Simular una operacion asincrona compleja
    let resultado = ejecutar_asincrono(async {
        // Primera operacion
        let val1 = esperar_ms(10).await?;
        
        // Segunda operacion
        let _val2 = esperar_ms(10).await?;
        
        // Retornar resultado
        match val1 {
            Valor::Vacio => Ok(Valor::Texto("completado".to_string())),
            _ => Ok(val1),
        }
    }).await;
    
    assert!(resultado.is_ok());
}
