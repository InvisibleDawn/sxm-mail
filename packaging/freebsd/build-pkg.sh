#!/bin/sh
# Fabrique le paquet FreeBSD de SXM Mail (installable avec « pkg add »).
# À lancer depuis la racine du dépôt, après :
#   cargo build --release --features tauri/custom-protocol --manifest-path src-tauri/Cargo.toml
# Le paquet est écrit dans dist/.
set -eu

BIN=src-tauri/target/release/sxm-mail
[ -x "$BIN" ] || { echo "Binaire absent : $BIN (compiler d'abord)" >&2; exit 1; }

VERSION=$(sed -n 's/^ *"version": *"\([^"]*\)".*/\1/p' src-tauri/tauri.conf.json | head -n 1)
[ -n "$VERSION" ] || { echo "Version introuvable dans tauri.conf.json" >&2; exit 1; }

STAGE=$(mktemp -d)
trap 'rm -rf "$STAGE"' EXIT
P="$STAGE/usr/local"

install -d "$P/bin" "$P/share/applications" \
  "$P/share/icons/hicolor/32x32/apps" "$P/share/icons/hicolor/128x128/apps"
install -m 755 -s "$BIN" "$P/bin/sxm-mail"
install -m 644 packaging/freebsd/sxm-mail.desktop "$P/share/applications/sxm-mail.desktop"
install -m 644 src-tauri/icons/32x32.png "$P/share/icons/hicolor/32x32/apps/sxm-mail.png"
install -m 644 src-tauri/icons/128x128.png "$P/share/icons/hicolor/128x128/apps/sxm-mail.png"

# Dépendances : les paquets qui fournissent les bibliothèques liées directement au binaire
# (pkg installe lui-même les dépendances de ces paquets), plus ceux utilisés à l'exécution
# (ouverture des liens, trousseau, D-Bus).
DEPS=$(
  {
    readelf -d "$BIN" | sed -n 's/.*Shared library: \[\(.*\)\]/\1/p' | while read -r name; do
      path=$(ldd "$BIN" | awk -v n="$name" '$1 == n { print $3 }')
      [ -n "$path" ] && pkg which -q "$path" 2>/dev/null || true
    done
    for extra in xdg-utils gnome-keyring dbus; do pkg query '%n-%v' "$extra" 2>/dev/null || true; done
  } | grep -v '^$' | grep -v '^FreeBSD-' | sort -u   # FreeBSD-* : système de base, toujours présent
)

{
  printf '{\n  "name": "sxm-mail",\n  "origin": "mail/sxm-mail",\n  "version": "%s",\n' "$VERSION"
  printf '  "comment": "Client mail de bureau pour Gmail",\n'
  printf '  "desc": "SXM Mail : client mail de bureau (Tauri 2 + Rust) pour Gmail. Jetons dans le trousseau, aucun serveur tiers.",\n'
  printf '  "maintainer": "InvisibleDawn@users.noreply.github.com",\n'
  printf '  "www": "https://github.com/InvisibleDawn/sxm-mail",\n'
  printf '  "prefix": "/usr/local",\n  "licenselogic": "single",\n  "licenses": ["MIT"],\n'
  printf '  "deps": {\n'
  first=1
  for p in $DEPS; do
    line=$(pkg query '%n\t%o\t%v' "$p") || continue
    n=$(printf '%s' "$line" | cut -f1); o=$(printf '%s' "$line" | cut -f2); v=$(printf '%s' "$line" | cut -f3)
    [ "$n" = "sxm-mail" ] && continue
    [ $first -eq 1 ] || printf ',\n'
    printf '    "%s": { "origin": "%s", "version": "%s" }' "$n" "$o" "$v"
    first=0
  done
  printf '\n  }\n}\n'
} > "$STAGE/+MANIFEST"

PLIST="$STAGE/plist"
( cd "$STAGE" && find usr -type f | sed 's|^usr/local/||' | sort ) > "$PLIST"

mkdir -p dist
pkg create -M "$STAGE/+MANIFEST" -r "$STAGE" -p "$PLIST" -o dist
echo "--- Dépendances déclarées :"
printf '%s\n' $DEPS
ls -l dist
