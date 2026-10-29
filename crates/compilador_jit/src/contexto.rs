use crate::{ConfiguracionJit, ErrorJit, EstadisticasJit, FuncionNativa};

pub struct ContextoJit {
    configuracion: ConfiguracionJit,
    estadisticas: EstadisticasJit,
}

impl ContextoJit {
    pub fn nuevo(configuracion: ConfiguracionJit) -> Self {
        Self {
            configuracion,
            estadisticas: EstadisticasJit::default(),
        }
    }

    pub fn configuracion(&self) -> ConfiguracionJit {
        self.configuracion
    }

    pub fn estadisticas(&self) -> EstadisticasJit {
        self.estadisticas
    }

    pub fn registrar_fallback(&mut self) {
        self.estadisticas.fallbacks = self.estadisticas.fallbacks.saturating_add(1);
    }

    pub fn deberia_anticipar(&self, funcion: &bytecode::FuncionCompilada) -> bool {
        #[cfg(feature = "nativo")]
        {
            if !self.configuracion.anticipacion {
                return false;
            }
            let cantidad = funcion.trozo.instrucciones.len();
            if cantidad == 0 || cantidad > self.configuracion.max_instrucciones_anticipacion {
                return false;
            }
            let costo =
                cantidad.saturating_add((funcion.trozo.max_pila as usize).saturating_mul(2));
            if costo > self.configuracion.max_costo_estimado_anticipacion {
                return false;
            }
            if self.estadisticas.funciones_compiladas > 0 {
                let promedio = self.estadisticas.tiempo_compilacion
                    / self.estadisticas.funciones_compiladas as u32;
                if promedio > self.configuracion.max_tiempo_compilacion_anticipacion {
                    return false;
                }
            }
            funcion
                .trozo
                .instrucciones
                .iter()
                .enumerate()
                .any(|(pc, instruccion)| {
                    instruccion
                        .destino_salto()
                        .is_some_and(|destino| destino < pc as u32)
                })
        }
        #[cfg(not(feature = "nativo"))]
        {
            let _ = funcion;
            false
        }
    }

    pub fn compilar_funcion(
        &mut self,
        modulo: &bytecode::ModuloCompilado,
        funcion: &bytecode::FuncionCompilada,
    ) -> Result<FuncionNativa, ErrorJit> {
        #[cfg(feature = "nativo")]
        {
            bytecode::verificar_funcion_inmutable(modulo, funcion)
                .map_err(|error| ErrorJit::Bytecode(error.to_string()))?;
            if self.estadisticas.funciones_compiladas >= self.configuracion.max_funciones {
                return Err(ErrorJit::Limite("cantidad de funciones compiladas"));
            }
            let inicio = std::time::Instant::now();
            let resultado = crate::traductor::compilar(modulo, funcion);
            self.estadisticas.tiempo_compilacion += inicio.elapsed();
            let (funcion, bytes) = resultado?;
            let total = self.estadisticas.bytes_codigo.saturating_add(bytes);
            if total > self.configuracion.max_bytes_codigo {
                return Err(ErrorJit::Limite("memoria de código generado"));
            }
            self.estadisticas.funciones_compiladas += 1;
            self.estadisticas.bytes_codigo = total;
            Ok(funcion)
        }
        #[cfg(not(feature = "nativo"))]
        {
            let _ = (modulo, funcion);
            Err(ErrorJit::NoDisponible)
        }
    }

    pub fn compilar_trivial(&mut self, valor: i64) -> Result<FuncionNativa, ErrorJit> {
        #[cfg(feature = "nativo")]
        {
            if self.estadisticas.funciones_compiladas >= self.configuracion.max_funciones {
                return Err(ErrorJit::Limite("cantidad de funciones compiladas"));
            }
            let (funcion, bytes) = crate::generador::compilar_trivial(valor)?;
            let total = self.estadisticas.bytes_codigo.saturating_add(bytes);
            if total > self.configuracion.max_bytes_codigo {
                return Err(ErrorJit::Limite("memoria de código generado"));
            }
            self.estadisticas.funciones_compiladas += 1;
            self.estadisticas.bytes_codigo = total;
            Ok(funcion)
        }
        #[cfg(not(feature = "nativo"))]
        {
            let _ = valor;
            Err(ErrorJit::NoDisponible)
        }
    }
}

