#!/bin/sh
# ctxhelpr installer — https://sh.ctxhelpr.dev
# Usage: curl -sSf https://sh.ctxhelpr.dev | sh
#
# Options:
#   --version VERSION    Install a specific version (default: latest)
#   --install-dir DIR    Install directory (default: ~/.local/bin)
#   --skip-setup         Skip Claude Code integration prompt
#   --help               Show this help message

set -eu

REPO="rijuma/ctxhelpr"
DEFAULT_INSTALL_DIR="$HOME/.local/bin"

# ---------------------------------------------------------------------------
# Output helpers
# ---------------------------------------------------------------------------

use_color() {
    [ -t 1 ] && [ -z "${NO_COLOR:-}" ]
}

info() {
    if use_color; then
        printf '\033[1;34m::\033[0m %s\n' "$1"
    else
        printf ':: %s\n' "$1"
    fi
}

success() {
    if use_color; then
        printf '\033[1;32m::\033[0m %s\n' "$1"
    else
        printf ':: %s\n' "$1"
    fi
}

warn() {
    if use_color; then
        printf '\033[1;33mwarning:\033[0m %s\n' "$1" >&2
    else
        printf 'warning: %s\n' "$1" >&2
    fi
}

err() {
    if use_color; then
        printf '\033[1;31merror:\033[0m %s\n' "$1" >&2
    else
        printf 'error: %s\n' "$1" >&2
    fi
    exit 1
}

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------

VERSION=""
INSTALL_DIR=""
SKIP_SETUP=false

usage() {
    cat <<'EOF'
ctxhelpr installer

Usage:
    curl -sSf https://sh.ctxhelpr.dev | sh
    curl -sSf https://sh.ctxhelpr.dev | sh -s -- [OPTIONS]

Options:
    --version VERSION    Install a specific version (default: latest)
    --install-dir DIR    Install directory (default: ~/.local/bin)
    --skip-setup         Skip Claude Code integration prompt
    --help               Show this help message
EOF
    exit 0
}

while [ $# -gt 0 ]; do
    case "$1" in
        --version)
            [ $# -lt 2 ] && err "--version requires a value"
            VERSION="$2"
            shift 2
            ;;
        --install-dir)
            [ $# -lt 2 ] && err "--install-dir requires a value"
            INSTALL_DIR="$2"
            shift 2
            ;;
        --skip-setup)
            SKIP_SETUP=true
            shift
            ;;
        --help)
            usage
            ;;
        *)
            err "unknown option: $1"
            ;;
    esac
done

INSTALL_DIR="${INSTALL_DIR:-$DEFAULT_INSTALL_DIR}"

# ---------------------------------------------------------------------------
# HTTP client detection
# ---------------------------------------------------------------------------

_fetch=""

detect_http_client() {
    if command -v curl >/dev/null 2>&1; then
        _fetch="curl"
    elif command -v wget >/dev/null 2>&1; then
        _fetch="wget"
    else
        err "either curl or wget is required"
    fi
}

# Fetch a URL to stdout
fetch() {
    if [ "$_fetch" = "curl" ]; then
        curl --proto '=https' --tlsv1.2 -sSfL "$1"
    else
        wget --https-only --quiet -O- "$1"
    fi
}

# Fetch a URL to a file
fetch_to() {
    if [ "$_fetch" = "curl" ]; then
        curl --proto '=https' --tlsv1.2 -sSfL -o "$2" "$1"
    else
        wget --https-only --quiet -O "$2" "$1"
    fi
}

# ---------------------------------------------------------------------------
# Platform detection
# ---------------------------------------------------------------------------

detect_platform() {
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)  os="linux" ;;
        Darwin) os="macos" ;;
        MINGW*|MSYS*|CYGWIN*|Windows_NT)
            err "Windows is not supported by this installer. Download the .zip from https://github.com/$REPO/releases/latest"
            ;;
        *)
            err "unsupported operating system: $os"
            ;;
    esac

    case "$arch" in
        x86_64|amd64)   arch="x64" ;;
        aarch64|arm64)   arch="arm64" ;;
        *)
            err "unsupported architecture: $arch"
            ;;
    esac

    PLATFORM="${os}-${arch}"
}

# ---------------------------------------------------------------------------
# Version resolution
# ---------------------------------------------------------------------------

