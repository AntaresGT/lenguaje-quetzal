# runtime

Capa de acceso al sistema del Lenguaje Quetzal, segura por defecto. Define:

- El modelo de permisos (`red`, `sistema_archivos`, `ejecucion`) leído de `quetzal.json`; sin permiso explícito no hay acceso.
- La consola (`consola.mostrar`, `mostrar_error`, `pedir`, `pedir_secreto`, ...).
- Acceso controlado a archivos y procesos del sistema.
