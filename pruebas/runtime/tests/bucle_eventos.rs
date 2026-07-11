//! Pruebas del bucle de eventos real: tareas nativas asincrónicas resueltas
//! con `esperar` sin bloquear otros eventos, funciones nativas con acceso a
//! la VM (para invocar manejadores de Quetzal por referencia) y el drenado
//! del bucle al terminar el programa principal (servidores que siguen
//! vivos).

use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use indexmap::IndexMap;
use maquina_virtual::{CargaNativa, EstadoTareaNativa, Fallo, Mensaje, RegistroNativos, Valor, Vm};
use nucleo::{Fuente, Ubicacion};

fn ubicacion() -> Ubicacion {
    Ubicacion::nueva(0, 0)
}

/// Registra una función nativa `prueba.retrasada(texto)` que lanza un hilo
/// de fondo que espera `retraso` y devuelve `texto` en mayúsculas como
/// resultado de una tarea nativa (simula una operación de E/S real, como una
/// petición HTTP asincrónica).
fn registrar_prueba_retrasada(registro: &mut RegistroNativos, retraso: Duration) {
    registro.registrar_funcion_con_vm(
        "prueba.retrasada",
        Box::new(move |vm, argumentos| {
            let texto = match argumentos.first() {
                Some(Valor::Texto(texto)) => texto.to_string(),
                _ => String::new(),
            };
            let id = vm.bucle().nuevo_id();
            let manija = vm.bucle().manija();
            let retraso = retraso;
            std::thread::spawn(move || {
                std::thread::sleep(retraso);
                manija.enviar(Mensaje::TareaLista {
                    id,
                    resultado: Ok(CargaNativa::Texto(texto.to_uppercase())),
                });
            });
            Ok(Valor::TareaNativa(Rc::new(EstadoTareaNativa { id })))
        }),
    );
}

#[test]
fn tarea_nativa_se_resuelve_con_esperar() {
    let mut registro = RegistroNativos::nuevo();
    registrar_prueba_retrasada(&mut registro, Duration::from_millis(20));
    let mut vm = Vm::nueva(Rc::new(registro));

    let nativos = Rc::clone(&vm.nativos);
    let funcion = nativos
        .buscar_funcion_con_vm("prueba.retrasada")
        .expect("la función de prueba está registrada");
    let tarea = funcion(&mut vm, &[Valor::texto("hola")]).expect("la tarea se lanza sin error");
    assert!(
        matches!(tarea, Valor::TareaNativa(_)),
        "debe devolver una tarea nativa pendiente, no el resultado ya resuelto"
    );

    let resultado = vm
        .esperar(tarea, ubicacion())
        .expect("la tarea nativa se resuelve sin error");
    match resultado {
        Valor::Texto(texto) => assert_eq!(&*texto, "HOLA"),
        otro => panic!("se esperaba texto, llegó {}", otro.nombre_tipo()),
    }
}

#[test]
fn tareas_nativas_fuera_de_orden_se_guardan_para_su_propio_esperar() {
    let mut registro = RegistroNativos::nuevo();
    // La primera tarea tarda más que la segunda: cuando se espere la
    // primera, el resultado de la segunda debe llegar de paso y quedar
    // guardado hasta que se espere específicamente.
    registrar_prueba_retrasada(&mut registro, Duration::from_millis(60));
    let mut vm = Vm::nueva(Rc::new(registro));

    let nativos = Rc::clone(&vm.nativos);
    let funcion = nativos
        .buscar_funcion_con_vm("prueba.retrasada")
        .expect("la función de prueba está registrada");

    let tarea_lenta = funcion(&mut vm, &[Valor::texto("lenta")]).expect("se lanza la tarea lenta");

    // Una segunda tarea nativa, creada e insertada manualmente con un
    // retraso menor, para forzar que su resultado llegue primero.
    let id_rapida = vm.bucle().nuevo_id();
    let manija = vm.bucle().manija();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(5));
        manija.enviar(Mensaje::TareaLista {
            id: id_rapida,
            resultado: Ok(CargaNativa::Texto("RAPIDA".to_string())),
        });
    });
    let tarea_rapida = Valor::TareaNativa(Rc::new(EstadoTareaNativa { id: id_rapida }));

    // Esperamos primero la lenta: mientras se bombea el bucle, el resultado
    // de la rápida llega de paso y debe quedar guardado.
    let resultado_lenta = vm
        .esperar(tarea_lenta, ubicacion())
        .expect("la tarea lenta se resuelve");
    assert_eq!(
        maquina_virtual::texto_de_valor(&resultado_lenta),
        "LENTA"
    );

    let resultado_rapida = vm
        .esperar(tarea_rapida, ubicacion())
        .expect("la tarea rápida, guardada de paso, se resuelve sin volver a bombear");
    assert_eq!(
        maquina_virtual::texto_de_valor(&resultado_rapida),
        "RAPIDA"
    );
}

