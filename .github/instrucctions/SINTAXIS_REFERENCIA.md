## Referencia de Sintaxis Quetzal (Derivada de Ejemplos Reales `ejemplos/*.qz`)

Esta referencia enumera únicamente construcciones y patrones observados en los archivos de ejemplo del repositorio. Su objetivo es evitar que agentes de IA inventen sintaxis no soportada. Si algo NO aparece aquí ni en la documentación oficial, no debe asumirse como válido.

---
### 1. Comentarios
// Comentario de una línea
No se observan comentarios multilínea de otro tipo.

### 2. Terminación de Instrucciones
No se usa punto y coma al final de líneas. Cada instrucción ocupa su propia línea lógica.

### 3. Palabras Clave Confirmadas
```
entero, número, texto, log, lóg, lista, jsn, vacio,
var, nulo, verdadero, falso,
objeto, privado:, publico:, libre,
si, sino, intentar, capturar, finalmente, lanzar, retornar,
mientras, para, hacer, en, cada, romper, continuar,
exportar, importar, desde, como, nuevo,
ambiente, rango, excepcion
```
Nota: `excepcion` aparece como tipo de parámetro en captura (`capturar (excepcion e)`). Ver sección 24 para política de tildes.

### 4. Declaración de Variables y Mutabilidad
Patrones:
```
entero x = 3
entero var x = 3         // mutable
lista<entero> l = [1,2]
lista var l = [1,2,3]
jsn var datos = { ... }
```
Orden con mutabilidad: `<tipo> var <identificador> = ...`
Se permite `lista<Tipo>` para listas tipadas; listas no tipadas usan `lista` sin genérico.
Asignación de `nulo` a cualquier tipo observado.

Regla FUNDAMENTAL: TODA declaración sin `var` es INMUTABLE (constante). Solo debe agregarse `var` cuando el valor será reasignado o su estructura interna cambiará (ej. re‑empaquetar una lista completa). Evitar `var` por defecto.

Ejemplos correctos:
```
entero limite = 10              // constante
entero var indice = 0           // cambiará en un bucle
texto saludo = "Hola"          // no muta
lista var acumulados = []       // se le agregarán elementos
```
Anti‑patrones (no usar):
```
entero var limite = 10          // 'var' innecesario si nunca cambia
texto var saludo = "Hola"      // innecesario si solo se lee
```
Parámetros de función: `tipo var nombre` indica que el parámetro puede modificarse dentro de la función (ver sección 8).

### 5. Tipos Primitivos y Estructurados
- Primitivos: `entero`, `número`, `texto`, `log`
- Estructurados: `lista`, `lista<tipo>`, `jsn`, objetos definidos con `objeto Nombre { ... }`
- Valor especial: `nulo`
- Booleanos: `verdadero`, `falso`

### 6. Listas
Líteras:
```
lista l = [1, 2, "Texto", verdadero, [1, "otra lista"]]
lista<entero> numeros = [1, 2, 3]
```
Acceso por índice:
```
mi_lista[0]
mi_lista[-1]          // índice negativo desde el final
```
Métodos observados (solo los presentes):
```
longitud(), esta_vacia(), agregar(valor), insertar(i, v), remover(valor), quitar_en(i), limpiar(),
contiene(valor), buscar(valor), buscar_ultimo(valor), contar(valor), ordenar(), ordenar_descendente(),
ordenado(), invertir(), primero(), ultimo(), tomar(n), saltar(n), sublista(inicio, fin), sumar(), promedio(),
maximo(), minimo(), unir(separador), concatenar(otra_lista), extender(otra_lista), texto(), json(), log(),
sumar() aplicado también a lista<número>
```
Creación por rango:
```
lista r = rango(1, 10)
```

### 7. JSON (jsn)
Literal con llaves y pares clave: valor (sin comillas en la clave, string en valor usa comillas dobles):
```
jsn datos = { clave: "valor", numero: 42 }
```
Accesos soportados observados:
```
persona.nombre
persona["datos_personales"].peso
persona.direcciones[0].direccion
persona.telefonos[1]
```
Mutaciones:
```
persona.nombre = "María"
persona.establecer("nombre", "María")
persona.eliminar("activo")
persona.fusionar(otro_json)
```
Métodos observados:
```
contiene_clave(clave), claves(), valores(), establecer(clave, valor), eliminar(clave), fusionar(json), texto(), texto_formateado(), jsn() (desde texto)
```

