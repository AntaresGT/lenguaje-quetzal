//! Objetos en runtime: `nuevo`, `esto`, miembros `libre`, herencia y `padre`.

use std::rc::Rc;

use indexmap::IndexMap;
use maquina_virtual::{Vm, texto_de_valor};
use nucleo::Fuente;

fn ejecutar(codigo: &str) -> Rc<maquina_virtual::valores::EntornoModulo> {
    let fuente = Fuente::nueva("prueba.qz", codigo);
    let ast = sintaxis::parsear_modulo(&fuente).expect("el código de prueba debe parsear");
    semantica::analizar_modulo(&ast).expect("el código de prueba debe pasar la semántica");
    let modulo = bytecode::generar_modulo(&ast).expect("debe generar bytecode");
    let mut vm = Vm::nueva(Rc::new(modulos_nativos::crear_registro()));
    let (entorno, _valor) = vm
        .cargar_modulo(Rc::new(modulo), IndexMap::new())
        .expect("el código de prueba debe ejecutar sin errores");
    entorno
}

fn global(entorno: &maquina_virtual::valores::EntornoModulo, nombre: &str) -> String {
    let valor = &entorno
        .globales
        .borrow()
        .get(nombre)
        .unwrap_or_else(|| panic!("debe existir '{nombre}'"))
        .valor
        .clone();
    texto_de_valor(valor)
}

#[test]
fn objetos_deberian_instanciarse_con_constructor_y_esto() {
    let entorno = ejecutar(
        "objeto Usuario {\n\
             privado:\n\
                 texto var nombre\n\
                 texto obtener_nombre_privado() {\n\
                     retornar esto.nombre\n\
                 }\n\
             publico:\n\
                 Usuario(texto nombre) {\n\
                     esto.nombre = nombre\n\
                 }\n\
                 texto obtener_nombre() {\n\
                     retornar esto.obtener_nombre_privado()\n\
                 }\n\
         }\n\
         Usuario usuario = nuevo Usuario(\"Juan\")\n\
         texto nombre = usuario.obtener_nombre()\n",
    );
    assert_eq!(global(&entorno, "nombre"), "Juan");
}

#[test]
fn miembros_libres_deberian_funcionar_sin_instancia() {
    let entorno = ejecutar(
        "objeto Util {\n\
             libre entero var contador = 0\n\
             libre entero absoluto(entero valor) {\n\
                 retornar valor < 0 ? -valor : valor\n\
             }\n\
             libre vacio incrementar() {\n\
                 esto.contador++\n\
             }\n\
         }\n\
         entero absoluto = Util.absoluto(-10)\n\
         Util.incrementar()\n\
         Util.incrementar()\n\
         entero contador = Util.contador\n",
    );
    assert_eq!(global(&entorno, "absoluto"), "10");
    assert_eq!(global(&entorno, "contador"), "2");
}

#[test]
fn herencia_simple_deberia_resolver_metodos_y_padre() {
    let entorno = ejecutar(
        "objeto Animal {\n\
             publico:\n\
                 texto var nombre\n\
                 Animal(texto nombre) {\n\
                     esto.nombre = nombre\n\
                 }\n\
                 texto sonido() {\n\
                     retornar \"...\"\n\
                 }\n\
                 texto presentar() {\n\
                     retornar t\"{esto.nombre} dice {esto.sonido()}\"\n\
                 }\n\
         }\n\
         objeto Perro hereda Animal {\n\
             publico:\n\
                 Perro(texto nombre) {\n\
                     padre.Animal(nombre)\n\
                 }\n\
                 texto sonido() {\n\
                     retornar \"Guau\"\n\
                 }\n\
                 texto sonido_padre() {\n\
                     retornar padre.sonido()\n\
                 }\n\
         }\n\
         Perro perro = nuevo Perro(\"Firulais\")\n\
         texto presentacion = perro.presentar()\n\
         texto base = perro.sonido_padre()\n",
    );
    assert_eq!(global(&entorno, "presentacion"), "Firulais dice Guau");
    assert_eq!(global(&entorno, "base"), "...");
}

#[test]
fn herencia_multiple_deberia_buscar_en_todos_los_padres() {
    let entorno = ejecutar(
        "objeto Nadador {\n\
             publico:\n\
                 texto nadar() {\n\
                     retornar \"nada\"\n\
                 }\n\
         }\n\
         objeto Volador {\n\
             publico:\n\
                 texto volar() {\n\
                     retornar \"vuela\"\n\
                 }\n\
         }\n\
         objeto Pato hereda Nadador, Volador {\n\
             publico:\n\
                 texto hacer_todo() {\n\
                     retornar esto.nadar() + \" y \" + esto.volar()\n\
                 }\n\
         }\n\
         Pato pato = nuevo Pato()\n\
         texto todo = pato.hacer_todo()\n",
    );
    assert_eq!(global(&entorno, "todo"), "nada y vuela");
}

