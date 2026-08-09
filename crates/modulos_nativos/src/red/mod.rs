//! Módulo nativo `quetzal/red`.
//!
//! Expone la API de red del Lenguaje Quetzal, toda en español:
//!
//! - **Servidor** (`ServidorHttp`, `Enrutador`, `Ruta`, `PeticionEntrante`,
//!   `RespuestaSaliente`, `Continuacion`): el equivalente de Express 5, con
//!   interceptores y manejadores declarados como funciones nombradas.
//! - **Cliente** (`ClienteHttp`, `RespuestaHttp`, `ProgresoPeticion`): el
//!   equivalente de Axios, con interceptores y progreso de subida y descarga.
//! - **Formularios** (`Formulario`, `ParteArchivo`): armado y lectura de
//!   `multipart/form-data` (RFC 7578) para transmitir campos y archivos en
//!   la misma petición.
//! - **Códigos** (`HttpCodigos`): descripciones en español de los estados
//!   HTTP según la referencia de MDN.
//!
//! Los métodos HTTP se escriben en español (`obtener`, `publicar`, `poner`,
//! `parchear`, `borrar`, `cabecera`, `opciones`, `consultar`) y viajan por el
//! cable como los verbos estándar; `consultar` es el método QUERY del
//! RFC 10008.
//!
//! Toda operación de red pasa antes por el [`GuardianPermisos`]: hace falta
//! el permiso `red` habilitado en `quetzal.json`.

pub(crate) mod cliente;
pub(crate) mod codigos;
pub(crate) mod formulario;
pub(crate) mod http;
pub(crate) mod metodos;
pub(crate) mod objetos;
pub(crate) mod rutas;
pub(crate) mod servidor;

use std::rc::Rc;

use maquina_virtual::RegistroNativos;
use runtime::GuardianPermisos;

/// Registra el módulo `red` con sus objetos de servidor, cliente y códigos.
pub fn registrar(registro: &mut RegistroNativos, guardian: &Rc<GuardianPermisos>) {
    registro.registrar_modulo("red");
    codigos::registrar(registro);
    formulario::registrar(registro, guardian);
    servidor::registrar(registro, guardian);
    cliente::registrar(registro, guardian);
}

/// Tipos que `quetzal/red` exporta además del módulo, para que
/// `importar { ServidorHttp } desde "quetzal/red"` resuelva al objeto nativo.
/// Recibe el símbolo ya normalizado (minúsculas, sin tildes ni guiones bajos).
pub fn tipo_exportado(simbolo_normalizado: &str) -> Option<&'static str> {
    Some(match simbolo_normalizado {
        "servidorhttp" => servidor::TIPO_SERVIDOR,
        "enrutador" => servidor::TIPO_ENRUTADOR,
        "ruta" => servidor::TIPO_RUTA,
        "peticionentrante" => servidor::TIPO_PETICION,
        "respuestasaliente" => servidor::TIPO_RESPUESTA,
        "continuacion" => servidor::TIPO_CONTINUACION,
        "errorhttp" => servidor::TIPO_ERROR,
        "clientehttp" => cliente::TIPO_CLIENTE,
        "respuestahttp" => cliente::TIPO_RESPUESTA,
        "progresopeticion" => cliente::TIPO_PROGRESO,
        "httpcodigos" => codigos::TIPO,
        "formulario" => formulario::TIPO_FORMULARIO,
        "partearchivo" => formulario::TIPO_PARTE,
        _ => return None,
    })
}