resolve_version() {
    if [ -n "$VERSION" ]; then
        # Strip leading 'v' if present
        VERSION="${VERSION#v}"
        return
    fi

    info "Fetching latest version..."
    api_response="$(fetch "https://api.github.com/repos/$REPO/releases/latest")" || \
        err "failed to query GitHub API for latest release"

    VERSION="$(printf '%s' "$api_response" | grep '"tag_name"' | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"v\{0,1\}\([^"]*\)".*/\1/')"

    [ -z "$VERSION" ] && err "could not determine latest version from GitHub API"
}

# ---------------------------------------------------------------------------
# Download & verify
# ---------------------------------------------------------------------------

download_and_verify() {
    asset="ctxhelpr-${VERSION}-${PLATFORM}.tar.gz"
    base_url="https://github.com/$REPO/releases/download/v${VERSION}"
    tarball_url="${base_url}/${asset}"
    checksum_url="${base_url}/${asset}.sha256"

    TMPDIR="$(mktemp -d)" || err "failed to create temp directory"
    trap 'rm -rf "$TMPDIR"' EXIT INT TERM

    info "Downloading ctxhelpr v${VERSION} (${PLATFORM})..."
    fetch_to "$tarball_url" "$TMPDIR/$asset" || \
        err "failed to download $tarball_url"

    # Checksum verification
    if fetch_to "$checksum_url" "$TMPDIR/${asset}.sha256" 2>/dev/null; then
        expected="$(cut -d ' ' -f1 < "$TMPDIR/${asset}.sha256")"
        if command -v sha256sum >/dev/null 2>&1; then
            actual="$(sha256sum "$TMPDIR/$asset" | cut -d ' ' -f1)"
        elif command -v shasum >/dev/null 2>&1; then
            actual="$(shasum -a 256 "$TMPDIR/$asset" | cut -d ' ' -f1)"
        else
            warn "no sha256sum or shasum found — skipping checksum verification"
            actual=""
        fi

        if [ -n "$actual" ]; then
            if [ "$expected" != "$actual" ]; then
                err "checksum mismatch (expected $expected, got $actual)"
            fi
            success "Checksum verified."
        fi
    else
        warn "checksum file not available — skipping verification"
    fi
}

# ---------------------------------------------------------------------------
# Install
# ---------------------------------------------------------------------------

install_binary() {
    mkdir -p "$INSTALL_DIR" || err "failed to create $INSTALL_DIR"

    tar xzf "$TMPDIR/$asset" -C "$TMPDIR" || err "failed to extract archive"
    mv "$TMPDIR/ctxhelpr" "$INSTALL_DIR/ctxhelpr" || err "failed to move binary to $INSTALL_DIR"
    chmod +x "$INSTALL_DIR/ctxhelpr"

    success "Installed ctxhelpr to $INSTALL_DIR/ctxhelpr"
}

# ---------------------------------------------------------------------------
# PATH check
# ---------------------------------------------------------------------------

check_path() {
    case ":${PATH}:" in
        *":${INSTALL_DIR}:"*) return ;;
    esac

    warn "$INSTALL_DIR is not in your PATH"

    shell_name="$(basename "${SHELL:-/bin/sh}")"
    case "$shell_name" in
        bash)
            printf '  Add it by running:\n'
            # shellcheck disable=SC2016
            printf '    echo '\''export PATH="$HOME/.local/bin:$PATH"'\'' >> ~/.bashrc && source ~/.bashrc\n'
            ;;
        zsh)
            printf '  Add it by running:\n'
            # shellcheck disable=SC2016
            printf '    echo '\''export PATH="$HOME/.local/bin:$PATH"'\'' >> ~/.zshrc && source ~/.zshrc\n'
            ;;
        fish)
            printf '  Add it by running:\n'
            printf '    fish_add_path %s\n' "$INSTALL_DIR"
            ;;
        *)
            printf '  Add %s to your PATH in your shell configuration file.\n' "$INSTALL_DIR"
            ;;
    esac
    printf '\n'
}

# ---------------------------------------------------------------------------
# Claude Code integration
# ---------------------------------------------------------------------------

setup_claude_code() {
    if [ "$SKIP_SETUP" = true ]; then
        return
    fi

    printf '\n'

    # Reconnect stdin to the terminal when piped (curl | sh)
    if [ ! -t 0 ] && [ -e /dev/tty ]; then
        exec < /dev/tty
    fi

    if [ -t 0 ]; then
        printf 'Set up Claude Code integration now? [Y/n] '
        read -r answer </dev/stdin
        case "$answer" in
            [nN]|[nN][oO]) ;;
            *)
                info "Running ctxhelpr install -g..."
                "$INSTALL_DIR/ctxhelpr" install -g
                ;;
        esac
    else
        info "Run 'ctxhelpr install -g' to set up Claude Code integration."
    fi
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

main() {
    detect_http_client
    detect_platform
    resolve_version
    download_and_verify
    install_binary
    check_path
    setup_claude_code

    printf '\n'
    success "ctxhelpr v${VERSION} is ready!"
}

main
