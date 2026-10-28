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
