// Módulo de manejo de errores para el intérprete Quetzal
// Define todos los tipos de errores que puede generar el intérprete

use colored::Colorize;
use thiserror::Error;

/// Tipos de errores del intérprete Quetzal
#[derive(Error, Debug, Clone)]
pub enum ErrorQuetzal {
    #[error("Error de sintaxis en línea {linea}: {mensaje}")]
    ErrorSintaxis { linea: usize, mensaje: String },

    #[error("Error de tipo en línea {linea}: {mensaje}")]
    ErrorTipo { linea: usize, mensaje: String },

    #[error("Error de ejecución en línea {linea}: {mensaje}")]
    ErrorEjecucion { linea: usize, mensaje: String },

    #[error("Variable no definida en línea {linea}: '{nombre}'")]
    VariableNoDefinida { linea: usize, nombre: String },

    #[error("Función no definida en línea {linea}: '{nombre}'")]
    FuncionNoDefinida { linea: usize, nombre: String },

    #[error("Intento de asignación a variable inmutable en línea {linea}: '{nombre}'")]
    #[allow(dead_code)]
    VariableInmutable { linea: usize, nombre: String },

    #[error("División por cero en línea {linea}")]
    DivisionPorCero { linea: usize },

    #[error(
        "Índice fuera de rango en línea {linea}: índice {indice} en lista de tamaño {tamanio}"
    )]
    #[allow(dead_code)]
    IndiceFueraDeRango {
        linea: usize,
        indice: usize,
        tamanio: usize,
    },

    #[error("Conversión de tipo inválida en línea {linea}: no se puede convertir {tipo_origen} a {tipo_destino}")]
    #[allow(dead_code)]
    ConversionInvalida {
        linea: usize,
        tipo_origen: String,
        tipo_destino: String,
    },

    #[error("Error de conversión en línea {linea}: {mensaje}")]
    ErrorConversion { linea: usize, mensaje: String },

    #[error("Número incorrecto de argumentos en línea {linea}: se esperaban {esperados}, se recibieron {recibidos}")]
    ArgumentosIncorrectos {
        linea: usize,
        esperados: usize,
        recibidos: usize,
    },

    #[error("Token inesperado en línea {linea}: '{token}'")]
    TokenInesperado { linea: usize, token: String },

    #[error("Fin de archivo inesperado")]
    #[allow(dead_code)]
    FinArchivoInesperado,

    #[error("Error de importación: {mensaje}")]
    #[allow(dead_code)]
    ErrorImportacion { mensaje: String },

    #[error("Módulo no encontrado en línea {linea}: '{ruta}' - {detalle}")]
    ModuloNoEncontrado {
        linea: usize,
        ruta: String,
        detalle: String,
    },

    #[error("Error al cargar módulo en línea {linea}: '{ruta}' - {detalle}")]
    ErrorCargaModulo {
        linea: usize,
        ruta: String,
        detalle: String,
    },

    #[error("Elemento no exportado en línea {linea}: '{elemento}' no está disponible en el módulo '{modulo}' - {sugerencia}")]
    ElementoNoExportado {
        linea: usize,
        elemento: String,
        modulo: String,
        sugerencia: String,
    },

    #[error("Error de exportación en línea {linea}: '{elemento}' no está definido en el contexto actual - {sugerencia}")]
    ErrorExportacion {
        linea: usize,
        elemento: String,
        sugerencia: String,
    },

    #[error("Ruta de módulo inválida en línea {linea}: '{ruta}' - {razon}")]
    RutaModuloInvalida {
        linea: usize,
        ruta: String,
        razon: String,
    },

    #[error("Dependencia circular detectada en línea {linea}: módulo '{modulo}' - {cadena}")]
    DependenciaCircular {
        linea: usize,
        modulo: String,
        cadena: String,
    },

    #[error("Error interno del intérprete: {mensaje}")]
    #[allow(dead_code)]
    ErrorInterno { mensaje: String },
}

impl ErrorQuetzal {
    /// Muestra el error con formato estilo Rust en la consola
    pub fn mostrar_error(&self) {
        self.mostrar_error_con_codigo(None, None);
    }

