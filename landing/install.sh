#!/bin/sh
# yaak installer — https://getyaak.ai
# Usage: curl -fsSL https://getyaak.ai/install.sh | bash
#
# Environment variables:
#   YAAK_VERSION      Pin a specific version (e.g. v0.0.8)
#   YAAK_INSTALL_DIR  Override install directory (default: ~/.local/bin)

set -eu

REPO="hanneshapke/yaak"
DEFAULT_INSTALL_DIR="$HOME/.local/bin"
INSTALL_DIR="${YAAK_INSTALL_DIR:-$DEFAULT_INSTALL_DIR}"

# ── helpers ──────────────────────────────────────────────────────────

info()  { printf '  \033[1;34m>\033[0m %s\n' "$1"; }
ok()    { printf '  \033[1;32m✓\033[0m %s\n' "$1"; }
err()   { printf '  \033[1;31m✗\033[0m %s\n' "$1" >&2; }
bold()  { printf '\033[1m%s\033[0m\n' "$1"; }

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    err "$1 is required but not found"
    exit 1
  fi
}

cleanup() {
  if [ -n "${TMPDIR_YAAK:-}" ] && [ -d "$TMPDIR_YAAK" ]; then
    rm -rf "$TMPDIR_YAAK"
  fi
}
trap cleanup EXIT

# ── pre-flight ───────────────────────────────────────────────────────

bold ""
bold "  Installing yaak — Yet Another AI for the Kommandozeile"
bold ""

need curl
need tar

# ── detect platform ──────────────────────────────────────────────────

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)  OS_TARGET="unknown-linux-gnu" ;;
  Darwin) OS_TARGET="apple-darwin" ;;
  *)      err "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64)         ARCH_TARGET="x86_64" ;;
  aarch64|arm64)  ARCH_TARGET="aarch64" ;;
  *)              err "Unsupported architecture: $ARCH"; exit 1 ;;
esac

# aarch64 Linux is not yet available
if [ "$OS" = "Linux" ] && [ "$ARCH_TARGET" = "aarch64" ]; then
  err "aarch64-linux binaries are not yet available."
  err "Install via cargo instead: cargo install yaak"
  exit 1
fi

TARGET="${ARCH_TARGET}-${OS_TARGET}"
info "Detected platform: $OS $ARCH ($TARGET)"

# ── resolve version ──────────────────────────────────────────────────

if [ -n "${YAAK_VERSION:-}" ]; then
  VERSION="$YAAK_VERSION"
  info "Using pinned version: $VERSION"
else
  info "Resolving latest version..."
  VERSION="$(curl -fsS -o /dev/null -w '%{redirect_url}' \
    "https://github.com/$REPO/releases/latest" \
    | sed 's|.*/||')"
  if [ -z "$VERSION" ]; then
    err "Could not determine latest version"
    exit 1
  fi
  info "Latest version: $VERSION"
fi

# ── download ─────────────────────────────────────────────────────────

ARCHIVE="yaak-${VERSION}-${TARGET}.tar.gz"
URL="https://github.com/$REPO/releases/download/${VERSION}/${ARCHIVE}"
SUMS_URL="https://github.com/$REPO/releases/download/${VERSION}/SHA256SUMS.txt"

TMPDIR_YAAK="$(mktemp -d)"
info "Downloading $ARCHIVE..."
curl -fSL -o "$TMPDIR_YAAK/$ARCHIVE" "$URL"
curl -fSL -o "$TMPDIR_YAAK/SHA256SUMS.txt" "$SUMS_URL"

# ── verify checksum ──────────────────────────────────────────────────

info "Verifying checksum..."

if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL="$(sha256sum "$TMPDIR_YAAK/$ARCHIVE" | cut -d' ' -f1)"
elif command -v shasum >/dev/null 2>&1; then
  ACTUAL="$(shasum -a 256 "$TMPDIR_YAAK/$ARCHIVE" | cut -d' ' -f1)"
else
  err "sha256sum or shasum is required for checksum verification"
  exit 1
fi

EXPECTED="$(grep "$ARCHIVE" "$TMPDIR_YAAK/SHA256SUMS.txt" | head -1 | cut -d' ' -f1)"

if [ -z "$EXPECTED" ]; then
  err "Could not find checksum for $ARCHIVE in SHA256SUMS.txt"
  exit 1
fi

if [ "$ACTUAL" != "$EXPECTED" ]; then
  err "Checksum mismatch!"
  err "  expected: $EXPECTED"
  err "  actual:   $ACTUAL"
  exit 1
fi

ok "Checksum verified"

# ── extract and install ──────────────────────────────────────────────

tar xzf "$TMPDIR_YAAK/$ARCHIVE" -C "$TMPDIR_YAAK"

mkdir -p "$INSTALL_DIR"
mv "$TMPDIR_YAAK/yaak-${VERSION}-${TARGET}/yaak" "$INSTALL_DIR/yaak"
chmod +x "$INSTALL_DIR/yaak"

ok "Installed yaak $VERSION to $INSTALL_DIR/yaak"

# ── PATH check ───────────────────────────────────────────────────────

case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    bold ""
    info "$INSTALL_DIR is not in your PATH. Add it with:"
    bold ""
    SHELL_NAME="$(basename "${SHELL:-/bin/sh}")"
    # shellcheck disable=SC2016
    case "$SHELL_NAME" in
      zsh)
        printf '    echo '\''export PATH="%s:$PATH"'\'' >> ~/.zshrc\n' "$INSTALL_DIR"
        printf '    source ~/.zshrc\n'
        ;;
      fish)
        printf '    fish_add_path %s\n' "$INSTALL_DIR"
        ;;
      *)
        printf '    echo '\''export PATH="%s:$PATH"'\'' >> ~/.bashrc\n' "$INSTALL_DIR"
        printf '    source ~/.bashrc\n'
        ;;
    esac
    bold ""
    ;;
esac

bold ""
printf '  Run yaak --config to get started? [Y/n] '
read -r REPLY < /dev/tty 2>/dev/null || REPLY=""
case "$REPLY" in
  [nN]*) ;;
  *)     "$INSTALL_DIR/yaak" --config ;;
esac
