# Línea base de estabilización

- Rama de origen: `quetzal-0.0.3`.
- SHA de origen: `647cfe73e035799393315835e8e025a52a9a0fd0`.
- SHA de `v0.0.2`: `533807f3066ebd6ad21c5885b360fbf07dee5965`.
- SHA-256 de `Cargo.lock`: `436ead71303d9bce46630891852801f32087fa81a12503a584f58536b296d669`.
- Perfil release: `opt-level = 3`, LTO `fat`, una unidad de generación de código y `panic = "abort"`.
- Entorno detallado: `entorno.txt`.

Todos los comandos iniciales terminaron con código 0, sin excluir ni ignorar pruebas:

| Registro | Comando |
|---|---|
| `formato.log` | `cargo fmt --all -- --check` |
| `check.log` | `cargo check --workspace --locked` |
| `pruebas_workspace.log` | `cargo test --workspace --locked --no-fail-fast` |
| `pruebas_jit.log` | `cargo test -p pruebas_runtime --features jit --locked --no-fail-fast` |
| `pruebas_jit_nativo.log` | `cargo test -p compilador_jit --features nativo --locked --no-fail-fast` |
| `build_release.log` | `cargo build -p lnc --release --locked` |
| `build_release_jit.log` | `cargo build -p lnc --release --features jit --locked` |

La referencia esperada de `referencia/` se produjo con una copia del binario release sin JIT del SHA de origen. Los 19 casos terminaron con código 0 y sin tiempo excedido. Estos resultados describen únicamente la línea base; no constituyen un veredicto de estabilización.
