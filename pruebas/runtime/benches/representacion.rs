use std::hint::black_box;
use std::rc::Rc;
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};

#[derive(Clone)]
enum TextoActual {
    Texto(Rc<str>),
    Entero(i64),
    Nulo,
}

#[derive(Clone)]
enum TextoDelgado {
    Texto(Rc<String>),
    Entero(i64),
    Nulo,
}

fn actual() -> TextoActual {
    TextoActual::Texto(Rc::from("Quetzal🦜"))
}

fn delgado() -> TextoDelgado {
    TextoDelgado::Texto(Rc::new("Quetzal🦜".to_string()))
}

fn representacion(criterion: &mut Criterion) {
    #[cfg(target_pointer_width = "64")]
    {
        assert_eq!(std::mem::size_of::<TextoActual>(), 24);
        assert_eq!(std::mem::size_of::<TextoDelgado>(), 16);
        let _ = TextoActual::Entero(0);
        let _ = TextoActual::Nulo;
        let _ = TextoDelgado::Entero(0);
        let _ = TextoDelgado::Nulo;
    }

    criterion.bench_function("representacion/creacion_rc_str", |b| {
        b.iter(|| black_box(actual()))
    });
    criterion.bench_function("representacion/creacion_rc_string", |b| {
        b.iter(|| black_box(delgado()))
    });
    criterion.bench_function("representacion/clonacion_rc_str", |b| {
        let valores = vec![actual(); 10_000];
        b.iter(|| black_box(valores.to_vec()))
    });
    criterion.bench_function("representacion/clonacion_rc_string", |b| {
        let valores = vec![delgado(); 10_000];
        b.iter(|| black_box(valores.to_vec()))
    });
    criterion.bench_function("representacion/lectura_rc_str", |b| {
        let valor = actual();
        b.iter(|| match &valor {
            TextoActual::Texto(texto) => black_box(texto.chars().nth(7)),
            TextoActual::Entero(valor) => char::from_u32(*valor as u32),
            TextoActual::Nulo => None,
        })
    });
    criterion.bench_function("representacion/lectura_rc_string", |b| {
        let valor = delgado();
        b.iter(|| match &valor {
            TextoDelgado::Texto(texto) => black_box(texto.chars().nth(7)),
            TextoDelgado::Entero(valor) => char::from_u32(*valor as u32),
            TextoDelgado::Nulo => None,
        })
    });
}

criterion_group! {
    name = grupo;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));
    targets = representacion
}
criterion_main!(grupo);
