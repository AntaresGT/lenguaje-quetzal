//! Intérprete de bytecode: pila de valores, marcos y excepciones.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use bytecode::{Constante, FuncionCompilada, Instruccion, ModuloCompilado, ObjetoCompilado, Trozo};
use indexmap::IndexMap;
use nucleo::{ErrorQuetzal, Ubicacion};
use rust_decimal::Decimal;

use crate::bucle_eventos::{BucleEventos, CargaNativa, Mensaje};
use crate::errores::{DatosExcepcion, Fallo};
use crate::nativos::RegistroNativos;
use crate::valores::{
    ClaseObjeto, DatosMetodoEnlazado, DatosTarea, EntornoModulo, EstadoIterador, Instancia,
    MetodoRuntime, Valor, Variable, carga_a_valor, texto_de_valor,
};

/// Límite de marcos anidados (recursión). Vive en el heap, no en la pila de Rust.
const LIMITE_PROFUNDIDAD: usize = 10_000;

/// Despachador genérico de un servicio nativo (por ejemplo `servidor_http`):
/// recibe la VM, el identificador del recurso concreto y los datos de la
/// solicitud, y devuelve la carga de respuesta. Lo registra el módulo nativo
/// correspondiente la primera vez que se usa (ver [`Vm::registrar_despachador`]).
type Despachador = Rc<dyn Fn(&mut Vm, u64, CargaNativa) -> CargaNativa>;

/// Máquina virtual del Lenguaje Quetzal.
pub struct Vm {
    pub nativos: Rc<RegistroNativos>,
    /// Entornos de los módulos cargados, por nombre.
    pub modulos: IndexMap<String, Rc<EntornoModulo>>,
    /// Pila de nombres de funciones en ejecución (para `e.llamadas`).
    pila_llamadas: Vec<String>,
    /// Marcos de activación en el heap. La recursión Quetzal no crece la pila de Rust.
    marcos: Vec<Marco>,
    /// Bucle de eventos: E/S de fondo (tokio) + despacho de solicitudes en
    /// el hilo de la VM. Ver `crate::bucle_eventos`.
    bucle: BucleEventos,
    /// Resultados de tareas nativas que llegaron mientras se esperaba otra
    /// tarea distinta (se guardan para cuando se espere la suya).
    resultados_pendientes: RefCell<HashMap<u64, Result<CargaNativa, String>>>,
    /// Despachadores de solicitudes de servidores/recursos, por servicio.
    despachadores: RefCell<HashMap<String, Despachador>>,
}

/// Código que ejecuta un marco: función compilada o trozo suelto (módulo, inicializador).
enum CodigoMarco {
    Funcion(Rc<FuncionCompilada>),
    Trozo(Rc<Trozo>),
}

impl CodigoMarco {
    fn trozo(&self) -> &Trozo {
        match self {
            Self::Funcion(funcion) => &funcion.trozo,
            Self::Trozo(trozo) => trozo,
        }
    }
}

/// Marco de activación: locales, pila de operandos y manejadores de excepción.
struct Marco {
    codigo: CodigoMarco,
    pc: usize,
    entorno: Rc<EntornoModulo>,
    /// Ranuras locales del marco (parámetros en `0..n`).
    locales: Vec<Valor>,
    esto: Option<Valor>,
    pila: Vec<Valor>,
    manejadores: Vec<Manejador>,
    /// Nombre para `e.llamadas`. `None` = cuerpo de módulo o trozo aislado.
    nombre: Option<String>,
    /// Si está, al retornar se empuja esto en el caller (constructores).
    valor_al_volver: Option<Valor>,
}

/// Resultado de preparar una llamada: valor ya resuelto o marco que hay que empujar.
enum DestinoLlamada {
    Valor(Valor),
    Marco(Marco),
}

/// Manejador de excepciones activo dentro de un marco.
struct Manejador {
    pc: usize,
    profundidad_pila: usize,
    ranuras_locales: u16,
}

impl Vm {
    pub fn nueva(nativos: Rc<RegistroNativos>) -> Self {
        Self {
            nativos,
            modulos: IndexMap::new(),
            pila_llamadas: Vec::new(),
            marcos: Vec::new(),
            bucle: BucleEventos::nuevo(),
            resultados_pendientes: RefCell::new(HashMap::new()),
            despachadores: RefCell::new(HashMap::new()),
        }
    }

    /// El bucle de eventos de esta VM (E/S de fondo y despacho de eventos).
    pub fn bucle(&self) -> &BucleEventos {
        &self.bucle
    }

    /// Registra el despachador de un servicio nativo (por ejemplo
    /// `"servidor_http"`) si todavía no existe uno. Los módulos nativos lo
    /// llaman la primera vez que se usa el recurso correspondiente
    /// (`servidor.escuchar(...)`); el mismo despachador atiende a todos los
    /// recursos de ese servicio, identificándolos por `id_recurso`.
    pub fn registrar_despachador<F>(&self, servicio: impl Into<String>, funcion: F)
    where
        F: Fn(&mut Vm, u64, CargaNativa) -> CargaNativa + 'static,
    {
        self.despachadores
            .borrow_mut()
            .entry(servicio.into())
            .or_insert_with(|| Rc::new(funcion));
    }

    /// Atiende una solicitud entrante despachándola al servicio registrado.
    /// Devuelve `CargaNativa::Nula` si no hay despachador para ese servicio.
    fn despachar_solicitud(
        &mut self,
        servicio: &str,
        id_recurso: u64,
        datos: CargaNativa,
    ) -> CargaNativa {
        let Some(funcion) = self.despachadores.borrow().get(servicio).cloned() else {
            return CargaNativa::Nula;
        };
        funcion(self, id_recurso, datos)
    }

    /// Bombea el bucle de eventos hasta que la tarea nativa `id` se resuelve,
    /// atendiendo mientras tanto cualquier otra solicitud que llegue (por
    /// ejemplo, peticiones entrantes de un servidor en escucha). Así ninguna
    /// espera bloquea al resto del programa.
    fn resolver_tarea_nativa(&mut self, id: u64, ubicacion: Ubicacion) -> Result<Valor, Fallo> {
        if let Some(resultado) = self.resultados_pendientes.borrow_mut().remove(&id) {
            return self.convertir_resultado_tarea(resultado, ubicacion);
        }
        loop {
            match self.bucle.recibir() {
                Some(Mensaje::TareaLista {
                    id: llegado,
                    resultado,
                }) if llegado == id => {
                    return self.convertir_resultado_tarea(resultado, ubicacion);
                }
                Some(Mensaje::TareaLista {
                    id: llegado,
                    resultado,
                }) => {
                    self.resultados_pendientes
                        .borrow_mut()
                        .insert(llegado, resultado);
                }
                Some(Mensaje::Solicitud {
                    servicio,
                    id_recurso,
                    datos,
                    respuesta,
                }) => {
                    let salida = self.despachar_solicitud(&servicio, id_recurso, datos);
                    let _ = respuesta.send(salida);
                }
                None => {
                    return Err(Fallo::Error(Box::new(
                        ErrorQuetzal::interno(
                            "el bucle de eventos se cerró antes de resolver una tarea nativa",
                        )
                        .con_ubicacion(ubicacion),
                    )));
                }
            }
        }
    }

    fn convertir_resultado_tarea(
        &self,
        resultado: Result<CargaNativa, String>,
        ubicacion: Ubicacion,
    ) -> Result<Valor, Fallo> {
        resultado
            .map(carga_a_valor)
            .map_err(|mensaje| self.excepcion("E0702", mensaje, Some(ubicacion)))
    }

    /// Mantiene vivo el bucle de eventos mientras haya servidores u otras
    /// tareas activas registradas (`BucleEventos::registrar_trabajo_activo`),
    /// atendiendo peticiones y tareas hasta que todas se detengan. Lo llama
    /// el motor al terminar el código principal del programa, igual que el
    /// bucle de eventos de Node.js sigue vivo mientras haya servidores
    /// escuchando.
    pub fn drenar_bucle_eventos(&mut self) {
        use std::time::Duration;
        while self.bucle.hay_trabajo_activo() {
            match self.bucle.recibir_con_limite(Duration::from_millis(200)) {
                Some(Mensaje::TareaLista { id, resultado }) => {
                    self.resultados_pendientes
                        .borrow_mut()
                        .insert(id, resultado);
                }
                Some(Mensaje::Solicitud {
                    servicio,
                    id_recurso,
                    datos,
                    respuesta,
                }) => {
                    let salida = self.despachar_solicitud(&servicio, id_recurso, datos);
                    let _ = respuesta.send(salida);
                }
                None => {}
            }
        }
    }

