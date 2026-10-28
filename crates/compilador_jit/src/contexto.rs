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
            let (funcion, bytes) = crate::traductor::compilar(modulo, funcion)?;
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
