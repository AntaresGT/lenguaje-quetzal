//! Generador de bytecode: convierte el AST validado en un módulo compilado.

use std::collections::HashMap;
use std::str::FromStr;

use ast::{
    Bloque, ClaseMiembro, ClaveJsn, DefinicionObjeto, Elemento, Expresion, Funcion, Modulo,
    NodoExpresion, NodoSentencia, OperadorAsignacion, OperadorBinario, OperadorUnario,
    SegmentoInterpolado, Sentencia, Tipo, Visibilidad,
};
use nucleo::{CategoriaError, ErrorQuetzal, Ubicacion};
use rust_decimal::Decimal;

use crate::constantes::Constante;
use crate::instruccion::{Instruccion, Trozo};
use crate::modulo::{
    AtributoCompilado, FuncionCompilada, ImportacionCompilada, MetodoCompilado, ModuloCompilado,
    ObjetoCompilado,
};

/// Genera el bytecode de un módulo completo.
pub fn generar_modulo(modulo: &Modulo) -> Result<ModuloCompilado, ErrorQuetzal> {
    let mut generador = Generador::nuevo(&modulo.nombre);
    generador.generar(modulo)
}

/// Contexto de un bucle activo para `romper`/`continuar`.
#[derive(Debug, Default)]
struct ContextoBucle {
    rupturas: Vec<usize>,
    continuaciones: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TipoRapido {
    Entero,
    Otro,
}

#[derive(Debug, Clone)]
struct Local {
    nombre: String,
    ranura: u16,
    profundidad: u32,
    tipo: TipoRapido,
}

#[derive(Debug, Clone, Copy)]
struct Global {
    indice: u16,
    tipo: TipoRapido,
}

#[derive(Debug, Clone, Copy)]
enum Destino {
    Local(u16, TipoRapido),
    Global(u16, TipoRapido),
    Externo(u32),
}

struct Generador {
    archivo: String,
    constantes: Vec<Constante>,
    nombres: Vec<String>,
    indice_nombres: HashMap<String, u32>,
    bucles: Vec<ContextoBucle>,
    globales: HashMap<String, Global>,
    nombres_globales: Vec<String>,
    globales_mutables: Vec<bool>,
    locales: Vec<Local>,
    profundidad: u32,
    ranuras_max: u16,
}

impl Generador {
    fn nuevo(archivo: &str) -> Self {
        Self {
            archivo: archivo.to_string(),
            constantes: Vec::new(),
            nombres: Vec::new(),
            indice_nombres: HashMap::new(),
            bucles: Vec::new(),
            globales: HashMap::new(),
            nombres_globales: Vec::new(),
            globales_mutables: Vec::new(),
            locales: Vec::new(),
            profundidad: 0,
            ranuras_max: 0,
        }
    }

    fn generar(&mut self, modulo: &Modulo) -> Result<ModuloCompilado, ErrorQuetzal> {
        let mut compilado = ModuloCompilado {
            nombre: modulo.nombre.clone(),
            ..Default::default()
        };

        self.recolectar_globales(modulo);

        for elemento in &modulo.elementos {
            match elemento {
                Elemento::Importacion(importacion) => {
                    compilado.importaciones.push(ImportacionCompilada {
                        simbolos: importacion
                            .simbolos
                            .iter()
                            .map(|simbolo| {
                                (simbolo.nombre.clone(), simbolo.nombre_local().to_string())
                            })
                            .collect(),
                        origen: importacion.origen.clone(),
                    });
                }
                Elemento::Exportacion { simbolos, .. } => {
                    compilado.exportaciones.extend(simbolos.iter().cloned());
                }
                Elemento::Funcion(funcion) => {
                    compilado.funciones.push(self.generar_funcion(funcion)?);
                }
                Elemento::Objeto(objeto) => {
                    compilado.objetos.push(self.generar_objeto(objeto)?);
                }
                Elemento::Prototipo(_) => {
                    // Los prototipos se validan en semántica; no generan código.
                }
                Elemento::Sentencia(_) => {}
            }
        }

        self.reiniciar_locales();
        self.profundidad = 0;
        for elemento in &modulo.elementos {
            if let Elemento::Sentencia(sentencia) = elemento {
                self.generar_sentencia(&mut compilado.principal, sentencia)?;
            }
        }

        compilado.constantes = std::mem::take(&mut self.constantes);
        compilado.nombres = std::mem::take(&mut self.nombres);
        compilado.nombres_globales = std::mem::take(&mut self.nombres_globales);
        compilado.globales_mutables = std::mem::take(&mut self.globales_mutables);
        compilado.ranuras_locales_principal = self.ranuras_max;
        Ok(compilado)
    }

