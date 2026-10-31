use std::hint::black_box;
use std::rc::Rc;
use std::time::Duration;

use bytecode::{FuncionCompilada, ModuloCompilado};
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use indexmap::IndexMap;
use maquina_virtual::valores::EntornoModulo;
use maquina_virtual::{Valor, Vm};
use nucleo::Fuente;

const BUCLE: &str = r#"
entero sumar_hasta(entero limite) {
    entero var suma = 0
    para (entero var i = 0; i < limite; i++) {
        suma += i
    }
    retornar suma
}
"#;

const FIBONACCI: &str = r#"
entero fib(entero n) {
    si (n < 2) { retornar n }
    retornar fib(n - 1) + fib(n - 2)
}
"#;

#[cfg(feature = "jit")]
const IDENTIDAD: &str = r#"
entero identidad(entero n) {
    retornar n
}
"#;

const ATRIBUTOS: &str = r#"
objeto Punto {
    publico:
        entero var x = 3
}
entero leer_repetido(Punto punto, entero limite) {
    entero var suma = 0
    para (entero var i = 0; i < limite; i++) {
        suma += punto.x
    }
    retornar suma
}
Punto punto = nuevo Punto()
"#;

const ITERACION: &str = r#"
entero recorrer(lista<entero> datos) {
    entero var suma = 0
    para (entero n en datos) {
        suma += n
    }
    retornar suma
}
lista<entero> datos = rango(1, 10000)
"#;

const DINAMICO: &str = r#"
numero sumar_dinamico(jsn valores, entero limite) {
    numero var suma = 0
    para (entero var i = 0; i < limite; i++) {
        suma = valores.a + valores.b
    }
    retornar suma
}
"#;

const LLAMADAS: &str = r#"
entero sumar_uno(entero n) {
    retornar n + 1
}
entero llamar_repetido(entero limite) {
    entero var suma = 0
    para (entero var i = 0; i < limite; i++) {
        suma = sumar_uno(suma)
    }
    retornar suma
}
"#;

const JSN: &str = r#"
entero sumar_jsn(jsn valores, entero limite) {
    entero var suma = 0
    para (entero var i = 0; i < limite; i++) {
        suma += valores.a + valores.b
    }
    retornar suma
}
"#;

const DECIMALES: &str = r#"
numero sumar_decimales(entero limite) {
    numero var suma = 0
    para (entero var i = 0; i < limite; i++) {
        suma += 0.1 + 0.2
    }
    retornar suma
}
"#;

const UNICODE: &str = r#"
entero recorrer_unicode(entero limite) {
    texto contenido = "Quetzal🦜áéíóúQuetzal🦜áéíóúQuetzal🦜áéíóúQuetzal🦜áéíóúQuetzal🦜áéíóúQuetzal🦜áéíóúQuetzal🦜áéíóúQuetzal🦜áéíóúQuetzal🦜áéíóúQuetzal🦜áéíóúQuetzal🦜áéíóúQuetzal🦜áéíóúQuetzal🦜áéíóúQuetzal🦜áéíóúQuetzal🦜áéíóúQuetzal🦜áéíóúQuetzal🦜áéíóúQuetzal🦜áéíóúQuetzal🦜áéíóúQuetzal🦜áéíóú"
    entero var suma = 0
    para (entero var i = 0; i < limite; i++) {
        suma += contenido[i % contenido.longitud()].longitud()
    }
    retornar suma
}
"#;

const OBJETOS: &str = r#"
objeto PuntoCreado {
    publico:
        entero var x = 3
}
entero crear_repetido(entero limite) {
    entero var suma = 0
    para (entero var i = 0; i < limite; i++) {
        PuntoCreado punto = nuevo PuntoCreado()
        suma += punto.x
    }
    retornar suma
}
"#;

const ESCRITURA_ATRIBUTOS: &str = r#"
objeto PuntoMutable {
    publico:
        entero var x = 3
}
entero escribir_repetido(PuntoMutable punto, entero limite) {
    para (entero var i = 0; i < limite; i++) {
        punto.x = i
    }
    retornar punto.x
}
PuntoMutable punto = nuevo PuntoMutable()
"#;

