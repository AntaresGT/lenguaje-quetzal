//! Analizador semántico: recorre el AST validando símbolos, tipos,
//! mutabilidad, visibilidad y prototipos. Acumula todos los errores.

use ast::{
    Bloque, ClaseMiembro, DefinicionObjeto, DefinicionPrototipo, Elemento, Expresion, FirmaMiembro,
    Funcion, Modulo, NodoExpresion, NodoSentencia, OperadorAsignacion, OperadorBinario,
    OperadorUnario, SegmentoInterpolado, Sentencia, Tipo, Visibilidad,
};
use indexmap::IndexMap;
use nucleo::{CategoriaError, ErrorQuetzal, Ubicacion};

use crate::resolucion_imports::clasificar_origen;
use crate::tabla_simbolos::{Simbolo, TablaSimbolos};
use crate::tipos::TipoSemantico;

/// Analiza un módulo completo. Devuelve todos los errores encontrados.
pub fn analizar_modulo(modulo: &Modulo) -> Result<(), Vec<ErrorQuetzal>> {
    let mut analizador = Analizador::nuevo(&modulo.nombre);
    analizador.analizar(modulo);
    if analizador.errores.is_empty() {
        Ok(())
    } else {
        Err(analizador.errores)
    }
}

/// Información registrada de un atributo de objeto.
#[derive(Debug, Clone)]
struct InfoAtributo {
    tipo: TipoSemantico,
    mutable: bool,
    publico: bool,
}

/// Información registrada de un método de objeto.
#[derive(Debug, Clone)]
struct InfoMetodo {
    parametros: Vec<TipoSemantico>,
    retorno: TipoSemantico,
    publico: bool,
}

/// Información registrada de un objeto.
#[derive(Debug, Clone, Default)]
struct InfoObjeto {
    padres: Vec<String>,
    /// Objetos extendidos con `como` (también aportan miembros).
    extiende_como: Vec<String>,
    atributos: IndexMap<String, InfoAtributo>,
    metodos: IndexMap<String, InfoMetodo>,
    /// Cantidades de parámetros de los constructores declarados.
    constructores: Vec<usize>,
}

/// Información registrada de un prototipo.
#[derive(Debug, Clone, Default)]
struct InfoPrototipo {
    /// Miembros obligatorios (no `opcional`): nombre → es método.
    obligatorios: IndexMap<String, bool>,
}

struct Analizador {
    archivo: String,
    errores: Vec<ErrorQuetzal>,
    tabla: TablaSimbolos,
    objetos: IndexMap<String, InfoObjeto>,
    prototipos: IndexMap<String, InfoPrototipo>,
    /// Tipo de retorno de la función que se analiza (pila por anidamiento).
    pila_retorno: Vec<TipoSemantico>,
    /// Si la función que se analiza es `asincrono` (pila por anidamiento;
    /// en el scope global la pila está vacía y `esperar` está permitido).
    pila_asincrona: Vec<bool>,
    /// Objeto cuyo cuerpo se está analizando.
    objeto_actual: Option<String>,
    /// Si se analiza el cuerpo de un constructor (permite inicializar
    /// atributos constantes).
    en_constructor: bool,
    profundidad_bucle: usize,
}

impl Analizador {
    fn nuevo(archivo: &str) -> Self {
        let mut tabla = TablaSimbolos::nueva();
        // Globales del lenguaje: la consola no necesita importarse y `rango`
        // crea listas de números consecutivos.
        for global in ["consola", "rango"] {
            tabla.declarar(Simbolo {
                nombre: global.to_string(),
                tipo: TipoSemantico::Desconocido,
                mutable: false,
                ubicacion: Ubicacion::default(),
            });
        }
        Self {
            archivo: archivo.to_string(),
            errores: Vec::new(),
            tabla,
            objetos: IndexMap::new(),
            prototipos: IndexMap::new(),
            pila_retorno: Vec::new(),
            pila_asincrona: Vec::new(),
            objeto_actual: None,
            en_constructor: false,
            profundidad_bucle: 0,
        }
    }

    fn error(&mut self, error: ErrorQuetzal) {
        self.errores.push(error.con_archivo(&self.archivo));
    }

    fn analizar(&mut self, modulo: &Modulo) {
        // Primera pasada: registrar funciones, objetos, prototipos e imports
        // para permitir referencias hacia adelante (recursión, instancias).
        for elemento in &modulo.elementos {
            match elemento {
                Elemento::Funcion(funcion) => self.registrar_funcion(funcion),
                Elemento::Objeto(objeto) => self.registrar_objeto(objeto),
                Elemento::Prototipo(prototipo) => self.registrar_prototipo(prototipo),
                Elemento::Importacion(importacion) => self.registrar_importacion(importacion),
                _ => {}
            }
        }

        // Verificación de prototipos implementados.
        let objetos: Vec<(String, DefinicionObjeto)> = modulo
            .elementos
            .iter()
            .filter_map(|elemento| match elemento {
                Elemento::Objeto(objeto) => Some((objeto.nombre.clone(), objeto.clone())),
                _ => None,
            })
            .collect();
        for (nombre, objeto) in &objetos {
            self.verificar_prototipos(nombre, objeto);
            self.verificar_padres(objeto);
        }

        // Segunda pasada: analizar cuerpos y sentencias en orden.
        let mut exportaciones: Vec<(Vec<String>, Ubicacion)> = Vec::new();
        for elemento in &modulo.elementos {
            match elemento {
                Elemento::Funcion(funcion) => self.analizar_funcion(funcion),
                Elemento::Objeto(objeto) => self.analizar_objeto(objeto),
                Elemento::Sentencia(sentencia) => self.analizar_sentencia(sentencia),
                Elemento::Exportacion {
                    simbolos,
                    ubicacion,
                } => exportaciones.push((simbolos.clone(), *ubicacion)),
                Elemento::Importacion(_) | Elemento::Prototipo(_) => {}
            }
        }

        // Las exportaciones se validan al final: pueden listar símbolos
        // declarados en cualquier punto del módulo.
        for (simbolos, ubicacion) in exportaciones {
            for simbolo in simbolos {
                if !self.tabla.existe(&simbolo) {
                    self.error(
                        ErrorQuetzal::nuevo(
                            "E0302",
                            CategoriaError::Modulos,
                            format!("se exporta '{simbolo}', pero no está definido en el módulo"),
                        )
                        .con_ubicacion(ubicacion)
                        .con_etiqueta("este símbolo no existe")
                        .con_ayuda("declara el símbolo antes de exportarlo o quítalo de la lista"),
                    );
                }
            }
        }
    }

