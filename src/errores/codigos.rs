/// Códigos de error del Lenguaje Quetzal
/// 
/// Los códigos están organizados en rangos:
/// - E0001-E0099: Errores de análisis léxico y sintáctico
/// - E0100-E0199: Errores de declaración y definición
/// - E0200-E0299: Errores de tipos de datos
/// - E0300-E0399: Errores de control de flujo
/// - E0400-E0499: Errores de módulos y dependencias
/// - E0500-E0599: Errores de objetos y métodos
/// - E0600-E0699: Errores de listas y estructuras de datos
/// - E0700-E0799: Errores de JSON y serialización
/// - E0800-E0899: Errores de funciones y llamadas
/// - E0900-E0999: Errores de ejecución y sistema

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodigoError {
    // E0001-E0099: Errores de Análisis Léxico y Sintáctico
    SintaxisGeneral,
    ComentarioSinCerrar,
    CadenaSinCerrar,
    EscapeInvalido,
    NumeroMalformado,
    CaracterInvalido,
    PalabraReservadaIncorrecta,
    IdentificadorInvalido,
    OperadorMalformado,
    DelimitadoresDesbalanceados,
    
    // E0100-E0199: Errores de Declaración y Definición
    VariableNoDeclarada,
    VariableRedeclarada,
    FuncionNoDeclarada,
    FuncionRedeclarada,
    ParametrosDuplicados,
    ParametroRedeclarado,
    TipoInvalido,
    ModificadorAccesoIncorrecto,
    DeclaracionIncompleta,
    ObjetoRedeclarado,
    AsignacionAInmutable,
    
    // E0200-E0299: Errores de Tipos de Datos
    TiposIncompatibles,
    ConversionTipoInvalida,
    AsignacionTipoIncorrecto,
    ComparacionTiposIncompatibles,
    OperacionNoSoportada,
    DesbordamientoEntero,
    DivisionPorCero,
    ModuloPorCero,
    PrecisionDecimalExcedida,
    
    // E0300-E0399: Errores de Control de Flujo
    RetornarFueraDeFuncion,
    FuncionSinRetorno,
    RetornoTipoIncorrecto,
    RomperFueraDeBucle,
    ContinuarFueraDeBucle,
    CondicionInvalida,
    BucleInfinitoSinControl,
    ExcepcionNoCapturada,
    ExcepcionLanzada,
    
    // E0400-E0499: Errores de Módulos y Dependencias
    ModuloNoEncontrado,
    ErrorCargarModulo,
    DependenciaCircular,
    ElementoNoExportado,
    AliasImportacionConflictivo,
    RutaModuloInvalida,
    ElementoImportadoNoEncontrado,
    ErrorPermisosModulo,
    
    // E0500-E0599: Errores de Objetos y Métodos
    ObjetoNoEncontrado,
    MetodoNoEncontrado,
    MiembroNoExiste,
    AccesoMiembroPrivado,
    ConstructorParametrosIncorrectos,
    InstanciacionSinConstructor,
    AmbienteFueraDeObjeto,
    HerenciaCircular,
    
    // E0600-E0699: Errores de Listas y Estructuras de Datos
    IndiceFueraDeRango,
    IndiceTipoIncorrecto,
    ListaVaciaNoPermitida,
    ModificacionListaConstante,
    TipoElementoIncompatible,
    DimensionesMatrizIncorrectas,
    AsignacionMatrizInvalida,
    MetodoNoDisponibleLista,
    
    // E0700-E0799: Errores de JSON y Serialización
    JsonMalformado,
    ClaveDuplicadaJson,
    PropiedadInexistenteJson,
    TipoIncompatibleJson,
    DecodificacionJsonFallida,
    EstructuraJsonMuyProfunda,
    
    // E0800-E0899: Errores de Funciones y Llamadas
    NumeroArgumentosIncorrecto,
    TipoArgumentoIncorrecto,
    FuncionRecursivaSinCasoBase,
    DesbordamientoPilaRecursion,
    FuncionAsincronaSinEsperar,
    FuncionAsincronaSinOperaciones,
    EsperarFueraDeAsincrona,
    TipoRetornoIncorrecto,
    
    // E0900-E0999: Errores de Ejecución y Sistema
    ErrorLecturaArchivo,
    ErrorEscrituraArchivo,
    MemoriaInsuficiente,
    TiempoEjecucionExcedido,
    InterrupcionUsuario,
    ErrorInternoInterprete,
    OperacionNoSoportadaSO,
    ViolacionSeguridad,
}