    /// Crea el entorno de un módulo con sus importaciones ya resueltas,
    /// registra funciones y objetos, y ejecuta su código principal.
    ///
    /// Devuelve el entorno y el valor que dejó el código principal en la
    /// pila (útil para que el REPL muestre expresiones sueltas).
    pub fn cargar_modulo(
        &mut self,
        modulo: Rc<ModuloCompilado>,
        importaciones: IndexMap<String, Variable>,
    ) -> Result<(Rc<EntornoModulo>, Valor), ErrorQuetzal> {
        let constantes = modulo.constantes.iter().map(valor_de_constante).collect();
        let entorno = Rc::new(EntornoModulo {
            modulo: Rc::clone(&modulo),
            globales: RefCell::new(IndexMap::new()),
            valores_globales: RefCell::new(Vec::new()),
            constantes,
        });

        let funciones: HashMap<&str, &FuncionCompilada> = modulo
            .funciones
            .iter()
            .map(|funcion| (funcion.nombre.as_str(), funcion))
            .collect();

        let mut importaciones = importaciones;
        let mut valores_globales = Vec::with_capacity(modulo.nombres_globales.len());
        {
            let mut globales = entorno.globales.borrow_mut();
            for (indice, nombre) in modulo.nombres_globales.iter().enumerate() {
                let mutable = modulo
                    .globales_mutables
                    .get(indice)
                    .copied()
                    .unwrap_or(false);
                let valor = if let Some(funcion) = funciones.get(nombre.as_str()) {
                    Valor::Funcion(Rc::new((*funcion).clone()), Rc::clone(&entorno))
                } else if modulo.objetos.iter().any(|objeto| objeto.nombre == *nombre) {
                    Valor::Nulo
                } else if let Some(variable) = importaciones.shift_remove(nombre) {
                    variable.valor
                } else {
                    Valor::Nulo
                };
                let variable = Variable { valor, mutable };
                globales.insert(nombre.clone(), variable.clone());
                valores_globales.push(variable);
            }
            // Símbolos extra (REPL: variables de entradas anteriores).
            for (nombre, variable) in importaciones {
                globales.entry(nombre).or_insert(variable);
            }
        }
        *entorno.valores_globales.borrow_mut() = valores_globales;

        // Objetos del módulo (con sus atributos libres inicializados).
        for objeto in &modulo.objetos {
            let clase = self.crear_clase(objeto, &entorno)?;
            if let Some(variable) = entorno.globales.borrow_mut().get_mut(&objeto.nombre) {
                variable.valor = Valor::Clase(Rc::clone(&clase));
            }
            if let Some(indice) = modulo
                .nombres_globales
                .iter()
                .position(|nombre| nombre == &objeto.nombre)
                && let Some(variable) = entorno.valores_globales.borrow_mut().get_mut(indice)
            {
                variable.valor = Valor::Clase(clase);
            }
        }

        // Código principal del módulo.
        let valor = self
            .correr_trozo(
                Rc::new(modulo.principal.clone()),
                Rc::clone(&entorno),
                vec![Valor::Nulo; modulo.ranuras_locales_principal as usize],
                None,
            )
            .map_err(|fallo| fallo.a_error(&modulo.nombre))?;

        sincronizar_globales(&entorno);

        self.modulos
            .insert(modulo.nombre.clone(), Rc::clone(&entorno));
        Ok((entorno, valor))
    }

    fn crear_clase(
        &mut self,
        objeto: &ObjetoCompilado,
        entorno: &Rc<EntornoModulo>,
    ) -> Result<Rc<ClaseObjeto>, ErrorQuetzal> {
        let metodos = objeto
            .metodos
            .iter()
            .map(|metodo| {
                (
                    metodo.funcion.nombre.clone(),
                    MetodoRuntime {
                        funcion: Rc::new(metodo.funcion.clone()),
                        libre: metodo.libre,
                    },
                )
            })
            .collect();
        let constructores = objeto
            .constructores
            .iter()
            .map(|constructor| Rc::new(constructor.clone()))
            .collect();
        let clase = Rc::new(ClaseObjeto {
            compilado: objeto.clone(),
            entorno: Rc::clone(entorno),
            atributos_libres: RefCell::new(IndexMap::new()),
            metodos,
            constructores,
        });

        for atributo in &objeto.atributos {
            if !atributo.libre {
                continue;
            }
            let valor = match &atributo.inicial {
                Some(trozo) => self
                    .ejecutar_trozo_aislado(trozo, entorno, None)
                    .map_err(|fallo| fallo.a_error(&entorno.modulo.nombre))?,
                None => Valor::Nulo,
            };
            clase.atributos_libres.borrow_mut().insert(
                atributo.nombre.clone(),
                Variable {
                    valor,
                    mutable: atributo.mutable,
                },
            );
        }

        Ok(clase)
    }

    /// Ejecuta un trozo en un marco nuevo (valores iniciales, constructores).
    fn ejecutar_trozo_aislado(
        &mut self,
        trozo: &Trozo,
        entorno: &Rc<EntornoModulo>,
        esto: Option<Valor>,
    ) -> Result<Valor, Fallo> {
        self.correr_trozo(Rc::new(trozo.clone()), Rc::clone(entorno), Vec::new(), esto)
    }

    /// Llama una función de Quetzal con sus argumentos.
    pub fn llamar_funcion(
        &mut self,
        funcion: &Rc<FuncionCompilada>,
        entorno: &Rc<EntornoModulo>,
        argumentos: Vec<Valor>,
        esto: Option<Valor>,
        ubicacion: Option<Ubicacion>,
    ) -> Result<Valor, Fallo> {
        let marco = self.preparar_marco_funcion(funcion, entorno, argumentos, esto, ubicacion)?;
        self.entrar_marco(marco, ubicacion)
    }

    /// Resuelve un valor `esperar`: si es una [`Valor::Tarea`] (función
    /// Quetzal `asincrono`) la ejecuta; si es una [`Valor::TareaNativa`]
    /// (E/S en el bucle de eventos) bombea el bucle hasta que se resuelve;
    /// cualquier otro valor se devuelve sin cambios. Es la implementación de
    /// la instrucción `Esperar`, expuesta también para que los módulos
    /// nativos (y las pruebas) puedan esperar tareas fuera del bytecode.
    pub fn esperar(&mut self, valor: Valor, ubicacion: Ubicacion) -> Result<Valor, Fallo> {
        match self.destino_esperar(valor, ubicacion)? {
            DestinoLlamada::Valor(valor) => Ok(valor),
            DestinoLlamada::Marco(marco) => self.entrar_marco(marco, Some(ubicacion)),
        }
    }

    fn destino_esperar(
        &mut self,
        valor: Valor,
        ubicacion: Ubicacion,
    ) -> Result<DestinoLlamada, Fallo> {
        match valor {
            Valor::Tarea(tarea) => Ok(DestinoLlamada::Marco(self.preparar_marco_funcion(
                &tarea.funcion,
                &tarea.entorno,
                tarea.argumentos.clone(),
                tarea.instancia.clone(),
                Some(ubicacion),
            )?)),
            Valor::TareaNativa(tarea) => Ok(DestinoLlamada::Valor(
                self.resolver_tarea_nativa(tarea.id, ubicacion)?,
            )),
            otro => Ok(DestinoLlamada::Valor(otro)),
        }
    }

    fn preparar_marco_funcion(
        &self,
        funcion: &Rc<FuncionCompilada>,
        entorno: &Rc<EntornoModulo>,
        argumentos: Vec<Valor>,
        esto: Option<Valor>,
        ubicacion: Option<Ubicacion>,
    ) -> Result<Marco, Fallo> {
        if argumentos.len() != funcion.parametros.len() {
            return Err(self.excepcion(
                "E0210",
                format!(
                    "la función '{}' espera {} argumentos, pero recibió {}",
                    funcion.nombre,
                    funcion.parametros.len(),
                    argumentos.len()
                ),
                ubicacion,
            ));
        }

        let ranuras = (funcion.ranuras_locales as usize).max(argumentos.len());
        let mut locales = vec![Valor::Nulo; ranuras];
        for (indice, valor) in argumentos.into_iter().enumerate() {
            locales[indice] = valor;
        }

        Ok(Marco {
            codigo: CodigoMarco::Funcion(Rc::clone(funcion)),
            pc: 0,
            entorno: Rc::clone(entorno),
            locales,
            esto,
            pila: Vec::new(),
            manejadores: Vec::new(),
            nombre: Some(funcion.nombre.clone()),
            valor_al_volver: None,
        })
    }

    fn error_desborde_pila(&self, ubicacion: Option<Ubicacion>) -> Fallo {
        self.excepcion(
            "E0409",
            format!(
                "se alcanzó el límite de {LIMITE_PROFUNDIDAD} llamadas anidadas (¿recursión infinita?)"
            ),
            ubicacion,
        )
    }

    fn entrar_marco(&mut self, marco: Marco, ubicacion: Option<Ubicacion>) -> Result<Valor, Fallo> {
        if self.pila_llamadas.len() >= LIMITE_PROFUNDIDAD {
            return Err(self.error_desborde_pila(ubicacion));
        }
        let base = self.marcos.len();
        if let Some(ref nombre) = marco.nombre {
            self.pila_llamadas.push(nombre.clone());
        }
        self.marcos.push(marco);
        self.correr_hasta(base)
    }

    fn correr_trozo(
        &mut self,
        trozo: Rc<Trozo>,
        entorno: Rc<EntornoModulo>,
        locales: Vec<Valor>,
        esto: Option<Valor>,
    ) -> Result<Valor, Fallo> {
        let base = self.marcos.len();
        self.marcos.push(Marco {
            codigo: CodigoMarco::Trozo(trozo),
            pc: 0,
            entorno,
            locales,
            esto,
            pila: Vec::new(),
            manejadores: Vec::new(),
            nombre: None,
            valor_al_volver: None,
        });
        self.correr_hasta(base)
    }

    fn excepcion(&self, codigo: &str, mensaje: String, ubicacion: Option<Ubicacion>) -> Fallo {
        Fallo::Excepcion(DatosExcepcion {
            mensaje,
            llamadas: self.pila_llamadas.clone(),
            codigo: Some(codigo.to_string()),
            ubicacion,
        })
    }

    // ----- Bucle principal del intérprete -----

    fn correr_hasta(&mut self, base: usize) -> Result<Valor, Fallo> {
        loop {
            let mut marco = self.marcos.pop().ok_or_else(|| {
                Fallo::Error(Box::new(ErrorQuetzal::interno(
                    "pila de marcos vacía (bytecode inválido)",
                )))
            })?;

            if marco.pc >= marco.codigo.trozo().instrucciones.len() {
                let valor = marco.pila.pop().unwrap_or(Valor::Nulo);
                if let Some(valor) = self.completar_retorno(base, marco, valor) {
                    return Ok(valor);
                }
                continue;
            }

            let instruccion = marco.codigo.trozo().instrucciones[marco.pc].clone();
            let ubicacion = marco.codigo.trozo().ubicaciones[marco.pc];
            let resultado = self.ejecutar_instruccion(&instruccion, ubicacion, &mut marco);

            match resultado {
                Ok(Flujo::Continuar) => {
                    marco.pc += 1;
                    self.marcos.push(marco);
                }
                Ok(Flujo::SinAvanzar) => {
                    self.marcos.push(marco);
                }
                Ok(Flujo::Retornar(valor)) => {
                    if let Some(valor) = self.completar_retorno(base, marco, valor) {
                        return Ok(valor);
                    }
                }
                Ok(Flujo::Llamar(callee)) => {
                    if self.pila_llamadas.len() >= LIMITE_PROFUNDIDAD {
                        let datos = match self.error_desborde_pila(Some(ubicacion)) {
                            Fallo::Excepcion(datos) => datos,
                            otro => {
                                self.descartar_hasta(base, Some(marco));
                                return Err(otro);
                            }
                        };
                        self.propagar_excepcion(base, marco, datos, ubicacion)?;
                        continue;
                    }
                    marco.pc += 1;
                    if let Some(ref nombre) = callee.nombre {
                        self.pila_llamadas.push(nombre.clone());
                    }
                    self.marcos.push(marco);
                    self.marcos.push(callee);
                }
                Err(Fallo::Excepcion(datos)) => {
                    self.propagar_excepcion(base, marco, datos, ubicacion)?;
                }
                Err(otro) => {
                    self.descartar_hasta(base, Some(marco));
                    return Err(otro);
                }
            }
        }
    }

