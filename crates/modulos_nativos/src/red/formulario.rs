//! Objetos `Formulario` y `ParteArchivo` de `quetzal/red`.
//!
//! `Formulario` es un formulario `multipart/form-data` (RFC 7578): campos de
//! texto y archivos que viajan juntos en una sola petición. Sirve para las
//! dos direcciones:
//!
//! - **Enviar**: se arma con `campo` y `archivo` y se pasa como `datos` a
//!   cualquier método del `ClienteHttp` que lleve cuerpo.
//! - **Recibir**: el `ServidorHttp` entrega uno ya analizado en
//!   `peticion.cuerpo()` cuando el `Content-Type` es multipart.
//!
//! Por el cable solo viaja `multipart/form-data` estándar, así que cualquier
//! cliente o servidor de otro lenguaje lo entiende sin saber nada de Quetzal.

use std::path::Path;
use std::rc::Rc;

use maquina_virtual::{Fallo, RegistroNativos, Valor, texto_de_valor};
use runtime::GuardianPermisos;

use crate::bits;
use crate::red::http;
use crate::red::objetos::{campo, campo_texto, error_permiso, error_tipo, instancia, receptor};
use crate::util::{arg_texto, error, exigir_aridad};

pub(crate) const TIPO_FORMULARIO: &str = "Formulario";
pub(crate) const TIPO_PARTE: &str = "ParteArchivo";

/// Tipo del objeto `Archivo` de `quetzal/sistema_archivos`.
const TIPO_ARCHIVO: &str = "Archivo";

// ----- Construcción de instancias -----

/// Formulario vacío, listo para agregarle campos y archivos.
pub(crate) fn instancia_formulario() -> Valor {
    let formulario = instancia(
        TIPO_FORMULARIO,
        vec![
            ("campos", Valor::lista(Vec::new())),
            ("archivos", Valor::lista(Vec::new())),
            ("texto", Valor::texto("<Formulario 0 campos, 0 archivos>")),
        ],
    );
    describir(&formulario);
    formulario
}

/// Una parte de archivo: nombre del campo, nombre del archivo, tipo y bytes.
pub(crate) fn instancia_parte(
    campo_form: &str,
    nombre: &str,
    tipo: &str,
    contenido: &[u8],
) -> Valor {
    instancia(
        TIPO_PARTE,
        vec![
            ("campo", Valor::texto(campo_form)),
            ("nombre", Valor::texto(nombre)),
            ("tipo", Valor::texto(tipo)),
            ("bits", bits::instancia(contenido)),
            (
                "texto",
                Valor::texto(format!(
                    "<ParteArchivo {campo_form}: {nombre} ({} bytes)>",
                    contenido.len()
                )),
            ),
        ],
    )
}

/// Si el valor es un `Formulario`.
pub(crate) fn es_formulario(valor: &Valor) -> bool {
    matches!(valor, Valor::InstanciaNativa(datos) if &*datos.tipo == TIPO_FORMULARIO)
}

/// Si el valor es un `Archivo` de `quetzal/sistema_archivos`.
pub(crate) fn es_archivo(valor: &Valor) -> bool {
    matches!(valor, Valor::InstanciaNativa(datos) if &*datos.tipo == TIPO_ARCHIVO)
}

/// Si el valor es un `Bits`.
pub(crate) fn es_bits(valor: &Valor) -> bool {
    matches!(valor, Valor::InstanciaNativa(datos) if &*datos.tipo == bits::TIPO)
}

