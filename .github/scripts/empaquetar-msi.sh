#!/usr/bin/env bash
# Genera el MSI de Windows (Program Files + PATH de máquina) con WiX 5.
set -euo pipefail

exe=${1:?falta la ruta de quetzal.exe}
salida=${2:?falta el directorio de salida}

raiz=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
version=$("$raiz/.github/scripts/version-workspace.sh")
wxs="$raiz/.github/wix/quetzal.wxs"
icono="$raiz/recursos/imagenes/ico_lenguaje_quetzal.ico"
msi="$salida/quetzal-${version}-x86_64.msi"

export PATH="$PATH:${HOME}/.dotnet/tools"

if ! command -v wix >/dev/null 2>&1; then
  echo "wix no está en PATH; instala el CLI con: dotnet tool install --global wix" >&2
  exit 1
fi

if command -v cygpath >/dev/null 2>&1; then
  wxs=$(cygpath -w "$wxs")
  exe=$(cygpath -w "$exe")
  icono=$(cygpath -w "$icono")
  msi=$(cygpath -w "$msi")
fi

mkdir -p "$salida"
wix build "$wxs" \
  -arch x64 \
  -pdbtype none \
  -d "ProductVersion=${version}" \
  -d "ExeSource=${exe}" \
  -d "IconSource=${icono}" \
  -o "$msi"