    fn completar_retorno(&mut self, base: usize, marco: Marco, valor: Valor) -> Option<Valor> {
        let valor = marco.valor_al_volver.unwrap_or(valor);
        if marco.nombre.is_some() {
            self.pila_llamadas.pop();
        }
        if self.marcos.len() == base {
            Some(valor)
        } else if let Some(caller) = self.marcos.last_mut() {
            caller.pila.push(valor);
            None
        } else {
            Some(valor)
        }
    }

    fn propagar_excepcion(
        &mut self,
        base: usize,
        mut marco: Marco,
        mut datos: DatosExcepcion,
        ubicacion: Ubicacion,
    ) -> Result<(), Fallo> {
        if datos.ubicacion.is_none() {
            datos.ubicacion = Some(ubicacion);
        }
        loop {
            if let Some(manejador) = marco.manejadores.pop() {
                marco.pila.truncate(manejador.profundidad_pila);
                limpiar_locales(&mut marco.locales, manejador.ranuras_locales);
                marco.pila.push(Valor::Excepcion(Rc::new(datos)));
                marco.pc = manejador.pc;
                self.marcos.push(marco);
                return Ok(());
            }
            if marco.nombre.is_some() {
                self.pila_llamadas.pop();
            }
            if self.marcos.len() == base {
                return Err(Fallo::Excepcion(datos));
            }
            marco = self.marcos.pop().ok_or_else(|| {
                Fallo::Error(Box::new(ErrorQuetzal::interno(
                    "pila de marcos vacía al propagar excepción",
                )))
            })?;
        }
    }

    fn descartar_hasta(&mut self, base: usize, actual: Option<Marco>) {
        if let Some(marco) = actual
            && marco.nombre.is_some()
        {
            self.pila_llamadas.pop();
        }
        while self.marcos.len() > base {
            if let Some(marco) = self.marcos.pop()
                && marco.nombre.is_some()
            {
                self.pila_llamadas.pop();
            }
        }
    }

    fn aplicar_destino(pila: &mut Vec<Valor>, destino: DestinoLlamada) -> Flujo {
        match destino {
            DestinoLlamada::Valor(valor) => {
                pila.push(valor);
                Flujo::Continuar
            }
            DestinoLlamada::Marco(marco) => Flujo::Llamar(marco),
        }
    }