    fn recolectar_globales(&mut self, modulo: &Modulo) {
        for elemento in &modulo.elementos {
            match elemento {
                Elemento::Importacion(importacion) => {
                    for simbolo in &importacion.simbolos {
                        self.declarar_global(simbolo.nombre_local(), false, TipoRapido::Otro);
                    }
                }
                Elemento::Funcion(funcion) => {
                    self.declarar_global(
                        &funcion.nombre,
                        false,
                        tipo_rapido(&funcion.tipo_retorno),
                    );
                }
                Elemento::Objeto(objeto) => {
                    self.declarar_global(&objeto.nombre, false, TipoRapido::Otro);
                }
                Elemento::Sentencia(sentencia) => {
                    if let NodoSentencia::DeclaracionVariable {
                        mutable,
                        nombre,
                        tipo,
                        ..
                    } = &sentencia.nodo
                    {
                        self.declarar_global(nombre, *mutable, tipo_rapido(tipo));
                    }
                }
                Elemento::Exportacion { .. } | Elemento::Prototipo(_) => {}
            }
        }
    }

    fn declarar_global(&mut self, nombre: &str, mutable: bool, tipo: TipoRapido) {
        if self.globales.contains_key(nombre) {
            return;
        }
        let indice = self.nombres_globales.len() as u16;
        self.nombres_globales.push(nombre.to_string());
        self.globales_mutables.push(mutable);
        self.globales
            .insert(nombre.to_string(), Global { indice, tipo });
    }

    fn reiniciar_locales(&mut self) {
        self.locales.clear();
        self.ranuras_max = 0;
    }

    // ----- Tablas -----

    fn indice_constante(&mut self, constante: Constante) -> u32 {
        if let Some(indice) = self
            .constantes
            .iter()
            .position(|existente| existente == &constante)
        {
            return indice as u32;
        }
        self.constantes.push(constante);
        (self.constantes.len() - 1) as u32
    }

    fn indice_nombre(&mut self, nombre: &str) -> u32 {
        if let Some(indice) = self.indice_nombres.get(nombre) {
            return *indice;
        }
        let indice = self.nombres.len() as u32;
        self.nombres.push(nombre.to_string());
        self.indice_nombres.insert(nombre.to_string(), indice);
        indice
    }

    fn agregar_local(&mut self, nombre: &str, tipo: TipoRapido) -> u16 {
        let ranura = self.locales.len() as u16;
        self.locales.push(Local {
            nombre: nombre.to_string(),
            ranura,
            profundidad: self.profundidad,
            tipo,
        });
        self.ranuras_max = self.ranuras_max.max(ranura + 1);
        ranura
    }

    fn abrir_ambito(&mut self) {
        self.profundidad += 1;
    }

    fn cerrar_ambito(&mut self, trozo: &mut Trozo, ubicacion: Ubicacion) {
        let desde = self.locales.len() as u16;
        let mut marca = desde;
        while self
            .locales
            .last()
            .is_some_and(|local| local.profundidad == self.profundidad)
        {
            marca = self.locales.pop().map(|local| local.ranura).unwrap_or(0);
        }
        if self.profundidad > 0 {
            self.profundidad -= 1;
        }
        if marca < desde {
            trozo.emitir(Instruccion::LimpiarLocales(marca), ubicacion);
        }
    }

    fn resolver(&mut self, nombre: &str) -> Destino {
        for local in self.locales.iter().rev() {
            if local.nombre == nombre {
                return Destino::Local(local.ranura, local.tipo);
            }
        }
        if let Some(global) = self.globales.get(nombre) {
            return Destino::Global(global.indice, global.tipo);
        }
        Destino::Externo(self.indice_nombre(nombre))
    }

    fn emitir_carga(
        &mut self,
        trozo: &mut Trozo,
        nombre: &str,
        ubicacion: Ubicacion,
    ) -> TipoRapido {
        match self.resolver(nombre) {
            Destino::Local(ranura, tipo) => {
                trozo.emitir(Instruccion::CargarLocal(ranura), ubicacion);
                tipo
            }
            Destino::Global(indice, tipo) => {
                trozo.emitir(Instruccion::CargarGlobal(indice), ubicacion);
                tipo
            }
            Destino::Externo(indice) => {
                trozo.emitir(Instruccion::CargarExterno(indice), ubicacion);
                TipoRapido::Otro
            }
        }
    }

    fn emitir_guarda(&mut self, trozo: &mut Trozo, nombre: &str, ubicacion: Ubicacion) {
        match self.resolver(nombre) {
            Destino::Local(ranura, _) => {
                trozo.emitir(Instruccion::GuardarLocal(ranura), ubicacion);
            }
            Destino::Global(indice, _) => {
                trozo.emitir(Instruccion::GuardarGlobal(indice), ubicacion);
            }
            Destino::Externo(indice) => {
                trozo.emitir(Instruccion::GuardarExterno(indice), ubicacion);
            }
        }
    }

