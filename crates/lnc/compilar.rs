fn main() {
    let directorio_manifiesto =
        std::env::var("CARGO_MANIFEST_DIR").expect("falta CARGO_MANIFEST_DIR");
    let icono_ico = format!(
        "{directorio_manifiesto}/../../recursos/imagenes/ico_lenguaje_quetzal.ico"
    );
    let icono_png = format!(
        "{directorio_manifiesto}/../../recursos/imagenes/ico_lenguaje_quetzal.png"
    );

    println!("cargo:rerun-if-changed={icono_ico}");
    println!("cargo:rerun-if-changed={icono_png}");

    let sistema_objetivo =
        std::env::var("CARGO_CFG_TARGET_OS").expect("falta CARGO_CFG_TARGET_OS");
    if sistema_objetivo == "windows" {
        let mut recurso = winresource::WindowsResource::new();
        recurso.set_icon(&icono_ico);
        recurso
            .compile()
            .expect("no se pudo embeber el icono de Windows en quetzal.exe");
    }
}
