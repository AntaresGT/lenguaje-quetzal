# Diagramas de Sintaxis — Lenguaje Quetzal

> Diagramas estilo *railroad* (Mermaid) + **grafo de pipeline**.
> Machine/IA: nodos = producciones; aristas = flujo de tokens/AST.
> Canónico: [`identidad.md`](./identidad.md) · [`catalogo.json`](./catalogo.json).

---

## 0. Grafo maestro: pipeline

Cómo se mueve un programa Quetzal (fuente → valor):

```mermaid
flowchart TB
    SRC[".qz UTF-8"] --> LEX["Lexer\ntokenizar"]
    MAN["quetzal.json"] --> PERM["Permisos"]
    LEX --> TOK["Tokens"]
    TOK --> PAR["Parser\nEBNF"]
    PAR --> AST["AST"]
    AST --> TIP["Tipos\nexplícitos"]
    TIP --> IMP["Resolver\nimportar/exportar"]
    IMP --> PERM
    PERM --> LOAD["Cargador"]
    LOAD --> EVAL["Evaluador"]
    EVAL -->|"llamada"| STD["Stdlib"]
    STD -->|"valor / excepción"| EVAL
    EVAL --> OUT["Efectos\nconsola · FS · red"]
```

### 0.1 Grafo de categorías sintácticas

```mermaid
flowchart LR
    PROG[programa] --> SENT[sentencia*]
    SENT --> DECL[declaración]
    SENT --> ASIG[asignación]
    SENT --> CTRL[control]
    SENT --> DEF[definición]
    SENT --> MOD[módulo]
    SENT --> EXPR[expresión;]

    DECL --> TIPO[tipo]
    TIPO --> PRIM[entero número texto log vacío]
    TIPO --> COMP[lista jsn funcion]

    CTRL --> SI[si]
    CTRL --> LOOP[mientras para hacer]
    CTRL --> EX[intentar capturar]

    DEF --> FN[función asincrono]
    DEF --> OBJ[objeto]
    DEF --> PROT[prototipo]

    MOD --> IMP[importar desde]
    MOD --> EXP[exportar]

    EXPR --> OPS[ops · ternario · llamar · nuevo]
```

### 0.2 Flujo de una expresión (precedencia)

```mermaid
flowchart BT
    PRIM[primario · literal · id · lista · jsn] --> POST[postfijo . [] call ++ --]
    POST --> UN[unario ! - ++ --]
    UN --> MUL[* / %]
    MUL --> SUM[+ -]
    SUM --> CMP[< <= > >=]
    CMP --> EQ[== !=]
    EQ --> AND[&&]
    AND --> OR[||]
    OR --> TER[? :]
```

---

## 1. Programa (estructura general)

```mermaid
flowchart LR
    Start(( )) --> Sent["sentencia"]
    Sent --> More{"¿más?"}
    More -- sí --> Sent
    More -- no --> End(( ))
```

---

## 2. Declaración de variable

```mermaid
flowchart LR
    A(( )) --> T["tipo"]
    T --> V["var (opcional)"]
    V --> ID["identificador"]
    ID --> Eq["="]
    Eq --> E["expresión"]
    E --> Fin(( ))
```

Variantes equivalentes:
```quetzal
entero edad = 41                    // inmutable
texto var nombre = "Ana"           // mutable
número var pi = 3.14
```

---

## 3. Asignación

```mermaid
flowchart LR
    A(( )) --> L["lvalue (id o miembro)"]
    L --> Op{{"+=" | "-=" | "*=" | "/=" | "%=" | "="}}
    Op --> E["expresión"]
    E --> Sc[";"]
    Sc --> Fin(( ))
```

---

## 4. Condicional `si / sino si / sino`

```mermaid
flowchart LR
    A(( )) --> Si["si"]
    Si --> Lp["("]
    Lp --> Cond["expresión"]
    Cond --> Rp[")"]
    Rp --> Blk["bloque { ... }"]
    Blk --> Elif{"sino si?"}
    Elif -- sí --> Si2["si"]
    Si2 --> Lp2["("]
    Lp2 --> Cond2["expresión"]
    Cond2 --> Rp2[")"]
    Rp2 --> Blk2["bloque { ... }"]
    Blk2 --> Elif
    Elif -- no --> Else{"¿sino?"}
    Else -- sí --> Sino["sino"]
    Sino --> Blk3["bloque { ... }"]
    Else -- no --> Fin(( ))
```

---

## 5. Bucle `mientras`

```mermaid
flowchart LR
    A(( )) --> M["mientras"]
    M --> Lp["("]
    Lp --> Cond["expresión"]
    Cond --> Rp[")"]
    Rp --> Blk["bloque { ... }"]
    Blk --> Fin(( ))
```

---

## 6. Bucle `hacer ... mientras`

```mermaid
flowchart LR
    A(( )) --> H["hacer"]
    H --> Blk["bloque { ... }"]
    Blk --> M["mientras"]
    M --> Lp["("]
    Lp --> Cond["expresión"]
    Cond --> Rp[")"]
    Rp --> Sc[";"]
    Sc --> Fin(( ))
```

