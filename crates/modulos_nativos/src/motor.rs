//! Módulo nativo `quetzal/motor`: utilería general del lenguaje.
//!
//! Esta primera entrega expone el objeto `ExpresiónRegular` (alias válido:
//! `ExpresionRegular`). Al importar `{ ExpresiónRegular }` desde
//! `"quetzal/motor"` el símbolo resuelve al módulo nativo `motor`, cuyo
//! `motor.constructor` atiende `nuevo ExpresiónRegular(patron)`.
//!
//! Cada instancia es inmutable: guarda el patrón original y sus cuatro
//! banderas (`ignorar_mayúsculas`, `multilínea`, `punto_total`, `unicode`),
//! y los métodos `con_*` devuelven una instancia nueva. Las búsquedas que no
//! encuentran nada devuelven `nulo` (o listas vacías), nunca lanzan.

use std::cell::RefCell;
use std::rc::Rc;

use indexmap::IndexMap;
use maquina_virtual::{DatosInstanciaNativa, Fallo, RegistroNativos, Valor};
use regex::{Regex, RegexBuilder};

use crate::util::{arg_texto, error, exigir_aridad};

/// Nombre del tipo visible para el usuario (las claves se normalizan sin
/// tildes, así que `ExpresionRegular` queda cubierto como alias).
const TIPO: &str = "ExpresiónRegular";

/// Función nativa de este módulo, registrable bajo varios nombres.
type Nativa = fn(&[Valor]) -> Result<Valor, Fallo>;

// ----- Banderas -----

/// Las cuatro banderas de una instancia, en el orden en que se guardan.
///
/// `unicode` nace activada: el motor trabaja sobre textos UTF-8 de Quetzal,
/// y desactivarla haría que `.` pudiera cortar caracteres a la mitad.
#[derive(Clone, Copy)]
struct Banderas {
    ignorar_mayusculas: bool,
    multilinea: bool,
    punto_total: bool,
    unicode: bool,
}

impl Default for Banderas {
    fn default() -> Self {
        Self {
            ignorar_mayusculas: false,
            multilinea: false,
            punto_total: false,
            unicode: true,
        }
    }
}

impl Banderas {
    /// Texto con las banderas activas al estilo `ims`, para la
    /// representación `/patron/banderas` de la instancia.
    fn texto(&self) -> String {
        let mut texto = String::new();
        if self.ignorar_mayusculas {
            texto.push('i');
        }
        if self.multilinea {
            texto.push('m');
        }
        if self.punto_total {
            texto.push('s');
        }
        if self.unicode {
            texto.push('u');
        }
        texto
    }

    /// Compila el patrón con estas banderas aplicadas.
    fn compilar(&self, funcion: &str, patron: &str) -> Result<Regex, Fallo> {
        RegexBuilder::new(patron)
            .case_insensitive(self.ignorar_mayusculas)
            .multi_line(self.multilinea)
            .dot_matches_new_line(self.punto_total)
            .unicode(self.unicode)
            .build()
            .map_err(|fallo| {
                error(
                    "E0406",
                    format!("'{funcion}' recibió una expresión regular inválida: {fallo}"),
                )
            })
    }
}

// ----- Instancias -----

/// Crea una instancia validando que el patrón compile con las banderas.
fn instancia(funcion: &str, patron: &str, banderas: Banderas) -> Result<Valor, Fallo> {
    // La construcción valida: un patrón que no compila es una excepción y
    // nunca produce un objeto en estado inválido.
    banderas.compilar(funcion, patron)?;
    Ok(instancia_sin_validar(patron, banderas))
}

/// Crea la instancia sin recompilar (las banderas ya vienen validadas).
fn instancia_sin_validar(patron: &str, banderas: Banderas) -> Valor {
    let mut datos = IndexMap::new();
    datos.insert("patron".to_string(), Valor::texto(patron));
    datos.insert(
        "ignorar_mayusculas".to_string(),
        Valor::Log(banderas.ignorar_mayusculas),
    );
    datos.insert("multilinea".to_string(), Valor::Log(banderas.multilinea));
    datos.insert("punto_total".to_string(), Valor::Log(banderas.punto_total));
    datos.insert("unicode".to_string(), Valor::Log(banderas.unicode));
    // Texto visible al interpolar: `/patron/ims` (útil en consola).
    datos.insert(
        "texto".to_string(),
        Valor::texto(format!("/{patron}/{}", banderas.texto())),
    );
    Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO),
        datos: RefCell::new(datos),
    }))
}

