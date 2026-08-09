//! Códigos estables de error del Lenguaje Quetzal.
//!
//! Convención de rangos:
//! - `E00xx`: errores léxicos
//! - `E01xx`: errores sintácticos
//! - `E02xx`: errores semánticos
//! - `E03xx`: errores de módulos e imports
//! - `E04xx`: errores de runtime
//! - `E05xx`: errores de permisos
//! - `E06xx`: errores de paquetes
//! - `E9999`: error interno

// Léxico
pub const CARACTER_INESPERADO: &str = "E0001";
pub const TEXTO_SIN_CERRAR: &str = "E0002";
pub const NUMERO_INVALIDO: &str = "E0003";
pub const COMENTARIO_SIN_CERRAR: &str = "E0004";

// Sintaxis
pub const TOKEN_INESPERADO: &str = "E0101";
pub const SE_ESPERABA_EXPRESION: &str = "E0102";
pub const BLOQUE_SIN_CERRAR: &str = "E0103";
pub const DECLARACION_INVALIDA: &str = "E0104";

// Semántica
pub const VARIABLE_NO_DEFINIDA: &str = "E0201";
pub const VARIABLE_DUPLICADA: &str = "E0202";
pub const REASIGNACION_CONSTANTE: &str = "E0203";
pub const TIPOS_INCOMPATIBLES: &str = "E0204";
pub const RETORNO_INVALIDO: &str = "E0205";
pub const TIPO_VACIO_INVALIDO: &str = "E0206";
pub const MIEMBRO_PRIVADO: &str = "E0207";
pub const PROTOTIPO_INCOMPLETO: &str = "E0208";
pub const FUNCION_NO_DEFINIDA: &str = "E0209";
pub const ARGUMENTOS_INVALIDOS: &str = "E0210";
/// `esperar` usado dentro de una función síncrona (solo es válido en
/// funciones `asincrono` o a nivel de scope global).
pub const ESPERAR_FUERA_DE_ASINCRONO: &str = "E0213";

// Módulos
pub const MODULO_NO_ENCONTRADO: &str = "E0301";
pub const EXPORTACION_INEXISTENTE: &str = "E0302";
pub const IMPORTACION_CIRCULAR: &str = "E0303";

// Runtime
pub const DIVISION_POR_CERO: &str = "E0401";
pub const DESBORDAMIENTO_ENTERO: &str = "E0402";
pub const INDICE_FUERA_DE_RANGO: &str = "E0403";
pub const VALOR_NULO: &str = "E0404";
pub const EXCEPCION_NO_CAPTURADA: &str = "E0405";
pub const CONVERSION_INVALIDA: &str = "E0406";
/// Fallo del sistema de archivos (no existe, sin espacio, sin acceso del SO).
pub const ERROR_ENTRADA_SALIDA: &str = "E0407";
/// Fallo de red: conexión rechazada, tiempo agotado, respuesta HTTP inválida
/// o estado HTTP fuera del rango aceptado.
pub const ERROR_RED: &str = "E0408";

// Permisos
pub const PERMISO_DENEGADO: &str = "E0501";

// Paquetes
pub const QUETZAL_JSON_INVALIDO: &str = "E0601";
pub const DEPENDENCIA_NO_ENCONTRADA: &str = "E0602";

// Interno
pub const ERROR_INTERNO: &str = "E9999";
