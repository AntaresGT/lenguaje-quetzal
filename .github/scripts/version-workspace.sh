#!/usr/bin/env bash
# Imprime la versión de [workspace.package] en Cargo.toml.
set -euo pipefail

raiz=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$raiz/Cargo.toml" | head -n 1)
if [[ -z "$version" ]]; then
  echo "no se encontró version en Cargo.toml" >&2
  exit 1
fi
printf '%s\n' "$version"
