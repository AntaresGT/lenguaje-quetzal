//! Reglas de mutabilidad del Lenguaje Quetzal.
//!
//! Reglas (definidas aquí y aplicadas en `analizador`):
//!
//! 1. Toda variable es constante por defecto; solo `var` permite reasignar.
//! 2. Una instancia de objeto sin `var` es una **constante de referencia**:
//!    la referencia no puede cambiar (`usuario = nuevo Usuario(...)` falla),
//!    pero sus propiedades declaradas con `var` sí pueden modificarse.
//!    Esto NO es inmutabilidad profunda.
//! 3. Una propiedad de objeto sin `var` solo puede asignarse dentro del
//!    constructor del propio objeto (inicialización).
//! 4. Para mutar un `jsn` o una `lista` (agregar, eliminar, asignar claves o
//!    índices) la variable raíz debe ser `var`.
