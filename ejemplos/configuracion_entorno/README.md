# Configuración de entorno

Este ejemplo muestra cómo cargar variables desde un archivo `.env` sin
incluir valores de configuración directamente en el código Quetzal.

```bash
copy .env.example .env   # Windows
cp .env.example .env     # Linux/macOS
quetzal ejecutar principal.qz
```

El archivo `.env` real puede contener secretos y debe estar incluido en
`.gitignore`. Publica únicamente `.env.example` con nombres y valores de
ejemplo.