#[test]
fn esperar_atiende_solicitudes_de_servidores_mientras_espera_una_tarea() {
    // Simula un "servidor" (despachador) que responde a solicitudes
    // devolviendo el texto en mayúsculas. Mientras la VM espera una tarea
    // nativa no relacionada, debe atender la solicitud entrante sin
    // bloquearse.
    let registro = RegistroNativos::nuevo();
    let mut vm = Vm::nueva(Rc::new(registro));

    vm.registrar_despachador("servicio_de_prueba", |_vm, _id_recurso, datos| match datos {
        CargaNativa::Texto(texto) => CargaNativa::Texto(texto.to_uppercase()),
        otro => otro,
    });

    let manija = vm.bucle().manija();
    let id_tarea = vm.bucle().nuevo_id();
    let (respuesta_tx, respuesta_rx) = mpsc::channel();

    std::thread::spawn(move || {
        // La "solicitud" llega primero...
        manija.enviar(Mensaje::Solicitud {
            servicio: "servicio_de_prueba".to_string(),
            id_recurso: 1,
            datos: CargaNativa::Texto("peticion".to_string()),
            respuesta: respuesta_tx,
        });
        // ...y la tarea que sí estamos esperando llega después.
        std::thread::sleep(Duration::from_millis(20));
        manija.enviar(Mensaje::TareaLista {
            id: id_tarea,
            resultado: Ok(CargaNativa::Entero(42)),
        });
    });

    let tarea = Valor::TareaNativa(Rc::new(EstadoTareaNativa { id: id_tarea }));
    let resultado = vm
        .esperar(tarea, ubicacion())
        .expect("la tarea se resuelve tras atender la solicitud del servidor");
    match resultado {
        Valor::Entero(numero) => assert_eq!(numero, 42),
        otro => panic!("se esperaba entero, llegó {}", otro.nombre_tipo()),
    }

    // La solicitud del "servidor" se atendió de paso: la respuesta ya está.
    let respuesta = respuesta_rx
        .recv_timeout(Duration::from_millis(100))
        .expect("el despachador respondió durante la espera de la otra tarea");
    match respuesta {
        CargaNativa::Texto(texto) => assert_eq!(texto, "PETICION"),
        _ => panic!("se esperaba texto en la respuesta"),
    }
}

#[test]
fn tarea_nativa_fallida_lanza_excepcion_con_codigo_e0702() {
    let mut registro = RegistroNativos::nuevo();
    registro.registrar_funcion_con_vm(
        "prueba.fallida",
        Box::new(|vm, _argumentos| {
            let id = vm.bucle().nuevo_id();
            let manija = vm.bucle().manija();
            manija.enviar(Mensaje::TareaLista {
                id,
                resultado: Err("algo salió mal".to_string()),
            });
            Ok(Valor::TareaNativa(Rc::new(EstadoTareaNativa { id })))
        }),
    );
    let mut vm = Vm::nueva(Rc::new(registro));

    let nativos = Rc::clone(&vm.nativos);
    let funcion = nativos
        .buscar_funcion_con_vm("prueba.fallida")
        .expect("la función de prueba está registrada");
    let tarea = funcion(&mut vm, &[]).expect("la tarea se lanza sin error");

    let error = vm
        .esperar(tarea, ubicacion())
        .expect_err("la tarea nativa fallida debe propagar un fallo");
    match error {
        Fallo::Excepcion(datos) => {
            assert_eq!(datos.codigo.as_deref(), Some("E0702"));
            assert!(datos.mensaje.contains("algo salió mal"));
        }
        Fallo::Error(_) => panic!("se esperaba una excepción de Quetzal, no un error interno"),
    }
}

