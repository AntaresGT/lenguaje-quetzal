

# Lenguaje Quetzal

  **Un lenguaje de programación interpretado completamente en español**

  [Rust](https://www.rust-lang.org/)
  [Version](https://github.com/AntaresGT/lenguaje-quetzal)
  [VS Code Extension](https://marketplace.visualstudio.com/items?itemName=AntaresGT.lenguaje-quetzal-vscode-extension)
  [Downloads](https://marketplace.visualstudio.com/items?itemName=AntaresGT.lenguaje-quetzal-vscode-extension)



---



## Descripción

**Lenguaje Quetzal** es un lenguaje de programación interpretado diseñado completamente en español, desarrollado en Rust. Su objetivo es hacer la programación más accesible para hispanohablantes mediante una sintaxis natural y palabras clave en español.

> **Misión**: Crear un lenguaje de programación en español y que sea totalmente funcional para propósito general, no únicamente para enseñanza.

> **Visión a corto plazo**: Que Lenguaje Quetzal se convierta en una herramienta capaz de ejecutarse en un servidor para responder peticiones y ejecutar código en tiempo real, facilitando el desarrollo de aplicaciones web y servicios backend.

> **Motivación**: La programación no tiene fronteras, pero sí puede tener raíces. Quetzal busca ofrecer a la comunidad hispanohablante un lenguaje moderno, seguro, eficiente y totalmente en español, permitiendo crear aplicaciones de manera más intuitiva y natural. Además de facilitar la enseñanza de conceptos de programación a quienes tienen el español como lengua materna.

> **Nota**: No veas Quetzal como un lenguaje solo para principiantes, sino como una herramienta para desarrolladores experimentados. En su versión **0.0.2**, Quetzal ya cuenta con intérprete funcional, módulos, excepciones, POO, stdlib (consola, matemática, tiempo, regex, FS, red), REPL y extensión para Visual Studio Code.

Especificación del lenguaje: `[definicion-lenguaje/](./definicion-lenguaje/README.md)`.

### Estado del proyecto

- **Versión**: 0.0.2
- **Implementación**: workspace Rust (~20 000 líneas)
- **Ejemplos**: suites en `ejemplos/` (tipos, módulos, FS, red, POO, async, …)
- **Extensión VS Code**: soporte de desarrollo para archivos `.qz`



### Características principales

- **Sintaxis en español**: keywords y API en español
- **Tipado estático explícito**: sin inferencia; `var` para mutabilidad
- **Inmutable por defecto**: reasignación solo con `var`
- **JSON nativo**: tipo `jsn` sin librerías extra
- **POO**: `objeto` / `prototipo` / `hereda` / `implementa`
- **Módulos**: `importar` / `exportar`
- **Excepciones**: `intentar` / `capturar` / `finalmente` / `lanzar`
- **Async**: `asincrono` / `esperar`
- **Stdlib**: consola, Matemática, Tiempo, ExpresiónRegular, sistema de archivos, red
- **Seguridad**: permisos declarativos en `quetzal.json`
- **REPL interactivo**

---



## Instalación



### Prerrequisitos

- [Rust](https://rustup.rs/) (edición 2024 del workspace)
- Git



### Compilar desde código fuente

```bash
git clone https://github.com/AntaresGT/lenguaje-quetzal.git
cd lenguaje-quetzal

cargo build --release

# Binario: target/release/quetzal (o quetzal.exe en Windows)
./target/release/quetzal ejemplos/hola_mundo/principal.qz
```

También:

```bash
cargo run -p lnc -- ejemplos/tipos.qz
```

---



## Extensión para VS Code

  


La extensión oficial para Visual Studio Code incluye:

- Resaltado de sintaxis para archivos `.qz`
- Autocompletado de palabras clave y funciones
- Detección de errores en tiempo real
- Snippets de código
- Iconos temáticos para archivos Quetzal
- Temas de colores para la sintaxis española



### Instalación rápida

```bash
code --install-extension AntaresGT.lenguaje-quetzal-vscode-extension
```

**[Instalar desde Marketplace](https://marketplace.visualstudio.com/items?itemName=AntaresGT.lenguaje-quetzal-vscode-extension)**

---



## Sintaxis y ejemplos



### Variables y tipos

```qz
// Tipos básicos
entero edad = 25
número precio = 99.99
texto nombre = "María García"
log activo = verdadero

// Mutabilidad explícita
texto var saludo = "hola"
saludo = "adios"

// Listas
lista<entero> numeros = [1, 2, 3, 4, 5]
lista<texto> colores = ["rojo", "verde", "azul"]

// JSON nativo
jsn persona = {
    nombre: "Juan Pérez",
    edad: 30,
    activo: verdadero,
    hobbies: ["programar", "leer", "viajar"],
    direccion: {
        calle: "Calle Falsa 123",
        "ciudad": "Ciudad Ejemplo",
        "pais": "País Ejemplo"
    }
}
```



### Funciones

```qz
entero sumar(entero a, entero b) {
    retornar a + b
}

vacio saludar(texto nombre) {
    consola.mostrar("¡Hola " + nombre + "!")
}

asincrono texto obtener_datos(texto url) {
    // Código asíncrono
    retornar "Datos obtenidos"
}
```



### Control de flujo

```qz
si (edad >= 18) {
    consola.mostrar("Es mayor de edad")
} sino {
    consola.mostrar("Es menor de edad")
}

para (entero i = 1; i <= 10; i++) {
    consola.mostrar("Número: " + i.texto())
}

mientras (condicion) {
    // Código del bucle
}

para (texto color en colores) {
    consola.mostrar("Color: " + color)
}

para (entero numero cada numeros) {
    consola.mostrar("Número: " + numero.texto())
}
```



### Excepciones

```qz
intentar {
    lanzar excepcion("algo falló")
} capturar (excepcion e) {
    consola.mostrar_error(e.mensaje)
} finalmente {
    consola.mostrar("limpieza")
}
```



### Consola

```qz
consola.mostrar("Mensaje normal")
consola.mostrar_error("Error: Algo salió mal")
consola.mostrar_advertencia("Advertencia: Revisa este valor")
consola.mostrar_exito("Operación completada exitosamente")
consola.mostrar_informacion("Información importante")
```



### Ejemplo: calculadora

```qz
texto operacion = "suma"
entero num1 = 10
entero num2 = 5

entero calcular(texto op, entero a, entero b) {
    si (op == "suma") {
        retornar a + b
    } sino si (op == "resta") {
        retornar a - b
    } sino si (op == "multiplicacion") {
        retornar a * b
    } sino si (op == "division") {
        si (b != 0) {
            retornar a / b
        } sino {
            consola.mostrar_error("Error: División por cero")
            retornar 0
        }
    } sino {
        consola.mostrar_advertencia("Operación no válida")
        retornar 0
    }
}

entero resultado = calcular(operacion, num1, num2)
consola.mostrar_exito("Resultado: " + resultado.texto())
```



### JSON

```qz
jsn configuracion = {
    aplicacion: "MiApp",
    version: "1.0.0",
    configuraciones: {
        tema: "oscuro",
        idioma: "español",
        notificaciones: verdadero
    },
    modulos: ["auth", "database", "api"]
}

texto tema = configuracion.configuraciones.tema
consola.mostrar("Tema actual: " + tema)

para (texto modulo en configuracion.modulos) {
    consola.mostrar_informacion("Módulo cargado: " + modulo)
}
```

Más ejemplos en `[ejemplos/](./ejemplos/)`.

---



## Por qué Quetzal


| Característica      | Quetzal | Python | JavaScript |
| ------------------- | ------- | ------ | ---------- |
| Sintaxis en español | Sí      | No     | No         |
| Tipado fuerte       | Sí      | No     | No         |
| Simplicidad         | Sí      | Sí     | No         |
| Async/await         | Sí      | Sí     | Sí         |


---



## Estructura del proyecto

```
lenguaje-quetzal/
├── crates/
│   ├── lnc/                 # CLI binario `quetzal`
│   ├── motor/               # Orquestación del intérprete
│   ├── lexico/              # Lexer
│   ├── sintaxis/            # Parser
│   ├── ast/                 # AST
│   ├── semantica/           # Tipos, imports, permisos
│   ├── bytecode/            # Compilación a bytecode
│   ├── maquina_virtual/     # VM
│   ├── runtime/             # Runtime
│   ├── modulos_nativos/     # Stdlib
│   ├── paquetes/            # Manifiesto y resolución
│   ├── repl/                # REPL
│   ├── lsp/                 # LSP (esqueleto)
│   ├── nucleo/              # Tipos compartidos
│   └── diagnosticos/        # Diagnósticos
├── definicion-lenguaje/     # Especificación del lenguaje
├── ejemplos/                # Programas de ejemplo
├── pruebas/                 # Suites de prueba por crate
├── recursos/imagenes/       # Logos e iconos
└── README.md
```

Detalle de arquitectura: `[definicion-lenguaje/arquitectura.md](./definicion-lenguaje/arquitectura.md)`.

---



## Ejecutar ejemplos

```bash
# Ejemplo simple
cargo run -p lnc -- ejemplos/tipos.qz

# Proyecto con manifiesto quetzal.json
cargo run -p lnc -- ejemplos/hola_mundo/principal.qz
cargo run -p lnc -- ejemplos/modulos/principal.qz
cargo run -p lnc -- ejemplos/red_servidor_rest/principal.qz
```

---



## Hoja de ruta



### v0.0.1

- [x] Intérprete básico en Rust
- [x] Tipos de datos fundamentales
- [x] Funciones definidas por el usuario
- [x] Control de flujo (`si` / `sino`, bucles)
- [x] Soporte JSON nativo
- [x] Extensión VS Code



### v0.0.2 (actual)

- [x] Sistema de módulos (`importar` / `exportar`)
- [x] Manejo de excepciones (`intentar` / `capturar` / `finalmente`)
- [x] Herencia (simple y múltiple)
- [x] Funciones asíncronas
- [x] REPL interactivo
- [x] Stdlib: consola, matemática, tiempo, regex, FS, red
- [x] Manifiesto `quetzal.json` y permisos
- [x] Operadores avanzados



### v0.1.0 (futuro)

- [ ] Gestión de paquetes madura
- [ ] Debugger integrado
- [ ] APIs REST de producción más completas
- [ ] Conexión a bases de datos
- [ ] Herramientas de desarrollo adicionales
- [ ] LSP funcional



### v0.2.0 (futuro lejano)

- [ ] UI de escritorio / móvil (evaluando opciones)
- [ ] Aplicaciones web con Quetzal
- [ ] Computación gráfica avanzada

---



## Contribuir

Las contribuciones son bienvenidas:

1. Haz un fork del repositorio
2. Crea una rama (`git checkout -b nueva-caracteristica`)
3. Realiza tus cambios y pruebas
4. Commit (`git commit -am 'carac: Agregar nueva característica'`)
5. Push (`git push origin nueva-caracteristica`)
6. Abre un Pull Request



### Guías

- Mantén el código en español (comentarios, variables, funciones)
- Sigue las convenciones de nomenclatura del proyecto
- Incluye pruebas para nuevas características
- Actualiza la documentación según sea necesario
- La especificación vive en `definicion-lenguaje/`

---



## Preguntas frecuentes (FAQ)



### ¿Es Quetzal compilado o interpretado?

Quetzal es **interpretado** con validación estática: se chequean sintaxis y tipos antes de ejecutar en la máquina virtual.

### ¿Puedo usar identificadores en inglés?

Sí, los identificadores pueden estar en cualquier idioma si siguen **camelCase** o **snake_case**. Las palabras clave del lenguaje deben estar en español.

### ¿Qué tan rápido es Quetzal?

Al estar implementado en Rust (lexer, parser, semántica, bytecode y VM), ofrece buen rendimiento para scripts, backends y procesamiento de datos.

### ¿Hay librerías / módulos?

Sí. En 0.0.2 existen módulos de usuario (`importar` / `exportar`) y módulos nativos (`quetzal/matemática`, `quetzal/tiempo`, `quetzal/motor`, `quetzal/sistema_archivos`, `quetzal/red`). El ecosistema de paquetes externos sigue madurando hacia 0.1.0.

---



## Contacto y soporte

- **Reportar bugs**: [Issues en GitHub](https://github.com/AntaresGT/lenguaje-quetzal/issues)
- **Solicitar características**: [Feature Requests](https://github.com/AntaresGT/lenguaje-quetzal/issues/new)
- **Discusiones**: [GitHub Discussions](https://github.com/AntaresGT/lenguaje-quetzal/discussions)
- **Email**: [alan@antaresgt.com](mailto:alan@antaresgt.com)
- **Sitio web**: [lenguajequetzal.com](https://lenguajequetzal.com)

---

### Únete a la programación en español

**Hecho para la comunidad hispanohablante**

[Extensión VS Code](https://marketplace.visualstudio.com/items?itemName=AntaresGT.lenguaje-quetzal-vscode-extension) • [Documentación](#sintaxis-y-ejemplos) • [Ejemplos](#ejecutar-ejemplos) • [Contribuir](#contribuir) • [FAQ](#preguntas-frecuentes-faq)



*"La programación no tiene fronteras, pero sí puede tener raíces"*

[Dale una estrella si te gusta el proyecto](https://github.com/AntaresGT/lenguaje-quetzal/stargazers)