    fn ejecutar_instruccion(
        &mut self,
        instruccion: &Instruccion,
        ubicacion: Ubicacion,
        marco: &mut Marco,
    ) -> Result<Flujo, Fallo> {
        match instruccion {
            Instruccion::CargarConstante(indice) => {
                let constante = marco.entorno.constantes[*indice as usize].clone();
                marco.pila.push(constante);
            }
            Instruccion::Duplicar => {
                let tope = self.tope(&marco.pila, ubicacion)?.clone();
                marco.pila.push(tope);
            }
            Instruccion::Desechar => {
                self.sacar(&mut marco.pila, ubicacion)?;
            }

            Instruccion::CargarLocal(ranura) => {
                let valor = marco
                    .locales
                    .get(*ranura as usize)
                    .cloned()
                    .ok_or_else(|| {
                        Fallo::Error(Box::new(ErrorQuetzal::interno(
                            "ranura local fuera de rango (bytecode inválido)",
                        )))
                    })?;
                marco.pila.push(valor);
            }
            Instruccion::GuardarLocal(ranura) => {
                let valor = self.sacar(&mut marco.pila, ubicacion)?;
                let ranura = *ranura as usize;
                if ranura >= marco.locales.len() {
                    return Err(Fallo::Error(Box::new(ErrorQuetzal::interno(
                        "ranura local fuera de rango (bytecode inválido)",
                    ))));
                }
                marco.locales[ranura] = valor;
            }
            Instruccion::LimpiarLocales(desde) => {
                limpiar_locales(&mut marco.locales, *desde);
            }

            Instruccion::CargarGlobal(indice) => {
                let valor = self.cargar_global(marco, *indice, ubicacion)?;
                marco.pila.push(valor);
            }
            Instruccion::DeclararGlobal(indice) => {
                let valor = self.sacar(&mut marco.pila, ubicacion)?;
                self.declarar_global(marco, *indice, valor, ubicacion)?;
            }
            Instruccion::GuardarGlobal(indice) => {
                let valor = self.sacar(&mut marco.pila, ubicacion)?;
                self.guardar_global(marco, *indice, valor, ubicacion)?;
            }
            Instruccion::CargarExterno(indice) => {
                let nombre = marco.entorno.modulo.nombres[*indice as usize].clone();
                let valor = self.buscar_externo(marco, &nombre).ok_or_else(|| {
                    self.excepcion(
                        "E0201",
                        format!("'{nombre}' no está definido"),
                        Some(ubicacion),
                    )
                })?;
                marco.pila.push(valor);
            }
            Instruccion::GuardarExterno(indice) => {
                let valor = self.sacar(&mut marco.pila, ubicacion)?;
                let nombre = marco.entorno.modulo.nombres[*indice as usize].clone();
                self.guardar_externo(marco, &nombre, valor, ubicacion)?;
            }

            Instruccion::Sumar
            | Instruccion::Restar
            | Instruccion::Multiplicar
            | Instruccion::Dividir
            | Instruccion::Modulo
            | Instruccion::Igual
            | Instruccion::Diferente
            | Instruccion::Mayor
            | Instruccion::Menor
            | Instruccion::MayorOIgual
            | Instruccion::MenorOIgual => {
                let derecha = self.sacar(&mut marco.pila, ubicacion)?;
                let izquierda = self.sacar(&mut marco.pila, ubicacion)?;
                marco.pila.push(self.operacion_binaria(
                    instruccion,
                    izquierda,
                    derecha,
                    ubicacion,
                )?);
            }
            Instruccion::SumarEntero
            | Instruccion::RestarEntero
            | Instruccion::MultiplicarEntero
            | Instruccion::DividirEntero
            | Instruccion::ModuloEntero
            | Instruccion::IgualEntero
            | Instruccion::DiferenteEntero
            | Instruccion::MayorEntero
            | Instruccion::MenorEntero
            | Instruccion::MayorOIgualEntero
            | Instruccion::MenorOIgualEntero => {
                let derecha = self.sacar(&mut marco.pila, ubicacion)?;
                let izquierda = self.sacar(&mut marco.pila, ubicacion)?;
                marco.pila.push(self.operacion_entera(
                    instruccion,
                    izquierda,
                    derecha,
                    ubicacion,
                )?);
            }
            Instruccion::Negar => {
                let valor = self.sacar(&mut marco.pila, ubicacion)?;
                marco.pila.push(self.negar(valor, ubicacion)?);
            }
            Instruccion::NegarEntero => {
                let valor = self.sacar(&mut marco.pila, ubicacion)?;
                marco.pila.push(self.negar_entero(valor, ubicacion)?);
            }
            Instruccion::NoLogico => {
                let valor = self.sacar(&mut marco.pila, ubicacion)?;
                let logico = self.como_logico(&valor, ubicacion)?;
                marco.pila.push(Valor::Log(!logico));
            }

            Instruccion::Saltar(destino) => {
                marco.pc = *destino;
                return Ok(Flujo::SinAvanzar);
            }
            Instruccion::SaltarSiFalso(destino) => {
                let valor = self.sacar(&mut marco.pila, ubicacion)?;
                if !self.como_logico(&valor, ubicacion)? {
                    marco.pc = *destino;
                    return Ok(Flujo::SinAvanzar);
                }
            }
            Instruccion::SaltarSiVerdadero(destino) => {
                let valor = self.sacar(&mut marco.pila, ubicacion)?;
                if self.como_logico(&valor, ubicacion)? {
                    marco.pc = *destino;
                    return Ok(Flujo::SinAvanzar);
                }
            }
            Instruccion::SaltarSiFalsoYDejar(destino) => {
                let valor = self.tope(&marco.pila, ubicacion)?.clone();
                if !self.como_logico(&valor, ubicacion)? {
                    marco.pc = *destino;
                    return Ok(Flujo::SinAvanzar);
                }
                self.sacar(&mut marco.pila, ubicacion)?;
            }
            Instruccion::SaltarSiVerdaderoYDejar(destino) => {
                let valor = self.tope(&marco.pila, ubicacion)?.clone();
                if self.como_logico(&valor, ubicacion)? {
                    marco.pc = *destino;
                    return Ok(Flujo::SinAvanzar);
                }
                self.sacar(&mut marco.pila, ubicacion)?;
            }

            Instruccion::Llamar(cantidad) => {
                let argumentos =
                    self.sacar_varios(&mut marco.pila, *cantidad as usize, ubicacion)?;
                let objetivo = self.sacar(&mut marco.pila, ubicacion)?;
                let destino = self.llamar_valor(objetivo, argumentos, ubicacion)?;
                return Ok(Self::aplicar_destino(&mut marco.pila, destino));
            }
            Instruccion::LlamarMetodo(indice_nombre, cantidad) => {
                let nombre = marco.entorno.modulo.nombres[*indice_nombre as usize].clone();
                let argumentos =
                    self.sacar_varios(&mut marco.pila, *cantidad as usize, ubicacion)?;
                let receptor = self.sacar(&mut marco.pila, ubicacion)?;
                let destino = self.llamar_metodo(receptor, &nombre, argumentos, ubicacion)?;
                return Ok(Self::aplicar_destino(&mut marco.pila, destino));
            }
            Instruccion::Retornar => {
                let valor = marco.pila.pop().unwrap_or(Valor::Nulo);
                return Ok(Flujo::Retornar(valor));
            }

            Instruccion::CargarMiembro(indice) => {
                let nombre = marco.entorno.modulo.nombres[*indice as usize].clone();
                let objeto = self.sacar(&mut marco.pila, ubicacion)?;
                marco
                    .pila
                    .push(self.cargar_miembro(objeto, &nombre, ubicacion)?);
            }
            Instruccion::GuardarMiembro(indice) => {
                let valor = self.sacar(&mut marco.pila, ubicacion)?;
                let objeto = self.sacar(&mut marco.pila, ubicacion)?;
                let nombre = marco.entorno.modulo.nombres[*indice as usize].clone();
                self.guardar_miembro(objeto, &nombre, valor, ubicacion)?;
            }
            Instruccion::Indexar => {
                let indice = self.sacar(&mut marco.pila, ubicacion)?;
                let objeto = self.sacar(&mut marco.pila, ubicacion)?;
                marco.pila.push(self.indexar(objeto, indice, ubicacion)?);
            }
            Instruccion::GuardarIndice => {
                let valor = self.sacar(&mut marco.pila, ubicacion)?;
                let indice = self.sacar(&mut marco.pila, ubicacion)?;
                let objeto = self.sacar(&mut marco.pila, ubicacion)?;
                self.guardar_indice(objeto, indice, valor, ubicacion)?;
            }

            Instruccion::CrearLista(cantidad) => {
                let elementos =
                    self.sacar_varios(&mut marco.pila, *cantidad as usize, ubicacion)?;
                marco.pila.push(Valor::lista(elementos));
            }
            Instruccion::CrearJsn(cantidad) => {
                let pares =
                    self.sacar_varios(&mut marco.pila, (*cantidad as usize) * 2, ubicacion)?;
                let mut mapa = IndexMap::new();
                for par in pares.chunks(2) {
                    let clave = texto_de_valor(&par[0]);
                    mapa.insert(clave, par[1].clone());
                }
                marco.pila.push(Valor::jsn(mapa));
            }
            Instruccion::Interpolar(cantidad) => {
                let partes = self.sacar_varios(&mut marco.pila, *cantidad as usize, ubicacion)?;
                let texto: String = partes.iter().map(texto_de_valor).collect();
                marco.pila.push(Valor::texto(texto));
            }

            Instruccion::Instanciar(cantidad) => {
                let argumentos =
                    self.sacar_varios(&mut marco.pila, *cantidad as usize, ubicacion)?;
                let valor_clase = self.sacar(&mut marco.pila, ubicacion)?;
                match valor_clase {
                    Valor::Clase(clase) => {
                        let destino = self.instanciar(&clase, argumentos, ubicacion)?;
                        return Ok(Self::aplicar_destino(&mut marco.pila, destino));
                    }
                    // Objetos nativos (`nuevo Tiempo(...)`): el módulo registra
                    // su constructor con la clave `{modulo}.constructor`.
                    Valor::ModuloNativo(modulo) => {
                        let clave = format!("{modulo}.constructor");
                        if !self.nativos.existe_funcion(&clave) {
                            return Err(self.excepcion(
                                "E0406",
                                format!("'{modulo}' no es un objeto instanciable"),
                                Some(ubicacion),
                            ));
                        }
                        marco
                            .pila
                            .push(self.llamar_nativa(&clave, &argumentos, ubicacion)?);
                    }
                    otro => {
                        return Err(self.excepcion(
                            "E0406",
                            format!(
                                "un valor de tipo '{}' no es un objeto instanciable",
                                otro.nombre_tipo()
                            ),
                            Some(ubicacion),
                        ));
                    }
                }
            }
            Instruccion::CargarEsto => {
                let esto = marco.esto.clone().ok_or_else(|| {
                    self.excepcion(
                        "E0212",
                        "'esto' solo existe dentro de un objeto".to_string(),
                        Some(ubicacion),
                    )
                })?;
                marco.pila.push(esto);
            }
            Instruccion::CargarPadre => {
                let esto = marco.esto.clone().ok_or_else(|| {
                    self.excepcion(
                        "E0212",
                        "'padre' solo existe dentro de un objeto".to_string(),
                        Some(ubicacion),
                    )
                })?;
                let Valor::Instancia(instancia) = esto else {
                    return Err(self.excepcion(
                        "E0212",
                        "'padre' solo existe dentro de una instancia".to_string(),
                        Some(ubicacion),
                    ));
                };
                marco.pila.push(Valor::Padre {
                    instancia,
                    clase: None,
                });
            }

            Instruccion::CrearIterador => {
                let valor = self.sacar(&mut marco.pila, ubicacion)?;
                let elementos = match &valor {
                    Valor::Lista(lista) => lista.borrow().clone(),
                    Valor::Jsn(mapa) => mapa.borrow().keys().map(Valor::texto).collect(),
                    otro => {
                        return Err(self.excepcion(
                            "E0406",
                            format!(
                                "no se puede iterar un valor de tipo '{}'",
                                otro.nombre_tipo()
                            ),
                            Some(ubicacion),
                        ));
                    }
                };
                marco
                    .pila
                    .push(Valor::Iterador(Rc::new(RefCell::new(EstadoIterador {
                        elementos,
                        posicion: 0,
                    }))));
            }
            Instruccion::IteradorSiguiente(destino) => {
                let valor = self.sacar(&mut marco.pila, ubicacion)?;
                let Valor::Iterador(iterador) = valor else {
                    return Err(self.excepcion(
                        "E0406",
                        "se esperaba un iterador".to_string(),
                        Some(ubicacion),
                    ));
                };
                let mut estado = iterador.borrow_mut();
                if estado.posicion < estado.elementos.len() {
                    let elemento = estado.elementos[estado.posicion].clone();
                    estado.posicion += 1;
                    marco.pila.push(elemento);
                } else {
                    marco.pc = *destino;
                    return Ok(Flujo::SinAvanzar);
                }
            }

            Instruccion::EntrarIntentar(pc_manejador, ranuras) => {
                marco.manejadores.push(Manejador {
                    pc: *pc_manejador,
                    profundidad_pila: marco.pila.len(),
                    ranuras_locales: *ranuras,
                });
            }
            Instruccion::SalirIntentar => {
                marco.manejadores.pop();
            }
            Instruccion::Lanzar => {
                let valor = self.sacar(&mut marco.pila, ubicacion)?;
                let datos = match valor {
                    Valor::Excepcion(datos) => (*datos).clone(),
                    otro => DatosExcepcion {
                        mensaje: texto_de_valor(&otro),
                        llamadas: self.pila_llamadas.clone(),
                        codigo: None,
                        ubicacion: Some(ubicacion),
                    },
                };
                return Err(Fallo::Excepcion(datos));
            }

            Instruccion::Esperar => {
                let valor = self.sacar(&mut marco.pila, ubicacion)?;
                let destino = self.destino_esperar(valor, ubicacion)?;
                return Ok(Self::aplicar_destino(&mut marco.pila, destino));
            }
        }
        Ok(Flujo::Continuar)
    }

    // ----- Variables -----

    fn cargar_global(
        &self,
        marco: &Marco,
        indice: u16,
        ubicacion: Ubicacion,
    ) -> Result<Valor, Fallo> {
        marco
            .entorno
            .valores_globales
            .borrow()
            .get(indice as usize)
            .map(|variable| variable.valor.clone())
            .ok_or_else(|| {
                self.excepcion(
                    "E0201",
                    "la global no está definida".to_string(),
                    Some(ubicacion),
                )
            })
    }

    fn declarar_global(
        &self,
        marco: &Marco,
        indice: u16,
        valor: Valor,
        ubicacion: Ubicacion,
    ) -> Result<(), Fallo> {
        let mut valores = marco.entorno.valores_globales.borrow_mut();
        let variable = valores.get_mut(indice as usize).ok_or_else(|| {
            self.excepcion(
                "E0201",
                "la global no está definida".to_string(),
                Some(ubicacion),
            )
        })?;
        variable.valor = valor;
        Ok(())
    }

    fn guardar_global(
        &self,
        marco: &Marco,
        indice: u16,
        valor: Valor,
        ubicacion: Ubicacion,
    ) -> Result<(), Fallo> {
        let mut valores = marco.entorno.valores_globales.borrow_mut();
        let variable = valores.get_mut(indice as usize).ok_or_else(|| {
            self.excepcion(
                "E0201",
                "la global no está definida".to_string(),
                Some(ubicacion),
            )
        })?;
        if !variable.mutable {
            let nombre = marco
                .entorno
                .modulo
                .nombres_globales
                .get(indice as usize)
                .map(String::as_str)
                .unwrap_or("?");
            return Err(self.error_constante(nombre, ubicacion));
        }
        variable.valor = valor;
        Ok(())
    }

    fn buscar_externo(&self, marco: &Marco, nombre: &str) -> Option<Valor> {
        if let Some(indice) = marco
            .entorno
            .modulo
            .nombres_globales
            .iter()
            .position(|global| global == nombre)
            && let Some(variable) = marco.entorno.valores_globales.borrow().get(indice)
        {
            return Some(variable.valor.clone());
        }
        if let Some(variable) = marco.entorno.globales.borrow().get(nombre) {
            return Some(variable.valor.clone());
        }
        if self.nativos.buscar_funcion(nombre).is_some() {
            return Some(Valor::Nativa(Rc::from(nombre)));
        }
        if self.nativos.existe_modulo(nombre) {
            return Some(Valor::ModuloNativo(Rc::from(nombre)));
        }
        None
    }

    fn guardar_externo(
        &self,
        marco: &Marco,
        nombre: &str,
        valor: Valor,
        ubicacion: Ubicacion,
    ) -> Result<(), Fallo> {
        let mut globales = marco.entorno.globales.borrow_mut();
        if let Some(variable) = globales.get_mut(nombre) {
            if !variable.mutable {
                return Err(self.error_constante(nombre, ubicacion));
            }
            variable.valor = valor;
            return Ok(());
        }
        Err(self.excepcion(
            "E0201",
            format!("'{nombre}' no está definido"),
            Some(ubicacion),
        ))
    }

