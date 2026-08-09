//! Generador de bytecode: convierte el AST validado en un módulo compilado.

use std::collections::HashMap;
use std::str::FromStr;

use ast::{
    Bloque, ClaseMiembro, ClaveJsn, DefinicionObjeto, Elemento, Expresion, Funcion, Modulo,
    NodoExpresion, NodoSentencia, OperadorAsignacion, OperadorBinario, OperadorUnario,
    SegmentoInterpolado, Sentencia, Visibilidad,
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

struct Generador {
    archivo: String,
    constantes: Vec<Constante>,
    nombres: Vec<String>,
    indice_nombres: HashMap<String, u32>,
    bucles: Vec<ContextoBucle>,
}

impl Generador {
    fn nuevo(archivo: &str) -> Self {
        Self {
            archivo: archivo.to_string(),
            constantes: Vec::new(),
            nombres: Vec::new(),
            indice_nombres: HashMap::new(),
            bucles: Vec::new(),
        }
    }

    fn generar(&mut self, modulo: &Modulo) -> Result<ModuloCompilado, ErrorQuetzal> {
        let mut compilado = ModuloCompilado {
            nombre: modulo.nombre.clone(),
            ..Default::default()
        };

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
                    let funcion_compilada = self.generar_funcion(funcion)?;
                    compilado.funciones.push(funcion_compilada);
                }
                Elemento::Objeto(objeto) => {
                    let objeto_compilado = self.generar_objeto(objeto)?;
                    compilado.objetos.push(objeto_compilado);
                }
                Elemento::Prototipo(_) => {
                    // Los prototipos se validan en semántica; no generan código.
                }
                Elemento::Sentencia(sentencia) => {
                    self.generar_sentencia(&mut compilado.principal, sentencia)?;
                }
            }
        }

        compilado.constantes = std::mem::take(&mut self.constantes);
        compilado.nombres = std::mem::take(&mut self.nombres);
        Ok(compilado)
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

    // ----- Funciones y objetos -----

    fn generar_funcion(&mut self, funcion: &Funcion) -> Result<FuncionCompilada, ErrorQuetzal> {
        let mut trozo = Trozo::default();
        self.generar_bloque_sin_ambito(&mut trozo, &funcion.cuerpo)?;
        // Retorno implícito `nulo` si la ejecución llega al final.
        let indice_nulo = self.indice_constante(Constante::Nulo);
        trozo.emitir(Instruccion::CargarConstante(indice_nulo), funcion.ubicacion);
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
        trozo.emitir(Instruccion::AbrirAmbito, ubicacion);
        self.generar_bloque_sin_ambito(trozo, bloque)?;
        trozo.emitir(Instruccion::CerrarAmbito, ubicacion);
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
                mutable,
                nombre,
                valor,
                ..
            } => {
                match valor {
                    Some(expresion) => self.generar_expresion(trozo, expresion)?,
                    None => {
                        let indice = self.indice_constante(Constante::Nulo);
                        trozo.emitir(Instruccion::CargarConstante(indice), ubicacion);
                    }
                }
                let indice_nombre = self.indice_nombre(nombre);
                trozo.emitir(
                    Instruccion::DeclararVariable(indice_nombre, *mutable),
                    ubicacion,
                );
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
                trozo.emitir(Instruccion::AbrirAmbito, ubicacion);
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
                trozo.emitir(Instruccion::CerrarAmbito, ubicacion);
            }
            NodoSentencia::ParaEn {
                mutable,
                nombre,
                iterable,
                cuerpo,
                ..
            } => {
                trozo.emitir(Instruccion::AbrirAmbito, ubicacion);
                self.generar_expresion(trozo, iterable)?;
                trozo.emitir(Instruccion::CrearIterador, ubicacion);
                // El iterador queda guardado en una variable oculta.
                let indice_iterador = self.indice_nombre("__iterador");
                trozo.emitir(
                    Instruccion::DeclararVariable(indice_iterador, true),
                    ubicacion,
                );

                let inicio = trozo.posicion_actual();
                trozo.emitir(Instruccion::CargarVariable(indice_iterador), ubicacion);
                let salto_fin = trozo.emitir(Instruccion::IteradorSiguiente(0), ubicacion);

                trozo.emitir(Instruccion::AbrirAmbito, ubicacion);
                let indice_elemento = self.indice_nombre(nombre);
                trozo.emitir(
                    Instruccion::DeclararVariable(indice_elemento, *mutable),
                    ubicacion,
                );
                self.bucles.push(ContextoBucle::default());
                self.generar_bloque_sin_ambito(trozo, cuerpo)?;
                trozo.emitir(Instruccion::CerrarAmbito, ubicacion);
                trozo.emitir(Instruccion::Saltar(inicio), ubicacion);
                let fin = trozo.posicion_actual();
                trozo.parchar_salto(salto_fin, fin);
                self.cerrar_bucle(trozo, fin, inicio);
                trozo.emitir(Instruccion::CerrarAmbito, ubicacion);
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
                    Some(expresion) => self.generar_expresion(trozo, expresion)?,
                    None => {
                        let indice = self.indice_constante(Constante::Nulo);
                        trozo.emitir(Instruccion::CargarConstante(indice), ubicacion);
                    }
                }
                trozo.emitir(Instruccion::Retornar, ubicacion);
            }
            NodoSentencia::Intentar {
                bloque,
                captura,
                finalmente,
            } => {
                let entrada = trozo.emitir(Instruccion::EntrarIntentar(0), ubicacion);
                self.generar_bloque(trozo, bloque, ubicacion)?;
                trozo.emitir(Instruccion::SalirIntentar, ubicacion);
                let salto_sin_error = trozo.emitir(Instruccion::Saltar(0), ubicacion);

                // Manejador: la VM deja el valor de la excepción en la pila.
                trozo.parchar_salto(entrada, trozo.posicion_actual());
                match captura {
                    Some(captura) => {
                        trozo.emitir(Instruccion::AbrirAmbito, ubicacion);
                        let indice_nombre = self.indice_nombre(&captura.nombre);
                        trozo.emitir(
                            Instruccion::DeclararVariable(indice_nombre, false),
                            ubicacion,
                        );
                        self.generar_bloque_sin_ambito(trozo, &captura.bloque)?;
                        trozo.emitir(Instruccion::CerrarAmbito, ubicacion);
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
        let operacion = operador_compuesto(operador);

        match &objetivo.nodo {
            NodoExpresion::Identificador(nombre) => {
                let indice = self.indice_nombre(nombre);
                if let Some(instruccion) = operacion {
                    trozo.emitir(Instruccion::CargarVariable(indice), ubicacion);
                    self.generar_expresion(trozo, valor)?;
                    trozo.emitir(instruccion, ubicacion);
                } else {
                    self.generar_expresion(trozo, valor)?;
                }
                trozo.emitir(Instruccion::GuardarVariable(indice), ubicacion);
            }
            NodoExpresion::AccesoMiembro { objeto, miembro } => {
                let indice = self.indice_nombre(miembro);
                self.generar_expresion(trozo, objeto)?;
                if let Some(instruccion) = operacion {
                    trozo.emitir(Instruccion::Duplicar, ubicacion);
                    trozo.emitir(Instruccion::CargarMiembro(indice), ubicacion);
                    self.generar_expresion(trozo, valor)?;
                    trozo.emitir(instruccion, ubicacion);
                } else {
                    self.generar_expresion(trozo, valor)?;
                }
                trozo.emitir(Instruccion::GuardarMiembro(indice), ubicacion);
            }
            NodoExpresion::Indexacion { objeto, indice } => {
                self.generar_expresion(trozo, objeto)?;
                self.generar_expresion(trozo, indice)?;
                if operacion.is_some() {
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
    ) -> Result<(), ErrorQuetzal> {
        let ubicacion = expresion.ubicacion;
        match &expresion.nodo {
            NodoExpresion::LiteralEntero(valor) => {
                let indice = self.indice_constante(Constante::Entero(*valor));
                trozo.emitir(Instruccion::CargarConstante(indice), ubicacion);
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
            }
            NodoExpresion::LiteralTexto(texto) => {
                let indice = self.indice_constante(Constante::Texto(texto.clone()));
                trozo.emitir(Instruccion::CargarConstante(indice), ubicacion);
            }
            NodoExpresion::LiteralLog(valor) => {
                let indice = self.indice_constante(Constante::Log(*valor));
                trozo.emitir(Instruccion::CargarConstante(indice), ubicacion);
            }
            NodoExpresion::Nulo => {
                let indice = self.indice_constante(Constante::Nulo);
                trozo.emitir(Instruccion::CargarConstante(indice), ubicacion);
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
            }
            NodoExpresion::ListaLiteral(elementos) => {
                for elemento in elementos {
                    self.generar_expresion(trozo, elemento)?;
                }
                trozo.emitir(Instruccion::CrearLista(elementos.len() as u32), ubicacion);
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
            }
            NodoExpresion::Identificador(nombre) => {
                let indice = self.indice_nombre(nombre);
                trozo.emitir(Instruccion::CargarVariable(indice), ubicacion);
            }
            NodoExpresion::Esto => {
                trozo.emitir(Instruccion::CargarEsto, ubicacion);
            }
            NodoExpresion::Padre => {
                trozo.emitir(Instruccion::CargarPadre, ubicacion);
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
                }
                OperadorBinario::O => {
                    self.generar_expresion(trozo, izquierda)?;
                    let salto = trozo.emitir(Instruccion::SaltarSiVerdaderoYDejar(0), ubicacion);
                    self.generar_expresion(trozo, derecha)?;
                    trozo.parchar_salto(salto, trozo.posicion_actual());
                }
                _ => {
                    self.generar_expresion(trozo, izquierda)?;
                    self.generar_expresion(trozo, derecha)?;
                    let instruccion = match operador {
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
                    };
                    trozo.emitir(instruccion, ubicacion);
                }
            },
            NodoExpresion::Unaria { operador, operando } => {
                self.generar_expresion(trozo, operando)?;
                let instruccion = match operador {
                    OperadorUnario::Negacion => Instruccion::Negar,
                    OperadorUnario::NoLogico => Instruccion::NoLogico,
                };
                trozo.emitir(instruccion, ubicacion);
            }
            NodoExpresion::Ternaria {
                condicion,
                si_verdadero,
                si_falso,
            } => {
                self.generar_expresion(trozo, condicion)?;
                let salto_falso = trozo.emitir(Instruccion::SaltarSiFalso(0), ubicacion);
                self.generar_expresion(trozo, si_verdadero)?;
                let salto_fin = trozo.emitir(Instruccion::Saltar(0), ubicacion);
                trozo.parchar_salto(salto_falso, trozo.posicion_actual());
                self.generar_expresion(trozo, si_falso)?;
                trozo.parchar_salto(salto_fin, trozo.posicion_actual());
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
                } else {
                    self.generar_expresion(trozo, objetivo)?;
                    for argumento in argumentos {
                        self.generar_expresion(trozo, argumento)?;
                    }
                    trozo.emitir(Instruccion::Llamar(argumentos.len() as u32), ubicacion);
                }
            }
            NodoExpresion::AccesoMiembro { objeto, miembro } => {
                self.generar_expresion(trozo, objeto)?;
                let indice = self.indice_nombre(miembro);
                trozo.emitir(Instruccion::CargarMiembro(indice), ubicacion);
            }
            NodoExpresion::Indexacion { objeto, indice } => {
                self.generar_expresion(trozo, objeto)?;
                self.generar_expresion(trozo, indice)?;
                trozo.emitir(Instruccion::Indexar, ubicacion);
            }
            NodoExpresion::Nuevo { clase, argumentos } => {
                for argumento in argumentos {
                    self.generar_expresion(trozo, argumento)?;
                }
                let indice = self.indice_nombre(clase);
                trozo.emitir(
                    Instruccion::Instanciar(indice, argumentos.len() as u32),
                    ubicacion,
                );
            }
            NodoExpresion::Esperar(interior) => {
                self.generar_expresion(trozo, interior)?;
                trozo.emitir(Instruccion::Esperar, ubicacion);
            }
        }
        Ok(())
    }
}

fn operador_compuesto(operador: OperadorAsignacion) -> Option<Instruccion> {
    match operador {
        OperadorAsignacion::Asignar => None,
        OperadorAsignacion::Sumar => Some(Instruccion::Sumar),
        OperadorAsignacion::Restar => Some(Instruccion::Restar),
        OperadorAsignacion::Multiplicar => Some(Instruccion::Multiplicar),
        OperadorAsignacion::Dividir => Some(Instruccion::Dividir),
        OperadorAsignacion::Modulo => Some(Instruccion::Modulo),
    }
}
