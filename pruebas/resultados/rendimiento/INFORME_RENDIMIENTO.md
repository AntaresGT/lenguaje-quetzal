# Informe de rendimiento

Mediciones de procesos release; el tiempo no incluye compilación de Rust. `aceleracion = tiempo_sin_jit / tiempo_automatico`.

## Escenarios

| Escenario | Sin JIT (ms) | Automático (ms) | Aceleración | Δ automático |
|---|---:|---:|---:|---:|
| corto | 3.686 | 3.641 | 1.012× | -1.22 % |
| pequenas | 3.709 | 3.651 | 1.016× | -1.58 % |
| bucles_cortos | 3.754 | 3.682 | 1.020× | -1.92 % |
| repetidas | 40.812 | 39.810 | 1.025× | -2.45 % |
| caliente | 143.972 | 6.378 | 22.572× | -95.57 % |
| no_elegible | 15.531 | 15.249 | 1.019× | -1.82 % |

## Histórico

| Escenario | v0.0.2 (ms) | 0.0.3 original (ms) | Actual sin feature (ms) |
|---|---:|---:|---:|
| corto | 3.899 | 3.503 | 3.559 |
| pequenas | 3.854 | 3.567 | 3.515 |
| bucles_cortos | 3.954 | 3.622 | 3.518 |
| repetidas | 144.577 | 40.672 | 40.346 |
| caliente | 912.902 | 143.741 | 137.378 |
| no_elegible | 31.416 | 14.696 | 14.596 |

## Caché y memoria

| Modo | Fría (ms) | Caliente (ms) | RSS corto (KiB) | RSS caliente (KiB) |
|---|---:|---:|---:|---:|
| automatico | 3.346 | 3.570 | 11356 | 11368 |
| sin_jit | 3.426 | 3.538 | 11372 | 11372 |
| sin_feature | 3.334 | 3.403 | 11372 | 11372 |

Los datos completos, dispersión, tamaños de muestra y entorno están en `resultados.json` y `resultados.csv`.