const MODIFICACION_JSN: &str = r#"
entero modificar_jsn(jsn var valores, entero limite) {
    para (entero var i = 0; i < limite; i++) {
        valores.a = i
    }
    retornar valores.a
}
"#;

const METODOS_OBJETO: &str = r#"
objeto PuntoConMetodo {
    publico:
        entero var x = 3
        entero leer() { retornar esto.x }
}
entero llamar_metodos(PuntoConMetodo punto, entero limite) {
    entero var suma = 0
    para (entero var i = 0; i < limite; i++) {
        suma += punto.leer()
    }
    retornar suma
}
PuntoConMetodo punto = nuevo PuntoConMetodo()
"#;

const METODOS_NATIVOS: &str = r#"
entero llamar_nativos(entero limite) {
    texto contenido = "Hola Mundo"
    entero var suma = 0
    para (entero var i = 0; i < limite; i++) {
        suma += contenido.longitud()
    }
    retornar suma
}
"#;

struct Preparada {
    vm: Vm,
    funcion: Rc<FuncionCompilada>,
    entorno: Rc<EntornoModulo>,
}

fn compilar(codigo: &str) -> ModuloCompilado {
    let fuente = Fuente::nueva("criterion.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("el benchmark debe parsear");
    semantica::analizar_modulo(&ast).expect("el benchmark debe pasar semántica");
    bytecode::generar_modulo(&ast).expect("el benchmark debe generar bytecode")
}

fn preparar(codigo: &str, nombre: &str) -> Preparada {
    preparar_con_modo(codigo, nombre, true)
}

fn preparar_con_modo(codigo: &str, nombre: &str, optimizar: bool) -> Preparada {
    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro()));
    if !optimizar {
        vm = vm.sin_optimizar();
    }
    let (entorno, _) = vm
        .cargar_modulo(Rc::new(compilar(codigo)), IndexMap::new())
        .expect("el módulo del benchmark debe cargar");
    let funcion = match &entorno
        .globales
        .borrow()
        .get(nombre)
        .expect("función global")
        .valor
    {
        Valor::Funcion(datos) => Rc::clone(&datos.funcion),
        otro => panic!("se esperaba función, se obtuvo {otro:?}"),
    };
    Preparada {
        vm,
        funcion,
        entorno,
    }
}

#[cfg(feature = "jit")]
fn preparar_jit(codigo: &str, nombre: &str) -> Preparada {
    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro())).con_jit(true);
    let (entorno, _) = vm
        .cargar_modulo(Rc::new(compilar(codigo)), IndexMap::new())
        .expect("el módulo del benchmark debe cargar");
    let funcion = match &entorno
        .globales
        .borrow()
        .get(nombre)
        .expect("función global")
        .valor
    {
        Valor::Funcion(datos) => Rc::clone(&datos.funcion),
        otro => panic!("se esperaba función, se obtuvo {otro:?}"),
    };
    Preparada {
        vm,
        funcion,
        entorno,
    }
}

fn llamar(preparada: &mut Preparada, argumentos: Vec<Valor>) -> Valor {
    preparada
        .vm
        .llamar_funcion(
            &preparada.funcion,
            &preparada.entorno,
            argumentos,
            None,
            None,
        )
        .expect("la llamada del benchmark debe funcionar")
}

fn global(preparada: &Preparada, nombre: &str) -> Valor {
    preparada
        .entorno
        .globales
        .borrow()
        .get(nombre)
        .unwrap_or_else(|| panic!("debe existir la global '{nombre}'"))
        .valor
        .clone()
}

