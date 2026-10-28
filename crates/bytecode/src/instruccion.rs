//! Conjunto de instrucciones de la máquina virtual de Quetzal.

use nucleo::Ubicacion;

/// Instrucción de la VM. Los índices `u16` de locales/globales/nombres y los
/// `u32` de constantes refieren a las tablas del [`crate::ModuloCompilado`].
///
/// Cabecera de 8 bytes: opcode + operandos compactos. Los saltos usan `u32`
/// (índice absoluto de instrucción), no `usize`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(C)]
pub struct Instruccion {
    pub op: CodigoOp,
    extra: u8,
    pub a: u16,
    pub b: u32,
}

/// Opcode de una instrucción. `#[repr(u8)]` para que [`Instruccion`] mida 8 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum CodigoOp {
    CargarConstante = 0,
    CargarEntero,
    Duplicar,
    Desechar,
    CargarLocal,
    GuardarLocal,
    LimpiarLocales,
    AjustarLocal,
    CargarGlobal,
    DeclararGlobal,
    GuardarGlobal,
    CargarExterno,
    GuardarExterno,
    Sumar,
    Restar,
    Multiplicar,
    Dividir,
    Modulo,
    Negar,
    NoLogico,
    Igual,
    Diferente,
    Mayor,
    Menor,
    MayorOIgual,
    MenorOIgual,
    SumarEntero,
    RestarEntero,
    MultiplicarEntero,
    DividirEntero,
    ModuloEntero,
    NegarEntero,
    IgualEntero,
    DiferenteEntero,
    MayorEntero,
    MenorEntero,
    MayorOIgualEntero,
    MenorOIgualEntero,
    Saltar,
    SaltarSiFalso,
    SaltarSiVerdadero,
    SaltarSiFalsoYDejar,
    SaltarSiVerdaderoYDejar,
    Llamar,
    LlamarMetodo,
    Retornar,
    CargarMiembro,
    GuardarMiembro,
    Indexar,
    GuardarIndice,
    CrearLista,
    CrearJsn,
    Interpolar,
    Instanciar,
    CargarEsto,
    CargarPadre,
    CrearIterador,
    IteradorSiguiente,
    EntrarIntentar,
    SalirIntentar,
    Lanzar,
    Esperar,
    CargarAtributo,
    GuardarAtributo,
    CargarEstoAtributo,
    MenorEnteroSaltarSiFalso,
    CargarLocalLocalSumarEntero,
    RetornarLocal,
    LlamarNativa,
    AjustarLocalConLocal,
    CargarLocalMiembro,
    CargarLocalAtributo,
}

#[allow(non_snake_case)]
impl Instruccion {
    const fn nueva(op: CodigoOp, extra: u8, a: u16, b: u32) -> Self {
        Self { op, extra, a, b }
    }