    fn error_constante(&self, nombre: &str, ubicacion: Ubicacion) -> Fallo {
        self.excepcion(
            "E0203",
            format!("no puedes reasignar la constante '{nombre}'"),
            Some(ubicacion),
        )
    }

    // ----- Pila -----

    fn sacar(&self, pila: &mut Vec<Valor>, ubicacion: Ubicacion) -> Result<Valor, Fallo> {
        pila.pop().ok_or_else(|| {
            Fallo::Error(Box::new(
                ErrorQuetzal::interno("la pila de la VM quedó vacía (bytecode inválido)")
                    .con_ubicacion(ubicacion),
            ))
        })
    }

    fn tope<'a>(&self, pila: &'a [Valor], ubicacion: Ubicacion) -> Result<&'a Valor, Fallo> {
        pila.last().ok_or_else(|| {
            Fallo::Error(Box::new(
                ErrorQuetzal::interno("la pila de la VM quedó vacía (bytecode inválido)")
                    .con_ubicacion(ubicacion),
            ))
        })
    }

    fn sacar_varios(
        &self,
        pila: &mut Vec<Valor>,
        cantidad: usize,
        ubicacion: Ubicacion,
    ) -> Result<Vec<Valor>, Fallo> {
        if pila.len() < cantidad {
            return Err(Fallo::Error(Box::new(
                ErrorQuetzal::interno("la pila de la VM quedó vacía (bytecode inválido)")
                    .con_ubicacion(ubicacion),
            )));
        }
        Ok(pila.split_off(pila.len() - cantidad))
    }

    fn como_logico(&self, valor: &Valor, ubicacion: Ubicacion) -> Result<bool, Fallo> {
        match valor {
            Valor::Log(valor_logico) => Ok(*valor_logico),
            Valor::Nulo => Ok(false),
            otro => Err(self.excepcion(
                "E0406",
                format!(
                    "se esperaba un valor lógico, pero se encontró '{}'",
                    otro.nombre_tipo()
                ),
                Some(ubicacion),
            )),
        }
    }

    // ----- Operaciones binarias -----

    fn operacion_binaria(
        &self,
        instruccion: &Instruccion,
        izquierda: Valor,
        derecha: Valor,
        ubicacion: Ubicacion,
    ) -> Result<Valor, Fallo> {
        match instruccion {
            Instruccion::Sumar => self.sumar(izquierda, derecha, ubicacion),
            Instruccion::Restar => self.aritmetica(izquierda, derecha, ubicacion, "restar"),
            Instruccion::Multiplicar => {
                self.aritmetica(izquierda, derecha, ubicacion, "multiplicar")
            }
            Instruccion::Dividir => self.aritmetica(izquierda, derecha, ubicacion, "dividir"),
            Instruccion::Modulo => self.aritmetica(izquierda, derecha, ubicacion, "modulo"),
            Instruccion::Igual => Ok(Valor::Log(izquierda.es_igual(&derecha))),
            Instruccion::Diferente => Ok(Valor::Log(!izquierda.es_igual(&derecha))),
            Instruccion::Mayor
            | Instruccion::Menor
            | Instruccion::MayorOIgual
            | Instruccion::MenorOIgual => {
                let orden = self.comparar(&izquierda, &derecha, ubicacion)?;
                let resultado = match instruccion {
                    Instruccion::Mayor => orden == std::cmp::Ordering::Greater,
                    Instruccion::Menor => orden == std::cmp::Ordering::Less,
                    Instruccion::MayorOIgual => orden != std::cmp::Ordering::Less,
                    Instruccion::MenorOIgual => orden != std::cmp::Ordering::Greater,
                    _ => unreachable!(),
                };
                Ok(Valor::Log(resultado))
            }
            _ => unreachable!("instrucción binaria inesperada"),
        }
    }

    fn operacion_entera(
        &self,
        instruccion: &Instruccion,
        izquierda: Valor,
        derecha: Valor,
        ubicacion: Ubicacion,
    ) -> Result<Valor, Fallo> {
        let Valor::Entero(a) = izquierda else {
            return Err(self.error_se_esperaba_entero(&izquierda, ubicacion));
        };
        let Valor::Entero(b) = derecha else {
            return Err(self.error_se_esperaba_entero(&derecha, ubicacion));
        };
        match instruccion {
            Instruccion::IgualEntero => Ok(Valor::Log(a == b)),
            Instruccion::DiferenteEntero => Ok(Valor::Log(a != b)),
            Instruccion::MayorEntero => Ok(Valor::Log(a > b)),
            Instruccion::MenorEntero => Ok(Valor::Log(a < b)),
            Instruccion::MayorOIgualEntero => Ok(Valor::Log(a >= b)),
            Instruccion::MenorOIgualEntero => Ok(Valor::Log(a <= b)),
            _ => {
                let nombre = match instruccion {
                    Instruccion::SumarEntero => "sumar",
                    Instruccion::RestarEntero => "restar",
                    Instruccion::MultiplicarEntero => "multiplicar",
                    Instruccion::DividirEntero => "dividir",
                    Instruccion::ModuloEntero => "modulo",
                    _ => unreachable!("instrucción entera inesperada"),
                };
                self.aritmetica(Valor::Entero(a), Valor::Entero(b), ubicacion, nombre)
            }
        }
    }

    fn error_se_esperaba_entero(&self, valor: &Valor, ubicacion: Ubicacion) -> Fallo {
        self.excepcion(
            "E0406",
            format!(
                "se esperaba un entero, pero se encontró '{}'",
                valor.nombre_tipo()
            ),
            Some(ubicacion),
        )
    }

    fn negar(&self, valor: Valor, ubicacion: Ubicacion) -> Result<Valor, Fallo> {
        match valor {
            Valor::Entero(entero) => self.negar_entero(Valor::Entero(entero), ubicacion),
            Valor::Numero(decimal) => Ok(Valor::Numero(-decimal)),
            otro => Err(self.excepcion(
                "E0406",
                format!(
                    "no se puede negar un valor de tipo '{}'",
                    otro.nombre_tipo()
                ),
                Some(ubicacion),
            )),
        }
    }

    fn negar_entero(&self, valor: Valor, ubicacion: Ubicacion) -> Result<Valor, Fallo> {
        match valor {
            Valor::Entero(entero) => entero.checked_neg().map(Valor::Entero).ok_or_else(|| {
                self.excepcion(
                    "E0402",
                    "desbordamiento al negar el entero".to_string(),
                    Some(ubicacion),
                )
            }),
            otro => Err(self.error_se_esperaba_entero(&otro, ubicacion)),
        }
    }

    fn sumar(
        &self,
        izquierda: Valor,
        derecha: Valor,
        ubicacion: Ubicacion,
    ) -> Result<Valor, Fallo> {
        // La suma con texto concatena.
        if matches!(izquierda, Valor::Texto(_)) || matches!(derecha, Valor::Texto(_)) {
            let mut texto = texto_de_valor(&izquierda);
            texto.push_str(&texto_de_valor(&derecha));
            return Ok(Valor::texto(texto));
        }
        // Las listas se concatenan con `+`.
        if let (Valor::Lista(a), Valor::Lista(b)) = (&izquierda, &derecha) {
            let mut resultado = a.borrow().clone();
            resultado.extend(b.borrow().iter().cloned());
            return Ok(Valor::lista(resultado));
        }
        self.aritmetica(izquierda, derecha, ubicacion, "sumar")
    }

    fn aritmetica(
        &self,
        izquierda: Valor,
        derecha: Valor,
        ubicacion: Ubicacion,
        operacion: &str,
    ) -> Result<Valor, Fallo> {
        match (&izquierda, &derecha) {
            (Valor::Entero(a), Valor::Entero(b)) => {
                let resultado = match operacion {
                    "sumar" => a.checked_add(*b),
                    "restar" => a.checked_sub(*b),
                    "multiplicar" => a.checked_mul(*b),
                    "dividir" => {
                        if *b == 0 {
                            return Err(self.excepcion(
                                "E0401",
                                "división por cero".to_string(),
                                Some(ubicacion),
                            ));
                        }
                        a.checked_div(*b)
                    }
                    "modulo" => {
                        if *b == 0 {
                            return Err(self.excepcion(
                                "E0401",
                                "módulo por cero".to_string(),
                                Some(ubicacion),
                            ));
                        }
                        a.checked_rem(*b)
                    }
                    _ => unreachable!(),
                };
                resultado.map(Valor::Entero).ok_or_else(|| {
                    self.excepcion(
                        "E0402",
                        format!("desbordamiento de entero al {operacion} {a} y {b}"),
                        Some(ubicacion),
                    )
                })
            }
            _ => {
                let a = self.como_decimal(&izquierda, ubicacion, operacion)?;
                let b = self.como_decimal(&derecha, ubicacion, operacion)?;
                let resultado = match operacion {
                    "sumar" => a.checked_add(b),
                    "restar" => a.checked_sub(b),
                    "multiplicar" => a.checked_mul(b),
                    "dividir" => {
                        if b.is_zero() {
                            return Err(self.excepcion(
                                "E0401",
                                "división por cero".to_string(),
                                Some(ubicacion),
                            ));
                        }
                        a.checked_div(b)
                    }
                    "modulo" => {
                        if b.is_zero() {
                            return Err(self.excepcion(
                                "E0401",
                                "módulo por cero".to_string(),
                                Some(ubicacion),
                            ));
                        }
                        a.checked_rem(b)
                    }
                    _ => unreachable!(),
                };
                resultado.map(Valor::Numero).ok_or_else(|| {
                    self.excepcion(
                        "E0402",
                        format!("desbordamiento numérico al {operacion}"),
                        Some(ubicacion),
                    )
                })
            }
        }
    }

    fn como_decimal(
        &self,
        valor: &Valor,
        ubicacion: Ubicacion,
        operacion: &str,
    ) -> Result<Decimal, Fallo> {
        match valor {
            Valor::Entero(entero) => Ok(Decimal::from(*entero)),
            Valor::Numero(decimal) => Ok(*decimal),
            otro => Err(self.excepcion(
                "E0406",
                format!(
                    "no se puede {operacion} con un valor de tipo '{}'",
                    otro.nombre_tipo()
                ),
                Some(ubicacion),
            )),
        }
    }

    fn comparar(
        &self,
        izquierda: &Valor,
        derecha: &Valor,
        ubicacion: Ubicacion,
    ) -> Result<std::cmp::Ordering, Fallo> {
        match (izquierda, derecha) {
            (Valor::Entero(a), Valor::Entero(b)) => Ok(a.cmp(b)),
            (Valor::Texto(a), Valor::Texto(b)) => Ok(a.cmp(b)),
            _ => {
                let a = self.como_decimal(izquierda, ubicacion, "comparar")?;
                let b = self.como_decimal(derecha, ubicacion, "comparar")?;
                Ok(a.cmp(&b))
            }
        }
    }

    // ----- Llamadas -----

    fn llamar_valor(
        &mut self,
        objetivo: Valor,
        argumentos: Vec<Valor>,
        ubicacion: Ubicacion,
    ) -> Result<DestinoLlamada, Fallo> {
        match objetivo {
            Valor::Funcion(funcion, entorno) => {
                if funcion.asincrona {
                    return Ok(DestinoLlamada::Valor(Valor::Tarea(Rc::new(DatosTarea {
                        funcion,
                        entorno,
                        argumentos,
                        instancia: None,
                    }))));
                }
                Ok(DestinoLlamada::Marco(self.preparar_marco_funcion(
                    &funcion,
                    &entorno,
                    argumentos,
                    None,
                    Some(ubicacion),
                )?))
            }
            Valor::MetodoEnlazado(datos) => self.invocar_metodo(
                Rc::clone(&datos.funcion),
                Rc::clone(&datos.entorno),
                argumentos,
                datos.receptor.clone(),
                ubicacion,
            ),
            Valor::Nativa(nombre) => self
                .llamar_nativa(&nombre, &argumentos, ubicacion)
                .map(DestinoLlamada::Valor),
            otro => Err(self.excepcion(
                "E0406",
                format!(
                    "un valor de tipo '{}' no puede llamarse",
                    otro.nombre_tipo()
                ),
                Some(ubicacion),
            )),
        }
    }

    fn llamar_nativa(
        &mut self,
        nombre: &str,
        argumentos: &[Valor],
        ubicacion: Ubicacion,
    ) -> Result<Valor, Fallo> {
        // Clon del `Rc`: evita mantener un préstamo de `self.nativos` mientras
        // se llama una función `ConVm` que necesita `&mut self`.
        let nativos = Rc::clone(&self.nativos);
        let resultado = if let Some(funcion) = nativos.buscar_funcion(nombre) {
            funcion(argumentos)
        } else if let Some(funcion) = nativos.buscar_funcion_con_vm(nombre) {
            funcion(self, argumentos)
        } else {
            return Err(self.excepcion(
                "E0201",
                format!("la función nativa '{nombre}' no existe"),
                Some(ubicacion),
            ));
        };
        resultado.map_err(|fallo| match fallo {
            Fallo::Excepcion(mut datos) => {
                if datos.ubicacion.is_none() {
                    datos.ubicacion = Some(ubicacion);
                }
                if datos.llamadas.is_empty() {
                    datos.llamadas = self.pila_llamadas.clone();
                }
                Fallo::Excepcion(datos)
            }
            otro => otro,
        })
    }

    fn llamar_metodo(
        &mut self,
        receptor: Valor,
        nombre: &str,
        argumentos: Vec<Valor>,
        ubicacion: Ubicacion,
    ) -> Result<DestinoLlamada, Fallo> {
        match &receptor {
            Valor::ModuloNativo(modulo) => self
                .llamar_nativa(&format!("{modulo}.{nombre}"), &argumentos, ubicacion)
                .map(DestinoLlamada::Valor),
            Valor::Instancia(instancia) => {
                let clase = Rc::clone(&instancia.borrow().clase);
                match buscar_metodo_en_clase(self, &clase, nombre) {
                    Some((metodo, entorno)) => self.invocar_metodo(
                        metodo,
                        entorno,
                        argumentos,
                        Valor::Instancia(Rc::clone(instancia)),
                        ubicacion,
                    ),
                    None => Err(self.excepcion(
                        "E0201",
                        format!(
                            "el objeto '{}' no tiene un método '{nombre}'",
                            clase.compilado.nombre
                        ),
                        Some(ubicacion),
                    )),
                }
            }
            Valor::Clase(clase) => {
                // Métodos `libre`: se llaman sin instancia, `esto` es la clase.
                match buscar_metodo_en_clase(self, clase, nombre) {
                    Some((metodo, entorno)) if metodo_es_libre(clase, nombre) => self
                        .invocar_metodo(
                            metodo,
                            entorno,
                            argumentos,
                            Valor::Clase(Rc::clone(clase)),
                            ubicacion,
                        ),
                    Some(_) => Err(self.excepcion(
                        "E0406",
                        format!(
                            "el método '{nombre}' de '{}' necesita una instancia (no es 'libre')",
                            clase.compilado.nombre
                        ),
                        Some(ubicacion),
                    )),
                    None => Err(self.excepcion(
                        "E0201",
                        format!(
                            "el objeto '{}' no tiene un método '{nombre}'",
                            clase.compilado.nombre
                        ),
                        Some(ubicacion),
                    )),
                }
            }
            Valor::Padre { instancia, clase } => {
                let clase_actual = Rc::clone(&instancia.borrow().clase);
                match clase {
                    // `padre.Mamifero.comer()`: buscar desde el padre concreto.
                    Some(nombre_padre) => {
                        let Some(clase_padre) =
                            buscar_clase_ancestro(self, &clase_actual, nombre_padre)
                        else {
                            return Err(self.excepcion(
                                "E0201",
                                format!("'{nombre_padre}' no es un padre de la instancia"),
                                Some(ubicacion),
                            ));
                        };
                        match buscar_metodo_en_clase(self, &clase_padre, nombre) {
                            Some((metodo, entorno)) => self.invocar_metodo(
                                metodo,
                                entorno,
                                argumentos,
                                Valor::Instancia(Rc::clone(instancia)),
                                ubicacion,
                            ),
                            None => Err(self.excepcion(
                                "E0201",
                                format!("el objeto padre no tiene un método '{nombre}'"),
                                Some(ubicacion),
                            )),
                        }
                    }
                    None => {
                        // `padre.Animal(args)`: si el nombre es un ancestro, es
                        // una llamada a su constructor.
                        if let Some(clase_padre) =
                            buscar_clase_ancestro(self, &clase_actual, nombre)
                        {
                            return self.llamar_constructor(
                                &clase_padre,
                                argumentos,
                                Valor::Instancia(Rc::clone(instancia)),
                                ubicacion,
                            );
                        }
                        // `padre.metodo(args)`: buscar en los padres directos.
                        for nombre_padre in clase_actual
                            .compilado
                            .padres
                            .iter()
                            .chain(clase_actual.compilado.extiende_como.iter())
                        {
                            if let Some(clase_padre) =
                                buscar_clase_por_nombre(self, &clase_actual, nombre_padre)
                                && let Some((metodo, entorno)) =
                                    buscar_metodo_en_clase(self, &clase_padre, nombre)
                            {
                                return self.invocar_metodo(
                                    metodo,
                                    entorno,
                                    argumentos,
                                    Valor::Instancia(Rc::clone(instancia)),
                                    ubicacion,
                                );
                            }
                        }
                        Err(self.excepcion(
                            "E0201",
                            format!("el objeto padre no tiene un método '{nombre}'"),
                            Some(ubicacion),
                        ))
                    }
                }
            }
            // Instancias de objetos nativos: `instante.agregar_dias(1)` se
            // despacha como `Tiempo.agregar_dias` con el receptor primero.
            Valor::InstanciaNativa(datos) => {
                let clave = format!("{}.{nombre}", datos.tipo);
                if !self.nativos.existe_funcion(&clave) {
                    return Err(self.excepcion(
                        "E0201",
                        format!("el objeto '{}' no tiene un método '{nombre}'", datos.tipo),
                        Some(ubicacion),
                    ));
                }
                let mut completos = Vec::with_capacity(argumentos.len() + 1);
                completos.push(receptor.clone());
                completos.extend(argumentos);
                self.llamar_nativa(&clave, &completos, ubicacion)
                    .map(DestinoLlamada::Valor)
            }
            // Métodos nativos de los tipos primitivos: `texto.mayusculas`, ...
            _ => {
                let tipo = receptor.nombre_tipo();
                let clave = format!("{tipo}.{nombre}");
                if !self.nativos.existe_funcion(&clave) {
                    return Err(self.excepcion(
                        "E0406",
                        format!("el tipo '{tipo}' no tiene un método '{nombre}'"),
                        Some(ubicacion),
                    ));
                }
                // El receptor viaja como primer argumento.
                let mut completos = Vec::with_capacity(argumentos.len() + 1);
                completos.push(receptor);
                completos.extend(argumentos);
                self.llamar_nativa(&clave, &completos, ubicacion)
                    .map(DestinoLlamada::Valor)
            }
        }
    }

    fn invocar_metodo(
        &mut self,
        metodo: Rc<FuncionCompilada>,
        entorno: Rc<EntornoModulo>,
        argumentos: Vec<Valor>,
        esto: Valor,
        ubicacion: Ubicacion,
    ) -> Result<DestinoLlamada, Fallo> {
        if metodo.asincrona {
            return Ok(DestinoLlamada::Valor(Valor::Tarea(Rc::new(DatosTarea {
                funcion: metodo,
                entorno,
                argumentos,
                instancia: Some(esto),
            }))));
        }
        Ok(DestinoLlamada::Marco(self.preparar_marco_funcion(
            &metodo,
            &entorno,
            argumentos,
            Some(esto),
            Some(ubicacion),
        )?))
    }

    // ----- Objetos -----

    fn instanciar(
        &mut self,
        clase: &Rc<ClaseObjeto>,
        argumentos: Vec<Valor>,
        ubicacion: Ubicacion,
    ) -> Result<DestinoLlamada, Fallo> {
        // Atributos de toda la cadena (ancestros primero), aplanados.
        let mut atributos = IndexMap::new();
        self.recolectar_atributos(clase, &mut atributos)?;

        let instancia = Rc::new(RefCell::new(Instancia {
            clase: Rc::clone(clase),
            atributos,
        }));

        // Constructor propio (o de la cadena) con la aridad correcta.
        if let Some(clase_constructora) =
            buscar_clase_con_constructor(self, clase, argumentos.len())
        {
            let constructor = clase_constructora
                .constructores
                .iter()
                .find(|constructor| constructor.parametros.len() == argumentos.len())
                .cloned();
            if let Some(constructor) = constructor {
                let mut marco = self.preparar_marco_funcion(
                    &constructor,
                    &clase_constructora.entorno,
                    argumentos,
                    Some(Valor::Instancia(Rc::clone(&instancia))),
                    Some(ubicacion),
                )?;
                marco.valor_al_volver = Some(Valor::Instancia(Rc::clone(&instancia)));
                return Ok(DestinoLlamada::Marco(marco));
            }
        } else if !argumentos.is_empty() {
            return Err(self.excepcion(
                "E0210",
                format!(
                    "el objeto '{}' no tiene un constructor que acepte {} argumentos",
                    clase.compilado.nombre,
                    argumentos.len()
                ),
                Some(ubicacion),
            ));
        }

        Ok(DestinoLlamada::Valor(Valor::Instancia(instancia)))
    }

    fn llamar_constructor(
        &mut self,
        clase: &Rc<ClaseObjeto>,
        argumentos: Vec<Valor>,
        esto: Valor,
        ubicacion: Ubicacion,
    ) -> Result<DestinoLlamada, Fallo> {
        let constructor = clase
            .constructores
            .iter()
            .find(|constructor| constructor.parametros.len() == argumentos.len())
            .cloned()
            .ok_or_else(|| {
                self.excepcion(
                    "E0210",
                    format!(
                        "el objeto '{}' no tiene un constructor que acepte {} argumentos",
                        clase.compilado.nombre,
                        argumentos.len()
                    ),
                    Some(ubicacion),
                )
            })?;
        let mut marco = self.preparar_marco_funcion(
            &constructor,
            &clase.entorno,
            argumentos,
            Some(esto),
            Some(ubicacion),
        )?;
        marco.valor_al_volver = Some(Valor::Nulo);
        Ok(DestinoLlamada::Marco(marco))
    }

    fn recolectar_atributos(
        &mut self,
        clase: &Rc<ClaseObjeto>,
        destino: &mut IndexMap<String, Variable>,
    ) -> Result<(), Fallo> {
        // Primero los ancestros, para que el objeto propio pueda sobrescribir.
        for nombre_padre in clase
            .compilado
            .padres
            .iter()
            .chain(clase.compilado.extiende_como.iter())
        {
            if let Some(clase_padre) = buscar_clase_por_nombre(self, clase, nombre_padre) {
                self.recolectar_atributos(&clase_padre, destino)?;
            }
        }
        for atributo in &clase.compilado.atributos {
            if atributo.libre {
                continue;
            }
            let valor = match &atributo.inicial {
                Some(trozo) => self.ejecutar_trozo_aislado(trozo, &clase.entorno, None)?,
                None => Valor::Nulo,
            };
            destino.insert(
                atributo.nombre.clone(),
                Variable {
                    valor,
                    mutable: atributo.mutable,
                },
            );
        }
        Ok(())
    }

    // ----- Miembros e índices -----

    fn cargar_miembro(
        &mut self,
        objeto: Valor,
        nombre: &str,
        ubicacion: Ubicacion,
    ) -> Result<Valor, Fallo> {
        match &objeto {
            Valor::Jsn(mapa) => Ok(mapa.borrow().get(nombre).cloned().unwrap_or(Valor::Nulo)),
            Valor::Instancia(instancia) => {
                if let Some(variable) = instancia.borrow().atributos.get(nombre) {
                    return Ok(variable.valor.clone());
                }
                // Atributos libres de la cadena de clases.
                let clase = Rc::clone(&instancia.borrow().clase);
                if let Some(valor) = buscar_atributo_libre(self, &clase, nombre) {
                    return Ok(valor);
                }
                if let Some((funcion, entorno)) = buscar_metodo_libre_en_clase(self, &clase, nombre)
                {
                    return Ok(Valor::Funcion(funcion, entorno));
                }
                // Método de instancia como valor: se enlaza con su receptor
                // para que `esto` siga apuntando a esta instancia.
                if let Some((funcion, entorno)) = buscar_metodo_en_clase(self, &clase, nombre) {
                    return Ok(Valor::MetodoEnlazado(Rc::new(DatosMetodoEnlazado {
                        funcion,
                        entorno,
                        receptor: Valor::Instancia(Rc::clone(instancia)),
                    })));
                }
                Err(self.excepcion(
                    "E0201",
                    format!(
                        "la instancia de '{}' no tiene un atributo '{nombre}'",
                        clase.compilado.nombre
                    ),
                    Some(ubicacion),
                ))
            }
            Valor::Clase(clase) => {
                if let Some(valor) = buscar_atributo_libre(self, clase, nombre) {
                    return Ok(valor);
                }
                if let Some((funcion, entorno)) = buscar_metodo_libre_en_clase(self, clase, nombre)
                {
                    return Ok(Valor::Funcion(funcion, entorno));
                }
                Err(self.excepcion(
                    "E0201",
                    format!(
                        "el objeto '{}' no tiene un miembro libre '{nombre}'",
                        clase.compilado.nombre
                    ),
                    Some(ubicacion),
                ))
            }
            Valor::ModuloNativo(modulo) => {
                let clave = format!("{modulo}.{nombre}");
                if let Some(constante) = self.nativos.buscar_constante(&clave) {
                    return Ok(constante.clone());
                }
                if self.nativos.existe_funcion(&clave) {
                    return Ok(Valor::Nativa(Rc::from(clave.as_str())));
                }
                Err(self.excepcion(
                    "E0201",
                    format!("el módulo '{modulo}' no tiene '{nombre}'"),
                    Some(ubicacion),
                ))
            }
            Valor::Excepcion(datos) => match nombre {
                "mensaje" => Ok(Valor::texto(&datos.mensaje)),
                "llamadas" => Ok(Valor::lista(
                    datos.llamadas.iter().map(Valor::texto).collect(),
                )),
                _ => Err(self.excepcion(
                    "E0201",
                    format!("la excepción no tiene un miembro '{nombre}'"),
                    Some(ubicacion),
                )),
            },
            Valor::Padre { instancia, clase } => {
                if clase.is_none() {
                    let clase_actual = Rc::clone(&instancia.borrow().clase);
                    // `padre.Mamifero` → vista del padre concreto.
                    if buscar_clase_ancestro(self, &clase_actual, nombre).is_some() {
                        return Ok(Valor::Padre {
                            instancia: Rc::clone(instancia),
                            clase: Some(Rc::from(nombre)),
                        });
                    }
                }
                // `padre.atributo`: los atributos viven aplanados en la instancia.
                if let Some(variable) = instancia.borrow().atributos.get(nombre) {
                    return Ok(variable.valor.clone());
                }
                Err(self.excepcion(
                    "E0201",
                    format!("el objeto padre no tiene un miembro '{nombre}'"),
                    Some(ubicacion),
                ))
            }
            otro => Err(self.excepcion(
                "E0406",
                format!(
                    "un valor de tipo '{}' no tiene el miembro '{nombre}'",
                    otro.nombre_tipo()
                ),
                Some(ubicacion),
            )),
        }
    }

    fn guardar_miembro(
        &mut self,
        objeto: Valor,
        nombre: &str,
        valor: Valor,
        ubicacion: Ubicacion,
    ) -> Result<(), Fallo> {
        match &objeto {
            Valor::Jsn(mapa) => {
                mapa.borrow_mut().insert(nombre.to_string(), valor);
                Ok(())
            }
            Valor::Instancia(instancia) => {
                let mut instancia = instancia.borrow_mut();
                if let Some(variable) = instancia.atributos.get_mut(nombre) {
                    variable.valor = valor;
                    return Ok(());
                }
                let clase = Rc::clone(&instancia.clase);
                drop(instancia);
                if guardar_atributo_libre(self, &clase, nombre, valor.clone()) {
                    return Ok(());
                }
                Err(self.excepcion(
                    "E0201",
                    format!(
                        "la instancia de '{}' no tiene un atributo '{nombre}'",
                        clase.compilado.nombre
                    ),
                    Some(ubicacion),
                ))
            }
            Valor::Clase(clase) => {
                if guardar_atributo_libre(self, clase, nombre, valor) {
                    Ok(())
                } else {
                    Err(self.excepcion(
                        "E0201",
                        format!(
                            "el objeto '{}' no tiene un atributo libre '{nombre}'",
                            clase.compilado.nombre
                        ),
                        Some(ubicacion),
                    ))
                }
            }
            otro => Err(self.excepcion(
                "E0406",
                format!(
                    "no se puede asignar el miembro '{nombre}' a un valor de tipo '{}'",
                    otro.nombre_tipo()
                ),
                Some(ubicacion),
            )),
        }
    }

    fn indexar(&self, objeto: Valor, indice: Valor, ubicacion: Ubicacion) -> Result<Valor, Fallo> {
        match (&objeto, &indice) {
            (Valor::Lista(lista), Valor::Entero(posicion)) => {
                let lista = lista.borrow();
                let indice_real = indice_normalizado(*posicion, lista.len()).ok_or_else(|| {
                    self.excepcion(
                        "E0403",
                        format!(
                            "el índice {posicion} está fuera de rango (la lista tiene {} elementos)",
                            lista.len()
                        ),
                        Some(ubicacion),
                    )
                })?;
                Ok(lista[indice_real].clone())
            }
            (Valor::Texto(texto), Valor::Entero(posicion)) => {
                let caracteres: Vec<char> = texto.chars().collect();
                let indice_real =
                    indice_normalizado(*posicion, caracteres.len()).ok_or_else(|| {
                        self.excepcion(
                            "E0403",
                            format!(
                                "el índice {posicion} está fuera de rango (el texto tiene {} caracteres)",
                                caracteres.len()
                            ),
                            Some(ubicacion),
                        )
                    })?;
                Ok(Valor::texto(caracteres[indice_real].to_string()))
            }
            (Valor::Jsn(mapa), Valor::Texto(clave)) => Ok(mapa
                .borrow()
                .get(clave.as_ref())
                .cloned()
                .unwrap_or(Valor::Nulo)),
            _ => Err(self.excepcion(
                "E0406",
                format!(
                    "no se puede indexar '{}' con '{}'",
                    objeto.nombre_tipo(),
                    indice.nombre_tipo()
                ),
                Some(ubicacion),
            )),
        }
    }

    fn guardar_indice(
        &self,
        objeto: Valor,
        indice: Valor,
        valor: Valor,
        ubicacion: Ubicacion,
    ) -> Result<(), Fallo> {
        match (&objeto, &indice) {
            (Valor::Lista(lista), Valor::Entero(posicion)) => {
                let mut lista = lista.borrow_mut();
                let longitud = lista.len();
                let indice_real = indice_normalizado(*posicion, longitud).ok_or_else(|| {
                    self.excepcion(
                        "E0403",
                        format!(
                            "el índice {posicion} está fuera de rango (la lista tiene {longitud} elementos)"
                        ),
                        Some(ubicacion),
                    )
                })?;
                lista[indice_real] = valor;
                Ok(())
            }
            (Valor::Jsn(mapa), Valor::Texto(clave)) => {
                mapa.borrow_mut().insert(clave.to_string(), valor);
                Ok(())
            }
            _ => Err(self.excepcion(
                "E0406",
                format!(
                    "no se puede asignar por índice en '{}' con '{}'",
                    objeto.nombre_tipo(),
                    indice.nombre_tipo()
                ),
                Some(ubicacion),
            )),
        }
    }
}

