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
    ClaseObjeto, DatosMetodoEnlazado, DatosTarea, EntornoModulo, EstadoIterador, Instancia, Valor,
    Variable, carga_a_valor, texto_de_valor,
};

/// Límite de marcos anidados (recursión).
const LIMITE_PROFUNDIDAD: usize = 200;

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
    /// Bucle de eventos: E/S de fondo (tokio) + despacho de solicitudes en
    /// el hilo de la VM. Ver `crate::bucle_eventos`.
    bucle: BucleEventos,
    /// Resultados de tareas nativas que llegaron mientras se esperaba otra
    /// tarea distinta (se guardan para cuando se espere la suya).
    resultados_pendientes: RefCell<HashMap<u64, Result<CargaNativa, String>>>,
    /// Despachadores de solicitudes de servidores/recursos, por servicio.
    despachadores: RefCell<HashMap<String, Despachador>>,
}

/// Marco de ejecución de un trozo de bytecode.
struct Marco {
    entorno: Rc<EntornoModulo>,
    /// Ámbitos locales; vacío en el nivel superior (las declaraciones van a
    /// las globales del módulo).
    ambitos: Vec<IndexMap<String, Variable>>,
    esto: Option<Valor>,
}

/// Manejador de excepciones activo dentro de un marco.
struct Manejador {
    pc: usize,
    profundidad_pila: usize,
    profundidad_ambitos: usize,
}