fn fases(criterion: &mut Criterion) {
    let registro = Rc::new(modulos_nativos::crear_registro());
    criterion.bench_function("fases/registro_nativos", |b| {
        b.iter(|| black_box(modulos_nativos::crear_registro()))
    });
    criterion.bench_function("fases/inicializacion_vm", |b| {
        b.iter(|| black_box(Vm::nueva(Rc::clone(&registro))))
    });
    criterion.bench_function("fases/lexico", |b| {
        let fuente = Fuente::nueva("criterion.qz", BUCLE);
        b.iter(|| black_box(lexico::tokenizar(black_box(&fuente)).expect("léxico válido")))
    });
    criterion.bench_function("fases/parseo_con_lexico", |b| {
        let fuente = Fuente::nueva("criterion.qz", BUCLE);
        b.iter(|| black_box(sintaxis::parsear_modulo(black_box(&fuente)).expect("sintaxis válida")))
    });
    criterion.bench_function("fases/semantica", |b| {
        let fuente = Fuente::nueva("criterion.qz", BUCLE);
        let ast = sintaxis::parsear_modulo(&fuente).expect("sintaxis válida");
        b.iter(|| semantica::analizar_modulo(black_box(&ast)).expect("semántica válida"))
    });
    criterion.bench_function("fases/compilacion", |b| {
        b.iter(|| black_box(compilar(black_box(BUCLE))))
    });
    criterion.bench_function("fases/optimizacion_bytecode", |b| {
        b.iter_batched(
            || compilar(BUCLE),
            |mut modulo| {
                bytecode::optimizar_modulo(&mut modulo);
                black_box(modulo)
            },
            BatchSize::SmallInput,
        )
    });
    criterion.bench_function("fases/verificacion_bytecode", |b| {
        b.iter_batched(
            || compilar(BUCLE),
            |mut modulo| {
                bytecode::verificar_modulo(&mut modulo).expect("bytecode válido");
                black_box(modulo)
            },
            BatchSize::SmallInput,
        )
    });
    criterion.bench_function("fases/carga_y_principal", |b| {
        b.iter_batched(
            || (Vm::nueva(Rc::clone(&registro)), Rc::new(compilar(BUCLE))),
            |(mut vm, modulo)| {
                black_box(
                    vm.cargar_modulo(modulo, IndexMap::new())
                        .expect("carga y ejecución principal"),
                )
            },
            BatchSize::SmallInput,
        )
    });
    #[cfg(feature = "jit")]
    criterion.bench_function("fases/compilacion_jit", |b| {
        b.iter_batched(
            || {
                let modulo = Rc::new(compilar(BUCLE));
                let vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro())).con_jit(true);
                (vm, modulo)
            },
            |(mut vm, modulo)| {
                black_box(
                    vm.cargar_modulo(modulo, IndexMap::new())
                        .expect("carga y compilación JIT"),
                )
            },
            BatchSize::SmallInput,
        )
    });
}