    pub const fn CargarConstante(indice: u32) -> Self {
        Self::nueva(CodigoOp::CargarConstante, 0, 0, indice)
    }
    pub const fn CargarEntero(valor: i16) -> Self {
        Self::nueva(CodigoOp::CargarEntero, 0, valor as u16, 0)
    }
    pub const fn Duplicar() -> Self {
        Self::nueva(CodigoOp::Duplicar, 0, 0, 0)
    }
    pub const fn Desechar() -> Self {
        Self::nueva(CodigoOp::Desechar, 0, 0, 0)
    }
    pub const fn CargarLocal(ranura: u16) -> Self {
        Self::nueva(CodigoOp::CargarLocal, 0, ranura, 0)
    }
    pub const fn GuardarLocal(ranura: u16) -> Self {
        Self::nueva(CodigoOp::GuardarLocal, 0, ranura, 0)
    }
    pub const fn LimpiarLocales(desde: u16) -> Self {
        Self::nueva(CodigoOp::LimpiarLocales, 0, desde, 0)
    }
    pub const fn AjustarLocal(ranura: u16, delta: i16) -> Self {
        Self::nueva(CodigoOp::AjustarLocal, 0, ranura, delta as u16 as u32)
    }
    pub const fn CargarGlobal(indice: u16) -> Self {
        Self::nueva(CodigoOp::CargarGlobal, 0, indice, 0)
    }
    pub const fn DeclararGlobal(indice: u16) -> Self {
        Self::nueva(CodigoOp::DeclararGlobal, 0, indice, 0)
    }
    pub const fn GuardarGlobal(indice: u16) -> Self {
        Self::nueva(CodigoOp::GuardarGlobal, 0, indice, 0)
    }
    pub const fn CargarExterno(indice: u32) -> Self {
        Self::nueva(CodigoOp::CargarExterno, 0, 0, indice)
    }
    pub const fn GuardarExterno(indice: u32) -> Self {
        Self::nueva(CodigoOp::GuardarExterno, 0, 0, indice)
    }
    pub const fn Sumar() -> Self {
        Self::nueva(CodigoOp::Sumar, 0, 0, 0)
    }
    pub const fn Restar() -> Self {
        Self::nueva(CodigoOp::Restar, 0, 0, 0)
    }
    pub const fn Multiplicar() -> Self {
        Self::nueva(CodigoOp::Multiplicar, 0, 0, 0)
    }
    pub const fn Dividir() -> Self {
        Self::nueva(CodigoOp::Dividir, 0, 0, 0)
    }
    pub const fn Modulo() -> Self {
        Self::nueva(CodigoOp::Modulo, 0, 0, 0)
    }
    pub const fn Negar() -> Self {
        Self::nueva(CodigoOp::Negar, 0, 0, 0)
    }
    pub const fn NoLogico() -> Self {
        Self::nueva(CodigoOp::NoLogico, 0, 0, 0)
    }
    pub const fn Igual() -> Self {
        Self::nueva(CodigoOp::Igual, 0, 0, 0)
    }
    pub const fn Diferente() -> Self {
        Self::nueva(CodigoOp::Diferente, 0, 0, 0)
    }
    pub const fn Mayor() -> Self {
        Self::nueva(CodigoOp::Mayor, 0, 0, 0)
    }
    pub const fn Menor() -> Self {
        Self::nueva(CodigoOp::Menor, 0, 0, 0)
    }
    pub const fn MayorOIgual() -> Self {
        Self::nueva(CodigoOp::MayorOIgual, 0, 0, 0)
    }
    pub const fn MenorOIgual() -> Self {
        Self::nueva(CodigoOp::MenorOIgual, 0, 0, 0)
    }
    pub const fn SumarEntero() -> Self {
        Self::nueva(CodigoOp::SumarEntero, 0, 0, 0)
    }
    pub const fn RestarEntero() -> Self {
        Self::nueva(CodigoOp::RestarEntero, 0, 0, 0)
    }
    pub const fn MultiplicarEntero() -> Self {
        Self::nueva(CodigoOp::MultiplicarEntero, 0, 0, 0)
    }
    pub const fn DividirEntero() -> Self {
        Self::nueva(CodigoOp::DividirEntero, 0, 0, 0)
    }
    pub const fn ModuloEntero() -> Self {
        Self::nueva(CodigoOp::ModuloEntero, 0, 0, 0)
    }
    pub const fn NegarEntero() -> Self {
        Self::nueva(CodigoOp::NegarEntero, 0, 0, 0)
    }
    pub const fn IgualEntero() -> Self {
        Self::nueva(CodigoOp::IgualEntero, 0, 0, 0)
    }
    pub const fn DiferenteEntero() -> Self {
        Self::nueva(CodigoOp::DiferenteEntero, 0, 0, 0)
    }
    pub const fn MayorEntero() -> Self {
        Self::nueva(CodigoOp::MayorEntero, 0, 0, 0)
    }
    pub const fn MenorEntero() -> Self {
        Self::nueva(CodigoOp::MenorEntero, 0, 0, 0)
    }
    pub const fn MayorOIgualEntero() -> Self {
        Self::nueva(CodigoOp::MayorOIgualEntero, 0, 0, 0)
    }
    pub const fn MenorOIgualEntero() -> Self {
        Self::nueva(CodigoOp::MenorOIgualEntero, 0, 0, 0)
    }
    pub const fn Saltar(destino: u32) -> Self {
        Self::nueva(CodigoOp::Saltar, 0, 0, destino)
    }
    pub const fn SaltarSiFalso(destino: u32) -> Self {
        Self::nueva(CodigoOp::SaltarSiFalso, 0, 0, destino)
    }
    pub const fn SaltarSiVerdadero(destino: u32) -> Self {
        Self::nueva(CodigoOp::SaltarSiVerdadero, 0, 0, destino)
    }
    pub const fn SaltarSiFalsoYDejar(destino: u32) -> Self {
        Self::nueva(CodigoOp::SaltarSiFalsoYDejar, 0, 0, destino)
    }
    pub const fn SaltarSiVerdaderoYDejar(destino: u32) -> Self {
        Self::nueva(CodigoOp::SaltarSiVerdaderoYDejar, 0, 0, destino)
    }
    pub const fn Llamar(cantidad: u32) -> Self {
        Self::nueva(CodigoOp::Llamar, 0, 0, cantidad)
    }
    pub const fn LlamarMetodo(indice_nombre: u32, cantidad: u32) -> Self {
        Self::nueva(CodigoOp::LlamarMetodo, 0, cantidad as u16, indice_nombre)
    }
    pub const fn Retornar() -> Self {
        Self::nueva(CodigoOp::Retornar, 0, 0, 0)
    }
    pub const fn CargarMiembro(indice: u32) -> Self {
        Self::nueva(CodigoOp::CargarMiembro, 0, 0, indice)
    }
    pub const fn GuardarMiembro(indice: u32) -> Self {
        Self::nueva(CodigoOp::GuardarMiembro, 0, 0, indice)
    }
    pub const fn Indexar() -> Self {
        Self::nueva(CodigoOp::Indexar, 0, 0, 0)
    }
    pub const fn GuardarIndice() -> Self {
        Self::nueva(CodigoOp::GuardarIndice, 0, 0, 0)
    }
    pub const fn CrearLista(cantidad: u32) -> Self {
        Self::nueva(CodigoOp::CrearLista, 0, 0, cantidad)
    }
    pub const fn CrearJsn(cantidad: u32) -> Self {
        Self::nueva(CodigoOp::CrearJsn, 0, 0, cantidad)
    }
    pub const fn Interpolar(cantidad: u32) -> Self {
        Self::nueva(CodigoOp::Interpolar, 0, 0, cantidad)
    }
    pub const fn Instanciar(cantidad: u32) -> Self {
        Self::nueva(CodigoOp::Instanciar, 0, 0, cantidad)
    }
    pub const fn CargarEsto() -> Self {
        Self::nueva(CodigoOp::CargarEsto, 0, 0, 0)
    }
    pub const fn CargarPadre() -> Self {
        Self::nueva(CodigoOp::CargarPadre, 0, 0, 0)
    }
    pub const fn CrearIterador() -> Self {
        Self::nueva(CodigoOp::CrearIterador, 0, 0, 0)
    }
    pub const fn IteradorSiguiente(destino: u32) -> Self {
        Self::nueva(CodigoOp::IteradorSiguiente, 0, 0, destino)
    }
    pub const fn EntrarIntentar(destino: u32, ranuras: u16) -> Self {
        Self::nueva(CodigoOp::EntrarIntentar, 0, ranuras, destino)
    }
    pub const fn SalirIntentar() -> Self {
        Self::nueva(CodigoOp::SalirIntentar, 0, 0, 0)
    }
    pub const fn Lanzar() -> Self {
        Self::nueva(CodigoOp::Lanzar, 0, 0, 0)
    }
    pub const fn Esperar() -> Self {
        Self::nueva(CodigoOp::Esperar, 0, 0, 0)
    }
    pub const fn CargarAtributo(slot: u16) -> Self {
        Self::nueva(CodigoOp::CargarAtributo, 0, slot, 0)
    }
    pub const fn GuardarAtributo(slot: u16) -> Self {
        Self::nueva(CodigoOp::GuardarAtributo, 0, slot, 0)
    }
    pub const fn CargarEstoAtributo(slot: u16) -> Self {
        Self::nueva(CodigoOp::CargarEstoAtributo, 0, slot, 0)
    }
    pub const fn MenorEnteroSaltarSiFalso(destino: u32) -> Self {
        Self::nueva(CodigoOp::MenorEnteroSaltarSiFalso, 0, 0, destino)
    }
    pub const fn CargarLocalLocalSumarEntero(izquierda: u16, derecha: u16) -> Self {
        Self::nueva(
            CodigoOp::CargarLocalLocalSumarEntero,
            0,
            izquierda,
            derecha as u32,
        )
    }
    pub const fn AjustarLocalConLocal(ranura: u16, otra: u16) -> Self {
        Self::nueva(CodigoOp::AjustarLocalConLocal, 0, ranura, otra as u32)
    }
    pub const fn CargarLocalMiembro(ranura: u16, nombre: u32) -> Self {
        Self::nueva(CodigoOp::CargarLocalMiembro, 0, ranura, nombre)
    }
    pub const fn CargarLocalAtributo(ranura: u16, slot: u16) -> Self {
        Self::nueva(CodigoOp::CargarLocalAtributo, 0, ranura, slot as u32)
    }
    pub const fn RetornarLocal(ranura: u16) -> Self {
        Self::nueva(CodigoOp::RetornarLocal, 0, ranura, 0)
    }
    pub const fn LlamarNativa(id: u16, cantidad: u16) -> Self {
        Self::nueva(CodigoOp::LlamarNativa, 0, id, cantidad as u32)
    }

