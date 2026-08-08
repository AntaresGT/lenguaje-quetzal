//! Valores en runtime del Lenguaje Quetzal.

use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;

use bytecode::{FuncionCompilada, ModuloCompilado, ObjetoCompilado};
use indexmap::IndexMap;
use rust_decimal::Decimal;

use crate::bucle_eventos::CargaNativa;
use crate::errores::DatosExcepcion;

/// Entorno de un módulo cargado: su bytecode y sus variables globales.
///
/// Las funciones y objetos llevan una referencia a su entorno para que un
/// símbolo importado siga resolviendo nombres dentro de su propio módulo.
#[derive(Debug)]
pub struct EntornoModulo {
    pub modulo: Rc<ModuloCompilado>,
    pub globales: RefCell<IndexMap<String, Variable>>,
}

/// Un valor de Quetzal en ejecución.
#[derive(Debug, Clone)]
pub enum Valor {
    Entero(i64),
    /// Decimal exacto: `0.1 + 0.2 == 0.3`.
    Numero(Decimal),
    Texto(Rc<str>),
    Log(bool),
    Nulo,
    Lista(Rc<RefCell<Vec<Valor>>>),
    Jsn(Rc<RefCell<IndexMap<String, Valor>>>),
    /// Función definida en Quetzal, junto a su entorno de módulo.
    Funcion(Rc<FuncionCompilada>, Rc<EntornoModulo>),
    /// Función nativa registrada por nombre (`consola.mostrar`, `rango`).
    Nativa(Rc<str>),
    /// Módulo nativo importado (`Matemática` → `matematica`).
    ModuloNativo(Rc<str>),
    /// Definición de objeto cargada (permite miembros `libre` y `nuevo`).
    Clase(Rc<ClaseObjeto>),
    /// Instancia de un objeto.
    Instancia(Rc<RefCell<Instancia>>),
    /// Instancia de un objeto nativo (p. ej. `Tiempo`).
    InstanciaNativa(Rc<DatosInstanciaNativa>),
    /// Vista `padre` dentro de un método: la instancia vista como su objeto
    /// padre (con un padre concreto opcional: `padre.Mamifero.comer()`).
    Padre {
        instancia: Rc<RefCell<Instancia>>,
        clase: Option<Rc<str>>,
    },
    /// Resultado pendiente de una función `asincrono` (se resuelve con `esperar`).
    Tarea(Rc<DatosTarea>),
    /// Tarea nativa asincrónica pendiente en el bucle de eventos (E/S de red,
    /// sockets, ...). Se resuelve con `esperar`, igual que una [`Valor::Tarea`].
    TareaNativa(Rc<EstadoTareaNativa>),
    /// Valor capturado por `capturar (excepcion e)`.
    Excepcion(Rc<DatosExcepcion>),
    /// Iterador interno de los bucles `para ... en/cada`.
    Iterador(Rc<RefCell<EstadoIterador>>),
}

/// Variable declarada con su mutabilidad.
#[derive(Debug, Clone)]
pub struct Variable {
    pub valor: Valor,
    pub mutable: bool,
}

/// Definición de objeto en runtime.
#[derive(Debug)]
pub struct ClaseObjeto {
    pub compilado: ObjetoCompilado,
    pub entorno: Rc<EntornoModulo>,
    /// Atributos `libre` compartidos (estado de la definición, no de instancias).
    pub atributos_libres: RefCell<IndexMap<String, Variable>>,
}

/// Instancia de un objeto en runtime. Los atributos de toda la cadena de
/// herencia viven aplanados en el mismo mapa.
#[derive(Debug)]
pub struct Instancia {
    pub clase: Rc<ClaseObjeto>,
    pub atributos: IndexMap<String, Variable>,
}

/// Instancia de un objeto implementado en Rust (módulos nativos).
///
/// `tipo` despacha los métodos como `{tipo}.{metodo}` en el registro de
/// nativos y `datos` guarda el estado interno como valores de Quetzal, de
/// modo que la VM no depende de los crates de cada módulo. Si existe un campo
/// `texto`, se usa como representación al imprimir.
#[derive(Debug)]
pub struct DatosInstanciaNativa {
    pub tipo: Rc<str>,
    pub datos: RefCell<IndexMap<String, Valor>>,
}

/// Tarea asincrónica pendiente.
#[derive(Debug)]
pub struct DatosTarea {
    pub funcion: Rc<FuncionCompilada>,
    pub entorno: Rc<EntornoModulo>,
    pub argumentos: Vec<Valor>,
    /// `esto` capturado si la tarea proviene de un método.
    pub instancia: Option<Valor>,
}

/// Estado de un iterador de lista.
#[derive(Debug)]
pub struct EstadoIterador {
    pub elementos: Vec<Valor>,
    pub posicion: usize,
}

/// Identificador de una tarea nativa asincrónica pendiente en el bucle de
/// eventos (ver [`crate::bucle_eventos`]). El resultado real viaja por el
/// canal del bucle como [`CargaNativa`] y se convierte a [`Valor`] al
/// resolverse (`esperar`), vía [`carga_a_valor`].
#[derive(Debug)]
pub struct EstadoTareaNativa {
    pub id: u64,
}