    /// Muestra el error con formato estilo Rust incluyendo el código fuente
    pub fn mostrar_error_con_codigo(
        &self,
        codigo_fuente: Option<&str>,
        nombre_archivo: Option<&str>,
    ) {
        match self {
            ErrorQuetzal::ErrorSintaxis { linea, mensaje } => {
                // Verificar si es un error de palabra reservada para usar código específico
                if mensaje.contains("palabra reservada") {
                    self.mostrar_error_formato_rust(
                        "error",
                        "E0106",
                        *linea,
                        mensaje,
                        codigo_fuente,
                        nombre_archivo,
                        Some("las palabras reservadas están predefinidas por el lenguaje"),
                        Some("usa un nombre diferente para tu variable"),
                    );
                } else {
                    self.mostrar_error_formato_rust(
                        "error",
                        "E0001",
                        *linea,
                        mensaje,
                        codigo_fuente,
                        nombre_archivo,
                        None,
                        Some("verifica la sintaxis del código"),
                    );
                }
            }
            ErrorQuetzal::ErrorTipo { linea, mensaje } => {
                self.mostrar_error_formato_rust(
                    "error",
                    "E0002",
                    *linea,
                    mensaje,
                    codigo_fuente,
                    nombre_archivo,
                    None,
                    Some("verifica que los tipos sean compatibles"),
                );
            }
            ErrorQuetzal::ErrorEjecucion { linea, mensaje } => {
                self.mostrar_error_formato_rust(
                    "error",
                    "E0003",
                    *linea,
                    mensaje,
                    codigo_fuente,
                    nombre_archivo,
                    None,
                    None,
                );
            }
            ErrorQuetzal::VariableNoDefinida { linea, nombre } => {
                let mensaje = format!("no se puede encontrar el valor `{}` en este ámbito", nombre);
                let sugerencia = Some("¿quisiste decir una variable similar?");
                self.mostrar_error_formato_rust(
                    "error",
                    "E0425",
                    *linea,
                    &mensaje,
                    codigo_fuente,
                    nombre_archivo,
                    None,
                    sugerencia,
                );
            }
            ErrorQuetzal::FuncionNoDefinida { linea, nombre } => {
                let mensaje = format!(
                    "no se puede encontrar la función `{}` en este ámbito",
                    nombre
                );
                let sugerencia = Some("verifica que la función esté definida antes de usarla");
                self.mostrar_error_formato_rust(
                    "error",
                    "E0425",
                    *linea,
                    &mensaje,
                    codigo_fuente,
                    nombre_archivo,
                    None,
                    sugerencia,
                );
            }
            ErrorQuetzal::VariableInmutable { linea, nombre } => {
                let mensaje = format!(
                    "no se puede asignar dos veces a la variable inmutable `{}`",
                    nombre
                );
                let sugerencia = Some("considera hacer la variable mutable: `var`");
                self.mostrar_error_formato_rust(
                    "error",
                    "E0384",
                    *linea,
                    &mensaje,
                    codigo_fuente,
                    nombre_archivo,
                    None,
                    sugerencia,
                );
            }
            ErrorQuetzal::DivisionPorCero { linea } => {
                let mensaje = "intento de dividir por cero";
                let sugerencia = Some("verifica que el divisor no sea cero antes de la operación");
                self.mostrar_error_formato_rust(
                    "error",
                    "E0080",
                    *linea,
                    mensaje,
                    codigo_fuente,
                    nombre_archivo,
                    None,
                    sugerencia,
                );
            }
            ErrorQuetzal::IndiceFueraDeRango {
                linea,
                indice,
                tamanio,
            } => {
                let mensaje = format!(
                    "el índice {} está fuera de los límites de una lista de longitud {}",
                    indice, tamanio
                );
                let sugerencia = Some("los índices válidos van desde 0 hasta longitud-1");
                self.mostrar_error_formato_rust(
                    "error",
                    "E0080",
                    *linea,
                    &mensaje,
                    codigo_fuente,
                    nombre_archivo,
                    None,
                    sugerencia,
                );
            }
            ErrorQuetzal::ConversionInvalida {
                linea,
                tipo_origen,
                tipo_destino,
            } => {
                let mensaje = format!(
                    "no se puede convertir el tipo `{}` al tipo `{}`",
                    tipo_origen, tipo_destino
                );
                let sugerencia = Some("verifica que la conversión sea válida");
                self.mostrar_error_formato_rust(
                    "error",
                    "E0308",
                    *linea,
                    &mensaje,
                    codigo_fuente,
                    nombre_archivo,
                    None,
                    sugerencia,
                );
            }
            ErrorQuetzal::ArgumentosIncorrectos {
                linea,
                esperados,
                recibidos,
            } => {
                let mensaje = format!(
                    "esta función toma {} argumentos pero se proporcionaron {}",
                    esperados, recibidos
                );
                let sugerencia = Some("verifica el número de argumentos en la llamada");
                self.mostrar_error_formato_rust(
                    "error",
                    "E0061",
                    *linea,
                    &mensaje,
                    codigo_fuente,
                    nombre_archivo,
                    None,
                    sugerencia,
                );
            }
            ErrorQuetzal::TokenInesperado { linea, token } => {
                let mensaje = format!("token inesperado `{}`", token);
                let sugerencia = Some("verifica la sintaxis alrededor de este token");
                self.mostrar_error_formato_rust(
                    "error",
                    "E0001",
                    *linea,
                    &mensaje,
                    codigo_fuente,
                    nombre_archivo,
                    None,
                    sugerencia,
                );
            }
            ErrorQuetzal::FinArchivoInesperado => {
                eprintln!("{}[E0001]: fin de archivo inesperado", "error".red().bold());
                eprintln!(
                    "  {} el archivo terminó de forma inesperada",
                    "=".blue().bold()
                );
            }
            ErrorQuetzal::ErrorImportacion { mensaje } => {
                eprintln!("{}[E0432]: {}", "error".red().bold(), mensaje);
            }
            ErrorQuetzal::ModuloNoEncontrado {
                linea,
                ruta,
                detalle,
            } => {
                let mensaje = format!("no se encontró el módulo `{}`", ruta);
                let nota = Some(detalle.as_str());
                let sugerencia = Some("verifica que la ruta sea correcta y que el archivo exista");
                self.mostrar_error_formato_rust(
                    "error",
                    "E0432",
                    *linea,
                    &mensaje,
                    codigo_fuente,
                    nombre_archivo,
                    nota,
                    sugerencia,
                );
            }
            ErrorQuetzal::ErrorCargaModulo {
                linea,
                ruta,
                detalle,
            } => {
                let mensaje = format!("error al cargar el módulo `{}`", ruta);
                let nota = Some(detalle.as_str());
                let sugerencia = Some("revisa la sintaxis del módulo y que no tenga errores");
                self.mostrar_error_formato_rust(
                    "error",
                    "E0432",
                    *linea,
                    &mensaje,
                    codigo_fuente,
                    nombre_archivo,
                    nota,
                    sugerencia,
                );
            }
            ErrorQuetzal::ElementoNoExportado {
                linea,
                elemento,
                modulo,
                sugerencia,
            } => {
                let mensaje = format!("no se encontró `{}` en el módulo `{}`", elemento, modulo);
                let nota = Some(sugerencia.as_str());
                let ayuda = Some("verifica las declaraciones `exportar` en el módulo");
                self.mostrar_error_formato_rust(
                    "error",
                    "E0432",
                    *linea,
                    &mensaje,
                    codigo_fuente,
                    nombre_archivo,
                    nota,
                    ayuda,
                );
            }
            ErrorQuetzal::ErrorExportacion {
                linea,
                elemento,
                sugerencia,
            } => {
                let mensaje = format!(
                    "no se puede exportar `{}` porque no está definido",
                    elemento
                );
                let nota = Some(sugerencia.as_str());
                let ayuda = Some("define la variable antes de exportarla");
                self.mostrar_error_formato_rust(
                    "error",
                    "E0425",
                    *linea,
                    &mensaje,
                    codigo_fuente,
                    nombre_archivo,
                    nota,
                    ayuda,
                );
            }
            ErrorQuetzal::RutaModuloInvalida { linea, ruta, razon } => {
                let mensaje = format!("ruta de módulo inválida `{}`", ruta);
                let nota = Some(razon.as_str());
                let sugerencia = Some(
                    "usa rutas relativas desde el archivo principal o rutas absolutas válidas",
                );
                self.mostrar_error_formato_rust(
                    "error",
                    "E0432",
                    *linea,
                    &mensaje,
                    codigo_fuente,
                    nombre_archivo,
                    nota,
                    sugerencia,
                );
            }
            ErrorQuetzal::DependenciaCircular {
                linea,
                modulo,
                cadena,
            } => {
                let mensaje = format!("dependencia circular detectada en el módulo `{}`", modulo);
                let nota = Some(cadena.as_str());
                let sugerencia =
                    Some("reorganiza los módulos para evitar importaciones circulares");
                self.mostrar_error_formato_rust(
                    "error",
                    "E0369",
                    *linea,
                    &mensaje,
                    codigo_fuente,
                    nombre_archivo,
                    nota,
                    sugerencia,
                );
            }
            ErrorQuetzal::ErrorInterno { mensaje } => {
                eprintln!(
                    "{}[E0999]: error interno del intérprete",
                    "error".red().bold()
                );
                eprintln!("  {} {}", "=".blue().bold(), mensaje);
                eprintln!(
                    "  {} esto es un bug en el intérprete, por favor reporta este error",
                    "=".blue().bold()
                );
            }
            ErrorQuetzal::ErrorConversion { linea, mensaje } => {
                let sugerencia =
                    Some("verifica que el valor se pueda convertir al tipo solicitado");
                self.mostrar_error_formato_rust(
                    "error",
                    "E0308",
                    *linea,
                    mensaje,
                    codigo_fuente,
                    nombre_archivo,
                    None,
                    sugerencia,
                );
            }
        }
    }