impl Default for ContextoJit {
    fn default() -> Self {
        Self::nuevo(ConfiguracionJit::default())
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[cfg(feature = "nativo")]
    #[test]
    fn trivial_deberia_ejecutar_y_liberarse() {
        let mut contexto = ContextoJit::default();
        let funcion = contexto.compilar_trivial(42).expect("código trivial");
        assert_eq!(funcion.ejecutar_trivial(&[]), Ok(42));
        assert_eq!(
            funcion.ejecutar_trivial(&[1]),
            Err(ErrorJit::Aridad {
                esperados: 0,
                recibidos: 1,
            })
        );
        assert_eq!(contexto.estadisticas().funciones_compiladas, 1);
        drop(funcion);

        let mut limitado = ContextoJit::nuevo(ConfiguracionJit {
            max_funciones: 0,
            ..ConfiguracionJit::default()
        });
        assert!(matches!(
            limitado.compilar_trivial(1),
            Err(ErrorJit::Limite("cantidad de funciones compiladas"))
        ));
    }

    #[cfg(feature = "nativo")]
    fn funcion_manual(
        instrucciones: Vec<bytecode::Instruccion>,
        ranuras: u16,
    ) -> (bytecode::ModuloCompilado, bytecode::FuncionCompilada) {
        let ubicaciones = vec![nucleo::Ubicacion::default(); instrucciones.len()];
        (
            bytecode::ModuloCompilado::default(),
            bytecode::FuncionCompilada {
                nombre: "prueba".to_string(),
                parametros: vec![("n".to_string(), false)],
                trozo: bytecode::Trozo {
                    instrucciones,
                    ubicaciones,
                    max_pila: 0,
                },
                asincrona: false,
                ranuras_locales: ranuras,
            },
        )
    }

    #[cfg(feature = "nativo")]
    #[test]
    fn baseline_deberia_ejecutar_bucles_y_retornos() {
        use bytecode::Instruccion;

        let mut contexto = ContextoJit::default();
        let (modulo, identidad) = funcion_manual(vec![Instruccion::RetornarLocal(0)], 1);
        let funcion = contexto
            .compilar_funcion(&modulo, &identidad)
            .expect("retorno compilable");
        let salida = funcion.ejecutar(&[42]).expect("ejecución nativa");
        assert_eq!(salida.valor, 42);
        assert_eq!(salida.estado, crate::ESTADO_EXITO);

        let (modulo, bucle) = funcion_manual(
            vec![
                Instruccion::CargarEntero(0),
                Instruccion::GuardarLocal(1),
                Instruccion::CargarEntero(0),
                Instruccion::GuardarLocal(2),
                Instruccion::CargarLocal(2),
                Instruccion::CargarLocal(0),
                Instruccion::MenorEnteroSaltarSiFalso(10),
                Instruccion::AjustarLocalConLocal(1, 2),
                Instruccion::AjustarLocal(2, 1),
                Instruccion::Saltar(4),
                Instruccion::RetornarLocal(1),
            ],
            3,
        );
        let funcion = contexto
            .compilar_funcion(&modulo, &bucle)
            .expect("bucle compilable");
        let salida = funcion.ejecutar(&[10_000]).expect("bucle nativo");
        assert_eq!(salida.valor, 49_995_000);
        assert_eq!(salida.estado, crate::ESTADO_EXITO);
    }

    #[cfg(feature = "nativo")]
    #[test]
    fn baseline_deberia_conservar_errores_numericos() {
        use bytecode::Instruccion;

        let mut contexto = ContextoJit::default();
        let (modulo, suma) = funcion_manual(
            vec![
                Instruccion::CargarLocal(0),
                Instruccion::CargarEntero(1),
                Instruccion::SumarEntero(),
                Instruccion::Retornar(),
            ],
            1,
        );
        let funcion = contexto
            .compilar_funcion(&modulo, &suma)
            .expect("suma compilable");
        let salida = funcion.ejecutar(&[i64::MAX]).expect("error de lenguaje");
        assert_eq!(salida.estado, crate::ESTADO_DESBORDAMIENTO);
        assert_eq!(salida.pc, 2);

        let (modulo, division) = funcion_manual(
            vec![
                Instruccion::CargarLocal(0),
                Instruccion::CargarEntero(0),
                Instruccion::DividirEntero(),
                Instruccion::Retornar(),
            ],
            1,
        );
        let funcion = contexto
            .compilar_funcion(&modulo, &division)
            .expect("división compilable");
        let salida = funcion.ejecutar(&[10]).expect("error de lenguaje");
        assert_eq!(salida.estado, crate::ESTADO_DIVISION_CERO);
        assert_eq!(salida.pc, 2);

        let (modulo, division_minima) = funcion_manual(
            vec![
                Instruccion::CargarLocal(0),
                Instruccion::CargarEntero(-1),
                Instruccion::DividirEntero(),
                Instruccion::Retornar(),
            ],
            1,
        );
        let funcion = contexto
            .compilar_funcion(&modulo, &division_minima)
            .expect("división compilable");
        let salida = funcion.ejecutar(&[i64::MIN]).expect("error de lenguaje");
        assert_eq!(salida.estado, crate::ESTADO_DESBORDAMIENTO);
        assert_eq!(salida.pc, 2);
    }

    #[cfg(feature = "nativo")]
    #[test]
    fn baseline_deberia_ejecutar_condicionales() {
        use bytecode::Instruccion;

        let mut contexto = ContextoJit::default();
        let (modulo, condicional) = funcion_manual(
            vec![
                Instruccion::CargarLocal(0),
                Instruccion::CargarEntero(0),
                Instruccion::IgualEntero(),
                Instruccion::SaltarSiFalso(6),
                Instruccion::CargarEntero(7),
                Instruccion::Retornar(),
                Instruccion::CargarEntero(9),
                Instruccion::Retornar(),
            ],
            1,
        );
        let funcion = contexto
            .compilar_funcion(&modulo, &condicional)
            .expect("condicional compilable");
        assert_eq!(funcion.ejecutar(&[0]).expect("rama verdadera").valor, 7);
        assert_eq!(funcion.ejecutar(&[1]).expect("rama falsa").valor, 9);
    }

    #[cfg(feature = "nativo")]
    #[test]
    fn baseline_deberia_coincidir_con_operaciones_checked() {
        use bytecode::{CodigoOp, Instruccion};

        fn esperado(op: CodigoOp, a: i64, b: i64) -> Option<i64> {
            match op {
                CodigoOp::SumarEntero => a.checked_add(b),
                CodigoOp::RestarEntero => a.checked_sub(b),
                CodigoOp::MultiplicarEntero => a.checked_mul(b),
                CodigoOp::DividirEntero => a.checked_div(b),
                CodigoOp::ModuloEntero => a.checked_rem(b),
                _ => unreachable!(),
            }
        }

        let operaciones = [
            CodigoOp::SumarEntero,
            CodigoOp::RestarEntero,
            CodigoOp::MultiplicarEntero,
            CodigoOp::DividirEntero,
            CodigoOp::ModuloEntero,
        ];
        let valores = [0, 1, 2, -5, i64::MAX, i64::MIN];
        let mut contexto = ContextoJit::default();
        for op in operaciones {
            for a in valores {
                for b in [0, 1, -1, 3] {
                    let instruccion = match op {
                        CodigoOp::SumarEntero => Instruccion::SumarEntero(),
                        CodigoOp::RestarEntero => Instruccion::RestarEntero(),
                        CodigoOp::MultiplicarEntero => Instruccion::MultiplicarEntero(),
                        CodigoOp::DividirEntero => Instruccion::DividirEntero(),
                        CodigoOp::ModuloEntero => Instruccion::ModuloEntero(),
                        _ => unreachable!(),
                    };
                    let (modulo, funcion) = funcion_manual(
                        vec![
                            Instruccion::CargarLocal(0),
                            Instruccion::CargarEntero(b),
                            instruccion,
                            Instruccion::Retornar(),
                        ],
                        1,
                    );
                    let nativa = contexto
                        .compilar_funcion(&modulo, &funcion)
                        .expect("operación compilable");
                    let salida = nativa.ejecutar(&[a]).expect("ejecución nativa");
                    match esperado(op, a, b as i64) {
                        Some(valor) => {
                            assert_eq!(salida.estado, crate::ESTADO_EXITO, "{op:?} {a} {b}");
                            assert_eq!(salida.valor, valor, "{op:?} {a} {b}");
                        }
                        None if b == 0
                            && matches!(op, CodigoOp::DividirEntero | CodigoOp::ModuloEntero) =>
                        {
                            assert_eq!(salida.estado, crate::ESTADO_DIVISION_CERO);
                        }
                        None => assert_eq!(salida.estado, crate::ESTADO_DESBORDAMIENTO),
                    }
                }
            }
        }
    }

    #[cfg(feature = "nativo")]
    #[test]
    fn anticipacion_deberia_exigir_bucles_y_respetar_limites() {
        use bytecode::Instruccion;

        let contexto = ContextoJit::default();
        let (_modulo, bucle) = funcion_manual(
            vec![
                Instruccion::CargarEntero(0),
                Instruccion::GuardarLocal(1),
                Instruccion::CargarLocal(1),
                Instruccion::CargarLocal(0),
                Instruccion::MenorEnteroSaltarSiFalso(7),
                Instruccion::AjustarLocal(1, 1),
                Instruccion::Saltar(2),
                Instruccion::RetornarLocal(1),
            ],
            2,
        );
        assert!(contexto.deberia_anticipar(&bucle));

        let (_modulo, identidad) = funcion_manual(vec![Instruccion::RetornarLocal(0)], 1);
        assert!(!contexto.deberia_anticipar(&identidad));

        let sin_anticipacion = ContextoJit::nuevo(ConfiguracionJit {
            anticipacion: false,
            ..ConfiguracionJit::default()
        });
        assert!(!sin_anticipacion.deberia_anticipar(&bucle));

        let limitado = ContextoJit::nuevo(ConfiguracionJit {
            max_instrucciones_anticipacion: 2,
            ..ConfiguracionJit::default()
        });
        assert!(!limitado.deberia_anticipar(&bucle));
    }

    #[cfg(not(feature = "nativo"))]
    #[test]
    fn sin_feature_deberia_informar_no_disponible() {
        let mut contexto = ContextoJit::default();
        assert!(matches!(
            contexto.compilar_trivial(42),
            Err(ErrorJit::NoDisponible)
        ));
    }
}
