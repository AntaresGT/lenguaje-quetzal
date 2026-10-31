//! Punto de entrada del ejecutable `quetzal-lsp`.
#![forbid(unsafe_code)]

fn main() {
    let mut argumentos = std::env::args().skip(1);
    let transporte_valido = matches!(
        (argumentos.next().as_deref(), argumentos.next()),
        (None | Some("--stdio"), None)
    );

    if !transporte_valido {
        eprintln!("Uso: quetzal-lsp [--stdio]");
        std::process::exit(2);
    }

    if let Err(error) = lsp::servidor::ejecutar_stdio() {
        eprintln!("Error del servidor LSP: {error}");
        std::process::exit(1);
    }
}
