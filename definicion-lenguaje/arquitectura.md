# Arquitectura de Lenguaje Quetzal

> Mapa de crates, pipeline de ejecución y límites de responsabilidad.
> Pareja de sintaxis: [`diagramas.md`](./diagramas.md).
> Identidad: [`identidad.md`](./identidad.md).

```yaml
doc: arquitectura
implementación: Rust workspace
binario: quetzal (crate lnc)
orquestador: motor (MotorQuetzal)
versión_workspace: 0.0.2
```

---

## Objetivo

Documentar **cómo está construido** el intérprete Quetzal (crates + flujo),
para humanos, IA y tooling — no confundir con la gramática del lenguaje.

### Funciones de este documento
- Grafo de capas (CLI → motor → pipeline → VM → stdlib)
- Dependencias entre crates
- Pipeline de un `.qz` hasta efecto (consola / FS / red)
- Límites: qué crate hace qué

### Funciones de la API (punto de entrada)
| Superficie | Crate | Símbolo |
|---|---|---|
| CLI | `lnc` | binario `quetzal` |
| Embebido | `motor` | `MotorQuetzal` |
| REPL | `repl` | `iniciar()` |
| LSP | `lsp` | esqueleto (fases futuras) |

---

## 1. Grafo maestro de arquitectura

```mermaid
flowchart TB
    subgraph SUPERFICIE["Superficie"]
        CLI["lnc\nbinario quetzal"]
        REPL["repl"]
        EMB["API embebida\nMotorQuetzal"]
        LSP["lsp\nesqueleto"]
    end

    subgraph ORQ["Orquestación"]
        MOT["motor\nCargador · Sesión · Config"]
        PAQ["paquetes\nquetzal.json · bloqueo · cache"]
    end

    subgraph FRONT["Frontend del lenguaje"]
        LEX["lexico\ntokenizar"]
        SYN["sintaxis\nparsear_modulo"]
        AST["ast\nárbol"]
        SEM["semantica\ntipos · imports · permisos"]
    end

    subgraph BACK["Backend de ejecución"]
        BC["bytecode\ngenerar_modulo"]
        VM["maquina_virtual\nVm · BucleEventos"]
        RT["runtime\nGuardianPermisos"]
        NAT["modulos_nativos\nconsola · math · tiempo\nmotor · FS · red"]
    end

    subgraph BASE["Base compartida"]
        NUC["nucleo\nFuente · Error · Ubicación"]
        DIA["diagnosticos\nreportes estilo Rust"]
    end

    CLI --> MOT
    REPL --> MOT
    EMB --> MOT
    LSP -.-> MOT

    MOT --> PAQ
    MOT --> LEX --> SYN --> AST
    AST --> SEM
    SEM --> BC --> VM
    MOT --> VM
    VM --> NAT
    NAT --> RT
    PAQ --> RT

    LEX --> NUC
    SYN --> NUC
    SEM --> NUC
    VM --> NUC
    MOT --> DIA
    CLI --> DIA
```

---

## 2. Capas (vista vertical)

```mermaid
flowchart LR
    L1["1. Superficie\nlnc · repl · lsp · embed"]
    L2["2. Motor\norquesta + carga módulos"]
    L3["3. Análisis\nlexico → sintaxis → ast → semantica"]
    L4["4. Código\nbytecode"]
    L5["5. Ejecución\nVM + bucle eventos"]
    L6["6. Efectos\nnativos + permisos + OS"]

    L1 --> L2 --> L3 --> L4 --> L5 --> L6
```

| Capa | Crates | Responsabilidad |
|---|---|---|
| Superficie | `lnc`, `repl`, `lsp` | CLI, REPL, editor |
| Orquestación | `motor`, `paquetes` | Ejecutar/revisar, manifiesto, deps, cache |
| Análisis | `lexico`, `sintaxis`, `ast`, `semantica` | Fuente → AST tipado |
| Código | `bytecode` | AST → instrucciones |
| Ejecución | `maquina_virtual` | Pila, marcos, async/eventos |
| Efectos | `modulos_nativos`, `runtime` | Stdlib + guardian de permisos |
| Base | `nucleo`, `diagnosticos` | Errores, fuente, reportes |

---

## 3. Pipeline de ejecución (detalle)

Flujo real al llamar `MotorQuetzal::ejecutar_*`:

```mermaid
sequenceDiagram
    participant U as Usuario / Host
    participant M as motor
    participant P as paquetes
    participant L as lexico
    participant S as sintaxis
    participant A as ast
    participant E as semantica
    participant B as bytecode
    participant V as maquina_virtual
    participant N as modulos_nativos
    participant R as runtime

    U->>M: ejecutar_archivo / ejecutar_texto
    M->>P: leer quetzal.json (si proyecto)
    P->>R: cargar permisos → GuardianPermisos
    M->>N: crear_registro_con_permisos
    M->>V: Vm::nueva(nativos)

    loop cada módulo .qz
        M->>L: tokenizar(Fuente)
        L->>S: tokens
        S->>A: Modulo AST
        A->>E: analizar_modulo
        E->>B: generar_modulo
        B->>V: cargar bytecode
    end

    V->>V: ejecutar entrada
    V->>N: llamadas nativas
    N->>R: chequear permiso
    N-->>V: valor / excepción
    V->>V: drenar_bucle_eventos
    M-->>U: Ok / Vec ErrorQuetzal
```

### Artefactos por etapa

