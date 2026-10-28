//! Registro de funciones y constantes nativas.
//!
//! El crate `modulos_nativos` llena este registro y el `motor` lo instala en
//! la VM. Las claves usan la forma `modulo.funcion` (`consola.mostrar`,
//! `matematica.sumar`) o `tipo.metodo` (`texto.mayusculas`, `lista.agregar`).
//!
//! El camino caliente despacha por [`IdNativo`] (índice), sin hashear nombres.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::errores::Fallo;
use crate::valores::Valor;
use crate::vm::Vm;

/// Identificador estable de una función nativa, asignado al registrar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdNativo(pub u32);

/// Una función nativa: recibe los argumentos (y el receptor como primer
/// argumento cuando es un método) y devuelve un valor o un fallo.
pub type FuncionNativa = Box<dyn Fn(&[Valor]) -> Result<Valor, Fallo>>;

/// Una función nativa con acceso a la VM.
pub type FuncionNativaConVm = Box<dyn Fn(&mut Vm, &[Valor]) -> Result<Valor, Fallo>>;

enum Entrada {
    Simple(FuncionNativa),
    ConVm(FuncionNativaConVm),
}

/// Registro central de funciones, constantes y módulos nativos.
#[derive(Default)]
pub struct RegistroNativos {
    entradas: Vec<Entrada>,
    nombres: Vec<String>,
    indice: HashMap<String, IdNativo>,
    constantes: HashMap<String, Valor>,
    /// `tipo` → `metodo` → id, con claves internadas para lookup sin asignar.
    metodos: HashMap<Rc<str>, HashMap<Rc<str>, IdNativo>>,
    constantes_miembro: HashMap<Rc<str>, HashMap<Rc<str>, Valor>>,
    modulos: HashSet<String>,
}

impl RegistroNativos {
    pub fn nuevo() -> Self {
        Self::default()
    }

    /// Registra una función nativa, por ejemplo `consola.mostrar`.
    pub fn registrar_funcion(&mut self, nombre: &str, funcion: FuncionNativa) {
        self.registrar(nombre, Entrada::Simple(funcion));
    }

    /// Registra una función nativa con acceso a la VM.
    pub fn registrar_funcion_con_vm(&mut self, nombre: &str, funcion: FuncionNativaConVm) {
        self.registrar(nombre, Entrada::ConVm(funcion));
    }

    fn registrar(&mut self, nombre: &str, entrada: Entrada) {
        let clave = normalizar_nombre(nombre);
        if let Some(id) = self.indice.get(&clave) {
            self.entradas[id.0 as usize] = entrada;
            return;
        }
        let id = IdNativo(self.entradas.len() as u32);
        self.entradas.push(entrada);
        self.nombres.push(clave.clone());
        self.indice.insert(clave.clone(), id);
        if let Some((tipo, metodo)) = clave.split_once('.') {
            self.metodos
                .entry(Rc::from(tipo))
                .or_default()
                .insert(Rc::from(metodo), id);
        }
    }

    /// Registra una constante nativa, por ejemplo `matematica.PI`.
    pub fn registrar_constante(&mut self, nombre: &str, valor: Valor) {
        let clave = normalizar_nombre(nombre);
        if let Some((tipo, miembro)) = clave.split_once('.') {
            self.constantes_miembro
                .entry(Rc::from(tipo))
                .or_default()
                .insert(Rc::from(miembro), valor.clone());
        }
        self.constantes.insert(clave, valor);
    }

    /// Declara que existe un módulo nativo (`matematica`, `tiempo`, ...).
    pub fn registrar_modulo(&mut self, nombre: &str) {
        self.modulos.insert(normalizar_nombre(nombre));
    }

    pub fn existe_modulo(&self, nombre: &str) -> bool {
        self.modulos.contains(&normalizar_nombre(nombre))
    }

    pub fn id_de(&self, nombre: &str) -> Option<IdNativo> {
        self.indice.get(&normalizar_nombre(nombre)).copied()
    }

    pub fn nombre_de(&self, id: IdNativo) -> Option<&str> {
        self.nombres.get(id.0 as usize).map(String::as_str)
    }

    pub fn metodo_de(&self, tipo: &str, nombre: &str) -> Option<IdNativo> {
        let tipo = normalizar_nombre(tipo);
        let nombre = normalizar_nombre(nombre);
        self.metodos
            .get(tipo.as_str())?
            .get(nombre.as_str())
            .copied()
    }

    pub fn constante_miembro(&self, modulo: &str, nombre: &str) -> Option<Valor> {
        let modulo = normalizar_nombre(modulo);
        let nombre = normalizar_nombre(nombre);
        self.constantes_miembro
            .get(modulo.as_str())?
            .get(nombre.as_str())
            .cloned()
    }

    pub fn buscar_funcion(&self, nombre: &str) -> Option<&FuncionNativa> {
        let id = self.id_de(nombre)?;
        match self.entradas.get(id.0 as usize)? {
            Entrada::Simple(funcion) => Some(funcion),
            Entrada::ConVm(_) => None,
        }
    }

    pub fn buscar_funcion_con_vm(&self, nombre: &str) -> Option<&FuncionNativaConVm> {
        let id = self.id_de(nombre)?;
        match self.entradas.get(id.0 as usize)? {
            Entrada::ConVm(funcion) => Some(funcion),
            Entrada::Simple(_) => None,
        }
    }

    pub fn existe_funcion(&self, nombre: &str) -> bool {
        self.id_de(nombre).is_some()
    }

    pub fn existe_id(&self, id: IdNativo) -> bool {
        (id.0 as usize) < self.entradas.len()
    }

    pub fn buscar_constante(&self, nombre: &str) -> Option<&Valor> {
        self.constantes.get(&normalizar_nombre(nombre))
    }

    pub fn invocar(&self, id: IdNativo, argumentos: &[Valor]) -> Result<Valor, Fallo> {
        match self.entradas.get(id.0 as usize) {
            Some(Entrada::Simple(funcion)) => funcion(argumentos),
            Some(Entrada::ConVm(_)) => Err(Fallo::excepcion(
                "E0201",
                "la función nativa requiere la VM",
                Vec::new(),
                None,
            )),
            None => Err(Fallo::excepcion(
                "E0201",
                "la función nativa no existe",
                Vec::new(),
                None,
            )),
        }
    }

    pub fn es_con_vm(&self, id: IdNativo) -> bool {
        matches!(self.entradas.get(id.0 as usize), Some(Entrada::ConVm(_)))
    }

    pub fn invocar_con_vm(
        &self,
        vm: &mut Vm,
        id: IdNativo,
        argumentos: &[Valor],
    ) -> Result<Valor, Fallo> {
        match self.entradas.get(id.0 as usize) {
            Some(Entrada::Simple(funcion)) => funcion(argumentos),
            Some(Entrada::ConVm(funcion)) => funcion(vm, argumentos),
            None => Err(Fallo::excepcion(
                "E0201",
                "la función nativa no existe",
                Vec::new(),
                None,
            )),
        }
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