impl Vm {
    pub fn nueva(nativos: Rc<RegistroNativos>) -> Self {
        Self {
            nativos,
            modulos: IndexMap::new(),
            pila_llamadas: Vec::new(),
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
        let entorno = Rc::new(EntornoModulo {
            modulo: Rc::clone(&modulo),
            globales: RefCell::new(importaciones),
        });

        // Funciones del módulo.
        for funcion in &modulo.funciones {
            entorno.globales.borrow_mut().insert(
                funcion.nombre.clone(),
                Variable {
                    valor: Valor::Funcion(Rc::new(funcion.clone()), Rc::clone(&entorno)),
                    mutable: false,
                },
            );
        }

        // Objetos del módulo (con sus atributos libres inicializados).
        for objeto in &modulo.objetos {
            let clase = self.crear_clase(objeto, &entorno)?;
            entorno.globales.borrow_mut().insert(
                objeto.nombre.clone(),
                Variable {
                    valor: Valor::Clase(clase),
                    mutable: false,
                },
            );
        }

        // Código principal del módulo.
        let mut marco = Marco {
            entorno: Rc::clone(&entorno),
            ambitos: Vec::new(),
            esto: None,
        };
        let valor = self
            .ejecutar_trozo(&modulo.principal, &mut marco)
            .map_err(|fallo| fallo.a_error(&modulo.nombre))?;

        self.modulos
            .insert(modulo.nombre.clone(), Rc::clone(&entorno));
        Ok((entorno, valor))
    }

    fn crear_clase(
        &mut self,
        objeto: &ObjetoCompilado,
        entorno: &Rc<EntornoModulo>,
    ) -> Result<Rc<ClaseObjeto>, ErrorQuetzal> {
        let clase = Rc::new(ClaseObjeto {
            compilado: objeto.clone(),
            entorno: Rc::clone(entorno),
            atributos_libres: RefCell::new(IndexMap::new()),
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
        let mut marco = Marco {
            entorno: Rc::clone(entorno),
            ambitos: vec![IndexMap::new()],
            esto,
        };
        self.ejecutar_trozo(trozo, &mut marco)
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
        if self.pila_llamadas.len() >= LIMITE_PROFUNDIDAD {
            return Err(self.excepcion(
                "E0407",
                format!(
                    "se alcanzó el límite de {LIMITE_PROFUNDIDAD} llamadas anidadas (¿recursión infinita?)"
                ),
                ubicacion,
            ));
        }

        let mut locales = IndexMap::new();
        for ((nombre, mutable), valor) in funcion.parametros.iter().zip(argumentos) {
            locales.insert(
                nombre.clone(),
                Variable {
                    valor,
                    mutable: *mutable,
                },
            );
        }

        let mut marco = Marco {
            entorno: Rc::clone(entorno),
            ambitos: vec![locales],
            esto,
        };

        self.pila_llamadas.push(funcion.nombre.clone());
        let resultado = self.ejecutar_trozo(&funcion.trozo, &mut marco);
        self.pila_llamadas.pop();
        resultado
    }

    /// Resuelve un valor `esperar`: si es una [`Valor::Tarea`] (función
    /// Quetzal `asincrono`) la ejecuta; si es una [`Valor::TareaNativa`]
    /// (E/S en el bucle de eventos) bombea el bucle hasta que se resuelve;
    /// cualquier otro valor se devuelve sin cambios. Es la implementación de
    /// la instrucción `Esperar`, expuesta también para que los módulos
    /// nativos (y las pruebas) puedan esperar tareas fuera del bytecode.
    pub fn esperar(&mut self, valor: Valor, ubicacion: Ubicacion) -> Result<Valor, Fallo> {
        match valor {
            Valor::Tarea(tarea) => self.llamar_funcion(
                &tarea.funcion,
                &tarea.entorno,
                tarea.argumentos.clone(),
                tarea.instancia.clone(),
                Some(ubicacion),
            ),
            Valor::TareaNativa(tarea) => self.resolver_tarea_nativa(tarea.id, ubicacion),
            otro => Ok(otro),
        }
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

    fn ejecutar_trozo(&mut self, trozo: &Trozo, marco: &mut Marco) -> Result<Valor, Fallo> {
        let mut pila: Vec<Valor> = Vec::new();
        let mut manejadores: Vec<Manejador> = Vec::new();
        let mut pc = 0usize;

        while pc < trozo.instrucciones.len() {
            let ubicacion = trozo.ubicaciones[pc];
            let resultado = self.ejecutar_instruccion(
                &trozo.instrucciones[pc],
                ubicacion,
                marco,
                &mut pila,
                &mut manejadores,
                &mut pc,
            );

            match resultado {
                Ok(Flujo::Continuar) => pc += 1,
                Ok(Flujo::SinAvanzar) => {}
                Ok(Flujo::Retornar(valor)) => return Ok(valor),
                Err(Fallo::Excepcion(mut datos)) => {
                    if datos.ubicacion.is_none() {
                        datos.ubicacion = Some(ubicacion);
                    }
                    match manejadores.pop() {
                        Some(manejador) => {
                            pila.truncate(manejador.profundidad_pila);
                            marco.ambitos.truncate(manejador.profundidad_ambitos);
                            pila.push(Valor::Excepcion(Rc::new(datos)));
                            pc = manejador.pc;
                        }
                        None => return Err(Fallo::Excepcion(datos)),
                    }
                }
                Err(otro) => return Err(otro),
            }
        }

        Ok(pila.pop().unwrap_or(Valor::Nulo))
    }

    fn ejecutar_instruccion(
        &mut self,
        instruccion: &Instruccion,
        ubicacion: Ubicacion,
        marco: &mut Marco,
        pila: &mut Vec<Valor>,
        manejadores: &mut Vec<Manejador>,
        pc: &mut usize,
    ) -> Result<Flujo, Fallo> {
        match instruccion {
            Instruccion::CargarConstante(indice) => {
                let constante = &marco.entorno.modulo.constantes[*indice as usize];
                pila.push(valor_de_constante(constante));
            }
            Instruccion::Duplicar => {
                let tope = self.tope(pila, ubicacion)?.clone();
                pila.push(tope);
            }
            Instruccion::Desechar => {
                self.sacar(pila, ubicacion)?;
            }

            Instruccion::DeclararVariable(indice, mutable) => {
                let valor = self.sacar(pila, ubicacion)?;
                let nombre = marco.entorno.modulo.nombres[*indice as usize].clone();
                let variable = Variable {
                    valor,
                    mutable: *mutable,
                };
                match marco.ambitos.last_mut() {
                    Some(ambito) => {
                        ambito.insert(nombre, variable);
                    }
                    None => {
                        marco.entorno.globales.borrow_mut().insert(nombre, variable);
                    }
                }
            }
            Instruccion::CargarVariable(indice) => {
                let nombre = &marco.entorno.modulo.nombres[*indice as usize];
                let valor = self.buscar_variable(marco, nombre).ok_or_else(|| {
                    self.excepcion(
                        "E0201",
                        format!("'{nombre}' no está definido"),
                        Some(ubicacion),
                    )
                })?;
                pila.push(valor);
            }
            Instruccion::GuardarVariable(indice) => {
                let valor = self.sacar(pila, ubicacion)?;
                let nombre = marco.entorno.modulo.nombres[*indice as usize].clone();
                self.guardar_variable(marco, &nombre, valor, ubicacion)?;
            }
            Instruccion::AbrirAmbito => marco.ambitos.push(IndexMap::new()),
            Instruccion::CerrarAmbito => {
                marco.ambitos.pop();
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
                let derecha = self.sacar(pila, ubicacion)?;
                let izquierda = self.sacar(pila, ubicacion)?;
                pila.push(self.operacion_binaria(instruccion, izquierda, derecha, ubicacion)?);
            }
            Instruccion::Negar => {
                let valor = self.sacar(pila, ubicacion)?;
                let resultado = match valor {
                    Valor::Entero(entero) => {
                        Valor::Entero(entero.checked_neg().ok_or_else(|| {
                            self.excepcion(
                                "E0402",
                                "desbordamiento al negar el entero".to_string(),
                                Some(ubicacion),
                            )
                        })?)
                    }
                    Valor::Numero(decimal) => Valor::Numero(-decimal),
                    otro => {
                        return Err(self.excepcion(
                            "E0406",
                            format!(
                                "no se puede negar un valor de tipo '{}'",
                                otro.nombre_tipo()
                            ),
                            Some(ubicacion),
                        ));
                    }
                };
                pila.push(resultado);
            }
            Instruccion::NoLogico => {
                let valor = self.sacar(pila, ubicacion)?;
                let logico = self.como_logico(&valor, ubicacion)?;
                pila.push(Valor::Log(!logico));
            }

            Instruccion::Saltar(destino) => {
                *pc = *destino;
                return Ok(Flujo::SinAvanzar);
            }
            Instruccion::SaltarSiFalso(destino) => {
                let valor = self.sacar(pila, ubicacion)?;
                if !self.como_logico(&valor, ubicacion)? {
                    *pc = *destino;
                    return Ok(Flujo::SinAvanzar);
                }
            }
            Instruccion::SaltarSiVerdadero(destino) => {
                let valor = self.sacar(pila, ubicacion)?;
                if self.como_logico(&valor, ubicacion)? {
                    *pc = *destino;
                    return Ok(Flujo::SinAvanzar);
                }
            }
            Instruccion::SaltarSiFalsoYDejar(destino) => {
                let valor = self.tope(pila, ubicacion)?.clone();
                if !self.como_logico(&valor, ubicacion)? {
                    *pc = *destino;
                    return Ok(Flujo::SinAvanzar);
                }
                self.sacar(pila, ubicacion)?;
            }
            Instruccion::SaltarSiVerdaderoYDejar(destino) => {
                let valor = self.tope(pila, ubicacion)?.clone();
                if self.como_logico(&valor, ubicacion)? {
                    *pc = *destino;
                    return Ok(Flujo::SinAvanzar);
                }
                self.sacar(pila, ubicacion)?;
            }

            Instruccion::Llamar(cantidad) => {
                let argumentos = self.sacar_varios(pila, *cantidad as usize, ubicacion)?;
                let objetivo = self.sacar(pila, ubicacion)?;
                let resultado = self.llamar_valor(objetivo, argumentos, ubicacion)?;
                pila.push(resultado);
            }
            Instruccion::LlamarMetodo(indice_nombre, cantidad) => {
                let nombre = marco.entorno.modulo.nombres[*indice_nombre as usize].clone();
                let argumentos = self.sacar_varios(pila, *cantidad as usize, ubicacion)?;
                let receptor = self.sacar(pila, ubicacion)?;
                let resultado = self.llamar_metodo(receptor, &nombre, argumentos, ubicacion)?;
                pila.push(resultado);
            }
            Instruccion::Retornar => {
                let valor = pila.pop().unwrap_or(Valor::Nulo);
                return Ok(Flujo::Retornar(valor));
            }

            Instruccion::CargarMiembro(indice) => {
                let nombre = marco.entorno.modulo.nombres[*indice as usize].clone();
                let objeto = self.sacar(pila, ubicacion)?;
                pila.push(self.cargar_miembro(objeto, &nombre, ubicacion)?);
            }
            Instruccion::GuardarMiembro(indice) => {
                let valor = self.sacar(pila, ubicacion)?;
                let objeto = self.sacar(pila, ubicacion)?;
                let nombre = marco.entorno.modulo.nombres[*indice as usize].clone();
                self.guardar_miembro(objeto, &nombre, valor, ubicacion)?;
            }
            Instruccion::Indexar => {
                let indice = self.sacar(pila, ubicacion)?;
                let objeto = self.sacar(pila, ubicacion)?;
                pila.push(self.indexar(objeto, indice, ubicacion)?);
            }
            Instruccion::GuardarIndice => {
                let valor = self.sacar(pila, ubicacion)?;
                let indice = self.sacar(pila, ubicacion)?;
                let objeto = self.sacar(pila, ubicacion)?;
                self.guardar_indice(objeto, indice, valor, ubicacion)?;
            }

            Instruccion::CrearLista(cantidad) => {
                let elementos = self.sacar_varios(pila, *cantidad as usize, ubicacion)?;
                pila.push(Valor::lista(elementos));
            }
            Instruccion::CrearJsn(cantidad) => {
                let pares = self.sacar_varios(pila, (*cantidad as usize) * 2, ubicacion)?;
                let mut mapa = IndexMap::new();
                for par in pares.chunks(2) {
                    let clave = texto_de_valor(&par[0]);
                    mapa.insert(clave, par[1].clone());
                }
                pila.push(Valor::jsn(mapa));
            }
            Instruccion::Interpolar(cantidad) => {
                let partes = self.sacar_varios(pila, *cantidad as usize, ubicacion)?;
                let texto: String = partes.iter().map(texto_de_valor).collect();
                pila.push(Valor::texto(texto));
            }

            Instruccion::Instanciar(indice, cantidad) => {
                let nombre = marco.entorno.modulo.nombres[*indice as usize].clone();
                let argumentos = self.sacar_varios(pila, *cantidad as usize, ubicacion)?;
                let valor_clase = self.buscar_variable(marco, &nombre).ok_or_else(|| {
                    self.excepcion(
                        "E0201",
                        format!("el objeto '{nombre}' no está definido"),
                        Some(ubicacion),
                    )
                })?;
                match valor_clase {
                    Valor::Clase(clase) => {
                        pila.push(self.instanciar(&clase, argumentos, ubicacion)?);
                    }
                    // Objetos nativos (`nuevo Tiempo(...)`): el módulo registra
                    // su constructor con la clave `{modulo}.constructor`.
                    Valor::ModuloNativo(modulo) => {
                        let clave = format!("{modulo}.constructor");
                        if !self.nativos.existe_funcion(&clave) {
                            return Err(self.excepcion(
                                "E0406",
                                format!("'{nombre}' no es un objeto instanciable"),
                                Some(ubicacion),
                            ));
                        }
                        pila.push(self.llamar_nativa(&clave, &argumentos, ubicacion)?);
                    }
                    _ => {
                        return Err(self.excepcion(
                            "E0406",
                            format!("'{nombre}' no es un objeto instanciable"),
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
                pila.push(esto);
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
                pila.push(Valor::Padre {
                    instancia,
                    clase: None,
                });
            }

            Instruccion::CrearIterador => {
                let valor = self.sacar(pila, ubicacion)?;
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
                pila.push(Valor::Iterador(Rc::new(RefCell::new(EstadoIterador {
                    elementos,
                    posicion: 0,
                }))));
            }
            Instruccion::IteradorSiguiente(destino) => {
                let valor = self.sacar(pila, ubicacion)?;
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
                    pila.push(elemento);
                } else {
                    *pc = *destino;
                    return Ok(Flujo::SinAvanzar);
                }
            }

            Instruccion::EntrarIntentar(pc_manejador) => {
                manejadores.push(Manejador {
                    pc: *pc_manejador,
                    profundidad_pila: pila.len(),
                    profundidad_ambitos: marco.ambitos.len(),
                });
            }
            Instruccion::SalirIntentar => {
                manejadores.pop();
            }
            Instruccion::Lanzar => {
                let valor = self.sacar(pila, ubicacion)?;
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
                let valor = self.sacar(pila, ubicacion)?;
                pila.push(self.esperar(valor, ubicacion)?);
            }
        }
        Ok(Flujo::Continuar)
    }

    // ----- Variables -----

    fn buscar_variable(&self, marco: &Marco, nombre: &str) -> Option<Valor> {
        for ambito in marco.ambitos.iter().rev() {
            if let Some(variable) = ambito.get(nombre) {
                return Some(variable.valor.clone());
            }
        }
        if let Some(variable) = marco.entorno.globales.borrow().get(nombre) {
            return Some(variable.valor.clone());
        }
        // Globales del lenguaje (consola, rango) registradas como nativas.
        if self.nativos.buscar_funcion(nombre).is_some() {
            return Some(Valor::Nativa(Rc::from(nombre)));
        }
        if self.nativos.existe_modulo(nombre) {
            return Some(Valor::ModuloNativo(Rc::from(nombre)));
        }
        None
    }

    fn guardar_variable(
        &self,
        marco: &mut Marco,
        nombre: &str,
        valor: Valor,
        ubicacion: Ubicacion,
    ) -> Result<(), Fallo> {
        for ambito in marco.ambitos.iter_mut().rev() {
            if let Some(variable) = ambito.get_mut(nombre) {
                if !variable.mutable {
                    return Err(self.error_constante(nombre, ubicacion));
                }
                variable.valor = valor;
                return Ok(());
            }
        }
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
    ) -> Result<Valor, Fallo> {
        match objetivo {
            Valor::Funcion(funcion, entorno) => {
                if funcion.asincrona {
                    return Ok(Valor::Tarea(Rc::new(DatosTarea {
                        funcion,
                        entorno,
                        argumentos,
                        instancia: None,
                    })));
                }
                self.llamar_funcion(&funcion, &entorno, argumentos, None, Some(ubicacion))
            }
            Valor::MetodoEnlazado(datos) => self.invocar_metodo(
                Rc::clone(&datos.funcion),
                Rc::clone(&datos.entorno),
                argumentos,
                datos.receptor.clone(),
                ubicacion,
            ),
            Valor::Nativa(nombre) => self.llamar_nativa(&nombre, &argumentos, ubicacion),
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
    ) -> Result<Valor, Fallo> {
        match &receptor {
            Valor::ModuloNativo(modulo) => {
                self.llamar_nativa(&format!("{modulo}.{nombre}"), &argumentos, ubicacion)
            }
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
                    Some((metodo, entorno)) if metodo_es_libre(&clase.compilado, nombre) => self
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
    ) -> Result<Valor, Fallo> {
        if metodo.asincrona {
            return Ok(Valor::Tarea(Rc::new(DatosTarea {
                funcion: metodo,
                entorno,
                argumentos,
                instancia: Some(esto),
            })));
        }
        self.llamar_funcion(&metodo, &entorno, argumentos, Some(esto), Some(ubicacion))
    }

    // ----- Objetos -----

    fn instanciar(
        &mut self,
        clase: &Rc<ClaseObjeto>,
        argumentos: Vec<Valor>,
        ubicacion: Ubicacion,
    ) -> Result<Valor, Fallo> {
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
                .compilado
                .constructores
                .iter()
                .find(|constructor| constructor.parametros.len() == argumentos.len())
                .cloned();
            if let Some(constructor) = constructor {
                self.llamar_funcion(
                    &Rc::new(constructor),
                    &clase_constructora.entorno,
                    argumentos,
                    Some(Valor::Instancia(Rc::clone(&instancia))),
                    Some(ubicacion),
                )?;
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

        Ok(Valor::Instancia(instancia))
    }

    fn llamar_constructor(
        &mut self,
        clase: &Rc<ClaseObjeto>,
        argumentos: Vec<Valor>,
        esto: Valor,
        ubicacion: Ubicacion,
    ) -> Result<Valor, Fallo> {
        let constructor = clase
            .compilado
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
        self.llamar_funcion(
            &Rc::new(constructor),
            &clase.entorno,
            argumentos,
            Some(esto),
            Some(ubicacion),
        )?;
        Ok(Valor::Nulo)
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

/// Flujo de control al ejecutar una instrucción.
enum Flujo {
    Continuar,
    /// El `pc` ya fue actualizado (saltos).
    SinAvanzar,
    Retornar(Valor),
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

/// Busca una clase por nombre: primero en el entorno de la clase de partida,
/// luego entre los módulos cargados.
fn buscar_clase_por_nombre(
    vm: &Vm,
    desde: &Rc<ClaseObjeto>,
    nombre: &str,
) -> Option<Rc<ClaseObjeto>> {
    if let Some(Variable {
        valor: Valor::Clase(clase),
        ..
    }) = desde.entorno.globales.borrow().get(nombre)
    {
        return Some(Rc::clone(clase));
    }
    for entorno in vm.modulos.values() {
        if let Some(Variable {
            valor: Valor::Clase(clase),
            ..
        }) = entorno.globales.borrow().get(nombre)
        {
            return Some(Rc::clone(clase));
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
    if let Some(metodo) = clase
        .compilado
        .metodos
        .iter()
        .find(|metodo| metodo.funcion.nombre == nombre)
    {
        return Some((Rc::new(metodo.funcion.clone()), Rc::clone(&clase.entorno)));
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
    if let Some(metodo) = clase
        .compilado
        .metodos
        .iter()
        .find(|metodo| metodo.funcion.nombre == nombre && metodo.libre)
    {
        return Some((Rc::new(metodo.funcion.clone()), Rc::clone(&clase.entorno)));
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

fn metodo_es_libre(compilado: &ObjetoCompilado, nombre: &str) -> bool {
    compilado
        .metodos
        .iter()
        .find(|metodo| metodo.funcion.nombre == nombre)
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
        .compilado
        .constructores
        .iter()
        .any(|constructor| constructor.parametros.len() == aridad)
    {
        return Some(Rc::clone(clase));
    }
    if !clase.compilado.constructores.is_empty() {
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
