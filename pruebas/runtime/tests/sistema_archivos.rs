//! Módulo nativo `quetzal/sistema_archivos`: los objetos `SistemaArchivos`,
//! `Archivo` y `Bits`.
//!
//! Cubre los criterios de aceptación de la especificación: importación de los
//! tres objetos, funciones libres síncronas y asíncronas, listados con
//! nombres relativos, metadatos y operaciones de `Archivo`, datos binarios
//! con `Bits`, y el modelo de permisos de `quetzal.json` (niveles `lectura`,
//! `escritura` y `todo`, rutas fuera de lo declarado y excepciones
//! capturables con `intentar` / `capturar`).

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

use indexmap::IndexMap;
use maquina_virtual::{Valor, Vm};
use nucleo::Fuente;
use paquetes::Permisos;
use runtime::GuardianPermisos;

/// Cada prueba trabaja en su propio directorio temporal: los proyectos de
/// Quetzal resuelven las rutas relativas contra la raíz del proyecto.
fn raiz_temporal(nombre: &str) -> PathBuf {
    static CONTADOR: AtomicU32 = AtomicU32::new(0);
    let unico = CONTADOR.fetch_add(1, Ordering::Relaxed);
    let ruta = std::env::temp_dir().join(format!(
        "quetzal_sisarch_{nombre}_{}_{unico}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&ruta);
    std::fs::create_dir_all(&ruta).expect("se puede crear la raíz temporal");
    ruta
}

/// Permiso `todo` sobre la raíz del proyecto (el caso más común).
fn permisos_totales() -> &'static str {
    r#"{"sistema_archivos": {"habilitado": true,
        "directorios": [{"ruta": "./", "permiso": "todo"}]}}"#
}

fn ejecutar_en(
    raiz: &Path,
    permisos_json: &str,
    codigo: &str,
) -> Rc<maquina_virtual::valores::EntornoModulo> {
    let fuente = Fuente::nueva("prueba.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("el código de prueba debe parsear");
    semantica::analizar_modulo(&ast).expect("el código de prueba debe pasar la semántica");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar bytecode");

    let valor: serde_json::Value =
        serde_json::from_str(permisos_json).expect("permisos de prueba en JSON válido");
    let guardian = Rc::new(GuardianPermisos::denegado());
    guardian.configurar(
        Permisos::desde_json(&valor).expect("permisos de prueba válidos"),
        raiz,
    );

    // Resolución mínima de imports nativos (el motor hace esto en producción).
    let mut importaciones = IndexMap::new();
    for importacion in &modulo.importaciones {
        if let Some(nombre_modulo) = importacion.origen.strip_prefix("quetzal/") {
            let modulo_normalizado =
                maquina_virtual::normalizar_nombre(nombre_modulo).to_lowercase();
            for (nombre, local) in &importacion.simbolos {
                let simbolo = maquina_virtual::normalizar_nombre(nombre)
                    .to_lowercase()
                    .replace('_', "");
                let destino =
                    modulos_nativos::modulo_de_tipo_exportado(&modulo_normalizado, &simbolo)
                        .map(str::to_string)
                        .unwrap_or_else(|| modulo_normalizado.clone());
                importaciones.insert(
                    local.clone(),
                    maquina_virtual::Variable {
                        valor: Valor::ModuloNativo(Rc::from(destino.as_str())),
                        mutable: false,
                    },
                );
            }
        }
    }

    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro_con_permisos(
        &guardian,
    )));
    let (entorno, _valor) = vm
        .cargar_modulo(Rc::new(modulo), importaciones)
        .expect("el código de prueba debe ejecutar sin errores");
    entorno
}

fn texto_global(entorno: &maquina_virtual::valores::EntornoModulo, nombre: &str) -> String {
    match &entorno
        .globales
        .borrow()
        .get(nombre)
        .unwrap_or_else(|| panic!("debe existir '{nombre}'"))
        .valor
    {
        Valor::Texto(texto) => texto.to_string(),
        otro => maquina_virtual::texto_de_valor(otro),
    }
}

const IMPORTAR: &str =
    "importar { SistemaArchivos, Archivo, Bits } desde \"quetzal/sistema_archivos\"\n";