    pub fn entero_inmediato(self) -> i16 {
        self.a as i16
    }
    pub fn ranura(self) -> u16 {
        self.a
    }
    pub fn delta_ajuste(self) -> i16 {
        self.b as u16 as i16
    }
    pub fn indice_u16(self) -> u16 {
        self.a
    }
    pub fn indice_u32(self) -> u32 {
        self.b
    }
    pub fn cantidad(self) -> u32 {
        self.b
    }
    pub fn destino(self) -> u32 {
        self.b
    }
    pub fn ranuras_intentar(self) -> u16 {
        self.a
    }
    pub fn indice_nombre(self) -> u32 {
        self.b
    }
    pub fn cantidad_metodo(self) -> u32 {
        self.a as u32
    }
    pub fn slot(self) -> u16 {
        self.a
    }
    pub fn slot_atributo(self) -> u16 {
        self.b as u16
    }
    pub fn ranura_derecha(self) -> u16 {
        self.b as u16
    }
    pub fn id_nativo(self) -> u16 {
        self.a
    }

    /// Destino absoluto si esta instrucción es un salto parchable.
    pub fn destino_salto(self) -> Option<u32> {
        match self.op {
            CodigoOp::Saltar
            | CodigoOp::SaltarSiFalso
            | CodigoOp::SaltarSiVerdadero
            | CodigoOp::SaltarSiFalsoYDejar
            | CodigoOp::SaltarSiVerdaderoYDejar
            | CodigoOp::IteradorSiguiente
            | CodigoOp::EntrarIntentar
            | CodigoOp::MenorEnteroSaltarSiFalso => Some(self.b),
            _ => None,
        }
    }