fn ejecucion(criterion: &mut Criterion) {
    for (modo, optimizar) in [("puro", false), ("optimizado", true)] {
        let nombre = format!("ejecucion/bucles_enteros_{modo}");
        criterion.bench_function(&nombre, |b| {
            b.iter_batched(
                || preparar_con_modo(BUCLE, "sumar_hasta", optimizar),
                |mut preparada| black_box(llamar(&mut preparada, vec![Valor::Entero(2_000_000)])),
                BatchSize::SmallInput,
            )
        });
        let nombre_repetida = format!("ejecucion/bucles_enteros_repetida_{modo}");
        criterion.bench_function(&nombre_repetida, |b| {
            let mut preparada = preparar_con_modo(BUCLE, "sumar_hasta", optimizar);
            b.iter(|| black_box(llamar(&mut preparada, vec![Valor::Entero(2_000_000)])))
        });
    }
    #[cfg(feature = "jit")]
    criterion.bench_function("ejecucion/bucles_enteros_jit_primera", |b| {
        b.iter_batched(
            || preparar_jit(BUCLE, "sumar_hasta"),
            |mut preparada| black_box(llamar(&mut preparada, vec![Valor::Entero(2_000_000)])),
            BatchSize::SmallInput,
        )
    });
    #[cfg(feature = "jit")]
    criterion.bench_function("ejecucion/bucles_enteros_jit", |b| {
        b.iter_batched(
            || {
                let mut preparada = preparar_jit(BUCLE, "sumar_hasta");
                for _ in 0..4 {
                    black_box(llamar(&mut preparada, vec![Valor::Entero(1)]));
                }
                preparada
            },
            |mut preparada| black_box(llamar(&mut preparada, vec![Valor::Entero(2_000_000)])),
            BatchSize::SmallInput,
        )
    });
    #[cfg(feature = "jit")]
    criterion.bench_function("ejecucion/llamada_nativa_pequena", |b| {
        b.iter_batched(
            || {
                let mut preparada = preparar_jit(IDENTIDAD, "identidad");
                for _ in 0..4 {
                    black_box(llamar(&mut preparada, vec![Valor::Entero(1)]));
                }
                preparada
            },
            |mut preparada| {
                for _ in 0..200_000 {
                    black_box(llamar(&mut preparada, vec![Valor::Entero(7)]));
                }
            },
            BatchSize::SmallInput,
        )
    });
    criterion.bench_function("ejecucion/llamadas_recursivas", |b| {
        b.iter_batched(
            || preparar(FIBONACCI, "fib"),
            |mut preparada| black_box(llamar(&mut preparada, vec![Valor::Entero(25)])),
            BatchSize::SmallInput,
        )
    });
    #[cfg(feature = "jit")]
    criterion.bench_function("ejecucion/llamadas_pequenas_jit", |b| {
        b.iter_batched(
            || preparar_jit(LLAMADAS, "llamar_repetido"),
            |mut preparada| black_box(llamar(&mut preparada, vec![Valor::Entero(200_000)])),
            BatchSize::SmallInput,
        )
    });
    criterion.bench_function("ejecucion/llamadas_pequenas", |b| {
        b.iter_batched(
            || preparar(LLAMADAS, "llamar_repetido"),
            |mut preparada| black_box(llamar(&mut preparada, vec![Valor::Entero(200_000)])),
            BatchSize::SmallInput,
        )
    });
    criterion.bench_function("ejecucion/atributos", |b| {
        b.iter_batched(
            || {
                let preparada = preparar(ATRIBUTOS, "leer_repetido");
                let punto = preparada
                    .entorno
                    .globales
                    .borrow()
                    .get("punto")
                    .expect("punto global")
                    .valor
                    .clone();
                (preparada, punto)
            },
            |(mut preparada, punto)| {
                black_box(llamar(&mut preparada, vec![punto, Valor::Entero(200_000)]))
            },
            BatchSize::SmallInput,
        )
    });
    criterion.bench_function("ejecucion/atributos_escritura", |b| {
        b.iter_batched(
            || {
                let preparada = preparar(ESCRITURA_ATRIBUTOS, "escribir_repetido");
                let punto = global(&preparada, "punto");
                (preparada, punto)
            },
            |(mut preparada, punto)| {
                black_box(llamar(&mut preparada, vec![punto, Valor::Entero(200_000)]))
            },
            BatchSize::SmallInput,
        )
    });
    criterion.bench_function("ejecucion/metodos_objeto", |b| {
        b.iter_batched(
            || {
                let preparada = preparar(METODOS_OBJETO, "llamar_metodos");
                let punto = global(&preparada, "punto");
                (preparada, punto)
            },
            |(mut preparada, punto)| {
                black_box(llamar(&mut preparada, vec![punto, Valor::Entero(200_000)]))
            },
            BatchSize::SmallInput,
        )
    });
    criterion.bench_function("ejecucion/iteracion_lista", |b| {
        b.iter_batched(
            || {
                let preparada = preparar(ITERACION, "recorrer");
                let datos = preparada
                    .entorno
                    .globales
                    .borrow()
                    .get("datos")
                    .expect("datos globales")
                    .valor
                    .clone();
                (preparada, datos)
            },
            |(mut preparada, datos)| black_box(llamar(&mut preparada, vec![datos])),
            BatchSize::SmallInput,
        )
    });
    criterion.bench_function("ejecucion/jsn", |b| {
        b.iter_batched(
            || {
                let preparada = preparar(JSN, "sumar_jsn");
                let valores = Valor::jsn(IndexMap::from([
                    ("a".to_string(), Valor::Entero(20)),
                    ("b".to_string(), Valor::Entero(22)),
                ]));
                (preparada, valores)
            },
            |(mut preparada, valores)| {
                black_box(llamar(
                    &mut preparada,
                    vec![valores, Valor::Entero(100_000)],
                ))
            },
            BatchSize::SmallInput,
        )
    });
    criterion.bench_function("ejecucion/jsn_modificacion", |b| {
        b.iter_batched(
            || {
                let preparada = preparar(MODIFICACION_JSN, "modificar_jsn");
                let valores = Valor::jsn(IndexMap::from([
                    ("a".to_string(), Valor::Entero(20)),
                    ("b".to_string(), Valor::Entero(22)),
                ]));
                (preparada, valores)
            },
            |(mut preparada, valores)| {
                black_box(llamar(
                    &mut preparada,
                    vec![valores, Valor::Entero(100_000)],
                ))
            },
            BatchSize::SmallInput,
        )
    });
    criterion.bench_function("ejecucion/decimales", |b| {
        b.iter_batched(
            || preparar(DECIMALES, "sumar_decimales"),
            |mut preparada| black_box(llamar(&mut preparada, vec![Valor::Entero(100_000)])),
            BatchSize::SmallInput,
        )
    });
    criterion.bench_function("ejecucion/unicode", |b| {
        b.iter_batched(
            || preparar(UNICODE, "recorrer_unicode"),
            |mut preparada| black_box(llamar(&mut preparada, vec![Valor::Entero(100_000)])),
            BatchSize::SmallInput,
        )
    });
    criterion.bench_function("ejecucion/metodos_nativos", |b| {
        b.iter_batched(
            || preparar(METODOS_NATIVOS, "llamar_nativos"),
            |mut preparada| black_box(llamar(&mut preparada, vec![Valor::Entero(200_000)])),
            BatchSize::SmallInput,
        )
    });
    criterion.bench_function("ejecucion/objetos", |b| {
        b.iter_batched(
            || preparar(OBJETOS, "crear_repetido"),
            |mut preparada| black_box(llamar(&mut preparada, vec![Valor::Entero(100_000)])),
            BatchSize::SmallInput,
        )
    });
}