fn limpiar_locales(locales: &mut [Valor], desde: u16) {
    let desde = desde as usize;
    if desde >= locales.len() {
        return;
    }
    for ranura in &mut locales[desde..] {
        *ranura = Valor::Nulo;
    }
}

/// Flujo de control al ejecutar una instrucción.
enum Flujo {
    Continuar,
    /// El `pc` ya fue actualizado (saltos).
    SinAvanzar,
    Retornar(Valor),
    /// Empujar un marco y seguir en el mismo bucle (sin anidar Rust).
    Llamar(Marco),
}

fn valor_de_constante(constante: &Constante) -> Valor {
    match constante {
        Constante::Entero(entero) => Valor::Entero(*entero),
        Constante::Numero(decimal) => Valor::Numero(*decimal),
        Constante::Texto(texto) => Valor::texto(texto),
        Constante::Log(valor_logico) => Valor::Log(*valor_logico),
        Constante::Nulo => Valor::Nulo,
    }
}

/// Normaliza un índice (acepta negativos desde el final).
fn indice_normalizado(indice: i64, longitud: usize) -> Option<usize> {
    let longitud = longitud as i64;
    let real = if indice < 0 {
        longitud + indice
    } else {
        indice
    };
    if real >= 0 && real < longitud {
        Some(real as usize)
    } else {
        None
    }
}

