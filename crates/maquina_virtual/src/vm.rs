//! Intérprete de bytecode: pila única, marcos livianos y excepciones.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use bytecode::{
    CodigoOp, Constante, FuncionCompilada, Instruccion, ModuloCompilado, ObjetoCompilado, Trozo,
};
use indexmap::IndexMap;
use nucleo::{ErrorQuetzal, Ubicacion};
use rust_decimal::Decimal;

use crate::bucle_eventos::{BucleEventos, CargaNativa, Mensaje};
use crate::cierres::ControlCierre;
use crate::codigo::{CacheSitio, CodigoRuntime, clave_clase, tipo_cache};
use crate::errores::{DatosExcepcion, Fallo};
use crate::nativos::{IdNativo, RegistroNativos};
use crate::valores::{
    ClaseObjeto, DatosMetodoEnlazado, DatosTarea, EnlaceEntorno, EntornoModulo, EstadoIterador,
    Instancia, MetodoRuntime, Valor, Variable, carga_a_valor, texto_de_valor,
};

const LIMITE_PROFUNDIDAD: usize = 10_000;
static SIGUIENTE_ID_CLASE: AtomicU64 = AtomicU64::new(1);

type Despachador = Rc<dyn Fn(&mut Vm, u64, CargaNativa) -> CargaNativa>;

/// Máquina virtual del Lenguaje Quetzal.
pub struct Vm {
    pub nativos: Rc<RegistroNativos>,
    pub modulos: IndexMap<String, Rc<EntornoModulo>>,
    /// Pila única: locales de cada marco en `[base, base+ranuras)` y operandos encima.
    pub(crate) pila: Vec<Valor>,
    pub(crate) optimizar: bool,
    marcos: Vec<Marco>,
    bucle: BucleEventos,
    resultados_pendientes: RefCell<HashMap<u64, Result<CargaNativa, String>>>,
    despachadores: RefCell<HashMap<String, Despachador>>,
}

struct Marco {
    codigo: Rc<CodigoRuntime>,
    pc: u32,
    base: u32,
    ranuras: u16,
    entorno: Rc<EntornoModulo>,
    esto: Option<Valor>,
    manejadores: Vec<Manejador>,
    valor_al_volver: Option<Valor>,
}

enum DestinoLlamada {
    Valor(Valor),
    Marco(Marco),
}

struct Manejador {
    pc: u32,
    profundidad_pila: usize,
    ranuras_locales: u16,
}

enum Flujo {
    Continuar,
    SinAvanzar,
    Retornar(Valor),
    Llamar(Marco),
}

impl Vm {
    pub fn nueva(nativos: Rc<RegistroNativos>) -> Self {
        Self {
            nativos,
            modulos: IndexMap::new(),
            pila: Vec::with_capacity(256),
            optimizar: true,
            marcos: Vec::new(),
            bucle: BucleEventos::nuevo(),
            resultados_pendientes: RefCell::new(HashMap::new()),
            despachadores: RefCell::new(HashMap::new()),
        }
    }

    pub fn sin_optimizar(mut self) -> Self {
        self.optimizar = false;
        self
    }

    pub fn bucle(&self) -> &BucleEventos {
        &self.bucle
    }

    pub fn registrar_despachador<F>(&self, servicio: impl Into<String>, funcion: F)
    where
        F: Fn(&mut Vm, u64, CargaNativa) -> CargaNativa + 'static,
    {
        self.despachadores
            .borrow_mut()
            .entry(servicio.into())
            .or_insert_with(|| Rc::new(funcion));
    }

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