    fn emitir_nulo(&mut self, trozo: &mut Trozo, ubicacion: Ubicacion) {
        let indice = self.indice_constante(Constante::Nulo);
        trozo.emitir(Instruccion::CargarConstante(indice), ubicacion);
    }

    // ----- Funciones y objetos -----

    fn generar_funcion(&mut self, funcion: &Funcion) -> Result<FuncionCompilada, ErrorQuetzal> {
        self.reiniciar_locales();
        self.profundidad = 1;
        for parametro in &funcion.parametros {
            self.agregar_local(&parametro.nombre, tipo_rapido(&parametro.tipo));
        }

        let mut trozo = Trozo::default();
        self.generar_bloque_sin_ambito(&mut trozo, &funcion.cuerpo)?;
        self.emitir_nulo(&mut trozo, funcion.ubicacion);
        trozo.emitir(Instruccion::Retornar, funcion.ubicacion);

        Ok(FuncionCompilada {
            nombre: funcion.nombre.clone(),
            parametros: funcion
                .parametros
                .iter()
                .map(|parametro| (parametro.nombre.clone(), parametro.mutable))
                .collect(),
            trozo,
            asincrona: funcion.asincrona,
            ranuras_locales: self.ranuras_max,
        })
    }

    fn generar_objeto(
        &mut self,
        objeto: &DefinicionObjeto,
    ) -> Result<ObjetoCompilado, ErrorQuetzal> {
        let mut compilado = ObjetoCompilado {
            nombre: objeto.nombre.clone(),
            padres: objeto.padres.clone(),
            extiende_como: objeto.extiende_como.clone(),
            ..Default::default()
        };

        for miembro in &objeto.miembros {
            let publico = miembro.visibilidad == Visibilidad::Publico;
            match &miembro.clase {
                ClaseMiembro::Atributo {
                    mutable,
                    nombre,
                    valor_inicial,
                    ubicacion,
                    ..
                } => {
                    let inicial = match valor_inicial {
                        Some(valor) => {
                            self.reiniciar_locales();
                            self.profundidad = 0;
                            let mut trozo = Trozo::default();
                            self.generar_expresion(&mut trozo, valor)?;
                            trozo.emitir(Instruccion::Retornar, *ubicacion);
                            Some(trozo)
                        }
                        None => None,
                    };
                    compilado.atributos.push(AtributoCompilado {
                        nombre: nombre.clone(),
                        mutable: *mutable,
                        publico,
                        libre: miembro.libre,
                        inicial,
                    });
                }
                ClaseMiembro::Constructor(funcion) => {
                    compilado.constructores.push(self.generar_funcion(funcion)?);
                }
                ClaseMiembro::Metodo(funcion) => {
                    compilado.metodos.push(MetodoCompilado {
                        funcion: self.generar_funcion(funcion)?,
                        publico,
                        libre: miembro.libre,
                    });
                }
            }
        }

        Ok(compilado)
    }

    // ----- Sentencias -----

    fn generar_bloque(
        &mut self,
        trozo: &mut Trozo,
        bloque: &Bloque,
        ubicacion: Ubicacion,
    ) -> Result<(), ErrorQuetzal> {
        self.abrir_ambito();
        self.generar_bloque_sin_ambito(trozo, bloque)?;
        self.cerrar_ambito(trozo, ubicacion);
        Ok(())
    }

    fn generar_bloque_sin_ambito(
        &mut self,
        trozo: &mut Trozo,
        bloque: &Bloque,
    ) -> Result<(), ErrorQuetzal> {
        for sentencia in &bloque.sentencias {
            self.generar_sentencia(trozo, sentencia)?;
        }
        Ok(())
    }

