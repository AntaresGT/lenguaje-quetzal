# Gramática Formal de Lenguaje Quetzal (EBNF)

> Sintaxis formal. Pareja machine-readable: [`gramatica.json`](./gramatica.json).
> Identidad: [`identidad.md`](./identidad.md) · catálogo: [`catalogo.json`](./catalogo.json).

```yaml
doc: gramatica.ebnf
start: programa
normaliza_tildes: true
```

### Objetivo
Definir producciones para implementar parsers y validar programas.

### Funciones
Programa → sentencias → declaraciones / control / expresiones / POO / módulos.

### Funciones de la API
Ninguna — gramática, no runtime.

## Notación

| Símbolo | Significado |
|---|---|
| `"x"` | Literal exacto (case-sensitive, con tildes cuando aplique) |
| `A B` | Concatenación (secuencia) |
| `A \| B` | Alternativa (uno u otro) |
| `[ A ]` | Opcional (cero o una vez) |
| `{ A }` | Cero o más veces |
| `( A )` | Agrupación |
| `?` en JSON | Equivalente a `[ ... ]` |

## 1. Programa

```ebnf
programa        = { sentencia } ;

sentencia       = declaracion
                | asignacion
                | estructura_control
                | sentencia_expresion ;

sentencia_expresion = expresion , ";" ;
```

## 2. Tipos

```ebnf
tipo            = tipo_primitivo | tipo_compuesto ;

tipo_primitivo  = "entero"
                | "numero"
                | "texto"
                | "log"
                | "vacio" ;

tipo_compuesto  = tipo_lista | tipo_objeto ;
tipo_lista      = "lista" , [ "<" , tipo , ">" ] ;
tipo_objeto     = "jsn" ;
```

> **Nota:** El lexer normaliza tildes, por lo que `número` ≡ `numero`,
> `vacío` ≡ `vacio`, `público` ≡ `publico`, `asincróno` ≡ `asincrono`,
> `excepción` ≡ `excepcion` son todas equivalentes.

## 3. Declaraciones y asignaciones

```ebnf
declaracion     = tipo , [ "var" ] , IDENTIFICADOR , "=" , expresion ;

asignacion      = ( objetivo | expresion_miembro ) , op_asignacion , expresion , ";" ;
objetivo        = IDENTIFICADOR ;
expresion_miembro = expresion_primaria , { ( "." , IDENTIFICADOR | "[" , expresion , "]" ) } ;

op_asignacion   = "=" | "+=" | "-=" | "*=" | "/=" | "%=" ;
```

> Por defecto, las variables son **inmutables**. `var` introduce
> mutabilidad. Los atributos de un objeto sin `var` no son reasignables
> individualmente, pero los del objeto contenedor marcados con `var` sí.

## 4. Estructuras de control

### 4.1 Condicionales

```ebnf
condicional     = "si" , "(" , expresion , ")" , bloque
                , { "sino" , "si" , "(" , expresion , ")" , bloque }
                , [ "sino" , bloque ] ;
```

### 4.2 Bucles

```ebnf
bucle           = bucle_mientras
                | bucle_hacer
                | bucle_para_c
                | bucle_para_coleccion ;

bucle_mientras  = "mientras" , "(" , expresion , ")" , bloque ;

bucle_hacer     = "hacer" , bloque , "mientras" , "(" , expresion , ")" , ";" ;

bucle_para_c    = "para" , "(" , declaracion_for , ";" , expresion , ";" , expresion , ")" , bloque ;
declaracion_for = tipo , [ "var" ] , IDENTIFICADOR , "=" , expresion ;

bucle_para_coleccion
                = "para" , "(" , tipo , "var" , IDENTIFICADOR ,
                    ( "en" | "cada" ) , expresion , ")" , bloque ;
```

### 4.3 Control de flujo directo

```ebnf
romper          = "romper" , ";" ;
continuar       = "continuar" , ";" ;
retorno         = "retornar" , [ expresion ] , ";" ;
```

### 4.4 Excepciones

```ebnf
intentar        = "intentar" , bloque
                , "capturar" , "(" , "excepcion" , IDENTIFICADOR , ")" , bloque
                , [ "finalmente" , bloque ] ;

lanzar          = "lanzar" , expresion , ";" ;
```

### 4.5 Bloque

```ebnf
bloque          = "{" , { sentencia } , "}" ;
```

## 5. Funciones

```ebnf
funcion         = [ "asincrono" ] , tipo , IDENTIFICADOR ,
                  "(" , [ parametros ] , ")" , bloque ;

parametros      = parametro , { "," , parametro } ;
parametro       = tipo , [ "var" ] , IDENTIFICADOR ;
```

## 6. Programación orientada a objetos

```ebnf
objeto          = "objeto" , IDENTIFICADOR ,
                  [ "hereda" , lista_identificadores ]
                  , [ "como" , IDENTIFICADOR ]
                  , [ "implementa" , lista_identificadores ]
                  , "{" , { seccion_miembro } , "}" ;

prototipo       = "prototipo" , IDENTIFICADOR ,
                  "{" , { seccion_miembro } , "}" ;

lista_identificadores = IDENTIFICADOR , { "," , IDENTIFICADOR } ;

seccion_miembro = ( "privado" | "publico" ) , ":" , { miembro } ;
miembro         = declaracion
                | funcion
                | declaracion_libre ;

declaracion_libre = "libre" , ( declaracion | funcion ) ;
```

