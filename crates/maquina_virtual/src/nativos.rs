//! Registro de funciones y constantes nativas.
//!
//! El crate `modulos_nativos` llena este registro y el `motor` lo instala en
//! la VM. Las claves usan la forma `modulo.funcion` (`consola.mostrar`,
//! `matematica.sumar`) o `tipo.metodo` (`texto.mayusculas`, `lista.agregar`).

use std::collections::{HashMap, HashSet};

use crate::errores::Fallo;
use crate::valores::Valor;

/// Una función nativa: recibe los argumentos (y el receptor como primer
/// argumento cuando es un método) y devuelve un valor o un fallo.
pub type FuncionNativa = Box<dyn Fn(&[Valor]) -> Result<Valor, Fallo>>;

/// Registro central de funciones, constantes y módulos nativos.
#[derive(Default)]
pub struct RegistroNativos {
    funciones: HashMap<String, FuncionNativa>,
    constantes: HashMap<String, Valor>,
    modulos: HashSet<String>,
}

impl RegistroNativos {
    pub fn nuevo() -> Self {
        Self::default()
    }

    /// Registra una función nativa, por ejemplo `consola.mostrar`.
    pub fn registrar_funcion(&mut self, nombre: &str, funcion: FuncionNativa) {
        self.funciones.insert(normalizar_nombre(nombre), funcion);
    }

    /// Registra una constante nativa, por ejemplo `matematica.PI`.
    pub fn registrar_constante(&mut self, nombre: &str, valor: Valor) {
        self.constantes.insert(normalizar_nombre(nombre), valor);
    }

    /// Declara que existe un módulo nativo (`matematica`, `tiempo`, ...).
    pub fn registrar_modulo(&mut self, nombre: &str) {
        self.modulos.insert(normalizar_nombre(nombre));
    }

    pub fn existe_modulo(&self, nombre: &str) -> bool {
        self.modulos.contains(&normalizar_nombre(nombre))
    }

    pub fn buscar_funcion(&self, nombre: &str) -> Option<&FuncionNativa> {
        self.funciones.get(&normalizar_nombre(nombre))
    }

    pub fn buscar_constante(&self, nombre: &str) -> Option<&Valor> {
        self.constantes.get(&normalizar_nombre(nombre))
    }
}

/// Normaliza un nombre nativo: minúsculas no, solo se quitan tildes para que
/// `matemática.número` y `matematica.numero` coincidan.
///
/// (Copia mínima de `lexico::normalizacion::quitar_tildes` para no acoplar la
/// VM al lexer.)
pub fn normalizar_nombre(texto: &str) -> String {
    texto
        .chars()
        .map(|caracter| match caracter {
            'á' | 'à' => 'a',
            'é' | 'è' => 'e',
            'í' | 'ì' => 'i',
            'ó' | 'ò' => 'o',
            'ú' | 'ù' => 'u',
            'Á' | 'À' => 'A',
            'É' | 'È' => 'E',
            'Í' | 'Ì' => 'I',
            'Ó' | 'Ò' => 'O',
            'Ú' | 'Ù' => 'U',
            otro => otro,
        })
        .collect()
}
