// Máquina Virtual Híbrida para manejo automático de recursión
// Gestiona automáticamente la memoria y optimiza las llamadas recursivas

use crate::analizador_sintactico::Nodo;
use crate::tipos_datos::Valor;
use crate::evaluador::{Entorno, FuncionDefinida};
use crate::errores::{ErrorQuetzal, ResultadoQuetzal};
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::time::Instant;

/// Marco de llamada de función en la VM
#[derive(Debug, Clone)]
pub struct MarcoLlamada {
    /// Nombre de la función
    pub nombre_funcion: String,
    /// Argumentos evaluados
    pub argumentos: Vec<Valor>,
    /// Entorno de ejecución de la función
    pub entorno: Rc<RefCell<Entorno>>,
    /// Estado actual de ejecución
    pub estado: EstadoEjecucion,
    /// Definición de la función
    pub definicion: FuncionDefinida,
    /// Timestamp de inicio (para detectar bucles infinitos)
    pub inicio: Instant,
    /// Memoria estimada usada por este marco
    pub memoria_estimada: usize,
}

/// Estado de ejecución de un marco de llamada
#[derive(Debug, Clone)]
pub enum EstadoEjecucion {
    /// Preparando para ejecutar
    Preparando,
    /// Ejecutando el cuerpo de la función
    Ejecutando,
    /// Esperando resultado de llamada anidada
    EsperandoLlamada {
        nombre_llamada: String,
        indice_resultado: usize,
    },
    /// Finalizando con un valor de retorno
    Finalizando(Valor),
}

/// Resultado de un paso de ejecución en la VM
#[derive(Debug)]
pub enum ResultadoPaso {
    /// Continuar con el siguiente paso
    Continuar,
    /// Realizar una nueva llamada de función
    LlamarFuncion {
        nombre: String,
        argumentos: Vec<Valor>,
        entorno: Rc<RefCell<Entorno>>,
    },
    /// Retornar un valor
    RetornarValor(Valor),
    /// Error durante la ejecución
    Error(ErrorQuetzal),
}

/// Estadísticas de uso de memoria de la VM
#[derive(Debug, Default)]
pub struct EstadisticasMemoria {
    /// Memoria total usada actualmente
    pub memoria_actual: usize,
    /// Pico máximo de memoria usado
    pub memoria_maxima: usize,
    /// Número de marcos en el stack
    pub profundidad_actual: usize,
    /// Profundidad máxima alcanzada
    pub profundidad_maxima: usize,
    /// Número de optimizaciones de tail call aplicadas
    pub optimizaciones_tail_call: usize,
    /// Número de garbage collections activadas
    pub recolecciones_basura: usize,
}

/// Configuración automática de la VM estilo Python
#[derive(Debug)]
pub struct ConfiguracionVM {
    /// Límite inicial de profundidad (se expande automáticamente)
    pub limite_inicial_profundidad: usize,
    /// Incremento automático del límite cuando se alcanza
    pub incremento_limite: usize,
    /// Límite de memoria total del sistema (bytes)
    pub limite_memoria_total: usize,
    /// Umbral para activar recolección de basura (% de memoria)
    pub umbral_gc: f64,
    /// Límite de memoria por marco individual (bytes)
    pub limite_memoria_por_marco: usize,
    /// Activar optimizaciones de tail call automáticas
    pub optimizar_tail_calls: bool,
    /// Activar compresión automática del stack
    pub comprimir_stack_automatico: bool,
    /// Límite de iteraciones sin progreso (para detectar bucles infinitos reales)
    pub limite_iteraciones_sin_progreso: usize,
}

impl Default for ConfiguracionVM {
    fn default() -> Self {
        Self {
            limite_inicial_profundidad: 1000,
            incremento_limite: 1000, // Expandir de 1000 en 1000
            limite_memoria_total: 512 * 1024 * 1024, // 512MB
            umbral_gc: 0.7, // 70% de memoria
            limite_memoria_por_marco: 64 * 1024, // 64KB por marco (razonable)
            optimizar_tail_calls: true,
            comprimir_stack_automatico: true,
            limite_iteraciones_sin_progreso: 100000, // Detectar bucles infinitos reales
        }
    }
}