/// Extrae patrón y banderas de una instancia de `ExpresiónRegular`.
fn arg_expresion(
    funcion: &str,
    argumentos: &[Valor],
    indice: usize,
) -> Result<(String, Banderas), Fallo> {
    match argumentos.get(indice) {
        Some(Valor::InstanciaNativa(datos)) if &*datos.tipo == TIPO => {
            let datos = datos.datos.borrow();
            let patron = match datos.get("patron") {
                Some(Valor::Texto(patron)) => patron.to_string(),
                _ => {
                    return Err(error(
                        "E0406",
                        format!("'{funcion}' recibió una {TIPO} sin patrón interno"),
                    ));
                }
            };
            let bandera = |nombre: &str| matches!(datos.get(nombre), Some(Valor::Log(verdadero)) if *verdadero);
            let banderas = Banderas {
                ignorar_mayusculas: bandera("ignorar_mayusculas"),
                multilinea: bandera("multilinea"),
                punto_total: bandera("punto_total"),
                unicode: bandera("unicode"),
            };
            Ok((patron, banderas))
        }
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba una instancia de {TIPO} en el argumento {}, pero recibió '{}'",
                indice + 1,
                otro.nombre_tipo()
            ),
        )),
        None => Err(error(
            "E0210",
            format!("'{funcion}' necesita al menos {} argumentos", indice + 1),
        )),
    }
}

/// Receptor (`esto`) de un método: patrón, banderas y regex compilada.
fn receptor(funcion: &str, argumentos: &[Valor]) -> Result<(String, Banderas, Regex), Fallo> {
    let (patron, banderas) = arg_expresion(funcion, argumentos, 0)?;
    let regex = banderas.compilar(funcion, &patron)?;
    Ok((patron, banderas, regex))
}

/// Convierte un desfase en bytes a índice de caracteres dentro del texto.
fn byte_a_caracter(texto: &str, byte: usize) -> i64 {
    texto[..byte].chars().count() as i64
}

// ----- Constructor: `nuevo ExpresiónRegular(...)` -----

fn constructor(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "nuevo ExpresiónRegular";
    exigir_aridad(F, argumentos, 1)?;
    let patron = arg_texto(F, argumentos, 0)?;
    instancia(F, patron, Banderas::default())
}

// ----- Funciones libres: `ExpresiónRegular.x()` -----

/// Escapa los metacaracteres de un texto literal.
fn libre_escapar(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ExpresiónRegular.escapar";
    exigir_aridad(F, argumentos, 1)?;
    let literal = arg_texto(F, argumentos, 0)?;
    Ok(Valor::texto(regex::escape(literal)))
}

/// Convierte un comodín estilo shell (`*` y `?`) a una expresión regular.
fn libre_nueva_desde_comodin(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ExpresiónRegular.nueva_desde_comodín";
    exigir_aridad(F, argumentos, 1)?;
    let comodin = arg_texto(F, argumentos, 0)?;
    let mut patron = String::from("^");
    for caracter in comodin.chars() {
        match caracter {
            '*' => patron.push_str(".*"),
            '?' => patron.push('.'),
            otro => patron.push_str(&regex::escape(&otro.to_string())),
        }
    }
    patron.push('$');
    instancia(F, &patron, Banderas::default())
}

fn libre_nueva_solo_digitos(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ExpresiónRegular.nueva_solo_dígitos";
    exigir_aridad(F, argumentos, 0)?;
    instancia(F, r"^\d+$", Banderas::default())
}

fn libre_nueva_solo_letras(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ExpresiónRegular.nueva_solo_letras";
    exigir_aridad(F, argumentos, 0)?;
    instancia(F, r"^[A-Za-zÁÉÍÓÚÜáéíóúüÑñ]+$", Banderas::default())
}

fn libre_nueva_correo(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ExpresiónRegular.nueva_correo";
    exigir_aridad(F, argumentos, 0)?;
    // Formato habitual usuario@dominio.ext (no exhaustivo respecto al RFC).
    // Sin anclas: sirve para `buscar` en textos largos y para
    // `coincide_completo` en validaciones.
    instancia(
        F,
        r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}",
        Banderas::default(),
    )
}

fn libre_nueva_url(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ExpresiónRegular.nueva_url";
    exigir_aridad(F, argumentos, 0)?;
    instancia(
        F,
        r"https?://[A-Za-z0-9.-]+(?::\d+)?(?:/[^\s]*)?",
        Banderas::default(),
    )
}

fn libre_nueva_ipv4(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ExpresiónRegular.nueva_ipv4";
    exigir_aridad(F, argumentos, 0)?;
    instancia(F, r"^(\d{1,3}\.){3}\d{1,3}$", Banderas::default())
}

// ----- Métodos de instancia: coincidencia -----