---

## 7. Bucle `para` estilo C

```mermaid
flowchart LR
    A(( )) --> P["para"]
    P --> Lp["("]
    Lp --> Init["init: tipo var i = expr"]
    Init --> Sc1[";"]
    Sc1 --> Cond["condición: expr"]
    Cond --> Sc2[";"]
    Sc2 --> Step["paso: expr"]
    Step --> Rp[")"]
    Rp --> Blk["bloque { ... }"]
    Blk --> Fin(( ))
```

---

## 8. Bucle `para (var x en|cada lista)`

```mermaid
flowchart LR
    A(( )) --> P["para"]
    P --> Lp["("]
    Lp --> T["tipo"]
    T --> Var["var"]
    Var --> ID["identificador"]
    ID --> Kw{{"en" | "cada"}}
    Kw --> Coll["expresión (lista)"]
    Coll --> Rp[")"]
    Rp --> Blk["bloque { ... }"]
    Blk --> Fin(( ))
```

> `en` y `cada` son **sinónimos** y equivalentes en semántica.

---

## 9. Manejo de excepciones `intentar`

```mermaid
flowchart LR
    A(( )) --> Int["intentar"]
    Int --> Blk["bloque { ... }"]
    Blk --> Cap["capturar"]
    Cap --> Lp["("]
    Lp --> Ex["excepcion"]
    Ex --> ID["identificador"]
    ID --> Rp[")"]
    Rp --> BlkCap["bloque { ... }"]
    BlkCap --> FinOp{"¿finalmente?"}
    FinOp -- sí --> Fin["finalmente"]
    Fin --> BlkFin["bloque { ... }"]
    FinOp -- no --> End(( ))
    BlkFin --> End
```

---

## 10. `lanzar` excepción

```mermaid
flowchart LR
    A(( )) --> L["lanzar"]
    L --> E["expresión"]
    E --> Sc[";"]
    Sc --> Fin(( ))
```

---

## 11. Definición de función

```mermaid
flowchart LR
    A(( )) --> Async["asincrono (opcional)"]
    Async --> T["tipo de retorno"]
    T --> ID["nombre"]
    ID --> Lp["("]
    Lp --> Params["parámetros (opcional)"]
    Params --> Rp[")"]
    Rp --> Blk["bloque { ... }"]
    Blk --> Fin(( ))
```

Parámetros:
```mermaid
flowchart LR
    A(( )) --> P1["tipo [var] id"]
    P1 --> More{"¿más?"}
    More -- sí --> Coma[","]
    Coma --> Pn["tipo [var] id"]
    Pn --> More
    More -- no --> Fin(( ))
```

---

## 12. Definición de `objeto` (clase)

```mermaid
flowchart LR
    A(( )) --> Obj["objeto"]
    Obj --> ID["nombre"]
    ID --> Hereda{"¿hereda?"}
    Hereda -- sí --> H["hereda"]
    H --> Padres["id, id, ..."]
    Padres --> Como
    Hereda -- no --> Como
    Como{"¿como?"}
    Como -- sí --> C["como"]
    C --> Alias["id (padre)"]
    Alias --> Impl
    Como -- no --> Impl
    Impl{"¿implementa?"}
    Impl -- sí --> I["implementa"]
    I --> Prots["id, id, ..."]
    Prots --> Llave
    Impl -- no --> Llave["{"]
    Llave --> Secciones["secciones privado:/publico:"]
    Secciones --> Llave2["}"]
    Llave2 --> Fin(( ))
```

Sección de miembros:
```mermaid
flowchart LR
    A(( )) --> Vis{{"privado" | "publico"}}
    Vis --> D[":"]
    D --> M1["miembro"]
    M1 --> More{"¿más?"}
    More -- sí --> Mn["miembro"]
    Mn --> More
    More -- no --> Fin(( ))
```

---

## 13. Definición de `prototipo` (interfaz)

```mermaid
flowchart LR
    A(( )) --> P["prototipo"]
    P --> ID["nombre"]
    ID --> Llave["{"]
    Llave --> Secciones["secciones privado:/publico:"]
    Secciones --> Llave2["}"]
    Llave2 --> Fin(( ))
```

---

## 14. Miembro `libre` (estático)

```mermaid
flowchart LR
    A(( )) --> L["libre"]
    L --> D["declaración o función"]
    D --> Fin(( ))
```

---

## 15. Importar módulo

```mermaid
flowchart LR
    A(( )) --> Imp["importar"]
    Imp --> Llave["{"]
    Llave --> It1["id"]
    It1 --> Alias1["como id (opcional)"]
    Alias1 --> Coma{"¿más?"}
    Coma -- sí --> Co[","]
    Co --> Itn["id"]
    Itn --> Aliasn["como id (opcional)"]
    Aliasn --> Coma
    Coma -- no --> Llave2["}"]
    Llave2 --> Desde["desde"]
    Desde --> Str["\"ruta\""]
    Str --> Fin(( ))
```

---