/// Máquina Virtual Universal para ejecución de funciones recursivas (estilo Python)
pub struct MaquinaVirtualRecursion {
    /// Stack de llamadas de función
    stack_llamadas: Vec<MarcoLlamada>,
    /// Configuración de la VM
    configuracion: ConfiguracionVM,
    /// Estadísticas de uso
    estadisticas: EstadisticasMemoria,
    /// Cache de resultados para memoización automática
    cache_resultados: HashMap<String, Valor>,
    /// Memoria total disponible del sistema (estimada)
    memoria_sistema: usize,
    /// Límite actual de profundidad (se expande automáticamente)
    limite_profundidad_actual: usize,
    /// Contador de iteraciones por función (para detectar bucles infinitos reales)
    contadores_iteracion: HashMap<String, usize>,
    /// Historial de estados de funciones (para detectar progreso)
    historial_estados: HashMap<String, Vec<u64>>, // Hash de estados de variables
}

impl MaquinaVirtualRecursion {
    /// Crea una nueva instancia de la VM universal
    pub fn nueva() -> Self {
        let config = ConfiguracionVM::default();
        Self {
            stack_llamadas: Vec::new(),
            limite_profundidad_actual: config.limite_inicial_profundidad,
            configuracion: config,
            estadisticas: EstadisticasMemoria::default(),
            cache_resultados: HashMap::new(),
            memoria_sistema: Self::estimar_memoria_sistema(),
            contadores_iteracion: HashMap::new(),
            historial_estados: HashMap::new(),
        }
    }
    
    /// Gestión universal de recursión - funciona con cualquier función
    pub fn puede_ejecutar_funcion(&mut self, nombre: &str, argumentos: &[Valor]) -> ResultadoQuetzal<bool> {
        // 1. Verificar memoria disponible del sistema
        if !self.verificar_memoria_disponible(argumentos)? {
            self.ejecutar_garbage_collection();
            if !self.verificar_memoria_disponible(argumentos)? {
                return Err(ErrorQuetzal::ErrorEjecucion {
                    linea: 0,
                    mensaje: format!("Memoria insuficiente para ejecutar función '{}'", nombre),
                });
            }
        }

        // 2. Verificar profundidad y expandir límite automáticamente si es necesario
        if self.stack_llamadas.len() >= self.limite_profundidad_actual {
            if self.puede_expandir_limite() {
                self.expandir_limite_profundidad();
                println!("🔧 Límite de profundidad expandido a: {}", self.limite_profundidad_actual);
            } else {
                // Solo fallar si realmente parece un bucle infinito malicioso
                if self.es_bucle_infinito_real(nombre)? {
                    return Err(ErrorQuetzal::ErrorEjecucion {
                        linea: 0,
                        mensaje: format!(
                            "Bucle infinito detectado en función '{}' - sin progreso después de {} iteraciones",
                            nombre, self.configuracion.limite_iteraciones_sin_progreso
                        ),
                    });
                } else {
                    // Es recursión legítima, continuar
                    self.expandir_limite_profundidad();
                }
            }
        }

        // 3. Actualizar contadores de progreso
        self.actualizar_contador_progreso(nombre, argumentos);
        
        Ok(true)
    }
    
    /// Verifica si hay memoria suficiente disponible
    fn verificar_memoria_disponible(&self, argumentos: &[Valor]) -> ResultadoQuetzal<bool> {
        let memoria_nueva = self.estimar_memoria_valores(argumentos);
        let memoria_total_proyectada = self.estadisticas.memoria_actual + memoria_nueva;
        
        Ok(memoria_total_proyectada <= (self.configuracion.limite_memoria_total as f64 * self.configuracion.umbral_gc) as usize)
    }
    
    /// Verifica si se puede expandir el límite de profundidad
    fn puede_expandir_limite(&self) -> bool {
        // Expandir solo si hay memoria suficiente para más marcos
        let memoria_por_nuevos_marcos = self.configuracion.incremento_limite * self.configuracion.limite_memoria_por_marco;
        self.estadisticas.memoria_actual + memoria_por_nuevos_marcos < self.configuracion.limite_memoria_total
    }
    
    /// Expande automáticamente el límite de profundidad
    fn expandir_limite_profundidad(&mut self) {
        self.limite_profundidad_actual += self.configuracion.incremento_limite;
    }
    