fn metodo_coincide(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ExpresiónRegular.coincide";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let (_, _, regex) = receptor(F, argumentos)?;
    let texto = arg_texto(F, argumentos, 1)?;
    Ok(Valor::Log(regex.is_match(texto)))
}

fn metodo_coincide_desde_inicio(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ExpresiónRegular.coincide_desde_inicio";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let (_, _, regex) = receptor(F, argumentos)?;
    let texto = arg_texto(F, argumentos, 1)?;
    let coincide = regex
        .find(texto)
        .map(|hallazgo| hallazgo.start() == 0)
        .unwrap_or(false);
    Ok(Valor::Log(coincide))
}

fn metodo_coincide_completo(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ExpresiónRegular.coincide_completo";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let (_, _, regex) = receptor(F, argumentos)?;
    let texto = arg_texto(F, argumentos, 1)?;
    let coincide = regex
        .find(texto)
        .map(|hallazgo| hallazgo.start() == 0 && hallazgo.end() == texto.len())
        .unwrap_or(false);
    Ok(Valor::Log(coincide))
}

// ----- Métodos de instancia: búsqueda -----

fn metodo_buscar(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ExpresiónRegular.buscar";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let (_, _, regex) = receptor(F, argumentos)?;
    let texto = arg_texto(F, argumentos, 1)?;
    Ok(match regex.find(texto) {
        Some(hallazgo) => Valor::texto(hallazgo.as_str()),
        None => Valor::Nulo,
    })
}

fn metodo_buscar_posicion(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ExpresiónRegular.buscar_posicion";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let (_, _, regex) = receptor(F, argumentos)?;
    let texto = arg_texto(F, argumentos, 1)?;
    let Some(hallazgo) = regex.find(texto) else {
        return Ok(Valor::Nulo);
    };
    let mut mapa = IndexMap::new();
    // Índices en caracteres, coherentes con los métodos de `texto`.
    mapa.insert(
        "inicio".to_string(),
        Valor::Entero(byte_a_caracter(texto, hallazgo.start())),
    );
    mapa.insert(
        "fin".to_string(),
        Valor::Entero(byte_a_caracter(texto, hallazgo.end())),
    );
    mapa.insert("coincidencia".to_string(), Valor::texto(hallazgo.as_str()));
    Ok(Valor::jsn(mapa))
}

fn metodo_buscar_todo(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ExpresiónRegular.buscar_todo";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let (_, _, regex) = receptor(F, argumentos)?;
    let texto = arg_texto(F, argumentos, 1)?;
    let coincidencias: Vec<Valor> = regex
        .find_iter(texto)
        .map(|hallazgo| Valor::texto(hallazgo.as_str()))
        .collect();
    Ok(Valor::lista(coincidencias))
}

fn metodo_buscar_grupos(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ExpresiónRegular.buscar_grupos";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let (_, _, regex) = receptor(F, argumentos)?;
    let texto = arg_texto(F, argumentos, 1)?;
    let nombres: Vec<Option<&str>> = regex.capture_names().collect();
    let mut resultados = Vec::new();
    for grupos in regex.captures_iter(texto) {
        let mut mapa = IndexMap::new();
        for indice in 0..grupos.len() {
            let valor = match grupos.get(indice) {
                Some(grupo) => Valor::texto(grupo.as_str()),
                // Grupo que no participó en esta coincidencia.
                None => Valor::Nulo,
            };
            mapa.insert(indice.to_string(), valor);
            if let Some(Some(nombre)) = nombres.get(indice) {
                let valor = match grupos.name(nombre) {
                    Some(grupo) => Valor::texto(grupo.as_str()),
                    None => Valor::Nulo,
                };
                mapa.insert((*nombre).to_string(), valor);
            }
        }
        resultados.push(Valor::jsn(mapa));
    }
    Ok(Valor::lista(resultados))
}

fn metodo_contar(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ExpresiónRegular.contar";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let (_, _, regex) = receptor(F, argumentos)?;
    let texto = arg_texto(F, argumentos, 1)?;
    Ok(Valor::Entero(regex.find_iter(texto).count() as i64))
}

// ----- Métodos de instancia: reemplazo y división -----

fn metodo_reemplazar(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ExpresiónRegular.reemplazar";
    exigir_aridad(F, &argumentos[1..], 2)?;
    let (_, _, regex) = receptor(F, argumentos)?;
    let texto = arg_texto(F, argumentos, 1)?;
    let nuevo = arg_texto(F, argumentos, 2)?;
    // El reemplazo es literal: `replacen` con `NoExpand` ignora `$1` y
    // `$<nombre>` por diseño.
    Ok(Valor::texto(regex.replacen(
        texto,
        1,
        regex::NoExpand(nuevo),
    )))
}

