//! Módulo nativo `quetzal/tiempo`: el objeto `Tiempo`.
//!
//! Al importar `{ Tiempo }` desde `"quetzal/tiempo"` se expone un objeto que
//! puede instanciarse (`nuevo Tiempo(...)`) y que también ofrece funciones
//! libres (`Tiempo.ahora()`, `Tiempo.zonas()`, ...). Cada instancia es
//! inmutable: guarda una marca Unix en milisegundos y una zona horaria
//! (`"local"`, `"UTC"` o un nombre IANA como `"America/Guatemala"`), y toda
//! operación aritmética o de zona devuelve una instancia nueva.

use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;

use chrono::{
    DateTime, Datelike, FixedOffset, Local, Months, NaiveDate, NaiveDateTime, SecondsFormat,
    TimeZone, Timelike,
};
use chrono_tz::Tz;
use indexmap::IndexMap;
use maquina_virtual::{DatosInstanciaNativa, Fallo, RegistroNativos, Valor};

use crate::util::{arg_entero, arg_texto, error, exigir_aridad};

const TIPO: &str = "Tiempo";

/// Función nativa de este módulo, registrable bajo varios nombres.
type Nativa = fn(&[Valor]) -> Result<Valor, Fallo>;

const NOMBRES_DIAS: [&str; 7] = [
    "lunes",
    "martes",
    "miércoles",
    "jueves",
    "viernes",
    "sábado",
    "domingo",
];

const NOMBRES_MESES: [&str; 12] = [
    "enero",
    "febrero",
    "marzo",
    "abril",
    "mayo",
    "junio",
    "julio",
    "agosto",
    "septiembre",
    "octubre",
    "noviembre",
    "diciembre",
];

// ----- Zona horaria interna -----

/// Zona horaria de una instancia: la del sistema o una IANA (incluye UTC).
enum Zona {
    Local,
    Nombrada(Tz),
}

fn zona_de_texto(funcion: &str, nombre: &str) -> Result<Zona, Fallo> {
    if nombre.eq_ignore_ascii_case("local") {
        return Ok(Zona::Local);
    }
    Tz::from_str(nombre).map(Zona::Nombrada).map_err(|_| {
        error(
            "E0406",
            format!(
                "'{funcion}' no reconoce la zona horaria '{nombre}'; usa un nombre IANA como 'America/Guatemala', 'UTC' o 'local'"
            ),
        )
    })
}

/// Fecha con desfase fijo a partir de una marca (ms) y su zona.
fn fecha_con_desfase(
    funcion: &str,
    marca: i64,
    zona: &Zona,
) -> Result<DateTime<FixedOffset>, Fallo> {
    let fecha = match zona {
        Zona::Local => Local
            .timestamp_millis_opt(marca)
            .single()
            .map(|fecha| fecha.fixed_offset()),
        Zona::Nombrada(tz) => tz
            .timestamp_millis_opt(marca)
            .single()
            .map(|fecha| fecha.fixed_offset()),
    };
    fecha.ok_or_else(|| {
        error(
            "E0406",
            format!("'{funcion}' recibió una marca de tiempo fuera de rango ({marca})"),
        )
    })
}

/// Convierte una fecha/hora "de pared" a marca Unix dentro de una zona.
fn marca_de_naive(funcion: &str, naive: NaiveDateTime, zona: &Zona) -> Result<i64, Fallo> {
    let marca = match zona {
        Zona::Local => Local
            .from_local_datetime(&naive)
            .earliest()
            .map(|fecha| fecha.timestamp_millis()),
        Zona::Nombrada(tz) => tz
            .from_local_datetime(&naive)
            .earliest()
            .map(|fecha| fecha.timestamp_millis()),
    };
    marca.ok_or_else(|| {
        error(
            "E0406",
            format!("'{funcion}' recibió una fecha y hora que no existe en la zona indicada"),
        )
    })
}

// ----- Instancias -----

