#!/usr/bin/env bash
# Empaqueta el binario dinámico de Fedora en un .rpm para /usr/bin.
set -euo pipefail

binario=${1:?falta la ruta del binario}
salida=${2:?falta el directorio de salida}

raiz=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
version=$("$raiz/.github/scripts/version-workspace.sh")
binario=$(readlink -f "$binario")
top=$(mktemp -d)
trap 'rm -rf "$top"' EXIT

mkdir -p "$top"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
cp "$raiz/.github/rpm/quetzal.spec" "$top/SPECS/quetzal.spec"

icono=$(readlink -f "$raiz/recursos/imagenes/ico_lenguaje_quetzal.png")
desktop=$(readlink -f "$raiz/recursos/linux/quetzal.desktop")

rpmbuild -bb \
  --define "_topdir $top" \
  --define "_packager Antares <info@antaresgt.com>" \
  --define "quetzal_version ${version}" \
  --define "quetzal_binario ${binario}" \
  --define "quetzal_icono ${icono}" \
  --define "quetzal_desktop ${desktop}" \
  "$top/SPECS/quetzal.spec"

rpm=$(find "$top/RPMS" -name 'quetzal-*.rpm' ! -name '*.src.rpm' ! -name '*debuginfo*' ! -name '*debugsource*' | head -n 1)
if [[ -z "$rpm" ]]; then
  echo "rpmbuild no produjo un .rpm" >&2
  exit 1
fi

mkdir -p "$salida"
destino="$salida/quetzal-${version}-1.x86_64.rpm"
cp "$rpm" "$destino"
archivos=$(rpm -qlp "$destino")
if ! grep -qx '/usr/bin/quetzal' <<<"$archivos"; then
  echo "el .rpm no contiene /usr/bin/quetzal" >&2
  printf '%s\n' "$archivos" >&2
  exit 1
fi
if ! grep -qx '/usr/share/icons/hicolor/256x256/apps/quetzal.png' <<<"$archivos"; then
  echo "el .rpm no contiene el icono quetzal.png" >&2
  printf '%s\n' "$archivos" >&2
  exit 1
fi