fn especializacion(criterion: &mut Criterion) {
    fn valores() -> Valor {
        Valor::jsn(IndexMap::from([
            ("a".to_string(), Valor::Entero(20)),
            ("b".to_string(), Valor::Entero(22)),
        ]))
    }

    criterion.bench_function("especializacion/fria", |b| {
        b.iter_batched(
            || preparar(DINAMICO, "sumar_dinamico"),
            |mut preparada| black_box(llamar(&mut preparada, vec![valores(), Valor::Entero(100)])),
            BatchSize::SmallInput,
        )
    });
    criterion.bench_function("especializacion/caliente", |b| {
        b.iter_batched(
            || {
                let mut preparada = preparar(DINAMICO, "sumar_dinamico");
                for _ in 0..1_001 {
                    black_box(llamar(&mut preparada, vec![valores(), Valor::Entero(1)]));
                }
                preparada
            },
            |mut preparada| black_box(llamar(&mut preparada, vec![valores(), Valor::Entero(100)])),
            BatchSize::SmallInput,
        )
    });
}

criterion_group! {
    name = grupo_fases;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));
    targets = fases
}
criterion_group! {
    name = grupo_ejecucion;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));
    targets = ejecucion
}
criterion_group! {
    name = grupo_especializacion;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));
    targets = especializacion
}
criterion_main!(grupo_fases, grupo_ejecucion, grupo_especializacion);
