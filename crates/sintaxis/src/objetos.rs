//! Parseo de objetos y prototipos.

use ast::{
    ClaseMiembro, DefinicionObjeto, DefinicionPrototipo, FirmaMiembro, Funcion, MiembroObjeto,
    MiembroPrototipo, Tipo, Visibilidad,
};
use lexico::TipoToken;
use nucleo::ErrorQuetzal;

use crate::parser::Parser;

impl Parser<'_> {
    /// `objeto Nombre hereda A, B como C implementa P, Q { ... }`
    pub(crate) fn parsear_objeto(&mut self) -> Result<DefinicionObjeto, ErrorQuetzal> {
        let inicio = self.esperar(&TipoToken::Objeto, "'objeto'")?.ubicacion;
        let (nombre, _) = self.esperar_identificador("el nombre del objeto")?;

        let mut padres = Vec::new();
        let mut extiende_como = Vec::new();
        let mut prototipos = Vec::new();

        // Las cláusulas pueden aparecer en cualquier orden.
        loop {
            match self.tipo_actual() {
                TipoToken::Hereda => {
                    self.avanzar();
                    self.parsear_lista_de_nombres(&mut padres)?;
                }
                TipoToken::Como => {
                    self.avanzar();
                    self.parsear_lista_de_nombres(&mut extiende_como)?;
                }
                TipoToken::Implementa => {
                    self.avanzar();
                    self.parsear_lista_de_nombres(&mut prototipos)?;
                }
                _ => break,
            }
        }

        self.esperar(&TipoToken::LlaveIzquierda, "'{' para abrir el objeto")?;
        let mut miembros = Vec::new();
        let mut visibilidad = Visibilidad::Publico;

        while !self.comprobar(&TipoToken::LlaveDerecha) && !self.comprobar(&TipoToken::Fin) {
            // Etiquetas `publico:` / `privado:` cambian la visibilidad actual.
            if self.comprobar(&TipoToken::Publico)
                && matches!(self.siguiente().tipo, TipoToken::DosPuntos)
            {
                self.avanzar();
                self.avanzar();
                visibilidad = Visibilidad::Publico;
                continue;
            }
            if self.comprobar(&TipoToken::Privado)
                && matches!(self.siguiente().tipo, TipoToken::DosPuntos)
            {
                self.avanzar();
                self.avanzar();
                visibilidad = Visibilidad::Privado;
                continue;
            }

            miembros.push(self.parsear_miembro_de_objeto(&nombre, visibilidad)?);
        }

        self.esperar(&TipoToken::LlaveDerecha, "'}' para cerrar el objeto")?;
        Ok(DefinicionObjeto {
            nombre,
            padres,
            extiende_como,
            prototipos,
            miembros,
            ubicacion: inicio,
        })
    }

    fn parsear_lista_de_nombres(&mut self, destino: &mut Vec<String>) -> Result<(), ErrorQuetzal> {
        loop {
            let (nombre, _) = self.esperar_identificador("un nombre")?;
            destino.push(nombre);
            if !self.consumir_si(&TipoToken::Coma) {
                break;
            }
        }
        Ok(())
    }

    fn parsear_miembro_de_objeto(
        &mut self,
        nombre_objeto: &str,
        visibilidad: Visibilidad,
    ) -> Result<MiembroObjeto, ErrorQuetzal> {
        // Modificadores en cualquier orden: `libre asincrono` / `asincrono libre`.
        let mut libre = false;
        let mut asincrona = false;
        loop {
            match self.tipo_actual() {
                TipoToken::Libre => {
                    self.avanzar();
                    libre = true;
                }
                TipoToken::Asincrono => {
                    self.avanzar();
                    asincrona = true;
                }
                _ => break,
            }
        }

        // Constructor: `Usuario(texto nombre, ...) { ... }`
        if let TipoToken::Identificador(nombre) = self.tipo_actual()
            && nombre == nombre_objeto
            && matches!(self.siguiente().tipo, TipoToken::ParentesisIzquierdo)
        {
            let inicio = self.ubicacion_actual();
            let (nombre, _) = self.esperar_identificador("el nombre del constructor")?;
            let parametros = self.parsear_parametros()?;
            let cuerpo = self.parsear_bloque()?;
            return Ok(MiembroObjeto {
                visibilidad,
                libre,
                clase: ClaseMiembro::Constructor(Funcion {
                    nombre,
                    tipo_retorno: Tipo::Vacio,
                    parametros,
                    cuerpo,
                    asincrona,
                    ubicacion: inicio,
                }),
            });
        }

        let inicio = self.ubicacion_actual();
        let tipo = self.parsear_tipo()?;
        let mutable = self.consumir_si(&TipoToken::Var);
        let (nombre, ubicacion_nombre) = self.esperar_identificador("el nombre del miembro")?;

        if self.comprobar(&TipoToken::ParentesisIzquierdo) {
            // Método.
            let parametros = self.parsear_parametros()?;
            let cuerpo = self.parsear_bloque()?;
            return Ok(MiembroObjeto {
                visibilidad,
                libre,
                clase: ClaseMiembro::Metodo(Funcion {
                    nombre,
                    tipo_retorno: tipo,
                    parametros,
                    cuerpo,
                    asincrona,
                    ubicacion: inicio,
                }),
            });
        }

        // Atributo con valor inicial opcional.
        let valor_inicial = if self.consumir_si(&TipoToken::Asignar) {
            Some(self.parsear_expresion()?)
        } else {
            None
        };

        Ok(MiembroObjeto {
            visibilidad,
            libre,
            clase: ClaseMiembro::Atributo {
                tipo,
                mutable,
                nombre,
                valor_inicial,
                ubicacion: inicio.unir(ubicacion_nombre),
            },
        })
    }

    /// `prototipo Nombre { ... }`
    pub(crate) fn parsear_prototipo(&mut self) -> Result<DefinicionPrototipo, ErrorQuetzal> {
        let inicio = self
            .esperar(&TipoToken::Prototipo, "'prototipo'")?
            .ubicacion;
        let (nombre, _) = self.esperar_identificador("el nombre del prototipo")?;
        self.esperar(&TipoToken::LlaveIzquierda, "'{' para abrir el prototipo")?;

        let mut miembros = Vec::new();
        let mut visibilidad = Visibilidad::Publico;

        while !self.comprobar(&TipoToken::LlaveDerecha) && !self.comprobar(&TipoToken::Fin) {
            if self.comprobar(&TipoToken::Publico)
                && matches!(self.siguiente().tipo, TipoToken::DosPuntos)
            {
                self.avanzar();
                self.avanzar();
                visibilidad = Visibilidad::Publico;
                continue;
            }
            if self.comprobar(&TipoToken::Privado)
                && matches!(self.siguiente().tipo, TipoToken::DosPuntos)
            {
                self.avanzar();
                self.avanzar();
                visibilidad = Visibilidad::Privado;
                continue;
            }

            miembros.push(self.parsear_miembro_de_prototipo(visibilidad)?);
        }

        self.esperar(&TipoToken::LlaveDerecha, "'}' para cerrar el prototipo")?;
        Ok(DefinicionPrototipo {
            nombre,
            miembros,
            ubicacion: inicio,
        })
    }

    /// Miembro de prototipo: `tipo [opcional] [var] nombre [(parametros)]`.
    fn parsear_miembro_de_prototipo(
        &mut self,
        visibilidad: Visibilidad,
    ) -> Result<MiembroPrototipo, ErrorQuetzal> {
        let inicio = self.ubicacion_actual();
        let tipo = self.parsear_tipo()?;
        let opcional = self.consumir_si(&TipoToken::Opcional);
        let mutable = self.consumir_si(&TipoToken::Var);
        let (nombre, ubicacion_nombre) = self.esperar_identificador("el nombre del miembro")?;

        let firma = if self.comprobar(&TipoToken::ParentesisIzquierdo) {
            let parametros = self.parsear_parametros()?;
            FirmaMiembro::Metodo {
                tipo_retorno: tipo,
                nombre,
                parametros,
                ubicacion: inicio.unir(self.ubicacion_anterior()),
            }
        } else {
            FirmaMiembro::Atributo {
                tipo,
                mutable,
                nombre,
                ubicacion: inicio.unir(ubicacion_nombre),
            }
        };

        Ok(MiembroPrototipo {
            visibilidad,
            opcional,
            firma,
        })
    }
}
