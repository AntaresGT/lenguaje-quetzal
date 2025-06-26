mod utileria;
mod lexico;
mod interpretador;

use interpretador::Interprete;

fn main() {
    let mut argumentos = std::env::args();
    // El primer argumento es el ejecutable
    argumentos.next();

    let ruta = match argumentos.next() {
        Some(r) => r,
        None => {
            eprintln!("Uso: interprete <archivo.qz>");
            return;
        }
    };

    let interprete = Interprete::nuevo();
    match interprete.ejecutar_archivo(&ruta) {
        Ok(tokens) => {
            for token in tokens {
                println!("{:?}", token);
            }
        }
        Err(err) => eprintln!("Error: {}", err),
    }
}