    // ----- Registro (primera pasada) -----

    fn declarar_simbolo(
        &mut self,
        nombre: &str,
        tipo: TipoSemantico,
        mutable: bool,
        ubicacion: Ubicacion,
    ) {
        let previo = self.tabla.declarar(Simbolo {
            nombre: nombre.to_string(),
            tipo,
            mutable,
            ubicacion,
        });
        if previo.is_some() {
            self.error(
                ErrorQuetzal::nuevo(
                    "E0202",
                    CategoriaError::Semantico,
                    format!("'{nombre}' ya está declarado en este ámbito"),
                )
                .con_ubicacion(ubicacion)
                .con_etiqueta("nombre duplicado")
                .con_ayuda("usa un nombre distinto o elimina la declaración repetida"),
            );
        }
    }

    fn registrar_funcion(&mut self, funcion: &Funcion) {
        let parametros = funcion
            .parametros
            .iter()
            .map(|parametro| TipoSemantico::desde_ast(&parametro.tipo))
            .collect();
        let retorno = TipoSemantico::desde_ast(&funcion.tipo_retorno);
        self.declarar_simbolo(
            &funcion.nombre,
            TipoSemantico::Funcion {
                parametros,
                retorno: Box::new(retorno),
            },
            false,
            funcion.ubicacion,
        );
    }

    fn registrar_objeto(&mut self, objeto: &DefinicionObjeto) {
        let mut info = InfoObjeto {
            padres: objeto.padres.clone(),
            extiende_como: objeto.extiende_como.clone(),
            ..Default::default()
        };

        for miembro in &objeto.miembros {
            let publico = miembro.visibilidad == Visibilidad::Publico;
            match &miembro.clase {
                ClaseMiembro::Atributo {
                    tipo,
                    mutable,
                    nombre,
                    ..
                } => {
                    info.atributos.insert(
                        nombre.clone(),
                        InfoAtributo {
                            tipo: TipoSemantico::desde_ast(tipo),
                            mutable: *mutable,
                            publico,
                        },
                    );
                }
                ClaseMiembro::Constructor(funcion) => {
                    info.constructores.push(funcion.parametros.len());
                }
                ClaseMiembro::Metodo(funcion) => {
                    info.metodos.insert(
                        funcion.nombre.clone(),
                        InfoMetodo {
                            parametros: funcion
                                .parametros
                                .iter()
                                .map(|parametro| TipoSemantico::desde_ast(&parametro.tipo))
                                .collect(),
                            retorno: TipoSemantico::desde_ast(&funcion.tipo_retorno),
                            publico,
                        },
                    );
                }
            }
        }

        self.objetos.insert(objeto.nombre.clone(), info);
        // El nombre del objeto también es un símbolo (miembros libres:
        // `MatematicaUtil.absoluto(-10)`).
        self.declarar_simbolo(
            &objeto.nombre,
            TipoSemantico::Desconocido,
            false,
            objeto.ubicacion,
        );
    }

    fn registrar_prototipo(&mut self, prototipo: &DefinicionPrototipo) {
        let mut info = InfoPrototipo::default();
        for miembro in &prototipo.miembros {
            if !miembro.opcional {
                let es_metodo = matches!(miembro.firma, FirmaMiembro::Metodo { .. });
                info.obligatorios
                    .insert(miembro.firma.nombre().to_string(), es_metodo);
            }
        }
        self.prototipos.insert(prototipo.nombre.clone(), info);
        self.declarar_simbolo(
            &prototipo.nombre,
            TipoSemantico::Desconocido,
            false,
            prototipo.ubicacion,
        );
    }

    fn registrar_importacion(&mut self, importacion: &ast::Importacion) {
        if clasificar_origen(&importacion.origen).is_none() {
            self.error(
                ErrorQuetzal::nuevo(
                    "E0301",
                    CategoriaError::Modulos,
                    format!("no existe el módulo '{}'", importacion.origen),
                )
                .con_ubicacion(importacion.ubicacion)
                .con_etiqueta("módulo no encontrado")
                .con_ayuda(
                    "los módulos nativos disponibles son: quetzal/matematica, quetzal/texto, quetzal/listas, quetzal/jsn, quetzal/tiempo, quetzal/red y quetzal/sistema_archivos",
                ),
            );
        }
        // Los símbolos importados se consideran de tipo desconocido; la
        // verificación contra el módulo real la hace el motor al cargar el
        // grafo de imports.
        for simbolo in &importacion.simbolos {
            self.declarar_simbolo(
                simbolo.nombre_local(),
                TipoSemantico::Desconocido,
                false,
                importacion.ubicacion,
            );
        }
    }

    fn verificar_padres(&mut self, objeto: &DefinicionObjeto) {
        for padre in objeto.padres.iter().chain(objeto.extiende_como.iter()) {
            if !self.objetos.contains_key(padre) && !self.tabla.existe(padre) {
                self.error(
                    ErrorQuetzal::nuevo(
                        "E0201",
                        CategoriaError::Semantico,
                        format!("el objeto padre '{padre}' no está definido"),
                    )
                    .con_ubicacion(objeto.ubicacion)
                    .con_etiqueta("objeto no definido")
                    .con_ayuda("define el objeto padre o impórtalo antes de heredar de él"),
                );
            }
        }
    }

