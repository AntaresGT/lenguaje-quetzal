use paquetes::CacheBytecode;

#[test]
fn cache_deberia_cubrir_fria_caliente_y_corrupta_sin_omitir_verificacion() {
    let temporal =
        std::env::temp_dir().join(format!("quetzal_cache_verificada_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temporal);
    let fuente = nucleo::Fuente::nueva("cache.qz", "entero valor = 40 + 2\n");
    let ast = sintaxis::parsear_modulo(&fuente).expect("debe parsear");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar");
    let cache = CacheBytecode::del_proyecto(&temporal);

    assert!(cache.buscar(&fuente.contenido).is_none());
    cache.guardar(&fuente.contenido, &modulo);
    assert_eq!(cache.buscar(&fuente.contenido), Some(modulo.clone()));

    let ruta = temporal.join(".quetzal").join("cache").join(format!(
        "{}.bytecode",
        CacheBytecode::clave(&fuente.contenido)
    ));
    std::fs::write(&ruta, b"cache corrupta").expect("escribir corrupción");
    assert!(cache.buscar(&fuente.contenido).is_none());

    let mut invalido = modulo;
    invalido.principal.instrucciones[0] = bytecode::Instruccion::CargarLocal(u16::MAX);
    let datos = postcard::to_allocvec(&invalido).expect("serializar módulo inválido");
    std::fs::write(&ruta, datos).expect("escribir módulo inválido");
    assert!(cache.buscar(&fuente.contenido).is_none());

    let _ = std::fs::remove_dir_all(&temporal);
}
