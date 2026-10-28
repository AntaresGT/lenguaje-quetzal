#!/usr/bin/env bash
# Empaqueta el binario dinámico de Ubuntu en un .deb para /usr/bin.
set -euo pipefail

binario=${1:?falta la ruta del binario}
salida=${2:?falta el directorio de salida}

raiz=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
version=$("$raiz/.github/scripts/version-workspace.sh")
trabajo=$(mktemp -d)
trap 'rm -rf "$trabajo"' EXIT

mkdir -p "$trabajo/debian" "$trabajo/arbol/usr/bin"
install -m 755 "$binario" "$trabajo/arbol/usr/bin/quetzal"

cat > "$trabajo/debian/control" <<'EOF'
Source: quetzal
Section: devel
Priority: optional
Maintainer: Antares <info@antaresgt.com>
Standards-Version: 4.6.2

Package: quetzal
Architecture: amd64
Depends: ${shlibs:Depends}
Description: CLI del Lenguaje Quetzal
EOF

(
  cd "$trabajo"
  dpkg-shlibdeps -Tsubstvars arbol/usr/bin/quetzal
)
depends=$(sed -n 's/^shlibs:Depends=//p' "$trabajo/substvars")
if [[ -z "$depends" ]]; then
  echo "dpkg-shlibdeps no produjo shlibs:Depends" >&2
  cat "$trabajo/substvars" >&2 || true
  exit 1
fi

mkdir -p "$trabajo/pkg/DEBIAN" \
  "$trabajo/pkg/usr/bin" \
  "$trabajo/pkg/usr/share/applications" \
  "$trabajo/pkg/usr/share/icons/hicolor/256x256/apps"
install -m 755 "$binario" "$trabajo/pkg/usr/bin/quetzal"
install -m 644 "$raiz/recursos/linux/quetzal.desktop" \
  "$trabajo/pkg/usr/share/applications/quetzal.desktop"
install -m 644 "$raiz/recursos/imagenes/ico_lenguaje_quetzal.png" \
  "$trabajo/pkg/usr/share/icons/hicolor/256x256/apps/quetzal.png"

cat > "$trabajo/pkg/DEBIAN/control" <<EOF
Package: quetzal
Version: ${version}
Architecture: amd64
Maintainer: Antares <info@antaresgt.com>
Depends: ${depends}
Section: devel
Priority: optional
Homepage: https://lenguajequetzal.com
Description: CLI del Lenguaje Quetzal
 CLI del Lenguaje Quetzal: el binario quetzal.
 Autor: Allam López.
EOF

mkdir -p "$salida"
deb="$salida/quetzal_${version}_amd64.deb"
dpkg-deb --root-owner-group --build "$trabajo/pkg" "$deb"
contenido=$(dpkg-deb -c "$deb")
if ! grep -q 'usr/bin/quetzal' <<<"$contenido"; then
  echo "el .deb no contiene usr/bin/quetzal" >&2
  printf '%s\n' "$contenido" >&2
  exit 1
fi
if ! grep -q 'usr/share/icons/hicolor/256x256/apps/quetzal.png' <<<"$contenido"; then
  echo "el .deb no contiene el icono quetzal.png" >&2
  printf '%s\n' "$contenido" >&2
  exit 1
fi
