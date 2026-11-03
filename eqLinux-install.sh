#!/usr/bin/env bash
# --------------------------------------------------------------
# install‑linux.sh – instalador automático de eqLinux
# --------------------------------------------------------------

set -euo pipefail

# ---------------------- CONFIGURACIÓN -------------------------
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

BINARY_REL="target/release/eqlinux"          # binario compilado
INSTALL_SYSTEM="false"                       # "true" para instalar a /usr/local/bin
USER_BIN_DIR="$HOME/.local/bin"
SYSTEM_BIN_DIR="/usr/local/bin"

ICON_SRC="assets/logo.png"
ICON_SIZE="48"
ICON_DEST_USER="$HOME/.local/share/icons/hicolor/${ICON_SIZE}x${ICON_SIZE}/apps/eqlinux.png"
ICON_DEST_SYSTEM="/usr/share/icons/hicolor/${ICON_SIZE}x${ICON_SIZE}/apps/eqlinux.png"

DESKTOP_FILE="$HOME/.local/share/applications/eqlinux.desktop"
# ------------------------------------------------------------

# ---------- 1. Comprobar herramientas y compilar -----------------
if ! command -v cargo >/dev/null 2>&1; then
    echo "❌ No encuentro Cargo. Instala Rust y Cargo antes de continuar."
    exit 1
fi

# ---------- 2. Comprobar soporte AppIndicator -------------------
if ! ldconfig -p | grep "libayatana-appindicator3.so.1" >/dev/null; then
    echo "❌ Falta la biblioteca runtime de Ayatana AppIndicator."
    echo "   Debian/Ubuntu: sudo apt install libayatana-appindicator3-1"
    echo "   Fedora:        sudo dnf install libayatana-appindicator-gtk3"
    exit 1
fi

echo "🔨 Compilando eqLinux en modo release..."
cargo build --release

if [[ ! -f "$BINARY_REL" ]]; then
    echo "❌ La compilación terminó, pero no se encontró $BINARY_REL"
    exit 1
fi

# ---------- 3. Copiar el binario --------------------------------
if [[ "$INSTALL_SYSTEM" == "true" ]]; then
    BIN_DEST="$SYSTEM_BIN_DIR/eqlinux"
    BIN_TMP="$SYSTEM_BIN_DIR/.eqlinux.tmp.$$"
    sudo install -m 755 "$BINARY_REL" "$BIN_TMP"
    sudo mv -f "$BIN_TMP" "$BIN_DEST"
    echo "📦 Binario global en $SYSTEM_BIN_DIR/eqlinux"
else
    mkdir -p "$USER_BIN_DIR"
    BIN_DEST="$USER_BIN_DIR/eqlinux"
    BIN_TMP="$USER_BIN_DIR/.eqlinux.tmp.$$"
    install -m 755 "$BINARY_REL" "$BIN_TMP"
    mv -f "$BIN_TMP" "$BIN_DEST"
    echo "📦 Binario usuario en $USER_BIN_DIR/eqlinux"
fi

# ---------- 4. Instalar el icono --------------------------------
mkdir -p "$(dirname "$ICON_DEST_USER")"
cp "$ICON_SRC" "$ICON_DEST_USER"
echo "🖼️  Icono en $ICON_DEST_USER"

if [[ "$INSTALL_SYSTEM" == "true" ]]; then
    sudo mkdir -p "$(dirname "$ICON_DEST_SYSTEM")"
    sudo cp "$ICON_DEST_USER" "$ICON_DEST_SYSTEM"
    echo "🖼️  Icono también en directorio del sistema"
fi

# ---------- 5. Archivo .desktop (menú + barra) -----------------
mkdir -p "$(dirname "$DESKTOP_FILE")"
cat > "$DESKTOP_FILE" <<EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=eqLinux
Comment=Ecualizador y mezclador de volumen global para Linux, inspirado en eqMac
Exec=$BIN_DEST
Icon=$ICON_DEST_USER
Categories=AudioVideo;Audio;Music;GTK;
StartupNotify=false
Terminal=false
EOF
echo "✅ .desktop creado en $DESKTOP_FILE"

# ---------- 6. (Opcional) Autostart -----------------------------
# Descomenta las siguientes líneas si quieres que se abra solo al iniciar sesión:
# AUTOSTART="$HOME/.config/autostart/eqlinux.desktop"
# mkdir -p "$(dirname "$AUTOSTART")"
# ln -sf "$DESKTOP_FILE" "$AUTOSTART"
# echo "🔧 autostart creado"

# ---------- 7. Fin ------------------------------------------------
echo ""
echo "=========================================================="
echo "🟢 eqLinux instalado"
echo "   • Binario   : $BIN_DEST"
echo "   • Icono     : $ICON_DEST_USER"
echo "   • Menú      : $DESKTOP_FILE"
echo "   • Lanzar    : $BIN_DEST"
echo "   (para fijar el icono en la barra: arrastrarlo del menú al panel o usar “Add to panel” de tu escritorio)"
echo "=========================================================="