    /// Formatea y muestra un error al estilo de Rust
    fn mostrar_error_formato_rust(
        &self,
        tipo: &str,
        codigo: &str,
        linea: usize,
        mensaje: &str,
        codigo_fuente: Option<&str>,
        nombre_archivo: Option<&str>,
        nota: Option<&str>,
        sugerencia: Option<&str>,
    ) {
        // Encabezado del error (similar a Rust)
        eprintln!("{}[{}]: {}", tipo.red().bold(), codigo.white(), mensaje);

        // Mostrar ubicación del archivo
        let archivo = nombre_archivo.unwrap_or("archivo.qz");
        eprintln!(" {} {}:{}:1", "-->".blue().bold(), archivo, linea);

        // Mostrar código fuente si está disponible
        if let Some(codigo) = codigo_fuente {
            self.mostrar_codigo_con_indicador(codigo, linea);
        } else {
            // Mostrar indicador simple si no hay código fuente
            eprintln!("  {}", "|".blue().bold());
            eprintln!(
                "{} {} {}",
                format!("{:3}", linea).blue().bold(),
                "|".blue().bold(),
                "código no disponible".black()
            );
            eprintln!("  {}", "|".blue().bold());
        }

        // Mostrar nota si existe
        if let Some(nota_texto) = nota {
            eprintln!(
                "  {} {}: {}",
                "=".blue().bold(),
                "nota".white().bold(),
                nota_texto
            );
        }

        // Mostrar sugerencia si existe
        if let Some(sugerencia_texto) = sugerencia {
            eprintln!(
                "  {} {}: {}",
                "=".blue().bold(),
                "ayuda".green().bold(),
                sugerencia_texto
            );
        }

        eprintln!(); // Línea en blanco al final
    }