    fn verificar_prototipos(&mut self, nombre_objeto: &str, objeto: &DefinicionObjeto) {
        for nombre_prototipo in &objeto.prototipos {
            let Some(prototipo) = self.prototipos.get(nombre_prototipo).cloned() else {
                if !self.tabla.existe(nombre_prototipo) {
                    self.error(
                        ErrorQuetzal::nuevo(
                            "E0201",
                            CategoriaError::Semantico,
                            format!("el prototipo '{nombre_prototipo}' no está definido"),
                        )
                        .con_ubicacion(objeto.ubicacion)
                        .con_etiqueta("prototipo no definido"),
                    );
                }
                continue;
            };

            for (miembro, _es_metodo) in &prototipo.obligatorios {
                if self.buscar_atributo(nombre_objeto, miembro).is_none()
                    && self.buscar_metodo(nombre_objeto, miembro).is_none()
                {
                    self.error(
                        ErrorQuetzal::nuevo(
                            "E0208",
                            CategoriaError::Semantico,
                            format!(
                                "el objeto '{nombre_objeto}' implementa '{nombre_prototipo}' pero no define '{miembro}'"
                            ),
                        )
                        .con_ubicacion(objeto.ubicacion)
                        .con_etiqueta("prototipo incompleto")
                        .con_ayuda(format!(
                            "agrega el miembro '{miembro}' al objeto o márcalo como 'opcional' en el prototipo"
                        )),
                    );
                }
            }
        }
    }

    // ----- Búsqueda de miembros con herencia -----

    fn buscar_atributo(&self, objeto: &str, nombre: &str) -> Option<InfoAtributo> {
        let info = self.objetos.get(objeto)?;
        if let Some(atributo) = info.atributos.get(nombre) {
            return Some(atributo.clone());
        }
        for padre in info.padres.iter().chain(info.extiende_como.iter()) {
            if let Some(atributo) = self.buscar_atributo(padre, nombre) {
                return Some(atributo);
            }
        }
        None
    }

    fn buscar_metodo(&self, objeto: &str, nombre: &str) -> Option<InfoMetodo> {
        let info = self.objetos.get(objeto)?;
        if let Some(metodo) = info.metodos.get(nombre) {
            return Some(metodo.clone());
        }
        for padre in info.padres.iter().chain(info.extiende_como.iter()) {
            if let Some(metodo) = self.buscar_metodo(padre, nombre) {
                return Some(metodo);
            }
        }
        None
    }

    /// Si el análisis se ejecuta dentro del objeto dado o uno de sus descendientes.
    fn dentro_del_objeto(&self, objeto: &str) -> bool {
        let Some(actual) = &self.objeto_actual else {
            return false;
        };
        let mut pendientes = vec![actual.clone()];
        while let Some(nombre) = pendientes.pop() {
            if nombre == objeto {
                return true;
            }
            if let Some(info) = self.objetos.get(&nombre) {
                pendientes.extend(info.padres.iter().cloned());
                pendientes.extend(info.extiende_como.iter().cloned());
            }
        }
        false
    }

    // ----- Análisis de cuerpos -----

    fn analizar_funcion(&mut self, funcion: &Funcion) {
        self.tabla.abrir_ambito();
        for parametro in &funcion.parametros {
            self.declarar_simbolo(
                &parametro.nombre,
                TipoSemantico::desde_ast(&parametro.tipo),
                parametro.mutable,
                parametro.ubicacion,
            );
        }
        self.pila_retorno
            .push(TipoSemantico::desde_ast(&funcion.tipo_retorno));
        self.pila_asincrona.push(funcion.asincrona);
        self.analizar_bloque_sin_ambito(&funcion.cuerpo);
        self.pila_asincrona.pop();
        self.pila_retorno.pop();
        self.tabla.cerrar_ambito();
    }

    /// `true` si `esperar` es válido en este punto: a nivel de scope global
    /// (no hay función en la pila) o dentro de una función `asincrono`.
    fn esperar_es_valido(&self) -> bool {
        self.pila_asincrona.last().copied().unwrap_or(true)
    }

    fn analizar_objeto(&mut self, objeto: &DefinicionObjeto) {
        let objeto_anterior = self.objeto_actual.replace(objeto.nombre.clone());

        for miembro in &objeto.miembros {
            match &miembro.clase {
                ClaseMiembro::Atributo { valor_inicial, .. } => {
                    if let Some(valor) = valor_inicial {
                        self.inferir_expresion(valor);
                    }
                }
                ClaseMiembro::Constructor(funcion) => {
                    self.en_constructor = true;
                    self.analizar_funcion(funcion);
                    self.en_constructor = false;
                }
                ClaseMiembro::Metodo(funcion) => self.analizar_funcion(funcion),
            }
        }

        self.objeto_actual = objeto_anterior;
    }

    fn analizar_bloque(&mut self, bloque: &Bloque) {
        self.tabla.abrir_ambito();
        self.analizar_bloque_sin_ambito(bloque);
        self.tabla.cerrar_ambito();
    }

    fn analizar_bloque_sin_ambito(&mut self, bloque: &Bloque) {
        for sentencia in &bloque.sentencias {
            self.analizar_sentencia(sentencia);
        }
    }