### 8. Funciones
Declaración:
```
vacio saludar() { ... }
número sumar(número a, número b) { retornar a + b }
texto concatenar(texto a, texto b) { retornar a + b }
log es_par(entero valor) { retornar valor % 2 == 0 }
```
Parámetros mutables:
```
texto funcion_con_parametros_modificables(texto var palabra) { ... }
```
Recursión (factorial):
```
entero factorial(entero n) { si (n == 0) { retornar 1 } sino { retornar n * factorial(n - 1) } }
```

### 9. Objetos
Estructura básica:
```
objeto NombreObjeto {
    privado:
        texto nombre
    publico:
        NombreObjeto(texto nombre) { ambiente.nombre = nombre }
        texto obtener_nombre() { retornar ambiente.nombre }
}
```
Secciones posibles: `privado:` y `publico:` (orden puede variar). También atributos y métodos sin etiquetas (públicos por defecto).
Uso de `ambiente` dentro del objeto para acceder a atributos o métodos internos.
Instanciación:
```
NombreObjeto instancia = nuevo NombreObjeto("Juan", 30)
NombreObjeto var instancia_mutable = nuevo NombreObjeto("Juan", 30)
```
Miembros `libre` (estáticos) observados:
```
objeto UsuarioLibre { libre texto nombre; libre entero absoluto(entero valor) { ... } }
UsuarioLibre.absoluto(-10)
```
Se observaron `libre` combinados con mutabilidad: `libre texto var nombre = "Sin nombre"`.

Operador ternario visto:
```
valor < 0 ? -valor : valor
```

### 10. Módulos
Exportación:
```
exportar {
    sumar_entero,
    Usuario,
    instancia_usuario,
    saludo
}
```
Importación con alias:
```
importar { sumar_entero, Usuario, instancia_usuario, saludo como texto_saludo } desde "exportar.qz"
```

### 11. Control de Flujo
Condicionales:
```
si (condicion) { ... } sino si (otra) { ... } sino { ... }
```
Bucles:
```
mientras (cond) { ... }
para (entero var i = 0; i < 10; i++) { ... }
hacer { ... } mientras (cond)
para (entero var v en lista_numeros) { ... }
para (entero var v cada lista_numeros) { ... }
```
Control dentro de bucles:
```
continuar
romper
```

### 12. Manejo de Excepciones
Patrón:
```
intentar {
    // código
} capturar (excepcion e) {
    consola.mostrar(e.mensaje)
    consola.mostrar("Pila de llamadas:" + e.llamadas.texto())
} finalmente {
    // opcional
}
```
Lanzar excepción:
```
lanzar "mensaje"   // Observado: `lanzar "Error: ..."`
```
Propiedades de excepción usadas: `mensaje`, `llamadas`.

### 13. Operadores
Binarios aritméticos: `+ - * / %`
Comparación: `== != > < >= <=`
Lógicos: `!` (negación booleana)
Concatenación texto: `+`
Asignación simple: `=`
Incremento/decremento: `variable++`, `variable--`
Compuestos: `+= -= *= /= %=`
Ternario: `cond ? expr1 : expr2`
Acceso índice: `obj[exp]`
Acceso propiedad: `obj.prop`

### 14. Interpolación de Texto
Prefijo `t` seguido de cadena con llaves:
```
texto saludo = t"¡Hola {nombre}!"
texto expr = t"Suma: {a + b}, Multiplicación: {a * b}"
```
Soporta expresiones y distintos tipos (listas, json, booleanos, números).
Escape de llaves literales:
```
t"El valor es {numero} y esto son llaves: \{valor2}"
```
Multilínea: se observaron cadenas interpoladas con saltos de línea directos.

### 15. Conversión de Tipos (Métodos Postfijo)
Observados:
```
"123".entero()
"123.45".número()  // y .numero() (variación en ejemplos)
"verdadero".log()
"{\"clave\":\"valor\"}".jsn()
"1,2,3".lista()
valor.texto()
lista.json()
lista.log()    // convierte lista a valor booleano (verdadero si no vacía, falso si vacía)
```
Nota: Se debe normalizar a `.numero()` o `.número()` según definición real; ambos fueron usados en ejemplos.