/// Contenido de un `Archivo`: sus bytes, su nombre y el tipo MIME que le
/// corresponde por extensión. Leer del disco exige el permiso
/// `sistema-archivos` de lectura.
pub(crate) fn contenido_de_archivo(
    funcion: &str,
    guardian: &GuardianPermisos,
    valor: &Valor,
) -> Result<(Vec<u8>, String, String), Fallo> {
    let ruta = campo_texto(valor, "ruta");
    if ruta.is_empty() {
        return Err(error_tipo(format!(
            "'{funcion}' recibió un Archivo sin ruta válida"
        )));
    }
    let resuelta = guardian.verificar_lectura(&ruta).map_err(error_permiso)?;
    let bytes = std::fs::read(&resuelta).map_err(|fallo| {
        error(
            "E0407",
            format!("'{funcion}' no pudo leer '{}': {fallo}", resuelta.display()),
        )
    })?;
    let nombre = Path::new(&ruta)
        .file_name()
        .map(|nombre| nombre.to_string_lossy().to_string())
        .unwrap_or_else(|| ruta.clone());
    let extension = Path::new(&ruta)
        .extension()
        .map(|extension| extension.to_string_lossy().to_string())
        .unwrap_or_default();
    Ok((bytes, nombre, http::tipo_por_extension(&extension).to_string()))
}

// ----- Registro de las funciones nativas -----