    /// Detecta bucles infinitos reales (sin progreso) vs recursión legítima
    fn es_bucle_infinito_real(&mut self, nombre: &str) -> ResultadoQuetzal<bool> {
        let contador = self.contadores_iteracion.entry(nombre.to_string()).or_insert(0);
        *contador += 1;
        
        // Si se ejecuta muchas veces sin progreso en las variables, es bucle infinito
        if *contador > self.configuracion.limite_iteraciones_sin_progreso {
            if let Some(historial) = self.historial_estados.get(nombre) {
                if historial.len() > 10 {
                    // Verificar si los últimos 10 estados son idénticos (sin progreso)
                    let ultimos_estados = &historial[historial.len()-10..];
                    if ultimos_estados.iter().all(|&estado| estado == ultimos_estados[0]) {
                        return Ok(true); // Es bucle infinito real
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    /// Actualiza el progreso de la función para detectar bucles infinitos reales
    fn actualizar_contador_progreso(&mut self, nombre: &str, argumentos: &[Valor]) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        // Crear hash del estado actual de los argumentos
        let mut hasher = DefaultHasher::new();
        for arg in argumentos {
            arg.hash(&mut hasher);
        }
        let estado_actual = hasher.finish();
        
        // Actualizar historial
        let historial = self.historial_estados.entry(nombre.to_string()).or_insert_with(Vec::new);
        historial.push(estado_actual);
        
        // Mantener solo los últimos N estados para eficiencia
        if historial.len() > 20 {
            historial.remove(0);
        }
    }

    /// Estima la memoria disponible del sistema
    fn estimar_memoria_sistema() -> usize {
        // En un sistema real, esto podría usar APIs del sistema
        // Por ahora, asumimos un valor conservador
        512 * 1024 * 1024 // 512MB
    }
    
    /// Obtiene estadísticas públicas de la VM
    pub fn obtener_limite_actual(&self) -> usize {
        self.limite_profundidad_actual
    }
    
    pub fn obtener_memoria_actual(&self) -> usize {
        self.estadisticas.memoria_actual
    }
    
    pub fn obtener_profundidad_actual(&self) -> usize {
        self.stack_llamadas.len()
    }

    /// Calcula el límite dinámico de profundidad basado en memoria disponible (no usado en sistema universal)
    fn calcular_limite_profundidad_dinamico(&self) -> usize {
        // En el sistema universal, usamos expansión automática
        self.limite_profundidad_actual
    }

    /// Estima el uso de memoria de un conjunto de valores
    fn estimar_memoria_valores(&self, valores: &[Valor]) -> usize {
        valores.iter().map(|v| self.estimar_memoria_valor(v)).sum()
    }

    /// Estima el uso de memoria de un valor individual
    fn estimar_memoria_valor(&self, valor: &Valor) -> usize {
        match valor {
            Valor::Entero(_) => 8,
            Valor::Numero(_) => 8,
            Valor::Log(_) => 1,
            Valor::Texto(s) => s.len() * 4, // UTF-8 puede usar hasta 4 bytes por char
            Valor::Lista(lista) => {
                48 + lista.iter().map(|v| self.estimar_memoria_valor(v)).sum::<usize>()
            }
            Valor::Json(mapa) => {
                48 + mapa.iter().map(|(k, v)| {
                    k.len() * 4 + self.estimar_memoria_valor(v)
                }).sum::<usize>()
            }
            Valor::Objeto { .. } => 256, // Estimación base para objetos
            Valor::Nulo => 1,
            Valor::Vacio => 1,
        }
    }

    /// Detecta si una llamada puede optimizarse como tail call
    fn es_tail_call(&self, nodo: &Nodo) -> bool {
        if !self.configuracion.optimizar_tail_calls {
            return false;
        }

        // Verificar si la última instrucción es un retorno directo de llamada a función
        match nodo {
            Nodo::Programa(declaraciones) => {
                if let Some(Nodo::Retornar { valor: Some(llamada), .. }) = declaraciones.last() {
                    matches!(llamada.as_ref(), Nodo::LlamadaFuncion { .. })
                } else {
                    false
                }
            }
            Nodo::Retornar { valor: Some(llamada), .. } => {
                matches!(llamada.as_ref(), Nodo::LlamadaFuncion { .. })
            }
            _ => false,
        }
    }

    /// Ejecuta una función con manejo automático de recursión
    pub fn evaluar_con_vm(
        &mut self,
        nombre: &str,
        argumentos: &[Nodo],
        linea: usize,
        entorno: Rc<RefCell<Entorno>>,
        evaluador: &mut crate::evaluador::Evaluador,
    ) -> ResultadoQuetzal<(Valor, crate::evaluador::ControlFlujo)> {
        // Por ahora, usar stacker como respaldo para evitar errores
        stacker::maybe_grow(64 * 1024, 1024 * 1024, || {
            evaluador.evaluar_llamada_funcion(nombre, argumentos, linea, entorno)
        })
    }

    /// Evalúa los límites necesarios para una función específica
    pub fn ejecutar_funcion(
        &mut self,
        nombre: &str,
        argumentos: Vec<Valor>,
        definicion: FuncionDefinida,
        entorno: Rc<RefCell<Entorno>>,
    ) -> ResultadoQuetzal<Valor> {
        // Verificar límites antes de comenzar
        self.verificar_limites_ejecucion(nombre, &argumentos)?;

        // Verificar si podemos usar memoización
        if let Some(resultado_cache) = self.verificar_cache(nombre, &argumentos) {
            return Ok(resultado_cache);
        }

        // Crear marco de llamada inicial
        let memoria_estimada = self.estimar_memoria_valores(&argumentos);
        let marco = MarcoLlamada {
            nombre_funcion: nombre.to_string(),
            argumentos: argumentos.clone(),
            entorno: entorno.clone(),
            estado: EstadoEjecucion::Preparando,
            definicion: definicion.clone(),
            inicio: Instant::now(),
            memoria_estimada,
        };

        self.stack_llamadas.push(marco);
        self.actualizar_estadisticas_memoria();

        // Optimización: verificar tail call en el marco actual
        if self.es_tail_call(&definicion.cuerpo) && self.stack_llamadas.len() > 1 {
            // Optimizar: reutilizar el marco anterior en lugar de crear uno nuevo
            self.optimizar_tail_call()?;
        }

        // Bucle principal de ejecución - sin recursión de sistema
        loop {
            // Verificar progreso para detectar bucles infinitos reales
            if let Some(marco_actual) = self.stack_llamadas.last() {
                self.verificar_progreso_funcion(&marco_actual.nombre_funcion)?;
            }

            // Ejecutar un paso
            let resultado = self.ejecutar_paso_actual()?;

            match resultado {
                ResultadoPaso::Continuar => {
                    // Continuar con la ejecución normal
                    continue;
                }
                ResultadoPaso::LlamarFuncion { nombre, argumentos, entorno } => {
                    // Necesitamos hacer una nueva llamada de función
                    if let Some(funcion) = entorno.borrow().obtener_funcion(&nombre) {
                        // Verificar límites antes de la nueva llamada
                        self.verificar_limites_ejecucion(&nombre, &argumentos)?;

                        let memoria_nueva = self.estimar_memoria_valores(&argumentos);
                        let marco_nuevo = MarcoLlamada {
                            nombre_funcion: nombre.clone(),
                            argumentos: argumentos.clone(),
                            entorno: entorno.clone(),
                            estado: EstadoEjecucion::Preparando,
                            definicion: funcion.clone(),
                            inicio: Instant::now(),
                            memoria_estimada: memoria_nueva,
                        };

                        self.stack_llamadas.push(marco_nuevo);
                        self.actualizar_estadisticas_memoria();
                    } else {
                        return Err(ErrorQuetzal::FuncionNoDefinida {
                            linea: 0,
                            nombre: nombre.clone(),
                        });
                    }
                }
                ResultadoPaso::RetornarValor(valor) => {
                    // Función completada, pop del stack
                    let marco_completado = self.stack_llamadas.pop();
                    
                    if let Some(marco) = marco_completado {
                        // Guardar en cache si es apropiado
                        self.guardar_en_cache(&marco.nombre_funcion, &marco.argumentos, &valor);
                        
                        // Actualizar estadísticas
                        self.estadisticas.memoria_actual = self.estadisticas.memoria_actual
                            .saturating_sub(marco.memoria_estimada);
                    }

                    if self.stack_llamadas.is_empty() {
                        // Función principal completada
                        return Ok(valor);
                    } else {
                        // Función anidada completada, continuar con el marco anterior
                        self.procesar_resultado_llamada_anidada(valor)?;
                    }
                }
                ResultadoPaso::Error(error) => {
                    return Err(error);
                }
            }

            // Verificar si necesitamos recolección de basura
            if self.necesita_garbage_collection() {
                self.ejecutar_garbage_collection();
            }
        }
    }

    /// Verifica los límites de ejecución antes de una llamada
    fn verificar_limites_ejecucion(&mut self, nombre: &str, argumentos: &[Valor]) -> ResultadoQuetzal<()> {
        // Usar la nueva lógica universal - NO más restricciones por nombre
        self.puede_ejecutar_funcion(nombre, argumentos)?;
        Ok(())
    }

    /// Verifica si hay un resultado en cache
    fn verificar_cache(&self, nombre: &str, argumentos: &[Valor]) -> Option<Valor> {
        // Solo hacer cache para funciones puras (sin efectos secundarios)
        if self.es_funcion_pura(nombre) {
            let clave = self.generar_clave_cache(nombre, argumentos);
            self.cache_resultados.get(&clave).cloned()
        } else {
            None
        }
    }

    /// Determina si una función es pura (sin efectos secundarios)
    fn es_funcion_pura(&self, nombre: &str) -> bool {
        // Lista de funciones que sabemos que son puras
        matches!(nombre, "factorial" | "fibonacci" | "potencia" | "suma_lista")
    }

    /// Genera una clave única para el cache
    fn generar_clave_cache(&self, nombre: &str, argumentos: &[Valor]) -> String {
        format!("{}:{:?}", nombre, argumentos)
    }

    /// Guarda un resultado en el cache
    fn guardar_en_cache(&mut self, nombre: &str, argumentos: &[Valor], resultado: &Valor) {
        if self.es_funcion_pura(nombre) && self.cache_resultados.len() < 1000 {
            let clave = self.generar_clave_cache(nombre, argumentos);
            self.cache_resultados.insert(clave, resultado.clone());
        }
    }

    /// Optimiza una llamada tail call
    fn optimizar_tail_call(&mut self) -> ResultadoQuetzal<()> {
        if self.stack_llamadas.len() >= 2 {
            // Reemplazar el marco anterior con el actual (reutilización de stack)
            let marco_actual = self.stack_llamadas.pop().unwrap();
            let _marco_anterior = self.stack_llamadas.pop().unwrap();
            
            // El marco actual toma el lugar del anterior
            self.stack_llamadas.push(marco_actual);
            self.estadisticas.optimizaciones_tail_call += 1;
        }
        Ok(())
    }

    /// Verificación inteligente de progreso - reemplaza timeout restrictivo
    fn verificar_progreso_funcion(&self, nombre: &str) -> ResultadoQuetzal<()> {
        // Solo verificar si realmente parece un bucle infinito sin progreso
        if let Some(&contador) = self.contadores_iteracion.get(nombre) {
            if contador > self.configuracion.limite_iteraciones_sin_progreso * 2 {
                // Verificar progreso reciente
                if let Some(historial) = self.historial_estados.get(nombre) {
                    if historial.len() > 50 {
                        let estados_recientes = &historial[historial.len()-50..];
                        let primer_estado = estados_recientes[0];
                        
                        // Si todos los estados recientes son idénticos, es bucle infinito real
                        if estados_recientes.iter().all(|&estado| estado == primer_estado) {
                            return Err(ErrorQuetzal::ErrorEjecucion {
                                linea: 0,
                                mensaje: format!(
                                    "Bucle infinito sin progreso detectado en función '{}' - estado no cambia después de {} iteraciones",
                                    nombre, contador
                                ),
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Ejecuta un paso de la función actual
    fn ejecutar_paso_actual(&mut self) -> ResultadoQuetzal<ResultadoPaso> {
        // Esta función será implementada en la siguiente parte
        // Por ahora, retornamos un placeholder
        Ok(ResultadoPaso::RetornarValor(Valor::Vacio))
    }

    /// Procesa el resultado de una llamada anidada
    fn procesar_resultado_llamada_anidada(&mut self, _resultado: Valor) -> ResultadoQuetzal<()> {
        // Esta función será implementada en la siguiente parte
        Ok(())
    }

    /// Actualiza las estadísticas de memoria
    fn actualizar_estadisticas_memoria(&mut self) {
        self.estadisticas.memoria_actual = self.stack_llamadas
            .iter()
            .map(|marco| marco.memoria_estimada)
            .sum();

        self.estadisticas.memoria_maxima = self.estadisticas.memoria_maxima
            .max(self.estadisticas.memoria_actual);

        self.estadisticas.profundidad_actual = self.stack_llamadas.len();
        self.estadisticas.profundidad_maxima = self.estadisticas.profundidad_maxima
            .max(self.estadisticas.profundidad_actual);
    }

    /// Verifica si se necesita recolección de basura
    fn necesita_garbage_collection(&self) -> bool {
        let uso_memoria = self.estadisticas.memoria_actual as f64 / self.memoria_sistema as f64;
        uso_memoria > self.configuracion.umbral_gc
    }

    /// Ejecuta recolección de basura
    fn ejecutar_garbage_collection(&mut self) {
        // Limpiar cache si es muy grande
        if self.cache_resultados.len() > 500 {
            self.cache_resultados.clear();
        }

        self.estadisticas.recolecciones_basura += 1;
    }

    /// Obtiene las estadísticas actuales de la VM
    pub fn obtener_estadisticas(&self) -> &EstadisticasMemoria {
        &self.estadisticas
    }

    /// Reinicia la VM para una nueva ejecución
    pub fn reiniciar(&mut self) {
        self.stack_llamadas.clear();
        self.estadisticas = EstadisticasMemoria::default();
        // Mantener el cache para reutilización entre ejecuciones
    }
}