impl Valor {
    pub fn texto(texto: impl AsRef<str>) -> Self {
        Valor::Texto(Rc::from(texto.as_ref()))
    }

    pub fn lista(valores: Vec<Valor>) -> Self {
        Valor::Lista(Rc::new(RefCell::new(valores)))
    }

    pub fn jsn(mapa: IndexMap<String, Valor>) -> Self {
        Valor::Jsn(Rc::new(RefCell::new(mapa)))
    }

    /// Nombre del tipo en runtime, usado para despachar métodos nativos.
    pub fn nombre_tipo(&self) -> &'static str {
        match self {
            Valor::Entero(_) => "entero",
            Valor::Numero(_) => "numero",
            Valor::Texto(_) => "texto",
            Valor::Log(_) => "log",
            Valor::Nulo => "nulo",
            Valor::Lista(_) => "lista",
            Valor::Jsn(_) => "jsn",
            Valor::Funcion(..) | Valor::Nativa(_) => "funcion",
            Valor::ModuloNativo(_) => "modulo",
            Valor::Clase(_) => "objeto",
            Valor::Instancia(_) | Valor::Padre { .. } => "instancia",
            Valor::InstanciaNativa(_) => "instancia nativa",
            Valor::Tarea(_) | Valor::TareaNativa(_) => "tarea",
            Valor::Excepcion(_) => "excepcion",
            Valor::Iterador(_) => "iterador",
        }
    }

    /// Igualdad estructural entre valores.
    pub fn es_igual(&self, otro: &Valor) -> bool {
        match (self, otro) {
            (Valor::Entero(a), Valor::Entero(b)) => a == b,
            (Valor::Numero(a), Valor::Numero(b)) => a == b,
            (Valor::Entero(a), Valor::Numero(b)) | (Valor::Numero(b), Valor::Entero(a)) => {
                Decimal::from(*a) == *b
            }
            (Valor::Texto(a), Valor::Texto(b)) => a == b,
            (Valor::Log(a), Valor::Log(b)) => a == b,
            (Valor::Nulo, Valor::Nulo) => true,
            (Valor::Lista(a), Valor::Lista(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                a.len() == b.len()
                    && a.iter()
                        .zip(b.iter())
                        .all(|(izquierda, derecha)| izquierda.es_igual(derecha))
            }
            (Valor::Jsn(a), Valor::Jsn(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                a.len() == b.len()
                    && a.iter().all(|(clave, valor)| {
                        b.get(clave)
                            .map(|otro_valor| valor.es_igual(otro_valor))
                            .unwrap_or(false)
                    })
            }
            (Valor::Instancia(a), Valor::Instancia(b)) => Rc::ptr_eq(a, b),
            (Valor::InstanciaNativa(a), Valor::InstanciaNativa(b)) => {
                if a.tipo != b.tipo {
                    return false;
                }
                let datos_a = a.datos.borrow();
                let datos_b = b.datos.borrow();
                datos_a.len() == datos_b.len()
                    && datos_a.iter().all(|(clave, valor)| {
                        datos_b
                            .get(clave)
                            .map(|otro_valor| valor.es_igual(otro_valor))
                            .unwrap_or(false)
                    })
            }
            _ => false,
        }
    }
}

/// Representación textual de un valor (para `consola.mostrar`, `.texto()` e
/// interpolación).
pub fn texto_de_valor(valor: &Valor) -> String {
    match valor {
        Valor::Entero(entero) => entero.to_string(),
        Valor::Numero(decimal) => decimal.normalize().to_string(),
        Valor::Texto(texto) => texto.to_string(),
        Valor::Log(true) => "verdadero".to_string(),
        Valor::Log(false) => "falso".to_string(),
        Valor::Nulo => "nulo".to_string(),
        Valor::Lista(elementos) => {
            let elementos = elementos.borrow();
            let interior: Vec<String> = elementos.iter().map(texto_de_valor).collect();
            format!("[{}]", interior.join(", "))
        }
        Valor::Jsn(_) => jsn_a_texto(valor, false),
        Valor::Funcion(funcion, _) => format!("<función {}>", funcion.nombre),
        Valor::Nativa(nombre) => format!("<función nativa {nombre}>"),
        Valor::ModuloNativo(nombre) => format!("<módulo {nombre}>"),
        Valor::Clase(clase) => format!("<objeto {}>", clase.compilado.nombre),
        Valor::Instancia(instancia) => {
            format!(
                "<instancia de {}>",
                instancia.borrow().clase.compilado.nombre
            )
        }
        Valor::Padre { instancia, .. } => {
            format!("<padre de {}>", instancia.borrow().clase.compilado.nombre)
        }
        Valor::InstanciaNativa(instancia) => match instancia.datos.borrow().get("texto") {
            Some(Valor::Texto(texto)) => texto.to_string(),
            _ => format!("<{}>", instancia.tipo),
        },
        Valor::Tarea(_) | Valor::TareaNativa(_) => "<tarea pendiente>".to_string(),
        Valor::Excepcion(datos) => datos.mensaje.clone(),
        Valor::Iterador(_) => "<iterador>".to_string(),
    }
}

/// Serializa un valor como texto JSON (`.texto()` y `.texto_formateado()` de jsn).
pub fn jsn_a_texto(valor: &Valor, formateado: bool) -> String {
    let json = valor_a_json(valor);
    if formateado {
        serde_json::to_string_pretty(&json).unwrap_or_else(|_| "null".to_string())
    } else {
        serde_json::to_string(&json).unwrap_or_else(|_| "null".to_string())
    }
}

/// Convierte un valor de Quetzal a `serde_json::Value`.
pub fn valor_a_json(valor: &Valor) -> serde_json::Value {
    match valor {
        Valor::Entero(entero) => serde_json::Value::from(*entero),
        Valor::Numero(decimal) => {
            // Se serializa como número JSON conservando los dígitos exactos.
            serde_json::Number::from_str(&decimal.normalize().to_string())
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        }
        Valor::Texto(texto) => serde_json::Value::String(texto.to_string()),
        Valor::Log(valor_logico) => serde_json::Value::Bool(*valor_logico),
        Valor::Lista(elementos) => {
            serde_json::Value::Array(elementos.borrow().iter().map(valor_a_json).collect())
        }
        Valor::Jsn(mapa) => serde_json::Value::Object(
            mapa.borrow()
                .iter()
                .map(|(clave, valor)| (clave.clone(), valor_a_json(valor)))
                .collect(),
        ),
        _ => serde_json::Value::Null,
    }
}

/// Convierte un `serde_json::Value` a valor de Quetzal.
pub fn json_a_valor(json: &serde_json::Value) -> Valor {
    match json {
        serde_json::Value::Null => Valor::Nulo,
        serde_json::Value::Bool(valor_logico) => Valor::Log(*valor_logico),
        serde_json::Value::Number(numero) => {
            if let Some(entero) = numero.as_i64() {
                Valor::Entero(entero)
            } else {
                Decimal::from_str(&numero.to_string())
                    .map(Valor::Numero)
                    .unwrap_or(Valor::Nulo)
            }
        }
        serde_json::Value::String(texto) => Valor::texto(texto),
        serde_json::Value::Array(elementos) => {
            Valor::lista(elementos.iter().map(json_a_valor).collect())
        }
        serde_json::Value::Object(mapa) => Valor::jsn(
            mapa.iter()
                .map(|(clave, valor)| (clave.clone(), json_a_valor(valor)))
                .collect(),
        ),
    }
}

// ----- Bucle de eventos: conversión Valor <-> CargaNativa -----

/// Convierte el resultado (enviable entre hilos) de una tarea nativa al
/// valor de Quetzal equivalente.
pub fn carga_a_valor(carga: CargaNativa) -> Valor {
    match carga {
        CargaNativa::Nula => Valor::Nulo,
        CargaNativa::Entero(entero) => Valor::Entero(entero),
        CargaNativa::Log(valor_logico) => Valor::Log(valor_logico),
        CargaNativa::Texto(texto) => Valor::texto(texto),
        CargaNativa::Lista(elementos) => {
            Valor::lista(elementos.into_iter().map(carga_a_valor).collect())
        }
        CargaNativa::Mapa(pares) => Valor::jsn(
            pares
                .into_iter()
                .map(|(clave, valor)| (clave, carga_a_valor(valor)))
                .collect(),
        ),
        CargaNativa::Instancia { tipo, campos } => {
            Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
                tipo: Rc::from(tipo.as_str()),
                datos: RefCell::new(
                    campos
                        .into_iter()
                        .map(|(clave, valor)| (clave, carga_a_valor(valor)))
                        .collect(),
                ),
            }))
        }
    }
}