pub(crate) fn registrar(registro: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    for tipo in [TIPO_FORMULARIO, TIPO_PARTE] {
        registro.registrar_modulo(tipo);
    }

    registro.registrar_funcion(
        &format!("{TIPO_FORMULARIO}.constructor"),
        Box::new(move |argumentos| {
            const F: &str = "Formulario";
            exigir_aridad(F, argumentos, 0)?;
            Ok(instancia_formulario())
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_FORMULARIO}.campo"),
        Box::new(move |argumentos| {
            const F: &str = "Formulario.campo";
            exigir_aridad(F, &argumentos[1..], 2)?;
            let formulario = receptor(F, argumentos, TIPO_FORMULARIO)?;
            let nombre = arg_texto(F, argumentos, 1)?.to_string();
            agregar_campo(&formulario, &nombre, &texto_de_valor(&argumentos[2]));
            Ok(argumentos[0].clone())
        }),
    );

    let guardian_archivo = Rc::clone(guardian);
    registro.registrar_funcion(
        &format!("{TIPO_FORMULARIO}.archivo"),
        Box::new(move |argumentos| {
            const F: &str = "Formulario.archivo";
            if argumentos.len() < 3 || argumentos.len() > 4 {
                return Err(error(
                    "E0210",
                    format!("'{F}' espera (campo, contenido) o (campo, contenido, opciones)"),
                ));
            }
            let formulario = receptor(F, argumentos, TIPO_FORMULARIO)?;
            let nombre_campo = arg_texto(F, argumentos, 1)?.to_string();
            let opciones = argumentos.get(3).cloned().unwrap_or(Valor::Nulo);

            let (contenido, nombre_por_omision, tipo_por_omision) = match &argumentos[2] {
                valor if es_archivo(valor) => {
                    contenido_de_archivo(F, &guardian_archivo, valor)?
                }
                valor if es_bits(valor) => (
                    bits::arg_bits(F, argumentos, 2)?,
                    format!("{nombre_campo}.bin"),
                    "application/octet-stream".to_string(),
                ),
                otro => {
                    return Err(error_tipo(format!(
                        "'{F}' esperaba un Archivo o un Bits, pero recibió '{}'",
                        otro.nombre_tipo()
                    )));
                }
            };

            let nombre = opcion_texto(&opciones, "nombre").unwrap_or(nombre_por_omision);
            let tipo = opcion_texto(&opciones, "tipo")
                .map(|tipo| http::tipo_mime(&tipo))
                .unwrap_or(tipo_por_omision);

            agregar_archivo(
                &formulario,
                instancia_parte(&nombre_campo, &nombre, &tipo, &contenido),
            );
            Ok(argumentos[0].clone())
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_FORMULARIO}.campo_texto"),
        Box::new(move |argumentos| {
            const F: &str = "Formulario.campo_texto";
            exigir_aridad(F, &argumentos[1..], 1)?;
            let formulario = receptor(F, argumentos, TIPO_FORMULARIO)?;
            let nombre = arg_texto(F, argumentos, 1)?;
            Ok(buscar_campo(&formulario, nombre).unwrap_or(Valor::Nulo))
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_FORMULARIO}.tiene"),
        Box::new(move |argumentos| {
            const F: &str = "Formulario.tiene";
            exigir_aridad(F, &argumentos[1..], 1)?;
            let formulario = receptor(F, argumentos, TIPO_FORMULARIO)?;
            let nombre = arg_texto(F, argumentos, 1)?;
            Ok(Valor::Log(
                buscar_campo(&formulario, nombre).is_some()
                    || buscar_parte(&formulario, nombre).is_some(),
            ))
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_FORMULARIO}.campos"),
        Box::new(move |argumentos| {
            const F: &str = "Formulario.campos";
            exigir_aridad(F, &argumentos[1..], 0)?;
            let formulario = receptor(F, argumentos, TIPO_FORMULARIO)?;
            let mut mapa = indexmap::IndexMap::new();
            for (nombre, valor) in campos_de(&formulario) {
                mapa.insert(nombre, Valor::texto(valor));
            }
            Ok(Valor::jsn(mapa))
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_FORMULARIO}.archivo_parte"),
        Box::new(move |argumentos| {
            const F: &str = "Formulario.archivo_parte";
            exigir_aridad(F, &argumentos[1..], 1)?;
            let formulario = receptor(F, argumentos, TIPO_FORMULARIO)?;
            let nombre = arg_texto(F, argumentos, 1)?;
            Ok(buscar_parte(&formulario, nombre).unwrap_or(Valor::Nulo))
        }),
    );

    registro.registrar_funcion(
        &format!("{TIPO_FORMULARIO}.archivos"),
        Box::new(move |argumentos| {
            const F: &str = "Formulario.archivos";
            exigir_aridad(F, &argumentos[1..], 0)?;
            let formulario = receptor(F, argumentos, TIPO_FORMULARIO)?;
            Ok(campo(&formulario, "archivos"))
        }),
    );

    for metodo in ["campo", "nombre", "tipo", "bits"] {
        let funcion = format!("{TIPO_PARTE}.{metodo}");
        let nombre_campo = metodo;
        registro.registrar_funcion(
            &funcion.clone(),
            Box::new(move |argumentos| {
                exigir_aridad(&funcion, &argumentos[1..], 0)?;
                let parte = receptor(&funcion, argumentos, TIPO_PARTE)?;
                Ok(campo(&parte, nombre_campo))
            }),
        );
    }
}

fn opcion_texto(opciones: &Valor, clave: &str) -> Option<String> {
    match opciones {
        Valor::Jsn(mapa) => mapa.borrow().get(clave).and_then(|valor| match valor {
            Valor::Nulo => None,
            otro => Some(texto_de_valor(otro)),
        }),
        _ => None,
    }
}

// ----- Acceso al contenido del formulario -----

fn agregar_campo(formulario: &Valor, nombre: &str, valor: &str) {
    if let Valor::Lista(lista) = campo(formulario, "campos") {
        lista.borrow_mut().push(Valor::lista(vec![
            Valor::texto(nombre),
            Valor::texto(valor),
        ]));
    }
    describir(formulario);
}

fn agregar_archivo(formulario: &Valor, parte: Valor) {
    if let Valor::Lista(lista) = campo(formulario, "archivos") {
        lista.borrow_mut().push(parte);
    }
    describir(formulario);
}

/// Campos de texto en orden de inserción (admite nombres repetidos).
pub(crate) fn campos_de(formulario: &Valor) -> Vec<(String, String)> {
    let Valor::Lista(lista) = campo(formulario, "campos") else {
        return Vec::new();
    };
    let campos = lista.borrow();
    campos
        .iter()
        .filter_map(|par| match par {
            Valor::Lista(par) => {
                let par = par.borrow();
                match (par.first(), par.get(1)) {
                    (Some(nombre), Some(valor)) => {
                        Some((texto_de_valor(nombre), texto_de_valor(valor)))
                    }
                    _ => None,
                }
            }
            _ => None,
        })
        .collect()
}

/// Partes de archivo en orden de inserción.
pub(crate) fn partes_de(formulario: &Valor) -> Vec<Valor> {
    match campo(formulario, "archivos") {
        Valor::Lista(lista) => lista.borrow().clone(),
        _ => Vec::new(),
    }
}

fn buscar_campo(formulario: &Valor, nombre: &str) -> Option<Valor> {
    campos_de(formulario)
        .into_iter()
        .find(|(clave, _)| clave == nombre)
        .map(|(_, valor)| Valor::texto(valor))
}

fn buscar_parte(formulario: &Valor, nombre: &str) -> Option<Valor> {
    partes_de(formulario)
        .into_iter()
        .find(|parte| campo_texto(parte, "campo") == nombre)
}

fn describir(formulario: &Valor) {
    let campos = campos_de(formulario).len();
    let archivos = partes_de(formulario).len();
    crate::red::objetos::poner_campo(
        formulario,
        "texto",
        Valor::texto(format!(
            "<Formulario {campos} campos, {archivos} archivos>"
        )),
    );
}

// ----- Serialización multipart (lo que envía el cliente) -----

/// Frontera irrepetible para separar las partes del cuerpo.
pub(crate) fn nueva_frontera() -> String {
    use rand::Rng;
    let mut generador = rand::rng();
    let sufijo: String = (0..24)
        .map(|_| {
            const ALFABETO: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
            ALFABETO[generador.random_range(0..ALFABETO.len())] as char
        })
        .collect();
    format!("----QuetzalFormulario{sufijo}")
}

/// Serializa el formulario como `multipart/form-data` (RFC 7578) y devuelve
/// la frontera usada junto con el cuerpo.
pub(crate) fn cuerpo_multipart(formulario: &Valor) -> Result<(String, Vec<u8>), Fallo> {
    let frontera = nueva_frontera();
    let mut cuerpo = Vec::new();

    for (nombre, valor) in campos_de(formulario) {
        cuerpo.extend_from_slice(format!("--{frontera}\r\n").as_bytes());
        cuerpo.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"{}\"\r\n\r\n",
                escapar(&nombre)
            )
            .as_bytes(),
        );
        cuerpo.extend_from_slice(valor.as_bytes());
        cuerpo.extend_from_slice(b"\r\n");
    }

    for parte in partes_de(formulario) {
        let nombre_campo = campo_texto(&parte, "campo");
        let nombre_archivo = campo_texto(&parte, "nombre");
        let tipo = campo_texto(&parte, "tipo");
        let contenido = bits::arg_bits(
            "Formulario",
            std::slice::from_ref(&campo(&parte, "bits")),
            0,
        )?;

        cuerpo.extend_from_slice(format!("--{frontera}\r\n").as_bytes());
        cuerpo.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
                escapar(&nombre_campo),
                escapar(&nombre_archivo)
            )
            .as_bytes(),
        );
        cuerpo.extend_from_slice(format!("Content-Type: {tipo}\r\n\r\n").as_bytes());
        cuerpo.extend_from_slice(&contenido);
        cuerpo.extend_from_slice(b"\r\n");
    }

    cuerpo.extend_from_slice(format!("--{frontera}--\r\n").as_bytes());
    Ok((frontera, cuerpo))
}

/// Escapa comillas y barras en un valor de parámetro de cabecera.
fn escapar(texto: &str) -> String {
    texto.replace('\\', "\\\\").replace('"', "\\\"")
}

// ----- Análisis multipart (lo que recibe el servidor) -----

/// Frontera declarada en un `Content-Type: multipart/form-data; boundary=...`.
pub(crate) fn frontera_de_tipo(tipo_contenido: &str) -> Option<String> {
    if !tipo_contenido.to_ascii_lowercase().contains("multipart/") {
        return None;
    }
    for parametro in tipo_contenido.split(';').skip(1) {
        let (clave, valor) = parametro.split_once('=')?;
        if clave.trim().eq_ignore_ascii_case("boundary") {
            return Some(valor.trim().trim_matches('"').to_string());
        }
    }
    None
}

/// Convierte un cuerpo `multipart/form-data` en un `Formulario`.
pub(crate) fn desde_multipart(cuerpo: &[u8], frontera: &str) -> Valor {
    let formulario = instancia_formulario();
    let separador = format!("--{frontera}");

    for cruda in trozos(cuerpo, separador.as_bytes()) {
        let Some((cabeceras, contenido)) = separar_cabeceras(cruda) else {
            continue;
        };
        let disposicion = valor_cabecera(&cabeceras, "content-disposition").unwrap_or_default();
        let Some(nombre_campo) = parametro(&disposicion, "name") else {
            continue;
        };
        match parametro(&disposicion, "filename") {
            Some(nombre_archivo) => {
                let tipo = valor_cabecera(&cabeceras, "content-type")
                    .unwrap_or_else(|| "application/octet-stream".to_string());
                agregar_archivo(
                    &formulario,
                    instancia_parte(&nombre_campo, &nombre_archivo, &tipo, contenido),
                );
            }
            None => {
                let valor = String::from_utf8_lossy(contenido).to_string();
                agregar_campo(&formulario, &nombre_campo, &valor);
            }
        }
    }

    formulario
}

/// Divide el cuerpo en las partes delimitadas por la frontera, ya sin el
/// salto de línea que abre y cierra cada una.
fn trozos<'a>(cuerpo: &'a [u8], separador: &[u8]) -> Vec<&'a [u8]> {
    let mut inicios = Vec::new();
    let mut indice = 0;
    while indice + separador.len() <= cuerpo.len() {
        if &cuerpo[indice..indice + separador.len()] == separador {
            inicios.push(indice);
            indice += separador.len();
        } else {
            indice += 1;
        }
    }

    let mut partes = Vec::new();
    for (posicion, inicio) in inicios.iter().enumerate() {
        let desde = inicio + separador.len();
        // `--` justo después de la frontera marca el final del cuerpo.
        if cuerpo[desde..].starts_with(b"--") {
            break;
        }
        let hasta = inicios.get(posicion + 1).copied().unwrap_or(cuerpo.len());
        let trozo = &cuerpo[desde..hasta];
        partes.push(recortar(trozo));
    }
    partes
}