fn metodo_reemplazar_todo(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ExpresiónRegular.reemplazar_todo";
    exigir_aridad(F, &argumentos[1..], 2)?;
    let (_, _, regex) = receptor(F, argumentos)?;
    let texto = arg_texto(F, argumentos, 1)?;
    let nuevo = arg_texto(F, argumentos, 2)?;
    Ok(Valor::texto(
        regex.replace_all(texto, regex::NoExpand(nuevo)),
    ))
}

fn metodo_dividir(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ExpresiónRegular.dividir";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let (_, _, regex) = receptor(F, argumentos)?;
    let texto = arg_texto(F, argumentos, 1)?;
    let partes: Vec<Valor> = regex.split(texto).map(Valor::texto).collect();
    Ok(Valor::lista(partes))
}

// ----- Métodos de instancia: introspección -----

fn metodo_es_valida(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ExpresiónRegular.es_válida";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let (patron, banderas) = arg_expresion(F, argumentos, 0)?;
    Ok(Valor::Log(banderas.compilar(F, &patron).is_ok()))
}

fn metodo_diagnosticar(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ExpresiónRegular.diagnosticar";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let (patron, banderas) = arg_expresion(F, argumentos, 0)?;
    let mut mapa = IndexMap::new();
    match banderas.compilar(F, &patron) {
        Ok(_) => {
            mapa.insert("valida".to_string(), Valor::Log(true));
            mapa.insert("error".to_string(), Valor::Nulo);
            mapa.insert("posicion".to_string(), Valor::Nulo);
        }
        Err(Fallo::Excepcion(datos)) => {
            mapa.insert("valida".to_string(), Valor::Log(false));
            mapa.insert("error".to_string(), Valor::texto(datos.mensaje));
            // El motor de regex no expone siempre la posición del fallo.
            mapa.insert("posicion".to_string(), Valor::Nulo);
        }
        Err(fallo) => return Err(fallo),
    }
    Ok(Valor::jsn(mapa))
}

fn metodo_patron(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ExpresiónRegular.patrón";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let (patron, _) = arg_expresion(F, argumentos, 0)?;
    Ok(Valor::texto(patron))
}

fn metodo_grupos_nombrados(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "ExpresiónRegular.grupos_nombrados";
    exigir_aridad(F, &argumentos[1..], 0)?;
    let (_, _, regex) = receptor(F, argumentos)?;
    let nombres: Vec<Valor> = regex.capture_names().flatten().map(Valor::texto).collect();
    Ok(Valor::lista(nombres))
}

// ----- Métodos de instancia: banderas (lectura) -----

fn bandera(funcion: &str, argumentos: &[Valor], campo: &str) -> Result<Valor, Fallo> {
    exigir_aridad(funcion, &argumentos[1..], 0)?;
    match argumentos.first() {
        Some(Valor::InstanciaNativa(datos)) if &*datos.tipo == TIPO => {
            let datos = datos.datos.borrow();
            Ok(match datos.get(campo) {
                Some(Valor::Log(valor)) => Valor::Log(*valor),
                _ => Valor::Log(false),
            })
        }
        _ => Err(error(
            "E0406",
            format!("'{funcion}' esperaba una instancia de {TIPO}"),
        )),
    }
}

fn metodo_ignorar_mayusculas(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    bandera(
        "ExpresiónRegular.ignorar_mayúsculas",
        argumentos,
        "ignorar_mayusculas",
    )
}

fn metodo_multilinea(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    bandera("ExpresiónRegular.multilínea", argumentos, "multilinea")
}

fn metodo_punto_total(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    bandera("ExpresiónRegular.punto_total", argumentos, "punto_total")
}

fn metodo_unicode(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    bandera("ExpresiónRegular.unicode", argumentos, "unicode")
}

// ----- Métodos de instancia: banderas (variantes inmutables) -----

/// Crea una instancia nueva con una bandera activada/desactivada.
fn con_bandera(
    funcion: &str,
    argumentos: &[Valor],
    campo: fn(&mut Banderas, bool),
) -> Result<Valor, Fallo> {
    exigir_aridad(funcion, &argumentos[1..], 1)?;
    let (patron, mut banderas) = arg_expresion(funcion, argumentos, 0)?;
    let valor = match argumentos.get(1) {
        Some(Valor::Log(valor)) => *valor,
        Some(otro) => {
            return Err(error(
                "E0406",
                format!(
                    "'{funcion}' esperaba un valor log en el argumento 2, pero recibió '{}'",
                    otro.nombre_tipo()
                ),
            ));
        }
        None => {
            return Err(error(
                "E0210",
                format!("'{funcion}' necesita al menos 2 argumentos"),
            ));
        }
    };
    campo(&mut banderas, valor);
    instancia(funcion, &patron, banderas)
}

