#[cfg(windows)]
extern crate winres;

#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("imagenes/ico_lenguaje_quetzal.ico");
    res.compile().unwrap();
}

#[cfg(unix)]
fn main() {
}