/// Quita el CRLF que abre la parte y el que la cierra antes de la frontera.
fn recortar(trozo: &[u8]) -> &[u8] {
    let inicio = if trozo.starts_with(b"\r\n") { 2 } else { 0 };
    let mut fin = trozo.len();
    if trozo[inicio..fin].ends_with(b"\r\n") {
        fin -= 2;
    }
    &trozo[inicio..fin]
}

fn separar_cabeceras(parte: &[u8]) -> Option<(String, &[u8])> {
    let corte = parte
        .windows(4)
        .position(|ventana| ventana == b"\r\n\r\n")?;
    let cabeceras = String::from_utf8_lossy(&parte[..corte]).to_string();
    Some((cabeceras, &parte[corte + 4..]))
}

fn valor_cabecera(cabeceras: &str, nombre: &str) -> Option<String> {
    cabeceras.lines().find_map(|linea| {
        let (clave, valor) = linea.split_once(':')?;
        clave
            .trim()
            .eq_ignore_ascii_case(nombre)
            .then(|| valor.trim().to_string())
    })
}

/// Nombre de archivo declarado en un `Content-Disposition`.
pub(crate) fn nombre_en_disposicion(cabecera: &str) -> Option<String> {
    parametro(cabecera, "filename")
}

/// Lee un parámetro entre comillas de una cabecera (`name="archivo"`).
fn parametro(cabecera: &str, nombre: &str) -> Option<String> {
    for parametro in cabecera.split(';').skip(1) {
        let Some((clave, valor)) = parametro.split_once('=') else {
            continue;
        };
        if clave.trim().eq_ignore_ascii_case(nombre) {
            let valor = valor.trim().trim_matches('"');
            return Some(valor.replace("\\\"", "\"").replace("\\\\", "\\"));
        }
    }
    None
}