    fn analizar_sentencia(&mut self, sentencia: &Sentencia) {
        match &sentencia.nodo {
            NodoSentencia::DeclaracionVariable {
                tipo,
                mutable,
                nombre,
                valor,
            } => self.analizar_declaracion(tipo, *mutable, nombre, valor, sentencia.ubicacion),
            NodoSentencia::Asignacion {
                objetivo,
                operador,
                valor,
            } => self.analizar_asignacion(objetivo, *operador, valor),
            NodoSentencia::IncrementoDecremento { objetivo, .. } => {
                self.analizar_incremento(objetivo);
            }
            NodoSentencia::Expresion(expresion) => {
                self.inferir_expresion(expresion);
            }
            NodoSentencia::Si {
                condicion,
                entonces,
                sino,
            } => {
                self.exigir_condicion_logica(condicion);
                self.analizar_bloque(entonces);
                if let Some(bloque) = sino {
                    self.analizar_bloque(bloque);
                }
            }
            NodoSentencia::Mientras { condicion, cuerpo } => {
                self.exigir_condicion_logica(condicion);
                self.analizar_bloque_de_bucle(cuerpo);
            }
            NodoSentencia::HacerMientras { cuerpo, condicion } => {
                self.analizar_bloque_de_bucle(cuerpo);
                self.exigir_condicion_logica(condicion);
            }
            NodoSentencia::ParaClasico {
                inicializacion,
                condicion,
                paso,
                cuerpo,
            } => {
                self.tabla.abrir_ambito();
                self.analizar_sentencia(inicializacion);
                self.exigir_condicion_logica(condicion);
                self.profundidad_bucle += 1;
                self.analizar_bloque_sin_ambito(cuerpo);
                self.analizar_sentencia(paso);
                self.profundidad_bucle -= 1;
                self.tabla.cerrar_ambito();
            }
            NodoSentencia::ParaEn {
                tipo_elemento,
                mutable,
                nombre,
                iterable,
                cuerpo,
            } => {
                let tipo_iterable = self.inferir_expresion(iterable);
                if !matches!(
                    tipo_iterable,
                    TipoSemantico::Lista(_) | TipoSemantico::Desconocido | TipoSemantico::Jsn
                ) {
                    self.error(
                        ErrorQuetzal::nuevo(
                            "E0204",
                            CategoriaError::Semantico,
                            format!(
                                "no puedes iterar sobre un valor de tipo '{}'",
                                tipo_iterable.nombre()
                            ),
                        )
                        .con_ubicacion(iterable.ubicacion)
                        .con_etiqueta("se esperaba una lista"),
                    );
                }
                self.tabla.abrir_ambito();
                self.declarar_simbolo(
                    nombre,
                    TipoSemantico::desde_ast(tipo_elemento),
                    *mutable,
                    sentencia.ubicacion,
                );
                self.profundidad_bucle += 1;
                self.analizar_bloque_sin_ambito(cuerpo);
                self.profundidad_bucle -= 1;
                self.tabla.cerrar_ambito();
            }
            NodoSentencia::Romper | NodoSentencia::Continuar => {
                if self.profundidad_bucle == 0 {
                    let palabra = if matches!(sentencia.nodo, NodoSentencia::Romper) {
                        "romper"
                    } else {
                        "continuar"
                    };
                    self.error(
                        ErrorQuetzal::nuevo(
                            "E0211",
                            CategoriaError::Semantico,
                            format!("'{palabra}' solo puede usarse dentro de un bucle"),
                        )
                        .con_ubicacion(sentencia.ubicacion)
                        .con_etiqueta("fuera de un bucle"),
                    );
                }
            }
            NodoSentencia::Retornar(valor) => self.analizar_retornar(valor, sentencia.ubicacion),
            NodoSentencia::Intentar {
                bloque,
                captura,
                finalmente,
            } => {
                self.analizar_bloque(bloque);
                if let Some(captura) = captura {
                    self.tabla.abrir_ambito();
                    self.declarar_simbolo(
                        &captura.nombre,
                        TipoSemantico::Desconocido,
                        false,
                        sentencia.ubicacion,
                    );
                    self.analizar_bloque_sin_ambito(&captura.bloque);
                    self.tabla.cerrar_ambito();
                }
                if let Some(bloque) = finalmente {
                    self.analizar_bloque(bloque);
                }
            }
            NodoSentencia::Lanzar(expresion) => {
                self.inferir_expresion(expresion);
            }
        }
    }

    fn analizar_bloque_de_bucle(&mut self, cuerpo: &Bloque) {
        self.profundidad_bucle += 1;
        self.analizar_bloque(cuerpo);
        self.profundidad_bucle -= 1;
    }

    fn analizar_declaracion(
        &mut self,
        tipo: &Tipo,
        mutable: bool,
        nombre: &str,
        valor: &Option<Expresion>,
        ubicacion: Ubicacion,
    ) {
        if matches!(tipo, Tipo::Vacio) {
            self.error(
                ErrorQuetzal::nuevo(
                    "E0206",
                    CategoriaError::Semantico,
                    "el tipo 'vacio' solo puede usarse como retorno de funciones",
                )
                .con_ubicacion(ubicacion)
                .con_etiqueta("tipo inválido para una variable")
                .con_ayuda("usa un tipo concreto como entero, texto o jsn"),
            );
        }

        let tipo_declarado = TipoSemantico::desde_ast(tipo);

        // Verificar que los tipos nombrados existan (objetos, prototipos o
        // imports). `funcion` es un tipo del lenguaje, no un nombre a resolver.
        if let Tipo::Nombrado(nombre_tipo) = tipo
            && tipo_declarado != TipoSemantico::FuncionDinamica
            && !self.objetos.contains_key(nombre_tipo)
            && !self.prototipos.contains_key(nombre_tipo)
            && !self.tabla.existe(nombre_tipo)
        {
            self.error(
                ErrorQuetzal::nuevo(
                    "E0201",
                    CategoriaError::Semantico,
                    format!("el tipo '{nombre_tipo}' no está definido"),
                )
                .con_ubicacion(ubicacion)
                .con_etiqueta("tipo no definido")
                .con_ayuda("define el objeto o impórtalo antes de usarlo"),
            );
        }

        if let Some(valor) = valor {
            let tipo_valor = self.inferir_expresion(valor);
            if !tipo_valor.es_asignable_a(&tipo_declarado) {
                self.error(
                    ErrorQuetzal::nuevo(
                        "E0204",
                        CategoriaError::Semantico,
                        format!(
                            "no puedes asignar un valor de tipo '{}' a una variable de tipo '{}'",
                            tipo_valor.nombre(),
                            tipo_declarado.nombre()
                        ),
                    )
                    .con_ubicacion(valor.ubicacion)
                    .con_etiqueta(format!("este valor es de tipo '{}'", tipo_valor.nombre())),
                );
            }
        }

        self.declarar_simbolo(nombre, tipo_declarado, mutable, ubicacion);
    }