/// Una función nativa con acceso a la VM debe poder invocar un "manejador"
/// de Quetzal recibido por referencia (`Valor::Funcion`), sin copiarlo: es
/// la base para registrar rutas de un servidor (`servidor.obtener(ruta,
/// manejador)`) en las próximas fases.
#[test]
fn nativa_con_vm_puede_invocar_un_manejador_de_quetzal_por_referencia() {
    let mut registro = RegistroNativos::nuevo();
    registro.registrar_funcion_con_vm(
        "prueba.invocar_manejador",
        Box::new(|vm, argumentos| {
            let (Some(Valor::Funcion(funcion, entorno)), Some(argumento)) =
                (argumentos.first().cloned(), argumentos.get(1).cloned())
            else {
                panic!("se esperaba (funcion, argumento)");
            };
            vm.llamar_funcion(&funcion, &entorno, vec![argumento], None, None)
        }),
    );

    let fuente = Fuente::nueva(
        "prueba.qz",
        "entero triplicar(entero valor) {\n    retornar valor * 3\n}\n",
    );
    let ast = sintaxis::parsear_modulo(&fuente).expect("debe parsear");
    semantica::analizar_modulo(&ast).expect("debe pasar la semántica");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar bytecode");

    let mut vm = Vm::nueva(Rc::new(registro));
    let (entorno, _valor) = vm
        .cargar_modulo(Rc::new(modulo), IndexMap::new())
        .expect("debe ejecutar sin errores");

    let manejador = entorno
        .globales
        .borrow()
        .get("triplicar")
        .expect("la función 'triplicar' debe existir")
        .valor
        .clone();
    assert!(
        matches!(manejador, Valor::Funcion(..)),
        "una función declarada en Quetzal es un valor de primera clase"
    );

    let nativos = Rc::clone(&vm.nativos);
    let funcion = nativos
        .buscar_funcion_con_vm("prueba.invocar_manejador")
        .expect("la función de prueba está registrada");
    let resultado = funcion(&mut vm, &[manejador, Valor::Entero(14)])
        .expect("debe invocar el manejador sin error");
    match resultado {
        Valor::Entero(numero) => assert_eq!(numero, 42),
        otro => panic!("se esperaba entero, llegó {}", otro.nombre_tipo()),
    }
}

#[test]
fn drenar_bucle_eventos_termina_cuando_no_hay_trabajo_activo() {
    let registro = RegistroNativos::nuevo();
    let mut vm = Vm::nueva(Rc::new(registro));
    // Sin ningún trabajo activo registrado, drenar debe volver de inmediato.
    vm.drenar_bucle_eventos();
    assert!(!vm.bucle().hay_trabajo_activo());
}

#[test]
fn drenar_bucle_eventos_atiende_solicitudes_hasta_liberar_el_trabajo_activo() {
    let registro = RegistroNativos::nuevo();
    let mut vm = Vm::nueva(Rc::new(registro));

    vm.registrar_despachador("servidor_de_prueba", |vm, _id_recurso, datos| {
        // El propio manejador "detiene" el servidor tras la primera petición.
        vm.bucle().liberar_trabajo_activo();
        datos
    });
    vm.bucle().registrar_trabajo_activo();

    let manija = vm.bucle().manija();
    let (respuesta_tx, respuesta_rx) = mpsc::channel();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(10));
        manija.enviar(Mensaje::Solicitud {
            servicio: "servidor_de_prueba".to_string(),
            id_recurso: 1,
            datos: CargaNativa::Entero(7),
            respuesta: respuesta_tx,
        });
    });

    vm.drenar_bucle_eventos();
    assert!(!vm.bucle().hay_trabajo_activo());
    match respuesta_rx
        .recv_timeout(Duration::from_millis(100))
        .expect("la solicitud se atendió durante el drenado")
    {
        CargaNativa::Entero(numero) => assert_eq!(numero, 7),
        _ => panic!("se esperaba un entero en la respuesta"),
    }
}