    pub fn cargar_modulo(
        &mut self,
        modulo: Rc<ModuloCompilado>,
        importaciones: IndexMap<String, Variable>,
    ) -> Result<(Rc<EntornoModulo>, Valor), ErrorQuetzal> {
        let constantes = modulo.constantes.iter().map(valor_de_constante).collect();
        let nombres = modulo
            .nombres
            .iter()
            .map(|nombre| Rc::<str>::from(nombre.as_str()))
            .collect();
        let entorno = Rc::new(EntornoModulo {
            modulo: Rc::clone(&modulo),
            globales: RefCell::new(IndexMap::new()),
            valores_globales: RefCell::new(Vec::new()),
            constantes,
            nombres,
            runtimes: RefCell::new(HashMap::new()),
        });

        let mut runtimes = HashMap::new();
        for funcion in &modulo.funciones {
            let rc = Rc::new(funcion.clone());
            runtimes.insert(funcion.nombre.clone(), CodigoRuntime::de_funcion(&rc));
        }
        runtimes.insert(
            String::new(),
            CodigoRuntime::de_trozo(&modulo.principal, modulo.ranuras_locales_principal, None),
        );
        *entorno.runtimes.borrow_mut() = runtimes;

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
                    Valor::funcion_local(Rc::new((*funcion).clone()), &entorno)
                } else if modulo.objetos.iter().any(|objeto| objeto.nombre == *nombre) {
                    Valor::Nulo
                } else if let Some(variable) = importaciones.shift_remove(nombre) {
                    variable.valor.retener_entorno()
                } else {
                    Valor::Nulo
                };
                let variable = Variable { valor, mutable };
                globales.insert(nombre.clone(), variable.clone());
                valores_globales.push(variable);
            }
            for (nombre, mut variable) in importaciones {
                variable.valor = variable.valor.retener_entorno();
                globales.entry(nombre).or_insert(variable);
            }
        }
        *entorno.valores_globales.borrow_mut() = valores_globales;

        let mut clases = Vec::new();
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
                variable.valor = Valor::Clase(Rc::clone(&clase));
            }
            clases.push(clase);
        }
        let mut visitados = HashSet::new();
        for clase in &clases {
            aplanar_metodos(self, clase, &mut visitados);
        }
        visitados.clear();
        for clase in &clases {
            aplanar_slots(self, clase, &mut visitados);
        }

        let runtime_principal = entorno
            .runtimes
            .borrow()
            .get("")
            .cloned()
            .expect("runtime principal");
        let valor = self
            .correr_codigo(
                runtime_principal,
                Rc::clone(&entorno),
                modulo.ranuras_locales_principal,
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
        let mut slots = IndexMap::new();
        for atributo in &objeto.atributos {
            if atributo.libre {
                continue;
            }
            let slot = slots.len() as u16;
            slots.insert(Rc::from(atributo.nombre.as_str()), slot);
        }
        let mut codigos_metodo = HashMap::new();
        for metodo in &objeto.metodos {
            let rc = Rc::new(metodo.funcion.clone());
            codigos_metodo.insert(
                metodo.funcion.nombre.clone(),
                CodigoRuntime::de_funcion(&rc),
            );
        }
        let clase = Rc::new(ClaseObjeto {
            id: SIGUIENTE_ID_CLASE.fetch_add(1, Ordering::Relaxed),
            compilado: objeto.clone(),
            entorno: EnlaceEntorno::debil(entorno),
            atributos_libres: Rc::new(RefCell::new(IndexMap::new())),
            metodos: Rc::new(RefCell::new(metodos)),
            slots: Rc::new(RefCell::new(slots)),
            constructores,
            codigos_metodo: Rc::new(RefCell::new(codigos_metodo)),
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

    fn ejecutar_trozo_aislado(
        &mut self,
        trozo: &Trozo,
        entorno: &Rc<EntornoModulo>,
        esto: Option<Valor>,
    ) -> Result<Valor, Fallo> {
        let codigo = CodigoRuntime::de_trozo(trozo, 0, None);
        self.correr_codigo(codigo, Rc::clone(entorno), 0, esto)
    }

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
            Valor::Tarea(tarea) => {
                let entorno = tarea.entorno.resolver().ok_or_else(|| {
                    self.excepcion(
                        "E0201",
                        "el entorno de la tarea ya fue liberado".to_string(),
                        Some(ubicacion),
                    )
                })?;
                Ok(DestinoLlamada::Marco(self.preparar_marco_funcion(
                    &tarea.funcion,
                    &entorno,
                    tarea.argumentos.clone(),
                    tarea.instancia.clone(),
                    Some(ubicacion),
                )?))
            }
            Valor::TareaNativa(tarea) => Ok(DestinoLlamada::Valor(
                self.resolver_tarea_nativa(tarea.id, ubicacion)?,
            )),
            otro => Ok(DestinoLlamada::Valor(otro)),
        }
    }

    fn runtime_de(
        &self,
        funcion: &Rc<FuncionCompilada>,
        entorno: &Rc<EntornoModulo>,
    ) -> Rc<CodigoRuntime> {
        if let Some(codigo) = entorno.runtimes.borrow().get(&funcion.nombre) {
            return Rc::clone(codigo);
        }
        CodigoRuntime::de_funcion(funcion)
    }

    fn preparar_marco_funcion(
        &mut self,
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
        let codigo = self.runtime_de(funcion, entorno);
        let base = self.pila.len() as u32;
        let ranuras = funcion.ranuras_locales.max(argumentos.len() as u16);
        self.pila.extend(argumentos);
        let needed = base as usize + ranuras as usize;
        if self.pila.len() < needed {
            self.pila.resize(needed, Valor::Nulo);
        }
        Ok(Marco {
            codigo,
            pc: 0,
            base,
            ranuras,
            entorno: Rc::clone(entorno),
            esto,
            manejadores: Vec::new(),
            valor_al_volver: None,
        })
    }

    fn preparar_marco_en_pila(
        &mut self,
        funcion: &Rc<FuncionCompilada>,
        entorno: &Rc<EntornoModulo>,
        cantidad: usize,
        esto: Option<Valor>,
        ubicacion: Option<Ubicacion>,
    ) -> Result<Marco, Fallo> {
        if cantidad != funcion.parametros.len() {
            return Err(self.excepcion(
                "E0210",
                format!(
                    "la función '{}' espera {} argumentos, pero recibió {}",
                    funcion.nombre,
                    funcion.parametros.len(),
                    cantidad
                ),
                ubicacion,
            ));
        }
        let codigo = self.runtime_de(funcion, entorno);
        let base = self.pila.len() - cantidad;
        let ranuras = funcion.ranuras_locales.max(cantidad as u16);
        let necesarios = base + ranuras as usize;
        if self.pila.len() < necesarios {
            self.pila.resize(necesarios, Valor::Nulo);
        }
        Ok(Marco {
            codigo,
            pc: 0,
            base: base as u32,
            ranuras,
            entorno: Rc::clone(entorno),
            esto,
            manejadores: Vec::new(),
            valor_al_volver: None,
        })
    }

    fn preparar_llamada_sincrona_en_pila(
        &mut self,
        cantidad: usize,
        ubicacion: Ubicacion,
    ) -> Result<Option<Marco>, Fallo> {
        let Some(indice) = cantidad
            .checked_add(1)
            .and_then(|total| self.pila.len().checked_sub(total))
        else {
            return Ok(None);
        };
        let (funcion, entorno, esto) = match self.pila.get(indice) {
            Some(Valor::Funcion(datos)) if !datos.funcion.asincrona => {
                let entorno = datos.entorno.resolver().ok_or_else(|| {
                    self.excepcion(
                        "E0201",
                        "el entorno de la función ya fue liberado".to_string(),
                        Some(ubicacion),
                    )
                })?;
                (Rc::clone(&datos.funcion), entorno, None)
            }
            Some(Valor::MetodoEnlazado(datos)) if !datos.funcion.asincrona => {
                let entorno = datos.entorno.resolver().ok_or_else(|| {
                    self.excepcion(
                        "E0201",
                        "el entorno del método ya fue liberado".to_string(),
                        Some(ubicacion),
                    )
                })?;
                (
                    Rc::clone(&datos.funcion),
                    entorno,
                    Some(datos.receptor.clone()),
                )
            }
            _ => return Ok(None),
        };
        self.pila.remove(indice);
        self.preparar_marco_en_pila(&funcion, &entorno, cantidad, esto, Some(ubicacion))
            .map(Some)
    }

    #[cold]
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
        if self.marcos.len() >= LIMITE_PROFUNDIDAD {
            return Err(self.error_desborde_pila(ubicacion));
        }
        let base = self.marcos.len();
        self.marcos.push(marco);
        self.correr_hasta(base)
    }

    fn correr_codigo(
        &mut self,
        codigo: Rc<CodigoRuntime>,
        entorno: Rc<EntornoModulo>,
        ranuras: u16,
        esto: Option<Valor>,
    ) -> Result<Valor, Fallo> {
        let base_pila = self.pila.len() as u32;
        self.pila
            .resize(base_pila as usize + ranuras as usize, Valor::Nulo);
        let base = self.marcos.len();
        self.marcos.push(Marco {
            codigo,
            pc: 0,
            base: base_pila,
            ranuras,
            entorno,
            esto,
            manejadores: Vec::new(),
            valor_al_volver: None,
        });
        self.correr_hasta(base)
    }

    #[allow(dead_code)]
    fn correr_trozo(
        &mut self,
        trozo: Rc<Trozo>,
        entorno: Rc<EntornoModulo>,
        locales: Vec<Valor>,
        esto: Option<Valor>,
    ) -> Result<Valor, Fallo> {
        let ranuras = locales.len() as u16;
        let codigo = CodigoRuntime::de_trozo(&trozo, ranuras, None);
        let base_pila = self.pila.len() as u32;
        self.pila.extend(locales);
        if self.pila.len() < base_pila as usize + ranuras as usize {
            self.pila
                .resize(base_pila as usize + ranuras as usize, Valor::Nulo);
        }
        let base = self.marcos.len();
        self.marcos.push(Marco {
            codigo,
            pc: 0,
            base: base_pila,
            ranuras,
            entorno,
            esto,
            manejadores: Vec::new(),
            valor_al_volver: None,
        });
        self.correr_hasta(base)
    }

    #[cold]
    #[inline(never)]
    fn excepcion(&self, codigo: &str, mensaje: String, ubicacion: Option<Ubicacion>) -> Fallo {
        Fallo::Excepcion(DatosExcepcion {
            mensaje,
            llamadas: self.traza_llamadas(),
            codigo: Some(codigo.to_string()),
            ubicacion,
        })
    }

    fn traza_llamadas(&self) -> Vec<String> {
        self.marcos
            .iter()
            .filter_map(|marco| {
                marco
                    .codigo
                    .nombre
                    .as_ref()
                    .map(|nombre| nombre.to_string())
            })
            .collect()
    }

    fn ubi(&self, top: usize) -> Ubicacion {
        let marco = &self.marcos[top];
        marco
            .codigo
            .ubicaciones
            .get(marco.pc as usize)
            .copied()
            .unwrap_or_default()
    }

    fn correr_hasta(&mut self, base: usize) -> Result<Valor, Fallo> {
        loop {
            if self.marcos.len() <= base {
                return Ok(self.pila.pop().unwrap_or(Valor::Nulo));
            }
            let top = self.marcos.len() - 1;
            let pc = self.marcos[top].pc as usize;
            if pc >= self.marcos[top].codigo.instrucciones.len() {
                let valor = self.pila.pop().unwrap_or(Valor::Nulo);
                if let Some(valor) = self.completar_retorno(base, valor) {
                    return Ok(valor);
                }
                continue;
            }

            let instruccion = self.marcos[top].codigo.instruccion(pc, self.optimizar);
            let resultado = self.ejecutar_instruccion(instruccion, top);
            match resultado {
                Ok(Flujo::Continuar) => {
                    self.marcos[top].pc += 1;
                }
                Ok(Flujo::SinAvanzar) => {}
                Ok(Flujo::Retornar(valor)) => {
                    if let Some(valor) = self.completar_retorno(base, valor) {
                        return Ok(valor);
                    }
                }
                Ok(Flujo::Llamar(callee)) => {
                    if self.marcos.len() >= LIMITE_PROFUNDIDAD {
                        let ubicacion = self.ubi(top);
                        let datos = match self.error_desborde_pila(Some(ubicacion)) {
                            Fallo::Excepcion(datos) => datos,
                            otro => {
                                self.descartar_hasta(base);
                                return Err(otro);
                            }
                        };
                        self.propagar_excepcion(base, datos, ubicacion)?;
                        continue;
                    }
                    self.marcos[top].pc += 1;
                    self.marcos.push(callee);
                }
                Err(Fallo::Excepcion(datos)) => {
                    let ubicacion = self.ubi(top);
                    self.propagar_excepcion(base, datos, ubicacion)?;
                }
                Err(otro) => {
                    self.descartar_hasta(base);
                    return Err(otro);
                }
            }
        }
    }

    fn completar_retorno(&mut self, base: usize, valor: Valor) -> Option<Valor> {
        let Some(marco) = self.marcos.pop() else {
            return Some(valor);
        };
        if marco.valor_al_volver.is_some()
            && let Some(Valor::Instancia(instancia)) = &marco.esto
        {
            instancia.borrow_mut().inicializando = false;
        }
        let valor = marco.valor_al_volver.unwrap_or(valor);
        self.pila.truncate(marco.base as usize);
        if self.marcos.len() == base {
            Some(valor)
        } else {
            self.pila.push(valor);
            None
        }
    }

    fn propagar_excepcion(
        &mut self,
        base: usize,
        mut datos: DatosExcepcion,
        ubicacion: Ubicacion,
    ) -> Result<(), Fallo> {
        if datos.ubicacion.is_none() {
            datos.ubicacion = Some(ubicacion);
        }
        if datos.llamadas.is_empty() {
            datos.llamadas = self.traza_llamadas();
        }
        loop {
            let Some(marco) = self.marcos.last_mut() else {
                return Err(Fallo::Excepcion(datos));
            };
            if let Some(manejador) = marco.manejadores.pop() {
                self.pila.truncate(manejador.profundidad_pila);
                limpiar_locales(
                    &mut self.pila,
                    marco.base,
                    manejador.ranuras_locales,
                    marco.ranuras,
                );
                self.pila.push(Valor::Excepcion(Rc::new(datos)));
                marco.pc = manejador.pc;
                return Ok(());
            }
            let marco = self.marcos.pop().unwrap();
            self.pila.truncate(marco.base as usize);
            if self.marcos.len() == base {
                return Err(Fallo::Excepcion(datos));
            }
        }
    }

    fn descartar_hasta(&mut self, base: usize) {
        while self.marcos.len() > base {
            if let Some(marco) = self.marcos.pop() {
                self.pila.truncate(marco.base as usize);
            }
        }
    }

    fn aplicar_destino(&mut self, destino: DestinoLlamada) -> Flujo {
        match destino {
            DestinoLlamada::Valor(valor) => {
                self.pila.push(valor);
                Flujo::Continuar
            }
            DestinoLlamada::Marco(marco) => Flujo::Llamar(marco),
        }
    }

    pub(crate) fn ejecutar_op_cierre(
        &mut self,
        instruccion: Instruccion,
        pc: u32,
    ) -> Result<ControlCierre, Fallo> {
        let top = self.marcos.len().saturating_sub(1);
        if let Some(marco) = self.marcos.last_mut() {
            marco.pc = pc;
        }
        match self.ejecutar_instruccion(instruccion, top)? {
            Flujo::Continuar => Ok(ControlCierre::Continuar),
            Flujo::SinAvanzar => {
                let pc = self.marcos.get(top).map(|m| m.pc).unwrap_or(pc);
                Ok(ControlCierre::Saltar(pc))
            }
            Flujo::Retornar(valor) => Ok(ControlCierre::Retornar(valor)),
            Flujo::Llamar(_) => Ok(ControlCierre::Interpretar),
        }
    }

    fn ejecutar_instruccion(
        &mut self,
        instruccion: Instruccion,
        top: usize,
    ) -> Result<Flujo, Fallo> {
        match instruccion.op {
            CodigoOp::CargarConstante => {
                let constante =
                    self.marcos[top].entorno.constantes[instruccion.indice_u32() as usize].clone();
                self.pila.push(constante);
            }
            CodigoOp::CargarEntero => self
                .pila
                .push(Valor::Entero(instruccion.entero_inmediato() as i64)),
            CodigoOp::Duplicar => {
                let tope = self.tope()?.clone();
                self.pila.push(tope);
            }
            CodigoOp::Desechar => {
                self.sacar()?;
            }
            CodigoOp::CargarLocal => {
                let valor = self.local(top, instruccion.ranura())?;
                self.pila.push(valor);
            }
            CodigoOp::GuardarLocal => {
                let valor = self.sacar()?;
                self.set_local(top, instruccion.ranura(), valor)?;
            }
            CodigoOp::LimpiarLocales => {
                let marco = &self.marcos[top];
                limpiar_locales(
                    &mut self.pila,
                    marco.base,
                    instruccion.ranura(),
                    marco.ranuras,
                );
            }
            CodigoOp::AjustarLocal => {
                let ranura = instruccion.ranura();
                let delta = instruccion.delta_ajuste() as i64;
                let valor = self.local_mut(top, ranura)?;
                let Valor::Entero(actual) = valor else {
                    let copia = valor.clone();
                    return Err(self.error_se_esperaba_entero(&copia, self.ubi(top)));
                };
                let Some(nuevo) = actual.checked_add(delta) else {
                    let actual = *actual;
                    return Err(self.excepcion(
                        "E0402",
                        format!("desbordamiento de entero al ajustar {actual} en {delta}"),
                        Some(self.ubi(top)),
                    ));
                };
                *actual = nuevo;
            }
            CodigoOp::CargarGlobal => {
                let valor = self.cargar_global(top, instruccion.indice_u16())?;
                self.pila.push(valor);
            }
            CodigoOp::DeclararGlobal => {
                let valor = self.sacar()?;
                self.declarar_global(top, instruccion.indice_u16(), valor)?;
            }
            CodigoOp::GuardarGlobal => {
                let valor = self.sacar()?;
                self.guardar_global(top, instruccion.indice_u16(), valor)?;
            }
            CodigoOp::CargarExterno => {
                let nombre =
                    Rc::clone(&self.marcos[top].entorno.nombres[instruccion.indice_u32() as usize]);
                let valor = self.buscar_externo(top, &nombre).ok_or_else(|| {
                    self.excepcion(
                        "E0201",
                        format!("'{nombre}' no está definido"),
                        Some(self.ubi(top)),
                    )
                })?;
                self.pila.push(valor);
            }
            CodigoOp::GuardarExterno => {
                let valor = self.sacar()?;
                let nombre =
                    Rc::clone(&self.marcos[top].entorno.nombres[instruccion.indice_u32() as usize]);
                self.guardar_externo(top, &nombre, valor)?;
            }
            _ if instruccion.es_binaria_generica() => {
                let derecha = self.sacar()?;
                let izquierda = self.sacar()?;
                if self.optimizar {
                    self.especializar_binaria(top, instruccion, &izquierda, &derecha);
                }
                self.pila.push(self.operacion_binaria(
                    instruccion,
                    izquierda,
                    derecha,
                    self.ubi(top),
                )?);
            }
            _ if instruccion.es_binaria_entera() => {
                let pc = self.marcos[top].pc as usize;
                let original = self.marcos[top].codigo.instrucciones[pc];
                if self.optimizar && original.es_binaria_generica() {
                    let operandos_enteros = self.pila.len() >= 2
                        && matches!(self.pila[self.pila.len() - 2], Valor::Entero(_))
                        && matches!(self.pila[self.pila.len() - 1], Valor::Entero(_));
                    if !operandos_enteros {
                        self.marcos[top].codigo.desespecializar(pc);
                        self.marcos[top].codigo.caches[pc].set(CacheSitio::Polimorfica);
                        return self.ejecutar_instruccion(original, top);
                    }
                }
                let derecha = self.sacar()?;
                let izquierda = self.sacar()?;
                self.pila.push(self.operacion_entera(
                    instruccion,
                    izquierda,
                    derecha,
                    self.ubi(top),
                )?);
            }
            CodigoOp::Negar => {
                let valor = self.sacar()?;
                self.pila.push(self.negar(valor, self.ubi(top))?);
            }
            CodigoOp::NegarEntero => {
                let valor = self.sacar()?;
                self.pila.push(self.negar_entero(valor, self.ubi(top))?);
            }
            CodigoOp::NoLogico => {
                let valor = self.sacar()?;
                let logico = self.como_logico(&valor, self.ubi(top))?;
                self.pila.push(Valor::Log(!logico));
            }
            CodigoOp::Saltar => {
                self.marcos[top].pc = instruccion.destino();
                return Ok(Flujo::SinAvanzar);
            }
            CodigoOp::SaltarSiFalso => {
                let valor = self.sacar()?;
                if !self.como_logico(&valor, self.ubi(top))? {
                    self.marcos[top].pc = instruccion.destino();
                    return Ok(Flujo::SinAvanzar);
                }
            }
            CodigoOp::SaltarSiVerdadero => {
                let valor = self.sacar()?;
                if self.como_logico(&valor, self.ubi(top))? {
                    self.marcos[top].pc = instruccion.destino();
                    return Ok(Flujo::SinAvanzar);
                }
            }
            CodigoOp::SaltarSiFalsoYDejar => {
                let valor = self.tope()?.clone();
                if !self.como_logico(&valor, self.ubi(top))? {
                    self.marcos[top].pc = instruccion.destino();
                    return Ok(Flujo::SinAvanzar);
                }
                self.sacar()?;
            }
            CodigoOp::SaltarSiVerdaderoYDejar => {
                let valor = self.tope()?.clone();
                if self.como_logico(&valor, self.ubi(top))? {
                    self.marcos[top].pc = instruccion.destino();
                    return Ok(Flujo::SinAvanzar);
                }
                self.sacar()?;
            }
            CodigoOp::MenorEnteroSaltarSiFalso => {
                let derecha = self.sacar()?;
                let izquierda = self.sacar()?;
                let Valor::Entero(a) = izquierda else {
                    return Err(self.error_se_esperaba_entero(&izquierda, self.ubi(top)));
                };
                let Valor::Entero(b) = derecha else {
                    return Err(self.error_se_esperaba_entero(&derecha, self.ubi(top)));
                };
                if a < b {
                    // condition true: don't jump (same as Menor then SaltarSiFalso)
                } else {
                    self.marcos[top].pc = instruccion.destino();
                    return Ok(Flujo::SinAvanzar);
                }
            }
            CodigoOp::CargarLocalLocalSumarEntero => {
                let a = self.local(top, instruccion.ranura())?;
                let b = self.local(top, instruccion.ranura_derecha())?;
                self.pila.push(self.operacion_entera(
                    Instruccion::SumarEntero(),
                    a,
                    b,
                    self.ubi(top),
                )?);
            }
            CodigoOp::RetornarLocal => {
                let valor = self.local(top, instruccion.ranura())?;
                return Ok(Flujo::Retornar(valor));
            }
            CodigoOp::Llamar => {
                let cantidad = instruccion.cantidad() as usize;
                if let Some(marco) =
                    self.preparar_llamada_sincrona_en_pila(cantidad, self.ubi(top))?
                {
                    return Ok(Flujo::Llamar(marco));
                }
                let argumentos = self.sacar_varios(cantidad)?;
                let objetivo = self.sacar()?;
                let destino = self.llamar_valor(objetivo, argumentos, self.ubi(top))?;
                return Ok(self.aplicar_destino(destino));
            }
            CodigoOp::LlamarMetodo => {
                let nombre = Rc::clone(
                    &self.marcos[top].entorno.nombres[instruccion.indice_nombre() as usize],
                );
                let cantidad = instruccion.cantidad_metodo() as usize;
                let argumentos = self.sacar_varios(cantidad)?;
                let receptor = self.sacar()?;
                if self.optimizar {
                    self.especializar_llamada_metodo(top, instruccion, &receptor, &nombre);
                }
                let destino =
                    self.llamar_metodo(receptor, &nombre, argumentos, self.ubi(top), top)?;
                return Ok(self.aplicar_destino(destino));
            }
            CodigoOp::LlamarNativa => {
                let id = IdNativo(instruccion.id_nativo() as u32);
                let cantidad = instruccion.cantidad() as usize;
                let pc = self.marcos[top].pc as usize;
                let original = self.marcos[top].codigo.instrucciones[pc];
                if original.op == CodigoOp::LlamarMetodo {
                    let receptor = self
                        .pila
                        .len()
                        .checked_sub(cantidad)
                        .and_then(|indice| self.pila.get(indice));
                    let guarda = match (self.marcos[top].codigo.caches[pc].get(), receptor) {
                        (CacheSitio::Nativa { tipo, id: esperado }, Some(receptor)) => {
                            esperado == id
                                && tipo == tipo_cache(receptor.nombre_tipo())
                                && cantidad == original.cantidad_metodo() as usize + 1
                        }
                        _ => false,
                    };
                    if !guarda {
                        self.marcos[top].codigo.desespecializar(pc);
                        self.marcos[top].codigo.caches[pc].set(CacheSitio::Polimorfica);
                        return self.ejecutar_instruccion(original, top);
                    }
                }
                let argumentos = self.sacar_varios(cantidad)?;
                let valor = self.llamar_nativa_id(id, &argumentos, self.ubi(top))?;
                self.pila.push(valor);
            }
            CodigoOp::Retornar => {
                let valor = self.pila.pop().unwrap_or(Valor::Nulo);
                return Ok(Flujo::Retornar(valor));
            }
            CodigoOp::CargarMiembro => {
                let nombre =
                    Rc::clone(&self.marcos[top].entorno.nombres[instruccion.indice_u32() as usize]);
                let objeto = self.sacar()?;
                if self.optimizar {
                    self.cachear_miembro(top, instruccion, &objeto, &nombre);
                }
                let miembro = self.cargar_miembro(objeto, &nombre, self.ubi(top))?;
                self.pila.push(miembro);
            }
            CodigoOp::GuardarMiembro => {
                let valor = self.sacar()?;
                let objeto = self.sacar()?;
                let nombre =
                    Rc::clone(&self.marcos[top].entorno.nombres[instruccion.indice_u32() as usize]);
                self.guardar_miembro(objeto, &nombre, valor, self.ubi(top))?;
            }
            CodigoOp::CargarAtributo => {
                let pc = self.marcos[top].pc as usize;
                let original = self.marcos[top].codigo.instrucciones[pc];
                if original.op == CodigoOp::CargarMiembro {
                    let clase_actual = match self.tope()? {
                        Valor::Instancia(instancia) => Some(instancia.borrow().clase.id),
                        _ => None,
                    };
                    let guarda = match self.marcos[top].codigo.caches[pc].get() {
                        CacheSitio::Atributo { clase, slot } => {
                            slot == instruccion.slot() && Some(clase) == clase_actual
                        }
                        _ => false,
                    };
                    if !guarda {
                        self.marcos[top].codigo.desespecializar(pc);
                        self.marcos[top].codigo.caches[pc].set(CacheSitio::Polimorfica);
                        return self.ejecutar_instruccion(original, top);
                    }
                }
                let objeto = self.sacar()?;
                let atributo = self.cargar_atributo(objeto, instruccion.slot(), self.ubi(top))?;
                self.pila.push(atributo);
            }
            CodigoOp::GuardarAtributo => {
                let valor = self.sacar()?;
                let objeto = self.sacar()?;
                self.guardar_atributo(objeto, instruccion.slot(), valor, self.ubi(top))?;
            }
            CodigoOp::CargarEstoAtributo => {
                let esto = self.esto(top)?;
                let atributo = self.cargar_atributo(esto, instruccion.slot(), self.ubi(top))?;
                self.pila.push(atributo);
            }
            CodigoOp::Indexar => {
                let indice = self.sacar()?;
                let objeto = self.sacar()?;
                let indexado = self.indexar(objeto, indice, self.ubi(top))?;
                self.pila.push(indexado);
            }
            CodigoOp::GuardarIndice => {
                let valor = self.sacar()?;
                let indice = self.sacar()?;
                let objeto = self.sacar()?;
                self.guardar_indice(objeto, indice, valor, self.ubi(top))?;
            }
            CodigoOp::CrearLista => {
                let elementos = self.sacar_varios(instruccion.cantidad() as usize)?;
                self.pila.push(Valor::lista(elementos));
            }
            CodigoOp::CrearJsn => {
                let pares = self.sacar_varios(instruccion.cantidad() as usize * 2)?;
                let mut mapa = IndexMap::new();
                for par in pares.chunks(2) {
                    mapa.insert(texto_de_valor(&par[0]), par[1].clone());
                }
                self.pila.push(Valor::jsn(mapa));
            }
            CodigoOp::Interpolar => {
                let n = instruccion.cantidad() as usize;
                let start = self.pila.len().saturating_sub(n);
                let mut texto = String::with_capacity(n.saturating_mul(8));
                for valor in &self.pila[start..] {
                    texto.push_str(&texto_de_valor(valor));
                }
                self.pila.truncate(start);
                self.pila.push(Valor::texto(texto));
            }
            CodigoOp::Instanciar => {
                let argumentos = self.sacar_varios(instruccion.cantidad() as usize)?;
                let valor_clase = self.sacar()?;
                match valor_clase {
                    Valor::Clase(clase) => {
                        let destino = self.instanciar(&clase, argumentos, self.ubi(top))?;
                        return Ok(self.aplicar_destino(destino));
                    }
                    Valor::ModuloNativo(modulo) => {
                        let id =
                            self.nativos
                                .metodo_de(&modulo, "constructor")
                                .ok_or_else(|| {
                                    self.excepcion(
                                        "E0406",
                                        format!("'{modulo}' no es un objeto instanciable"),
                                        Some(self.ubi(top)),
                                    )
                                })?;
                        let instancia = self.llamar_nativa_id(id, &argumentos, self.ubi(top))?;
                        self.pila.push(instancia);
                    }
                    otro => {
                        return Err(self.excepcion(
                            "E0406",
                            format!(
                                "un valor de tipo '{}' no es un objeto instanciable",
                                otro.nombre_tipo()
                            ),
                            Some(self.ubi(top)),
                        ));
                    }
                }
            }
            CodigoOp::CargarEsto => {
                let esto = self.esto(top)?;
                self.pila.push(esto);
            }
            CodigoOp::CargarPadre => {
                let esto = self.esto(top)?;
                let Valor::Instancia(instancia) = esto else {
                    return Err(self.excepcion(
                        "E0212",
                        "'padre' solo existe dentro de una instancia".to_string(),
                        Some(self.ubi(top)),
                    ));
                };
                self.pila.push(Valor::padre(instancia, None));
            }
            CodigoOp::CrearIterador => {
                let valor = self.sacar()?;
                let elementos = match &valor {
                    Valor::Lista(lista) => Rc::clone(&lista.borrow().elementos),
                    Valor::Jsn(mapa) => Rc::new(mapa.borrow().keys().map(Valor::texto).collect()),
                    otro => {
                        return Err(self.excepcion(
                            "E0406",
                            format!(
                                "no se puede iterar un valor de tipo '{}'",
                                otro.nombre_tipo()
                            ),
                            Some(self.ubi(top)),
                        ));
                    }
                };
                self.pila
                    .push(Valor::Iterador(Rc::new(RefCell::new(EstadoIterador {
                        elementos,
                        posicion: 0,
                    }))));
            }
            CodigoOp::IteradorSiguiente => {
                let valor = self.sacar()?;
                let Valor::Iterador(iterador) = valor else {
                    return Err(self.excepcion(
                        "E0406",
                        "se esperaba un iterador".to_string(),
                        Some(self.ubi(top)),
                    ));
                };
                let mut estado = iterador.borrow_mut();
                if estado.posicion < estado.elementos.len() {
                    let elemento = estado.elementos[estado.posicion].clone();
                    estado.posicion += 1;
                    self.pila.push(elemento);
                } else {
                    self.marcos[top].pc = instruccion.destino();
                    return Ok(Flujo::SinAvanzar);
                }
            }
            CodigoOp::EntrarIntentar => {
                self.marcos[top].manejadores.push(Manejador {
                    pc: instruccion.destino(),
                    profundidad_pila: self.pila.len(),
                    ranuras_locales: instruccion.ranuras_intentar(),
                });
            }
            CodigoOp::SalirIntentar => {
                self.marcos[top].manejadores.pop();
            }
            CodigoOp::Lanzar => {
                let valor = self.sacar()?;
                let datos = match valor {
                    Valor::Excepcion(datos) => (*datos).clone(),
                    otro => DatosExcepcion {
                        mensaje: texto_de_valor(&otro),
                        llamadas: self.traza_llamadas(),
                        codigo: None,
                        ubicacion: Some(self.ubi(top)),
                    },
                };
                return Err(Fallo::Excepcion(datos));
            }
            CodigoOp::Esperar => {
                let valor = self.sacar()?;
                let destino = self.destino_esperar(valor, self.ubi(top))?;
                return Ok(self.aplicar_destino(destino));
            }
            _ => {}
        }
        Ok(Flujo::Continuar)
    }

    fn local(&self, top: usize, ranura: u16) -> Result<Valor, Fallo> {
        let indice = self.marcos[top].base as usize + ranura as usize;
        self.pila.get(indice).cloned().ok_or_else(|| {
            Fallo::Error(Box::new(ErrorQuetzal::interno(
                "ranura local fuera de rango (bytecode inválido)",
            )))
        })
    }

    fn set_local(&mut self, top: usize, ranura: u16, valor: Valor) -> Result<(), Fallo> {
        let indice = self.marcos[top].base as usize + ranura as usize;
        if indice >= self.pila.len() {
            return Err(Fallo::Error(Box::new(ErrorQuetzal::interno(
                "ranura local fuera de rango (bytecode inválido)",
            ))));
        }
        self.pila[indice] = valor;
        Ok(())
    }

    fn local_mut(&mut self, top: usize, ranura: u16) -> Result<&mut Valor, Fallo> {
        let indice = self.marcos[top].base as usize + ranura as usize;
        self.pila.get_mut(indice).ok_or_else(|| {
            Fallo::Error(Box::new(ErrorQuetzal::interno(
                "ranura local fuera de rango (bytecode inválido)",
            )))
        })
    }

    fn esto(&self, top: usize) -> Result<Valor, Fallo> {
        self.marcos[top].esto.clone().ok_or_else(|| {
            self.excepcion(
                "E0212",
                "'esto' solo existe dentro de un objeto".to_string(),
                Some(self.ubi(top)),
            )
        })
    }

    fn sacar(&mut self) -> Result<Valor, Fallo> {
        self.pila.pop().ok_or_else(|| {
            Fallo::Error(Box::new(ErrorQuetzal::interno(
                "la pila de la VM quedó vacía (bytecode inválido)",
            )))
        })
    }

    fn tope(&self) -> Result<&Valor, Fallo> {
        self.pila.last().ok_or_else(|| {
            Fallo::Error(Box::new(ErrorQuetzal::interno(
                "la pila de la VM quedó vacía (bytecode inválido)",
            )))
        })
    }

    fn sacar_varios(&mut self, cantidad: usize) -> Result<Vec<Valor>, Fallo> {
        if self.pila.len() < cantidad {
            return Err(Fallo::Error(Box::new(ErrorQuetzal::interno(
                "la pila de la VM quedó vacía (bytecode inválido)",
            ))));
        }
        Ok(self.pila.split_off(self.pila.len() - cantidad))
    }

    fn especializar_binaria(&self, top: usize, original: Instruccion, izq: &Valor, der: &Valor) {
        let pc = self.marcos[top].pc as usize;
        if matches!(
            self.marcos[top].codigo.caches[pc].get(),
            CacheSitio::Polimorfica
        ) {
            return;
        }
        let nueva = match (original.op, izq, der) {
            (CodigoOp::Sumar, Valor::Entero(_), Valor::Entero(_)) => Instruccion::SumarEntero(),
            (CodigoOp::Restar, Valor::Entero(_), Valor::Entero(_)) => Instruccion::RestarEntero(),
            (CodigoOp::Multiplicar, Valor::Entero(_), Valor::Entero(_)) => {
                Instruccion::MultiplicarEntero()
            }
            (CodigoOp::Dividir, Valor::Entero(_), Valor::Entero(_)) => Instruccion::DividirEntero(),
            (CodigoOp::Modulo, Valor::Entero(_), Valor::Entero(_)) => Instruccion::ModuloEntero(),
            (CodigoOp::Igual, Valor::Entero(_), Valor::Entero(_)) => Instruccion::IgualEntero(),
            (CodigoOp::Diferente, Valor::Entero(_), Valor::Entero(_)) => {
                Instruccion::DiferenteEntero()
            }
            (CodigoOp::Mayor, Valor::Entero(_), Valor::Entero(_)) => Instruccion::MayorEntero(),
            (CodigoOp::Menor, Valor::Entero(_), Valor::Entero(_)) => Instruccion::MenorEntero(),
            (CodigoOp::MayorOIgual, Valor::Entero(_), Valor::Entero(_)) => {
                Instruccion::MayorOIgualEntero()
            }
            (CodigoOp::MenorOIgual, Valor::Entero(_), Valor::Entero(_)) => {
                Instruccion::MenorOIgualEntero()
            }
            _ => return,
        };
        self.marcos[top].codigo.especializar(pc, nueva);
    }

    fn especializar_llamada_metodo(
        &self,
        top: usize,
        original: Instruccion,
        receptor: &Valor,
        nombre: &str,
    ) {
        let pc = self.marcos[top].pc as usize;
        if matches!(
            self.marcos[top].codigo.caches[pc].get(),
            CacheSitio::Polimorfica
        ) {
            return;
        }
        if let Some(id) = self.nativos.metodo_de(receptor.nombre_tipo(), nombre)
            && id.0 <= u16::MAX as u32
            && let Ok(cantidad) = u16::try_from(original.cantidad_metodo() + 1)
        {
            self.marcos[top].codigo.caches[pc].set(CacheSitio::Nativa {
                tipo: tipo_cache(receptor.nombre_tipo()),
                id,
            });
            self.marcos[top]
                .codigo
                .especializar(pc, Instruccion::LlamarNativa(id.0 as u16, cantidad));
        }
    }

    fn cachear_miembro(&self, top: usize, original: Instruccion, objeto: &Valor, nombre: &str) {
        let pc = self.marcos[top].pc as usize;
        if let Some(id) = self.nativos.metodo_de(objeto.nombre_tipo(), nombre) {
            self.marcos[top].codigo.caches[pc].set(CacheSitio::Nativa {
                tipo: tipo_cache(objeto.nombre_tipo()),
                id,
            });
            let _ = original;
            return;
        }
        let Valor::Instancia(instancia) = objeto else {
            return;
        };
        let clase = Rc::clone(&instancia.borrow().clase);
        let clave = clave_clase(&clase);
        let cache = self.marcos[top].codigo.caches[pc].get();
        match cache {
            CacheSitio::Vacia => {
                if let Some(&slot) = clase.slots.borrow().get(nombre) {
                    self.marcos[top].codigo.caches[pc]
                        .set(CacheSitio::Atributo { clase: clave, slot });
                    self.marcos[top]
                        .codigo
                        .especializar(pc, Instruccion::CargarAtributo(slot));
                }
            }
            CacheSitio::Atributo { clase: previa, .. } if previa != clave => {
                self.marcos[top].codigo.caches[pc].set(CacheSitio::Polimorfica);
                self.marcos[top].codigo.desespecializar(pc);
                let _ = original;
            }
            _ => {}
        }
    }

    fn cargar_global(&self, top: usize, indice: u16) -> Result<Valor, Fallo> {
        self.marcos[top]
            .entorno
            .valores_globales
            .borrow()
            .get(indice as usize)
            .map(|variable| variable.valor.clone())
            .ok_or_else(|| {
                self.excepcion(
                    "E0201",
                    "la global no está definida".to_string(),
                    Some(self.ubi(top)),
                )
            })
    }

    fn declarar_global(&self, top: usize, indice: u16, valor: Valor) -> Result<(), Fallo> {
        let mut valores = self.marcos[top].entorno.valores_globales.borrow_mut();
        let variable = valores.get_mut(indice as usize).ok_or_else(|| {
            self.excepcion(
                "E0201",
                "la global no está definida".to_string(),
                Some(self.ubi(top)),
            )
        })?;
        variable.valor = valor;
        Ok(())
    }

    fn guardar_global(&self, top: usize, indice: u16, valor: Valor) -> Result<(), Fallo> {
        let mut valores = self.marcos[top].entorno.valores_globales.borrow_mut();
        let variable = valores.get_mut(indice as usize).ok_or_else(|| {
            self.excepcion(
                "E0201",
                "la global no está definida".to_string(),
                Some(self.ubi(top)),
            )
        })?;
        if !variable.mutable {
            let nombre = self.marcos[top]
                .entorno
                .modulo
                .nombres_globales
                .get(indice as usize)
                .map(String::as_str)
                .unwrap_or("?");
            return Err(self.error_constante(nombre, self.ubi(top)));
        }
        variable.valor = valor;
        Ok(())
    }

    fn buscar_externo(&self, top: usize, nombre: &str) -> Option<Valor> {
        let entorno = &self.marcos[top].entorno;
        if let Some(indice) = entorno
            .modulo
            .nombres_globales
            .iter()
            .position(|global| global == nombre)
            && let Some(variable) = entorno.valores_globales.borrow().get(indice)
        {
            return Some(variable.valor.clone());
        }
        if let Some(variable) = entorno.globales.borrow().get(nombre) {
            return Some(variable.valor.clone());
        }
        if let Some(id) = self.nativos.id_de(nombre) {
            return Some(Valor::Nativa(id));
        }
        if self.nativos.existe_modulo(nombre) {
            return Some(Valor::modulo_nativo(nombre));
        }
        None
    }

    fn guardar_externo(&self, top: usize, nombre: &str, valor: Valor) -> Result<(), Fallo> {
        let mut globales = self.marcos[top].entorno.globales.borrow_mut();
        if let Some(variable) = globales.get_mut(nombre) {
            if !variable.mutable {
                return Err(self.error_constante(nombre, self.ubi(top)));
            }
            variable.valor = valor;
            return Ok(());
        }
        Err(self.excepcion(
            "E0201",
            format!("'{nombre}' no está definido"),
            Some(self.ubi(top)),
        ))
    }

    fn error_constante(&self, nombre: &str, ubicacion: Ubicacion) -> Fallo {
        self.excepcion(
            "E0203",
            format!("no puedes reasignar la constante '{nombre}'"),
            Some(ubicacion),
        )
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

    fn operacion_binaria(
        &self,
        instruccion: Instruccion,
        izquierda: Valor,
        derecha: Valor,
        ubicacion: Ubicacion,
    ) -> Result<Valor, Fallo> {
        match instruccion.op {
            CodigoOp::Sumar => self.sumar(izquierda, derecha, ubicacion),
            CodigoOp::Restar => self.aritmetica(izquierda, derecha, ubicacion, "restar"),
            CodigoOp::Multiplicar => self.aritmetica(izquierda, derecha, ubicacion, "multiplicar"),
            CodigoOp::Dividir => self.aritmetica(izquierda, derecha, ubicacion, "dividir"),
            CodigoOp::Modulo => self.aritmetica(izquierda, derecha, ubicacion, "modulo"),
            CodigoOp::Igual => Ok(Valor::Log(izquierda.es_igual(&derecha))),
            CodigoOp::Diferente => Ok(Valor::Log(!izquierda.es_igual(&derecha))),
            CodigoOp::Mayor | CodigoOp::Menor | CodigoOp::MayorOIgual | CodigoOp::MenorOIgual => {
                let orden = self.comparar(&izquierda, &derecha, ubicacion)?;
                let resultado = match instruccion.op {
                    CodigoOp::Mayor => orden == std::cmp::Ordering::Greater,
                    CodigoOp::Menor => orden == std::cmp::Ordering::Less,
                    CodigoOp::MayorOIgual => orden != std::cmp::Ordering::Less,
                    CodigoOp::MenorOIgual => orden != std::cmp::Ordering::Greater,
                    _ => unreachable!(),
                };
                Ok(Valor::Log(resultado))
            }
            _ => unreachable!("instrucción binaria inesperada"),
        }
    }

    fn operacion_entera(
        &self,
        instruccion: Instruccion,
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
        match instruccion.op {
            CodigoOp::IgualEntero => Ok(Valor::Log(a == b)),
            CodigoOp::DiferenteEntero => Ok(Valor::Log(a != b)),
            CodigoOp::MayorEntero => Ok(Valor::Log(a > b)),
            CodigoOp::MenorEntero => Ok(Valor::Log(a < b)),
            CodigoOp::MayorOIgualEntero => Ok(Valor::Log(a >= b)),
            CodigoOp::MenorOIgualEntero => Ok(Valor::Log(a <= b)),
            CodigoOp::SumarEntero => {
                self.entero_checked(a.checked_add(b), a, b, "sumar", ubicacion)
            }
            CodigoOp::RestarEntero => {
                self.entero_checked(a.checked_sub(b), a, b, "restar", ubicacion)
            }
            CodigoOp::MultiplicarEntero => {
                self.entero_checked(a.checked_mul(b), a, b, "multiplicar", ubicacion)
            }
            CodigoOp::DividirEntero => {
                if b == 0 {
                    return Err(self.excepcion(
                        "E0401",
                        "división por cero".to_string(),
                        Some(ubicacion),
                    ));
                }
                self.entero_checked(a.checked_div(b), a, b, "dividir", ubicacion)
            }
            CodigoOp::ModuloEntero => {
                if b == 0 {
                    return Err(self.excepcion(
                        "E0401",
                        "módulo por cero".to_string(),
                        Some(ubicacion),
                    ));
                }
                self.entero_checked(a.checked_rem(b), a, b, "modulo", ubicacion)
            }
            _ => unreachable!("instrucción entera inesperada"),
        }
    }

    fn entero_checked(
        &self,
        resultado: Option<i64>,
        izquierda: i64,
        derecha: i64,
        operacion: &str,
        ubicacion: Ubicacion,
    ) -> Result<Valor, Fallo> {
        resultado.map(Valor::Entero).ok_or_else(|| {
            self.excepcion(
                "E0402",
                format!("desbordamiento de entero al {operacion} {izquierda} y {derecha}"),
                Some(ubicacion),
            )
        })
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
            Valor::Numero(decimal) => Ok(Valor::numero(-*decimal)),
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
        if matches!(izquierda, Valor::Texto(_)) || matches!(derecha, Valor::Texto(_)) {
            let mut texto = texto_de_valor(&izquierda);
            texto.push_str(&texto_de_valor(&derecha));
            return Ok(Valor::texto(texto));
        }
        if let (Valor::Lista(a), Valor::Lista(b)) = (&izquierda, &derecha) {
            let mut resultado = a.borrow().to_vec();
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
                resultado.map(Valor::numero).ok_or_else(|| {
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
            Valor::Numero(decimal) => Ok(**decimal),
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
            (Valor::Texto(a), Valor::Texto(b)) => Ok((**a).cmp(&**b)),
            _ => {
                let a = self.como_decimal(izquierda, ubicacion, "comparar")?;
                let b = self.como_decimal(derecha, ubicacion, "comparar")?;
                Ok(a.cmp(&b))
            }
        }
    }

    fn llamar_valor(
        &mut self,
        objetivo: Valor,
        argumentos: Vec<Valor>,
        ubicacion: Ubicacion,
    ) -> Result<DestinoLlamada, Fallo> {
        match objetivo {
            Valor::Funcion(datos) => {
                if datos.funcion.asincrona {
                    return Ok(DestinoLlamada::Valor(Valor::Tarea(Rc::new(DatosTarea {
                        funcion: Rc::clone(&datos.funcion),
                        entorno: datos.entorno.clone(),
                        argumentos,
                        instancia: None,
                    }))));
                }
                let entorno = datos.entorno.resolver().ok_or_else(|| {
                    self.excepcion(
                        "E0201",
                        "el entorno de la función ya fue liberado".to_string(),
                        Some(ubicacion),
                    )
                })?;
                Ok(DestinoLlamada::Marco(self.preparar_marco_funcion(
                    &datos.funcion,
                    &entorno,
                    argumentos,
                    None,
                    Some(ubicacion),
                )?))
            }
            Valor::MetodoEnlazado(datos) => {
                let entorno = datos.entorno.resolver().ok_or_else(|| {
                    self.excepcion(
                        "E0201",
                        "el entorno del método ya fue liberado".to_string(),
                        Some(ubicacion),
                    )
                })?;
                self.invocar_metodo(
                    Rc::clone(&datos.funcion),
                    entorno,
                    argumentos,
                    datos.receptor.clone(),
                    ubicacion,
                )
            }
            Valor::Nativa(id) => self
                .llamar_nativa_id(id, &argumentos, ubicacion)
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

    #[allow(dead_code)]
    fn llamar_nativa(
        &mut self,
        nombre: &str,
        argumentos: &[Valor],
        ubicacion: Ubicacion,
    ) -> Result<Valor, Fallo> {
        let id = self.nativos.id_de(nombre).ok_or_else(|| {
            self.excepcion(
                "E0201",
                format!("la función nativa '{nombre}' no existe"),
                Some(ubicacion),
            )
        })?;
        self.llamar_nativa_id(id, argumentos, ubicacion)
    }

    fn llamar_nativa_id(
        &mut self,
        id: IdNativo,
        argumentos: &[Valor],
        ubicacion: Ubicacion,
    ) -> Result<Valor, Fallo> {
        let nativos = Rc::clone(&self.nativos);
        let resultado = nativos.invocar_con_vm(self, id, argumentos);
        resultado.map_err(|fallo| match fallo {
            Fallo::Excepcion(mut datos) => {
                if datos.ubicacion.is_none() {
                    datos.ubicacion = Some(ubicacion);
                }
                if datos.llamadas.is_empty() {
                    datos.llamadas = self.traza_llamadas();
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
        _top: usize,
    ) -> Result<DestinoLlamada, Fallo> {
        match &receptor {
            Valor::ModuloNativo(modulo) => {
                let id = self.nativos.metodo_de(modulo, nombre).ok_or_else(|| {
                    self.excepcion(
                        "E0201",
                        format!("el módulo '{modulo}' no tiene '{nombre}'"),
                        Some(ubicacion),
                    )
                })?;
                self.llamar_nativa_id(id, &argumentos, ubicacion)
                    .map(DestinoLlamada::Valor)
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
            Valor::Clase(clase) => match buscar_metodo_en_clase(self, clase, nombre) {
                Some((metodo, entorno)) if metodo_es_libre(clase, nombre) => self.invocar_metodo(
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
            },
            Valor::Padre(datos) => self.llamar_metodo_padre(datos, nombre, argumentos, ubicacion),
            Valor::InstanciaNativa(datos) => {
                let id = self.nativos.metodo_de(&datos.tipo, nombre).ok_or_else(|| {
                    self.excepcion(
                        "E0201",
                        format!("el objeto '{}' no tiene un método '{nombre}'", datos.tipo),
                        Some(ubicacion),
                    )
                })?;
                let mut completos = Vec::with_capacity(argumentos.len() + 1);
                completos.push(receptor.clone());
                completos.extend(argumentos);
                self.llamar_nativa_id(id, &completos, ubicacion)
                    .map(DestinoLlamada::Valor)
            }
            _ => {
                let tipo = receptor.nombre_tipo();
                let id = self.nativos.metodo_de(tipo, nombre).ok_or_else(|| {
                    self.excepcion(
                        "E0406",
                        format!("el tipo '{tipo}' no tiene un método '{nombre}'"),
                        Some(ubicacion),
                    )
                })?;
                let mut completos = Vec::with_capacity(argumentos.len() + 1);
                completos.push(receptor);
                completos.extend(argumentos);
                self.llamar_nativa_id(id, &completos, ubicacion)
                    .map(DestinoLlamada::Valor)
            }
        }
    }

    fn llamar_metodo_padre(
        &mut self,
        datos: &crate::valores::DatosPadre,
        nombre: &str,
        argumentos: Vec<Valor>,
        ubicacion: Ubicacion,
    ) -> Result<DestinoLlamada, Fallo> {
        let instancia = &datos.instancia;
        let clase_actual = Rc::clone(&instancia.borrow().clase);
        match &datos.clase {
            Some(nombre_padre) => {
                let Some(clase_padre) = buscar_clase_ancestro(self, &clase_actual, nombre_padre)
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
                if let Some(clase_padre) = buscar_clase_ancestro(self, &clase_actual, nombre) {
                    return self.llamar_constructor(
                        &clase_padre,
                        argumentos,
                        Valor::Instancia(Rc::clone(instancia)),
                        ubicacion,
                    );
                }
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
                entorno: EnlaceEntorno::debil(&entorno),
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

    fn instanciar(
        &mut self,
        clase: &Rc<ClaseObjeto>,
        argumentos: Vec<Valor>,
        ubicacion: Ubicacion,
    ) -> Result<DestinoLlamada, Fallo> {
        let mut atributos = Vec::new();
        self.recolectar_atributos(clase, &mut atributos)?;
        let instancia = Rc::new(RefCell::new(Instancia {
            clase: Rc::clone(clase),
            atributos,
            inicializando: false,
        }));
        if let Some(clase_constructora) =
            buscar_clase_con_constructor(self, clase, argumentos.len())
        {
            let constructor = clase_constructora
                .constructores
                .iter()
                .find(|constructor| constructor.parametros.len() == argumentos.len())
                .cloned();
            if let Some(constructor) = constructor {
                instancia.borrow_mut().inicializando = true;
                let entorno = clase_constructora.entorno.resolver().ok_or_else(|| {
                    self.excepcion(
                        "E0201",
                        "el entorno del constructor ya fue liberado".to_string(),
                        Some(ubicacion),
                    )
                })?;
                let mut marco = self.preparar_marco_funcion(
                    &constructor,
                    &entorno,
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
        if let Valor::Instancia(instancia) = &esto {
            instancia.borrow_mut().inicializando = true;
        }
        let entorno = clase.entorno.resolver().ok_or_else(|| {
            self.excepcion(
                "E0201",
                "el entorno del constructor ya fue liberado".to_string(),
                Some(ubicacion),
            )
        })?;
        let mut marco = self.preparar_marco_funcion(
            &constructor,
            &entorno,
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
        destino: &mut Vec<Variable>,
    ) -> Result<(), Fallo> {
        let entorno = clase.entorno.resolver().ok_or_else(|| {
            self.excepcion(
                "E0201",
                "el entorno de los atributos ya fue liberado".to_string(),
                None,
            )
        })?;
        let n = clase.slots.borrow().len();
        if destino.len() < n {
            destino.resize(
                n,
                Variable {
                    valor: Valor::Nulo,
                    mutable: true,
                },
            );
        }
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
            let Some(&slot) = clase.slots.borrow().get(atributo.nombre.as_str()) else {
                continue;
            };
            let valor = match &atributo.inicial {
                Some(trozo) => self.ejecutar_trozo_aislado(trozo, &entorno, None)?,
                None => Valor::Nulo,
            };
            destino[slot as usize] = Variable {
                valor,
                mutable: atributo.mutable,
            };
        }
        Ok(())
    }

    fn cargar_atributo(
        &self,
        objeto: Valor,
        slot: u16,
        ubicacion: Ubicacion,
    ) -> Result<Valor, Fallo> {
        match objeto {
            Valor::Instancia(instancia) => instancia
                .borrow()
                .atributos
                .get(slot as usize)
                .map(|variable| variable.valor.clone())
                .ok_or_else(|| {
                    self.excepcion(
                        "E0201",
                        "la instancia no tiene ese atributo".to_string(),
                        Some(ubicacion),
                    )
                }),
            otro => Err(self.excepcion(
                "E0406",
                format!(
                    "un valor de tipo '{}' no tiene atributos por slot",
                    otro.nombre_tipo()
                ),
                Some(ubicacion),
            )),
        }
    }

    fn guardar_atributo(
        &self,
        objeto: Valor,
        slot: u16,
        valor: Valor,
        ubicacion: Ubicacion,
    ) -> Result<(), Fallo> {
        match objeto {
            Valor::Instancia(instancia) => {
                let mut instancia = instancia.borrow_mut();
                let inicializando = instancia.inicializando;
                let variable = instancia.atributos.get_mut(slot as usize).ok_or_else(|| {
                    self.excepcion(
                        "E0201",
                        "la instancia no tiene ese atributo".to_string(),
                        Some(ubicacion),
                    )
                })?;
                if !variable.mutable && !inicializando {
                    return Err(self.excepcion(
                        "E0203",
                        "no puedes modificar una propiedad constante".to_string(),
                        Some(ubicacion),
                    ));
                }
                variable.valor = valor;
                Ok(())
            }
            otro => Err(self.excepcion(
                "E0406",
                format!(
                    "no se puede asignar un atributo a un valor de tipo '{}'",
                    otro.nombre_tipo()
                ),
                Some(ubicacion),
            )),
        }
    }

    fn cargar_miembro(
        &mut self,
        objeto: Valor,
        nombre: &str,
        ubicacion: Ubicacion,
    ) -> Result<Valor, Fallo> {
        match &objeto {
            Valor::Jsn(mapa) => Ok(mapa.borrow().get(nombre).cloned().unwrap_or(Valor::Nulo)),
            Valor::Instancia(instancia) => {
                let clase = Rc::clone(&instancia.borrow().clase);
                if let Some(&slot) = clase.slots.borrow().get(nombre)
                    && let Some(variable) = instancia.borrow().atributos.get(slot as usize)
                {
                    return Ok(variable.valor.clone());
                }
                if let Some(valor) = buscar_atributo_libre(self, &clase, nombre) {
                    return Ok(valor);
                }
                if let Some((funcion, entorno)) = buscar_metodo_libre_en_clase(self, &clase, nombre)
                {
                    return Ok(Valor::funcion_local(funcion, &entorno));
                }
                if let Some((funcion, entorno)) = buscar_metodo_en_clase(self, &clase, nombre) {
                    return Ok(Valor::MetodoEnlazado(Rc::new(DatosMetodoEnlazado {
                        funcion,
                        entorno: EnlaceEntorno::debil(&entorno),
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
                    return Ok(Valor::funcion(funcion, entorno));
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
                if let Some(constante) = self.nativos.constante_miembro(modulo, nombre) {
                    return Ok(constante);
                }
                if let Some(id) = self.nativos.metodo_de(modulo, nombre) {
                    return Ok(Valor::Nativa(id));
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
            Valor::Padre(datos) => {
                if datos.clase.is_none() {
                    let clase_actual = Rc::clone(&datos.instancia.borrow().clase);
                    if buscar_clase_ancestro(self, &clase_actual, nombre).is_some() {
                        return Ok(Valor::padre(
                            Rc::clone(&datos.instancia),
                            Some(Rc::from(nombre)),
                        ));
                    }
                }
                let clase = Rc::clone(&datos.instancia.borrow().clase);
                if let Some(&slot) = clase.slots.borrow().get(nombre)
                    && let Some(variable) = datos.instancia.borrow().atributos.get(slot as usize)
                {
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
                let clase = Rc::clone(&instancia.borrow().clase);
                if let Some(&slot) = clase.slots.borrow().get(nombre) {
                    let mut instancia = instancia.borrow_mut();
                    let inicializando = instancia.inicializando;
                    if let Some(variable) = instancia.atributos.get_mut(slot as usize) {
                        if !variable.mutable && !inicializando {
                            return Err(self.excepcion(
                                "E0203",
                                format!("no puedes modificar la propiedad constante '{nombre}'"),
                                Some(ubicacion),
                            ));
                        }
                        variable.valor = valor;
                        return Ok(());
                    }
                }
                if guardar_atributo_libre(self, &clase, nombre, valor.clone(), ubicacion)? {
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
                if guardar_atributo_libre(self, clase, nombre, valor, ubicacion)? {
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
                if *posicion >= 0
                    && let Ok(indice) = usize::try_from(*posicion)
                    && let Some(caracter) = texto.chars().nth(indice)
                {
                    return Ok(Valor::texto(caracter.to_string()));
                }
                let total = texto.chars().count();
                let indice_real = indice_normalizado(*posicion, total).ok_or_else(|| {
                    self.excepcion(
                        "E0403",
                        format!(
                            "el índice {posicion} está fuera de rango (el texto tiene {total} caracteres)"
                        ),
                        Some(ubicacion),
                    )
                })?;
                let caracter = texto.chars().nth(indice_real).ok_or_else(|| {
                    self.excepcion(
                        "E0403",
                        format!("el índice {posicion} está fuera de rango"),
                        Some(ubicacion),
                    )
                })?;
                Ok(Valor::texto(caracter.to_string()))
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

fn limpiar_locales(pila: &mut [Valor], base: u32, desde: u16, ranuras: u16) {
    let inicio = (base as usize).saturating_add(desde as usize);
    let fin = (base as usize)
        .saturating_add(ranuras as usize)
        .min(pila.len());
    if inicio >= fin {
        return;
    }
    for ranura in &mut pila[inicio..fin] {
        *ranura = Valor::Nulo;
    }
}

fn valor_de_constante(constante: &Constante) -> Valor {
    match constante {
        Constante::Entero(entero) => Valor::Entero(*entero),
        Constante::Numero(decimal) => Valor::numero(*decimal),
        Constante::Texto(texto) => Valor::texto(texto),
        Constante::Log(valor_logico) => Valor::Log(*valor_logico),
        Constante::Nulo => Valor::Nulo,
    }
}

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

fn buscar_clase_por_nombre(
    vm: &Vm,
    desde: &Rc<ClaseObjeto>,
    nombre: &str,
) -> Option<Rc<ClaseObjeto>> {
    let entorno_origen = desde.entorno.resolver()?;
    if let Some(clase) = clase_en_entorno(&entorno_origen, nombre) {
        return Some(clase);
    }
    for entorno in vm.modulos.values() {
        if let Some(clase) = clase_en_entorno(entorno, nombre) {
            return Some(clase);
        }
    }
    None
}

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

fn aplanar_metodos(vm: &Vm, clase: &Rc<ClaseObjeto>, visitados: &mut HashSet<usize>) {
    let id = Rc::as_ptr(clase) as usize;
    if !visitados.insert(id) {
        return;
    }
    for nombre_padre in clase
        .compilado
        .padres
        .iter()
        .chain(clase.compilado.extiende_como.iter())
    {
        let Some(padre) = buscar_clase_por_nombre(vm, clase, nombre_padre) else {
            continue;
        };
        aplanar_metodos(vm, &padre, visitados);
        let heredados: Vec<(String, MetodoRuntime)> = padre
            .metodos
            .borrow()
            .iter()
            .map(|(nombre, metodo)| (nombre.clone(), metodo.clone()))
            .collect();
        let mut propios = clase.metodos.borrow_mut();
        for (nombre, metodo) in heredados {
            propios.entry(nombre).or_insert(metodo);
        }
    }
}

fn aplanar_slots(vm: &Vm, clase: &Rc<ClaseObjeto>, visitados: &mut HashSet<usize>) {
    let id = Rc::as_ptr(clase) as usize;
    if !visitados.insert(id) {
        return;
    }
    let mut nombres = Vec::new();
    for nombre_padre in clase
        .compilado
        .padres
        .iter()
        .chain(clase.compilado.extiende_como.iter())
    {
        let Some(padre) = buscar_clase_por_nombre(vm, clase, nombre_padre) else {
            continue;
        };
        aplanar_slots(vm, &padre, visitados);
        for (nombre, _) in padre.slots.borrow().iter() {
            if !nombres
                .iter()
                .any(|existente: &Rc<str>| existente == nombre)
            {
                nombres.push(Rc::clone(nombre));
            }
        }
    }
    for atributo in &clase.compilado.atributos {
        if atributo.libre {
            continue;
        }
        let nombre: Rc<str> = Rc::from(atributo.nombre.as_str());
        if !nombres.iter().any(|existente| existente == &nombre) {
            nombres.push(nombre);
        }
    }
    let mut slots = IndexMap::new();
    for (indice, nombre) in nombres.into_iter().enumerate() {
        slots.insert(nombre, indice as u16);
    }
    *clase.slots.borrow_mut() = slots;
}

fn buscar_metodo_en_clase(
    vm: &Vm,
    clase: &Rc<ClaseObjeto>,
    nombre: &str,
) -> Option<(Rc<FuncionCompilada>, Rc<EntornoModulo>)> {
    {
        let metodos = clase.metodos.borrow();
        if let Some(metodo) = metodos.get(nombre) {
            return Some((Rc::clone(&metodo.funcion), clase.entorno.resolver()?));
        }
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

fn buscar_metodo_libre_en_clase(
    vm: &Vm,
    clase: &Rc<ClaseObjeto>,
    nombre: &str,
) -> Option<(Rc<FuncionCompilada>, Rc<EntornoModulo>)> {
    {
        let metodos = clase.metodos.borrow();
        if let Some(metodo) = metodos.get(nombre)
            && metodo.libre
        {
            return Some((Rc::clone(&metodo.funcion), clase.entorno.resolver()?));
        }
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
        .borrow()
        .get(nombre)
        .map(|metodo| metodo.libre)
        .unwrap_or(false)
}

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

fn guardar_atributo_libre(
    vm: &Vm,
    clase: &Rc<ClaseObjeto>,
    nombre: &str,
    valor: Valor,
    ubicacion: Ubicacion,
) -> Result<bool, Fallo> {
    if let Some(variable) = clase.atributos_libres.borrow_mut().get_mut(nombre) {
        if !variable.mutable {
            return Err(vm.excepcion(
                "E0203",
                format!("no puedes modificar la propiedad constante '{nombre}'"),
                Some(ubicacion),
            ));
        }
        variable.valor = valor;
        return Ok(true);
    }
    for nombre_padre in clase
        .compilado
        .padres
        .iter()
        .chain(clase.compilado.extiende_como.iter())
    {
        if let Some(clase_padre) = buscar_clase_por_nombre(vm, clase, nombre_padre)
            && guardar_atributo_libre(vm, &clase_padre, nombre, valor.clone(), ubicacion)?
        {
            return Ok(true);
        }
    }
    Ok(false)
}
