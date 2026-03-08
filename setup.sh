#!/usr/bin/env bash
# whisper-type setup script
# Run this once before first use

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log()  { echo -e "${BLUE}[setup]${NC} $*"; }
ok()   { echo -e "${GREEN}[✓]${NC} $*"; }
warn() { echo -e "${YELLOW}[!]${NC} $*"; }
err()  { echo -e "${RED}[✗]${NC} $*"; }

echo ""
echo "╔══════════════════════════════════════╗"
echo "║     whisper-type setup               ║"
echo "╚══════════════════════════════════════╝"
echo ""

# --- Detect distro ---
detect_distro() {
    if command -v pacman &>/dev/null || [ -f /etc/arch-release ]; then
        echo "arch"
    elif command -v apt-get &>/dev/null; then
        echo "debian"
    else
        echo "unknown"
    fi
}

DISTRO=$(detect_distro)
log "Detected package manager: $DISTRO"

# --- System dependencies ---
log "Checking system dependencies..."

install_pkg_arch() {
    local pkg="$1"
    if pacman -Qq "$pkg" &>/dev/null 2>&1; then
        ok "$pkg"
    else
        MISSING_PKGS+=("$pkg")
    fi
}

install_pkg_debian() {
    local deb_pkg="$1"
    local cmd_check="${2:-}"
    if dpkg -s "$deb_pkg" &>/dev/null 2>&1 || { [ -n "$cmd_check" ] && command -v "$cmd_check" &>/dev/null; }; then
        ok "$deb_pkg"
    else
        MISSING_PKGS+=("$deb_pkg")
    fi
}

MISSING_PKGS=()

if [ "$DISTRO" = "arch" ]; then
    install_pkg_arch xdotool
    install_pkg_arch alsa-lib
    install_pkg_arch pkgconf
    install_pkg_arch base-devel

    if [ ${#MISSING_PKGS[@]} -gt 0 ]; then
        log "Installing missing packages: ${MISSING_PKGS[*]}"
        sudo pacman -S --noconfirm "${MISSING_PKGS[@]}"
    else
        ok "All system dependencies present"
    fi

    # Optional: xclip for clipboard fallback (X11)
    if ! command -v xclip &>/dev/null && ! command -v xsel &>/dev/null; then
        warn "Neither xclip nor xsel found — installing xclip for clipboard fallback"
        sudo pacman -S --noconfirm xclip
    fi

    # Optional: wtype for Wayland text injection
    if [ "$XDG_SESSION_TYPE" = "wayland" ] || [ -n "${WAYLAND_DISPLAY:-}" ]; then
        if ! command -v wtype &>/dev/null; then
            log "Wayland session detected — installing wtype for text injection"
            sudo pacman -S --noconfirm wtype
        else
            ok "wtype (Wayland)"
        fi
        # wl-copy for Wayland clipboard fallback
        if ! command -v wl-copy &>/dev/null; then
            log "Installing wl-clipboard for Wayland clipboard fallback"
            sudo pacman -S --noconfirm wl-clipboard
        else
            ok "wl-clipboard"
        fi
        # ydotool: compositor-agnostic text injection (required on KDE, recommended elsewhere)
        desktop_lower="${XDG_CURRENT_DESKTOP,,}"
        if [[ "$desktop_lower" == *"kde"* ]]; then
            if ! command -v ydotool &>/dev/null; then
                log "KDE Plasma detected — installing ydotool (required for text injection)"
                sudo pacman -S --noconfirm ydotool
            else
                ok "ydotool (KDE Wayland)"
            fi
        else
            if ! command -v ydotool &>/dev/null; then
                warn "ydotool not found — text injection falls back to wtype; install ydotool for compositor-agnostic support"
            else
                ok "ydotool (optional, detected)"
            fi
        fi
        if command -v ydotool &>/dev/null; then
            log "Enabling ydotoold user service..."
            systemctl --user enable --now ydotoold || warn "Could not enable ydotoold — run: systemctl --user enable --now ydotoold"
        fi
    fi

elif [ "$DISTRO" = "debian" ]; then
    install_pkg_debian xdotool xdotool
    install_pkg_debian libasound2-dev
    install_pkg_debian pkg-config pkg-config
    install_pkg_debian build-essential gcc

    if [ ${#MISSING_PKGS[@]} -gt 0 ]; then
        log "Installing missing packages: ${MISSING_PKGS[*]}"
        sudo apt-get update -qq
        sudo apt-get install -y "${MISSING_PKGS[@]}"
    else
        ok "All system dependencies present"
    fi

    # Optional: xclip for clipboard fallback (X11)
    if ! command -v xclip &>/dev/null && ! command -v xsel &>/dev/null; then
        warn "Neither xclip nor xsel found — installing xclip for clipboard fallback"
        sudo apt-get install -y xclip
    fi

    # Optional: wtype + wl-clipboard for Wayland
    if [ "$XDG_SESSION_TYPE" = "wayland" ] || [ -n "${WAYLAND_DISPLAY:-}" ]; then
        if ! command -v wtype &>/dev/null; then
            log "Wayland session detected — installing wtype for text injection"
            sudo apt-get install -y wtype
        else
            ok "wtype (Wayland)"
        fi
        if ! command -v wl-copy &>/dev/null; then
            log "Installing wl-clipboard for Wayland clipboard fallback"
            sudo apt-get install -y wl-clipboard
        else
            ok "wl-clipboard"
        fi
        # ydotool: compositor-agnostic text injection (required on KDE, recommended elsewhere)
        desktop_lower="${XDG_CURRENT_DESKTOP,,}"
        if [[ "$desktop_lower" == *"kde"* ]]; then
            if ! command -v ydotool &>/dev/null; then
                log "KDE Plasma detected — installing ydotool (required for text injection)"
                sudo apt-get install -y ydotool
            else
                ok "ydotool (KDE Wayland)"
            fi
        else
            if ! command -v ydotool &>/dev/null; then
                warn "ydotool not found — text injection falls back to wtype; install ydotool for compositor-agnostic support"
            else
                ok "ydotool (optional, detected)"
            fi
        fi
        if command -v ydotool &>/dev/null; then
            log "Enabling ydotoold user service..."
            systemctl --user enable --now ydotoold || warn "Could not enable ydotoold — run: systemctl --user enable --now ydotoold"
        fi
    fi

else
    warn "Unknown distro — please install manually: xdotool, alsa-lib/libasound2-dev, pkgconf/pkg-config, base-devel/build-essential"
fi

# --- Rust ---
if ! command -v cargo &>/dev/null; then
    log "Rust not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    ok "Rust $(cargo --version)"
fi

# --- Whisper model ---
MODEL_DIR="$HOME/.local/share/whisper-type"
mkdir -p "$MODEL_DIR"

echo ""
echo "Available Whisper models (larger = more accurate, slower):"
echo "  1) ggml-tiny.bin    (~75 MB)   — fast, lower accuracy"
echo "  2) ggml-base.bin    (~142 MB)  — good balance  [recommended]"
echo "  3) ggml-small.bin   (~466 MB)  — better accuracy"
echo "  4) ggml-medium.bin  (~1.5 GB)  — high accuracy, needs 4+ GB RAM"
echo ""
read -rp "Choose model [2]: " MODEL_CHOICE
MODEL_CHOICE="${MODEL_CHOICE:-2}"

case "$MODEL_CHOICE" in
    1) MODEL_NAME="ggml-tiny.bin" ;;
    2) MODEL_NAME="ggml-base.bin" ;;
    3) MODEL_NAME="ggml-small.bin" ;;
    4) MODEL_NAME="ggml-medium.bin" ;;
    *) MODEL_NAME="ggml-base.bin" ;;