    fn generar_sentencia(
        &mut self,
        trozo: &mut Trozo,
        sentencia: &Sentencia,
    ) -> Result<(), ErrorQuetzal> {
        let ubicacion = sentencia.ubicacion;
        match &sentencia.nodo {
            NodoSentencia::DeclaracionVariable {
                tipo,
                mutable: _,
                nombre,
                valor,
            } => {
                match valor {
                    Some(expresion) => {
                        self.generar_expresion(trozo, expresion)?;
                    }
                    None => self.emitir_nulo(trozo, ubicacion),
                }
                if self.profundidad == 0 {
                    if let Destino::Global(indice, _) = self.resolver(nombre) {
                        trozo.emitir(Instruccion::DeclararGlobal(indice), ubicacion);
                    } else {
                        self.emitir_guarda(trozo, nombre, ubicacion);
                    }
                } else {
                    let ranura = self.agregar_local(nombre, tipo_rapido(tipo));
                    trozo.emitir(Instruccion::GuardarLocal(ranura), ubicacion);
                }
            }
            NodoSentencia::Asignacion {
                objetivo,
                operador,
                valor,
            } => self.generar_asignacion(trozo, objetivo, *operador, valor)?,
            NodoSentencia::IncrementoDecremento {
                objetivo,
                incremento,
            } => {
                let operador = if *incremento {
                    OperadorAsignacion::Sumar
                } else {
                    OperadorAsignacion::Restar
                };
                let uno = Expresion::nueva(NodoExpresion::LiteralEntero(1), objetivo.ubicacion);
                self.generar_asignacion(trozo, objetivo, operador, &uno)?;
            }
            NodoSentencia::Expresion(expresion) => {
                self.generar_expresion(trozo, expresion)?;
                trozo.emitir(Instruccion::Desechar, ubicacion);
            }
            NodoSentencia::Si {
                condicion,
                entonces,
                sino,
            } => {
                self.generar_expresion(trozo, condicion)?;
                let salto_sino = trozo.emitir(Instruccion::SaltarSiFalso(0), ubicacion);
                self.generar_bloque(trozo, entonces, ubicacion)?;
                match sino {
                    Some(bloque_sino) => {
                        let salto_fin = trozo.emitir(Instruccion::Saltar(0), ubicacion);
                        trozo.parchar_salto(salto_sino, trozo.posicion_actual());
                        self.generar_bloque(trozo, bloque_sino, ubicacion)?;
                        trozo.parchar_salto(salto_fin, trozo.posicion_actual());
                    }
                    None => {
                        trozo.parchar_salto(salto_sino, trozo.posicion_actual());
                    }
                }
            }
            NodoSentencia::Mientras { condicion, cuerpo } => {
                let inicio = trozo.posicion_actual();
                self.generar_expresion(trozo, condicion)?;
                let salto_fin = trozo.emitir(Instruccion::SaltarSiFalso(0), ubicacion);
                self.bucles.push(ContextoBucle::default());
                self.generar_bloque(trozo, cuerpo, ubicacion)?;
                trozo.emitir(Instruccion::Saltar(inicio), ubicacion);
                let fin = trozo.posicion_actual();
                trozo.parchar_salto(salto_fin, fin);
                self.cerrar_bucle(trozo, fin, inicio);
            }
            NodoSentencia::HacerMientras { cuerpo, condicion } => {
                let inicio = trozo.posicion_actual();
                self.bucles.push(ContextoBucle::default());
                self.generar_bloque(trozo, cuerpo, ubicacion)?;
                let pc_condicion = trozo.posicion_actual();
                self.generar_expresion(trozo, condicion)?;
                trozo.emitir(Instruccion::SaltarSiVerdadero(inicio), ubicacion);
                let fin = trozo.posicion_actual();
                self.cerrar_bucle(trozo, fin, pc_condicion);
            }
            NodoSentencia::ParaClasico {
                inicializacion,
                condicion,
                paso,
                cuerpo,
            } => {
                self.abrir_ambito();
                self.generar_sentencia(trozo, inicializacion)?;
                let inicio = trozo.posicion_actual();
                self.generar_expresion(trozo, condicion)?;
                let salto_fin = trozo.emitir(Instruccion::SaltarSiFalso(0), ubicacion);
                self.bucles.push(ContextoBucle::default());
                self.generar_bloque(trozo, cuerpo, ubicacion)?;
                let pc_paso = trozo.posicion_actual();
                self.generar_sentencia(trozo, paso)?;
                trozo.emitir(Instruccion::Saltar(inicio), ubicacion);
                let fin = trozo.posicion_actual();
                trozo.parchar_salto(salto_fin, fin);
                self.cerrar_bucle(trozo, fin, pc_paso);
                self.cerrar_ambito(trozo, ubicacion);
            }
            NodoSentencia::ParaEn {
                tipo_elemento,
                mutable: _,
                nombre,
                iterable,
                cuerpo,
                ..
            } => {
                self.abrir_ambito();
                self.generar_expresion(trozo, iterable)?;
                trozo.emitir(Instruccion::CrearIterador, ubicacion);
                let ranura_iterador = self.agregar_local("__iterador", TipoRapido::Otro);
                trozo.emitir(Instruccion::GuardarLocal(ranura_iterador), ubicacion);

                let inicio = trozo.posicion_actual();
                trozo.emitir(Instruccion::CargarLocal(ranura_iterador), ubicacion);
                let salto_fin = trozo.emitir(Instruccion::IteradorSiguiente(0), ubicacion);

                self.abrir_ambito();
                let ranura_elemento = self.agregar_local(nombre, tipo_rapido(tipo_elemento));
                trozo.emitir(Instruccion::GuardarLocal(ranura_elemento), ubicacion);
                self.bucles.push(ContextoBucle::default());
                self.generar_bloque_sin_ambito(trozo, cuerpo)?;
                self.cerrar_ambito(trozo, ubicacion);
                trozo.emitir(Instruccion::Saltar(inicio), ubicacion);
                let fin = trozo.posicion_actual();
                trozo.parchar_salto(salto_fin, fin);
                self.cerrar_bucle(trozo, fin, inicio);
                self.cerrar_ambito(trozo, ubicacion);
            }
            NodoSentencia::Romper => {
                let salto = trozo.emitir(Instruccion::Saltar(0), ubicacion);
                if let Some(bucle) = self.bucles.last_mut() {
                    bucle.rupturas.push(salto);
                }
            }
            NodoSentencia::Continuar => {
                let salto = trozo.emitir(Instruccion::Saltar(0), ubicacion);
                if let Some(bucle) = self.bucles.last_mut() {
                    bucle.continuaciones.push(salto);
                }
            }
            NodoSentencia::Retornar(valor) => {
                match valor {
                    Some(expresion) => {
                        self.generar_expresion(trozo, expresion)?;
                    }
                    None => self.emitir_nulo(trozo, ubicacion),
                }
                trozo.emitir(Instruccion::Retornar, ubicacion);
            }
            NodoSentencia::Intentar {
                bloque,
                captura,
                finalmente,
            } => {
                let ranuras = self.locales.len() as u16;
                let entrada = trozo.emitir(Instruccion::EntrarIntentar(0, ranuras), ubicacion);
                self.generar_bloque(trozo, bloque, ubicacion)?;
                trozo.emitir(Instruccion::SalirIntentar, ubicacion);
                let salto_sin_error = trozo.emitir(Instruccion::Saltar(0), ubicacion);

                // Manejador: la VM deja el valor de la excepción en la pila.
                trozo.parchar_salto(entrada, trozo.posicion_actual());
                match captura {
                    Some(captura) => {
                        self.abrir_ambito();
                        let ranura = self.agregar_local(&captura.nombre, TipoRapido::Otro);
                        trozo.emitir(Instruccion::GuardarLocal(ranura), ubicacion);
                        self.generar_bloque_sin_ambito(trozo, &captura.bloque)?;
                        self.cerrar_ambito(trozo, ubicacion);
                    }
                    None => {
                        // Sin `capturar`: ejecutar `finalmente` y relanzar.
                        if let Some(bloque_finalmente) = finalmente {
                            self.generar_bloque(trozo, bloque_finalmente, ubicacion)?;
                        }
                        trozo.emitir(Instruccion::Lanzar, ubicacion);
                    }
                }

                trozo.parchar_salto(salto_sin_error, trozo.posicion_actual());
                if captura.is_some()
                    && let Some(bloque_finalmente) = finalmente
                {
                    self.generar_bloque(trozo, bloque_finalmente, ubicacion)?;
                }
                if captura.is_none() && finalmente.is_some() {
                    // El camino sin error también ejecuta `finalmente`.
                    if let Some(bloque_finalmente) = finalmente {
                        self.generar_bloque(trozo, bloque_finalmente, ubicacion)?;
                    }
                }
            }
            NodoSentencia::Lanzar(expresion) => {
                self.generar_expresion(trozo, expresion)?;
                trozo.emitir(Instruccion::Lanzar, ubicacion);
            }
        }
        Ok(())
    }