## 16. Exportar módulo

```mermaid
flowchart LR
    A(( )) --> Exp["exportar"]
    Exp --> Llave["{"]
    Llave --> It1["id"]
    It1 --> Coma{"¿más?"}
    Coma -- sí --> Co[","]
    Co --> Itn["id"]
    Itn --> Coma
    Coma -- no --> Llave2["}"]
    Llave2 --> Sc[";"]
    Sc --> Fin(( ))
```

---

## 17. Expresión (jerarquía de operadores)

```mermaid
flowchart LR
    A(( )) --> T["ternario (? :)"]
    T --> Or["o (||)"]
    Or --> And["y (&&)"]
    And --> Eq["igualdad (== !=)"]
    Eq --> Cmp["comparación (< <= > >=)"]
    Cmp --> Sum["suma (+ -)"]
    Sum --> Mul["mult (* / %)"]
    Mul --> Un["unario (! - ++ --)"]
    Un --> Post["postfijo (. [] () ++ --)"]
    Post --> Prim["primario"]
    Prim --> Fin(( ))
```

---

## 18. Literal numérico

```mermaid
flowchart LR
    A(( )) --> Dig["dígito+"]
    Dig --> Punto{"¿decimal?"}
    Punto -- sí --> Pt["."]
    Pt --> Dig2["dígito+"]
    Dig2 --> Fin(( ))
    Punto -- no --> Fin
```

---

## 19. Literal de texto (cadena)

```mermaid
flowchart LR
    A(( )) --> Q1["\""]
    Q1 --> Body["caracteres y escapes (\n \t \" \\ \{ \})"]
    Body --> Q2["\""]
    Q2 --> Fin(( ))
```

---

## 20. Literal de texto interpolado (template string)

```mermaid
flowchart LR
    A(( )) --> T["t"]
    T --> Q1["\""]
    Q1 --> Body["texto plano o escapes"]
    Body --> More{"¿más?"}
    More -- sí --> Op{{"texto | {expr}"}}
    Op --> More
    More -- no --> Q2["\""]
    Q2 --> Fin(( ))
```

---

## 21. Literal de lista

```mermaid
flowchart LR
    A(( )) --> Lb["["]
    Lb --> E1["expresión"]
    E1 --> More{"¿más?"}
    More -- sí --> Co[","]
    Co --> En["expresión"]
    En --> More
    More -- no --> Rb["]"]
    Rb --> Fin(( ))
```

---

## 22. Literal de objeto (JSON)

```mermaid
flowchart LR
    A(( )) --> Lb["{"]
    Lb --> K1["clave (cadena o id)"]
    K1 --> Dp[":"]
    Dp --> V1["expresión"]
    V1 --> More{"¿más?"}
    More -- sí --> Co[","]
    Co --> Kn["clave"]
    Kn --> Dpn[":"]
    Dpn --> Vn["expresión"]
    Vn --> More
    More -- no --> Rb["}"]
    Rb --> Fin(( ))
```

---

## 23. Acceso a miembro / llamada a método

```mermaid
flowchart LR
    A(( )) --> P["primario"]
    P --> Op{{". id | [expr] | (args) | ++ | --"}}
    Op --> P
    Op --> Fin(( ))
```

---

## 24. `nuevo` (instanciación)

```mermaid
flowchart LR
    A(( )) --> N["nuevo"]
    N --> ID["identificador"]
    ID --> Lp["("]
    Lp --> Args["argumentos (opcional)"]
    Args --> Rp[")"]
    Rp --> Fin(( ))
```

---

## 25. Diagrama maestro: árbol de decisiones de sentencia

```mermaid
flowchart TB
    S((sentencia)) --> D{¿qué tipo?}
    D -- "tipo + id + =" --> Decl["declaración"]
    D -- "id + op=" --> Asig["asignación"]
    D -- "si / mientras / para / intentar" --> Ctrl["estructura de control"]
    D -- "romper / continuar / retornar" --> Flow["control de flujo"]
    D -- "expr + ;" --> SE["sentencia-expresión"]

    Decl --> D1["tipo [var] id = expr"]
    Asig --> A1["lvalue op= expr ;"]
    Ctrl --> C1{construcción}
    C1 --> C2["si ... sino ... { }"]
    C1 --> C3["mientras (expr) { }"]
    C1 --> C4["hacer { } mientras (expr) ;"]
    C1 --> C5["para (init; cond; step) { }"]
    C1 --> C6["para (tipo var id en expr) { }"]
    C1 --> C7["intentar { } capturar (ex id) { } [finalmente { }]"]
    C1 --> C8["lanzar expr ;"]
```

---

## Cómo usar estos diagramas

- **Pipeline / grafo:** §0 — flujo fuente→runtime y categorías sintácticas.
- **Railroad:** §1–§25 — una producción por diagrama.
- Render: GitHub / GitLab / VS Code Mermaid / Obsidian.
- IA/máquina: emparejar con `gramatica.json` + `catalogo.json`.
- Parser: LR del railroad = orden de tokens; rombos = decisiones.