    fn analizar_asignacion(
        &mut self,
        objetivo: &Expresion,
        operador: OperadorAsignacion,
        valor: &Expresion,
    ) {
        let tipo_valor = self.inferir_expresion(valor);

        match &objetivo.nodo {
            NodoExpresion::Identificador(nombre) => {
                let Some(simbolo) = self.tabla.buscar(nombre).cloned() else {
                    self.error_variable_no_definida(nombre, objetivo.ubicacion);
                    return;
                };
                if !simbolo.mutable {
                    self.error_reasignacion_constante(nombre, objetivo.ubicacion);
                    return;
                }
                if operador == OperadorAsignacion::Asignar
                    && !tipo_valor.es_asignable_a(&simbolo.tipo)
                {
                    self.error(
                        ErrorQuetzal::nuevo(
                            "E0204",
                            CategoriaError::Semantico,
                            format!(
                                "no puedes asignar un valor de tipo '{}' a '{nombre}' de tipo '{}'",
                                tipo_valor.nombre(),
                                simbolo.tipo.nombre()
                            ),
                        )
                        .con_ubicacion(valor.ubicacion)
                        .con_etiqueta(format!("este valor es de tipo '{}'", tipo_valor.nombre())),
                    );
                }
            }
            NodoExpresion::AccesoMiembro { objeto, miembro } => {
                let tipo_objeto = self.inferir_expresion(objeto);
                match tipo_objeto {
                    TipoSemantico::Objeto(nombre_objeto) => {
                        if let Some(atributo) = self.buscar_atributo(&nombre_objeto, miembro) {
                            // Los constructores pueden inicializar atributos constantes.
                            let inicializando =
                                self.en_constructor && matches!(objeto.nodo, NodoExpresion::Esto);
                            if !atributo.mutable && !inicializando {
                                self.error(
                                    ErrorQuetzal::nuevo(
                                        "E0203",
                                        CategoriaError::Semantico,
                                        format!(
                                            "no puedes modificar la propiedad constante '{miembro}'"
                                        ),
                                    )
                                    .con_ubicacion(objetivo.ubicacion)
                                    .con_etiqueta("esta propiedad fue declarada como constante")
                                    .con_ayuda("declara la propiedad como mutable usando 'var'"),
                                );
                            }
                            if !atributo.publico && !self.dentro_del_objeto(&nombre_objeto) {
                                self.error_miembro_privado(miembro, objetivo.ubicacion);
                            }
                        }
                        // Si el atributo no existe se reporta en la inferencia.
                    }
                    TipoSemantico::Jsn => {
                        // Para modificar un jsn, su raíz debe ser mutable.
                        self.exigir_raiz_mutable(objeto);
                    }
                    _ => {}
                }
            }
            NodoExpresion::Indexacion { objeto, .. } => {
                self.exigir_raiz_mutable(objeto);
            }
            _ => {
                self.error(
                    ErrorQuetzal::nuevo(
                        "E0104",
                        CategoriaError::Semantico,
                        "el objetivo de la asignación no es válido",
                    )
                    .con_ubicacion(objetivo.ubicacion)
                    .con_etiqueta("no se puede asignar aquí"),
                );
            }
        }
    }

    /// Para modificar estructuras (`jsn`, listas) la variable raíz debe ser mutable.
    fn exigir_raiz_mutable(&mut self, expresion: &Expresion) {
        let mut actual = expresion;
        loop {
            match &actual.nodo {
                NodoExpresion::AccesoMiembro { objeto, .. } => actual = objeto,
                NodoExpresion::Indexacion { objeto, .. } => actual = objeto,
                NodoExpresion::Identificador(nombre) => {
                    if let Some(simbolo) = self.tabla.buscar(nombre).cloned()
                        && !simbolo.mutable
                        && matches!(simbolo.tipo, TipoSemantico::Jsn | TipoSemantico::Lista(_))
                    {
                        self.error_reasignacion_constante(nombre, actual.ubicacion);
                    }
                    return;
                }
                _ => return,
            }
        }
    }

    fn analizar_incremento(&mut self, objetivo: &Expresion) {
        if let NodoExpresion::Identificador(nombre) = &objetivo.nodo {
            let Some(simbolo) = self.tabla.buscar(nombre).cloned() else {
                self.error_variable_no_definida(nombre, objetivo.ubicacion);
                return;
            };
            if !simbolo.mutable {
                self.error_reasignacion_constante(nombre, objetivo.ubicacion);
            }
            if !simbolo.tipo.es_numerico() {
                self.error(
                    ErrorQuetzal::nuevo(
                        "E0204",
                        CategoriaError::Semantico,
                        format!(
                            "'{nombre}' es de tipo '{}' y no admite incremento o decremento",
                            simbolo.tipo.nombre()
                        ),
                    )
                    .con_ubicacion(objetivo.ubicacion)
                    .con_etiqueta("se esperaba un valor numérico"),
                );
            }
        } else {
            self.inferir_expresion(objetivo);
        }
    }