#[test]
fn padre_con_nombre_deberia_elegir_el_padre_concreto() {
    let entorno = ejecutar(
        "objeto Mamifero {\n\
             publico:\n\
                 texto comer() {\n\
                     retornar \"come como mamífero\"\n\
                 }\n\
         }\n\
         objeto Ave {\n\
             publico:\n\
                 texto comer() {\n\
                     retornar \"come como ave\"\n\
                 }\n\
         }\n\
         objeto Ornitorrinco hereda Mamifero, Ave {\n\
             publico:\n\
                 texto comer_mamifero() {\n\
                     retornar padre.Mamifero.comer()\n\
                 }\n\
                 texto comer_ave() {\n\
                     retornar padre.Ave.comer()\n\
                 }\n\
         }\n\
         Ornitorrinco orni = nuevo Ornitorrinco()\n\
         texto mamifero = orni.comer_mamifero()\n\
         texto ave = orni.comer_ave()\n",
    );
    assert_eq!(global(&entorno, "mamifero"), "come como mamífero");
    assert_eq!(global(&entorno, "ave"), "come como ave");
}

#[test]
fn atributos_heredados_deberian_aplanarse_en_la_instancia() {
    let entorno = ejecutar(
        "objeto Base {\n\
             publico:\n\
                 entero var puntos = 100\n\
         }\n\
         objeto Derivado hereda Base {\n\
             publico:\n\
                 entero leer() {\n\
                     retornar esto.puntos\n\
                 }\n\
         }\n\
         Derivado objeto_derivado = nuevo Derivado()\n\
         entero puntos = objeto_derivado.leer()\n",
    );
    assert_eq!(global(&entorno, "puntos"), "100");
}

#[test]
fn instancia_constante_deberia_permitir_mutar_atributos_var() {
    // Constante de referencia: no se reasigna la variable, pero sus
    // atributos declarados con `var` sí cambian.
    let entorno = ejecutar(
        "objeto Caja {\n\
             publico:\n\
                 entero var contenido = 0\n\
         }\n\
         Caja caja = nuevo Caja()\n\
         caja.contenido = 42\n\
         entero contenido = caja.contenido\n",
    );
    assert_eq!(global(&entorno, "contenido"), "42");
}

#[test]
fn un_metodo_de_instancia_deberia_referenciarse_como_valor() {
    let entorno = ejecutar(
        "objeto Contador {\n\
             privado:\n\
                 entero var total = 0\n\
             publico:\n\
                 vacio sumar(entero valor) {\n\
                     esto.total = esto.total + valor\n\
                 }\n\
                 entero leer() {\n\
                     retornar esto.total\n\
                 }\n\
         }\n\
         Contador contador = nuevo Contador()\n\
         funcion sumar = contador.sumar\n\
         sumar(5)\n\
         sumar(7)\n\
         entero total = contador.leer()\n",
    );
    assert_eq!(global(&entorno, "total"), "12");
}

#[test]
fn un_metodo_de_instancia_deberia_pasarse_como_callback() {
    let entorno = ejecutar(
        "objeto Saludo {\n\
             privado:\n\
                 texto tratamiento = \"Hola\"\n\
             publico:\n\
                 texto saludar(texto nombre) {\n\
                     retornar esto.tratamiento + \", \" + nombre\n\
                 }\n\
         }\n\
         texto aplicar(funcion accion, texto nombre) {\n\
             retornar accion(nombre)\n\
         }\n\
         Saludo saludo = nuevo Saludo()\n\
         texto mensaje = aplicar(saludo.saludar, \"Ana\")\n",
    );
    assert_eq!(global(&entorno, "mensaje"), "Hola, Ana");
}

#[test]
fn un_metodo_heredado_deberia_enlazarse_con_la_instancia() {
    let entorno = ejecutar(
        "objeto Animal {\n\
             publico:\n\
                 texto var nombre = \"sin nombre\"\n\
                 texto describir() {\n\
                     retornar \"soy \" + esto.nombre\n\
                 }\n\
         }\n\
         objeto Perro hereda Animal {\n\
             publico:\n\
                 Perro(texto nombre) {\n\
                     esto.nombre = nombre\n\
                 }\n\
         }\n\
         Perro perro = nuevo Perro(\"Fido\")\n\
         funcion describir = perro.describir\n\
         texto descripcion = describir()\n",
    );
    assert_eq!(global(&entorno, "descripcion"), "soy Fido");
}

#[test]
fn un_metodo_asincrono_enlazado_deberia_resolverse_con_esperar() {
    let entorno = ejecutar(
        "objeto Servicio {\n\
             privado:\n\
                 entero factor = 3\n\
             publico:\n\
                 asincrono entero calcular(entero valor) {\n\
                     retornar valor * esto.factor\n\
                 }\n\
         }\n\
         Servicio servicio = nuevo Servicio()\n\
         funcion calcular = servicio.calcular\n\
         entero resultado = esperar calcular(7)\n",
    );
    assert_eq!(global(&entorno, "resultado"), "21");
}

#[test]
fn los_ejemplos_de_objetos_deberian_ejecutar() {
    for nombre in ["objetos.qz", "objetos_herencia.qz", "prototipos.qz"] {
        let ruta = format!("{}/../../ejemplos/{nombre}", env!("CARGO_MANIFEST_DIR"));
        let codigo = std::fs::read_to_string(&ruta).expect("ejemplo legible");
        ejecutar(&codigo);
    }
}