fn clase_en_entorno(entorno: &EntornoModulo, nombre: &str) -> Option<Rc<ClaseObjeto>> {
    if let Some(indice) = entorno
        .modulo
        .nombres_globales
        .iter()
        .position(|global| global == nombre)
        && let Some(Variable {
            valor: Valor::Clase(clase),
            ..
        }) = entorno.valores_globales.borrow().get(indice)
    {
        return Some(Rc::clone(clase));
    }
    if let Some(Variable {
        valor: Valor::Clase(clase),
        ..
    }) = entorno.globales.borrow().get(nombre)
    {
        return Some(Rc::clone(clase));
    }
    None
}

fn sincronizar_globales(entorno: &EntornoModulo) {
    let valores = entorno.valores_globales.borrow();
    let mut globales = entorno.globales.borrow_mut();
    for (indice, variable) in valores.iter().enumerate() {
        if let Some((_, destino)) = globales.get_index_mut(indice) {
            *destino = variable.clone();
        }
    }
}

/// Busca una clase por nombre: primero en el entorno de la clase de partida,
/// luego entre los módulos cargados.
fn buscar_clase_por_nombre(
    vm: &Vm,
    desde: &Rc<ClaseObjeto>,
    nombre: &str,
) -> Option<Rc<ClaseObjeto>> {
    if let Some(clase) = clase_en_entorno(&desde.entorno, nombre) {
        return Some(clase);
    }
    for entorno in vm.modulos.values() {
        if let Some(clase) = clase_en_entorno(entorno, nombre) {
            return Some(clase);
        }
    }
    None
}

