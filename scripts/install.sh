#!/bin/sh
# Installs the mpesa-dev CLI by downloading the latest GitHub release for
# your OS/architecture. Usage:
#
#   curl -fsSL https://raw.githubusercontent.com/DENNIS-CODES/mpesa-dev/main/scripts/install.sh | sh
#
# Override the install directory with MPESA_DEV_INSTALL_DIR (default:
# $HOME/.local/bin).
set -e

REPO="DENNIS-CODES/mpesa-dev"
INSTALL_DIR="${MPESA_DEV_INSTALL_DIR:-$HOME/.local/bin}"

say() { printf '%s\n' "$*"; }
err() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

detect_target() {
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux) os_part="unknown-linux-musl" ;;
        Darwin) os_part="apple-darwin" ;;
        *) err "unsupported OS: $os (see https://github.com/$REPO/releases for manual downloads)" ;;
    esac

    case "$arch" in
        x86_64 | amd64) arch_part="x86_64" ;;
        arm64 | aarch64) arch_part="aarch64" ;;
        *) err "unsupported architecture: $arch" ;;
    esac

    if [ "$os" = "Linux" ] && [ "$arch_part" = "aarch64" ]; then
        err "no prebuilt binary for linux/aarch64 yet; run 'cargo install --path .' from source instead"
    fi

    printf '%s-%s\n' "$arch_part" "$os_part"
}

main() {
    for tool in curl tar mktemp; do
        command -v "$tool" >/dev/null 2>&1 || err "$tool is required but not found"
    done

    target="$(detect_target)"
    say "Detecting latest mpesa-dev release for $target..."

    download_url=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
        | grep "browser_download_url.*${target}\\.tar\\.gz" \
        | head -n1 \
        | cut -d '"' -f4)

    [ -n "$download_url" ] || err "couldn't find a release asset for $target — see https://github.com/$REPO/releases"

    tmp_dir=$(mktemp -d 2>/dev/null || mktemp -d -t mpesa-dev)
    trap 'rm -rf "$tmp_dir"' EXIT

    say "Downloading $download_url"
    curl -fsSL "$download_url" -o "$tmp_dir/mpesa-dev.tar.gz"
    tar xzf "$tmp_dir/mpesa-dev.tar.gz" -C "$tmp_dir" mpesa-dev

    mkdir -p "$INSTALL_DIR"
    mv "$tmp_dir/mpesa-dev" "$INSTALL_DIR/mpesa-dev"
    chmod +x "$INSTALL_DIR/mpesa-dev"

    say "Installed mpesa-dev to $INSTALL_DIR/mpesa-dev"

    case ":$PATH:" in
        *":$INSTALL_DIR:"*) ;;
        *)
            say ""
            say "NOTE: $INSTALL_DIR is not on your PATH. Add this to your shell profile:"
            say "  export PATH=\"$INSTALL_DIR:\$PATH\""
            ;;
    esac

    say ""
    say "Run 'mpesa-dev --help' to get started."
}

main "$@"
