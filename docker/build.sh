#!/bin/bash
set -e

compress_with_upx() {
    local file="$1"
    if command -v upx &> /dev/null; then
        local orig_size new_size
        orig_size=$(stat -c %s "$file" 2>/dev/null || stat -f %z "$file" 2>/dev/null)
        if upx --best --lzma "$file" > /dev/null 2>&1; then
            new_size=$(stat -c %s "$file" 2>/dev/null || stat -f %z "$file" 2>/dev/null)
            echo "UPX: $file $(numfmt --to=iec "$orig_size") → $(numfmt --to=iec "$new_size")"
        else
            echo "UPX: $file compression failed"
        fi
    fi
}

BUILD_USR="build_usr"
mkdir -p "$BUILD_USR"

cargo build --release --target aarch64-unknown-linux-musl
BINARY_PATH="target/aarch64-unknown-linux-musl/release/novaprox"
if [ ! -f "$BINARY_PATH" ]; then
    echo "Binary not found: $BINARY_PATH"
    exit 1
fi
compress_with_upx "$BINARY_PATH"
cp "$BINARY_PATH" "$BUILD_USR/novaprox"

XRAY_VERSION="v26.2.6"
XRAY_URL="https://github.com/XTLS/Xray-core/releases/download/${XRAY_VERSION}/Xray-linux-arm64-v8a.zip"
XRAY_TEMP_DIR="xray_temp"
if [ ! -f "$BUILD_USR/xray" ]; then
    mkdir -p "$XRAY_TEMP_DIR"
    wget -q -O "$XRAY_TEMP_DIR/xray.zip" "$XRAY_URL"
    unzip -q "$XRAY_TEMP_DIR/xray.zip" -d "$XRAY_TEMP_DIR"
    cp "$XRAY_TEMP_DIR/xray" "$BUILD_USR/xray"
    chmod +x "$BUILD_USR/xray"
    compress_with_upx "$BUILD_USR/xray"
    rm -rf "$XRAY_TEMP_DIR"
fi

GHCP_VERSION="v1.15.2"
GHCP_URL="https://github.com/int128/ghcp/releases/download/${GHCP_VERSION}/ghcp_linux_arm64.zip"
GHCP_TEMP_DIR="ghcp_temp"
if [ ! -f "$BUILD_USR/ghcp" ]; then
    mkdir -p "$GHCP_TEMP_DIR"
    wget -q -O "$GHCP_TEMP_DIR/ghcp.zip" "$GHCP_URL"
    unzip -q "$GHCP_TEMP_DIR/ghcp.zip" -d "$GHCP_TEMP_DIR"
    cp "$GHCP_TEMP_DIR/ghcp" "$BUILD_USR/ghcp"
    chmod +x "$BUILD_USR/ghcp"
    compress_with_upx "$BUILD_USR/ghcp"
    rm -rf "$GHCP_TEMP_DIR"
fi

docker buildx build --platform linux/arm64 -t novaprox --load .
docker save -o novaprox.tar novaprox