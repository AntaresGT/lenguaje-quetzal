use crate::interprete::valores::Valor;
use rust_decimal::Decimal;

/// Convierte un valor a texto
pub fn convertir_a_texto(valor: &Valor) -> String {
    valor.a_texto()
}

/// Convierte un valor entero a número
pub fn convertir_entero_a_numero(valor: i64) -> Decimal {
    Decimal::from(valor)
}

/// Convierte un texto a entero
pub fn convertir_texto_a_entero(texto: &str) -> Option<i64> {
    texto.parse().ok()
}

/// Convierte un texto a número
pub fn convertir_texto_a_numero(texto: &str) -> Option<Decimal> {
    texto.parse().ok()
}
