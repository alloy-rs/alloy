#!/usr/bin/env bash
# Installs Geth binary
# Note: intended for use only with CI (x86_64 Ubuntu, MacOS or Windows)
set -e

GETH_BUILD=${GETH_BUILD:-"1.14.0-87246f3c"}

BIN_DIR=${BIN_DIR:-"$HOME/bin"}

PLATFORM="$(uname -s | awk '{print tolower($0)}')"

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
}

# Installs geth from https://geth.ethereum.org/downloads
install_geth() {
    case "$PLATFORM" in
        linux|darwin)
            name="geth-$PLATFORM-amd64-$GETH_BUILD"
            curl -s "https://gethstore.blob.core.windows.net/builds/$name.tar.gz" | tar -xzf -
            mv -f "$name/geth" ./
            rm -rf "$name"
            chmod +x geth
            ;;
        *)
            name="geth-windows-amd64-$GETH_BUILD"
            zip="$name.zip"
            curl -so "$zip" "https://gethstore.blob.core.windows.net/builds/$zip"
            unzip "$zip"
            mv -f "$name/geth.exe" ./
            rm -rf "$name" "$zip"
            ;;
    esac
}

main