    fn analizar_retornar(&mut self, valor: &Option<Expresion>, ubicacion: Ubicacion) {
        let Some(esperado) = self.pila_retorno.last().cloned() else {
            self.error(
                ErrorQuetzal::nuevo(
                    "E0205",
                    CategoriaError::Semantico,
                    "'retornar' solo puede usarse dentro de una función",
                )
                .con_ubicacion(ubicacion)
                .con_etiqueta("fuera de una función"),
            );
            return;
        };

        match (valor, &esperado) {
            (Some(expresion), TipoSemantico::Vacio) => {
                self.inferir_expresion(expresion);
                self.error(
                    ErrorQuetzal::nuevo(
                        "E0205",
                        CategoriaError::Semantico,
                        "una función 'vacio' no puede retornar un valor",
                    )
                    .con_ubicacion(expresion.ubicacion)
                    .con_etiqueta("valor no esperado")
                    .con_ayuda("usa 'retornar' sin valor o cambia el tipo de retorno"),
                );
            }
            (Some(expresion), _) => {
                let tipo_valor = self.inferir_expresion(expresion);
                if !tipo_valor.es_asignable_a(&esperado) {
                    self.error(
                        ErrorQuetzal::nuevo(
                            "E0205",
                            CategoriaError::Semantico,
                            format!(
                                "la función debe retornar '{}', pero retorna '{}'",
                                esperado.nombre(),
                                tipo_valor.nombre()
                            ),
                        )
                        .con_ubicacion(expresion.ubicacion)
                        .con_etiqueta(format!("este valor es de tipo '{}'", tipo_valor.nombre())),
                    );
                }
            }
            (None, TipoSemantico::Vacio) => {}
            (None, _) => {
                self.error(
                    ErrorQuetzal::nuevo(
                        "E0205",
                        CategoriaError::Semantico,
                        format!(
                            "la función debe retornar un valor de tipo '{}'",
                            esperado.nombre()
                        ),
                    )
                    .con_ubicacion(ubicacion)
                    .con_etiqueta("falta el valor de retorno"),
                );
            }
        }
    }

    fn exigir_condicion_logica(&mut self, condicion: &Expresion) {
        let tipo = self.inferir_expresion(condicion);
        if !matches!(
            tipo,
            TipoSemantico::Log | TipoSemantico::Desconocido | TipoSemantico::Nulo
        ) {
            self.error(
                ErrorQuetzal::nuevo(
                    "E0204",
                    CategoriaError::Semantico,
                    format!("la condición debe ser 'log', pero es '{}'", tipo.nombre()),
                )
                .con_ubicacion(condicion.ubicacion)
                .con_etiqueta("se esperaba un valor lógico"),
            );
        }
    }

    // ----- Inferencia de expresiones -----

    fn inferir_expresion(&mut self, expresion: &Expresion) -> TipoSemantico {
        match &expresion.nodo {
            NodoExpresion::LiteralEntero(_) => TipoSemantico::Entero,
            NodoExpresion::LiteralNumero(_) => TipoSemantico::Numero,
            NodoExpresion::LiteralTexto(_) => TipoSemantico::Texto,
            NodoExpresion::LiteralLog(_) => TipoSemantico::Log,
            NodoExpresion::Nulo => TipoSemantico::Nulo,
            NodoExpresion::TextoInterpolado(segmentos) => {
                for segmento in segmentos {
                    if let SegmentoInterpolado::Expresion(interior) = segmento {
                        self.inferir_expresion(interior);
                    }
                }
                TipoSemantico::Texto
            }
            NodoExpresion::ListaLiteral(elementos) => {
                let tipos: Vec<TipoSemantico> = elementos
                    .iter()
                    .map(|elemento| self.inferir_expresion(elemento))
                    .collect();
                let homogenea = tipos
                    .windows(2)
                    .all(|par| par[0] == par[1] && par[0] != TipoSemantico::Desconocido);
                match tipos.first() {
                    Some(primero) if homogenea => {
                        TipoSemantico::Lista(Some(Box::new(primero.clone())))
                    }
                    _ => TipoSemantico::Lista(None),
                }
            }
            NodoExpresion::JsnLiteral(entradas) => {
                for (_, valor) in entradas {
                    self.inferir_expresion(valor);
                }
                TipoSemantico::Jsn
            }
            NodoExpresion::Identificador(nombre) => match self.tabla.buscar(nombre) {
                Some(simbolo) => simbolo.tipo.clone(),
                None => {
                    self.error_variable_no_definida(nombre, expresion.ubicacion);
                    TipoSemantico::Desconocido
                }
            },
            NodoExpresion::Esto => match &self.objeto_actual {
                Some(nombre) => TipoSemantico::Objeto(nombre.clone()),
                None => {
                    self.error(
                        ErrorQuetzal::nuevo(
                            "E0212",
                            CategoriaError::Semantico,
                            "'esto' solo puede usarse dentro de un objeto",
                        )
                        .con_ubicacion(expresion.ubicacion)
                        .con_etiqueta("fuera de un objeto"),
                    );
                    TipoSemantico::Desconocido
                }
            },
            // `padre` se valida en runtime según la cadena de herencia real.
            NodoExpresion::Padre => TipoSemantico::Desconocido,
            NodoExpresion::Binaria {
                operador,
                izquierda,
                derecha,
            } => self.inferir_binaria(*operador, izquierda, derecha, expresion.ubicacion),
            NodoExpresion::Unaria { operador, operando } => {
                let tipo = self.inferir_expresion(operando);
                match operador {
                    OperadorUnario::Negacion => {
                        if !tipo.es_numerico() && tipo != TipoSemantico::Nulo {
                            self.error(
                                ErrorQuetzal::nuevo(
                                    "E0204",
                                    CategoriaError::Semantico,
                                    format!("no puedes negar un valor de tipo '{}'", tipo.nombre()),
                                )
                                .con_ubicacion(operando.ubicacion)
                                .con_etiqueta("se esperaba un valor numérico"),
                            );
                            TipoSemantico::Desconocido
                        } else {
                            tipo
                        }
                    }
                    OperadorUnario::NoLogico => TipoSemantico::Log,
                }
            }
            NodoExpresion::Ternaria {
                condicion,
                si_verdadero,
                si_falso,
            } => {
                self.exigir_condicion_logica(condicion);
                let tipo_verdadero = self.inferir_expresion(si_verdadero);
                let tipo_falso = self.inferir_expresion(si_falso);
                if tipo_verdadero == tipo_falso {
                    tipo_verdadero
                } else if tipo_verdadero.es_asignable_a(&tipo_falso) {
                    tipo_falso
                } else if tipo_falso.es_asignable_a(&tipo_verdadero) {
                    tipo_verdadero
                } else {
                    TipoSemantico::Desconocido
                }
            }
            NodoExpresion::Llamada {
                objetivo,
                argumentos,
            } => self.inferir_llamada(objetivo, argumentos, expresion.ubicacion),
            NodoExpresion::AccesoMiembro { objeto, miembro } => {
                self.inferir_acceso_miembro(objeto, miembro, expresion.ubicacion)
            }
            NodoExpresion::Indexacion { objeto, indice } => {
                let tipo_objeto = self.inferir_expresion(objeto);
                self.inferir_expresion(indice);
                match tipo_objeto {
                    TipoSemantico::Lista(Some(interior)) => *interior,
                    TipoSemantico::Lista(None)
                    | TipoSemantico::Jsn
                    | TipoSemantico::Desconocido => TipoSemantico::Desconocido,
                    TipoSemantico::Texto => TipoSemantico::Texto,
                    otro => {
                        self.error(
                            ErrorQuetzal::nuevo(
                                "E0204",
                                CategoriaError::Semantico,
                                format!("no puedes indexar un valor de tipo '{}'", otro.nombre()),
                            )
                            .con_ubicacion(objeto.ubicacion)
                            .con_etiqueta("este valor no admite índices"),
                        );
                        TipoSemantico::Desconocido
                    }
                }
            }
            NodoExpresion::Nuevo { clase, argumentos } => {
                for argumento in argumentos {
                    self.inferir_expresion(argumento);
                }
                if let Some(info) = self.objetos.get(clase) {
                    let cantidad = argumentos.len();
                    let compatible = if info.constructores.is_empty() {
                        cantidad == 0
                    } else {
                        info.constructores.contains(&cantidad)
                    };
                    if !compatible {
                        self.error(
                            ErrorQuetzal::nuevo(
                                "E0210",
                                CategoriaError::Semantico,
                                format!(
                                    "el constructor de '{clase}' no acepta {cantidad} argumentos"
                                ),
                            )
                            .con_ubicacion(expresion.ubicacion)
                            .con_etiqueta("cantidad de argumentos inválida"),
                        );
                    }
                    TipoSemantico::Objeto(clase.clone())
                } else if self.tabla.existe(clase) {
                    // Objeto importado: se valida al cargar el grafo de módulos.
                    TipoSemantico::Desconocido
                } else {
                    self.error(
                        ErrorQuetzal::nuevo(
                            "E0201",
                            CategoriaError::Semantico,
                            format!("el objeto '{clase}' no está definido"),
                        )
                        .con_ubicacion(expresion.ubicacion)
                        .con_etiqueta("objeto no definido")
                        .con_ayuda("define el objeto o impórtalo antes de instanciarlo"),
                    );
                    TipoSemantico::Desconocido
                }
            }
            NodoExpresion::Esperar(interior) => {
                // Regla: `esperar` solo es válido dentro de una función
                // `asincrono` o a nivel de scope global (top-level). Dentro
                // de una función síncrona es un error, porque esa función no
                // forma parte del bucle de eventos.
                if !self.esperar_es_valido() {
                    self.error(
                        ErrorQuetzal::nuevo(
                            "E0213",
                            CategoriaError::Semantico,
                            "'esperar' solo puede usarse dentro de una función 'asincrono' o a nivel de scope global",
                        )
                        .con_ubicacion(expresion.ubicacion)
                        .con_etiqueta("'esperar' dentro de una función síncrona")
                        .con_ayuda(
                            "declara la función como 'asincrono' para poder usar 'esperar' en su cuerpo, o traslada el 'esperar' al scope global",
                        ),
                    );
                }
                self.inferir_expresion(interior)
            }
        }
    }

