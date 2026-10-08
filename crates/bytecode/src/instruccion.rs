//! Conjunto de instrucciones de la máquina virtual de Quetzal.

use nucleo::Ubicacion;

/// Instrucción de la VM. Los índices `u16` de locales/globales y los `u32`
/// de constantes y nombres refieren a las tablas del [`crate::ModuloCompilado`].
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Instruccion {
    // ----- Pila y constantes -----
    CargarConstante(u32),
    Duplicar,
    Desechar,

    // ----- Locales (ranuras del marco) -----
    CargarLocal(u16),
    GuardarLocal(u16),
    /// Pone `nulo` en las ranuras `[desde, ranuras_locales)` para soltar `Rc`.
    LimpiarLocales(u16),

    // ----- Globales del módulo (índice fijo) -----
    CargarGlobal(u16),
    /// Primera escritura de una global (no comprueba `var`).
    DeclararGlobal(u16),
    GuardarGlobal(u16),

    // ----- Nombres no resueltos en el módulo (REPL, `consola`, `rango`) -----
    CargarExterno(u32),
    GuardarExterno(u32),

    // ----- Aritmética y lógica -----
    Sumar,
    Restar,
    Multiplicar,
    Dividir,
    Modulo,
    Negar,
    NoLogico,
    Igual,
    Diferente,
    Mayor,
    Menor,
    MayorOIgual,
    MenorOIgual,

    // ----- Aritmética de entero (ambos operandos son `entero`) -----
    SumarEntero,
    RestarEntero,
    MultiplicarEntero,
    DividirEntero,
    ModuloEntero,
    NegarEntero,
    IgualEntero,
    DiferenteEntero,
    MayorEntero,
    MenorEntero,
    MayorOIgualEntero,
    MenorOIgualEntero,

    // ----- Saltos -----
    Saltar(usize),
    SaltarSiFalso(usize),
    SaltarSiVerdadero(usize),
    /// Cortocircuito `&&`: si el tope es falso salta dejando el valor;
    /// si no, lo desecha y continúa.
    SaltarSiFalsoYDejar(usize),
    /// Cortocircuito `||`: si el tope es verdadero salta dejando el valor.
    SaltarSiVerdaderoYDejar(usize),

    // ----- Llamadas -----
    /// Llama al valor bajo los argumentos: `[funcion, arg1..argN]`.
    Llamar(u32),
    /// Llama un método del objeto bajo los argumentos: `(nombre, n_args)`.
    LlamarMetodo(u32, u32),
    Retornar,

    // ----- Miembros e índices -----
    CargarMiembro(u32),
    /// `[objeto, valor]` → asigna `objeto.miembro = valor`.
    GuardarMiembro(u32),
    /// `[objeto, indice]` → empuja `objeto[indice]`.
    Indexar,
    /// `[objeto, indice, valor]` → asigna `objeto[indice] = valor`.
    GuardarIndice,

    // ----- Estructuras -----
    /// Crea una lista con los N valores del tope.
    CrearLista(u32),
    /// Crea un jsn con N pares `[clave, valor]` del tope (claves constantes).
    CrearJsn(u32),
    /// Concatena N valores del tope convertidos a texto (interpolación).
    Interpolar(u32),

    // ----- Objetos -----
    /// `nuevo Clase(args)`: la clase está bajo los argumentos.
    Instanciar(u32),
    CargarEsto,
    CargarPadre,

    // ----- Iteración -----
    /// Convierte el tope (lista) en un iterador.
    CrearIterador,
    /// `[iterador]` → empuja el siguiente elemento, o salta si terminó.
    IteradorSiguiente(usize),

    // ----- Excepciones -----
    /// Activa un manejador: `(pc, ranuras_locales_al_entrar)`.
    EntrarIntentar(usize, u16),
    SalirIntentar,
    Lanzar,

    // ----- Asincronía -----
    /// Resuelve una tarea pendiente (`esperar`).
    Esperar,
}

/// Secuencia de instrucciones con la ubicación de origen de cada una.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Trozo {
    pub instrucciones: Vec<Instruccion>,
    pub ubicaciones: Vec<Ubicacion>,
}

impl Trozo {
    pub fn emitir(&mut self, instruccion: Instruccion, ubicacion: Ubicacion) -> usize {
        self.instrucciones.push(instruccion);
        self.ubicaciones.push(ubicacion);
        self.instrucciones.len() - 1
    }

    pub fn posicion_actual(&self) -> usize {
        self.instrucciones.len()
    }

    /// Reemplaza el destino de un salto emitido previamente.
    pub fn parchar_salto(&mut self, indice: usize, destino: usize) {
        match &mut self.instrucciones[indice] {
            Instruccion::Saltar(pc)
            | Instruccion::SaltarSiFalso(pc)
            | Instruccion::SaltarSiVerdadero(pc)
            | Instruccion::SaltarSiFalsoYDejar(pc)
            | Instruccion::SaltarSiVerdaderoYDejar(pc)
            | Instruccion::IteradorSiguiente(pc)
            | Instruccion::EntrarIntentar(pc, _) => *pc = destino,
            otra => {
                // Error de programación del generador, no del usuario.
                unreachable!("la instrucción {otra:?} no es un salto parchable")
            }
        }
    }
}