    /// Copia la instrucción con un destino de salto nuevo.
    pub fn con_destino_salto(self, destino: u32) -> Self {
        if self.destino_salto().is_some() {
            Self { b: destino, ..self }
        } else {
            self
        }
    }

    pub fn es_binaria_generica(self) -> bool {
        matches!(
            self.op,
            CodigoOp::Sumar
                | CodigoOp::Restar
                | CodigoOp::Multiplicar
                | CodigoOp::Dividir
                | CodigoOp::Modulo
                | CodigoOp::Igual
                | CodigoOp::Diferente
                | CodigoOp::Mayor
                | CodigoOp::Menor
                | CodigoOp::MayorOIgual
                | CodigoOp::MenorOIgual
        )
    }

    pub fn es_binaria_entera(self) -> bool {
        matches!(
            self.op,
            CodigoOp::SumarEntero
                | CodigoOp::RestarEntero
                | CodigoOp::MultiplicarEntero
                | CodigoOp::DividirEntero
                | CodigoOp::ModuloEntero
                | CodigoOp::IgualEntero
                | CodigoOp::DiferenteEntero
                | CodigoOp::MayorEntero
                | CodigoOp::MenorEntero
                | CodigoOp::MayorOIgualEntero
                | CodigoOp::MenorOIgualEntero
        )
    }
}

/// Secuencia de instrucciones con la ubicación de origen de cada una.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Trozo {
    pub instrucciones: Vec<Instruccion>,
    pub ubicaciones: Vec<Ubicacion>,
    /// Máxima profundidad de operandos, calculada por el verificador.
    #[serde(default)]
    pub max_pila: u16,
}

impl Trozo {
    pub fn emitir(&mut self, instruccion: Instruccion, ubicacion: Ubicacion) -> u32 {
        self.instrucciones.push(instruccion);
        self.ubicaciones.push(ubicacion);
        (self.instrucciones.len() - 1) as u32
    }

    pub fn posicion_actual(&self) -> u32 {
        self.instrucciones.len() as u32
    }

    /// Reemplaza el destino de un salto emitido previamente.
    pub fn parchar_salto(&mut self, indice: u32, destino: u32) {
        let indice = indice as usize;
        let actual = self.instrucciones[indice];
        if actual.destino_salto().is_none() {
            unreachable!("la instrucción {actual:?} no es un salto parchable")
        }
        self.instrucciones[indice] = actual.con_destino_salto(destino);
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn instruccion_deberia_medir_ocho_bytes() {
        assert_eq!(std::mem::size_of::<Instruccion>(), 8);
        assert_eq!(std::mem::size_of::<CodigoOp>(), 1);
    }
}