impl CodigoError {
    pub fn codigo(&self) -> &'static str {
        match self {
            // E0001-E0099
            CodigoError::SintaxisGeneral => "E0001",
            CodigoError::ComentarioSinCerrar => "E0002",
            CodigoError::CadenaSinCerrar => "E0003",
            CodigoError::EscapeInvalido => "E0004",
            CodigoError::NumeroMalformado => "E0005",
            CodigoError::CaracterInvalido => "E0006",
            CodigoError::PalabraReservadaIncorrecta => "E0007",
            CodigoError::IdentificadorInvalido => "E0008",
            CodigoError::OperadorMalformado => "E0009",
            CodigoError::DelimitadoresDesbalanceados => "E0010",
            
            // E0100-E0199
            CodigoError::VariableNoDeclarada => "E0100",
            CodigoError::VariableRedeclarada => "E0101",
            CodigoError::FuncionNoDeclarada => "E0102",
            CodigoError::FuncionRedeclarada => "E0103",
            CodigoError::ParametrosDuplicados => "E0104",
            CodigoError::ParametroRedeclarado => "E0105",
            CodigoError::TipoInvalido => "E0106",
            CodigoError::ModificadorAccesoIncorrecto => "E0107",
            CodigoError::DeclaracionIncompleta => "E0108",
            CodigoError::ObjetoRedeclarado => "E0109",
            CodigoError::AsignacionAInmutable => "E0110",
            
            // E0200-E0299
            CodigoError::TiposIncompatibles => "E0200",
            CodigoError::ConversionTipoInvalida => "E0201",
            CodigoError::AsignacionTipoIncorrecto => "E0202",
            CodigoError::ComparacionTiposIncompatibles => "E0203",
            CodigoError::OperacionNoSoportada => "E0204",
            CodigoError::DesbordamientoEntero => "E0205",
            CodigoError::DivisionPorCero => "E0206",
            CodigoError::ModuloPorCero => "E0207",
            CodigoError::PrecisionDecimalExcedida => "E0208",
            
            // E0300-E0399
            CodigoError::RetornarFueraDeFuncion => "E0300",
            CodigoError::FuncionSinRetorno => "E0301",
            CodigoError::RetornoTipoIncorrecto => "E0302",
            CodigoError::RomperFueraDeBucle => "E0303",
            CodigoError::ContinuarFueraDeBucle => "E0304",
            CodigoError::CondicionInvalida => "E0305",
            CodigoError::BucleInfinitoSinControl => "E0306",
            CodigoError::ExcepcionNoCapturada => "E0307",
            CodigoError::ExcepcionLanzada => "E0308",
            
            // E0400-E0499
            CodigoError::ModuloNoEncontrado => "E0400",
            CodigoError::ErrorCargarModulo => "E0401",
            CodigoError::DependenciaCircular => "E0402",
            CodigoError::ElementoNoExportado => "E0403",
            CodigoError::AliasImportacionConflictivo => "E0404",
            CodigoError::RutaModuloInvalida => "E0405",
            CodigoError::ElementoImportadoNoEncontrado => "E0406",
            CodigoError::ErrorPermisosModulo => "E0407",
            
            // E0500-E0599
            CodigoError::ObjetoNoEncontrado => "E0500",
            CodigoError::MetodoNoEncontrado => "E0501",
            CodigoError::MiembroNoExiste => "E0502",
            CodigoError::AccesoMiembroPrivado => "E0503",
            CodigoError::ConstructorParametrosIncorrectos => "E0504",
            CodigoError::InstanciacionSinConstructor => "E0505",
            CodigoError::AmbienteFueraDeObjeto => "E0506",
            CodigoError::HerenciaCircular => "E0507",
            
            // E0600-E0699
            CodigoError::IndiceFueraDeRango => "E0600",
            CodigoError::IndiceTipoIncorrecto => "E0601",
            CodigoError::ListaVaciaNoPermitida => "E0602",
            CodigoError::ModificacionListaConstante => "E0603",
            CodigoError::TipoElementoIncompatible => "E0604",
            CodigoError::DimensionesMatrizIncorrectas => "E0605",
            CodigoError::AsignacionMatrizInvalida => "E0606",
            CodigoError::MetodoNoDisponibleLista => "E0607",
            
            // E0700-E0799
            CodigoError::JsonMalformado => "E0700",
            CodigoError::ClaveDuplicadaJson => "E0701",
            CodigoError::PropiedadInexistenteJson => "E0702",
            CodigoError::TipoIncompatibleJson => "E0703",
            CodigoError::DecodificacionJsonFallida => "E0704",
            CodigoError::EstructuraJsonMuyProfunda => "E0705",
            
            // E0800-E0899
            CodigoError::NumeroArgumentosIncorrecto => "E0800",
            CodigoError::TipoArgumentoIncorrecto => "E0801",
            CodigoError::FuncionRecursivaSinCasoBase => "E0802",
            CodigoError::DesbordamientoPilaRecursion => "E0803",
            CodigoError::FuncionAsincronaSinEsperar => "E0804",
            CodigoError::FuncionAsincronaSinOperaciones => "E0805",
            CodigoError::EsperarFueraDeAsincrona => "E0806",
            CodigoError::TipoRetornoIncorrecto => "E0807",
            
            // E0900-E0999
            CodigoError::ErrorLecturaArchivo => "E0900",
            CodigoError::ErrorEscrituraArchivo => "E0901",
            CodigoError::MemoriaInsuficiente => "E0902",
            CodigoError::TiempoEjecucionExcedido => "E0903",
            CodigoError::InterrupcionUsuario => "E0904",
            CodigoError::ErrorInternoInterprete => "E0905",
            CodigoError::OperacionNoSoportadaSO => "E0906",
            CodigoError::ViolacionSeguridad => "E0907",
        }
    }
    
    pub fn descripcion(&self) -> &'static str {
        match self {
            // E0001-E0099
            CodigoError::SintaxisGeneral => "error de sintaxis general",
            CodigoError::ComentarioSinCerrar => "comentario sin cerrar",
            CodigoError::CadenaSinCerrar => "cadena de texto sin cerrar",
            CodigoError::EscapeInvalido => "secuencia de escape inválida",
            CodigoError::NumeroMalformado => "número malformado o inválido",
            CodigoError::CaracterInvalido => "carácter inválido o no reconocido",
            CodigoError::PalabraReservadaIncorrecta => "palabra reservada usada incorrectamente",
            CodigoError::IdentificadorInvalido => "identificador inválido",
            CodigoError::OperadorMalformado => "operador malformado",
            CodigoError::DelimitadoresDesbalanceados => "paréntesis, llaves o corchetes desbalanceados",
            
            // E0100-E0199
            CodigoError::VariableNoDeclarada => "variable no declarada o no encontrada",
            CodigoError::VariableRedeclarada => "redeclaración de variable",
            CodigoError::FuncionNoDeclarada => "función no declarada o no encontrada",
            CodigoError::FuncionRedeclarada => "redeclaración de función",
            CodigoError::ParametrosDuplicados => "parámetros duplicados en función",
            CodigoError::ParametroRedeclarado => "parámetro redeclarado en función",
            CodigoError::TipoInvalido => "tipo de datos inválido o no reconocido",
            CodigoError::ModificadorAccesoIncorrecto => "modificador de acceso incorrecto",
            CodigoError::DeclaracionIncompleta => "declaración incompleta o malformada",
            CodigoError::ObjetoRedeclarado => "redeclaración de objeto",
            CodigoError::AsignacionAInmutable => "asignación a variable inmutable",
            
            // E0200-E0299
            CodigoError::TiposIncompatibles => "tipos incompatibles en operación",
            CodigoError::ConversionTipoInvalida => "conversión de tipo inválida",
            CodigoError::AsignacionTipoIncorrecto => "asignación de tipo incorrecto",
            CodigoError::ComparacionTiposIncompatibles => "comparación entre tipos incompatibles",
            CodigoError::OperacionNoSoportada => "operación no soportada para el tipo",
            CodigoError::DesbordamientoEntero => "desbordamiento de enteros (overflow)",
            CodigoError::DivisionPorCero => "división por cero",
            CodigoError::ModuloPorCero => "módulo por cero",
            CodigoError::PrecisionDecimalExcedida => "precisión de números decimales excedida",
            
            // E0300-E0399
            CodigoError::RetornarFueraDeFuncion => "uso de 'retornar' fuera de una función",
            CodigoError::FuncionSinRetorno => "función sin valor de retorno esperado",
            CodigoError::RetornoTipoIncorrecto => "valor de retorno de tipo incorrecto",
            CodigoError::RomperFueraDeBucle => "uso de 'romper' fuera de un bucle",
            CodigoError::ContinuarFueraDeBucle => "uso de 'continuar' fuera de un bucle",
            CodigoError::CondicionInvalida => "condiciones inválidas en estructuras de control",
            CodigoError::BucleInfinitoSinControl => "bucles infinitos sin control de salida",
            CodigoError::ExcepcionNoCapturada => "excepción no capturada",
            CodigoError::ExcepcionLanzada => "excepción lanzada",
            
            // E0400-E0499
            CodigoError::ModuloNoEncontrado => "módulo no encontrado",
            CodigoError::ErrorCargarModulo => "error al cargar archivo de módulo",
            CodigoError::DependenciaCircular => "dependencia circular entre módulos",
            CodigoError::ElementoNoExportado => "elemento no exportado desde el módulo",
            CodigoError::AliasImportacionConflictivo => "alias de importación conflictivo",
            CodigoError::RutaModuloInvalida => "ruta de módulo inválida",
            CodigoError::ElementoImportadoNoEncontrado => "elemento importado no encontrado",
            CodigoError::ErrorPermisosModulo => "error de permisos al acceder al archivo de módulo",
            
            // E0500-E0599
            CodigoError::ObjetoNoEncontrado => "objeto no encontrado o no definido",
            CodigoError::MetodoNoEncontrado => "método no encontrado en el objeto",
            CodigoError::MiembroNoExiste => "miembro no existe en el objeto",
            CodigoError::AccesoMiembroPrivado => "acceso a miembro privado",
            CodigoError::ConstructorParametrosIncorrectos => "constructor con parámetros incorrectos",
            CodigoError::InstanciacionSinConstructor => "instanciación de objeto sin constructor",
            CodigoError::AmbienteFueraDeObjeto => "uso incorrecto de 'ambiente' fuera de objeto",
            CodigoError::HerenciaCircular => "herencia circular o inválida",
            
            // E0600-E0699
            CodigoError::IndiceFueraDeRango => "acceso a índice fuera de rango",
            CodigoError::IndiceTipoIncorrecto => "índice de tipo incorrecto (debe ser entero)",
            CodigoError::ListaVaciaNoPermitida => "operación en lista vacía no permitida",
            CodigoError::ModificacionListaConstante => "modificación de lista constante",
            CodigoError::TipoElementoIncompatible => "tipo de elemento incompatible con la lista tipada",
            CodigoError::DimensionesMatrizIncorrectas => "dimensiones incorrectas en matrices",
            CodigoError::AsignacionMatrizInvalida => "asignación inválida en matriz",
            CodigoError::MetodoNoDisponibleLista => "método no disponible para el tipo de lista",
            
            // E0700-E0799
            CodigoError::JsonMalformado => "JSON malformado o inválido",
            CodigoError::ClaveDuplicadaJson => "clave duplicada en objeto JSON",
            CodigoError::PropiedadInexistenteJson => "acceso a propiedad inexistente en JSON",
            CodigoError::TipoIncompatibleJson => "tipo incompatible en conversión a JSON",
            CodigoError::DecodificacionJsonFallida => "decodificación de JSON fallida",
            CodigoError::EstructuraJsonMuyProfunda => "estructura JSON anidada demasiado profunda",
            
            // E0800-E0899
            CodigoError::NumeroArgumentosIncorrecto => "número incorrecto de argumentos en llamada",
            CodigoError::TipoArgumentoIncorrecto => "tipo de argumento incorrecto",
            CodigoError::FuncionRecursivaSinCasoBase => "función recursiva sin caso base",
            CodigoError::DesbordamientoPilaRecursion => "desbordamiento de pila por recursión excesiva",
            CodigoError::FuncionAsincronaSinEsperar => "función asíncrona llamada sin 'esperar'",
            CodigoError::FuncionAsincronaSinOperaciones => "función marcada como asíncrona pero sin operaciones asíncronas",
            CodigoError::EsperarFueraDeAsincrona => "'esperar' usado fuera de función asíncrona",
            CodigoError::TipoRetornoIncorrecto => "tipo de retorno incorrecto",
            
            // E0900-E0999
            CodigoError::ErrorLecturaArchivo => "error de entrada/salida al leer archivo",
            CodigoError::ErrorEscrituraArchivo => "error de entrada/salida al escribir archivo",
            CodigoError::MemoriaInsuficiente => "memoria insuficiente para la operación",
            CodigoError::TiempoEjecucionExcedido => "tiempo de ejecución excedido",
            CodigoError::InterrupcionUsuario => "interrupción del usuario (Ctrl+C)",
            CodigoError::ErrorInternoInterprete => "error interno del intérprete",
            CodigoError::OperacionNoSoportadaSO => "operación no soportada en el sistema operativo",
            CodigoError::ViolacionSeguridad => "violación de seguridad o acceso",
        }
    }
    
    pub fn ayuda(&self) -> Option<&'static str> {
        match self {
            CodigoError::DelimitadoresDesbalanceados => Some("asegúrate de cerrar todos los bloques con '}'."),
            CodigoError::DivisionPorCero => Some("verifica que el divisor no sea cero antes de realizar la división."),
            CodigoError::VariableNoDeclarada => Some("asegúrate de declarar la variable antes de usarla."),
            CodigoError::FuncionNoDeclarada => Some("asegúrate de declarar la función antes de llamarla."),
            CodigoError::TiposIncompatibles => Some("verifica que los tipos sean compatibles para esta operación."),
            CodigoError::IndiceFueraDeRango => Some("verifica que el índice esté dentro del rango válido de la lista."),
            CodigoError::ModuloNoEncontrado => Some("verifica que la ruta del módulo sea correcta y que el archivo exista."),
            CodigoError::AccesoMiembroPrivado => Some("el miembro es privado y solo puede ser accedido desde dentro del objeto."),
            _ => None,
        }
    }
}
