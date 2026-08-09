//! Módulo nativo `quetzal/sistema_archivos`: `anexar`, el objeto `Flujo`
//! (posición persistente) y el objeto `Observador` con `EventoArchivo`.
//!
//! Cubre los criterios de la especificación: anexar sin perder contenido,
//! cursor que avanza con cada lectura o escritura, flujos cerrados que ya no
//! operan, permisos por modo de apertura, y el ciclo del observador
//! (`iniciar` / `esperar_evento` / `detener`) con errores capturables.

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
        "quetzal_flujos_{nombre}_{}_{unico}",
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

const IMPORTAR: &str = "importar { SistemaArchivos, Flujo, Observador, EventoArchivo, Bits } \
                        desde \"quetzal/sistema_archivos\"\n";

// anexar crea el archivo y concatena sin borrar lo anterior.
#[test]
fn anexar_deberia_crear_y_concatenar_sin_borrar() {
    let raiz = raiz_temporal("anexar");
    let entorno = ejecutar_en(
        &raiz,
        permisos_totales(),
        &format!(
            "{IMPORTAR}\
             SistemaArchivos.anexar(\"./bitacora.log\", \"uno\\n\")\n\
             SistemaArchivos.anexar(\"./bitacora.log\", \"dos\\n\")\n\
             esperar SistemaArchivos.anexar_asincrono(\"./bitacora.log\", \"tres\\n\")\n\
             texto contenido = SistemaArchivos.leer(\"./bitacora.log\")\n\
             SistemaArchivos.anexar_bits(\"./firma.bin\", Bits.desde_lista([1, 2]))\n\
             SistemaArchivos.anexar_bits(\"./firma.bin\", Bits.desde_lista([3]))\n\
             entero bytes = SistemaArchivos.leer_bits(\"./firma.bin\").longitud()\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "contenido"), "uno\ndos\ntres\n");
    assert_eq!(texto_global(&entorno, "bytes"), "3");
    assert_eq!(
        std::fs::read(raiz.join("firma.bin")).unwrap(),
        vec![1, 2, 3]
    );
    let _ = std::fs::remove_dir_all(&raiz);
}

// El cursor del flujo avanza con cada operación y se puede reposicionar.
#[test]
fn el_cursor_del_flujo_deberia_avanzar_y_reposicionarse() {
    let raiz = raiz_temporal("cursor");
    let entorno = ejecutar_en(
        &raiz,
        permisos_totales(),
        &format!(
            "{IMPORTAR}\
             Flujo salida = SistemaArchivos.abrir_flujo(\"./datos.txt\", \"escritura\")\n\
             entero escritos = salida.escribir(\"abcdefghij\")\n\
             entero fin = salida.posicion()\n\
             salida.cerrar()\n\
             Flujo lector = SistemaArchivos.abrir_flujo(\"./datos.txt\", \"lectura\")\n\
             texto primeros = lector.leer(3)\n\
             entero tras_leer = lector.posicion()\n\
             lector.ir_a(5)\n\
             texto desde_cinco = lector.leer(2)\n\
             lector.retroceder(2)\n\
             entero retrocedido = lector.posicion()\n\
             lector.ir_al_final()\n\
             entero total = lector.posicion()\n\
             lector.ir_al_inicio()\n\
             lector.avanzar(8)\n\
             texto ultimos = lector.leer_todo()\n\
             entero longitud = lector.longitud()\n\
             texto modo = lector.modo()\n\
             lector.cerrar()\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "escritos"), "10");
    assert_eq!(texto_global(&entorno, "fin"), "10");
    assert_eq!(texto_global(&entorno, "primeros"), "abc");
    assert_eq!(texto_global(&entorno, "tras_leer"), "3");
    assert_eq!(texto_global(&entorno, "desde_cinco"), "fg");
    assert_eq!(texto_global(&entorno, "retrocedido"), "5");
    assert_eq!(texto_global(&entorno, "total"), "10");
    assert_eq!(texto_global(&entorno, "ultimos"), "ij");
    assert_eq!(texto_global(&entorno, "longitud"), "10");
    assert_eq!(texto_global(&entorno, "modo"), "lectura");
    let _ = std::fs::remove_dir_all(&raiz);
}

// El modo lectura_escritura conserva el contenido y edita en el lugar; el
// modo anexar siempre escribe al final.
#[test]
fn los_modos_deberian_conservar_o_anexar_segun_corresponda() {
    let raiz = raiz_temporal("modos");
    let entorno = ejecutar_en(
        &raiz,
        permisos_totales(),
        &format!(
            "{IMPORTAR}\
             SistemaArchivos.escribir(\"./texto.txt\", \"aaaaa\")\n\
             Flujo editor = SistemaArchivos.abrir_flujo(\"./texto.txt\", \"lectura_escritura\")\n\
             editor.ir_a(2)\n\
             editor.escribir(\"bb\")\n\
             editor.cerrar()\n\
             texto editado = SistemaArchivos.leer(\"./texto.txt\")\n\
             Flujo cola = SistemaArchivos.abrir_flujo(\"./texto.txt\", \"anexar\")\n\
             cola.ir_al_inicio()\n\
             cola.escribir(\"ZZ\")\n\
             cola.cerrar()\n\
             texto anexado = SistemaArchivos.leer(\"./texto.txt\")\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "editado"), "aabba");
    assert_eq!(
        texto_global(&entorno, "anexado"),
        "aabbaZZ",
        "en modo anexar la escritura va al final aunque el cursor se mueva"
    );
    let _ = std::fs::remove_dir_all(&raiz);
}

// Las formas asíncronas del flujo comparten el mismo cursor.
#[test]
fn el_flujo_deberia_tener_formas_asincronas() {
    let raiz = raiz_temporal("flujo_asincrono");
    let entorno = ejecutar_en(
        &raiz,
        permisos_totales(),
        &format!(
            "{IMPORTAR}\
             Flujo salida = esperar \
             SistemaArchivos.abrir_flujo_asincrono(\"./datos.bin\", \"escritura\")\n\
             esperar salida.escribir_asincrono(\"hola\")\n\
             esperar salida.escribir_bits_asincrono(Bits.desde_lista([33]))\n\
             salida.cerrar()\n\
             Flujo lector = SistemaArchivos.abrir_flujo(\"./datos.bin\", \"lectura\")\n\
             texto saludo = esperar lector.leer_asincrono(4)\n\
             Bits resto = esperar lector.leer_bits_asincrono(1)\n\
             entero sobrantes = resto.longitud()\n\
             lector.ir_al_inicio()\n\
             texto completo = esperar lector.leer_todo_asincrono()\n\
             lector.cerrar()\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "saludo"), "hola");
    assert_eq!(texto_global(&entorno, "sobrantes"), "1");
    assert_eq!(texto_global(&entorno, "completo"), "hola!");
    let _ = std::fs::remove_dir_all(&raiz);
}

// Un flujo cerrado no vuelve a operar, y el error es capturable.
#[test]
fn un_flujo_cerrado_deberia_rechazar_operaciones() {
    let raiz = raiz_temporal("cerrado");
    let entorno = ejecutar_en(
        &raiz,
        permisos_totales(),
        &format!(
            "{IMPORTAR}\
             SistemaArchivos.escribir(\"./datos.txt\", \"contenido\")\n\
             Flujo lector = SistemaArchivos.abrir_flujo(\"./datos.txt\", \"lectura\")\n\
             log abierto = lector.esta_cerrado()\n\
             lector.cerrar()\n\
             lector.cerrar()\n\
             log cerrado = lector.esta_cerrado()\n\
             texto var mensaje = \"\"\n\
             intentar {{\n\
                 texto perdido = lector.leer(1)\n\
             }} capturar (excepcion e) {{\n\
                 mensaje = e.mensaje\n\
             }}\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "abierto"), "falso");
    assert_eq!(texto_global(&entorno, "cerrado"), "verdadero");
    assert!(
        texto_global(&entorno, "mensaje").contains("flujo cerrado"),
        "el mensaje debe explicar que el flujo ya se cerró"
    );
    let _ = std::fs::remove_dir_all(&raiz);
}

// El modo pedido decide el permiso: leer no autoriza escribir.
#[test]
fn abrir_flujo_deberia_respetar_los_niveles_de_permiso() {
    let raiz = raiz_temporal("permisos_flujo");
    std::fs::create_dir_all(raiz.join("datos")).unwrap();
    std::fs::write(raiz.join("datos/entrada.txt"), "entrada").unwrap();

    let entorno = ejecutar_en(
        &raiz,
        r#"{"sistema_archivos": {"habilitado": true, "directorios": [
            {"ruta": "./datos", "permiso": "lectura"}]}}"#,
        &format!(
            "{IMPORTAR}\
             Flujo lector = SistemaArchivos.abrir_flujo(\"./datos/entrada.txt\", \"lectura\")\n\
             texto contenido = lector.leer_todo()\n\
             texto var sin_escritura = \"\"\n\
             texto var modo_invalido = \"\"\n\
             intentar {{\n\
                 lector.escribir(\"no\")\n\
             }} capturar (excepcion e) {{\n\
                 sin_escritura = e.mensaje\n\
             }}\n\
             lector.cerrar()\n\
             texto var sin_permiso = \"\"\n\
             intentar {{\n\
                 Flujo escritor = SistemaArchivos.abrir_flujo(\"./datos/entrada.txt\", \
             \"escritura\")\n\
             }} capturar (excepcion e) {{\n\
                 sin_permiso = e.mensaje\n\
             }}\n\
             intentar {{\n\
                 Flujo raro = SistemaArchivos.abrir_flujo(\"./datos/entrada.txt\", \"append\")\n\
             }} capturar (excepcion e) {{\n\
                 modo_invalido = e.mensaje\n\
             }}\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "contenido"), "entrada");
    assert!(
        texto_global(&entorno, "sin_escritura").contains("modo 'lectura'"),
        "un flujo de solo lectura no debe escribir"
    );
    assert!(
        texto_global(&entorno, "sin_permiso").contains("no tiene permiso de escritura"),
        "abrir para escribir sin el nivel debe rechazarse"
    );
    assert!(
        texto_global(&entorno, "modo_invalido").contains("no reconoce el modo 'append'"),
        "el modo desconocido debe orientar a los modos válidos"
    );
    assert_eq!(
        std::fs::read_to_string(raiz.join("datos/entrada.txt")).unwrap(),
        "entrada"
    );
    let _ = std::fs::remove_dir_all(&raiz);
}

// El observador entrega los cambios y deja de hacerlo tras detenerse.
#[test]
fn el_observador_deberia_notificar_cambios_hasta_detenerse() {
    let raiz = raiz_temporal("observador");
    std::fs::create_dir_all(raiz.join("vigilado")).unwrap();

    let entorno = ejecutar_en(
        &raiz,
        permisos_totales(),
        &format!(
            "{IMPORTAR}\
             Observador vigia = SistemaArchivos.observar(\"./vigilado\", verdadero)\n\
             log inactivo = vigia.esta_activo()\n\
             vigia.iniciar()\n\
             log activo = vigia.esta_activo()\n\
             log recursivo = vigia.es_recursivo()\n\
             SistemaArchivos.escribir(\"./vigilado/pedido.txt\", \"café\")\n\
             EventoArchivo evento = vigia.esperar_evento(5000)\n\
             texto tipo = evento.tipo()\n\
             log es_del_archivo = evento.ruta().contiene(\"pedido.txt\")\n\
             log sin_anterior = evento.ruta_anterior() == nulo\n\
             vigia.detener()\n\
             log detenido = vigia.esta_activo()\n\
             texto var mensaje = \"\"\n\
             intentar {{\n\
                 EventoArchivo tarde = vigia.esperar_evento(100)\n\
             }} capturar (excepcion e) {{\n\
                 mensaje = e.mensaje\n\
             }}\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "inactivo"), "falso");
    assert_eq!(texto_global(&entorno, "activo"), "verdadero");
    assert_eq!(texto_global(&entorno, "recursivo"), "verdadero");
    assert_eq!(texto_global(&entorno, "tipo"), "creado");
    assert_eq!(texto_global(&entorno, "es_del_archivo"), "verdadero");
    assert_eq!(texto_global(&entorno, "sin_anterior"), "verdadero");
    assert_eq!(texto_global(&entorno, "detenido"), "falso");
    assert!(
        texto_global(&entorno, "mensaje").contains("observador activo"),
        "tras detener, esperar eventos debe fallar con un mensaje orientador"
    );
    let _ = std::fs::remove_dir_all(&raiz);
}

// La espera asíncrona entrega el mismo evento sin bloquear el bucle, y el
// límite vencido devuelve nulo en vez de colgar el programa.
#[test]
fn el_observador_deberia_esperar_de_forma_asincrona_y_vencer_el_limite() {
    let raiz = raiz_temporal("observador_asincrono");
    std::fs::create_dir_all(raiz.join("vigilado")).unwrap();

    let entorno = ejecutar_en(
        &raiz,
        permisos_totales(),
        &format!(
            "{IMPORTAR}\
             Observador vigia = SistemaArchivos.observar(\"./vigilado\", falso)\n\
             vigia.iniciar()\n\
             SistemaArchivos.escribir(\"./vigilado/nota.txt\", \"hola\")\n\
             EventoArchivo evento = esperar vigia.esperar_evento_asincrono(5000)\n\
             log llego = evento != nulo\n\
             texto tipo = evento.tipo()\n\
             EventoArchivo var extra = vigia.esperar_evento(200)\n\
             mientras (extra != nulo) {{\n\
                 extra = vigia.esperar_evento(200)\n\
             }}\n\
             log vencio = extra == nulo\n\
             vigia.detener()\n"
        ),
    );
    assert_eq!(texto_global(&entorno, "llego"), "verdadero");
    assert_eq!(texto_global(&entorno, "tipo"), "creado");
    assert_eq!(
        texto_global(&entorno, "vencio"),
        "verdadero",
        "sin cambios nuevos, el límite debe vencer y devolver nulo"
    );
    let _ = std::fs::remove_dir_all(&raiz);
}

// Observar exige el mismo nivel que modificar: con solo lectura se rechaza.
#[test]
fn observar_sin_permiso_de_escritura_deberia_rechazarse() {
    let raiz = raiz_temporal("observador_permisos");
    std::fs::create_dir_all(raiz.join("datos")).unwrap();

    let entorno = ejecutar_en(
        &raiz,
        r#"{"sistema_archivos": {"habilitado": true, "directorios": [
            {"ruta": "./datos", "permiso": "lectura"}]}}"#,
        &format!(
            "{IMPORTAR}\
             texto var mensaje = \"\"\n\
             intentar {{\n\
                 Observador vigia = SistemaArchivos.observar(\"./datos\", falso)\n\
             }} capturar (excepcion e) {{\n\
                 mensaje = e.mensaje\n\
             }}\n"
        ),
    );
    assert!(
        texto_global(&entorno, "mensaje").contains("no tiene permiso de escritura"),
        "observar con nivel lectura debe rechazarse"
    );
    let _ = std::fs::remove_dir_all(&raiz);
}