    /// Cierra el contexto de bucle parchando `romper` → fin y `continuar` → destino.
    fn cerrar_bucle(&mut self, trozo: &mut Trozo, fin: usize, destino_continuar: usize) {
        if let Some(bucle) = self.bucles.pop() {
            for salto in bucle.rupturas {
                trozo.parchar_salto(salto, fin);
            }
            for salto in bucle.continuaciones {
                trozo.parchar_salto(salto, destino_continuar);
            }
        }
    }

    fn generar_asignacion(
        &mut self,
        trozo: &mut Trozo,
        objetivo: &Expresion,
        operador: OperadorAsignacion,
        valor: &Expresion,
    ) -> Result<(), ErrorQuetzal> {
        let ubicacion = objetivo.ubicacion;
        let compuesto = operador != OperadorAsignacion::Asignar;

        match &objetivo.nodo {
            NodoExpresion::Identificador(nombre) => {
                if compuesto {
                    let tipo_objetivo = self.emitir_carga(trozo, nombre, ubicacion);
                    let tipo_valor = self.generar_expresion(trozo, valor)?;
                    trozo.emitir(
                        instruccion_binaria(
                            operador_a_binario(operador),
                            tipo_objetivo,
                            tipo_valor,
                        ),
                        ubicacion,
                    );
                } else {
                    self.generar_expresion(trozo, valor)?;
                }
                self.emitir_guarda(trozo, nombre, ubicacion);
            }
            NodoExpresion::AccesoMiembro { objeto, miembro } => {
                let indice = self.indice_nombre(miembro);
                self.generar_expresion(trozo, objeto)?;
                if compuesto {
                    trozo.emitir(Instruccion::Duplicar, ubicacion);
                    trozo.emitir(Instruccion::CargarMiembro(indice), ubicacion);
                    self.generar_expresion(trozo, valor)?;
                    trozo.emitir(instruccion_de_asignacion(operador), ubicacion);
                } else {
                    self.generar_expresion(trozo, valor)?;
                }
                trozo.emitir(Instruccion::GuardarMiembro(indice), ubicacion);
            }
            NodoExpresion::Indexacion { objeto, indice } => {
                self.generar_expresion(trozo, objeto)?;
                self.generar_expresion(trozo, indice)?;
                if compuesto {
                    return Err(ErrorQuetzal::nuevo(
                        "E0104",
                        CategoriaError::Sintactico,
                        "la asignación compuesta sobre índices no está soportada todavía",
                    )
                    .con_ubicacion(ubicacion)
                    .con_archivo(&self.archivo));
                }
                self.generar_expresion(trozo, valor)?;
                trozo.emitir(Instruccion::GuardarIndice, ubicacion);
            }
            _ => {
                return Err(ErrorQuetzal::nuevo(
                    "E0104",
                    CategoriaError::Sintactico,
                    "el objetivo de la asignación no es válido",
                )
                .con_ubicacion(ubicacion)
                .con_archivo(&self.archivo));
            }
        }
        Ok(())
    }