/// Crea una instancia de `Tiempo` con su texto ISO 8601 precalculado.
fn instancia(funcion: &str, marca: i64, nombre_zona: &str) -> Result<Valor, Fallo> {
    let zona = zona_de_texto(funcion, nombre_zona)?;
    let fecha = fecha_con_desfase(funcion, marca, &zona)?;
    let mut datos = IndexMap::new();
    datos.insert("marca".to_string(), Valor::Entero(marca));
    datos.insert("zona".to_string(), Valor::texto(nombre_zona));
    datos.insert(
        "texto".to_string(),
        Valor::texto(fecha.to_rfc3339_opts(SecondsFormat::Secs, false)),
    );
    Ok(Valor::InstanciaNativa(Rc::new(DatosInstanciaNativa {
        tipo: Rc::from(TIPO),
        datos: RefCell::new(datos),
    })))
}

/// Instancia de `Tiempo` en la zona local desde una marca Unix en ms; la usa
/// el objeto `Archivo` para sus fechas de creación, modificación y acceso.
pub(crate) fn instancia_local(funcion: &str, marca: i64) -> Result<Valor, Fallo> {
    instancia(funcion, marca, "local")
}

/// Extrae marca y zona de una instancia de `Tiempo` en la posición dada.
fn arg_tiempo(funcion: &str, argumentos: &[Valor], indice: usize) -> Result<(i64, String), Fallo> {
    match argumentos.get(indice) {
        Some(Valor::InstanciaNativa(datos)) if &*datos.tipo == TIPO => {
            let datos = datos.datos.borrow();
            let marca = match datos.get("marca") {
                Some(Valor::Entero(marca)) => *marca,
                _ => {
                    return Err(error(
                        "E0406",
                        format!("'{funcion}' recibió un Tiempo sin marca interna"),
                    ));
                }
            };
            let zona = match datos.get("zona") {
                Some(Valor::Texto(zona)) => zona.to_string(),
                _ => "local".to_string(),
            };
            Ok((marca, zona))
        }
        Some(otro) => Err(error(
            "E0406",
            format!(
                "'{funcion}' esperaba una instancia de Tiempo en el argumento {}, pero recibió '{}'",
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

/// Receptor (`esto`) de un método: marca, zona y fecha con desfase.
fn receptor(
    funcion: &str,
    argumentos: &[Valor],
) -> Result<(i64, String, DateTime<FixedOffset>), Fallo> {
    let (marca, nombre_zona) = arg_tiempo(funcion, argumentos, 0)?;
    let zona = zona_de_texto(funcion, &nombre_zona)?;
    let fecha = fecha_con_desfase(funcion, marca, &zona)?;
    Ok((marca, nombre_zona, fecha))
}

// ----- Helpers de componentes -----

fn componente_i32(funcion: &str, valor: i64, campo: &str) -> Result<i32, Fallo> {
    i32::try_from(valor).map_err(|_| {
        error(
            "E0406",
            format!("'{funcion}' recibió un valor fuera de rango para '{campo}' ({valor})"),
        )
    })
}

fn componente_u32(funcion: &str, valor: i64, campo: &str) -> Result<u32, Fallo> {
    u32::try_from(valor).map_err(|_| {
        error(
            "E0406",
            format!("'{funcion}' recibió un valor fuera de rango para '{campo}' ({valor})"),
        )
    })
}

#[allow(clippy::too_many_arguments)]
fn naive_de_componentes(
    funcion: &str,
    año: i64,
    mes: i64,
    dia: i64,
    hora: i64,
    minuto: i64,
    segundo: i64,
) -> Result<NaiveDateTime, Fallo> {
    let fecha = NaiveDate::from_ymd_opt(
        componente_i32(funcion, año, "año")?,
        componente_u32(funcion, mes, "mes")?,
        componente_u32(funcion, dia, "día")?,
    )
    .ok_or_else(|| {
        error(
            "E0406",
            format!("'{funcion}' recibió una fecha inválida ({año:04}-{mes:02}-{dia:02})"),
        )
    })?;
    fecha
        .and_hms_opt(
            componente_u32(funcion, hora, "hora")?,
            componente_u32(funcion, minuto, "minuto")?,
            componente_u32(funcion, segundo, "segundo")?,
        )
        .ok_or_else(|| {
            error(
                "E0406",
                format!(
                    "'{funcion}' recibió una hora inválida ({hora:02}:{minuto:02}:{segundo:02})"
                ),
            )
        })
}

/// Suma meses (o años convertidos a meses) respetando la zona de la instancia.
fn sumar_meses(funcion: &str, marca: i64, nombre_zona: &str, meses: i64) -> Result<Valor, Fallo> {
    fn ajustar<T: TimeZone>(tz: &T, marca: i64, meses: i64) -> Option<i64> {
        let fecha = tz.timestamp_millis_opt(marca).single()?;
        let cantidad = Months::new(u32::try_from(meses.unsigned_abs()).ok()?);
        let nueva = if meses >= 0 {
            fecha.checked_add_months(cantidad)?
        } else {
            fecha.checked_sub_months(cantidad)?
        };
        Some(nueva.timestamp_millis())
    }
    let zona = zona_de_texto(funcion, nombre_zona)?;
    let nueva_marca = match &zona {
        Zona::Local => ajustar(&Local, marca, meses),
        Zona::Nombrada(tz) => ajustar(tz, marca, meses),
    }
    .ok_or_else(|| {
        error(
            "E0406",
            format!("'{funcion}' produjo una fecha fuera de rango"),
        )
    })?;
    instancia(funcion, nueva_marca, nombre_zona)
}

/// Suma una cantidad fija de milisegundos (días, horas, minutos, segundos).
fn sumar_ms(funcion: &str, argumentos: &[Valor], ms_por_unidad: i64) -> Result<Valor, Fallo> {
    exigir_aridad(funcion, &argumentos[1..], 1)?;
    let (marca, zona) = arg_tiempo(funcion, argumentos, 0)?;
    let cantidad = arg_entero(funcion, argumentos, 1)?;
    let nueva = cantidad
        .checked_mul(ms_por_unidad)
        .and_then(|delta| marca.checked_add(delta))
        .ok_or_else(|| {
            error(
                "E0406",
                format!("'{funcion}' produjo una fecha fuera de rango"),
            )
        })?;
    instancia(funcion, nueva, &zona)
}

fn marca_actual() -> i64 {
    Local::now().timestamp_millis()
}

fn zona_local_sistema() -> String {
    iana_time_zone::get_timezone().unwrap_or_else(|_| "local".to_string())
}

// ----- Constructor: `nuevo Tiempo(...)` -----

fn constructor(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "nuevo Tiempo";
    match argumentos.len() {
        // `nuevo Tiempo()`: instante actual en la zona local.
        0 => instancia(F, marca_actual(), "local"),
        // `nuevo Tiempo(marca_ms)` o `nuevo Tiempo("2026-06-11 14:30:00")`.
        1 => match &argumentos[0] {
            Valor::Entero(marca) => instancia(F, *marca, "local"),
            Valor::Texto(texto) => analizar_texto_comun(F, texto),
            otro => Err(error(
                "E0406",
                format!(
                    "'{F}' espera una marca (entero) o un texto de fecha, pero recibió '{}'",
                    otro.nombre_tipo()
                ),
            )),
        },
        // `nuevo Tiempo(año, mes, dia[, hora, minuto, segundo[, zona]])`.
        3 | 6 | 7 => {
            let año = arg_entero(F, argumentos, 0)?;
            let mes = arg_entero(F, argumentos, 1)?;
            let dia = arg_entero(F, argumentos, 2)?;
            let (hora, minuto, segundo) = if argumentos.len() >= 6 {
                (
                    arg_entero(F, argumentos, 3)?,
                    arg_entero(F, argumentos, 4)?,
                    arg_entero(F, argumentos, 5)?,
                )
            } else {
                (0, 0, 0)
            };
            let nombre_zona = if argumentos.len() == 7 {
                arg_texto(F, argumentos, 6)?.to_string()
            } else {
                "local".to_string()
            };
            let naive = naive_de_componentes(F, año, mes, dia, hora, minuto, segundo)?;
            let zona = zona_de_texto(F, &nombre_zona)?;
            let marca = marca_de_naive(F, naive, &zona)?;
            instancia(F, marca, &nombre_zona)
        }
        otros => Err(error(
            "E0210",
            format!("'{F}' acepta 0, 1, 3, 6 o 7 argumentos, pero recibió {otros}"),
        )),
    }
}

/// Interpreta textos en formatos comunes: ISO 8601 / RFC 3339, fecha y hora
/// separadas por espacio, o solo fecha.
fn analizar_texto_comun(funcion: &str, texto: &str) -> Result<Valor, Fallo> {
    if let Ok(fecha) = DateTime::parse_from_rfc3339(texto) {
        return instancia(funcion, fecha.timestamp_millis(), "local");
    }
    for patron in [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%d/%m/%Y %H:%M:%S",
    ] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(texto, patron) {
            let marca = marca_de_naive(funcion, naive, &Zona::Local)?;
            return instancia(funcion, marca, "local");
        }
    }
    for patron in ["%Y-%m-%d", "%d/%m/%Y"] {
        if let Ok(fecha) = NaiveDate::parse_from_str(texto, patron) {
            let naive = fecha
                .and_hms_opt(0, 0, 0)
                .expect("medianoche siempre es válida");
            let marca = marca_de_naive(funcion, naive, &Zona::Local)?;
            return instancia(funcion, marca, "local");
        }
    }
    Err(error(
        "E0406",
        format!(
            "'{funcion}' no pudo interpretar '{texto}' como fecha; usa 'AAAA-MM-DD', 'AAAA-MM-DD HH:MM:SS' o ISO 8601"
        ),
    ))
}

// ----- Funciones libres: `Tiempo.x()` -----

fn libre_ahora(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    exigir_aridad("Tiempo.ahora", argumentos, 0)?;
    instancia("Tiempo.ahora", marca_actual(), "local")
}

fn libre_ahora_utc(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    exigir_aridad("Tiempo.ahora_utc", argumentos, 0)?;
    instancia("Tiempo.ahora_utc", marca_actual(), "UTC")
}

fn libre_ahora_en(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Tiempo.ahora_en";
    exigir_aridad(F, argumentos, 1)?;
    let zona = arg_texto(F, argumentos, 0)?;
    instancia(F, marca_actual(), zona)
}

fn libre_hoy(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Tiempo.hoy";
    exigir_aridad(F, argumentos, 0)?;
    let medianoche = Local::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("medianoche siempre es válida");
    let marca = marca_de_naive(F, medianoche, &Zona::Local)?;
    instancia(F, marca, "local")
}

fn libre_marca(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    exigir_aridad("Tiempo.marca", argumentos, 0)?;
    Ok(Valor::Entero(marca_actual()))
}

fn libre_desde_marca(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Tiempo.desde_marca";
    exigir_aridad(F, argumentos, 1)?;
    let marca = arg_entero(F, argumentos, 0)?;
    instancia(F, marca, "local")
}

fn libre_analizar(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Tiempo.analizar";
    exigir_aridad(F, argumentos, 2)?;
    let texto = arg_texto(F, argumentos, 0)?;
    let patron = arg_texto(F, argumentos, 1)?;
    if let Ok(fecha) = DateTime::parse_from_str(texto, patron) {
        return instancia(F, fecha.timestamp_millis(), "local");
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(texto, patron) {
        let marca = marca_de_naive(F, naive, &Zona::Local)?;
        return instancia(F, marca, "local");
    }
    if let Ok(fecha) = NaiveDate::parse_from_str(texto, patron) {
        let naive = fecha
            .and_hms_opt(0, 0, 0)
            .expect("medianoche siempre es válida");
        let marca = marca_de_naive(F, naive, &Zona::Local)?;
        return instancia(F, marca, "local");
    }
    Err(error(
        "E0406",
        format!("'{F}' no pudo interpretar '{texto}' con el patrón '{patron}'"),
    ))
}

fn libre_zona_local(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    exigir_aridad("Tiempo.zona_local", argumentos, 0)?;
    Ok(Valor::texto(zona_local_sistema()))
}

fn libre_zonas(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Tiempo.zonas";
    let filtro = match argumentos.len() {
        0 => None,
        1 => Some(arg_texto(F, argumentos, 0)?.to_lowercase()),
        otros => {
            return Err(error(
                "E0210",
                format!("'{F}' acepta 0 o 1 argumentos, pero recibió {otros}"),
            ));
        }
    };
    let zonas: Vec<Valor> = chrono_tz::TZ_VARIANTS
        .iter()
        .map(|zona| zona.name())
        .filter(|nombre| match &filtro {
            Some(region) => nombre
                .split('/')
                .next()
                .map(|prefijo| prefijo.to_lowercase() == *region)
                .unwrap_or(false),
            None => true,
        })
        .map(Valor::texto)
        .collect();
    Ok(Valor::lista(zonas))
}

fn libre_es_bisiesto(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Tiempo.es_bisiesto";
    exigir_aridad(F, argumentos, 1)?;
    let año = arg_entero(F, argumentos, 0)?;
    let año = componente_i32(F, año, "año")?;
    Ok(Valor::Log(NaiveDate::from_ymd_opt(año, 2, 29).is_some()))
}

fn libre_dias_en_mes(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Tiempo.dias_en_mes";
    exigir_aridad(F, argumentos, 2)?;
    let año = componente_i32(F, arg_entero(F, argumentos, 0)?, "año")?;
    let mes = componente_u32(F, arg_entero(F, argumentos, 1)?, "mes")?;
    let inicio = NaiveDate::from_ymd_opt(año, mes, 1)
        .ok_or_else(|| error("E0406", format!("'{F}' recibió un mes inválido ({mes})")))?;
    let siguiente = if mes == 12 {
        NaiveDate::from_ymd_opt(año + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(año, mes + 1, 1)
    }
    .ok_or_else(|| error("E0406", format!("'{F}' produjo una fecha fuera de rango")))?;
    Ok(Valor::Entero(
        siguiente.signed_duration_since(inicio).num_days(),
    ))
}

// ----- Métodos de instancia: componentes -----

fn metodo_año(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    let (_, _, fecha) = receptor("Tiempo.año", argumentos)?;
    Ok(Valor::Entero(i64::from(fecha.year())))
}

fn metodo_mes(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    let (_, _, fecha) = receptor("Tiempo.mes", argumentos)?;
    Ok(Valor::Entero(i64::from(fecha.month())))
}

fn metodo_dia(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    let (_, _, fecha) = receptor("Tiempo.dia", argumentos)?;
    Ok(Valor::Entero(i64::from(fecha.day())))
}

fn metodo_hora(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    let (_, _, fecha) = receptor("Tiempo.hora", argumentos)?;
    Ok(Valor::Entero(i64::from(fecha.hour())))
}

fn metodo_minuto(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    let (_, _, fecha) = receptor("Tiempo.minuto", argumentos)?;
    Ok(Valor::Entero(i64::from(fecha.minute())))
}

fn metodo_segundo(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    let (_, _, fecha) = receptor("Tiempo.segundo", argumentos)?;
    Ok(Valor::Entero(i64::from(fecha.second())))
}

fn metodo_milisegundo(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    let (_, _, fecha) = receptor("Tiempo.milisegundo", argumentos)?;
    Ok(Valor::Entero(i64::from(fecha.timestamp_subsec_millis())))
}

fn metodo_dia_semana(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    let (_, _, fecha) = receptor("Tiempo.dia_semana", argumentos)?;
    Ok(Valor::Entero(i64::from(
        fecha.weekday().number_from_monday(),
    )))
}

fn metodo_dia_año(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    let (_, _, fecha) = receptor("Tiempo.dia_año", argumentos)?;
    Ok(Valor::Entero(i64::from(fecha.ordinal())))
}

fn metodo_marca(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    let (marca, _) = arg_tiempo("Tiempo.marca", argumentos, 0)?;
    Ok(Valor::Entero(marca))
}

// ----- Métodos de instancia: texto -----

fn metodo_texto(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    let (_, _, fecha) = receptor("Tiempo.texto", argumentos)?;
    Ok(Valor::texto(
        fecha.to_rfc3339_opts(SecondsFormat::Secs, false),
    ))
}

fn metodo_texto_fecha(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    let (_, _, fecha) = receptor("Tiempo.texto_fecha", argumentos)?;
    Ok(Valor::texto(fecha.format("%Y-%m-%d").to_string()))
}

fn metodo_texto_hora(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    let (_, _, fecha) = receptor("Tiempo.texto_hora", argumentos)?;
    Ok(Valor::texto(fecha.format("%H:%M:%S").to_string()))
}

fn metodo_formatear(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Tiempo.formatear";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let (_, _, fecha) = receptor(F, argumentos)?;
    let patron = arg_texto(F, argumentos, 1)?;
    Ok(Valor::texto(fecha.format(patron).to_string()))
}

fn metodo_nombre_dia(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    let (_, _, fecha) = receptor("Tiempo.nombre_dia", argumentos)?;
    let indice = fecha.weekday().num_days_from_monday() as usize;
    Ok(Valor::texto(NOMBRES_DIAS[indice]))
}

fn metodo_nombre_mes(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    let (_, _, fecha) = receptor("Tiempo.nombre_mes", argumentos)?;
    let indice = (fecha.month() - 1) as usize;
    Ok(Valor::texto(NOMBRES_MESES[indice]))
}

// ----- Métodos de instancia: aritmética -----

fn metodo_agregar_años(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Tiempo.agregar_años";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let (marca, zona) = arg_tiempo(F, argumentos, 0)?;
    let años = arg_entero(F, argumentos, 1)?;
    let meses = años
        .checked_mul(12)
        .ok_or_else(|| error("E0406", format!("'{F}' produjo una fecha fuera de rango")))?;
    sumar_meses(F, marca, &zona, meses)
}

fn metodo_agregar_meses(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Tiempo.agregar_meses";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let (marca, zona) = arg_tiempo(F, argumentos, 0)?;
    let meses = arg_entero(F, argumentos, 1)?;
    sumar_meses(F, marca, &zona, meses)
}

fn metodo_agregar_dias(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    sumar_ms("Tiempo.agregar_dias", argumentos, 86_400_000)
}

fn metodo_agregar_horas(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    sumar_ms("Tiempo.agregar_horas", argumentos, 3_600_000)
}

fn metodo_agregar_minutos(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    sumar_ms("Tiempo.agregar_minutos", argumentos, 60_000)
}

fn metodo_agregar_segundos(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    sumar_ms("Tiempo.agregar_segundos", argumentos, 1_000)
}

/// `inicio.diferencia(fin)`: totales con signo (positivos si `fin` es después).
fn metodo_diferencia(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Tiempo.diferencia";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let (marca, _) = arg_tiempo(F, argumentos, 0)?;
    let (otra, _) = arg_tiempo(F, argumentos, 1)?;
    let delta = otra - marca;
    let mut mapa = IndexMap::new();
    mapa.insert("dias".to_string(), Valor::Entero(delta / 86_400_000));
    mapa.insert("horas".to_string(), Valor::Entero(delta / 3_600_000));
    mapa.insert("minutos".to_string(), Valor::Entero(delta / 60_000));
    mapa.insert("segundos".to_string(), Valor::Entero(delta / 1_000));
    mapa.insert("milisegundos".to_string(), Valor::Entero(delta));
    Ok(Valor::jsn(mapa))
}

// ----- Métodos de instancia: comparación -----

fn metodo_es_antes(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Tiempo.es_antes";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let (marca, _) = arg_tiempo(F, argumentos, 0)?;
    let (otra, _) = arg_tiempo(F, argumentos, 1)?;
    Ok(Valor::Log(marca < otra))
}

fn metodo_es_despues(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Tiempo.es_despues";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let (marca, _) = arg_tiempo(F, argumentos, 0)?;
    let (otra, _) = arg_tiempo(F, argumentos, 1)?;
    Ok(Valor::Log(marca > otra))
}

fn metodo_es_mismo_instante(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Tiempo.es_mismo_instante";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let (marca, _) = arg_tiempo(F, argumentos, 0)?;
    let (otra, _) = arg_tiempo(F, argumentos, 1)?;
    Ok(Valor::Log(marca == otra))
}

/// Compara las fechas de ambos instantes vistas en la zona del receptor.
fn metodo_es_mismo_dia(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Tiempo.es_mismo_dia";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let (marca, nombre_zona) = arg_tiempo(F, argumentos, 0)?;
    let (otra, _) = arg_tiempo(F, argumentos, 1)?;
    let zona = zona_de_texto(F, &nombre_zona)?;
    let fecha = fecha_con_desfase(F, marca, &zona)?;
    let otra_fecha = fecha_con_desfase(F, otra, &zona)?;
    Ok(Valor::Log(fecha.date_naive() == otra_fecha.date_naive()))
}

// ----- Métodos de instancia: zonas horarias -----

fn metodo_zona(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    let (_, zona) = arg_tiempo("Tiempo.zona", argumentos, 0)?;
    Ok(Valor::texto(zona))
}

fn metodo_desfase(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    let (_, _, fecha) = receptor("Tiempo.desfase", argumentos)?;
    Ok(Valor::texto(fecha.format("%:z").to_string()))
}

fn metodo_en_zona(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    const F: &str = "Tiempo.en_zona";
    exigir_aridad(F, &argumentos[1..], 1)?;
    let (marca, _) = arg_tiempo(F, argumentos, 0)?;
    let zona = arg_texto(F, argumentos, 1)?;
    instancia(F, marca, zona)
}

fn metodo_en_utc(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    let (marca, _) = arg_tiempo("Tiempo.en_utc", argumentos, 0)?;
    instancia("Tiempo.en_utc", marca, "UTC")
}

fn metodo_en_local(argumentos: &[Valor]) -> Result<Valor, Fallo> {
    let (marca, _) = arg_tiempo("Tiempo.en_local", argumentos, 0)?;
    instancia("Tiempo.en_local", marca, "local")
}

// ----- Registro -----

pub fn registrar(registro: &mut RegistroNativos) {
    registro.registrar_modulo("tiempo");

    // Constructor de `nuevo Tiempo(...)`.
    registro.registrar_funcion("tiempo.constructor", Box::new(constructor));

    // Funciones libres (no necesitan instancia): `Tiempo.ahora()`, ...
    let libres: [(&str, Nativa); 11] = [
        ("ahora", libre_ahora),
        ("ahora_utc", libre_ahora_utc),
        ("ahora_en", libre_ahora_en),
        ("hoy", libre_hoy),
        ("marca", libre_marca),
        ("desde_marca", libre_desde_marca),
        ("analizar", libre_analizar),
        ("zona_local", libre_zona_local),
        ("zonas", libre_zonas),
        ("es_bisiesto", libre_es_bisiesto),
        ("dias_en_mes", libre_dias_en_mes),
    ];
    for (nombre, funcion) in libres {
        registro.registrar_funcion(&format!("tiempo.{nombre}"), Box::new(funcion));
    }

    // Métodos de instancia: `instante.metodo(...)` despacha como `Tiempo.metodo`
    // con el receptor como primer argumento. Los nombres con ñ son los
    // canónicos; se registra también la variante sin ñ por comodidad.
    let metodos: [(&str, Nativa); 33] = [
        ("año", metodo_año),
        ("anio", metodo_año),
        ("mes", metodo_mes),
        ("dia", metodo_dia),
        ("hora", metodo_hora),
        ("minuto", metodo_minuto),
        ("segundo", metodo_segundo),
        ("milisegundo", metodo_milisegundo),
        ("dia_semana", metodo_dia_semana),
        ("dia_año", metodo_dia_año),
        ("dia_anio", metodo_dia_año),
        ("marca", metodo_marca),
        ("texto", metodo_texto),
        ("texto_fecha", metodo_texto_fecha),
        ("texto_hora", metodo_texto_hora),
        ("formatear", metodo_formatear),
        ("nombre_dia", metodo_nombre_dia),
        ("nombre_mes", metodo_nombre_mes),
        ("agregar_años", metodo_agregar_años),
        ("agregar_anios", metodo_agregar_años),
        ("agregar_meses", metodo_agregar_meses),
        ("agregar_dias", metodo_agregar_dias),
        ("agregar_horas", metodo_agregar_horas),
        ("agregar_minutos", metodo_agregar_minutos),
        ("agregar_segundos", metodo_agregar_segundos),
        ("diferencia", metodo_diferencia),
        ("es_antes", metodo_es_antes),
        ("es_despues", metodo_es_despues),
        ("es_mismo_instante", metodo_es_mismo_instante),
        ("es_mismo_dia", metodo_es_mismo_dia),
        ("zona", metodo_zona),
        ("desfase", metodo_desfase),
        ("en_zona", metodo_en_zona),
    ];
    for (nombre, funcion) in metodos {
        registro.registrar_funcion(&format!("{TIPO}.{nombre}"), Box::new(funcion));
    }
    registro.registrar_funcion(&format!("{TIPO}.en_utc"), Box::new(metodo_en_utc));
    registro.registrar_funcion(&format!("{TIPO}.en_local"), Box::new(metodo_en_local));
}