    fn inferir_binaria(
        &mut self,
        operador: OperadorBinario,
        izquierda: &Expresion,
        derecha: &Expresion,
        ubicacion: Ubicacion,
    ) -> TipoSemantico {
        let tipo_izquierda = self.inferir_expresion(izquierda);
        let tipo_derecha = self.inferir_expresion(derecha);

        let desconocido = tipo_izquierda == TipoSemantico::Desconocido
            || tipo_derecha == TipoSemantico::Desconocido;

        match operador {
            OperadorBinario::Sumar => {
                if tipo_izquierda == TipoSemantico::Texto || tipo_derecha == TipoSemantico::Texto {
                    return TipoSemantico::Texto;
                }
                if desconocido {
                    return TipoSemantico::Desconocido;
                }
                self.tipo_aritmetico(&tipo_izquierda, &tipo_derecha, ubicacion)
            }
            OperadorBinario::Restar
            | OperadorBinario::Multiplicar
            | OperadorBinario::Dividir
            | OperadorBinario::Modulo => {
                if desconocido {
                    return TipoSemantico::Desconocido;
                }
                self.tipo_aritmetico(&tipo_izquierda, &tipo_derecha, ubicacion)
            }
            OperadorBinario::Igual | OperadorBinario::Diferente => TipoSemantico::Log,
            OperadorBinario::Mayor
            | OperadorBinario::Menor
            | OperadorBinario::MayorOIgual
            | OperadorBinario::MenorOIgual => {
                if !(tipo_izquierda.es_numerico() && tipo_derecha.es_numerico())
                    && tipo_izquierda != tipo_derecha
                {
                    self.error(
                        ErrorQuetzal::nuevo(
                            "E0204",
                            CategoriaError::Semantico,
                            format!(
                                "no puedes comparar '{}' con '{}'",
                                tipo_izquierda.nombre(),
                                tipo_derecha.nombre()
                            ),
                        )
                        .con_ubicacion(ubicacion)
                        .con_etiqueta("tipos incompatibles"),
                    );
                }
                TipoSemantico::Log
            }
            OperadorBinario::Y | OperadorBinario::O => TipoSemantico::Log,
        }
    }