> `libre` denota un miembro **estático** (accesible sin instanciar la
> clase). `como` permite aliasing de padre (`objeto X como Padre
> implementa Interface`). `hereda` admite uno o más padres separados
> por coma (herencia múltiple).

## 7. Módulos

```ebnf
importar        = "importar" , "{" , lista_importaciones , "}" ,
                  "desde" , CADENA ;

lista_importaciones = item_importacion , { "," , item_importacion } ;
item_importacion = IDENTIFICADOR , [ "como" , IDENTIFICADOR ] ;

exportar        = "exportar" , "{" , lista_identificadores , "}" ;
```

## 8. Expresiones

Jerarquía de precedencia (de menor a mayor):

```ebnf
expresion               = expresion_ternaria ;

expresion_ternaria      = expresion_or
                          , [ "?" , expresion , ":" , expresion_ternaria ] ;

expresion_or            = expresion_and , { "||" , expresion_and } ;
expresion_and           = expresion_igualdad , { "&&" , expresion_igualdad } ;
expresion_igualdad      = expresion_comparacion , { ( "==" | "!=" ) , expresion_comparacion } ;
expresion_comparacion  = expresion_suma , { ( "<" | ">" | "<=" | ">=" ) , expresion_suma } ;
expresion_suma          = expresion_mult , { ( "+" | "-" ) , expresion_mult } ;
expresion_mult          = expresion_unaria , { ( "*" | "/" | "%" ) , expresion_unaria } ;

expresion_unaria        = [ ( "!" | "-" | "++" | "--" ) ] , expresion_postfija ;

expresion_postfija      = expresion_primaria ,
                          { ( "." , IDENTIFICADOR
                            | "[" , expresion , "]"
                            | "(" , [ argumentos ] , ")"
                            | "++"
                            | "--" ) } ;

expresion_primaria      = literal
                        | IDENTIFICADOR
                        | "esto"
                        | "padre" , ( "." , IDENTIFICADOR , [ "(" , [ argumentos ] , ")" ] )+
                        | "(" , expresion , ")"
                        | "nuevo" , IDENTIFICADOR , "(" , [ argumentos ] , ")" ;

argumentos              = expresion , { "," , expresion } ;
```

## 9. Literales

```ebnf
literal                 = literal_numero
                        | literal_texto
                        | literal_texto_interpolado
                        | literal_logico
                        | literal_nulo
                        | literal_lista
                        | literal_objeto ;

literal_numero          = NUMERO_ENTERO | NUMERO_DECIMAL ;
literal_logico          = "verdadero" | "falso" ;
literal_nulo            = "nulo" ;

literal_texto           = '"' , { caracter | escape } , '"' ;
literal_texto_interpolado = 't"' , { caracter | escape | interpolacion } , '"' ;
interpolacion           = '{' , expresion , '}' ;
escape                  = '\\' , ( 'n' | 't' | 'r' | '"' | '\\' | '{' | '}' ) ;

literal_lista           = "[" , [ expresion , { "," , expresion } ] , "]" ;

literal_objeto          = "{" , [ par , { "," , par } ] , "}" ;
par                     = ( CADENA | IDENTIFICADOR ) , ":" , expresion ;
```

> **Interpolación**: las cadenas con prefijo `t` (template string)
> evalúan expresiones dentro de `{ }`. Para representar una llave
> literal se escapa con `\{` o `\}`.

## 10. Tokens léxicos

```ebnf
IDENTIFICADOR   = ( letra | "_" ) , { letra | digito | "_" } ;
letra           = "A".."Z" | "a".."z"
                | "á" | "é" | "í" | "ó" | "ú"
                | "Á" | "É" | "Í" | "Ó" | "Ú" | "ñ" | "Ñ" ;
digito          = "0".."9" ;

NUMERO_ENTERO   = digito , { digito } ;
NUMERO_DECIMAL  = digito , { digito } , "." , digito , { digito } ;

CADENA          = '"' , { caracter | escape } , '"' ;

COMENTARIO      = "//" , { caracter - EOL } , EOL ;
```

## 11. Resumen de precedencia de operadores

| Nivel | Operador | Asociatividad |
|---|---|---|
| 1 (más bajo) | `? :` (ternario) | Derecha |
| 2 | `\|\|` | Izquierda |
| 3 | `&&` | Izquierda |
| 4 | `==` `!=` | Izquierda |
| 5 | `<` `<=` `>` `>=` | Izquierda |
| 6 | `+` `-` | Izquierda |
| 7 | `*` `/` `%` | Izquierda |
| 8 | `!` `-` (unario) `++` `--` (pre) | Derecha |
| 9 | `.` `[]` `()` `++` `--` (post) | Izquierda |
| 10 (más alto) | literales, `()`, `nuevo`, `esto`, `padre` | — |
