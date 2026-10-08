#!/usr/bin/env bash
# Copia el binario y las librerías que ldd reporta (más NSS, que glibc
# abre con dlopen y no aparece en ldd) junto a un lanzador. El usuario
# ejecuta ./quetzal y no necesita instalar gcc-libs ni nada parecido.
set -euo pipefail

origen=${1:?falta la ruta del binario}
raiz=${2:?falta el directorio de salida}

mkdir -p "$raiz/lib"
cp -L "$origen" "$raiz/quetzal.bin"
chmod 755 "$raiz/quetzal.bin"

while IFS= read -r linea; do
  if [[ "$linea" =~ [[:space:]](/[^[:space:]]+) ]]; then
    ruta="${BASH_REMATCH[1]}"
    if [[ -f "$ruta" ]]; then
      cp -L "$ruta" "$raiz/lib/$(basename "$ruta")"
    fi
  fi
done < <(ldd "$raiz/quetzal.bin")

interprete=$(readelf -l "$raiz/quetzal.bin" | sed -n 's/.*interpreter: \(.*\)\]/\1/p' | head -n 1)
if [[ -z "$interprete" || ! -f "$interprete" ]]; then
  echo "no se encontró el intérprete ELF de $origen" >&2
  exit 1
fi
cargador=$(basename "$interprete")
cp -L "$interprete" "$raiz/lib/$cargador"

# getaddrinfo carga estos módulos en caliente. Sin ellos, la red falla
# aunque ldd esté limpio.
while IFS= read -r modulo; do
  [[ -n "$modulo" && -f "$modulo" ]] || continue
  cp -L "$modulo" "$raiz/lib/$(basename "$modulo")"
done < <(find /lib /lib64 /usr/lib /usr/lib64 -name 'libnss_files.so*' -o -name 'libnss_dns.so*' -o -name 'libresolv.so*' 2>/dev/null || true)

cat > "$raiz/quetzal" <<EOF
#!/bin/sh
aqui=\$(CDPATH= cd -- "\$(dirname -- "\$0")" && pwd)
exec "\$aqui/lib/$cargador" --library-path "\$aqui/lib" "\$aqui/quetzal.bin" "\$@"
EOF
chmod 755 "$raiz/quetzal"
