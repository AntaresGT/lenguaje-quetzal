//! Módulos nativos: matematica, métodos de texto/lista/jsn, conversiones,
//! rango y tiempo.

use std::rc::Rc;

use indexmap::IndexMap;
use maquina_virtual::{Valor, Vm};
use nucleo::Fuente;

fn ejecutar_con_registro(
    codigo: &str,
    registro: maquina_virtual::RegistroNativos,
) -> Rc<maquina_virtual::valores::EntornoModulo> {
    let fuente = Fuente::nueva("prueba.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("el código de prueba debe parsear");
    semantica::analizar_modulo(&ast).expect("el código de prueba debe pasar la semántica");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar bytecode");

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
                // Tipos exportados (`Archivo`, `Ruta`) apuntan a su propio
                // módulo nativo, igual que hace el cargador del motor.
                let destino = modulos_nativos::modulo_de_tipo_exportado(
                    &modulo_normalizado,
                    &simbolo,
                )
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

    let mut vm = Vm::nueva(Rc::new(registro));
    let (entorno, _valor) = vm
        .cargar_modulo(Rc::new(modulo), importaciones)
        .expect("el código de prueba debe ejecutar sin errores");
    entorno
}

fn ejecutar(codigo: &str) -> Rc<maquina_virtual::valores::EntornoModulo> {
    ejecutar_con_registro(codigo, modulos_nativos::crear_registro())
}

/// Ejecuta con permiso `todo` sobre la raíz temporal dada.
fn ejecutar_con_permisos(
    codigo: &str,
    raiz: &std::path::Path,
) -> Rc<maquina_virtual::valores::EntornoModulo> {
    let permisos_json = serde_json::json!({
        "sistema_archivos": {
            "habilitado": true,
            "directorios": [{"ruta": ".", "permiso": "todo"}]
        }
    });
    let permisos =
        paquetes::Permisos::desde_json(&permisos_json).expect("permisos de prueba válidos");
    let guardian = Rc::new(runtime::GuardianPermisos::denegado());
    guardian.configurar(permisos, raiz);
    ejecutar_con_registro(codigo, modulos_nativos::crear_registro_con_permisos(&guardian))
}

/// Crea un directorio temporal único para una prueba.
fn directorio_temporal(nombre: &str) -> std::path::PathBuf {
    let ruta =
        std::env::temp_dir().join(format!("quetzal_nativos_{nombre}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&ruta);
    std::fs::create_dir_all(&ruta).expect("se puede crear el directorio temporal");
    ruta
}

/// Ruta para interpolar en código Quetzal (separadores `/`).
fn ruta_qz(ruta: &std::path::Path) -> String {
    ruta.to_string_lossy().replace('\\', "/")
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

#[test]
fn metodos_de_texto_deberian_funcionar() {
    let entorno = ejecutar(
        "texto base = \"Hola Mundo\"\n\
         texto mayus = base.mayusculas()\n\
         entero largo = base.longitud()\n\
         texto rev = base.invertir()\n\
         lista<texto> partes = base.dividir(\" \")\n\
         texto primera = partes[0]\n\
         entero pos = base.encontrar(\"Mundo\")\n\
         texto b64 = base.a_base64()\n\
         texto vuelta = b64.decodificar_base64()\n",
    );
    assert_eq!(texto_global(&entorno, "mayus"), "HOLA MUNDO");
    assert_eq!(texto_global(&entorno, "largo"), "10");
    assert_eq!(texto_global(&entorno, "rev"), "odnuM aloH");
    assert_eq!(texto_global(&entorno, "primera"), "Hola");
    assert_eq!(texto_global(&entorno, "pos"), "5");
    assert_eq!(texto_global(&entorno, "vuelta"), "Hola Mundo");
}

#[test]
fn metodos_de_lista_deberian_funcionar() {
    let entorno = ejecutar(
        "lista<entero> var numeros = [3, 1, 4, 2]\n\
         numeros.ordenar()\n\
         texto orden = numeros.texto()\n\
         entero suma = numeros.sumar()\n\
         número prom = numeros.promedio()\n\
         numeros.agregar(5)\n\
         entero largo = numeros.longitud()\n\
         lista enteros = rango(1, 5)\n\
         texto rangos = enteros.texto()\n\
         texto unido = numeros.unir(\"-\")\n",
    );
    assert_eq!(texto_global(&entorno, "orden"), "[1, 2, 3, 4]");
    assert_eq!(texto_global(&entorno, "suma"), "10");
    assert_eq!(texto_global(&entorno, "prom"), "2.5");
    assert_eq!(texto_global(&entorno, "largo"), "5");
    assert_eq!(texto_global(&entorno, "rangos"), "[1, 2, 3, 4, 5]");
    assert_eq!(texto_global(&entorno, "unido"), "1-2-3-4-5");
}

#[test]
fn metodos_de_jsn_deberian_funcionar() {
    let entorno = ejecutar(
        "jsn var persona = {nombre: \"Ana\", edad: 30}\n\
         log tiene = persona.contiene_clave(\"edad\")\n\
         lista<texto> claves = persona.claves()\n\
         persona.establecer(\"pais\", \"Guatemala\")\n\
         persona.eliminar(\"edad\")\n\
         texto serial = persona.texto()\n\
         jsn vuelta = \"{\\\"a\\\": 1}\".jsn()\n\
         entero a = vuelta.a\n",
    );
    assert_eq!(texto_global(&entorno, "tiene"), "verdadero");
    assert_eq!(texto_global(&entorno, "claves"), "[nombre, edad]");
    assert_eq!(
        texto_global(&entorno, "serial"),
        "{\"nombre\":\"Ana\",\"pais\":\"Guatemala\"}"
    );
    assert_eq!(texto_global(&entorno, "a"), "1");
}

#[test]
fn conversiones_deberian_funcionar() {
    let entorno = ejecutar(
        "entero e = \"1234\".entero()\n\
         número n = \"12.5\".número()\n\
         log v = \"verdadero\".log()\n\
         texto te = 99.texto()\n\
         texto tn = 12.5.texto()\n\
         texto tl = falso.texto()\n\
         lista<entero> desde_texto = \"1,2,3\".lista()\n\
         texto tdesde = desde_texto.texto()\n",
    );
    assert_eq!(texto_global(&entorno, "e"), "1234");
    assert_eq!(texto_global(&entorno, "n"), "12.5");
    assert_eq!(texto_global(&entorno, "v"), "verdadero");
    assert_eq!(texto_global(&entorno, "te"), "99");
    assert_eq!(texto_global(&entorno, "tn"), "12.5");
    assert_eq!(texto_global(&entorno, "tl"), "falso");
    assert_eq!(texto_global(&entorno, "tdesde"), "[1, 2, 3]");
}

#[test]
fn matematica_deberia_calcular() {
    // El acceso al módulo nativo se hace por la variable importada; aquí se
    // simula la importación usando los nombres internos.
    let entorno = ejecutar(
        "importar { Matemática } desde \"quetzal/matemática\"\n\
         número s = Matemática.sumar(12.5, 7.3)\n\
         número r = Matemática.raiz_cuadrada(16)\n\
         número p = Matemática.redondear(3.141592, 2)\n\
         número maximo = Matemática.maximo([2.5, 13.0, 5.0])\n",
    );
    assert_eq!(texto_global(&entorno, "s"), "19.8");
    assert_eq!(texto_global(&entorno, "r"), "4");
    assert_eq!(texto_global(&entorno, "p"), "3.14");
    assert_eq!(texto_global(&entorno, "maximo"), "13");
}

#[test]
fn tiempo_deberia_dar_marca_y_ahora() {
    let entorno = ejecutar(
        "importar { Tiempo } desde \"quetzal/tiempo\"\n\
         entero marca = Tiempo.marca()\n\
         Tiempo ahora = Tiempo.ahora()\n\
         texto fecha = ahora.texto_fecha()\n\
         texto zona = Tiempo.zona_local()\n",
    );
    assert!(texto_global(&entorno, "marca").parse::<i64>().is_ok());
    let fecha = texto_global(&entorno, "fecha");
    assert_eq!(fecha.len(), 10, "fecha en formato YYYY-MM-DD: {fecha}");
    assert!(!texto_global(&entorno, "zona").is_empty());
}

#[test]
fn tiempo_deberia_instanciarse_con_nuevo() {
    let entorno = ejecutar(
        "importar { Tiempo } desde \"quetzal/tiempo\"\n\
         Tiempo t = nuevo Tiempo(2026, 6, 11, 14, 30, 5, \"UTC\")\n\
         entero año = t.año()\n\
         entero mes = t.mes()\n\
         entero dia = t.dia()\n\
         entero hora = t.hora()\n\
         entero dia_semana = t.dia_semana()\n\
         entero dia_año = t.dia_año()\n\
         texto nombre_dia = t.nombre_dia()\n\
         texto nombre_mes = t.nombre_mes()\n\
         texto fecha = t.texto_fecha()\n\
         texto legible = t.formatear(\"%d/%m/%Y %H:%M\")\n",
    );
    assert_eq!(texto_global(&entorno, "año"), "2026");
    assert_eq!(texto_global(&entorno, "mes"), "6");
    assert_eq!(texto_global(&entorno, "dia"), "11");
    assert_eq!(texto_global(&entorno, "hora"), "14");
    assert_eq!(texto_global(&entorno, "dia_semana"), "4");
    assert_eq!(texto_global(&entorno, "dia_año"), "162");
    assert_eq!(texto_global(&entorno, "nombre_dia"), "jueves");
    assert_eq!(texto_global(&entorno, "nombre_mes"), "junio");
    assert_eq!(texto_global(&entorno, "fecha"), "2026-06-11");
    assert_eq!(texto_global(&entorno, "legible"), "11/06/2026 14:30");
}

#[test]
fn tiempo_deberia_sumar_y_restar() {
    let entorno = ejecutar(
        "importar { Tiempo } desde \"quetzal/tiempo\"\n\
         Tiempo inicio = nuevo Tiempo(2026, 1, 31, 12, 0, 0, \"UTC\")\n\
         Tiempo mas_dia = inicio.agregar_dias(1)\n\
         Tiempo mas_mes = inicio.agregar_meses(1)\n\
         Tiempo menos_año = inicio.agregar_años(-1)\n\
         texto f1 = mas_dia.texto_fecha()\n\
         texto f2 = mas_mes.texto_fecha()\n\
         texto f3 = menos_año.texto_fecha()\n\
         jsn delta = inicio.diferencia(mas_dia)\n\
         entero horas = delta.horas\n",
    );
    assert_eq!(texto_global(&entorno, "f1"), "2026-02-01");
    // Recorta al último día válido del mes.
    assert_eq!(texto_global(&entorno, "f2"), "2026-02-28");
    assert_eq!(texto_global(&entorno, "f3"), "2025-01-31");
    assert_eq!(texto_global(&entorno, "horas"), "24");
}

#[test]
fn tiempo_deberia_convertir_zonas_horarias() {
    let entorno = ejecutar(
        "importar { Tiempo } desde \"quetzal/tiempo\"\n\
         Tiempo utc = nuevo Tiempo(2026, 1, 15, 12, 0, 0, \"UTC\")\n\
         texto zona = utc.zona()\n\
         texto desfase = utc.desfase()\n\
         Tiempo guate = utc.en_zona(\"America/Guatemala\")\n\
         entero hora = guate.hora()\n\
         texto desfase_guate = guate.desfase()\n\
         log mismo = utc.es_mismo_instante(guate)\n\
         lista<texto> america = Tiempo.zonas(\"America\")\n\
         entero cantidad = america.longitud()\n",
    );
    assert_eq!(texto_global(&entorno, "zona"), "UTC");
    assert_eq!(texto_global(&entorno, "desfase"), "+00:00");
    assert_eq!(texto_global(&entorno, "hora"), "6");
    assert_eq!(texto_global(&entorno, "desfase_guate"), "-06:00");
    assert_eq!(texto_global(&entorno, "mismo"), "verdadero");
    assert!(texto_global(&entorno, "cantidad").parse::<i64>().unwrap() > 0);
}

#[test]
fn sistema_archivos_deberia_leer_y_escribir_texto() {
    let raiz = directorio_temporal("sa_texto");
    let base = ruta_qz(&raiz);
    let entorno = ejecutar_con_permisos(
        &format!(
            "importar {{ SistemaArchivos }} desde \"quetzal/sistema_archivos\"\n\
             SistemaArchivos.escribir_texto(\"{base}/notas.txt\", \"hola\\nmundo\\n\")\n\
             SistemaArchivos.agregar_texto(\"{base}/notas.txt\", \"fin\\n\")\n\
             texto contenido = SistemaArchivos.leer_texto(\"{base}/notas.txt\")\n\
             lista<texto> lineas = SistemaArchivos.leer_lineas(\"{base}/notas.txt\")\n\
             entero cantidad = lineas.longitud()\n\
             log hay = SistemaArchivos.existe(\"{base}/notas.txt\")\n\
             log es_archivo = SistemaArchivos.es_archivo(\"{base}/notas.txt\")\n\
             log es_dir = SistemaArchivos.es_directorio(\"{base}\")\n\
             entero bytes = SistemaArchivos.tamaño(\"{base}/notas.txt\")\n\
             jsn meta = SistemaArchivos.metadatos(\"{base}/notas.txt\")\n\
             entero meta_tamaño = meta.tamaño\n"
        ),
        &raiz,
    );
    assert_eq!(texto_global(&entorno, "contenido"), "hola\nmundo\nfin\n");
    assert_eq!(texto_global(&entorno, "cantidad"), "3");
    assert_eq!(texto_global(&entorno, "hay"), "verdadero");
    assert_eq!(texto_global(&entorno, "es_archivo"), "verdadero");
    assert_eq!(texto_global(&entorno, "es_dir"), "verdadero");
    assert_eq!(texto_global(&entorno, "bytes"), "15");
    assert_eq!(texto_global(&entorno, "meta_tamaño"), "15");
    let _ = std::fs::remove_dir_all(&raiz);
}

#[test]
fn sistema_archivos_deberia_manejar_directorios_y_mover() {
    let raiz = directorio_temporal("sa_dirs");
    let base = ruta_qz(&raiz);
    let entorno = ejecutar_con_permisos(
        &format!(
            "importar {{ SistemaArchivos }} desde \"quetzal/sistema_archivos\"\n\
             SistemaArchivos.crear_directorio(\"{base}/a/b\")\n\
             SistemaArchivos.escribir_texto(\"{base}/a/uno.txt\", \"1\")\n\
             SistemaArchivos.copiar(\"{base}/a/uno.txt\", \"{base}/a/dos.txt\")\n\
             SistemaArchivos.mover(\"{base}/a/dos.txt\", \"{base}/a/b/dos.txt\")\n\
             lista<texto> dentro = SistemaArchivos.listar(\"{base}/a/b\")\n\
             SistemaArchivos.eliminar_archivo(\"{base}/a/b/dos.txt\")\n\
             SistemaArchivos.eliminar_directorio(\"{base}/a/b\")\n\
             log queda_b = SistemaArchivos.existe(\"{base}/a/b\")\n\
             SistemaArchivos.eliminar_todo(\"{base}/a\")\n\
             log queda_a = SistemaArchivos.existe(\"{base}/a\")\n"
        ),
        &raiz,
    );
    assert_eq!(texto_global(&entorno, "dentro"), "[dos.txt]");
    assert_eq!(texto_global(&entorno, "queda_b"), "falso");
    assert_eq!(texto_global(&entorno, "queda_a"), "falso");
    let _ = std::fs::remove_dir_all(&raiz);
}

#[test]
fn sistema_archivos_deberia_leer_y_escribir_bits() {
    let raiz = directorio_temporal("sa_bits");
    let base = ruta_qz(&raiz);
    let entorno = ejecutar_con_permisos(
        &format!(
            "importar {{ SistemaArchivos }} desde \"quetzal/sistema_archivos\"\n\
             importar {{ Bits }} desde \"quetzal/bits\"\n\
             Bits datos = nuevo Bits([72, 111, 108, 97])\n\
             SistemaArchivos.escribir_bits(\"{base}/crudo.bin\", datos)\n\
             SistemaArchivos.agregar_bits(\"{base}/crudo.bin\", [33])\n\
             Bits vuelta = SistemaArchivos.leer_bits(\"{base}/crudo.bin\")\n\
             texto texto_vuelta = vuelta.a_texto()\n\
             entero largo = vuelta.longitud()\n"
        ),
        &raiz,
    );
    assert_eq!(texto_global(&entorno, "texto_vuelta"), "Hola!");
    assert_eq!(texto_global(&entorno, "largo"), "5");
    let _ = std::fs::remove_dir_all(&raiz);
}

#[test]
fn archivo_deberia_escribir_y_leer_por_modos() {
    let raiz = directorio_temporal("archivo_modos");
    let base = ruta_qz(&raiz);
    let entorno = ejecutar_con_permisos(
        &format!(
            "importar {{ SistemaArchivos, Archivo }} desde \"quetzal/sistema_archivos\"\n\
             Archivo salida = nuevo Archivo(\"{base}/registro.txt\", \"escritura\")\n\
             salida.escribir_linea(\"primera\")\n\
             salida.escribir_linea(\"segunda\")\n\
             texto modo = salida.modo()\n\
             salida.cerrar()\n\
             log abierto = salida.esta_abierto()\n\
             Archivo extra = nuevo Archivo(\"{base}/registro.txt\", \"agregar\")\n\
             extra.escribir_linea(\"tercera\")\n\
             extra.cerrar()\n\
             Archivo lector = nuevo Archivo(\"{base}/registro.txt\")\n\
             entero total = lector.tamaño()\n\
             lista<texto> lineas = lector.leer_lineas()\n\
             entero cantidad = lineas.longitud()\n\
             log al_final = lector.al_final()\n\
             lector.ir_al_inicio()\n\
             texto todo = lector.leer_texto()\n\
             texto nombre = lector.nombre()\n\
             texto extension = lector.extension()\n\
             lector.cerrar()\n"
        ),
        &raiz,
    );
    assert_eq!(texto_global(&entorno, "modo"), "escritura");
    assert_eq!(texto_global(&entorno, "abierto"), "falso");
    assert_eq!(texto_global(&entorno, "cantidad"), "3");
    assert_eq!(texto_global(&entorno, "al_final"), "verdadero");
    assert_eq!(texto_global(&entorno, "todo"), "primera\nsegunda\ntercera\n");
    assert_eq!(texto_global(&entorno, "total"), "24");
    assert_eq!(texto_global(&entorno, "nombre"), "registro.txt");
    assert_eq!(texto_global(&entorno, "extension"), "txt");
    let _ = std::fs::remove_dir_all(&raiz);
}

#[test]
fn archivo_deberia_leer_por_lotes_con_cursor() {
    let raiz = directorio_temporal("archivo_lotes");
    std::fs::write(
        raiz.join("grande.log"),
        "linea 1\nlinea 2\nlinea 3\nlinea 4\n",
    )
    .expect("se puede escribir el archivo de prueba");
    let base = ruta_qz(&raiz);
    let entorno = ejecutar_con_permisos(
        &format!(
            "importar {{ Archivo }} desde \"quetzal/sistema_archivos\"\n\
             importar {{ Bits }} desde \"quetzal/bits\"\n\
             Archivo a = nuevo Archivo(\"{base}/grande.log\")\n\
             entero var lineas = 0\n\
             mientras (!a.al_final()) {{\n\
                 texto linea = a.leer_linea()\n\
                 lineas = lineas + 1\n\
             }}\n\
             a.ir_al_inicio()\n\
             Bits lote = a.leer_bits(8)\n\
             texto lote_texto = lote.a_texto()\n\
             entero posicion = a.posicion()\n\
             entero restantes = a.bits_restantes()\n\
             a.ir_a(8)\n\
             texto trozo = a.leer_texto(7)\n\
             a.cerrar()\n"
        ),
        &raiz,
    );
    assert_eq!(texto_global(&entorno, "lineas"), "4");
    assert_eq!(texto_global(&entorno, "lote_texto"), "linea 1\n");
    assert_eq!(texto_global(&entorno, "posicion"), "8");
    assert_eq!(texto_global(&entorno, "restantes"), "24");
    assert_eq!(texto_global(&entorno, "trozo"), "linea 2");
    let _ = std::fs::remove_dir_all(&raiz);
}

#[test]
fn archivo_deberia_eliminar_y_rechazar_operaciones_invalidas() {
    let raiz = directorio_temporal("archivo_errores");
    let base = ruta_qz(&raiz);
    let entorno = ejecutar_con_permisos(
        &format!(
            "importar {{ Archivo }} desde \"quetzal/sistema_archivos\"\n\
             Archivo a = nuevo Archivo(\"{base}/efimero.txt\", \"escritura\")\n\
             a.escribir(\"datos\")\n\
             texto var error_lectura = \"\"\n\
             intentar {{\n\
                 texto contenido = a.leer_texto()\n\
             }} capturar (excepcion e) {{\n\
                 error_lectura = e.mensaje\n\
             }}\n\
             a.eliminar()\n\
             log queda = a.existe()\n\
             texto var error_cerrado = \"\"\n\
             intentar {{\n\
                 a.escribir(\"mas\")\n\
             }} capturar (excepcion e) {{\n\
                 error_cerrado = e.mensaje\n\
             }}\n"
        ),
        &raiz,
    );
    assert!(
        texto_global(&entorno, "error_lectura").contains("no puede leer"),
        "leer en modo escritura debe fallar"
    );
    assert!(
        texto_global(&entorno, "error_cerrado").contains("archivo cerrado"),
        "operar tras eliminar debe fallar"
    );
    let _ = std::fs::remove_dir_all(&raiz);
}

#[test]
fn ruta_deberia_manipular_rutas_sin_disco() {
    let entorno = ejecutar(
        "importar { Ruta } desde \"quetzal/sistema_archivos\"\n\
         Ruta r = nuevo Ruta(\"datos/informes\", \"ventas.csv\")\n\
         texto nombre = r.nombre()\n\
         texto sin_ext = r.nombre_sin_extension()\n\
         texto ext = r.extension()\n\
         Ruta carpeta = r.padre()\n\
         texto padre_texto = carpeta.texto()\n\
         Ruta json = r.con_extension(\"json\")\n\
         texto json_nombre = json.nombre()\n\
         Ruta unida = carpeta.unir(\"resumen.txt\")\n\
         texto unida_nombre = unida.nombre()\n\
         lista<texto> partes = r.componentes()\n\
         entero cantidad = partes.longitud()\n\
         log absoluta = r.es_absoluta()\n\
         texto separador = Ruta.separador()\n",
    );
    assert_eq!(texto_global(&entorno, "nombre"), "ventas.csv");
    assert_eq!(texto_global(&entorno, "sin_ext"), "ventas");
    assert_eq!(texto_global(&entorno, "ext"), "csv");
    assert!(texto_global(&entorno, "padre_texto").ends_with("informes"));
    assert_eq!(texto_global(&entorno, "json_nombre"), "ventas.json");
    assert_eq!(texto_global(&entorno, "unida_nombre"), "resumen.txt");
    assert_eq!(texto_global(&entorno, "cantidad"), "3");
    assert_eq!(texto_global(&entorno, "absoluta"), "falso");
    assert!(!texto_global(&entorno, "separador").is_empty());
}

#[test]
fn ruta_deberia_consultar_el_disco_con_permisos() {
    let raiz = directorio_temporal("ruta_disco");
    std::fs::write(raiz.join("real.txt"), "x").expect("se puede escribir el archivo de prueba");
    let base = ruta_qz(&raiz);
    let entorno = ejecutar_con_permisos(
        &format!(
            "importar {{ Ruta }} desde \"quetzal/sistema_archivos\"\n\
             Ruta archivo = nuevo Ruta(\"{base}\", \"real.txt\")\n\
             log hay = archivo.existe()\n\
             log es_archivo = archivo.es_archivo()\n\
             Ruta carpeta = nuevo Ruta(\"{base}\")\n\
             log es_dir = carpeta.es_directorio()\n\
             Ruta fantasma = nuevo Ruta(\"{base}\", \"no_existe.txt\")\n\
             log falta = fantasma.existe()\n"
        ),
        &raiz,
    );
    assert_eq!(texto_global(&entorno, "hay"), "verdadero");
    assert_eq!(texto_global(&entorno, "es_archivo"), "verdadero");
    assert_eq!(texto_global(&entorno, "es_dir"), "verdadero");
    assert_eq!(texto_global(&entorno, "falta"), "falso");
    let _ = std::fs::remove_dir_all(&raiz);
}

#[test]
fn bits_deberia_construirse_y_convertirse() {
    let entorno = ejecutar(
        "importar { Bits } desde \"quetzal/bits\"\n\
         Bits sin_datos = nuevo Bits()\n\
         entero largo_vacio = sin_datos.longitud()\n\
         Bits ceros = nuevo Bits(4)\n\
         entero largo_ceros = ceros.longitud()\n\
         Bits saludo = nuevo Bits(\"Hola\")\n\
         texto hex = saludo.a_hex()\n\
         Bits desde_hex = Bits.desde_hex(\"48 6f 6c 61\")\n\
         texto vuelta = desde_hex.a_texto()\n\
         Bits desde_lista = nuevo Bits([1, 2, 3])\n\
         lista<entero> enteros = desde_lista.a_lista()\n\
         entero primero = saludo.obtener(0)\n\
         entero ultimo = saludo.obtener(-1)\n\
         Bits parte = saludo.rebanada(1, 3)\n\
         texto parte_texto = parte.a_texto()\n",
    );
    assert_eq!(texto_global(&entorno, "largo_vacio"), "0");
    assert_eq!(texto_global(&entorno, "largo_ceros"), "4");
    assert_eq!(texto_global(&entorno, "hex"), "486f6c61");
    assert_eq!(texto_global(&entorno, "vuelta"), "Hola");
    assert_eq!(texto_global(&entorno, "enteros"), "[1, 2, 3]");
    assert_eq!(texto_global(&entorno, "primero"), "72");
    assert_eq!(texto_global(&entorno, "ultimo"), "97");
    assert_eq!(texto_global(&entorno, "parte_texto"), "ol");
}

#[test]
fn bits_deberia_mutarse_y_operar_bit_a_bit() {
    let entorno = ejecutar(
        "importar { Bits } desde \"quetzal/bits\"\n\
         Bits var datos = nuevo Bits([240, 15])\n\
         datos.agregar(255)\n\
         datos.fijar(0, 170)\n\
         datos.extender([0])\n\
         entero largo = datos.longitud()\n\
         texto hex = datos.a_hex()\n\
         Bits mascara = nuevo Bits([15, 15, 15, 15])\n\
         Bits con_y = datos.y(mascara)\n\
         texto hex_y = con_y.a_hex()\n\
         Bits con_o = datos.o(mascara)\n\
         texto hex_o = con_o.a_hex()\n\
         Bits con_xor = datos.oexclusivo(mascara)\n\
         texto hex_xor = con_xor.a_hex()\n\
         Bits negado = datos.negar()\n\
         texto hex_no = negado.a_hex()\n\
         Bits corrido = nuevo Bits([1, 128]).desplazar_izquierda(1)\n\
         texto hex_corrido = corrido.a_hex()\n\
         Bits derecha = nuevo Bits([1, 128]).desplazar_derecha(1)\n\
         texto hex_derecha = derecha.a_hex()\n\
         datos.limpiar()\n\
         entero limpio = datos.longitud()\n\
         texto var error_largo = \"\"\n\
         intentar {\n\
             Bits invalido = mascara.y(nuevo Bits(2))\n\
         } capturar (excepcion e) {\n\
             error_largo = e.mensaje\n\
         }\n",
    );
    assert_eq!(texto_global(&entorno, "largo"), "4");
    assert_eq!(texto_global(&entorno, "hex"), "aa0fff00");
    assert_eq!(texto_global(&entorno, "hex_y"), "0a0f0f00");
    assert_eq!(texto_global(&entorno, "hex_o"), "af0fff0f");
    assert_eq!(texto_global(&entorno, "hex_xor"), "a500f00f");
    assert_eq!(texto_global(&entorno, "hex_no"), "55f000ff");
    assert_eq!(texto_global(&entorno, "hex_corrido"), "0300");
    assert_eq!(texto_global(&entorno, "hex_derecha"), "00c0");
    assert_eq!(texto_global(&entorno, "limpio"), "0");
    assert!(
        texto_global(&entorno, "error_largo").contains("misma longitud"),
        "operación binaria con longitudes distintas debe fallar"
    );
}

#[test]
fn tiempo_deberia_comparar_y_analizar() {
    let entorno = ejecutar(
        "importar { Tiempo } desde \"quetzal/tiempo\"\n\
         Tiempo a = nuevo Tiempo(\"2026-06-11 08:00:00\")\n\
         Tiempo b = Tiempo.analizar(\"11/06/2026 20:15:00\", \"%d/%m/%Y %H:%M:%S\")\n\
         log antes = a.es_antes(b)\n\
         log despues = a.es_despues(b)\n\
         log mismo_dia = a.es_mismo_dia(b)\n\
         log bisiesto = Tiempo.es_bisiesto(2024)\n\
         entero dias_febrero = Tiempo.dias_en_mes(2024, 2)\n",
    );
    assert_eq!(texto_global(&entorno, "antes"), "verdadero");
    assert_eq!(texto_global(&entorno, "despues"), "falso");
    assert_eq!(texto_global(&entorno, "mismo_dia"), "verdadero");
    assert_eq!(texto_global(&entorno, "bisiesto"), "verdadero");
    assert_eq!(texto_global(&entorno, "dias_febrero"), "29");
}