// CA-01: la importación expone los tres objetos del módulo.
#[test]
fn deberia_exponer_sistema_archivos_archivo_y_bits() {
    let raiz = raiz_temporal("importacion");
    let entorno = ejecutar_en(
        &raiz,
        permisos_totales(),
        &format!(
            "{IMPORTAR}\
             SistemaArchivos.escribir(\"./nota.txt\", \"hola\")\n\
             Archivo nota = SistemaArchivos.abrir(\"./nota.txt\")\n\
             Bits datos = Bits.desde_texto(\"hola\")\n\
             texto nombre = nota.nombre()\n\
             entero bytes = datos.longitud()\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "nombre"), "nota.txt");
    assert_eq!(texto_global(&entorno, "bytes"), "4");
    let _ = std::fs::remove_dir_all(&raiz);
}

// CA-06: escribir crea o sobrescribe y leer devuelve el texto UTF-8.
#[test]
fn escribir_y_leer_deberia_sobrescribir_el_contenido() {
    let raiz = raiz_temporal("escribir_leer");
    let entorno = ejecutar_en(
        &raiz,
        permisos_totales(),
        &format!(
            "{IMPORTAR}\
             SistemaArchivos.escribir(\"./datos.txt\", \"uno\")\n\
             SistemaArchivos.escribir(\"./datos.txt\", \"dos\")\n\
             texto contenido = SistemaArchivos.leer(\"./datos.txt\")\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "contenido"), "dos");
    assert_eq!(
        std::fs::read_to_string(raiz.join("datos.txt")).unwrap(),
        "dos"
    );
    let _ = std::fs::remove_dir_all(&raiz);
}

// CA-04: los listados devuelven lista<texto> con nombres relativos.
#[test]
fn listados_deberian_devolver_solo_nombres_relativos() {
    let raiz = raiz_temporal("listados");
    let entorno = ejecutar_en(
        &raiz,
        permisos_totales(),
        &format!(
            "{IMPORTAR}\
             SistemaArchivos.crear_directorios(\"./reportes/2026\")\n\
             SistemaArchivos.escribir(\"./reportes/a.txt\", \"a\")\n\
             SistemaArchivos.escribir(\"./reportes/b.txt\", \"b\")\n\
             lista<texto> archivos = SistemaArchivos.listar_archivos(\"./reportes\")\n\
             lista<texto> directorios = SistemaArchivos.listar_directorios(\"./reportes\")\n\
             texto nombres = archivos.unir(\",\")\n\
             texto carpetas = directorios.unir(\",\")\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "nombres"), "a.txt,b.txt");
    assert_eq!(texto_global(&entorno, "carpetas"), "2026");
    let _ = std::fs::remove_dir_all(&raiz);
}

// CA-07: crear_directorios es recursivo e idempotente; crear_directorio no.
#[test]
fn crear_directorio_simple_deberia_fallar_si_ya_existe() {
    let raiz = raiz_temporal("crear");
    let entorno = ejecutar_en(
        &raiz,
        permisos_totales(),
        &format!(
            "{IMPORTAR}\
             texto var estado = \"sin error\"\n\
             SistemaArchivos.crear_directorios(\"./uno/dos\")\n\
             SistemaArchivos.crear_directorios(\"./uno/dos\")\n\
             intentar {{\n\
                 SistemaArchivos.crear_directorio(\"./uno\")\n\
             }} capturar (excepcion e) {{\n\
                 estado = \"falló\"\n\
             }}\n\
             log existe = SistemaArchivos.es_directorio(\"./uno/dos\")\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "estado"), "falló");
    assert_eq!(texto_global(&entorno, "existe"), "verdadero");
    let _ = std::fs::remove_dir_all(&raiz);
}

// CA-05: Archivo ofrece metadatos y operaciones sobre su propia ruta.
#[test]
fn archivo_deberia_exponer_metadatos_y_operaciones() {
    let raiz = raiz_temporal("archivo");
    let entorno = ejecutar_en(
        &raiz,
        permisos_totales(),
        &format!(
            "{IMPORTAR}\
             SistemaArchivos.escribir(\"./informe.txt\", \"12345\")\n\
             Archivo informe = SistemaArchivos.abrir(\"./informe.txt\")\n\
             texto nombre = informe.nombre()\n\
             texto base = informe.nombre_sin_extension()\n\
             texto extension = informe.extension()\n\
             entero tamaño = informe.tamaño()\n\
             log es_archivo = informe.es_archivo()\n\
             texto año = informe.fecha_modificacion().texto_fecha()\n\
             informe.escribir(\"nuevo contenido\")\n\
             texto contenido = informe.leer()\n\
             informe.copiar(\"./copia.txt\")\n\
             log hay_copia = SistemaArchivos.existe(\"./copia.txt\")\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "nombre"), "informe.txt");
    assert_eq!(texto_global(&entorno, "base"), "informe");
    assert_eq!(texto_global(&entorno, "extension"), "txt");
    assert_eq!(texto_global(&entorno, "tamaño"), "5");
    assert_eq!(texto_global(&entorno, "es_archivo"), "verdadero");
    assert_eq!(texto_global(&entorno, "contenido"), "nuevo contenido");
    assert_eq!(texto_global(&entorno, "hay_copia"), "verdadero");
    assert_eq!(
        texto_global(&entorno, "año").len(),
        10,
        "la fecha de modificación debe ser una instancia de Tiempo"
    );
    let _ = std::fs::remove_dir_all(&raiz);
}

// Renombrar y mover reapuntan la instancia a su nueva ruta.
#[test]
fn renombrar_y_mover_deberian_actualizar_la_ruta_del_archivo() {
    let raiz = raiz_temporal("renombrar");
    let entorno = ejecutar_en(
        &raiz,
        permisos_totales(),
        &format!(
            "{IMPORTAR}\
             SistemaArchivos.crear_directorios(\"./archivo_final\")\n\
             SistemaArchivos.escribir(\"./borrador.txt\", \"texto\")\n\
             Archivo documento = SistemaArchivos.abrir(\"./borrador.txt\")\n\
             documento.renombrar(\"definitivo.txt\")\n\
             texto tras_renombrar = documento.nombre()\n\
             documento.mover(\"./archivo_final/definitivo.txt\")\n\
             texto carpeta = documento.directorio_padre()\n\
             log en_destino = SistemaArchivos.existe(\"./archivo_final/definitivo.txt\")\n\
             log queda_original = SistemaArchivos.existe(\"./borrador.txt\")\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "tras_renombrar"), "definitivo.txt");
    assert!(texto_global(&entorno, "carpeta").ends_with("archivo_final"));
    assert_eq!(texto_global(&entorno, "en_destino"), "verdadero");
    assert_eq!(texto_global(&entorno, "queda_original"), "falso");
    let _ = std::fs::remove_dir_all(&raiz);
}

// Borrar exige un directorio vacío; borrar_recursivo elimina el árbol.
#[test]
fn borrar_deberia_distinguir_directorio_vacio_de_arbol_completo() {
    let raiz = raiz_temporal("borrar");
    let entorno = ejecutar_en(
        &raiz,
        permisos_totales(),
        &format!(
            "{IMPORTAR}\
             texto var estado = \"sin error\"\n\
             SistemaArchivos.crear_directorios(\"./arbol/rama\")\n\
             SistemaArchivos.escribir(\"./arbol/rama/hoja.txt\", \"hoja\")\n\
             intentar {{\n\
                 SistemaArchivos.borrar(\"./arbol\")\n\
             }} capturar (excepcion e) {{\n\
                 estado = e.mensaje\n\
             }}\n\
             SistemaArchivos.borrar_recursivo(\"./arbol\")\n\
             log queda = SistemaArchivos.existe(\"./arbol\")\n"
        ),
    );
    assert!(
        texto_global(&entorno, "estado").contains("borrar"),
        "borrar un directorio con contenido debe fallar con un mensaje claro"
    );
    assert_eq!(texto_global(&entorno, "queda"), "falso");
    let _ = std::fs::remove_dir_all(&raiz);
}

// CA-06, CA-09: Bits viaja a disco y vuelve sin perder bytes.
#[test]
fn bits_deberia_ir_y_volver_del_disco_sin_perder_bytes() {
    let raiz = raiz_temporal("bits");
    let entorno = ejecutar_en(
        &raiz,
        permisos_totales(),
        &format!(
            "{IMPORTAR}\
             Bits firma = Bits.desde_lista([137, 80, 78, 71])\n\
             SistemaArchivos.escribir_bits(\"./marca.bin\", firma)\n\
             Bits leidos = SistemaArchivos.leer_bits(\"./marca.bin\")\n\
             entero longitud = leidos.longitud()\n\
             log iguales = leidos.es_igual(firma)\n\
             texto var estado = \"sin error\"\n\
             intentar {{\n\
                 texto invalido = leidos.texto()\n\
             }} capturar (excepcion e) {{\n\
                 estado = \"no es UTF-8\"\n\
             }}\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "longitud"), "4");
    assert_eq!(texto_global(&entorno, "iguales"), "verdadero");
    assert_eq!(texto_global(&entorno, "estado"), "no es UTF-8");
    assert_eq!(
        std::fs::read(raiz.join("marca.bin")).unwrap(),
        vec![137, 80, 78, 71]
    );
    let _ = std::fs::remove_dir_all(&raiz);
}

// CA-02: cada operación tiene su forma asíncrona resuelta con esperar.
#[test]
fn las_formas_asincronas_deberian_resolverse_con_esperar() {
    let raiz = raiz_temporal("asincrono");
    let entorno = ejecutar_en(
        &raiz,
        permisos_totales(),
        &format!(
            "{IMPORTAR}\
             esperar SistemaArchivos.crear_directorios_asincrono(\"./salida\")\n\
             esperar SistemaArchivos.escribir_asincrono(\"./salida/datos.txt\", \"asíncrono\")\n\
             texto contenido = esperar SistemaArchivos.leer_asincrono(\"./salida/datos.txt\")\n\
             log existe = esperar SistemaArchivos.existe_asincrono(\"./salida/datos.txt\")\n\
             Archivo datos = esperar SistemaArchivos.abrir_asincrono(\"./salida/datos.txt\")\n\
             texto nombre = datos.nombre()\n\
             lista<texto> archivos = esperar \
             SistemaArchivos.listar_archivos_asincrono(\"./salida\")\n\
             texto nombres = archivos.unir(\",\")\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "contenido"), "asíncrono");
    assert_eq!(texto_global(&entorno, "existe"), "verdadero");
    assert_eq!(texto_global(&entorno, "nombre"), "datos.txt");
    assert_eq!(texto_global(&entorno, "nombres"), "datos.txt");
    let _ = std::fs::remove_dir_all(&raiz);
}

// CA-02: el objeto Archivo también tiene formas asíncronas.
#[test]
fn archivo_deberia_tener_formas_asincronas() {
    let raiz = raiz_temporal("archivo_asincrono");
    let entorno = ejecutar_en(
        &raiz,
        permisos_totales(),
        &format!(
            "{IMPORTAR}\
             SistemaArchivos.escribir(\"./nota.txt\", \"inicial\")\n\
             Archivo nota = SistemaArchivos.abrir(\"./nota.txt\")\n\
             esperar nota.escribir_asincrono(\"cambiado\")\n\
             texto contenido = esperar nota.leer_asincrono()\n\
             esperar nota.copiar_asincrono(\"./respaldo.txt\")\n\
             log hay_respaldo = SistemaArchivos.existe(\"./respaldo.txt\")\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "contenido"), "cambiado");
    assert_eq!(texto_global(&entorno, "hay_respaldo"), "verdadero");
    let _ = std::fs::remove_dir_all(&raiz);
}

// CA-08: sin el permiso habilitado no se puede tocar el disco.
#[test]
fn sin_permiso_habilitado_deberia_lanzar_excepcion_capturable() {
    let raiz = raiz_temporal("sin_permiso");
    let entorno = ejecutar_en(
        &raiz,
        r#"{"sistema_archivos": {"habilitado": false}}"#,
        &format!(
            "{IMPORTAR}\
             texto var mensaje = \"\"\n\
             intentar {{\n\
                 SistemaArchivos.escribir(\"./nota.txt\", \"hola\")\n\
             }} capturar (excepcion e) {{\n\
                 mensaje = e.mensaje\n\
             }}\n"
        ),
    );
    let mensaje = texto_global(&entorno, "mensaje");
    assert!(
        mensaje.contains("no tiene permiso de sistema de archivos")
            && mensaje.contains("quetzal.json"),
        "el mensaje debe orientar a corregir quetzal.json, se obtuvo: {mensaje}"
    );
    assert!(!raiz.join("nota.txt").exists());
    let _ = std::fs::remove_dir_all(&raiz);
}

// CA-07: lectura no autoriza escritura y escritura no autoriza lectura.
#[test]
fn los_niveles_de_permiso_deberian_respetarse_por_directorio() {
    let raiz = raiz_temporal("niveles");
    std::fs::create_dir_all(raiz.join("datos")).unwrap();
    std::fs::create_dir_all(raiz.join("salida")).unwrap();
    std::fs::write(raiz.join("datos/entrada.txt"), "entrada").unwrap();
    std::fs::write(raiz.join("salida/previo.txt"), "previo").unwrap();

    let entorno = ejecutar_en(
        &raiz,
        r#"{"sistema_archivos": {"habilitado": true, "directorios": [
            {"ruta": "./datos", "permiso": "lectura"},
            {"ruta": "./salida", "permiso": "escritura"}]}}"#,
        &format!(
            "{IMPORTAR}\
             texto entrada = SistemaArchivos.leer(\"./datos/entrada.txt\")\n\
             SistemaArchivos.escribir(\"./salida/informe.txt\", \"listo\")\n\
             texto var sin_escritura = \"\"\n\
             texto var sin_lectura = \"\"\n\
             intentar {{\n\
                 SistemaArchivos.escribir(\"./datos/nuevo.txt\", \"no\")\n\
             }} capturar (excepcion e) {{\n\
                 sin_escritura = e.mensaje\n\
             }}\n\
             intentar {{\n\
                 texto previo = SistemaArchivos.leer(\"./salida/previo.txt\")\n\
             }} capturar (excepcion e) {{\n\
                 sin_lectura = e.mensaje\n\
             }}\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "entrada"), "entrada");
    assert!(raiz.join("salida/informe.txt").exists());
    assert!(
        texto_global(&entorno, "sin_escritura").contains("no tiene permiso de escritura"),
        "el nivel lectura no debe permitir escribir"
    );
    assert!(
        texto_global(&entorno, "sin_lectura").contains("no tiene permiso de lectura"),
        "el nivel escritura no debe permitir leer"
    );
    assert!(!raiz.join("datos/nuevo.txt").exists());
    let _ = std::fs::remove_dir_all(&raiz);
}

// CA-07, CA-08: una ruta fuera de los directorios declarados se rechaza.
#[test]
fn una_ruta_fuera_de_lo_declarado_deberia_rechazarse() {
    let raiz = raiz_temporal("fuera");
    std::fs::create_dir_all(raiz.join("trabajo")).unwrap();

    let entorno = ejecutar_en(
        &raiz,
        r#"{"sistema_archivos": {"habilitado": true,
            "directorios": [{"ruta": "./trabajo", "permiso": "todo"}]}}"#,
        &format!(
            "{IMPORTAR}\
             texto var mensaje = \"\"\n\
             intentar {{\n\
                 texto secreto = SistemaArchivos.leer(\"./trabajo/../secreto.txt\")\n\
             }} capturar (excepcion e) {{\n\
                 mensaje = e.mensaje\n\
             }}\n\
             SistemaArchivos.escribir(\"./trabajo/permitido.txt\", \"ok\")\n"
        ),
    );
    assert!(
        texto_global(&entorno, "mensaje").contains("fuera de los directorios permitidos"),
        "salir del directorio con '..' debe rechazarse"
    );
    assert!(raiz.join("trabajo/permitido.txt").exists());
    let _ = std::fs::remove_dir_all(&raiz);
}

// CA-09: Bits en memoria funciona sin permisos de sistema de archivos.
#[test]
fn bits_en_memoria_no_deberia_requerir_permisos() {
    let raiz = raiz_temporal("bits_memoria");
    let entorno = ejecutar_en(
        &raiz,
        r#"{"sistema_archivos": {"habilitado": false}}"#,
        &format!(
            "{IMPORTAR}\
             Bits saludo = Bits.desde_texto(\"café\")\n\
             Bits unido = saludo.concatenar(Bits.desde_lista([33]))\n\
             entero longitud = unido.longitud()\n\
             texto decodificado = unido.texto()\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "longitud"), "6");
    assert_eq!(texto_global(&entorno, "decodificado"), "café!");
    let _ = std::fs::remove_dir_all(&raiz);
}