| Etapa | Crate | Entrada | Salida |
|---|---|---|---|
| Léxico | `lexico` | `Fuente` | `Vec<Token>` |
| Parser | `sintaxis` | tokens | `ast::Modulo` |
| Semántica | `semantica` | AST | AST tipado + símbolos + imports |
| Bytecode | `bytecode` | AST | módulo de instrucciones |
| VM | `maquina_virtual` | bytecode + nativos | valores / efectos |
| Nativos | `modulos_nativos` | llamada + args | valor / `Fallo` |
| Permisos | `runtime` | solicitud | permitir / denegar |

---

## 4. Dependencias entre crates (grafo)

```mermaid
flowchart TB
    lnc --> motor
    lnc --> repl
    lnc --> paquetes
    lnc --> diagnosticos

    repl --> motor

    motor --> lexico
    motor --> sintaxis
    motor --> ast
    motor --> semantica
    motor --> bytecode
    motor --> maquina_virtual
    motor --> modulos_nativos
    motor --> runtime
    motor --> paquetes
    motor --> nucleo
    motor --> diagnosticos

    sintaxis --> lexico
    sintaxis --> ast
    sintaxis --> nucleo

    semantica --> ast
    semantica --> nucleo

    bytecode --> ast
    bytecode --> nucleo

    maquina_virtual --> bytecode
    maquina_virtual --> nucleo

    modulos_nativos --> maquina_virtual
    modulos_nativos --> runtime
    modulos_nativos --> nucleo

    paquetes --> runtime
    paquetes --> nucleo

    lexico --> nucleo
    diagnosticos --> nucleo
    runtime --> nucleo
```

**Regla:** flechas = “depende de”. Base = `nucleo`. Orquestador = `motor`.

---

## 5. Crate por crate

| Crate | Objetivo | API pública clave |
|---|---|---|
| `nucleo` | Tipos base compartidos | `Fuente`, `ErrorQuetzal`, `Ubicacion`, `ResultadoQuetzal` |
| `diagnosticos` | Errores legibles (estilo Rust) | `reportar`, `reportar_varios` |
| `lexico` | Fuente → tokens | `tokenizar`, `TipoToken`, `Token` |
| `sintaxis` | Tokens → AST | `Parser`, `parsear_modulo` |
| `ast` | Nodos del árbol | `Modulo`, `Sentencia`, `Expresion`, `DefinicionObjeto`, … |
| `semantica` | Tipos, mutabilidad, imports, permisos estáticos | `analizar_modulo`, `TipoSemantico` |
| `bytecode` | AST → instrucciones | `generar_modulo`, `Instruccion`, `Constante` |
| `maquina_virtual` | Ejecutar bytecode | `Vm`, `BucleEventos`, `RegistroNativos`, valores |
| `runtime` | Permisos en caliente | `GuardianPermisos`, `AccesoSolicitado` |
| `modulos_nativos` | Stdlib ES | `crear_registro_con_permisos`, módulos consola/math/… |
| `paquetes` | Proyecto y deps | `Manifiesto`, `Permisos`, `instalar_dependencias`, cache |
| `motor` | Orquestar todo | `MotorQuetzal`, `ConfiguracionMotor`, `SesionInteractiva` |
| `repl` | Shell interactivo | `iniciar` |
| `lnc` | CLI `quetzal` | comandos ejecutar / revisar / repl / instalar / nuevo |
| `lsp` | Language Server | esqueleto (futuro) |

---

## 6. Stdlib dentro de la VM

```mermaid
flowchart LR
    VM["Vm"] --> REG["RegistroNativos"]
    REG --> CON["consola"]
    REG --> MAT["matemática"]
    REG --> TMP["tiempo"]
    REG --> MOT["motor / regex"]
    REG --> FS["sistema_archivos\nArchivo Bits Flujo Observador"]
    REG --> RED["red\nServidorHttp ClienteHttp"]
    REG --> MET["métodos de tipo\ntexto lista jsn …"]

    FS --> GP["GuardianPermisos"]
    RED --> GP
    GP --> MAN["permisos de quetzal.json"]
```

Detalle de API: [`modulos-nativos.md`](./modulos-nativos.md) · [`metodos-nativos.md`](./metodos-nativos.md).

---

## 7. Proyecto en disco

```mermaid
flowchart TB
    ROOT["raíz del proyecto"]
    ROOT --> QJ["quetzal.json\nmanifiesto + permisos"]
    ROOT --> QB["quetzal.bloquear\nlockfile deps"]
    ROOT --> ENT["entrada.qz\ncampo entrada"]
    ROOT --> MOD["otros .qz\nimportar relativo"]
    ROOT --> CACHE["cache bytecode\npaquetes"]

    QJ --> PAQ["crate paquetes"]
    ENT --> MOT["crate motor / Cargador"]
    MOD --> MOT
```

Schema: [`esquema_quetzal.json`](./esquema_quetzal.json) · doc: [`manifiesto.md`](./manifiesto.md).

---

## 8. Relación con la especificación del lenguaje

| Pregunta | Documento |
|---|---|
| ¿Qué es Quetzal? | [`identidad.md`](./identidad.md) |
| ¿Qué features/API tiene? | [`catalogo.json`](./catalogo.json) |
| ¿Cómo se escribe? | [`gramatica.json`](./gramatica.json), [`diagramas.md`](./diagramas.md) |
| ¿Cómo está implementado? | **Este archivo** |
| ¿Qué crate toca X? | §5 tabla |

```mermaid
flowchart LR
    SPEC["definicion-lenguaje/\nsyntax + catálogo"] -. implementa .-> IMP["crates/\narquitectura"]
    IMP -. comportamiento observado .-> SPEC
```

---

## 9. Resumen en una frase

**`lnc`/`repl` llaman a `motor`, que lexea → parsea → tipa → genera bytecode → corre en `maquina_virtual` con `modulos_nativos` bajo `GuardianPermisos` de `runtime`, todo sobre `nucleo` + `diagnosticos`.**