esac

MODEL_PATH="$MODEL_DIR/$MODEL_NAME"
BASE_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main"

if [ -f "$MODEL_PATH" ]; then
    ok "Model already exists: $MODEL_PATH"
else
    log "Downloading $MODEL_NAME..."
    if command -v wget &>/dev/null; then
        wget -q --show-progress -O "$MODEL_PATH" "$BASE_URL/$MODEL_NAME"
    elif command -v curl &>/dev/null; then
        curl -L --progress-bar -o "$MODEL_PATH" "$BASE_URL/$MODEL_NAME"
    else
        err "Neither wget nor curl found. Please download manually:"
        echo "  $BASE_URL/$MODEL_NAME"
        echo "  → $MODEL_PATH"
        exit 1
    fi
    ok "Model downloaded: $MODEL_PATH"
fi

# Write config
CONFIG_DIR="$HOME/.config/whisper-type"
mkdir -p "$CONFIG_DIR"
cat > "$CONFIG_DIR/config.json" << EOF
{
  "model_path": "$MODEL_PATH",
  "device_name": null,
  "language": "de",
  "silence_threshold_ms": 800,
  "min_speech_ms": 300,
  "max_buffer_secs": 30.0,
  "vad_threshold": 0.01
}
EOF
ok "Config written to $CONFIG_DIR/config.json"

# --- Build ---
echo ""
log "Building whisper-type (this may take a few minutes the first time)..."
cargo build --release

BINARY="./target/release/whisper-type"
if [ -f "$BINARY" ]; then
    ok "Build successful!"
else
    err "Build failed. Check errors above."
    exit 1
fi

# Install to ~/.local/bin
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"
cp "$BINARY" "$INSTALL_DIR/whisper-type"
ok "Installed to $INSTALL_DIR/whisper-type"

# Check if ~/.local/bin is in PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    warn "$INSTALL_DIR is not in your PATH."
    echo "  Add this to your ~/.bashrc, ~/.zshrc, or ~/.config/fish/config.fish:"
    echo '  export PATH="$HOME/.local/bin:$PATH"'
fi

echo ""
echo "╔══════════════════════════════════════╗"
echo "║  Setup complete!                     ║"
echo "╚══════════════════════════════════════╝"
echo ""
echo "Usage:"
echo "  whisper-type                   # Start with saved config"
echo "  whisper-type --list-devices    # Show audio devices"
echo "  whisper-type --language en     # Transcribe English"
echo "  whisper-type --dry-run         # Print text instead of typing"
echo "  whisper-type --help            # All options"
echo ""
echo "While running: speak naturally, pause to commit. Ctrl+C to stop."
echo ""
