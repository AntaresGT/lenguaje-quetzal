# Informe de rendimiento

Mediciones de procesos release; el tiempo no incluye compilación de Rust. `aceleracion = tiempo_sin_jit / tiempo_automatico`.

## Escenarios

| Escenario | Sin JIT (ms) | Automático (ms) | Aceleración | Δ automático |
|---|---:|---:|---:|---:|
| corto | 3.708 | 3.740 | 0.991× | 0.87 % |
| pequenas | 3.721 | 4.133 | 0.900× | 11.06 % |
| caliente | 141.635 | 6.543 | 21.647× | -95.38 % |
| no_elegible | 15.115 | 16.223 | 0.932× | 7.33 % |

## Histórico

| Escenario | v0.0.2 (ms) | 0.0.3 original (ms) | Actual sin feature (ms) |
|---|---:|---:|---:|
| corto | 4.053 | 3.569 | 3.518 |
| pequenas | 4.020 | 3.655 | 3.560 |
| caliente | 898.884 | 144.014 | 137.194 |
| no_elegible | 32.137 | 15.142 | 14.508 |

## Caché y memoria

| Modo | Fría (ms) | Caliente (ms) | RSS corto (KiB) | RSS caliente (KiB) |
|---|---:|---:|---:|---:|
| automatico | 3.957 | 3.572 | 11404 | 11416 |
| sin_jit | 3.704 | 3.564 | 11416 | 11416 |
| sin_feature | 3.782 | 3.342 | 11416 | 11416 |

Los datos completos, dispersión, tamaños de muestra y entorno están en `resultados.json` y `resultados.csv`.