    fn tipo_aritmetico(
        &mut self,
        izquierda: &TipoSemantico,
        derecha: &TipoSemantico,
        ubicacion: Ubicacion,
    ) -> TipoSemantico {
        if !izquierda.es_numerico() || !derecha.es_numerico() {
            self.error(
                ErrorQuetzal::nuevo(
                    "E0204",
                    CategoriaError::Semantico,
                    format!(
                        "operación aritmética inválida entre '{}' y '{}'",
                        izquierda.nombre(),
                        derecha.nombre()
                    ),
                )
                .con_ubicacion(ubicacion)
                .con_etiqueta("tipos incompatibles"),
            );
            return TipoSemantico::Desconocido;
        }
        if izquierda == &TipoSemantico::Entero && derecha == &TipoSemantico::Entero {
            TipoSemantico::Entero
        } else {
            TipoSemantico::Numero
        }
    }

    fn inferir_llamada(
        &mut self,
        objetivo: &Expresion,
        argumentos: &[Expresion],
        ubicacion: Ubicacion,
    ) -> TipoSemantico {
        let tipo_objetivo = self.inferir_expresion(objetivo);
        let tipos_argumentos: Vec<TipoSemantico> = argumentos
            .iter()
            .map(|argumento| self.inferir_expresion(argumento))
            .collect();

        match tipo_objetivo {
            TipoSemantico::Funcion {
                parametros,
                retorno,
            } => {
                if parametros.len() != tipos_argumentos.len() {
                    self.error(
                        ErrorQuetzal::nuevo(
                            "E0210",
                            CategoriaError::Semantico,
                            format!(
                                "la función espera {} argumentos, pero recibe {}",
                                parametros.len(),
                                tipos_argumentos.len()
                            ),
                        )
                        .con_ubicacion(ubicacion)
                        .con_etiqueta("cantidad de argumentos inválida"),
                    );
                } else {
                    for (indice, (esperado, recibido)) in
                        parametros.iter().zip(&tipos_argumentos).enumerate()
                    {
                        if !recibido.es_asignable_a(esperado) {
                            self.error(
                                ErrorQuetzal::nuevo(
                                    "E0204",
                                    CategoriaError::Semantico,
                                    format!(
                                        "el argumento {} debe ser '{}', pero es '{}'",
                                        indice + 1,
                                        esperado.nombre(),
                                        recibido.nombre()
                                    ),
                                )
                                .con_ubicacion(argumentos[indice].ubicacion)
                                .con_etiqueta(format!(
                                    "este valor es de tipo '{}'",
                                    recibido.nombre()
                                )),
                            );
                        }
                    }
                }
                *retorno
            }
            TipoSemantico::FuncionDinamica => TipoSemantico::Desconocido,
            TipoSemantico::Desconocido => TipoSemantico::Desconocido,
            otro => {
                self.error(
                    ErrorQuetzal::nuevo(
                        "E0209",
                        CategoriaError::Semantico,
                        format!("un valor de tipo '{}' no puede llamarse", otro.nombre()),
                    )
                    .con_ubicacion(objetivo.ubicacion)
                    .con_etiqueta("esto no es una función"),
                );
                TipoSemantico::Desconocido
            }
        }
    }

    fn inferir_acceso_miembro(
        &mut self,
        objeto: &Expresion,
        miembro: &str,
        ubicacion: Ubicacion,
    ) -> TipoSemantico {
        let tipo_objeto = self.inferir_expresion(objeto);

        match tipo_objeto {
            TipoSemantico::Objeto(nombre_objeto) => {
                if let Some(atributo) = self.buscar_atributo(&nombre_objeto, miembro) {
                    if !atributo.publico && !self.dentro_del_objeto(&nombre_objeto) {
                        self.error_miembro_privado(miembro, ubicacion);
                    }
                    return atributo.tipo;
                }
                if let Some(metodo) = self.buscar_metodo(&nombre_objeto, miembro) {
                    if !metodo.publico && !self.dentro_del_objeto(&nombre_objeto) {
                        self.error_miembro_privado(miembro, ubicacion);
                    }
                    return TipoSemantico::Funcion {
                        parametros: metodo.parametros,
                        retorno: Box::new(metodo.retorno),
                    };
                }
                if self.objetos.contains_key(&nombre_objeto) {
                    self.error(
                        ErrorQuetzal::nuevo(
                            "E0201",
                            CategoriaError::Semantico,
                            format!("el objeto '{nombre_objeto}' no tiene un miembro '{miembro}'"),
                        )
                        .con_ubicacion(ubicacion)
                        .con_etiqueta("miembro no definido"),
                    );
                }
                TipoSemantico::Desconocido
            }
            // Métodos nativos de los tipos primitivos (`.texto()`, `.longitud()`,
            // `.claves()`, ...) se validan en runtime.
            _ => TipoSemantico::Desconocido,
        }
    }

    // ----- Errores comunes -----

    fn error_variable_no_definida(&mut self, nombre: &str, ubicacion: Ubicacion) {
        self.error(
            ErrorQuetzal::nuevo(
                "E0201",
                CategoriaError::Semantico,
                format!("'{nombre}' no está definido"),
            )
            .con_ubicacion(ubicacion)
            .con_etiqueta("no se encontró esta variable")
            .con_ayuda(format!("declara '{nombre}' antes de usarlo")),
        );
    }

    fn error_reasignacion_constante(&mut self, nombre: &str, ubicacion: Ubicacion) {
        self.error(
            ErrorQuetzal::nuevo(
                "E0203",
                CategoriaError::Semantico,
                "no puedes reasignar una variable constante",
            )
            .con_ubicacion(ubicacion)
            .con_etiqueta("esta variable fue declarada como constante")
            .con_ayuda(format!(
                "declara la variable como mutable usando 'var': por ejemplo 'entero var {nombre} = ...'"
            )),
        );
    }

    fn error_miembro_privado(&mut self, miembro: &str, ubicacion: Ubicacion) {
        self.error(
            ErrorQuetzal::nuevo(
                "E0207",
                CategoriaError::Semantico,
                format!("'{miembro}' es privado y no puede accederse desde aquí"),
            )
            .con_ubicacion(ubicacion)
            .con_etiqueta("miembro privado")
            .con_ayuda("muévelo a la sección 'publico:' o accede a él desde el propio objeto"),
        );
    }
}