    // ----- Expresiones -----

    fn generar_expresion(
        &mut self,
        trozo: &mut Trozo,
        expresion: &Expresion,
    ) -> Result<TipoRapido, ErrorQuetzal> {
        let ubicacion = expresion.ubicacion;
        match &expresion.nodo {
            NodoExpresion::LiteralEntero(valor) => {
                let indice = self.indice_constante(Constante::Entero(*valor));
                trozo.emitir(Instruccion::CargarConstante(indice), ubicacion);
                Ok(TipoRapido::Entero)
            }
            NodoExpresion::LiteralNumero(texto) => {
                let decimal = Decimal::from_str(texto).map_err(|_| {
                    ErrorQuetzal::nuevo(
                        "E0003",
                        CategoriaError::Lexico,
                        format!("el número '{texto}' no es válido"),
                    )
                    .con_ubicacion(ubicacion)
                    .con_archivo(&self.archivo)
                })?;
                let indice = self.indice_constante(Constante::Numero(decimal));
                trozo.emitir(Instruccion::CargarConstante(indice), ubicacion);
                Ok(TipoRapido::Otro)
            }
            NodoExpresion::LiteralTexto(texto) => {
                let indice = self.indice_constante(Constante::Texto(texto.clone()));
                trozo.emitir(Instruccion::CargarConstante(indice), ubicacion);
                Ok(TipoRapido::Otro)
            }
            NodoExpresion::LiteralLog(valor) => {
                let indice = self.indice_constante(Constante::Log(*valor));
                trozo.emitir(Instruccion::CargarConstante(indice), ubicacion);
                Ok(TipoRapido::Otro)
            }
            NodoExpresion::Nulo => {
                self.emitir_nulo(trozo, ubicacion);
                Ok(TipoRapido::Otro)
            }
            NodoExpresion::TextoInterpolado(segmentos) => {
                for segmento in segmentos {
                    match segmento {
                        SegmentoInterpolado::Texto(texto) => {
                            let indice = self.indice_constante(Constante::Texto(texto.clone()));
                            trozo.emitir(Instruccion::CargarConstante(indice), ubicacion);
                        }
                        SegmentoInterpolado::Expresion(interior) => {
                            self.generar_expresion(trozo, interior)?;
                        }
                    }
                }
                trozo.emitir(Instruccion::Interpolar(segmentos.len() as u32), ubicacion);
                Ok(TipoRapido::Otro)
            }
            NodoExpresion::ListaLiteral(elementos) => {
                for elemento in elementos {
                    self.generar_expresion(trozo, elemento)?;
                }
                trozo.emitir(Instruccion::CrearLista(elementos.len() as u32), ubicacion);
                Ok(TipoRapido::Otro)
            }
            NodoExpresion::JsnLiteral(entradas) => {
                for (clave, valor) in entradas {
                    let texto_clave = match clave {
                        ClaveJsn::Identificador(texto) | ClaveJsn::Texto(texto) => texto.clone(),
                    };
                    let indice = self.indice_constante(Constante::Texto(texto_clave));
                    trozo.emitir(Instruccion::CargarConstante(indice), ubicacion);
                    self.generar_expresion(trozo, valor)?;
                }
                trozo.emitir(Instruccion::CrearJsn(entradas.len() as u32), ubicacion);
                Ok(TipoRapido::Otro)
            }
            NodoExpresion::Identificador(nombre) => Ok(self.emitir_carga(trozo, nombre, ubicacion)),
            NodoExpresion::Esto => {
                trozo.emitir(Instruccion::CargarEsto, ubicacion);
                Ok(TipoRapido::Otro)
            }
            NodoExpresion::Padre => {
                trozo.emitir(Instruccion::CargarPadre, ubicacion);
                Ok(TipoRapido::Otro)
            }
            NodoExpresion::Binaria {
                operador,
                izquierda,
                derecha,
            } => match operador {
                OperadorBinario::Y => {
                    self.generar_expresion(trozo, izquierda)?;
                    let salto = trozo.emitir(Instruccion::SaltarSiFalsoYDejar(0), ubicacion);
                    self.generar_expresion(trozo, derecha)?;
                    trozo.parchar_salto(salto, trozo.posicion_actual());
                    Ok(TipoRapido::Otro)
                }
                OperadorBinario::O => {
                    self.generar_expresion(trozo, izquierda)?;
                    let salto = trozo.emitir(Instruccion::SaltarSiVerdaderoYDejar(0), ubicacion);
                    self.generar_expresion(trozo, derecha)?;
                    trozo.parchar_salto(salto, trozo.posicion_actual());
                    Ok(TipoRapido::Otro)
                }
                _ => {
                    let tipo_izquierda = self.generar_expresion(trozo, izquierda)?;
                    let tipo_derecha = self.generar_expresion(trozo, derecha)?;
                    let instruccion = instruccion_binaria(*operador, tipo_izquierda, tipo_derecha);
                    trozo.emitir(instruccion, ubicacion);
                    Ok(tipo_resultado(*operador, tipo_izquierda, tipo_derecha))
                }
            },
            NodoExpresion::Unaria { operador, operando } => {
                let tipo = self.generar_expresion(trozo, operando)?;
                let instruccion = match operador {
                    OperadorUnario::Negacion if tipo == TipoRapido::Entero => {
                        Instruccion::NegarEntero
                    }
                    OperadorUnario::Negacion => Instruccion::Negar,
                    OperadorUnario::NoLogico => Instruccion::NoLogico,
                };
                trozo.emitir(instruccion, ubicacion);
                Ok(match operador {
                    OperadorUnario::Negacion => tipo,
                    OperadorUnario::NoLogico => TipoRapido::Otro,
                })
            }
            NodoExpresion::Ternaria {
                condicion,
                si_verdadero,
                si_falso,
            } => {
                self.generar_expresion(trozo, condicion)?;
                let salto_falso = trozo.emitir(Instruccion::SaltarSiFalso(0), ubicacion);
                let tipo_verdadero = self.generar_expresion(trozo, si_verdadero)?;
                let salto_fin = trozo.emitir(Instruccion::Saltar(0), ubicacion);
                trozo.parchar_salto(salto_falso, trozo.posicion_actual());
                let tipo_falso = self.generar_expresion(trozo, si_falso)?;
                trozo.parchar_salto(salto_fin, trozo.posicion_actual());
                Ok(
                    if tipo_verdadero == TipoRapido::Entero && tipo_falso == TipoRapido::Entero {
                        TipoRapido::Entero
                    } else {
                        TipoRapido::Otro
                    },
                )
            }
            NodoExpresion::Llamada {
                objetivo,
                argumentos,
            } => {
                if let NodoExpresion::AccesoMiembro { objeto, miembro } = &objetivo.nodo {
                    // Llamada de método: `objeto.metodo(args)`.
                    self.generar_expresion(trozo, objeto)?;
                    for argumento in argumentos {
                        self.generar_expresion(trozo, argumento)?;
                    }
                    let indice = self.indice_nombre(miembro);
                    trozo.emitir(
                        Instruccion::LlamarMetodo(indice, argumentos.len() as u32),
                        ubicacion,
                    );
                    Ok(TipoRapido::Otro)
                } else {
                    let tipo_retorno = match &objetivo.nodo {
                        NodoExpresion::Identificador(nombre) => match self.resolver(nombre) {
                            Destino::Global(_, tipo) => tipo,
                            _ => TipoRapido::Otro,
                        },
                        _ => TipoRapido::Otro,
                    };
                    self.generar_expresion(trozo, objetivo)?;
                    for argumento in argumentos {
                        self.generar_expresion(trozo, argumento)?;
                    }
                    trozo.emitir(Instruccion::Llamar(argumentos.len() as u32), ubicacion);
                    Ok(tipo_retorno)
                }
            }
            NodoExpresion::AccesoMiembro { objeto, miembro } => {
                self.generar_expresion(trozo, objeto)?;
                let indice = self.indice_nombre(miembro);
                trozo.emitir(Instruccion::CargarMiembro(indice), ubicacion);
                Ok(TipoRapido::Otro)
            }
            NodoExpresion::Indexacion { objeto, indice } => {
                self.generar_expresion(trozo, objeto)?;
                self.generar_expresion(trozo, indice)?;
                trozo.emitir(Instruccion::Indexar, ubicacion);
                Ok(TipoRapido::Otro)
            }
            NodoExpresion::Nuevo { clase, argumentos } => {
                self.emitir_carga(trozo, clase, ubicacion);
                for argumento in argumentos {
                    self.generar_expresion(trozo, argumento)?;
                }
                trozo.emitir(Instruccion::Instanciar(argumentos.len() as u32), ubicacion);
                Ok(TipoRapido::Otro)
            }
            NodoExpresion::Esperar(interior) => {
                let tipo = self.generar_expresion(trozo, interior)?;
                trozo.emitir(Instruccion::Esperar, ubicacion);
                Ok(tipo)
            }
        }
    }
}

fn tipo_rapido(tipo: &Tipo) -> TipoRapido {
    match tipo {
        Tipo::Entero => TipoRapido::Entero,
        _ => TipoRapido::Otro,
    }
}

fn operador_a_binario(operador: OperadorAsignacion) -> OperadorBinario {
    match operador {
        OperadorAsignacion::Asignar => unreachable!("la asignación simple no es binaria"),
        OperadorAsignacion::Sumar => OperadorBinario::Sumar,
        OperadorAsignacion::Restar => OperadorBinario::Restar,
        OperadorAsignacion::Multiplicar => OperadorBinario::Multiplicar,
        OperadorAsignacion::Dividir => OperadorBinario::Dividir,
        OperadorAsignacion::Modulo => OperadorBinario::Modulo,
    }
}

fn instruccion_de_asignacion(operador: OperadorAsignacion) -> Instruccion {
    instruccion_binaria(
        operador_a_binario(operador),
        TipoRapido::Otro,
        TipoRapido::Otro,
    )
}

fn instruccion_binaria(
    operador: OperadorBinario,
    izquierda: TipoRapido,
    derecha: TipoRapido,
) -> Instruccion {
    let enteros = izquierda == TipoRapido::Entero && derecha == TipoRapido::Entero;
    match operador {
        OperadorBinario::Sumar if enteros => Instruccion::SumarEntero,
        OperadorBinario::Restar if enteros => Instruccion::RestarEntero,
        OperadorBinario::Multiplicar if enteros => Instruccion::MultiplicarEntero,
        OperadorBinario::Dividir if enteros => Instruccion::DividirEntero,
        OperadorBinario::Modulo if enteros => Instruccion::ModuloEntero,
        OperadorBinario::Igual if enteros => Instruccion::IgualEntero,
        OperadorBinario::Diferente if enteros => Instruccion::DiferenteEntero,
        OperadorBinario::Mayor if enteros => Instruccion::MayorEntero,
        OperadorBinario::Menor if enteros => Instruccion::MenorEntero,
        OperadorBinario::MayorOIgual if enteros => Instruccion::MayorOIgualEntero,
        OperadorBinario::MenorOIgual if enteros => Instruccion::MenorOIgualEntero,
        OperadorBinario::Sumar => Instruccion::Sumar,
        OperadorBinario::Restar => Instruccion::Restar,
        OperadorBinario::Multiplicar => Instruccion::Multiplicar,
        OperadorBinario::Dividir => Instruccion::Dividir,
        OperadorBinario::Modulo => Instruccion::Modulo,
        OperadorBinario::Igual => Instruccion::Igual,
        OperadorBinario::Diferente => Instruccion::Diferente,
        OperadorBinario::Mayor => Instruccion::Mayor,
        OperadorBinario::Menor => Instruccion::Menor,
        OperadorBinario::MayorOIgual => Instruccion::MayorOIgual,
        OperadorBinario::MenorOIgual => Instruccion::MenorOIgual,
        OperadorBinario::Y | OperadorBinario::O => unreachable!(),
    }
}

fn tipo_resultado(
    operador: OperadorBinario,
    izquierda: TipoRapido,
    derecha: TipoRapido,
) -> TipoRapido {
    match operador {
        OperadorBinario::Sumar
        | OperadorBinario::Restar
        | OperadorBinario::Multiplicar
        | OperadorBinario::Dividir
        | OperadorBinario::Modulo
            if izquierda == TipoRapido::Entero && derecha == TipoRapido::Entero =>
        {
            TipoRapido::Entero
        }
        _ => TipoRapido::Otro,
    }
}