/// Convierte un valor de Quetzal a una carga enviable entre hilos, para que
/// un hilo de fondo (por ejemplo, la conexión de un servidor) pueda usar lo
/// que produjo un manejador de Quetzal. Los tipos sin equivalente binario se
/// serializan como su representación textual.
pub fn valor_a_carga(valor: &Valor) -> CargaNativa {
    match valor {
        Valor::Nulo => CargaNativa::Nula,
        Valor::Entero(entero) => CargaNativa::Entero(*entero),
        Valor::Log(valor_logico) => CargaNativa::Log(*valor_logico),
        Valor::Texto(texto) => CargaNativa::Texto(texto.to_string()),
        Valor::Lista(lista) => {
            CargaNativa::Lista(lista.borrow().iter().map(valor_a_carga).collect())
        }
        Valor::Jsn(mapa) => CargaNativa::Mapa(
            mapa.borrow()
                .iter()
                .map(|(clave, valor)| (clave.clone(), valor_a_carga(valor)))
                .collect(),
        ),
        Valor::InstanciaNativa(instancia) => CargaNativa::Instancia {
            tipo: instancia.tipo.to_string(),
            campos: instancia
                .datos
                .borrow()
                .iter()
                .map(|(clave, valor)| (clave.clone(), valor_a_carga(valor)))
                .collect(),
        },
        otro => CargaNativa::Texto(texto_de_valor(otro)),
    }
}