fn metodo_con_ignorar_mayusculas(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    con_bandera(
        "ExpresiónRegular.con_ignorar_mayúsculas",
        argumentos,
        |banderas, valor| banderas.ignorar_mayusculas = valor,
    )
}

fn metodo_con_multilinea(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    con_bandera(
        "ExpresiónRegular.con_multilínea",
        argumentos,
        |banderas, valor| banderas.multilinea = valor,
    )
}

fn metodo_con_punto_total(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    con_bandera(
        "ExpresiónRegular.con_punto_total",
        argumentos,
        |banderas, valor| banderas.punto_total = valor,
    )
}

fn metodo_con_unicode(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    con_bandera(
        "ExpresiónRegular.con_unicode",
        argumentos,
        |banderas, valor| banderas.unicode = valor,
    )
}

// ----- Registro -----

pub fn registrar(registro: &mut RegistroNativos) {
    registro.registrar_modulo("motor");

    // Constructor de `nuevo ExpresiónRegular(patron)`.
    registro.registrar_funcion("motor.constructor", Box::new(constructor));

    // Funciones libres (no necesitan instancia): `ExpresiónRegular.escapar(...)`, ...
    let libres: [(&str, Nativa); 7] = [
        ("escapar", libre_escapar),
        ("nueva_desde_comodín", libre_nueva_desde_comodin),
        ("nueva_desde_comodin", libre_nueva_desde_comodin),
        ("nueva_solo_dígitos", libre_nueva_solo_digitos),
        ("nueva_solo_digitos", libre_nueva_solo_digitos),
        ("nueva_solo_letras", libre_nueva_solo_letras),
        ("nueva_correo", libre_nueva_correo),
    ];
    for (nombre, funcion) in libres {
        registro.registrar_funcion(&format!("motor.{nombre}"), Box::new(funcion));
    }
    registro.registrar_funcion("motor.nueva_url", Box::new(libre_nueva_url));
    registro.registrar_funcion("motor.nueva_ipv4", Box::new(libre_nueva_ipv4));

    // Métodos de instancia: `expresion.coincide(...)` despacha como
    // `ExpresiónRegular.coincide` con el receptor como primer argumento.
    // Los nombres con tilde/ñ son los canónicos; se registra también la
    // variante sin tilde por comodidad (las claves se normalizan).
    let metodos: [(&str, Nativa); 26] = [
        ("coincide", metodo_coincide),
        ("coincide_desde_inicio", metodo_coincide_desde_inicio),
        ("coincide_completo", metodo_coincide_completo),
        ("buscar", metodo_buscar),
        ("buscar_posicion", metodo_buscar_posicion),
        ("buscar_todo", metodo_buscar_todo),
        ("buscar_grupos", metodo_buscar_grupos),
        ("contar", metodo_contar),
        ("reemplazar", metodo_reemplazar),
        ("reemplazar_todo", metodo_reemplazar_todo),
        ("dividir", metodo_dividir),
        ("es_válida", metodo_es_valida),
        ("es_valida", metodo_es_valida),
        ("diagnosticar", metodo_diagnosticar),
        ("patrón", metodo_patron),
        ("patron", metodo_patron),
        ("grupos_nombrados", metodo_grupos_nombrados),
        ("ignorar_mayúsculas", metodo_ignorar_mayusculas),
        ("ignorar_mayusculas", metodo_ignorar_mayusculas),
        ("multilínea", metodo_multilinea),
        ("multilinea", metodo_multilinea),
        ("punto_total", metodo_punto_total),
        ("unicode", metodo_unicode),
        ("con_ignorar_mayúsculas", metodo_con_ignorar_mayusculas),
        ("con_multilínea", metodo_con_multilinea),
        ("con_punto_total", metodo_con_punto_total),
    ];
    for (nombre, funcion) in metodos {
        registro.registrar_funcion(&format!("{TIPO}.{nombre}"), Box::new(funcion));
    }
    registro.registrar_funcion(
        &format!("{TIPO}.con_ignorar_mayusculas"),
        Box::new(metodo_con_ignorar_mayusculas),
    );
    registro.registrar_funcion(
        &format!("{TIPO}.con_multilinea"),
        Box::new(metodo_con_multilinea),
    );
    registro.registrar_funcion(&format!("{TIPO}.con_unicode"), Box::new(metodo_con_unicode));
}
