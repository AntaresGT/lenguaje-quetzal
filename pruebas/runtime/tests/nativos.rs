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

    let mut vm = Vm::nueva(Rc::new(registro));
    let (entorno, _valor) = vm
        .cargar_modulo(Rc::new(modulo), importaciones)
        .expect("el código de prueba debe ejecutar sin errores");
    entorno
}

fn ejecutar(codigo: &str) -> Rc<maquina_virtual::valores::EntornoModulo> {
    ejecutar_con_registro(codigo, modulos_nativos::crear_registro())
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