### 16. Métodos de Texto Observados
```
longitud(), entero(), numero()/número(), mayusculas(), minusculas(), capitalizar(), titulo(),
recortar(), recortar_inicio(), recortar_final(), contiene(sub), empieza_con(pref), termina_con(suf),
encontrar(sub), buscar_ultimo(sub), reemplazar(ant, nuevo), reemplazar_primero(ant, nuevo),
dividir(sep), partir_lineas(), repetir(n), subtexto(i, f), izquierda(n), derecha(n), invertir(),
es_numero(), es_entero(), es_alfanumerico(), a_base64(), decodificar_base64(), a_url(), decodificar_url(),
igual_sin_caso(otro), unir(sep) sobre listas de texto / enteros usando conversión implícita,
texto() (para conversión), log(), numero()/número() (desde contenido numérico)
```
Acceso por índice y negativo: `texto_var[0]`, `texto_var[-1]`.

### 17. Métodos JSON (Resumen ya listados)
Repetidos aquí para consolidar:
```
contiene_clave(), claves(), valores(), establecer(), eliminar(), fusionar(), texto(), texto_formateado(), jsn()
```

### 18. Métodos de Lista (Resumen ya listados)
Ver sección 6.

### 19. Consola
Métodos observados:
```
consola.mostrar(x)
consola.mostrar_error(x)
consola.mostrar_advertencia(x)
consola.mostrar_exito(x)
consola.mostrar_informacion(x)
consola.pedir(prompt)
consola.pedir_secreto(prompt)
```

### 20. Reglas de Estilo Inferidas
- Nombres de funciones y variables: snake_case (`valor_entero`, `sumar_entero`).
- Nombres de objetos: PascalCase (`Usuario`, `DefinicionUsuario`).
- No hay punto y coma al final de líneas.
- Strings usan comillas dobles.
- Comentarios con `//`.
- Espaciado: se permite o no espacio antes de llave (`mientras (...) {`).

### 21. Anti‑Ejemplos (No Inventar)
No observados y NO asumir válidos:
```
clases, switch, case, break (en lugar de romper), continue (en lugar de continuar), do { } while sin palabra hacer,
funciones flecha, import/export estilo JavaScript, tipos genéricos múltiples (lista<entero, texto>),
operadores && || (no observados, usar expansión lógica explícita con si anidado si se requiere),
interpolación con ${...} (no se usa formato estilo JS),
palabras clave async/await, try/catch ingles, return ingles, true/false ingles, null ingles.
```

### 22. Ambigüedades Detectadas
- Variación `.numero()` vs `.número()`: estandarizar en implementación (documentar preferencia).
- No se muestran ejemplos de combinación de varias capturas de excepción; solo un bloque `capturar`.
- No se observan métodos para slicing negativo en subtexto (solo índices negativos directos en acceso elemental).

### 23. Recomendaciones para Extensiones Futuras (Marcar Como Propuesta)
Si se necesita nueva sintaxis, documentar antes en propuesta:
```
// PROPUESTA SINTAXIS: <descripción>
```
No incluir en código ejecutable hasta aprobación.

---
Esta referencia debe mantenerse sincronizada con ejemplos reales. Al añadir nueva característica, primero agregar ejemplo válido en `ejemplos/` y luego actualizar este documento.

### 24. Política de Tildes en Palabras Reservadas
El lenguaje Quetzal debe aceptar ambas variantes (con y sin tilde) de palabras reservadas acentuadas, para facilitar escritura rápida y mantener corrección ortográfica cuando se desea. Los ejemplos originales pueden contener una u otra forma; el parser debe ser tolerante.

Pares equivalentes admitidos (no exhaustivo):
```
numero / número
log / lóg
publico / público
excepcion / excepción
vacio / vacío (actualmente se usa 'vacio'; 'vacío' es alias aceptado potencial al documentarse)
```
Reglas:
1. Forma recomendada al escribir NUEVOS ejemplos y documentación: con tilde cuando corresponda (ej. `número`, `público`, `lóg`).
2. No marcar como error la variante sin tilde; nunca auto‑corregir silenciosamente (aceptar `numero`, `publico`, `log`).
3. No mezclar dentro de un mismo bloque ambas formas de la MISMA palabra si se puede evitar.
4. Tests pueden incluir casos mixtos (`numero` y `número`) para asegurar compatibilidad.
5. Si se introduce nueva palabra susceptible de acento, documentar ambos alias en esta sección.

Nota: Esta flexibilidad aplica solo a tildes ortográficas válidas; no permite crear variantes arbitrarias.

Compatibilidad histórica: Algunos ejemplos heredados pueden usar `.logico()`; la forma vigente es `.log()`. No promover `.logico()` en documentación nueva; mantener alias interno solo si ya existe en la implementación por retrocompatibilidad.