    /// Muestra el código fuente con indicadores visuales
    fn mostrar_codigo_con_indicador(&self, codigo: &str, linea_error: usize) {
        let lineas: Vec<&str> = codigo.lines().collect();
        let inicio = linea_error.saturating_sub(2);
        let fin = (linea_error + 2).min(lineas.len());

        eprintln!("  {}", "|".blue().bold());

        for (i, linea_contenido) in lineas.iter().enumerate().skip(inicio).take(fin - inicio) {
            let numero_linea = i + 1;

            if numero_linea == linea_error {
                // Línea con error
                eprintln!(
                    "{} {} {}",
                    format!("{:3}", numero_linea).red().bold(),
                    "|".blue().bold(),
                    linea_contenido
                );
                // Indicador de error
                eprintln!(
                    "  {} {}",
                    "|".blue().bold(),
                    format!(
                        "{}^ aquí está el error",
                        " ".repeat(self.calcular_posicion_error(linea_contenido))
                    )
                    .red()
                    .bold()
                );
            } else {
                // Líneas de contexto
                eprintln!(
                    "{} {} {}",
                    format!("{:3}", numero_linea).blue().bold(),
                    "|".blue().bold(),
                    linea_contenido.bright_black()
                );
            }
        }

        eprintln!("  {}", "|".blue().bold());
    }

    /// Calcula la posición aproximada del error en la línea
    fn calcular_posicion_error(&self, linea: &str) -> usize {
        // Posición simplificada - en una implementación completa esto vendría del parser
        linea.len().saturating_sub(linea.trim_end().len()).max(0)
    }
}

/// Tipo de resultado estándar para el intérprete
pub type ResultadoQuetzal<T> = Result<T, ErrorQuetzal>;
