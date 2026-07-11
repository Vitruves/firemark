#!/bin/sh
# firemark installer — detects your platform, downloads the matching binary
# from the latest GitHub Release, and installs it.
#
#   curl -fsSL https://raw.githubusercontent.com/Vitruves/firemark/main/install.sh | sh
#
# Options (environment variables):
#   FIREMARK_VERSION      install a specific tag (e.g. v0.1.4). Default: latest.
#   FIREMARK_INSTALL_DIR  install location. Default: $HOME/.local/bin.

set -eu

REPO="Vitruves/firemark"
BIN="firemark"
INSTALL_DIR="${FIREMARK_INSTALL_DIR:-$HOME/.local/bin}"

info() { printf '%s\n' "$*" >&2; }
err()  { printf 'error: %s\n' "$*" >&2; exit 1; }

# --- Pick a downloader -------------------------------------------------------
if command -v curl >/dev/null 2>&1; then
    dl() { curl -fsSL "$1"; }
    dl_to() { curl -fsSL "$1" -o "$2"; }
elif command -v wget >/dev/null 2>&1; then
    dl() { wget -qO- "$1"; }
    dl_to() { wget -qO "$2" "$1"; }
else
    err "need curl or wget installed"
fi

# --- Detect platform ---------------------------------------------------------
os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
    Linux)  os_part="unknown-linux-gnu" ;;
    Darwin) os_part="apple-darwin" ;;
    *) err "unsupported OS '$os'. Windows users: download the .zip from https://github.com/$REPO/releases" ;;
esac

case "$arch" in
    x86_64|amd64) arch_part="x86_64" ;;
    arm64|aarch64) arch_part="aarch64" ;;
    *) err "unsupported architecture '$arch'" ;;
esac

target="${arch_part}-${os_part}"

# --- Resolve version ---------------------------------------------------------
if [ -n "${FIREMARK_VERSION:-}" ]; then
    tag="$FIREMARK_VERSION"
else
    info "Fetching latest release..."
    tag="$(dl "https://api.github.com/repos/$REPO/releases/latest" \
        | grep -m1 '"tag_name"' \
        | sed -E 's/.*"tag_name"[ ]*:[ ]*"([^"]+)".*/\1/')"
    [ -n "$tag" ] || err "could not determine the latest release tag"
fi

asset="${BIN}-${tag}-${target}.tar.gz"
url="https://github.com/$REPO/releases/download/$tag/$asset"

# --- Download & extract ------------------------------------------------------
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

info "Downloading $asset ..."
dl_to "$url" "$tmp/$asset" || err "download failed: $url"

tar -xzf "$tmp/$asset" -C "$tmp" || err "extraction failed"

# The archive contains a directory <asset-without-.tar.gz>/ holding the binary.
binpath="$(find "$tmp" -type f -name "$BIN" | head -n1)"
[ -n "$binpath" ] || err "binary '$BIN' not found in archive"

# --- Install -----------------------------------------------------------------
mkdir -p "$INSTALL_DIR"
install -m 0755 "$binpath" "$INSTALL_DIR/$BIN" 2>/dev/null \
    || { cp "$binpath" "$INSTALL_DIR/$BIN" && chmod 0755 "$INSTALL_DIR/$BIN"; }

info ""
info "Installed $BIN $tag -> $INSTALL_DIR/$BIN"

# --- PATH hint ---------------------------------------------------------------
case ":$PATH:" in
    *":$INSTALL_DIR:"*) : ;;
    *)
        info ""
        info "Note: $INSTALL_DIR is not in your PATH. Add it with:"
        info "  export PATH=\"$INSTALL_DIR:\$PATH\""
        info "(add that line to your ~/.bashrc, ~/.zshrc, or shell config)"
        ;;
esac

info ""
info "Run '$BIN --help' to get started."
