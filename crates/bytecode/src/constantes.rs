//! Constantes del bytecode.

use rust_decimal::Decimal;

/// Valor constante embebido en un módulo compilado.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Constante {
    Entero(i64),
    /// Decimal exacto (sin pasar por f64) para que `0.1 + 0.2 == 0.3`.
    Numero(Decimal),
    Texto(String),
    Log(bool),
    Nulo,
}
