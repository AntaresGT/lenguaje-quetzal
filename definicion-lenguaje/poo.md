# Programación Orientada a Objetos

> Sintaxis de `objeto` (clases), `prototipo` (interfaces), herencia,
> modificadores de visibilidad, miembros estáticos y `libre` en
> Lenguaje Quetzal.

## 1. `objeto` (clase)

### 1.1 Sintaxis

```ebnf
objeto = "objeto" , IDENTIFICADOR ,
         [ "hereda" , lista_identificadores ] ,
         [ "como" , IDENTIFICADOR ] ,
         [ "implementa" , lista_identificadores ] ,
         "{" , { seccion_miembro } , "}" ;
```

### 1.2 Ejemplo mínimo

```quetzal
objeto Usuario {
    publico:
        texto nombre
        entero edad

        Usuario(texto n, entero e) {
            esto.nombre = n
            esto.edad = e
        }

        texto obtener_info() {
            retornar t"{nombre} ({edad})"
        }
}
```

### 1.3 Bloques de visibilidad

Los miembros se agrupan en **secciones** según su visibilidad:

```quetzal
objeto Cuenta {
    privado:
        texto var titular
        número var saldo

        número obtener_saldo_privado() {
            retornar saldo
        }

    publico:
        Cuenta(texto titular_inicial, número saldo_inicial) {
            titular = titular_inicial
            saldo = saldo_inicial
        }

        número ver_saldo() {
            retornar obtener_saldo_privado()
        }
}
```

| Keyword | Variantes | Significado |
|---|---|---|
| `privado` | — | accesible solo dentro del objeto |
| `publico` | `público` | accesible desde fuera |

## 2. Miembros estáticos: `libre`

`libre` marca un miembro como **estático** (accesible sin instanciar
la clase). Equivale a `static` de Java/C#.

```quetzal
objeto UsuarioLibre {
    libre texto nombre
    libre entero edad
    libre entero absoluto(entero valor) {
        retornar valor < 0 ? -valor : valor
    }
}

// Uso sin instanciar
entero v = UsuarioLibre.absoluto(-10)   // 10
```

`libre` puede aplicarse tanto a **atributos** como a **funciones**.

## 3. Constructor

El constructor es un **método con el mismo nombre que la clase** y
**sin tipo de retorno** explícito.

```quetzal
objeto Punto {
    publico:
        número var x
        número var y

        Punto(número x_ini, número y_ini) {
            x = x_ini
            y = y_ini
        }
}
```

## 4. `esto` y `padre`

| Keyword | Significado |
|---|---|
| `esto` | referencia a la instancia actual (`this`) |
| `padre` | referencia a la clase padre (`super`) |

```quetzal
objeto Animal {
    publico:
        texto nombre
        texto especie

        Animal(texto n, texto e) {
            nombre = n
            especie = e
        }

        texto describir() {
            retornar t"{nombre} ({especie})"
        }
}

objeto Perro hereda Animal {
    publico:
        Perro(texto n, texto e) {
            padre.Animal(n, e)        // llamada al constructor padre
        }

        texto ladrar() {
            retornar "Guau!"
        }
}
```

> `padre` no se usa como llamada a función, sino como **objeto** sobre
> el cual se accede a constructores y métodos del padre:
> `padre.Animal(...)`, `padre.Mamifero.comer()`.

## 5. Herencia

### 5.1 Herencia simple

```quetzal
objeto Perro hereda Animal {
    publico:
        Perro(texto n) {
            padre.Animal(n, "Canis familiaris")
        }
}
```

### 5.2 Herencia en cascada

```quetzal
objeto Mamifero hereda Animal { ... }
objeto Gato hereda Mamifero {
    publico:
        Gato(texto n) {
            padre.Mamifero(n, "Felis catus")
        }
}
```

### 5.3 Herencia múltiple

```quetzal
objeto Felino hereda Mamifero, Animal {
    publico:
        Felino(texto n) {
            padre.Mamifero(n, "Felis")
            padre.Animal(n, "Felis")
        }

        texto comer() {
            retornar padre.Mamifero.comer()   // llamada a método de padre específico
        }
}
```

> **Resolución de ambigüedad**: anteponer `padre.NombreClase.metodo()`.

## 6. `prototipo` (interfaz)

```quetzal
prototipo DocumentoFiscal {
    publico:
        texto serie
        número subtotal

        número calcular_total()

        log opcional validar_nit(texto nit)    // método opcional
}
```

- Los prototipos declaran **firmas** (atributos con tipo, métodos sin
  cuerpo).
- Un miembro marcado `opcional` **no requiere implementación**.
- Miembros con `var` son implícitamente opcionales.

### 6.1 Implementación

```quetzal
objeto FacturaElectronica implementa DocumentoFiscal {
    publico:
        texto serie = "F-001"
        número subtotal = 0.0

        número calcular_total() {
            retornar subtotal * 1.12     // +IVA
        }

        log validar_nit(texto nit) {
            retornar nit.longitud() == 9
        }
}
```

### 6.2 Implementación múltiple

```quetzal
objeto MiClase implementa InterfaceA, InterfaceB, InterfaceC { ... }
```

## 7. Combinación herencia + interfaz

```quetzal
objeto CuentaInterna como EntidadAuditable implementa Autenticable {
    texto usuario = "admin"

    log autenticar(texto clave) {
        retornar clave == "admin-123"
    }
}
```

Sintaxis: `objeto X como <padre_alias> implementa <i1>[, <i2>...]`

## 8. Instanciación: `nuevo`

```quetzal
Usuario u = nuevo Usuario("Ana", 30)
Punto p = nuevo Punto(1.0, 2.0)
```

> Sintaxis: `nuevo Identificador(argumentos)`. No se observa el patrón
> `Usuario(...)` sin `nuevo` (estilo Kotlin).

## 9. Resumen de keywords POO

| Keyword | Significado |
|---|---|
| `objeto` | clase |
| `prototipo` | interfaz (con opcionales) |
| `hereda` | extiende otra clase |
| `implementa` | implementa prototipos |
| `como` | alias de padre (en declaración de objeto) o de import |
| `nuevo` | instanciación |
| `esto` | this |
| `padre` | super (usado como prefijo: `padre.Clase.metodo()`) |
| `publico` | public |
| `privado` | private |
| `libre` | static |
| `opcional` | marca miembros opcionales en prototipos |
