#!/usr/bin/env bash
# Installs Geth binary
# Note: intended for use only with CI (x86_64 Ubuntu, MacOS or Windows)
set -e

GETH_BUILD=${GETH_BUILD:-"1.14.8-a9523b64"}
RETH_BUILD=${RETH_BUILD:-"1.0.6"}

BIN_DIR=${BIN_DIR:-"$HOME/bin"}

PLATFORM="$(uname -s | awk '{print tolower($0)}')"
ARCHITECTURE="$(uname -m)"

main() {
    mkdir -p "$BIN_DIR"
    cd "$BIN_DIR"
    export PATH="$BIN_DIR:$PATH"
    if [ "$GITHUB_PATH" ]; then
        echo "$BIN_DIR" >> "$GITHUB_PATH"
    fi

    install_geth

    echo ""
    echo "Installed Geth:"
    geth version

    install_reth

    echo ""
    echo "Installed Reth:"
    reth --version
}

# Installs geth from https://geth.ethereum.org/downloads
install_geth() {
    case "$PLATFORM" in
        linux)
            NAME="geth-$PLATFORM-amd64-$GETH_BUILD"
            curl -sL "https://gethstore.blob.core.windows.net/builds/$NAME.tar.gz" | tar -xzf -
            mv -f "$NAME/geth" ./
            rm -rf "$NAME"
            chmod +x geth
            ;;
        *)
            NAME="geth-windows-amd64-$GETH_BUILD"
            curl -so "https://gethstore.blob.core.windows.net/builds/$NAME.zip"
            unzip $NAME.zip
            mv -f "$NAME/geth.exe" ./
            rm -rf "$NAME" "$NAME.zip"
            ;;
    esac
}

# Install reth from https://github.com/paradigmxyz/reth/releases
install_reth() {
    case "$PLATFORM" in
        linux)
            NAME="reth-v$RETH_BUILD-$PLATFORM-$ARCHITECTURE-unknown-linux-gnu"
            curl -sL "https://github.com/paradigmxyz/reth/releases/download/v$RETH_BUILD/$NAME.tar.gz" | tar -xzf -
            mv -f "$NAME/reth" ./
            rm -rf "$NAME"
            chmod +x reth
            ;;
        *)
            NAME="reth-v$RETH_BUILD-x86_64-pc-windows-gnu"
            curl -sL "https://github.com/paradigmxyz/reth/releases/download/v$RETH_BUILD/$NAME.tar.gz" | tar -xzf -
            mv -f "$NAME/reth.exe" ./
            rm -rf "$NAME"
            ;;
    esac
}

main
