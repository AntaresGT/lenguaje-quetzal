#!/usr/bin/env bash
# AppImage con el binario musl estático. Sin librerías extra ni linuxdeploy.
set -euo pipefail

binario=${1:?falta la ruta del binario}
salida=${2:?falta el directorio de salida}

raiz=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
version=$("$raiz/.github/scripts/version-workspace.sh")
trabajo=$(mktemp -d)
trap 'rm -rf "$trabajo"' EXIT

appdir="$trabajo/Quetzal.AppDir"
mkdir -p "$appdir/usr/bin" \
  "$appdir/usr/share/applications" \
  "$appdir/usr/share/icons/hicolor/256x256/apps"
install -m 755 "$binario" "$appdir/usr/bin/quetzal"
install -m 644 "$raiz/recursos/linux/quetzal.desktop" "$appdir/quetzal.desktop"
cp "$appdir/quetzal.desktop" "$appdir/usr/share/applications/quetzal.desktop"
install -m 644 "$raiz/recursos/imagenes/ico_lenguaje_quetzal.png" "$appdir/quetzal.png"
cp "$appdir/quetzal.png" "$appdir/.DirIcon"
install -m 644 "$raiz/recursos/imagenes/ico_lenguaje_quetzal.png" \
  "$appdir/usr/share/icons/hicolor/256x256/apps/quetzal.png"

cat > "$appdir/AppRun" <<'EOF'
#!/bin/sh
aqui=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec "$aqui/usr/bin/quetzal" "$@"
EOF
chmod 755 "$appdir/AppRun"

herramienta="$trabajo/appimagetool"
curl -fsSL \
  -o "$herramienta" \
  "https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage"
chmod 755 "$herramienta"

mkdir -p "$salida"
imagen="$salida/Quetzal-${version}-x86_64.AppImage"
ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$herramienta" "$appdir" "$imagen"
chmod 755 "$imagen"
if [[ ! -x "$imagen" ]]; then
  echo "appimagetool no produjo un AppImage ejecutable" >&2
  exit 1
fi