/// Busca un ancestro (por nombre) en la cadena de herencia de una clase.
fn buscar_clase_ancestro(
    vm: &Vm,
    clase: &Rc<ClaseObjeto>,
    nombre: &str,
) -> Option<Rc<ClaseObjeto>> {
    for nombre_padre in clase
        .compilado
        .padres
        .iter()
        .chain(clase.compilado.extiende_como.iter())
    {
        let clase_padre = buscar_clase_por_nombre(vm, clase, nombre_padre)?;
        if nombre_padre == nombre {
            return Some(clase_padre);
        }
        if let Some(encontrada) = buscar_clase_ancestro(vm, &clase_padre, nombre) {
            return Some(encontrada);
        }
    }
    None
}

/// Busca un método subiendo por la cadena de herencia.
fn buscar_metodo_en_clase(
    vm: &Vm,
    clase: &Rc<ClaseObjeto>,
    nombre: &str,
) -> Option<(Rc<FuncionCompilada>, Rc<EntornoModulo>)> {
    if let Some(metodo) = clase.metodos.get(nombre) {
        return Some((Rc::clone(&metodo.funcion), Rc::clone(&clase.entorno)));
    }
    for nombre_padre in clase
        .compilado
        .padres
        .iter()
        .chain(clase.compilado.extiende_como.iter())
    {
        if let Some(clase_padre) = buscar_clase_por_nombre(vm, clase, nombre_padre)
            && let Some(encontrado) = buscar_metodo_en_clase(vm, &clase_padre, nombre)
        {
            return Some(encontrado);
        }
    }
    None
}

/// Busca un método libre por referencia, incluyendo la cadena de herencia.
/// Los métodos de instancia se resuelven aparte, como [`Valor::MetodoEnlazado`],
/// porque necesitan capturar su receptor.
fn buscar_metodo_libre_en_clase(
    vm: &Vm,
    clase: &Rc<ClaseObjeto>,
    nombre: &str,
) -> Option<(Rc<FuncionCompilada>, Rc<EntornoModulo>)> {
    if let Some(metodo) = clase.metodos.get(nombre)
        && metodo.libre
    {
        return Some((Rc::clone(&metodo.funcion), Rc::clone(&clase.entorno)));
    }
    for nombre_padre in clase
        .compilado
        .padres
        .iter()
        .chain(clase.compilado.extiende_como.iter())
    {
        if let Some(clase_padre) = buscar_clase_por_nombre(vm, clase, nombre_padre)
            && let Some(encontrado) = buscar_metodo_libre_en_clase(vm, &clase_padre, nombre)
        {
            return Some(encontrado);
        }
    }
    None
}

fn metodo_es_libre(clase: &ClaseObjeto, nombre: &str) -> bool {
    clase
        .metodos
        .get(nombre)
        .map(|metodo| metodo.libre)
        .unwrap_or(false)
}

/// Busca la primera clase de la cadena que declare constructores con la aridad dada.
fn buscar_clase_con_constructor(
    vm: &Vm,
    clase: &Rc<ClaseObjeto>,
    aridad: usize,
) -> Option<Rc<ClaseObjeto>> {
    if clase
        .constructores
        .iter()
        .any(|constructor| constructor.parametros.len() == aridad)
    {
        return Some(Rc::clone(clase));
    }
    if !clase.constructores.is_empty() {
        // Tiene constructores pero ninguno con esa aridad: no seguir subiendo.
        return None;
    }
    for nombre_padre in clase
        .compilado
        .padres
        .iter()
        .chain(clase.compilado.extiende_como.iter())
    {
        if let Some(clase_padre) = buscar_clase_por_nombre(vm, clase, nombre_padre)
            && let Some(encontrada) = buscar_clase_con_constructor(vm, &clase_padre, aridad)
        {
            return Some(encontrada);
        }
    }
    None
}

/// Busca un atributo libre subiendo por la cadena de herencia.
fn buscar_atributo_libre(vm: &Vm, clase: &Rc<ClaseObjeto>, nombre: &str) -> Option<Valor> {
    if let Some(variable) = clase.atributos_libres.borrow().get(nombre) {
        return Some(variable.valor.clone());
    }
    for nombre_padre in clase
        .compilado
        .padres
        .iter()
        .chain(clase.compilado.extiende_como.iter())
    {
        if let Some(clase_padre) = buscar_clase_por_nombre(vm, clase, nombre_padre)
            && let Some(valor) = buscar_atributo_libre(vm, &clase_padre, nombre)
        {
            return Some(valor);
        }
    }
    None
}

/// Asigna un atributo libre si existe en la cadena. Devuelve `true` si lo asignó.
fn guardar_atributo_libre(vm: &Vm, clase: &Rc<ClaseObjeto>, nombre: &str, valor: Valor) -> bool {
    if let Some(variable) = clase.atributos_libres.borrow_mut().get_mut(nombre) {
        variable.valor = valor;
        return true;
    }
    for nombre_padre in clase
        .compilado
        .padres
        .iter()
        .chain(clase.compilado.extiende_como.iter())
    {
        if let Some(clase_padre) = buscar_clase_por_nombre(vm, clase, nombre_padre)
            && guardar_atributo_libre(vm, &clase_padre, nombre, valor.clone())
        {
            return true;
        }
    }
    false
}
